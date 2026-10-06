# Full 0.2.1 dev installation and manual continuation — 2026-10-06

> 2026-10-06 follow-up: [user manual feedback and current FAIL/queue](../validation/windows-full-2b98952-feedback.md). First browser connection/task creation PASS scoped; status consistency and both download-mode completion FAIL user-observed. Earlier NOT_RUN below is the prior checkpoint, not the latest manual result.

Source `2b98952aeaa717899ebcd13c09f058a28e998e73`; [build results and SHA](windows-full-2b98952-results.md). Current delivery: `dist-portable/XArchive-0.2.1-dev-2b98952-windows-x64-full.zip`, SHA `06ab9d7973a413f2bbd28d7c81dd83d24daa2e77384f751714c7a4f7affa1e11`.

1. Verify adjacent `.zip.sha256.txt`, then extract the entire ZIP into a new directory. Start its `xarchive-desktop.exe`; do not mix components with another Full build. Requires Windows x64 and WebView2. This is portable Full; no MSI/NSIS is generated.
2. Use the first-launch “创建便携目录” or select a dedicated archive directory. Settings should render; start Sidecar and confirm hello→ready. Download mode defaults off (gallery-dl); switching, saving and restart persistence have scoped new-artifact evidence.
3. For browser acceptance, use a dedicated Edge/Chrome profile. Register/repair Native Host from this moved package, then load its Extension directory and verify actual ID `iaajefkoanbkleojofoadeakelihbjne`. Never treat the build-time absolute host path as completed Registry installation. Automatic bootstrap/restart/reconnect needs actual-browser validation; controlled stdio/WebSocket probes are not that acceptance.
4. Use an authorized test X account and test media; compare actual committed archive files/SHA and COMPLETE state with aria2 off/on. Follow WQ-DL cancellation/recovery/reparse steps. Prior real-archive DOWNLOAD_TIMEOUT and automatic reconnect FAIL remain unresolved; do not claim a build fixes them.
5. PAC/WPAD actual-egress and DPI/busy/error/focus matrices use isolated OS/browser settings; preserve M13 implicit-local-bypass risk. The new authenticated controlled WebSocket close probe did not complete within 3 seconds; Cross-platform review is queued.
6. Telegram requires an authorized dedicated test Bot/Channel and explicit send approval. Tokens/cookies stay in the application/profile and out of evidence. No account, token, local Bot API Server or Telegram send result is bundled.

Current manual states, prerequisites and owners: [remaining acceptance](windows-full-2b98952-results.md#remaining-manual-acceptance-and-ownership). Windows Platform Owner executes after Git reconciliation. This dev candidate is not release acceptance.
