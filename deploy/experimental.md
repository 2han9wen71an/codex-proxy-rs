# Codex 降智缓解实验版

`experimental/codex-anti-degradation` 基于 v3.10.0，用于探索 [#144](https://github.com/zyycn/codex-proxy-rs/issues/144)
反馈的 Codex 疑似风控与响应质量下降（“降智”）问题。实现沿用 PR #151 的 `X-Codex-Turn-State` 刷新、会话保活和动态代理。
它独立于 `main`，是否有效取决于上游行为，不承诺长期有效或进入正式版。
按需维护必要修复，不自动跟随主分支，也不承诺与主版本同步发布；实验失效或无人维护时可以停止构建。

## 构建与更新

推送实验分支后，[实验工作流](https://github.com/zyycn/codex-proxy-rs/actions/workflows/experimental-codex-anti-degradation.yml)
复用项目质量检查、Dockerfile 和容器验收，通过后推送 Linux amd64 镜像：

```text
ghcr.io/zyycn/codex-proxy-rs:experimental-codex-anti-degradation-<完整提交 SHA>
```

具体地址见成功运行的 Summary；可进一步固定该镜像 digest。不会写入 `latest`、正式版本标签或 GitHub Release。
构建类型为 `experimental`，禁用应用内一键更新；更新时手动选择成功构建的镜像并重新执行下方部署命令。
源码自建也必须设置 `CPR_BUILD_TYPE=experimental`，避免误切稳定版更新。

## 独立部署

需要 Docker Compose 2.24.4 或更新版本。必须使用独立安装目录、配置、PostgreSQL、Redis 与数据目录，
不能在稳定版目录中切换分支后直接启动，也不能让两个版本连接同一个数据库。

```bash
git clone --branch experimental/codex-anti-degradation --single-branch   https://github.com/zyycn/codex-proxy-rs.git codex-proxy-rs-codex-anti-degradation
cd codex-proxy-rs-codex-anti-degradation
```

从工作流 Summary 选择成功构建的完整 SHA，执行 `git checkout <完整提交 SHA>`，使部署文件与镜像对应。
按照[手动安装](README.md#手动安装)创建 `.runtime` 目录、从当前分支的 `deploy/config.example.yaml` 创建配置并填写密码；
跳过下载正式 Release 部署文件的步骤。

```bash
export CPR_EXPERIMENTAL_IMAGE='ghcr.io/zyycn/codex-proxy-rs:experimental-codex-anti-degradation-<完整提交 SHA>'
docker compose -f deploy/compose.yaml -f deploy/compose.experimental.yaml config --quiet
docker compose -f deploy/compose.yaml -f deploy/compose.experimental.yaml pull
docker compose -f deploy/compose.yaml -f deploy/compose.experimental.yaml up -d --no-build --wait
```

管理端为 `http://127.0.0.1:8081`；数据库和 Redis 宿主端口分别为 `5433`、`6380`，容器内部连接无需修改。
实验覆盖文件使用单独的 Compose 项目名，绑定目录仍位于当前克隆的 `.runtime/`，不要改为稳定版目录。
所有后续 Compose 操作都应传入这两个文件与同一个镜像变量。
保活默认关闭，使用方法与边界见[功能说明](../docs/session-keepalive-design.md)。

## 数据与回退

本分支只保留一份新增 SQL：`0016_session_keepalive.sql`。稳定版目前使用 0001–0015；
实验库不能原地降级到稳定版，也不能随意合并主分支未来同编号迁移。
已经运行过 PR #151 原始 0016–0018 或旧版合并 0016 的源码实例，需要完整备份后重建与目标代码匹配的库，
并按目标表结构恢复业务数据；不要修改 `_sqlx_migrations` 的 checksum。

首次试用优先新建空库，通过管理端导入所需账号。回到稳定版时停止实验实例，使用原稳定版实例或其试用前备份。
需要迁移试用期间新增数据时，单独导出业务数据并核对字段，不直接把实验库备份还原给稳定版。

维护时按需挑选主分支修复，重新检查迁移编号、数据合同与实验功能；不要把整个实验分支合回 `main`。
