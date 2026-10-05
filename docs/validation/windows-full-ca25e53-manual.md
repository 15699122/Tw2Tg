# Full 验证目录与 Extension／Telegram 手动步骤

日期：2026-10-05。代码 source：ca25e5379302431a3130436896735b8be2bb4744；构建时 Git HEAD：9f383c0f08d70079880a652c5b3203e6db1786f7（仅追加验证文档）。Full Desktop SHA-256：37344cb7777152af1baf76eec85480e432e43ceaf44baae8ee31f8c598fe1b5e。当前包是 0.2.1 dev 验证构建，manifest 的 v0.2.1 是版本元数据，不表示正式发布或验收通过。

目录：E:\Shiraishi\VSCode Workspace\Tw2Tg\dist-portable\XArchive-full-validation-ca25e53-20261005。
启动：目录内 xarchive-desktop.exe。不要同时运行旧 Downloads Full 或 target/release Desktop，以免混淆 Native Host 和端口。优先使用隔离 Windows 用户／浏览器 profile。首次启动用目录选择器创建／选择应用旁 download 目录。目录移动后必须重新注册／修复 Native Host。

## 构建及范围

Desktop canonical Full build 无 PORTABLE_ALLOW_BINARY_REUSE；Native Host cargo --locked --release；worker 为当前源码新 PyInstaller one-dir。包含 Extension、Native Host、worker 及 _internal Python runtime、gallery-dl 1.32.13:2026.09.20、aria2 1.37.0。外部资源由固定可信摘要校验；安装清单全部存在，browser-pairing.js 已声明并实际打包。worker hello/ready 与 shutdown 退出 0、两个 downloader --version 退出 0。

没有打包 Telegram Local Bot API Server、api_id/api_hash、Bot Token、Channel Chat ID 或浏览器账号。当前 Full 的自动配对／真实归档／Telegram 发送仍 NOT_RUN。构建日志与逐文件哈希：validation-artifacts/windows-full-ca25e53/。

## A. 浏览器 Extension

1. 启动上述 Full Desktop，在设置检查 Sidecar、gallery-dl 和 aria2 路径均来自同一 Full 目录；启动 Sidecar，预期握手完成且状态为运行中。若任一组件缺失，保存脱敏错误并暂停归档。
2. 在 Desktop Extension 区块点击“注册 / 修复 Native Host”。用独立 Edge／Chrome profile 开启开发者模式并“加载解压缩的扩展”，选择上述 Full 的 extension 子目录。不要继续使用仓库 extension 或旧 Downloads 包。核对 ID 为 iaajefkoanbkleojofoadeakelihbjne。两个浏览器分别验收，不把一个浏览器结果推广。
3. 本轮先验证自动 bootstrap 配对：在新 profile 中打开 popup／options，预期 Native Host 引导到已认证 WebSocket，Desktop 同时为已认证／connected。options 仍提供手动端口/token 控件，但手动填写不能证明自动配对通过。若自动路径失败，记录双端状态／错误；当前 options 保留端口/token 控件，但保存调用未显式选择 legacy mode，当前默认 automatic 会忽略 token；不要按旧包步骤填 token 来修复自动配对，也不要据此验收 legacy 路径。自动失败时先检查注册、目录、ID 和两端脱敏错误，交 Owner 排查。不要复制 token 到聊天、截图或日志。
4. 保持 Desktop 不重启，约 5 秒和 45 秒分别刷新两端状态。记录相近时间、端口、认证状态、Desktop 观察及脱敏计数；预期实时状态一致，不出现一端已认证而另一端 disconnected。确认 Desktop 刷新不折叠面板。
5. 在已授权测试帖子 https://x.com/thsottiaux/status/2105039482013757749 上点击扩展注入的保存／归档按钮；确认工作台创建任务并到本地归档完成，媒体可打开，目标位于 download 目录。重复提交同一帖子，核对未生成重复归档／任务／媒体；保留任务 ID、状态、文件 hash 和截图。遇登录／CAPTCHA 由用户处理。
6. 分别重启浏览器、正常退出重启 Desktop，再查看自动配对恢复和再次提交；记录端口／会话变化。自动路径应重新 bootstrap，不能拿旧一次性 token 的手动结果证明自动恢复。暂不要求真实发往 Telegram。安装/升级/卸载、便携目录移动、睡眠及异常恢复属于后续独立验收。

## B. Telegram Cloud 与测试 Channel

1. 使用专用测试 Bot（@Zlib_siina_bot 或 @SUltra_bot）及已有测试 Channel；确认 Bot 有发布消息权限。首次只测试一个 Bot。准备 Channel 数字 Chat ID（一般为 -100...；以实际目标为准），Topic ID 留空。Token 仅在应用内填写。
2. 展开 Telegram，确认“凭据服务：可用”。选择 Cloud HTTPS，API 地址 https://api.telegram.org，填写 Chat ID；先保持“启用 Telegram”和“归档完成后自动入队”关闭，点击“保存设置”。输入“新 Bot Token”，点击“验证并替换凭据”；预期已保存、输入框清空、Token 不回显。
3. 点击“检查认证”，再“检查目标”。预期成功并且目标是专用 Channel。明确确认后点击“发送测试消息…”一次，在 Channel 核对收到一条测试消息。错误／UNKNOWN 时先检查频道和日志，不连续重试。
4. 配置“启用 Telegram”和“归档完成后自动入队”，选择媒体模式后保存，再“启动发送”。从 Extension 提交一个尚未归档的授权样本：既有帖子重复提交可能不会产生新发送意图。预期先本地归档完成，再发送确认；频道正文、媒体和顺序正确。归档完成与发送确认分别记录；确认发送不等于接收端文件一致。
5. 重复归档同一帖子，检查本地去重及频道没有重复发送。分别测试纯文字、单图、2/10/11 媒体、长文本/emoji；原文件文档模式下载后比对 hash，展示模式不承诺字节一致。可先做一份小样本，完整媒体矩阵单独记录。
6. 正常重启后检查设置/presence 持久化和 sender 状态；测试“停止发送”、取消／重试及 UNKNOWN 复核。凭据替换／删除、同 Bot 轮换自动/确认策略、不同 Bot 队列隔离在专用账户和受控待发项中单独执行；失败候选不应破坏旧凭据，UNKNOWN 不自动重发。不要对正常任务做强杀测试。

## C. Telegram Local 与进阶队列

Local Bot API Server 尚未包含在本目录。Local、大文件及迁移测试须先按项目固定版本准备 loopback server 和 api_id/api_hash。Cloud/Local 切换使用“确认并迁移端点…”流程；先停止发送、核对待发状态，不靠直接改 URL 验收迁移。按现有 windows-manual-steps §K 完成 endpoint/proxy/redirect、崩溃与 heartbeat、cache、>50MB/上限、Unigram 接收矩阵。未就绪继续 NOT_RUN。

每项记录 source SHA、Full Desktop hash、Windows/浏览器/WebView2/缩放、具体步骤和 PASS/FAIL/NOT_RUN。无密钥截图。构建/启动检查不能关闭真实集成队列。
