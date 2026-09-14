//! 当前 Key 的只读调用投影；与管理端仅成功用量的列表口径分开。

use chrono::{DateTime, Utc};
use gateway_admin::model::{
    client_usage::{
        ClientUsageCursor, ClientUsageQuery, ClientUsageRecord, ClientUsageRecordsPage,
    },
    observability::{CurrencyCost, RequestOutcome},
};
use gateway_core::policy::ClientApiKeyId;
use sqlx::PgPool;

use crate::{StoreError, StoreResult, postgres_unavailable};

#[derive(sqlx::FromRow)]
struct RecordRow {
    id: String,
    started_at: DateTime<Utc>,
    requested_model_id: Option<String>,
    outcome: String,
    total_tokens: Option<i64>,
    latency_ms: Option<i64>,
    cost_amount: Option<String>,
    cost_currency: Option<String>,
}

pub(crate) async fn list_client_usage_records(
    pool: &PgPool,
    key_id: &ClientApiKeyId,
    query: ClientUsageQuery,
) -> StoreResult<ClientUsageRecordsPage> {
    let mut rows = sqlx::query_as::<_, RecordRow>(
        "select id, started_at, requested_model_id, outcome, total_tokens, latency_ms,
                cost_amount::text, cost_currency
         from model_requests
         where client_api_key_ref = $1 and started_at >= $2 and started_at < $3
           and ($4::timestamptz is null or (started_at, id) < ($4, $5))
         order by started_at desc, id desc limit $6",
    )
    .bind(key_id.as_str())
    .bind(query.range.start)
    .bind(query.range.end)
    .bind(query.before.as_ref().map(|cursor| cursor.started_at))
    .bind(query.before.as_ref().map(|cursor| cursor.id.as_str()))
    .bind(i64::from(query.limit) + 1)
    .fetch_all(pool)
    .await
    .map_err(|_| postgres_unavailable("load client usage records"))?;
    let has_more = rows.len() > usize::from(query.limit);
    rows.truncate(usize::from(query.limit));
    let next_cursor = rows
        .last()
        .filter(|_| has_more)
        .map(|row| ClientUsageCursor {
            started_at: row.started_at,
            id: row.id.clone(),
        });
    let items = rows
        .into_iter()
        .map(|row| {
            let cost = match (row.cost_amount, row.cost_currency) {
                (Some(amount), Some(currency)) => Some(CurrencyCost {
                    amount: amount.parse().map_err(|_| invalid_record())?,
                    currency,
                }),
                (None, None) => None,
                _ => return Err(invalid_record()),
            };
            Ok(ClientUsageRecord {
                id: row.id,
                started_at: row.started_at,
                model: row.requested_model_id,
                outcome: RequestOutcome::new(row.outcome).map_err(|_| invalid_record())?,
                total_tokens: unsigned(row.total_tokens)?,
                latency_ms: unsigned(row.latency_ms)?,
                cost,
            })
        })
        .collect::<StoreResult<Vec<_>>>()?;
    Ok(ClientUsageRecordsPage { items, next_cursor })
}

fn unsigned(value: Option<i64>) -> StoreResult<Option<u64>> {
    value
        .map(|value| u64::try_from(value).map_err(|_| invalid_record()))
        .transpose()
}

fn invalid_record() -> StoreError {
    StoreError::InvalidData {
        entity: "client usage record",
        message: "invalid persisted usage fact".to_owned(),
    }
}
