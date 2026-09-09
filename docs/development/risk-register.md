# 风险登记

| 风险 | 等级 | 缓解策略 |
|---|---:|---|
| X DOM 改版 | P0 | 所有 selector 集中在 XDomAdapter；DOM 仅作辅助来源 |
| gallery-dl 认证/提取变化 | P0 | 锁定版本、Adapter 隔离、保留 extractor metadata |
| Edge Cookie 读取失败 | P0 | M0 早期验证 Edge Profile 和 AUTH_REQUIRED 映射 |
| Sidecar stdout 被日志污染 | P0 | stdout 仅 JSONL，stderr 统一日志 |
| Windows Named Pipe/Registry 差异 | P1 | 跨平台 framing/协议已完成；Windows transport、ACL、Registry 和实机验证保持独立，Host 保持极小 |
| aria2 URL/认证过期 | P1 | 403 重新提取；复杂认证回退 gallery-dl |
| aria2 GPL 分发义务 | P1 | M1.5 完成许可证清单和法律审查 |
| Telegram 限制变化 | P1 | `xarchive-telegram` 已固化请求模型、格式化、长文本、media group 和 `reqwest 0.13.4` + Rustls HTTPS transport 测试；API 限制、发送持久化和真实账号验证待后续完成 |
| 文件与数据库状态不一致 | P0 | staging、事务、事件和启动恢复 |
| Token/Cookie 泄露 | P0 | SecretStore abstraction、BotToken 脱敏和 Extension 消息边界已测试；Credential Manager backend、真实 Cookie 读取和端到端日志审查待 Windows/账号环境 |
| IDM 状态不可观测 | P1 | 不作为核心后端，仅考虑外部提交 |
| Windows clippy `large_enum_variant` | P2 | 已将 `SupervisorEvent::Download` 改为 `Box<DownloadEvent>`，原问题已排除；Telegram formatter 的 `single_char_add_str` 已修复，Windows 完整 workspace clippy 已通过 |
| Windows Tauri 正式图标与打包验证 | P1 | 已补齐开发阶段 `desktop/src-tauri/icons/icon.ico`，Windows Debug/Release 编译及 `npm run build:tauri` 通过；正式图标集、bundle 和安装器仍需验证 |
| Windows Tauri GUI 验证入口缺失 | P1 | Tauri CLI 2.11.4 已安装；`npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop 可执行文件，`npm run build:tauri` 生成 Release 可执行文件。停止开发进程时有 Chromium `Error = 1411` 注销警告；UI 自动化 helper 仍初始化失败，GUI/安装器视觉验证待完成 |
| Windows Rust 存储测试偶发路径错误 | P2 | 一次并行验证中出现路径不存在；目标测试单独重跑及串行完整 workspace 通过，继续观察并行运行稳定性 |
| gallery-dl 真实 X 提取/认证未验证 | P0 | 已记录 gallery-dl 1.32.11 和公开示例失败链路；在具备明确账号环境后验证 Edge Profile、AUTH_REQUIRED 和真实媒体归档 |
| Windows 专属集成尚未实现 | P1 | Named Pipe、Native Host transport、Registry、Credential Manager、Tray/Autostart、externalBin 和安装器继续作为平台适配项，按 Windows 清单逐项实现和验证 |
| Linux 验证工具缺失 | P2 | 当前环境缺少 `pytest` 和 `cargo-clippy`；本轮使用 Rust fmt/check/test、Node check/test/build 和 JSON/schema 校验。Windows Python 10 项测试及完整 clippy 均通过；Supervisor 测试需显式设置 `PYTHON` 指向项目 `.venv\Scripts\python.exe` |

详细 Windows 任务分解和当前状态见 [`windows-validation.md`](windows-validation.md)。
