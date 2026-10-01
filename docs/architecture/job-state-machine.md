# Job 状态机

## 主流程

```text
QUEUED → VALIDATING → METADATA_READY → TG_METADATA_SENDING → TG_METADATA_SENT
       → DOWNLOADING → DOWNLOADED → TG_MEDIA_UPLOADING → COMPLETE
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
| QUEUED/DOWNLOADING | 返回现有任务 |
| DOWNLOADED + Telegram 失败 | 只补传 Telegram |
| TG_METADATA_SENT + 下载失败 | 只重新下载 |
| COMPLETE | 不重复执行 |
| 本地文件丢失 | Re-download |
| Telegram 消息丢失 | Re-upload |

只有文件存在、路径安全、大小和 hash 校验成功、staging 已提交且数据库事务成功后，才能进入 `DOWNLOADED`。

## Telegram 发送与归档解耦（计划）

上表的 `TG_METADATA_SENDING` 位于下载之前，反映早期设计。按 [`../development/telegram-local-bot-api-plan.md`](../development/telegram-local-bot-api-plan.md) 第 2.1 节，Telegram 将被**移出归档主链路**：

- 本地归档成功独立成立，不依赖 Telegram 成功。
- Telegram 发送状态单独记录和展示。
- Telegram 不可用、失败或未配置都不阻塞归档。
- 仅补传 Telegram 时不触发重新下载。
- 自动发送默认关闭，需用户显式启用。

这是 shared 状态机变更，属于该 Plan 的 TG-00；在实现完成前，上表仍按当前代码描述现状。