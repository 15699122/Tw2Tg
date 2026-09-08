# MVP 范围

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