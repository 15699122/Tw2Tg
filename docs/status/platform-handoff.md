# Current Platform Handoff

Status: `CURRENT` — 本文件只记录**当前批次**。历史交接记录在 [`platform-handoff-history.md`](platform-handoff-history.md)（2026-09-28 至 2026-10-01，原文归档）。

本文件不保存历史批次。完成一个批次后，将该批次原文追加到 `platform-handoff-history.md`，再在此写入新的当前批次。

## 历史批次速查

| 批次 | 归档位置 |
|---|---|
| Settings 布局与归档目录、UI polish 与 known folder、`v0.2.0` 发布授权、候选冻结、canonical 集成、分支收敛、依赖整改、preN 迁移、集成基线、Sidecar follow-up、日志样式/渠道/图标、`6be3269` 的 Windows 验证、日志过滤器绑定有效等级、界面紧凑化／设置顺序／高清图标的 Windows 验证与图标运行时修复、`v0.2.1-pre1` 受限预发布 | [`platform-handoff-history.md`](platform-handoff-history.md) |

## 当前批次：Telegram Local Bot API 共享实现（Batch A，2026-10-01 启动）

- Task: 恢复此前暂停的 Telegram 发送范围并完成 Batch A 的跨平台共享实现。Plan: [`../development/telegram-local-bot-api-plan.md`](../development/telegram-local-bot-api-plan.md)。
- Branch: `dev`.
- Current owner: **Cross-platform Owner**. Current state: `CROSS_PLATFORM_IN_PROGRESS`.
- Uncommitted state: recorded at the handoff revision below.
- 本批次只覆盖 Plan 的 TG-00…TG-05 与 TG-06 的共享业务部分。Windows 原生集成（Credential Manager、GUI）、外部 Local Bot API Server 部署与真实账号发送属于 Batch B，交给 Windows Owner。

### 本批次范围

- TG-00：恢复范围、冻结方法与来源、批准“归档与发送解耦”。
- TG-01：Telegram 配置契约、secret 引用、endpoint 模式与安全校验。
- TG-02：生产 transport 与流式上传（不使用示例代码的 reqwest 0.12，不把 executor 整体改异步）。
- TG-03：formatter、媒体分类与发送计划；修正 `media_groups()` 尾组单项问题；稳定上传文件名。
- TG-04：发送 Outbox、原子领取、`UNKNOWN` 结果与恢复。
- TG-05：bot 隔离的 `file_id` 缓存。
- TG-06（共享部分）：设置与任务状态的业务模型，不含 Windows 原生适配。

### 当前进度（2026-10-01）

- TG-00 已提交；TG-01（endpoint 契约 + `with_api_endpoint` + Desktop `TelegramConfig`）、TG-02（流式上传 `send_upload`）、TG-03、TG-04（`TelegramOutboxStore` + migration `0007_telegram_outbox.sql` + 失败分类与决策）、TG-05（bot 隔离 `file_id` 缓存）与 TG-06 共享业务模型（设置投影、任务文案、深链）均已落地。Linux 验证：telegram 40/40、storage 52/52、desktop 166/166、clippy 0 警告、fmt PASS、docs-audit PASS。
- 仍未开始：Desktop 发送服务（claim 循环、payload 选择、设置界面接线）与 Windows 凭据适配（Batch B）。`WQ-TG-*` 全部 `NOT_RUN`，执行入口尚不存在，手工步骤已写入 [`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md) §K。
- Implementation revision: `a59aa37`（branch `dev`，已推送 `origin/dev`；本条文档随附 docs commit 更新）。
- TG-04（Outbox/`UNKNOWN` 恢复）、TG-05（`file_id` 缓存）、TG-06 共享部分尚未开始；Desktop 接线与真实发送不存在。`WQ-TG-*` 全部 `NOT_RUN`（Batch B 前置条件未满足，本批次不新增队列项）。
- 下一 Owner: **Cross-platform Owner**（下一步 TG-04）；本批次无 `CROSS_PLATFORM_CHANGE_REQUIRED`/`CROSS_PLATFORM_REVIEW_REQUIRED`/`WINDOWS_VERIFICATION_BLOCKING`。

### 明确不在本批次

- 不在 Desktop 内管理 Bot API Server 生命周期，不自动安装 Docker/WSL2。
- 不实现服务器本地路径上传（TG-08，后续批次）。
- 不引入 teloxide 或任何 TDLib binding；不接收 updates。
- 不自动转码、不自动删除远端消息、不承诺远端 exactly-once。

### Unigram 接收端

Windows 上以 Unigram（`unigramdev/unigram`）为主要接收端验收对象，但它是**接收客户端**，不是发送依赖：XArchive 在 Unigram 未运行时也必须能发送，且不读取其凭据、缓存或本地数据库。`WQ-TG-UNI-01`…`08` 已加入 [`../validation/windows-queue.md`](../validation/windows-queue.md)，与发送层分三层验收。

### Windows work and validation required

`WQ-TG-001`…`WQ-TG-009` 与 `WQ-TG-UNI-01`…`UNI-08` 全部为 `NOT_RUN`，在当前实现落地前**不可执行**。Windows Owner 的 Batch B 工作：Credential Manager 适配、设置页与任务状态 GUI、外部服务器部署与两条链路（`Desktop → Local API`、`Local API → Telegram`）分别验证、受控账号真实发送、大文件与取消/恢复。共享契约或状态机缺陷用 `CROSS_PLATFORM_CHANGE_REQUIRED`；保留抽象的小修用 `CROSS_PLATFORM_REVIEW_REQUIRED`。

### Scope boundaries

本批次不发布 release、不修改 GitHub Release、不关闭任何验收门槛、不 bump 版本文件。它把 Telegram 从 `PAUSED` 恢复为 `PLANNED`，并把 `v0.2.1-pre1` 的“Telegram 不在范围”保留为历史事实，不倒改。任何“Telegram 可用”的声明仍需 §9.1 的七项条件。

### Pre-release `v0.2.1-pre1` (2026-10-01)

The Owner separately authorized a **restricted** development pre-release ([`../release/release-policy.md`](../release/release-policy.md) §6). `dev` now carries the version bump from `0.2.0` to `0.2.1` (synchronized across `Cargo.toml`/`Cargo.lock`, `tauri.conf.json`, `package.json`/`package-lock.json`, `desktop/package.json`, `extension/manifest.json`/`extension/package.json`, and the `main.jsx` `app_version` placeholder) plus `docs/release/notes/v0.2.1-pre1.md`.

`.github/workflows/pre-release.yml` verifies Linux on the pinned `dev` revision, creates the `v0.2.1-pre1` tag and pre-release object, and dispatches `.github/workflows/windows-release.yml` on that tag for the seven Windows assets. This publishes a **narrower claim**: it does **not** close G4/G5/G7, does **not** promote any unverified capability, and is **not** a `v0.2.1` release approval.

Result: tag `v0.2.1-pre1` → `3115c3b50716be0155804ad4f94dd9d29e37d617`; `pre-release.yml` Run `36837862702` `success`; `windows-release.yml` Run `36838400270` build `success` with all seven assets uploaded and `source_sha` matching the tag target. The independent WDIO job is `FAIL` (`DevToolsActivePort file doesn't exist`, an environment-class failure that reproduces on the `v0.2.0` run) and is explicitly non-blocking. GUI, extension and filesystem/transfer acceptance remain `WINDOWS_BLOCKED` / `NOT_RUN`. The confirmed source SHA, asset state and CI results are recorded in [`../release/release-history.md`](../release/release-history.md) and [`../validation/windows-queue.md`](../validation/windows-queue.md).
