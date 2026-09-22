-- 可选跨权重轮询：默认关闭，保持现有最高权重档内轮询行为。
alter table runtime_settings
    add column round_robin_cross_weight_enabled boolean not null default false;
