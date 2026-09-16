//! Admin query service 使用的账号运行态组合 adapter。

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::future::join_all;
use gateway_admin::{
    model::accounts::AccountRuntimeSnapshot,
    ports::store::{AccountRuntimeStore, AdminStoreResult},
};

use super::{
    CredentialCooldownRepository as _, CredentialLeaseRepository as _,
    RedisCredentialCooldownRepository, RedisCredentialLeaseRepository,
};
use crate::Revision;

/// 只组合可丢失 Redis 事实；不持有 PostgreSQL，也不执行状态投影。
#[derive(Clone)]
pub struct RedisAdminAccountRuntimeStore {
    cooldowns: RedisCredentialCooldownRepository,
    leases: RedisCredentialLeaseRepository,
}

impl RedisAdminAccountRuntimeStore {
    #[must_use]
    pub const fn new(
        cooldowns: RedisCredentialCooldownRepository,
        leases: RedisCredentialLeaseRepository,
    ) -> Self {
        Self { cooldowns, leases }
    }
}

#[async_trait]
impl AccountRuntimeStore for RedisAdminAccountRuntimeStore {
    async fn active_rate_limits(&self) -> AdminStoreResult<AccountRuntimeSnapshot> {
        self.cooldowns
            .active_cooldowns()
            .await
            .map_err(|error| crate::admin_store_error("account runtime", error))
    }

    async fn account_runtime(
        &self,
        account_ids: &[String],
    ) -> AdminStoreResult<AccountRuntimeSnapshot> {
        let reads = account_ids.iter().map(|account_id| async move {
            self.cooldowns
                .read_credential_cooldown(account_id)
                .await
                .map(|cooldown| {
                    cooldown.map(|cooldown| (account_id.clone(), cooldown.cooldown_until))
                })
        });
        let mut rate_limited_until = BTreeMap::new();
        for result in join_all(reads).await {
            if let Some((account_id, until)) =
                result.map_err(|error| crate::admin_store_error("account runtime", error))?
            {
                rate_limited_until.insert(account_id, until);
            }
        }
        let in_flight = self
            .leases
            .credential_runtime_signals(account_ids)
            .await
            .ok()
            .map(|signals| {
                signals
                    .into_iter()
                    .map(|signal| (signal.resource_id, u64::from(signal.in_flight)))
                    .collect()
            });
        Ok(AccountRuntimeSnapshot {
            rate_limited_until,
            in_flight,
        })
    }

    async fn active_freezes(&self) -> AdminStoreResult<BTreeMap<String, DateTime<Utc>>> {
        self.cooldowns
            .active_freezes()
            .await
            .map_err(|error| crate::admin_store_error("account runtime", error))
    }

    async fn capacity_peaks(
        &self,
        account_ids: &[String],
    ) -> AdminStoreResult<BTreeMap<String, u32>> {
        let reads = account_ids.iter().map(|account_id| async move {
            self.cooldowns
                .read_capacity_peak(account_id)
                .await
                .map(|peak| peak.map(|value| (account_id.clone(), value)))
        });
        let mut peaks = BTreeMap::new();
        for result in join_all(reads).await {
            if let Some((account_id, peak)) =
                result.map_err(|error| crate::admin_store_error("account runtime", error))?
            {
                peaks.insert(account_id, peak);
            }
        }
        Ok(peaks)
    }

    async fn clear_rate_limit(
        &self,
        account_id: &str,
        through_revision: gateway_admin::model::Revision,
    ) -> AdminStoreResult<bool> {
        let revision = Revision::new(through_revision.get())
            .map_err(|error| crate::admin_store_error("account runtime", error))?;
        self.cooldowns
            .invalidate_credential_cooldown(account_id, revision)
            .await
            .map_err(|error| crate::admin_store_error("account runtime", error))
    }

    async fn extend_rate_limit(
        &self,
        account_id: &str,
        through_revision: gateway_admin::model::Revision,
        until: DateTime<Utc>,
    ) -> AdminStoreResult<bool> {
        let invalid = |message: &str| {
            crate::admin_store_error(
                "account runtime",
                crate::StoreError::InvalidData {
                    entity: "account runtime",
                    message: message.to_owned(),
                },
            )
        };
        let account_id = gateway_core::account::ProviderAccountId::new(account_id.to_owned())
            .map_err(|_| invalid("invalid provider account ID"))?;
        let revision = gateway_core::account::CredentialRevision::new(through_revision.get())
            .map_err(|_| invalid("invalid credential revision"))?;
        let cooldown = gateway_core::provider_ports::ProviderCooldown::new_with_kind(
            account_id,
            revision,
            until.into(),
            gateway_core::provider_ports::ProviderCooldownKind::CapacityFreeze,
        );
        use gateway_core::provider_ports::ProviderCooldownPort as _;
        self.cooldowns.put_if_later(cooldown).await.map_err(|_| {
            crate::admin_store_error(
                "account runtime",
                crate::StoreError::Unavailable {
                    backend: crate::StoreBackend::Redis,
                    message: "extend account rate limit".to_owned(),
                },
            )
        })
    }
}
