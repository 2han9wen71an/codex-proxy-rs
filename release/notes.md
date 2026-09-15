# v3.7.1

## 新增功能

- OpenAI 请求地区可通过 `openai.wire_profile.location` 统一配置国家、地区、城市和 IANA 时区，并同时用于 Web Search 地区与结构化环境日期。省略该配置时继续使用现有默认地区。

## 问题修复

- 修复 OpenAI WebSocket 上游在流中返回 `error` 事件时，SSE 客户端只能看到无原因断流的问题。网关现在将其转换为 `response.failed`，保留上游的错误类型、代码、消息和响应 ID，便于客户端识别并决定是否重试。
- 移除默认 Compose 部署中固定的 2 CPU 与 1 GiB 容器资源限制，允许服务在 CPU 少于 2 核的主机上启动；实际资源上限由部署环境决定。

## 升级说明

- 从 v3.7.0 升级无需数据库迁移，已有配置可以继续使用，`openai.wire_profile.location` 为可选项。
- 使用新版 `deploy/config.example.yaml` 中的 `openai.wire_profile.location` 时，需要同步升级到 v3.7.1 镜像；v3.7.0 镜像会因严格配置校验拒绝该字段。
- 需要解除默认 CPU 和内存限制的部署应同步更新 `deploy/compose.yaml`。自行设置的宿主机或容器资源限制仍然生效。
