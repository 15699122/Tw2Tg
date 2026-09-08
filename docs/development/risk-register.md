# 风险登记

| 风险 | 等级 | 缓解策略 |
|---|---:|---|
| X DOM 改版 | P0 | 所有 selector 集中在 XDomAdapter；DOM 仅作辅助来源 |
| gallery-dl 认证/提取变化 | P0 | 锁定版本、Adapter 隔离、保留 extractor metadata |
| Edge Cookie 读取失败 | P0 | M0 早期验证 Edge Profile 和 AUTH_REQUIRED 映射 |
| Sidecar stdout 被日志污染 | P0 | stdout 仅 JSONL，stderr 统一日志 |
| Windows Named Pipe/Registry 差异 | P1 | Windows CI/实机验证，Host 保持极小 |
| aria2 URL/认证过期 | P1 | 403 重新提取；复杂认证回退 gallery-dl |
| aria2 GPL 分发义务 | P1 | M1.5 完成许可证清单和法律审查 |
| Telegram 限制变化 | P1 | M2 按官方文档和集成测试固化 |
| 文件与数据库状态不一致 | P0 | staging、事务、事件和启动恢复 |
| Token/Cookie 泄露 | P0 | SecretStore、日志脱敏、禁止进入协议和 SQLite |
| IDM 状态不可观测 | P1 | 不作为核心后端，仅考虑外部提交 |
| Windows clippy `large_enum_variant` | P2 | 已将 `SupervisorEvent::Download` 改为 `Box<DownloadEvent>`；在 Windows 环境重新运行 lint/测试确认 |
| gallery-dl 真实 X 提取/认证未验证 | P0 | 使用项目本地 `.venv` 锁定版本；在具备明确账号环境后验证 Edge Profile、AUTH_REQUIRED 和真实媒体归档 |
| Windows 专属集成尚未实现 | P1 | 先实现 Named Pipe/Native Host/Tauri 打包，再按验收清单验证 |

详细 Windows 任务分解和当前状态见 [`windows-validation.md`](windows-validation.md)。
