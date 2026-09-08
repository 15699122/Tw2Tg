# Requirements Summary

## Functional

1. 在 X Tweet 页面注入归档按钮。
2. 提取 DOM metadata，并与 gallery-dl metadata 合并。
3. 由 Rust 创建、调度和恢复归档 Job。
4. 将媒体下载到 staging，校验后提交稳定目录。
5. 维护 SQLite、Tweet/媒体去重、用户名称历史和事件记录。
6. 生成 `tweet.json` 与 `tweet.txt`。
7. 可选发送 Telegram metadata 和媒体。
8. 将归档状态和进度同步到 Extension。
9. 支持 Sidecar 崩溃、Telegram 失败和应用重启后的恢复。

## Non-functional

- Rust 是唯一业务状态所有者。
- 不在 Extension 传 Cookie 或媒体。
- Sidecar 不开放 HTTP。
- Secret 不进入 SQLite、日志或 Extension。
- 跨进程协议版本化并使用 JSON Schema。
- 首要平台为 Windows。
- 下载默认保存原文件，不自动转码或删除重复内容。