# Windows 40858835 batch plan

Date: 2026-10-06. Owner: Windows Platform Owner. Return branch `codex/windows-validation-40858835`.

Input `40858835d249ead37ad1f3efb2dc1571821f9f8f` includes GUI implementation `8a8fa8a` and formal handoff `a3b7870`. Current return implementation/validation `60b76f357acbcf3510fa9bfcdc32309a35ca97cf`.

## Selected scope and outcome

1. Fetch and align exact input, inspect shared diff and current queue: completed through Git; no direct sync, local untracked data protected.
2. Targeted frontend + Storage/Desktop modules: completed; preserve initial Sidebar, fixture/environment and intermittent PAC failures alongside corrected results.
3. Minimal shared Sidebar callback restoration with event regression: implemented as `60b76f3`, CROSS_PLATFORM_REVIEW_REQUIRED.
4. Fresh Desktop Full build, pinned component inventory, ZIP/source identity: completed scoped; initial incorrect gallery input rejected, worker fresh-build provenance not established.
5. Native isolated empty/21-task/reversed-order UI subset at ordinary/maximized sizes: completed scoped. Remaining date, summary index 19, DPI, five-state/error/keyboard and real integration acceptance stays in the manual queue.
6. Final diff/docs audit and Git return: handoff commit containing [results](../validation/windows-40858835-results.md); next Owner Cross-platform for integration/review. Windows retains native follow-up.

Full workspace regression and unrelated service/real-account suites not run because the change affects frontend layout and one Storage/Desktop ordering command. No release GO or whole WQ-UI-CONSISTENCY-01 PASS is claimed.

Current state: WINDOWS_VERIFICATION_PENDING. Authoritative remaining steps and evidence: [batch results/manual queue](../validation/windows-40858835-results.md), [Windows queue](../validation/windows-queue.md), [current handoff](../status/platform-handoff.md).
