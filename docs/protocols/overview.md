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

当前 Native Host 已完成消息读取、JSON 解码、协议版本/ID/Tweet URL/类型/数量校验、结构化错误响应和可插拔 transport 转发。配置 `XARCHIVE_PIPE_ENDPOINT` 后，Native Host 会以读写方式打开指定 Desktop endpoint，转发一个经过校验的 `BrowserRequest` 并读取 `BrowserResponse`；未配置时仍返回 `NATIVE_PIPE_UNAVAILABLE`，连接或协议失败返回 `NATIVE_PIPE_ERROR`。Windows Named Pipe server、ACL、Registry 注册和实机重连仍未完成，不能将 Linux fake transport 测试视为 Windows Named Pipe 验证。
## 当前 Desktop 到 Sidecar：v1（CURRENT / MIGRATION）

```text
hello
download
cancel
shutdown
```

当前 Rust、Python worker、Schema 和 Desktop consumer 使用 v1 `download` command。v1 会让 gallery-dl 直接写入 staging，并产生下载事件；这是当前迁移中的运行时事实，不是目标终态。

## 目标 Desktop 到 Sidecar：v2（PLANNED / U3）

```text
hello
extract
cancel
shutdown
```

目标协议为 Sidecar v2：`hello`、`extract`、`cancel`、`shutdown`。U3 完成前，Rust、Python、Schema、fixtures、Supervisor 和 Desktop consumer 不得单侧切换到 v2；不支持 v1/v2 双解析或 capability 不足时回退旧路径。

目标媒体链路为 gallery-dl extraction-only → typed `ExtractionResult` → Rust `MediaTransferPlan` → aria2-only transfer。当前 `xarchive-download` 仍包含 `DownloadRouter` 和旧 fallback 测试，它们属于 U8 前的 `MIGRATION` 残留。

## 当前 Sidecar 事件：v1（CURRENT / MIGRATION）

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

当前 v1 下载事件顺序约定为：

```text
started → metadata → file* → progress → complete
```

`file` 事件报告单个已发现文件；`complete.files` 是最终文件清单。Rust 必须再次检查文件存在、相对路径安全、大小和 hash，不能仅凭 Sidecar 事件将 Job 标记为完成。

## 目标 Sidecar 事件：v2（PLANNED / U3）

```text
ready
extraction_started
extracted
cancelled
failed
log
```

目标 v2 extraction 事件顺序约定为：

```text
ready → extraction_started → extracted
```

v2 的 `extracted` 只携带 typed extraction result，不携带已下载文件清单；媒体主体由后续 aria2 transfer 阶段写入 staging。
归档时 Rust 不信任 Sidecar 上报的 `size_bytes`；该字段只用于进度和诊断，最终数据库值必须来自本地文件系统。`sha256` 由 Rust 计算，Sidecar 不上报最终 hash。

## Schema

当前 Schema 位于 [`shared/protocol-schema/`](../../shared/protocol-schema/)。Schema 是 Rust、JavaScript 和 Python 的契约来源；浏览器请求/响应 schema、Sidecar schema、aria2 fixture 和 browser fixture 已配套，Windows Named Pipe framing 和真实 Telegram/aria2 网络集成仍需后续平台或网络适配。