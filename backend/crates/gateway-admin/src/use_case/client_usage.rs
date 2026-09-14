//! 强制会话 Key 范围的自助用量查询；会话生命周期归属 AuthService。

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;

use crate::{
    AuthService,
    model::{
        AdminError, AdminErrorKind,
        auth::SessionSubject,
        client_usage::{
            ClientUsageKey, ClientUsageQuery, ClientUsageRecordsPage, ClientUsageSnapshot,
        },
        observability::{TimeRange, UsageFilter},
    },
    ports::store::{ClientUsageStore, ObservabilityStore},
};

use super::map_store_error;

#[async_trait]
pub trait ClientUsageService: Send + Sync {
    async fn usage(
        &self,
        session_id: Option<&str>,
        range: TimeRange,
    ) -> Result<Option<ClientUsageSnapshot>, AdminError>;
    async fn records(
        &self,
        session_id: Option<&str>,
        query: ClientUsageQuery,
    ) -> Result<Option<ClientUsageRecordsPage>, AdminError>;
}

pub(crate) struct DefaultClientUsageService {
    auth: Arc<dyn AuthService>,
    store: Arc<dyn ClientUsageStore>,
    observability: Arc<dyn ObservabilityStore>,
}

impl DefaultClientUsageService {
    pub(crate) fn new(
        auth: Arc<dyn AuthService>,
        store: Arc<dyn ClientUsageStore>,
        observability: Arc<dyn ObservabilityStore>,
    ) -> Self {
        Self {
            auth,
            store,
            observability,
        }
    }

    async fn resolve_key(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ClientUsageKey>, AdminError> {
        let Some(session) = self.auth.session(session_id).await? else {
            return Ok(None);
        };
        let SessionSubject::Key { client_key_id } = session.subject else {
            return Err(AdminError::new(
                AdminErrorKind::Forbidden,
                "当前身份无权访问密钥用量接口",
            ));
        };
        self.store
            .load_client_usage_key(&client_key_id)
            .await
            .map_err(|error| map_store_error(error, "client usage key"))
    }
}

#[async_trait]
impl ClientUsageService for DefaultClientUsageService {
    async fn usage(
        &self,
        session_id: Option<&str>,
        range: TimeRange,
    ) -> Result<Option<ClientUsageSnapshot>, AdminError> {
        let Some(key) = self.resolve_key(session_id).await? else {
            return Ok(None);
        };
        let filter = UsageFilter {
            client_api_key_ref: Some(key.id.as_str().to_owned()),
            ..UsageFilter::default()
        };
        let (overview, trend) = futures::try_join!(
            self.observability.usage_summary(range, filter.clone()),
            self.observability.usage_trend(range, filter),
        )
        .map_err(|error| map_store_error(error, "client usage activity"))?;
        Ok(Some(ClientUsageSnapshot {
            as_of: key.observed_at,
            key,
            range,
            overview,
            trend,
        }))
    }

    async fn records(
        &self,
        session_id: Option<&str>,
        query: ClientUsageQuery,
    ) -> Result<Option<ClientUsageRecordsPage>, AdminError> {
        let Some(key) = self.resolve_key(session_id).await? else {
            return Ok(None);
        };
        if query.range.end <= query.range.start
            || query.range.end - query.range.start > Duration::days(30)
            || !(1..=100).contains(&query.limit)
        {
            return Err(AdminError::invalid("调用明细查询范围无效"));
        }
        self.store
            .list_client_usage_records(&key.id, query)
            .await
            .map(Some)
            .map_err(|error| map_store_error(error, "client usage records"))
    }
}
