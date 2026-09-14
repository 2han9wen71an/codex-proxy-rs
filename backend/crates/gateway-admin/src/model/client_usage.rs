//! Client API Key 自助用量查询事实。

use chrono::{DateTime, Utc};
use gateway_core::{
    engine::budget::ClientBudgetStatus,
    policy::{ClientApiKeyId, RateLimits},
};

use super::observability::{
    CurrencyCost, RequestMetricPoint, RequestOutcome, TimeRange, UsageOverview,
};

/// 当前 Client 会话可见的安全 Key 摘要与账本状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientUsageKey {
    pub id: ClientApiKeyId,
    pub name: String,
    pub label: Option<String>,
    pub prefix: String,
    pub limits: RateLimits,
    pub budget: ClientBudgetStatus,
    pub last_used_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
}

/// 当前会话 Key 的一次强制范围用量快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientUsageSnapshot {
    pub as_of: DateTime<Utc>,
    pub key: ClientUsageKey,
    pub range: TimeRange,
    pub overview: UsageOverview,
    pub trend: Vec<RequestMetricPoint>,
}

/// 明细仅投影当前 Key 可见的调用事实，不包含账号、上游凭据和请求内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientUsageRecord {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub model: Option<String>,
    pub outcome: RequestOutcome,
    pub total_tokens: Option<u64>,
    pub latency_ms: Option<u64>,
    pub cost: Option<CurrencyCost>,
}

/// 时间与 ID 共同定位下一页，避免同一时间的调用被遗漏。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientUsageCursor {
    pub started_at: DateTime<Utc>,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientUsageQuery {
    pub range: TimeRange,
    pub before: Option<ClientUsageCursor>,
    pub limit: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientUsageRecordsPage {
    pub items: Vec<ClientUsageRecord>,
    pub next_cursor: Option<ClientUsageCursor>,
}
