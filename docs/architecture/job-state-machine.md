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