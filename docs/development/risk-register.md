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
| Windows rustfmt 缺失 | P2 | 不修改全局工具链；在开发镜像或 CI 安装 rustfmt 后执行格式检查 |
| Windows 集成验证尚未开始 | P1 | 先实现 Named Pipe/Native Host/Tauri 打包，再按验收清单验证 |

详细 Windows 任务分解和当前状态见 [`windows-validation.md`](windows-validation.md)。