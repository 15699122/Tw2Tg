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

浏览器消息的 Rust 模型位于 `xarchive-protocol::BrowserRequest/BrowserResponse`，JSON Schema 位于 `shared/protocol-schema/browser-request.schema.json` 和 `browser-response.schema.json`。Native Messaging 的 4 字节 little-endian framing 已在 `xarchive-native-host` 中实现，并限制单个 payload 不超过 1 MiB。

当前 Native Host 已完成消息读取、JSON 解码、协议版本/ID/Tweet URL/类型/数量校验和结构化错误响应；由于 Windows Named Pipe 尚未接入，合法请求当前返回 `NATIVE_PIPE_UNAVAILABLE`，不会静默挂起浏览器请求。

## Desktop 到 Sidecar

```text
hello
download
cancel
shutdown
```

后期可拆分为 `extract` 和 `download_media`，以支持 aria2 作为独立传输后端。`xarchive-download` 当前已提供不依赖平台的 loopback HTTP JSON-RPC client 和 `DownloadBackend` 实现，但真实 `aria2c` 进程监督、artifact 分发和默认路由仍未启用。

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

下载事件顺序约定为：

```text
started → metadata → file* → progress → complete
```

`file` 事件报告单个已发现文件；`complete.files` 是最终文件清单。Rust 必须再次检查文件存在、相对路径安全、大小和 hash，不能仅凭 Sidecar 事件将 Job 标记为完成。

归档时 Rust 不信任 Sidecar 上报的 `size_bytes`；该字段只用于进度和诊断，最终数据库值必须来自本地文件系统。`sha256` 由 Rust 计算，Sidecar 不上报最终 hash。

## Schema

当前 Schema 位于 [`shared/protocol-schema/`](../../shared/protocol-schema/)。Schema 是 Rust、JavaScript 和 Python 的契约来源；浏览器请求/响应 schema、Sidecar schema、aria2 fixture 和 browser fixture 已配套，Windows Named Pipe framing 和真实 Telegram/aria2 网络集成仍需后续平台或网络适配。