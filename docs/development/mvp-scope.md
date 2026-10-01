# MVP 范围（历史快照）

> **本文件是 MVP 初始范围快照，不描述当前实现，也不作为验收依据。** 当前能力以 [`status.md`](status.md) 为准，未来方向以 [`roadmap.md`](roadmap.md) 为准。
>
> 保留本文件的原因是它记录了项目最初的范围取舍决策。该决策后来被明确扩张，下列条目已不再成立：
>
> - 「不包含整个用户 Timeline 批量抓取」——已实现账号/主页维度的批量归档（`desktop/src/pages/batches-page.jsx`）。
> - 「aria2 默认下载」不包含——媒体传输已按 extraction-only → aria2-only 链路实现。
> - 「其他平台」不包含——跨平台 Owner 机制已建立，Linux 承担共享实现。
>
> 引用本文时必须同时标注上述限定，不得据其推断当前范围。

## 包含

- Windows。
- Edge/Chrome Manifest V3 Extension。
- Timeline 和 Tweet Detail 的单条手动归档。
- DOM + gallery-dl metadata。
- 图片和 gallery-dl 能直接取得的视频。
- Edge Profile 认证。
- SQLite、本地原文件、`tweet.json`、`tweet.txt`。
- Telegram Official Bot API 和 Preview 模式。
- 页面状态、基础重试和 Tray 后台运行。

## 不包含

- 整个用户 Timeline 批量抓取。
- 自动账号监听。
- aria2 默认下载。
- IDM 核心集成。
- yt-dlp fallback 和 FFmpeg 转码。
- 云同步、OCR、AI 标签、复杂媒体浏览器。
- 其他平台。

## MVP 完成定义

同一 Tweet 重复点击不重复下载；下载中退出后可识别为中断；成功文件经 Rust 校验和 hash；Telegram 失败不损坏本地归档；浏览器刷新后可以查询并显示状态；协议、Rust、Python 和 Extension 基础测试通过。