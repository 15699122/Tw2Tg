# 跨进程协议

## 原则

- 所有消息带 `protocol_version` 和 `request_id`。
- Tweet ID 使用字符串。
- 时间使用 RFC 3339。
- stdout 只输出机器协议；日志走 stderr。
- 所有输入都必须重新校验。
- 不传 Cookie、Bot Token、媒体二进制或任意本地路径。

## 浏览器到 Desktop（当前实现）

```text
archive_request
query_status
```

当前 Rust、Schema、Native Host、Desktop transport 和 Extension 实际只实现 `archive_request` 与 `query_status`。`ping`、`retry_job`、`reupload`、`redownload`、`open_folder`、`open_telegram` 等命令如果未来需要，必须先进入 roadmap 和 schema 设计，不能作为当前可用能力描述。

浏览器消息的 Rust 模型位于 `xarchive-protocol::BrowserRequest/BrowserResponse`，JSON Schema 位于 `shared/protocol-schema/browser-request.schema.json` 和 `browser-response.schema.json`。Native Messaging 的 4 字节 little-endian framing 已在 `xarchive-native-host` 中实现，并限制单个 payload 不超过 1 MiB。

`archive_request` 返回单条 `archive_status`。`query_status` 是批量请求，返回 `archive_status_batch`；`statuses` 与请求中的 Tweet ID 一一对应，未找到 Job 时使用 `state = NOT_ARCHIVED` 和 `job_id = null`，不能将“没有匹配 Job”当成整个批次的协议错误。Browser Rust model 使用 `serde(deny_unknown_fields)` 拒绝 Schema 未声明字段；当前 Browser schema source 只有 `browser-request.schema.json`、`browser-response.schema.json`，不再维护独立的重复 archive request/status schema。

当前 Native Host 已完成消息读取、JSON 解码、协议版本/ID/Tweet URL/类型/数量校验、结构化错误响应和可插拔 transport 转发。配置 `XARCHIVE_PIPE_ENDPOINT` 后，Native Host 会以读写方式打开指定 Desktop endpoint，转发一个经过校验的 `BrowserRequest` 并读取 `BrowserResponse`；未配置时仍返回 `NATIVE_PIPE_UNAVAILABLE`，连接或协议失败返回 `NATIVE_PIPE_ERROR`。Linux/Unix Desktop endpoint 已实现；Windows Named Pipe server、ACL、Registry 注册、真实浏览器连接状态和实机重连仍属于 U17/E5–E7，不能将 Linux fake transport 测试视为 Windows Named Pipe 验证。
## WebSocket transport（迁移中）

WebSocket 只替换浏览器到 Desktop 的传输适配层，不改变 Browser protocol v1。Desktop 优先使用成熟 `tungstenite` 库，浏览器使用内建 `WebSocket`；禁止自行实现 RFC 6455 帧和握手。

- listener 仅绑定 loopback，端口由固定默认值和受控环境变量覆盖；
- 连接必须先完成独立 transport envelope 的一次性认证，Origin、端口和连接成功本身不构成身份；
- 认证凭据通过本机配对流程交付 Extension storage，不进入业务消息、Job spec、日志或 Native Host manifest；
- 连接认证后，每个 WebSocket text message 是一个符合 `browser-request.schema.json` 的 `BrowserRequest`，响应使用 `browser-response.schema.json` 并保持 `request_id`；
- 未认证连接不得调用 `BrowserTransportAdapter`；断线、关闭、超时和服务 worker 重启必须清理 pending request；
- 迁移期间保留 Native Messaging 回退，WebSocket-only 发布必须等待 Windows Edge/Chrome 实机与打包验收。

具体端口发现、凭据轮换和失败恢复以 [`../architecture/decisions.md`](../architecture/decisions.md) ADR-014 为准；当前实现与目标状态以 [`../development/status.md`](../development/status.md) 为准。


```text
hello
extract
cancel
shutdown
```

Rust `SidecarV2Command`、Python `worker_v2`、Schema `sidecar-v2-command.schema.json` 和 Desktop consumer 只使用 v2，不存在 v1/v2 双解析或 capability 不足时回退旧路径。v1 `download` command、v1 命令/事件类型、v1 Schema 和 Python v1 worker 已在 U8 删除；Supervisor stdout reader 只接受 `protocol_version = 2` 的事件，其他版本记为 `ProtocolError`。

媒体链路为 gallery-dl extraction-only → typed `ExtractionResult` → Rust `MediaTransferPlan` → aria2-only transfer。`DownloadRouter` 的 gallery-dl/aria2 fallback 和 gallery-dl 媒体下载已在 U8 删除。

## Sidecar 事件：v2（CURRENT）

```text
ready
extraction_started
extracted
cancelled
failed
log
```

v2 extraction 事件顺序约定为：

```text
ready → extraction_started → extracted
```

终止事件为 `extracted`、`cancelled` 或 `failed`；late result 不得覆盖终态。`extracted` 只携带 typed extraction result，不携带已下载文件清单；媒体主体由后续 aria2 transfer 阶段写入 staging。
归档时 Rust 不信任 Sidecar 上报的 `size_bytes`；该字段只用于进度和诊断，最终数据库值必须来自本地文件系统。`sha256` 由 Rust 计算，Sidecar 不上报最终 hash。

## Schema

当前 Schema 位于 [`shared/protocol-schema/`](../../shared/protocol-schema/)。Schema 是 Rust、JavaScript 和 Python 的契约来源；浏览器请求/响应 schema、Sidecar v2 command/event schema 和 aria2/browser/v2 fixtures 已配套。v1 `download-command.schema.json`、`download-event.schema.json` 及其 fixtures 已在 U8 删除，`fixtures/sidecar-v1-rejected.jsonl` 保留用于验证 v2 消费者拒绝 legacy 命令。Windows Named Pipe framing 和真实 Telegram/aria2 网络集成仍需后续平台或网络适配。