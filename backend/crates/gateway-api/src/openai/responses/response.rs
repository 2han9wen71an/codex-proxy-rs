//! Provider OpenAI Responses wire 到客户端 transport 的透明转发边界。

use bytes::Bytes;
use gateway_core::event::{GatewayEvent, ProtocolWireEvent, ProviderEvent};
use gateway_protocol::openai::sse::{
    encode_sse_event_with_metadata, response_failed_sse_event_with_id,
};
use serde_json::Value;

use super::error::ResponseEncodeError;

const OPENAI_PROTOCOL: &str = "openai";

/// OpenAI Responses wire 的转发器。
///
/// OpenAI Provider 与 xAI adapter 都必须交付 OpenAI wire。wire 是客户端交付
/// 与终态判断的事实来源；canonical facts 只在 wire 暂未携带 ID 时提供旁路观测，
/// 不得反过来拒绝或改写上游事件。
#[derive(Debug, Default)]
pub struct OpenAiResponsesEncoder {
    response_id: Option<String>,
    wire_terminal: Option<Value>,
    wire_failure: bool,
}

impl OpenAiResponsesEncoder {
    /// 创建响应 wire 转发器。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            response_id: None,
            wire_terminal: None,
            wire_failure: false,
        }
    }

    /// 消费一个 Provider event，并返回 SSE frames。
    pub fn push_sse(&mut self, event: &ProviderEvent) -> Vec<Bytes> {
        self.observe_canonical_identity(event);
        let Some(wire) = openai_wire(event) else {
            return Vec::new();
        };
        self.observe_wire(wire);
        // SSE 协议没有 `error` 事件：那是 WS 上游协议的错误形态。codex 等 SSE
        // 客户端会把未知事件类型静默忽略，裸转发只会让客户端看到无原因的
        // 流 EOF。翻译成 SSE 协议的 `response.failed`，保留上游错误负载。
        if wire.event_type() == Some("error")
            && let Some(frame) = self.response_failed_from_error_wire(wire)
        {
            return vec![Bytes::from(frame)];
        }
        if let Some(raw_sse_frame) = wire.raw_sse_frame() {
            return vec![raw_sse_frame.clone()];
        }
        vec![Bytes::from(encode_sse_event_with_metadata(
            wire.event_type().unwrap_or_default(),
            &wire.data().to_string(),
            wire.sse_id(),
            wire.sse_retry(),
        ))]
    }

    /// 把 WS 上游的 `error` 终止帧翻译成 SSE 协议的 `response.failed` 事件。
    ///
    /// 上游错误负载原样保留；response id 沿用已交付的 `response.created`，
    /// 缺失时由编码器回退随机 id，保证事件形状仍是合法的 SSE 终态。
    fn response_failed_from_error_wire(&self, wire: &ProtocolWireEvent) -> Option<String> {
        let error = match wire.data().get("error") {
            Some(error) if !error.is_null() => error,
            _ => return None,
        };
        let error_type = error
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unavailable");
        let code = error
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("upstream_error");
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("upstream request failed");
        Some(response_failed_sse_event_with_id(
            self.response_id.as_deref(),
            error_type,
            code,
            message,
        ))
    }

    /// 消费一个 Provider event，并返回 WebSocket JSON messages。
    pub fn push_websocket(&mut self, event: &ProviderEvent) -> Vec<String> {
        self.observe_canonical_identity(event);
        let Some(wire) = openai_wire(event).filter(|wire| wire.has_json_data()) else {
            return Vec::new();
        };
        self.observe_wire(wire);
        vec![wire.data().to_string()]
    }

    /// 返回是否已经看到客户端可见的 wire 终态。
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.wire_terminal.is_some()
    }

    /// 返回是否已把 Provider 原生失败 event 交付给客户端。
    #[must_use]
    pub const fn has_wire_failure(&self) -> bool {
        self.wire_failure
    }

    /// 返回 Core 已观察到的客户端可见 Provider 原生响应 ID。
    #[must_use]
    pub fn response_id(&self) -> Option<&str> {
        self.response_id.as_deref()
    }

    /// 校验完整响应并返回原生终态 response object。
    ///
    /// # Errors
    ///
    /// 缺少可转换为非流式响应的 wire 终态时返回错误。
    pub fn finish(self) -> Result<Value, ResponseEncodeError> {
        self.wire_terminal
            .ok_or(ResponseEncodeError::MissingWireTerminal)
    }

    fn observe_canonical_identity(&mut self, event: &ProviderEvent) {
        for fact in event.canonical_facts() {
            let metadata = match fact {
                GatewayEvent::Started(metadata) | GatewayEvent::Completed(metadata) => metadata,
                _ => continue,
            };
            self.response_id = Some(metadata.response_id().to_owned());
        }
    }

    fn observe_wire(&mut self, wire: &ProtocolWireEvent) {
        if let Some(response_id) = wire
            .data()
            .pointer("/response/id")
            .or_else(|| wire.data().get("response_id"))
            .and_then(Value::as_str)
        {
            self.response_id = Some(response_id.to_owned());
        }
        let effective_type = wire
            .event_type()
            .or_else(|| wire.data().get("type").and_then(Value::as_str));
        if matches!(
            effective_type,
            Some("response.completed" | "response.incomplete")
        ) {
            self.wire_terminal = wire.data().get("response").cloned();
        } else if matches!(effective_type, Some("response.failed" | "error")) {
            self.wire_failure = true;
        }
    }
}

fn openai_wire(event: &ProviderEvent) -> Option<&ProtocolWireEvent> {
    event
        .wire_event()
        .filter(|wire| wire.protocol() == OPENAI_PROTOCOL)
}
