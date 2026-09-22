# v3.13.0+cpr.3

> Fork 集成版本：基于上游 v3.13.0，合并 PR #161（含最新 web token 提取与 rustls 安全修复）、#192、#223。

## 新增功能

- Codex 会话亲和开关：关闭后普通请求不再绑定首个账号；continuation/required account 等必要 pin 保持不变
- 轮询跨权重开关：开启后所有当次可调度账号按 weight 比例参与 RoundRobin（例如 weight 2:1 约按 2:1 分配）
- 额度恢复后自动回池：quota worker 复核确认恢复后，账号重新进入候选池
- 账号定时预激活（PR #192）
- PAT 账号 Web Access Token（PR #161，含最新粘贴会话数据提取与 rustls TLS 安全修复）

## 升级说明

- 自动执行 0017、0018、0019 数据库迁移
- 建议设置：管理端 → 设置 → 网关调度 → 关闭 Codex 会话亲和，并开启轮询跨权重
- 本版本在线更新仍指向上游仓库；升级请使用 fork 镜像或本 Release
