# Platform Handoff（Windows → Cross-platform）

Windows Validation Queue 的唯一事实源仍是 [`../validation/windows-queue.md`](../validation/windows-queue.md)；平台验证流程见 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md)。

## 当前状态

- Branch：`security/tweet-url-host-validation`
- Implementation revision：`7b218f8a5ff4a10590cea3cf762fd30a82d6c6c9`
- Validation revision：`cd04f269fca55d92f1a2336df6fbd8e9911d7dbf`（验证结果首次写入 validation 文档的 commit）。
- 状态：`CROSS_PLATFORM_RECONCILE_REQUIRED`
- 工作副本：Windows 正式 E: checkout；本轮只写入验证与 handoff 文档，未改实现源码。

## Reconcile 所需工作

- `CROSS_PLATFORM_CHANGE_REQUIRED`：修复 Windows Junction 暴露的共享 metadata path containment 缺陷；修复 clean `npm ci` 后的 Node override/lock dependency contract；调查 Tauri WebDriver 155 会话停在 `data:,` 的 native startup 问题。
- `CROSS_PLATFORM_REVIEW_REQUIRED`：检查 `desktop/scripts/wdio-tauri-service.mjs` 的 Windows `killTree` 与测试退出证据契约。Windows 全量 Node 测试中该断言失败且测试进程未及时退出。
- Windows PASS/FAIL/BLOCKED 的精确证据、命令、package/driver 版本与手工队列见 [`../development/windows-validation.md`](../development/windows-validation.md) 的 2026-09-27 phase section 和 [`../validation/windows-queue.md`](../validation/windows-queue.md) 的 phase result。

## 下一 Owner

Cross-platform Owner 先完成上述 reconcile 并更新 implementation/handoff revision；随后仅把命中改动的条目交回 Windows 做 focused revalidation。未完成的真实账号、GUI、Named Pipe、workflow 和 archive extraction 项继续保留在 Manual Windows Validation Queue。

## 同步方式

沿 Git 将验证文档写回 source branch；不通过直接文件同步覆盖正式 Windows repo。Windows phase 使用 `git fetch` 后验证 exact handoff revision `7b218f8a5ff4a10590cea3cf762fd30a82d6c6c9`。
