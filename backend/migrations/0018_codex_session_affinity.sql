-- Codex 会话亲和开关：关闭后 Codex 不再把会话绑定到固定账号，逐请求按调度策略分配。
-- Codex 的 prompt cache 依赖 prompt_cache_key 跨账号命中，关闭亲和可获得更均匀的账号用量。
alter table runtime_settings
    add column codex_session_affinity_enabled boolean not null default true;
