# 架构决策记录

## ADR-001：Rust 是唯一业务状态所有者

**状态：已接受**

SQLite、Job 状态、文件提交、去重、Telegram 和恢复全部由 Rust 管理。Python 只做受控执行，避免多个事实来源不一致。

## ADR-002：使用独立 Native Host

**状态：已接受**

浏览器连接极小的 Rust Bridge，Bridge 再通过 Windows Named Pipe 连接 Desktop。Tauri 进程不直接作为浏览器 Native Host。

## ADR-003：Python Sidecar 使用 JSONL stdio

**状态：已接受**

不开放 Sidecar localhost HTTP，减少端口、鉴权和进程暴露。stdout 是机器协议，stderr 是诊断日志。

## ADR-004：gallery-dl 是默认 X extractor 和下载器

**状态：已接受**

gallery-dl 负责适应 X 变化、读取浏览器登录状态、提取 metadata 和第一版媒体下载。

## ADR-005：aria2 只作为可选 DownloadTransport

**状态：条件接受**

aria2 的 RPC、断点续传和进度适合大直链文件，但不理解 Tweet，也可能扩大 Cookie/Header 传播范围。M1.5 完成技术 Spike 后再决定是否正式启用。

## ADR-006：IDM 不作为核心后端

**状态：已拒绝作为核心**

官方 CLI 能提交 URL，但缺少足够的公开结构化状态接口来可靠映射 Job 状态、进度、失败和恢复。未来只考虑实验性外部提交。

## ADR-007：原始媒体优先

**状态：已接受**

本地保存原文件，不默认转码，不默认删除 hash 重复文件。