# v3.10.0-exp.1

## 实验说明

Codex 降智缓解实验版，基于正式版 v3.10.0，面向希望体验 [#144](https://github.com/zyycn/codex-proxy-rs/issues/144) 中方案的用户。
本版本接续 `v3.10.0-codex-anti-degradation.1`，采用简短的 `exp.N` 命名；旧版本标签与产物保留。
实验功能独立于主版本，实际效果仍需使用反馈验证，不保证改善模型质量，也不承诺合入正式版。

## 本次变化

- 合入 [#155](https://github.com/zyycn/codex-proxy-rs/pull/155)：各模型独立刷新 State，成功后立即写入缓存；其他失败模型继续重试，互不等待。单模型前三轮各发起 1 个探针，之后每轮最多 3 个，接受首个成功结果并取消其余请求。
- 手动刷新实时显示逐模型结果，支持停止刷新、关闭弹窗及离开页面时取消未完成请求，保留已成功结果。
- State Header 按实验规则严格校验为 292 字节；长度不符时立即丢弃响应，不读取正文、不写缓存。通过长度检查后，仍需 HTTP 成功及完整响应事件才可缓存。
- 429 冷却按账号和模型隔离，遵守 Retry-After；取消刷新或配置失效不会清除冷却。失败不会延长旧 State 的 60 分钟有效期。
- 后台启动后执行一轮，每轮结束后随机等待 53～55 分钟；手动刷新不重置后台周期。

## 安装与使用

```bash
docker pull ghcr.io/zyycn/codex-proxy-rs:3.10.0-exp.1
```

- Docker 镜像支持 Linux amd64、arm64；二进制归档提供 Linux amd64、Linux arm64、macOS arm64，包含管理端静态资源。
- 从本页附件下载 `compose.yaml`、`config.example.yaml` 和 `checksums.txt`。Compose 已固定本版本镜像并合并隔离配置，使用独立项目名，管理端默认端口为 `8081`。
- 首次安装使用独立目录、配置、PostgreSQL、Redis 和数据目录，按[实验部署说明](https://github.com/zyycn/codex-proxy-rs/blob/v3.10.0-exp.1/deploy/experimental.md)操作。
- 功能默认关闭；启用及动态代理配置见[功能说明](https://github.com/zyycn/codex-proxy-rs/blob/v3.10.0-exp.1/docs/session-keepalive-design.md)。

归档提供校验和与 GitHub 构建产物证明；镜像附带 SBOM、构建来源证明及 cosign 签名。

## 使用须知

- 相对上一实验版未修改数据库迁移。升级已有实验实例前请备份，手动替换镜像或二进制；应用内一键更新禁用。
- 手动刷新中失败模型会持续重试，可停止刷新或关闭弹窗取消。后台按账号顺序处理，一个账号长期失败可能推迟后续账号刷新。
- 292 字节准入是当前实验规则，不是上游公开格式保证，也不是凭证完整性验证；上游行为变化可能使实验失效。
- 不与稳定实例共用数据库或数据目录，实验库不能直接降级回稳定版。运行过 PR #151 早期迁移的源码实例，需按部署说明核对重建与恢复边界。
- 本版本为 Pre-release，不设为 GitHub Latest，不覆盖 Docker `latest`；稳定更新渠道不会提示本次实验升级，主动选择 `preview` 渠道的用户可能看到预发行。

## 反馈与致谢

欢迎在 [#144](https://github.com/zyycn/codex-proxy-rs/issues/144) 补充实际体验，注明版本、使用场景及观察结果，区分现象和推测；已有反馈请继续补充在原帖。
感谢 [@chenyanshan](https://github.com/chenyanshan) 提供本次修复，以及 [@Clarence12138](https://github.com/Clarence12138) 和参与讨论、测试的社区朋友。
