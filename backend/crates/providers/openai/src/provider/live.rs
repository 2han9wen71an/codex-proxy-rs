//! Codex Live 语音会话：SDP 引导 attempt、call 会话注册表与账号级 sideband 网关。
//!
//! 通话引导（`POST /v1/live`）复用 Provider HTTP 端点通道：Core 路由按客户端
//! Key 的账号范围选路，Provider 把固定端点 `codex/realtime/calls` 映射到
//! Codex backend，并把成功响应 `Location` 里的 call id 登记到进程内注册表。
//! 后续 sideband / hangup 由 [`CodexLiveGateway`] 用钉住账号的凭据直连
//! `api.openai.com`；音频不经过网关。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use bytes::Bytes;
use gateway_core::engine::provider::{ProviderSelectionObservation, ProviderStream};
use gateway_core::error::{ProviderError, ProviderErrorKind};
use gateway_core::event::{
    GatewayEvent, ProtocolWireEvent, ProviderEvent, ProviderResponseHeader,
    ProviderResponseObservation, ResponseMeta,
};
use gateway_core::live::{
    LiveCallOutcome, LiveGateway, LiveGatewayError, LiveGatewayErrorKind, LiveHangupRequest,
    LiveRelay, LiveSidebandRequest, LiveSidebandStyle, call_id_from_location, is_valid_call_id,
};
use gateway_core::operation::{ProviderHttpMethod, ProviderHttpRequest};
use gateway_core::policy::ClientApiKeyId;
use gateway_core::upstream::{UpstreamSendState, UpstreamTransport};
use secrecy::ExposeSecret;
use serde_json::Value;
use url::Url;
use uuid::Uuid;

use super::*;

use crate::credential::{
    CODEX_AUTHENTICATION_KIND_OAUTH, CodexCredentialRepository,
    SelectCodexProviderEndpointCredential,
};
use crate::transport::websocket::{connect_live_sideband, into_live_relay};
use crate::transport::{CODEX_REALTIME_CALLS_PATH, CodexRequestContext};

/// realtime calls 在 [`ProviderHttpRequest::endpoint`] 中的符号名；
/// Provider 将其映射到固定上游路径 [`CODEX_REALTIME_CALLS_PATH`]。
pub(super) const LIVE_CALLS_ENDPOINT: &str = "realtime-calls";
/// sideband / hangup 的 OpenAI 公共 API 基址；与通话引导的 chatgpt.com backend 不同。
const OPENAI_API_WS_BASE: &str = "wss://api.openai.com/v1";
const OPENAI_API_HTTP_BASE: &str = "https://api.openai.com/v1";
/// call id 与创建会话的绑定寿命；过期后 sideband 无法再加入。
const LIVE_SESSION_TTL: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveClaimError {
    Missing,
    Busy,
    OwnerMismatch,
    Invalid,
}

impl LiveClaimError {
    fn kind(self) -> LiveGatewayErrorKind {
        match self {
            Self::Missing => LiveGatewayErrorKind::CallNotFound,
            Self::Busy => LiveGatewayErrorKind::CallBusy,
            Self::OwnerMismatch => LiveGatewayErrorKind::OwnerMismatch,
            Self::Invalid => LiveGatewayErrorKind::InvalidCallId,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Missing => "codex live call not found",
            Self::Busy => "codex live call already has a sideband connection",
            Self::OwnerMismatch => "codex live call belongs to another API key",
            Self::Invalid => "invalid codex live call id",
        }
    }
}

struct LiveRegistryEntry {
    account_id: gateway_core::account::ProviderAccountId,
    client_api_key_id: ClientApiKeyId,
    expires_at: Option<SystemTime>,
    claimed: bool,
}

/// 进程内 call id → 创建账号注册表。
///
/// 注册表只服务本进程的 sideband 钉住与身份复验；网关重启后未完成的语音
/// 通话失去 sideband 能力，与上游会话自身的存活一致，不需要持久化。
#[derive(Default)]
pub(crate) struct CodexLiveRegistry {
    entries: Mutex<HashMap<String, LiveRegistryEntry>>,
}

impl CodexLiveRegistry {
    fn sweep_expired(entries: &mut HashMap<String, LiveRegistryEntry>, now: SystemTime) {
        entries.retain(|_, entry| {
            entry.claimed || entry.expires_at.is_none_or(|expires_at| expires_at > now)
        });
    }

    /// 登记成功创建的通话；call id 非法时忽略。
    pub(crate) fn register(
        &self,
        call_id: &str,
        account_id: gateway_core::account::ProviderAccountId,
        client_api_key_id: ClientApiKeyId,
    ) {
        if !is_valid_call_id(call_id) {
            return;
        }
        let mut entries = self.entries.lock().expect("codex live registry poisoned");
        Self::sweep_expired(&mut entries, SystemTime::now());
        entries.insert(
            call_id.to_owned(),
            LiveRegistryEntry {
                account_id,
                client_api_key_id,
                expires_at: Some(SystemTime::now() + LIVE_SESSION_TTL),
                claimed: false,
            },
        );
    }

    /// 认领 call：校验调用方并阻止并发 sideband；认领后暂停过期。
    fn claim(
        &self,
        call_id: &str,
        client_api_key_id: &ClientApiKeyId,
    ) -> Result<gateway_core::account::ProviderAccountId, LiveClaimError> {
        if !is_valid_call_id(call_id) {
            return Err(LiveClaimError::Invalid);
        }
        let mut entries = self.entries.lock().expect("codex live registry poisoned");
        Self::sweep_expired(&mut entries, SystemTime::now());
        let Some(entry) = entries.get_mut(call_id) else {
            return Err(LiveClaimError::Missing);
        };
        if entry.claimed {
            return Err(LiveClaimError::Busy);
        }
        if &entry.client_api_key_id != client_api_key_id {
            return Err(LiveClaimError::OwnerMismatch);
        }
        entry.claimed = true;
        // 认领重置 TTL：正常通话由 complete_call 结算；upgrade 回调未执行的
        // 罕见窗口也保证条目最终被回收，不会永久占用 call id。
        entry.expires_at = Some(SystemTime::now() + LIVE_SESSION_TTL);
        Ok(entry.account_id.clone())
    }

    /// 归还认领（拨号失败时调用）；过期计时恢复。
    fn release(&self, call_id: &str) {
        let mut entries = self.entries.lock().expect("codex live registry poisoned");
        if let Some(entry) = entries.get_mut(call_id) {
            entry.claimed = false;
            entry.expires_at = Some(SystemTime::now() + LIVE_SESSION_TTL);
        }
    }

    fn complete(&self, call_id: &str) {
        let mut entries = self.entries.lock().expect("codex live registry poisoned");
        entries.remove(call_id);
    }

    /// hangup 前的身份复验；不认领，允许与 sideband 并存。
    fn peek_owner(
        &self,
        call_id: &str,
        client_api_key_id: &ClientApiKeyId,
    ) -> Result<gateway_core::account::ProviderAccountId, LiveClaimError> {
        if !is_valid_call_id(call_id) {
            return Err(LiveClaimError::Invalid);
        }
        let mut entries = self.entries.lock().expect("codex live registry poisoned");
        Self::sweep_expired(&mut entries, SystemTime::now());
        let Some(entry) = entries.get_mut(call_id) else {
            return Err(LiveClaimError::Missing);
        };
        if entry.claimed {
            return Err(LiveClaimError::Busy);
        }
        if &entry.client_api_key_id != client_api_key_id {
            return Err(LiveClaimError::OwnerMismatch);
        }
        Ok(entry.account_id.clone())
    }
}

impl CodexProvider {
    /// 引擎 arm：把受限的 realtime calls Provider HTTP 操作发往 Codex backend。
    pub(super) async fn execute_live_call(
        self: Arc<Self>,
        request: ProviderHttpRequest,
        context: AttemptContext,
    ) -> Result<ProviderStream, ProviderError> {
        if request.method() != ProviderHttpMethod::Post || request.endpoint() != LIVE_CALLS_ENDPOINT
        {
            return Err(provider_error(
                ProviderErrorKind::Unsupported,
                UpstreamSendState::NotSent,
            ));
        }
        let selection_started_at = Instant::now();
        let lease = self
            .selector
            .select_for_provider_endpoint(&SelectCodexProviderEndpointCredential {
                request_url: &self.live_calls_url,
                attempt: &context,
                session_affinity: None,
            })
            .await
            .map_err(map_selection_error)?;
        if lease.authentication().oauth().is_none() {
            // realtime calls 端点绑定 ChatGPT OAuth 身份；API Key 凭据不在本端点范围。
            return Err(provider_error(
                ProviderErrorKind::Unsupported,
                UpstreamSendState::NotSent,
            ));
        }
        let account_selection_wait_ms =
            u64::try_from(selection_started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        let lease = Arc::new(lease);
        let operation = Operation::ProviderHttp(request);
        let provider_kind = ProviderKind::new(PROVIDER_NAME)
            .map_err(|_| provider_error(ProviderErrorKind::Protocol, UpstreamSendState::NotSent))?;
        let account_id = lease.account_id().clone();
        let provider = Arc::clone(&self);
        let terminal_context = context.clone();
        context
            .execute_middleware(
                operation,
                provider_kind,
                None,
                account_id,
                Box::new(move |operation, middleware_headers| {
                    Box::pin(async move {
                        provider
                            .execute_selected_live_call(
                                terminal_context,
                                operation,
                                middleware_headers,
                                lease,
                                account_selection_wait_ms,
                            )
                            .await
                    })
                }),
            )
            .await
    }

    async fn execute_selected_live_call(
        self: Arc<Self>,
        context: AttemptContext,
        operation: Operation,
        middleware_headers: Vec<MiddlewareHeader>,
        lease: Arc<CodexCredentialLease>,
        account_selection_wait_ms: u64,
    ) -> Result<ProviderStream, ProviderError> {
        let Operation::ProviderHttp(request) = operation else {
            return Err(provider_error(
                ProviderErrorKind::InvalidRequest,
                UpstreamSendState::NotSent,
            ));
        };
        let allows_account_state_mutation = lease.allows_account_state_mutation();
        let provider_kind = ProviderKind::new(PROVIDER_NAME)
            .map_err(|_| provider_error(ProviderErrorKind::Protocol, UpstreamSendState::NotSent))?;
        let metadata = ProviderCallMetadata::for_provider_endpoint(
            provider_kind,
            lease.account_id().clone(),
            UpstreamTransport::new(HTTP_JSON_TRANSPORT).map_err(|_| {
                provider_error(ProviderErrorKind::Protocol, UpstreamSendState::NotSent)
            })?,
        )
        .with_selection_observation(ProviderSelectionObservation::new(
            account_selection_wait_ms,
            lease.capacity_snapshot(),
        ));
        let content_type = request
            .headers()
            .iter()
            .find(|header| header.name().eq_ignore_ascii_case("content-type"))
            .and_then(|header| std::str::from_utf8(header.value()).ok())
            .map(str::to_owned);
        let protocol_headers = request
            .headers()
            .iter()
            .filter(|header| !header.name().eq_ignore_ascii_case("content-type"))
            .map(|header| {
                (
                    header.name().to_ascii_lowercase(),
                    String::from_utf8_lossy(header.value()).into_owned(),
                )
            })
            .collect::<Vec<_>>();
        let requested_model = live_requested_model(request.payload().body());
        let events = cold_live_call_stream(ColdLiveCall {
            client: self
                .client_for_request(&context)?
                .for_account(lease.account())
                .map_err(|_| {
                    provider_error(ProviderErrorKind::Unavailable, UpstreamSendState::NotSent)
                })?
                .with_authentication(lease.authentication())
                .with_middleware_headers(middleware_headers),
            registry: Arc::clone(&self.live_registry),
            response_origin: self.live_calls_url.clone(),
            endpoint_query: request.query().map(str::to_owned),
            content_type,
            protocol_headers,
            requested_model,
            body: request.payload().body().clone(),
            context,
            selector: Arc::clone(&self.selector),
            quota: Arc::clone(&self.quota),
            lease: Arc::clone(&lease),
        });
        let stream = ProviderStream::new(metadata, events, lease);
        Ok(if allows_account_state_mutation {
            stream.with_filtered_account_feedback(
                Arc::clone(&self.account_feedback),
                openai_failure_affects_account_score,
            )
        } else {
            stream
        })
    }
}

struct ColdLiveCall {
    client: CodexBackendClient,
    registry: Arc<CodexLiveRegistry>,
    response_origin: Url,
    endpoint_query: Option<String>,
    content_type: Option<String>,
    protocol_headers: Vec<(String, String)>,
    requested_model: Option<String>,
    body: Bytes,
    context: AttemptContext,
    selector: Arc<CodexCredentialSelector>,
    quota: Arc<CodexCredentialQuotaService>,
    lease: Arc<CodexCredentialLease>,
}

/// 从标准化后的引导请求提取模型名；请求体恒为 `{"sdp": …, "session"?: …}` JSON。
fn live_requested_model(body: &[u8]) -> Option<String> {
    let payload = serde_json::from_slice::<Value>(body).ok()?;
    let extract = |value: Option<&Value>| -> Option<String> {
        value
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .map(str::to_owned)
    };
    extract(payload.pointer("/session/model")).or_else(|| extract(payload.get("model")))
}

fn cold_live_call_stream(request: ColdLiveCall) -> EventStream {
    Box::pin(async_stream::try_stream! {
        let requested_model = request.requested_model.clone();
        let allows_account_state_mutation = request.lease.allows_account_state_mutation();
        let failure_context = OpenAiFailureContext {
            client: &request.client,
            selector: &request.selector,
            quota: &request.quota,
            response_origin: &request.response_origin,
            cyber_policy_scope: None,
            allows_account_state_mutation,
            allows_capacity_feedback: !request.context.is_diagnostic_required_account(),
        };
        let active_account = request.lease.account().clone();
        let authorization = request
            .lease
            .authentication()
            .authorization_header()
            .map_err(|_| {
                provider_error(ProviderErrorKind::Unauthorized, UpstreamSendState::NotSent)
            })?;
        let request_id = request.context.request_id().as_str().to_owned();
        let trace = request.context.trace();
        let mut request_context = CodexRequestContext::auxiliary(
            authorization.expose_secret(),
            active_account.upstream_account_id(),
            &request_id,
            Some(request.lease.installation_id()),
        );
        request_context.trace = Some(&trace);
        request_context.account_selection = CodexAccountSelectionTelemetry::new(
            request.lease.affinity_hit(),
            request.lease.escape_reason(),
            request.lease.account_switch(),
        );

        let Some(handshake_deadline) = remaining(request.context.deadline()) else {
            Err(provider_error(ProviderErrorKind::Timeout, UpstreamSendState::NotSent))?;
            return;
        };
        let cancellation = request.context.cancellation().clone();
        let attempt = tokio::select! {
            biased;
            _ = cancellation.cancelled() => Err(CodexHandshakeAttemptError::Cancelled),
            _ = tokio::time::sleep(handshake_deadline) => Err(CodexHandshakeAttemptError::Timeout),
            response = request.client.post_live_call(
                CODEX_REALTIME_CALLS_PATH,
                request.endpoint_query.as_deref(),
                request.content_type.as_deref(),
                &request.protocol_headers,
                request.body.clone(),
                request_context,
            ) => response.map_err(CodexHandshakeAttemptError::Client),
        };
        if let Err(CodexHandshakeAttemptError::Client(error)) = &attempt {
            log_client_upstream_error(
                UpstreamErrorLogContext::new(&request.context, &active_account, None),
                error,
            );
        }
        let response = match attempt.map_err(map_handshake_attempt_error) {
            Ok(response) => response,
            Err(mut failure) => {
                if let Some(observation) = failure.observation.take() {
                    yield ProviderEvent::observation(observation);
                }
                apply_failure(&failure_context, &active_account, &failure).await;
                Err(failure.error)?;
                return;
            }
        };

        let observation = ProviderResponseObservation::new(
            UpstreamTransport::new(HTTP_JSON_TRANSPORT).map_err(|_| {
                provider_error(ProviderErrorKind::Protocol, UpstreamSendState::Sent)
            })?,
        )
        .with_status_code(response.status)
        .with_client_headers(
            response
                .forwarded_headers
                .iter()
                .map(|(name, value)| {
                    ProviderResponseHeader::new(
                        name.clone(),
                        Bytes::copy_from_slice(value.as_bytes()),
                    )
                })
                .collect(),
        );
        yield ProviderEvent::observation(observation);

        if let Some(location) = response.location.as_deref()
            && let Some(call_id) = call_id_from_location(location)
        {
            request.registry.register(
                &call_id,
                active_account.id().clone(),
                request.context.client_api_key_ref().clone(),
            );
        } else {
            tracing::warn!(
                request_id = %request_id,
                model = requested_model.as_deref().unwrap_or_default(),
                "codex live call response is missing a parseable Location header"
            );
        }

        let response_meta = ResponseMeta::for_provider_endpoint(&request_id);
        yield ProviderEvent::canonical(GatewayEvent::Started(response_meta.clone()));
        let wire = ProtocolWireEvent::raw_http_body(PROVIDER_NAME, response.body).map_err(|_| {
            provider_error(ProviderErrorKind::Protocol, UpstreamSendState::Sent)
        })?;
        yield ProviderEvent::wire(wire);
        yield ProviderEvent::canonical(GatewayEvent::Completed(
            response_meta.with_finish_reason(FinishReason::Stop),
        ));
    })
}

/// Core [`LiveGateway`] 的 Codex 实现：call 钉住账号 + 账号级凭据拨号。
pub(crate) struct CodexLiveGateway {
    registry: Arc<CodexLiveRegistry>,
    repository: CodexCredentialRepository,
    client: CodexBackendClient,
}

impl CodexLiveGateway {
    pub(crate) fn new(
        registry: Arc<CodexLiveRegistry>,
        repository: CodexCredentialRepository,
        client: CodexBackendClient,
    ) -> Self {
        Self {
            registry,
            repository,
            client,
        }
    }

    /// 解析钉住账号的运行时凭据；非 OAuth 账号不支持 realtime 端点。
    async fn load_pinned_credential(
        &self,
        account_id: &gateway_core::account::ProviderAccountId,
    ) -> Result<(ProviderAccount, String), LiveGatewayError> {
        let credential_unavailable = |message: &'static str| {
            LiveGatewayError::new(LiveGatewayErrorKind::CredentialUnavailable, message)
        };
        let account = self
            .repository
            .store()
            .get_account(account_id)
            .await
            .map_err(|_| credential_unavailable("codex live pinned account is unavailable"))?
            .ok_or_else(|| credential_unavailable("codex live pinned account is unavailable"))?;
        if account.authentication_kind() != CODEX_AUTHENTICATION_KIND_OAUTH {
            return Err(LiveGatewayError::new(
                LiveGatewayErrorKind::Unsupported,
                "codex live requires an OAuth codex account",
            ));
        }
        let credential = self
            .repository
            .load_runtime_credential(&account)
            .await
            .map_err(|_| credential_unavailable("codex live pinned credential is unavailable"))?;
        let authorization = credential
            .authentication
            .authorization_header()
            .map_err(|_| credential_unavailable("codex live pinned credential is unusable"))?;
        Ok((account, authorization.expose_secret().to_owned()))
    }

    fn sideband_endpoint(style: LiveSidebandStyle, call_id: &str) -> String {
        match style {
            LiveSidebandStyle::Live => format!("{OPENAI_API_WS_BASE}/live/{call_id}"),
            LiveSidebandStyle::RealtimeCalls => {
                format!("{OPENAI_API_WS_BASE}/realtime/calls/{call_id}")
            }
            LiveSidebandStyle::RealtimeQuery => {
                format!("{OPENAI_API_WS_BASE}/realtime?intent=quicksilver&call_id={call_id}")
            }
        }
    }
}

fn map_sideband_dial_error(error: CodexWebSocketExchangeError) -> LiveGatewayError {
    match error {
        CodexWebSocketExchangeError::Upstream(upstream) => {
            let body = upstream
                .client_response
                .as_ref()
                .map(|response| response.body().clone());
            LiveGatewayError::new(
                LiveGatewayErrorKind::UpstreamUnavailable,
                format!("codex live sideband handshake failed: {}", upstream.body),
            )
            .with_upstream(upstream.status_code, body)
        }
        other => LiveGatewayError::new(
            LiveGatewayErrorKind::UpstreamUnavailable,
            format!("codex live sideband upstream unavailable: {other}"),
        ),
    }
}

fn map_live_http_error(error: CodexClientError) -> LiveGatewayError {
    match error {
        CodexClientError::Upstream {
            status: _,
            client_response: Some(response),
            ..
        } => LiveGatewayError::new(
            LiveGatewayErrorKind::UpstreamUnavailable,
            format!(
                "codex live upstream call failed with status {}",
                response.status()
            ),
        )
        .with_upstream(response.status(), Some(response.body().clone())),
        CodexClientError::Upstream { status, body, .. } => LiveGatewayError::new(
            LiveGatewayErrorKind::UpstreamUnavailable,
            format!(
                "codex live upstream call failed with status {}",
                status.as_u16()
            ),
        )
        .with_upstream(status.as_u16(), Some(Bytes::from(body))),
        other => LiveGatewayError::new(
            LiveGatewayErrorKind::UpstreamUnavailable,
            format!("codex live upstream call failed: {other}"),
        ),
    }
}

fn host_header(endpoint: &str) -> (String, String) {
    let url = reqwest::Url::parse(endpoint).expect("live sideband endpoint is a valid URL");
    let host = url.host_str().unwrap_or_default();
    let authority = match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    };
    ("host".to_owned(), authority)
}

impl LiveGateway for CodexLiveGateway {
    fn open_sideband<'a>(
        &'a self,
        request: LiveSidebandRequest<'a>,
    ) -> BoxFuture<'a, Result<LiveRelay, LiveGatewayError>> {
        Box::pin(async move {
            let account_id = self
                .registry
                .claim(request.call_id, request.client_api_key_id)
                .map_err(|claim_error| {
                    LiveGatewayError::new(claim_error.kind(), claim_error.message())
                })?;
            let (account, authorization) = match self.load_pinned_credential(&account_id).await {
                Ok(outcome) => outcome,
                Err(error) => {
                    self.registry.release(request.call_id);
                    return Err(error);
                }
            };
            let endpoint = Self::sideband_endpoint(request.style, request.call_id);
            let mut headers = vec![host_header(&endpoint)];
            headers.push(("authorization".to_owned(), authorization));
            if let Some(account_id) = account.upstream_account_id() {
                headers.push(("chatgpt-account-id".to_owned(), account_id.to_owned()));
            }
            headers.extend(request.protocol_headers);
            if !request.subprotocols.is_empty() {
                headers.push((
                    "sec-websocket-protocol".to_owned(),
                    request.subprotocols.join(", "),
                ));
            }
            match connect_live_sideband(endpoint, headers, account.outbound_proxy().cloned()).await
            {
                Ok(sideband) => Ok(into_live_relay(sideband.stream, sideband.subprotocol)),
                Err(error) => {
                    self.registry.release(request.call_id);
                    Err(map_sideband_dial_error(error))
                }
            }
        })
    }

    fn hangup<'a>(
        &'a self,
        request: LiveHangupRequest<'a>,
    ) -> BoxFuture<'a, Result<LiveCallOutcome, LiveGatewayError>> {
        Box::pin(async move {
            let account_id = self
                .registry
                .peek_owner(request.call_id, request.client_api_key_id)
                .map_err(|claim_error| {
                    LiveGatewayError::new(claim_error.kind(), claim_error.message())
                })?;
            let (account, authorization) = self.load_pinned_credential(&account_id).await?;
            let request_id = Uuid::new_v4().to_string();
            let mut context = CodexRequestContext::auxiliary(
                authorization.as_str(),
                account.upstream_account_id(),
                &request_id,
                None,
            );
            context.account_selection = CodexAccountSelectionTelemetry::NONE;
            let url = format!(
                "{OPENAI_API_HTTP_BASE}/realtime/calls/{}/hangup",
                request.call_id
            );
            let outcome = self
                .client
                .post_live_hangup(
                    url,
                    request.content_type.as_deref(),
                    &request.protocol_headers,
                    request.body.clone(),
                    context,
                )
                .await;
            match outcome {
                Ok(response) => {
                    if (200..300).contains(&response.status) {
                        self.registry.complete(request.call_id);
                    }
                    Ok(LiveCallOutcome {
                        status: response.status,
                        headers: response.forwarded_headers,
                        body: response.body,
                    })
                }
                Err(error) => Err(map_live_http_error(error)),
            }
        })
    }

    fn complete_call(&self, call_id: &str, _reason: &str) {
        self.registry.complete(call_id);
    }
}
