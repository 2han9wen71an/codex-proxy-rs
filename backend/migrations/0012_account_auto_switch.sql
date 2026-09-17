-- 账号级自动换号开关：控制当该账号配额耗尽或遇到错误时，是否允许自动切换到账号池中的其他可用账号。
alter table provider_accounts
    add column auto_switch_enabled boolean not null default true;
