# 数据模型

主数据库：便携布局下为 `<portable-root>/config/archive.sqlite3`；portable root 默认是 `.exe` 所在目录，可用 `XARCHIVE_PORTABLE_ROOT` 环境变量覆盖。Telegram send-state 暂时继续存储在主 SQLite。

gallery-dl 的辅助去重属于 Sidecar 运行时行为，不是业务事实来源；当前便携布局不定义独立的业务数据库文件。

## 主要表

| 表 | 用途 |
|---|---|
| `users` | 稳定 X user ID、稳定目录名 |
| `user_names` | username/display name 历史和来源 |
| `tweets` | Tweet 主记录、双来源 metadata、归档目录 |
| `media` | 媒体索引、文件路径、大小、SHA-256 |
| `jobs` | 业务任务状态、尝试次数、错误和生命周期 |
| `transfers` | gallery-dl/aria2 外部传输任务映射 |
| `telegram_messages` | Tweet/media 与 Telegram message 的关系 |
| `tags` / `tweet_tags` | 标签定义和关联 |
| `events` | 状态转换、重试、崩溃和冲突历史 |
| `archive_batches` | 账号批次头（username/profile URL、可选稳定 `user_id`、`state`、`discovery_state`、`filters_json`、`retry_at_ms`、最近错误） |
| `batch_candidates` | 批次候选 Tweet（`batch_id` + `tweet_id` 唯一、媒体/转推事实、`state`、`job_id`、错误与 `skip_reason`） |
| `settings_meta` | 非敏感设置 |

## 关键字段

`tweet_id`、`x_user_id`、`media_id`、`chat_id`、`message_id` 使用字符串或明确的大整数安全表示。`media` 至少保存 `relative_path`、`mime_type`、`size_bytes`、`sha256`；外部执行器的 GID 只保存为 `backend_task_id`，不能作为业务主键。

## 文件布局

```text
<portable-root>/
├─ config/
│  ├─ config.yaml
│  └─ archive.sqlite3
├─ cache/
│  ├─ staging/<job_id>/
│  ├─ downloads/
│  └─ runtime/
├─ download/                  # 或系统 Downloads/XArchive
│  ├─ Tweets/<tweet_id>/
│  │  ├─ tweet.json
│  │  ├─ tweet.txt
│  │  ├─ 01.jpg
│  │  └─ 02.mp4
│  └─ Users/<stable_directory_name>/profile.json
├─ logs/xarchive-*.log
├─ sidecar/gallery-dl/ 与 sidecar/aria2/
└─ extension/
```

当前布局不创建 `telegram/` 目录。`FileStore::new` 的 `_staging` 布局仅保留给测试；生产运行通过 `FileStore::with_staging_root` 使用 `cache/staging/`。

所有 Sidecar/aria2 输出先写 staging，Rust 校验并提交后才写入最终数据库状态。

## Local archive service

`xarchive-storage::ArchiveService` 将已完成的 Sidecar 结果提交为本地归档：生成 `tweet.json`/`tweet.txt`、登记媒体及 SHA-256、提交 staging 目录，并将 Job 推进到 `DOWNLOADED`。它不会执行 X 提取、Telegram 上传或浏览器通信。

Desktop 通过 Rust Tauri `list_jobs` command 查询最近 Job；前端不直接打开 SQLite。查询按 `updated_at DESC` 返回，单次最多 100 条，状态和错误字段由 Rust 从数据库校验后序列化。

`complete_sidecar_archive` 是 Sidecar 与本地归档的边界：Sidecar 只提供 metadata 和相对文件路径，Rust 在 staging 中重新检查文件、读取实际大小、计算 SHA-256 后才生成最终 `ArchiveMetadata`。

端到端提交顺序为：先写入并提交 staging 目录，再更新 Tweet/media 数据，最后推进 Job 到 `DOWNLOADED`。Sidecar 报告的文件大小仅用于诊断，最终大小和 hash 由 Rust 从本地文件重新计算。

## Migration

Storage crate 自主管理版本化 migration，文件位于：

```text
crates/xarchive-storage/migrations/0001_initial.sql
crates/xarchive-storage/migrations/0002_telegram_send_state.sql
crates/xarchive-storage/migrations/0003_quote_reply_relationships.sql
crates/xarchive-storage/migrations/0004_archive_job_requests.sql
crates/xarchive-storage/migrations/0005_account_batches.sql
crates/xarchive-storage/migrations/0006_batch_discovery_paused.sql
```

`0006` 以 SQLite 表重建方式把账号发现的 `PAUSED` 加入 `discovery_state` CHECK constraint，保留 `archive_batches`/`batch_candidates` 行、索引与外键；升级测试必须覆盖 v5 候选保留和 `PRAGMA foreign_key_check`。

其中 `jobs_one_active_archive_per_tweet` 部分唯一索引保证同一个 Tweet 同时最多一个活动归档任务。`settings_meta` 只保存非敏感设置，不保存 Token、Cookie 或 RPC Secret。Job failure 文本在写入 SQLite 和投影到前端前均执行 URL userinfo / sensitive query redaction。

`xarchive-storage::Database` 当前已提供 users/user_names/tags/tweet_tags 的跨平台 Repository API：用户 upsert 会复用 `xarchive-core` 的稳定目录名策略，名称历史按观测时间保存，标签及 Tweet 关联写入均为幂等。Credential、Cookie 和 Telegram message 的真实生命周期仍由后续平台/网络适配层接入。

`xarchive-core` 的 `RetryPolicy` 只描述错误分类、重试预算和退避时间，不直接启动线程或修改数据库；调度器必须由 Rust Desktop 根据 Job 状态和事件历史使用它。`xarchive-telegram` 不保存 Token；它构造 Bot API 请求、格式化内容，并提供基于 `reqwest` + Rustls 的 HTTPS transport。真实发送状态持久化、恢复调度、Credential Manager 和账号环境仍由后续适配层接入。