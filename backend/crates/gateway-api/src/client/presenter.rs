//! Client 自助用量领域事实到安全 wire DTO 的投影。

use chrono::{DateTime, Duration, Utc};
use gateway_admin::model::{
    client_usage::{
        ClientUsageCursor, ClientUsageKey, ClientUsageRecordsPage, ClientUsageSnapshot,
    },
    observability::{CostCoverage, Granularity, RequestMetricPoint},
};
use gateway_core::{engine::budget::ClientBudgetStatus, metering::Decimal};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClientUsageRecordsData {
    items: Vec<ClientUsageRecordData>,
    next_cursor: Option<ClientUsageCursorData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientUsageRecordData {
    id: String,
    started_at: String,
    model: Option<String>,
    outcome: String,
    total_tokens: Option<u64>,
    latency_ms: Option<u64>,
    cost: Option<ClientUsageCostData>,
}

#[derive(Debug, Serialize)]
struct ClientUsageCostData {
    amount: String,
    currency: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientUsageCursorData {
    started_at: String,
    id: String,
}

impl From<ClientUsageCursor> for ClientUsageCursorData {
    fn from(cursor: ClientUsageCursor) -> Self {
        Self {
            started_at: cursor.started_at.to_rfc3339(),
            id: cursor.id,
        }
    }
}

impl From<ClientUsageRecordsPage> for ClientUsageRecordsData {
    fn from(page: ClientUsageRecordsPage) -> Self {
        Self {
            items: page
                .items
                .into_iter()
                .map(|record| ClientUsageRecordData {
                    id: record.id,
                    started_at: record.started_at.to_rfc3339(),
                    model: record.model,
                    outcome: record.outcome.as_str().to_owned(),
                    total_tokens: record.total_tokens,
                    latency_ms: record.latency_ms,
                    cost: record.cost.map(|cost| ClientUsageCostData {
                        amount: cost.amount.as_str().to_owned(),
                        currency: cost.currency,
                    }),
                })
                .collect(),
            next_cursor: page.next_cursor.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClientKeyData {
    name: String,
    label: Option<String>,
    prefix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_used_at: Option<String>,
}

impl From<&ClientUsageKey> for ClientKeyData {
    fn from(key: &ClientUsageKey) -> Self {
        Self {
            name: key.name.clone(),
            label: key.label.clone(),
            prefix: key.prefix.clone(),
            last_used_at: key.last_used_at.map(|value| value.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClientUsageData {
    as_of: String,
    timezone: &'static str,
    currency: &'static str,
    key: ClientKeyData,
    budget: BudgetData,
    limits: LimitsData,
    activity: ActivityData,
}

impl ClientUsageData {
    pub(super) fn new(snapshot: &ClientUsageSnapshot, range_kind: &'static str) -> Self {
        Self {
            as_of: snapshot.as_of.to_rfc3339(),
            timezone: "Asia/Shanghai",
            currency: "USD",
            key: ClientKeyData::from(&snapshot.key),
            budget: BudgetData::from(&snapshot.key.budget),
            limits: LimitsData {
                max_concurrency: snapshot.key.limits.max_concurrency,
                requests_per_minute: snapshot.key.limits.requests_per_minute,
            },
            activity: ActivityData::new(snapshot, range_kind),
        }
    }
}

#[derive(Debug, Serialize)]
struct BudgetData {
    daily: BudgetWindowData,
    weekly: BudgetWindowData,
}

impl From<&ClientBudgetStatus> for BudgetData {
    fn from(budget: &ClientBudgetStatus) -> Self {
        Self {
            daily: BudgetWindowData::new(
                budget.limits.daily_usd,
                budget.daily_used_usd,
                budget.daily_resets_at.map(DateTime::<Utc>::from),
                Duration::days(1),
            ),
            weekly: BudgetWindowData::new(
                budget.limits.weekly_usd,
                budget.weekly_used_usd,
                budget.weekly_resets_at.map(DateTime::<Utc>::from),
                Duration::days(7),
            ),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BudgetWindowData {
    limit_usd: String,
    used_usd: String,
    remaining_usd: Option<String>,
    starts_at: Option<String>,
    resets_at: Option<String>,
}

impl BudgetWindowData {
    fn new(
        limit: Decimal,
        used: Decimal,
        resets_at: Option<DateTime<Utc>>,
        width: Duration,
    ) -> Self {
        let remaining_usd = (limit != Decimal::ZERO).then(|| {
            Decimal::from_scaled(limit.scaled().saturating_sub(used.scaled()))
                .unwrap_or(Decimal::ZERO)
                .canonical()
        });
        Self {
            limit_usd: limit.canonical(),
            used_usd: used.canonical(),
            remaining_usd,
            starts_at: resets_at.map(|value| (value - width).to_rfc3339()),
            resets_at: resets_at.map(|value| value.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LimitsData {
    max_concurrency: u64,
    requests_per_minute: u64,
}

#[derive(Debug, Serialize)]
struct ActivityData {
    range: ActivityRangeData,
    totals: ActivityTotalsData,
    trend: Vec<ActivityTrendPointData>,
}

impl ActivityData {
    fn new(snapshot: &ClientUsageSnapshot, range_kind: &'static str) -> Self {
        let requests = &snapshot.overview.requests;
        Self {
            range: ActivityRangeData {
                kind: range_kind,
                starts_at: snapshot.range.start.to_rfc3339(),
                ends_at: snapshot.range.end.to_rfc3339(),
                granularity: snapshot
                    .trend
                    .first()
                    .map_or("hour", |point| granularity(point.granularity)),
            },
            totals: ActivityTotalsData {
                request_count: requests.request_count,
                success_count: requests.success_count,
                failure_count: requests.failure_count,
                cancelled_count: requests.cancelled_count,
                incomplete_count: requests.incomplete_count,
                input_tokens: requests.input_tokens,
                output_tokens: requests.output_tokens,
                cached_tokens: requests.cached_tokens,
                total_tokens: requests.total_tokens,
                billed_usd: usd_cost(&snapshot.overview.attempts.costs),
                cost_coverage: CostCoverageData::from(&snapshot.overview.attempts.cost_coverage),
            },
            trend: snapshot
                .trend
                .iter()
                .map(ActivityTrendPointData::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityRangeData {
    kind: &'static str,
    starts_at: String,
    ends_at: String,
    granularity: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityTotalsData {
    request_count: u64,
    success_count: u64,
    failure_count: u64,
    cancelled_count: u64,
    incomplete_count: u64,
    input_tokens: u64,
    output_tokens: u64,
    cached_tokens: u64,
    total_tokens: u64,
    billed_usd: String,
    cost_coverage: CostCoverageData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityTrendPointData {
    bucket_start: String,
    request_count: u64,
    success_count: u64,
    failure_count: u64,
    total_tokens: u64,
    billed_usd: String,
}

impl From<&RequestMetricPoint> for ActivityTrendPointData {
    fn from(point: &RequestMetricPoint) -> Self {
        Self {
            bucket_start: point.bucket_start.to_rfc3339(),
            request_count: point.metrics.request_count,
            success_count: point.metrics.success_count,
            failure_count: point.metrics.failure_count,
            total_tokens: point.metrics.total_tokens,
            billed_usd: usd_cost(&point.costs),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CostCoverageData {
    provider_reported_count: u64,
    calculated_count: u64,
    partial_count: u64,
    unavailable_count: u64,
    not_billable_count: u64,
}

impl From<&CostCoverage> for CostCoverageData {
    fn from(value: &CostCoverage) -> Self {
        Self {
            provider_reported_count: value.provider_reported_count,
            calculated_count: value.calculated_count,
            partial_count: value.partial_count,
            unavailable_count: value.unavailable_count,
            not_billable_count: value.not_billable_count,
        }
    }
}

fn usd_cost(costs: &[gateway_admin::model::observability::CurrencyCost]) -> String {
    costs
        .iter()
        .find(|cost| cost.currency == "USD")
        .map_or_else(|| "0".to_owned(), |cost| cost.amount.to_string())
}

const fn granularity(value: Granularity) -> &'static str {
    match value {
        Granularity::FifteenMinutes => "15m",
        Granularity::Hour => "hour",
        Granularity::Day => "day",
    }
}
