mod middleware;
mod route_schedule;

use std::{fmt, sync::Arc, time::Duration};

use gateway_admin::model::{
    AdminError,
    plugins::instances::{PluginCapabilityBinding, PluginFailurePolicy},
};
use gateway_core::engine::middleware::MiddlewareMount;
use gateway_plugin_sdk::{Capability, Manifest, Permission, Stage};

use crate::{RpcSession, adapter::scope::BindingScope, callback::PluginCallbacks};

const POLICY_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) enum PolicyEntry {
    Router(ModelRouterEntry),
    Scheduler(AccountSchedulerEntry),
    Middleware(MiddlewareEntry),
}

pub(crate) struct ModelRouterEntry {
    order: i32,
    plugin_id: String,
    instance_id: String,
    session: Arc<RpcSession>,
    callbacks: Arc<PluginCallbacks>,
    scope: BindingScope,
    failure_policy: PluginFailurePolicy,
    requests_authorized: bool,
}

pub(crate) struct AccountSchedulerEntry {
    plugin_id: String,
    instance_id: String,
    session: Arc<RpcSession>,
    callbacks: Arc<PluginCallbacks>,
    scope: BindingScope,
    failure_policy: PluginFailurePolicy,
    requests_authorized: bool,
}

pub(crate) struct MiddlewareEntry {
    order: i32,
    plugin_id: String,
    instance_id: String,
    mount: MiddlewareMount,
    session: Arc<RpcSession>,
    callbacks: Arc<PluginCallbacks>,
    scope: BindingScope,
    failure_policy: PluginFailurePolicy,
    requests_authorized: bool,
}

pub(crate) fn validate_bindings(
    manifest: &Manifest,
    bindings: &[PluginCapabilityBinding],
) -> Result<(), AdminError> {
    let mut has_router = false;
    let mut has_scheduler = false;
    let mut middleware_stages = std::collections::BTreeSet::new();
    for binding in bindings {
        let capability = crate::contribution::resolve(manifest, binding)?.capability;
        if !matches!(
            capability,
            Capability::ModelRouter | Capability::Scheduler | Capability::Middleware
        ) {
            continue;
        }
        let stage: Stage = serde_json::from_value(serde_json::Value::String(binding.stage.clone()))
            .map_err(|_| AdminError::invalid("插件请求阶段无效"))?;
        if !matches!(
            binding.failure_policy,
            PluginFailurePolicy::Reject | PluginFailurePolicy::Delegate
        ) {
            return Err(AdminError::invalid("数据面绑定的故障策略无效"));
        }
        match capability {
            Capability::ModelRouter => {
                if std::mem::replace(&mut has_router, true) || stage != Stage::Routing {
                    return Err(AdminError::invalid(
                        "同一插件实例的模型路由能力只能绑定一次且必须使用 routing 阶段",
                    ));
                }
            }
            Capability::Scheduler => {
                if std::mem::replace(&mut has_scheduler, true) || stage != Stage::Scheduling {
                    return Err(AdminError::invalid(
                        "同一插件实例的账号调度能力只能绑定一次且必须使用 scheduling 阶段",
                    ));
                }
            }
            Capability::Middleware => {
                if !matches!(stage, Stage::Request | Stage::Attempt)
                    || !middleware_stages.insert(stage)
                {
                    return Err(AdminError::invalid(
                        "同一插件实例的中间件只能在 request/attempt 各绑定一次",
                    ));
                }
            }
            _ => unreachable!("capability was filtered above"),
        }
        let scope = BindingScope::compile(binding)?;
        if (capability == Capability::ModelRouter
            || (capability == Capability::Middleware && stage == Stage::Request))
            && scope.has_provider_condition()
        {
            return Err(AdminError::invalid(
                "Provider 尚未冻结的阶段不能绑定 Provider 条件",
            ));
        }
    }
    Ok(())
}

pub(crate) fn compile_entries(
    manifest: &Manifest,
    instance_id: &str,
    bindings: &[PluginCapabilityBinding],
    permissions: &[Permission],
    session: Arc<RpcSession>,
    callbacks: Arc<PluginCallbacks>,
) -> Result<Vec<PolicyEntry>, AdminError> {
    validate_bindings(manifest, bindings)?;
    let plugin_id = manifest
        .plugin_id()
        .map_err(|_| AdminError::invalid("插件身份无效"))?;
    let requests_authorized = permissions.contains(&Permission::Requests);
    let mut entries = Vec::new();
    for binding in bindings {
        let capability = crate::contribution::resolve(manifest, binding)?.capability;
        if !matches!(
            capability,
            Capability::ModelRouter | Capability::Scheduler | Capability::Middleware
        ) {
            continue;
        }
        let scope = BindingScope::compile(binding)?;
        entries.push(match capability {
            Capability::ModelRouter => PolicyEntry::Router(ModelRouterEntry {
                order: binding.order,
                plugin_id: plugin_id.clone(),
                instance_id: instance_id.to_owned(),
                session: Arc::clone(&session),
                callbacks: Arc::clone(&callbacks),
                scope,
                failure_policy: binding.failure_policy.clone(),
                requests_authorized,
            }),
            Capability::Scheduler => PolicyEntry::Scheduler(AccountSchedulerEntry {
                plugin_id: plugin_id.clone(),
                instance_id: instance_id.to_owned(),
                session: Arc::clone(&session),
                callbacks: Arc::clone(&callbacks),
                scope,
                failure_policy: binding.failure_policy.clone(),
                requests_authorized,
            }),
            Capability::Middleware => {
                let stage: Stage =
                    serde_json::from_value(serde_json::Value::String(binding.stage.clone()))
                        .map_err(|_| AdminError::invalid("插件中间件阶段无效"))?;
                let mount = match stage {
                    Stage::Request => MiddlewareMount::Request,
                    Stage::Attempt => MiddlewareMount::Attempt,
                    _ => return Err(AdminError::invalid("插件中间件阶段无效")),
                };
                PolicyEntry::Middleware(MiddlewareEntry {
                    order: binding.order,
                    plugin_id: plugin_id.clone(),
                    instance_id: instance_id.to_owned(),
                    mount,
                    session: Arc::clone(&session),
                    callbacks: Arc::clone(&callbacks),
                    scope,
                    failure_policy: binding.failure_policy.clone(),
                    requests_authorized,
                })
            }
            _ => unreachable!("capability was filtered above"),
        });
    }
    Ok(entries)
}

pub(crate) struct PluginRequestPolicyPlan {
    routers: Arc<[ModelRouterEntry]>,
    schedulers: Arc<[AccountSchedulerEntry]>,
    middleware: Arc<[MiddlewareEntry]>,
    policy_timeout: Duration,
    middleware_timeout: Duration,
    maximum_payload_bytes: usize,
}

impl fmt::Debug for PluginRequestPolicyPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PluginRequestPolicyPlan")
            .field("router_count", &self.routers.len())
            .field("scheduler_count", &self.schedulers.len())
            .field("middleware_count", &self.middleware.len())
            .finish_non_exhaustive()
    }
}

impl PluginRequestPolicyPlan {
    pub(crate) fn compile(
        entries: Vec<PolicyEntry>,
        maximum_call_timeout: Duration,
        maximum_frame_bytes: usize,
    ) -> Result<Option<Arc<Self>>, AdminError> {
        let mut routers = Vec::new();
        let mut schedulers = Vec::new();
        let mut middleware = Vec::new();
        for entry in entries {
            match entry {
                PolicyEntry::Router(entry) => routers.push(entry),
                PolicyEntry::Scheduler(entry) => schedulers.push(entry),
                PolicyEntry::Middleware(entry) => middleware.push(entry),
            }
        }
        if routers.is_empty() && schedulers.is_empty() && middleware.is_empty() {
            return Ok(None);
        }
        routers.sort_by(|left, right| {
            (left.order, &left.plugin_id, &left.instance_id).cmp(&(
                right.order,
                &right.plugin_id,
                &right.instance_id,
            ))
        });
        for (index, left) in schedulers.iter().enumerate() {
            if schedulers[index + 1..]
                .iter()
                .any(|right| left.scope.overlaps(&right.scope))
            {
                return Err(AdminError::invalid("账号调度绑定作用范围重叠"));
            }
        }
        middleware.sort_by(|left, right| {
            (
                mount_order(left.mount),
                left.order,
                &left.plugin_id,
                &left.instance_id,
            )
                .cmp(&(
                    mount_order(right.mount),
                    right.order,
                    &right.plugin_id,
                    &right.instance_id,
                ))
        });
        Ok(Some(Arc::new(Self {
            routers: routers.into(),
            schedulers: schedulers.into(),
            middleware: middleware.into(),
            policy_timeout: maximum_call_timeout.min(POLICY_TIMEOUT),
            middleware_timeout: maximum_call_timeout,
            maximum_payload_bytes: maximum_frame_bytes,
        })))
    }

    #[must_use]
    pub(crate) fn has_request_policy(&self) -> bool {
        !self.routers.is_empty() || !self.schedulers.is_empty()
    }

    #[must_use]
    pub(crate) fn has_middleware(&self) -> bool {
        !self.middleware.is_empty()
    }
}

const fn mount_order(mount: MiddlewareMount) -> u8 {
    match mount {
        MiddlewareMount::Request => 0,
        MiddlewareMount::Attempt => 1,
    }
}
