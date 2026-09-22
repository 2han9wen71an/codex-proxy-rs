# v3.13.1+cpr.1

> Fork 集成版本：基于上游 v3.13.1，合并 PR #161（含 profile 修复）、#192、#223。

## 新增功能

- Codex 会话亲和开关：关闭后普通请求不再绑定首个账号；continuation/required account 等必要 pin 保持不变
- 轮询跨权重开关：开启后所有当次可调度账号按 weight 比例参与 RoundRobin
- 额度恢复后自动回池：quota worker 复核确认恢复后，账号重新进入候选池
- 个人资料查询改用 web access token：PAT 账号主 token 过期也能查看个人首页（PR #161 后续修复）
- 账号定时预激活（PR #192）
- PAT 账号 Web Access Token（PR #161，含粘贴会话数据提取与 rustls TLS 安全修复）

## 同步的上游 v3.13.1 修复

- 账号支持无限并发与快照竞态恢复（#225）
- 模型元数据与终态保留、额度重置改进（#224）
- 透传 turn metadata 的 installation 身份按账号隔离
- rustls 升级越过 TLS handshake 安全通告

## 升级说明

- 上游新增迁移 0017（unlimited default account concurrency）；自有迁移调整为 0018/0019/0020，首次应用自动完成
- 建议设置：管理端 → 设置 → 网关调度 → 关闭 Codex 会话亲和，并开启轮询跨权重
- 本版本在线更新仍指向上游仓库；升级请使用 fork 镜像或本 Release
