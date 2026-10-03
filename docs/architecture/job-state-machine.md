# Job 状态机

## 当前归档主流程

```text
QUEUED → VALIDATING → METADATA_READY → DOWNLOADING → DOWNLOADED → COMPLETE
```

## 异常状态

```text
INTERRUPTED
AUTH_REQUIRED
FAILED
CANCELLED
```

详细原因放在 `last_error_code`，不为每个错误创建独立状态。

## 幂等规则

| 状态 | 再次点击 |
|---|---|
| 无记录 | 创建任务 |
| QUEUED / VALIDATING / DOWNLOADING | 返回现有任务或报告当前执行状态 |
| DOWNLOADED + Telegram 失败 | 归档保持完成；Telegram outbox 独立重试/人工复核 |
| COMPLETE | 不重复执行 |
| 本地文件丢失 | Re-download |
| Telegram 消息需补发 | 仅通过独立 outbox 操作；不重跑归档 |

只有文件存在、路径安全、大小和 hash 校验成功、staging 已提交且数据库事务成功后，才能进入 `DOWNLOADED`。

## Telegram 发送与归档解耦（已确认契约，TG-00/TG-04）

上表的 `TG_METADATA_SENDING` 位于下载之前，反映早期设计。按 [`../development/telegram-local-bot-api-plan.md`](../development/telegram-local-bot-api-plan.md) 第 2.1 节（TG-00 已批准），Telegram **不在归档主链路中**：

- 本地归档成功独立成立，不依赖 Telegram 成功。
- Telegram 发送状态单独记录和展示（SQLite outbox，见下）。
- Telegram 不可用、失败或未配置都不阻塞归档。
- 仅补传 Telegram 时不触发重新下载。
- 自动发送默认关闭，需用户显式启用（`TelegramConfig.enabled` / `auto_send_on_archive`）。

`TG_METADATA_SENDING`/`TG_METADATA_SENT`/`TG_MEDIA_UPLOADING` 三个状态目前只作为**历史 schema 值**保留在 `jobs` CHECK constraint 与前端映射中，当前代码不再进入它们；第一版本也不在下载前发送 metadata。删除这些取值属于后续 schema 变更，不在本批次范围。

早期 Job 流程中的 `TG_METADATA_SENT` 等状态与“TG_METADATA_SENT + 下载失败”的旧重试规则不再是当前流程；不得据此实现或描述当前归档状态机。

Telegram 的进度不再用 Job 状态表达，而是独立的 outbox 状态机（`xarchive_telegram::OutboxState`）：

```text
QUEUED → IN_FLIGHT → SENT
                  ├─ RETRY_WAIT
                  ├─ FAILED_PERMANENT
                  └─ UNKNOWN
QUEUED → CANCELLED
```

其中 `UNKNOWN` 表示请求可能已被 Telegram 接受但响应丢失，必须人工复核，**默认不自动重发**（本地幂等不等于远端 exactly-once）。