# Platform Handoff（Linux → Windows）

本文是跨平台 handoff 的当前状态记录。Windows Validation Queue 的唯一事实源仍是
[`../validation/windows-queue.md`](../validation/windows-queue.md)；
Linux ↔ Windows 工作流权威来源是
[`../development/cross-platform-validation.md`](../development/cross-platform-validation.md)。

## 当前 handoff

- Branch：`security/tweet-url-host-validation`
- Handoff revision：本文件所在 commit（`docs: add platform handoff status for cross-platform batch`；
  含 ENG-13 两次 fix；以 push 后的 HEAD hash 为准，见最终报告记录）
- Working tree：干净，已 push，与 `origin` 同步
- 状态：`READY_FOR_WINDOWS`

## 本 batch 内容（Cross-platform Owner）

- ENG-13（P1，RISK-005 子向量）：`--rpc-secret` 移出 aria2 子进程 argv，
  改走 owner-only 短期 `--conf-path` 文件；对应 Windows 验证项为 `WQ-ENG-12`。
- R7 其余项（ENG-01～ENG-12、ENG-14～ENG-16）Linux 实现与验证结论见
  [`../development/roadmap.md`](../development/roadmap.md) 执行进度表；本 batch 未改动其代码。

## Reconcile 结论

- `CROSS_PLATFORM_REVIEW_REQUIRED` / `CROSS_PLATFORM_CHANGE_REQUIRED`：全仓库无命中，无需处理。
- `WINDOWS_VERIFICATION_BLOCKING`：不存在；队列中全部阻塞标记为 `no`。
- Windows 上一轮 implementation / validation：无新的写回需要 reconcile，
  仍以 `windows-queue.md` 与 `windows-validation.md` 历史记录为准。

## Windows work / validation queue（下一 batch）

- `WQ-ENG-01`～`WQ-ENG-12`（含新增 `WQ-ENG-12`）；
- GUI/WebView2/DPI/辅助技术（`BLOCKED_AUTOMATION` 转手工）；
- 真实账号/凭据、Named Pipe/Registry、Installer（`BLOCKED`/`NOT RUN`，手工步骤已生成）。
- 详细分类与手工步骤见 `windows-queue.md` 的 `2026-09-27 R7 非 Windows 开发收口与 Windows 集中验证汇总` 节。

## 同步方式

正式 handoff 走 Git（同步 `d635f09` 到 Windows 工作副本，Linux → Windows 单向），
不直接覆盖 Windows 正式 repo。
