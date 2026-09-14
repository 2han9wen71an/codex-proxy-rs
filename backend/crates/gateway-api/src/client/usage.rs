//! 当前 Client 会话 Key 的强制范围聚合用量接口。

use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use chrono::{DateTime, Duration, Utc};
use gateway_admin::model::{
    client_usage::{ClientUsageCursor, ClientUsageQuery as UsageQuery},
    observability::TimeRange,
};
use serde::Deserialize;

use crate::{admin::wire::map_admin_service_error, auth::SessionState};

use crate::admin::{AdminEnvelope, AdminError, AdminQuery, AdminResponse};

use super::presenter::{ClientUsageData, ClientUsageRecordsData};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ClientUsageQuery {
    range: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ClientRecordsQuery {
    range: Option<String>,
    ends_at: DateTime<Utc>,
    before_at: Option<DateTime<Utc>>,
    before_id: Option<String>,
    limit: Option<u16>,
}

#[derive(Debug, Clone, Copy)]
enum ClientUsageRange {
    Hours24,
    Days7,
    Days30,
}

impl ClientUsageRange {
    fn parse(value: Option<&str>) -> Result<Self, AdminError> {
        match value.unwrap_or("7d") {
            "24h" => Ok(Self::Hours24),
            "7d" => Ok(Self::Days7),
            "30d" => Ok(Self::Days30),
            _ => Err(AdminError::bad_request("range 只支持 24h、7d 或 30d")),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Hours24 => "24h",
            Self::Days7 => "7d",
            Self::Days30 => "30d",
        }
    }

    fn duration(self) -> Duration {
        match self {
            Self::Hours24 => Duration::hours(24),
            Self::Days7 => Duration::days(7),
            Self::Days30 => Duration::days(30),
        }
    }
}

pub(super) fn router<S>() -> Router<S>
where
    S: SessionState + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/client/usage", get(usage::<S>))
        .route("/api/client/usage/records", get(records::<S>))
}

async fn usage<S>(
    State(state): State<S>,
    headers: axum::http::HeaderMap,
    AdminQuery(query): AdminQuery<ClientUsageQuery>,
) -> Result<impl IntoResponse, AdminError>
where
    S: SessionState + Send + Sync,
{
    let kind = ClientUsageRange::parse(query.range.as_deref())?;
    let end = Utc::now();
    let range =
        TimeRange::new(end - kind.duration(), end).map_err(|_| AdminError::invalid_time_range())?;
    let snapshot = state
        .admin_services()
        .client_usage()
        .usage(crate::session_cookie::value(&headers).as_deref(), range)
        .await
        .map_err(map_admin_service_error)?
        .ok_or_else(AdminError::session_required)?;
    Ok(AdminResponse::new(
        StatusCode::OK,
        AdminEnvelope::ok(ClientUsageData::new(&snapshot, kind.as_str())),
    ))
}

async fn records<S>(
    State(state): State<S>,
    headers: axum::http::HeaderMap,
    AdminQuery(query): AdminQuery<ClientRecordsQuery>,
) -> Result<impl IntoResponse, AdminError>
where
    S: SessionState + Send + Sync,
{
    let kind = ClientUsageRange::parse(query.range.as_deref())?;
    let before = match (query.before_at, query.before_id) {
        (None, None) => None,
        (Some(started_at), Some(id))
            if !id.trim().is_empty() && id.len() <= 256 && !id.chars().any(char::is_control) =>
        {
            Some(ClientUsageCursor { started_at, id })
        }
        _ => return Err(AdminError::bad_request("分页游标无效")),
    };
    let range = TimeRange::new(query.ends_at - kind.duration(), query.ends_at)
        .map_err(|_| AdminError::invalid_time_range())?;
    let page = state
        .admin_services()
        .client_usage()
        .records(
            crate::session_cookie::value(&headers).as_deref(),
            UsageQuery {
                range,
                before,
                limit: query.limit.unwrap_or(20),
            },
        )
        .await
        .map_err(map_admin_service_error)?
        .ok_or_else(AdminError::session_required)?;
    Ok(AdminResponse::new(
        StatusCode::OK,
        AdminEnvelope::ok(ClientUsageRecordsData::from(page)),
    ))
}
