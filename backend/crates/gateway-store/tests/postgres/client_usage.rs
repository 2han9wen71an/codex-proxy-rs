use chrono::{Duration, Utc};
use gateway_admin::model::{client_usage::ClientUsageQuery, observability::TimeRange};
use gateway_core::policy::ClientApiKeyId;
use gateway_store::{StoreConfig, initialize};

use super::TestDatabase;

#[tokio::test]
async fn records_should_isolate_keys_and_page_all_outcomes_without_losing_equal_timestamps() {
    let Some(redis_url) = crate::support::test_env("CPR_TEST_REDIS_URL") else {
        return;
    };
    let Some(database) = TestDatabase::create("client_records").await else {
        return;
    };
    let database_url = crate::support::test_env("CPR_TEST_DATABASE_URL").expect("database URL");
    let mut isolated_url = url::Url::parse(&database_url).expect("database URL");
    isolated_url
        .query_pairs_mut()
        .append_pair("options[search_path]", &database.schema);
    let connection = |value: &str| {
        let mut url = url::Url::parse(value).expect("connection URL");
        let password = url.password().expect("connection password").to_owned();
        url.set_password(None).expect("remove inline password");
        serde_json::json!({ "url": url.as_str(), "password": password })
    };
    let mut config: StoreConfig = serde_json::from_value(serde_json::json!({
        "database": connection(isolated_url.as_str()), "redis": connection(&redis_url),
    }))
    .expect("store config");
    let runtime_data = tempfile::tempdir().expect("runtime directory");
    config
        .resolve_and_validate(runtime_data.path())
        .expect("resolve store config");
    let bundle = initialize(config).await.expect("store bundle");
    let port = bundle.admin_ports().client_usage();
    let start = Utc::now() - Duration::hours(1);
    let end = start + Duration::hours(1);
    for (id, key, outcome, timestamp) in [
        ("a", "key-a", "succeeded", start),
        ("b", "key-a", "failed", start),
        ("c", "key-a", "cancelled", start),
        ("d", "key-a", "incomplete", start),
        ("e", "key-a", "running", start),
        ("other-key", "key-b", "succeeded", start),
        (
            "too-early",
            "key-a",
            "succeeded",
            start - Duration::seconds(1),
        ),
        ("at-end", "key-a", "succeeded", end),
    ] {
        sqlx::query("insert into model_requests
            (id, client_api_key_ref, config_revision, protocol, operation, endpoint, client_transport,
             requested_model_id, outcome, started_at, deadline_at, completed_at, routing_scope)
            values ($1, $2, 1, 'openai', 'responses', '/v1/responses', 'http', 'gpt-5', $3, $4, $4 + interval '1 minute',
                    case when $3 = 'running' then null else $4 + interval '1 second' end, 'all')")
            .bind(id).bind(key).bind(outcome).bind(timestamp).execute(&database.pool).await.expect("seed request");
    }
    sqlx::query("update model_requests set cost_source = 'calculated', cost_amount = 0, cost_currency = 'USD', total_tokens = 0, latency_ms = 0 where id = 'a'")
        .execute(&database.pool).await.expect("seed real zero facts");
    let key = ClientApiKeyId::new("key-a").expect("key ID");
    let range = TimeRange::new(start, end).expect("range");
    let mut query = ClientUsageQuery {
        range,
        before: None,
        limit: 2,
    };
    let mut records = Vec::new();
    loop {
        let page = port
            .list_client_usage_records(&key, query.clone())
            .await
            .expect("records page");
        records.extend(page.items);
        query.before = page.next_cursor;
        if query.before.is_none() {
            break;
        }
        // 新调用比游标更新，不能使后续页重复或跨出查询的 Key。
        sqlx::query("update model_requests set started_at = $1, deadline_at = $1 + interval '1 minute', completed_at = $1 + interval '1 second' where id = 'other-key'")
            .bind(start + Duration::minutes(1))
            .execute(&database.pool)
            .await
            .expect("advance other key");
    }
    assert_eq!(
        records
            .iter()
            .map(|record| record.id.as_str())
            .collect::<Vec<_>>(),
        ["e", "d", "c", "b", "a"]
    );
    assert_eq!(
        records
            .iter()
            .map(|record| record.outcome.as_str())
            .collect::<Vec<_>>(),
        ["running", "incomplete", "cancelled", "failed", "succeeded"]
    );
    assert!(records[..4].iter().all(|record| record.cost.is_none()
        && record.total_tokens.is_none()
        && record.latency_ms.is_none()));
    let zero = records.last().expect("zero record");
    assert_eq!(
        zero.cost.as_ref().expect("known zero cost").amount.as_str(),
        "0"
    );
    assert_eq!((zero.total_tokens, zero.latency_ms), (Some(0), Some(0)));
    drop(port);
    drop(bundle);
    database.close().await;
}
