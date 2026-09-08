# 跨进程协议

## 原则

- 所有消息带 `protocol_version` 和 `request_id`。
- Tweet ID 使用字符串。
- 时间使用 RFC 3339。
- stdout 只输出机器协议；日志走 stderr。
- 所有输入都必须重新校验。
- 不传 Cookie、Bot Token、媒体二进制或任意本地路径。

## 浏览器到 Desktop

```text
ping
archive_request
query_status
retry_job
reupload
redownload
open_folder
open_telegram
```

## Desktop 到 Sidecar

```text
hello
download
cancel
shutdown
```

后期可拆分为 `extract` 和 `download_media`，以支持 aria2 作为独立传输后端。

## Sidecar 事件

```text
ready
started
metadata
progress
file
complete
failed
log
```

## Schema

当前 Schema 位于 [`shared/protocol-schema/`](../../shared/protocol-schema/)。Schema 是 Rust、TypeScript 和 Python 的契约来源，后续必须配套 fixtures 和契约测试。