# 开发路线图

> 本文只记录未来方向、依赖和完成标准。当前实现事实以 [`status.md`](status.md) 为准；Windows 验证队列以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准。

## R1：应用编排与并发模型

### 目标

将归档请求从 Tauri command 中的长时间同步流程，演进为可观测、可取消、可恢复的后台 Job executor。

### 依赖

- 当前 `RuntimeState`、Database、SidecarSupervisor 和 FileStore 所有权梳理。
- 明确 Job 状态转换、取消语义和 Sidecar 生命周期。
- 先完成 application service API 和 ADR，再改变运行时行为。

### 完成标准

- Tauri command 只负责输入校验、创建/查询 Job 和发送控制请求。
- 网络、Sidecar、文件和 Telegram I/O 不在全局 RuntimeState 锁内执行。
- Job 状态、事件和错误在后台执行期间可查询。
- stop/cancel、Sidecar 崩溃和应用退出有明确结果。
- 增加并发、取消、失败恢复和重复请求测试。

## R2：DownloadRouter 与真实传输接入

### 目标

完成 gallery-dl 默认路径与 aria2 fallback 的应用级接入，而不仅是 crate 级路由策略。

### 依赖

- R1 的后台 Job executor。
- 受控的 aria2 executable 和本地 media fixture。
- 明确 403/过期 URL 的重新提取边界。

### 完成标准

- gallery-dl 仍是默认 extractor/downloader。
- 只有明确允许且有新鲜 URL 时才使用 aria2。
- 403 或 URL 过期时可重新提取，不复用过期 URL。
- aria2 transfer 状态、Job 状态、事件和文件提交一致。
- fallback 失败时保留 gallery-dl 与 aria2 两侧安全错误信息。

## R3：Native Host 与 Windows IPC

### 目标

完成 Native Host 到 Desktop 的 Windows Named Pipe、ACL、安装和浏览器连接链路。

### 依赖

- 跨平台 protocol/framing API 稳定。
- Windows Named Pipe server 实现和权限设计。
- Native Host manifest、Registry 和固定 Extension ID。

### 完成标准

- 合法 BrowserRequest 可转发到 Desktop 并返回匹配 request_id 的 BrowserResponse。
- 非法协议、越权连接、断线、重连和关闭有确定行为。
- Edge/Chrome 安装、升级和卸载路径可重复。
- Windows ACL 不允许无关进程访问业务管道。

## R4：真实账号与凭据边界

### 目标

完成 Edge Cookie、Credential Manager 和 Telegram 真实账号链路。

### 依赖

- 受控测试账号、Edge Profile、Telegram test chat 和可用网络。
- SecretStore 与 Windows Credential Manager adapter。
- 日志、错误 UI、SQLite 和进程输出脱敏审查。

### 完成标准

- Cookie、Bot Token 和 RPC secret 不进入 Extension、SQLite 或普通日志。
- AUTH_REQUIRED、限流、网络失败和重试状态可诊断。
- Telegram 发送状态跨重启可恢复且幂等。
- 真实账号验证结果与单元/fake-server 结果分开记录。

## R5：文件系统、恢复和隐私边界

### 目标

完成应用级文件数据库恢复、稳定数据目录、ACL 和异常文件系统场景。

### 依赖

- R1 的 Job executor 和应用退出语义。
- 明确 archive root、应用数据目录和用户选择目录的职责。
- Windows 文件数据库、第二用户和 reparse fixture。

### 完成标准

- 旧 migration 可在真实文件数据库中升级并保持数据一致。
- 应用重启、遗留 staging、WAL/SHM 和异常退出可恢复或明确失败。
- symlink/junction/reparse、长路径、Unicode、空格和文件锁场景有测试。
- 归档数据权限符合产品隐私约定。

## R6：打包、安装和桌面体验

### 目标

完成 externalBin、bundle、Native Host 安装、GUI Windows 验收和发布基础设施。

### 依赖

- R3、R4、R5 完成或有明确替代方案。
- Sidecar、aria2 和 Native Host 的分发许可证确认。
- Windows 签名、安装器、WebView2 和辅助技术环境。

### 完成标准

- 安装、升级、卸载、回滚和数据保留可重复。
- Sidecar、Native Host 和配置资源在安装后可定位。
- WebView2、DPI、键盘、Focus-visible、屏幕阅读器和对比度通过实机验收。
- 发布包包含完整许可证、版本和源代码获取信息。

## 依赖顺序

```text
R1 应用编排
  → R2 下载接入
  → R3 Native Host/IPC
  → R4 账号与凭据
  → R5 文件恢复与隐私
  → R6 打包与发布
```

Windows-specific 项目在 Linux 继续实现时统一加入 [`../validation/windows-queue.md`](../validation/windows-queue.md)，不得因普通 pending 项目提前中断 Linux development phase。