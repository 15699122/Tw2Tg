# Third-party notices

本文件记录当前项目使用或计划使用的第三方组件。正式发行包的许可证清单必须以实际捆绑文件、锁定版本和许可证扫描结果为准。

## Runtime and downloader components

- **gallery-dl**：Sidecar 使用的 X extractor/downloader；当前通过运行环境提供，正式打包版本、许可证副本和源代码获取方式待发布阶段确认。
- **aria2**：可选的下载传输和 Windows artifact；当前默认下载路径仍为 gallery-dl，是否随安装包分发需要单独确认 GPL-2.0 义务。

### Planned distribution policy

目标发布策略（尚未完成）为：固定版本、固定 SHA-256、官方来源和随 Release 资产生成的许可证文件。

- gallery-dl：官方 Codeberg stable release；目标架构中仅负责 extraction；
- aria2：官方 GitHub stable release；目标架构中作为唯一媒体 transfer backend；
- Worker、Native Host、Extension：来自 `https://github.com/15699122/Tw2Tg/releases` 的版本化 Release assets；
- 每个正式版本必须同时记录精确版本、来源、SHA-256、许可证文本和源码获取方式。

在实际固定版本、hash 和资产生成前，本节只表达计划，不构成已发布组件清单。

## Application dependencies

- Rust crates、Tauri、React、Vite、Node packages 和 Python packages 的许可证应按 `Cargo.lock`、`package-lock.json`、Python 环境和最终 SBOM 扫描结果维护。
- 本项目源代码使用 MIT License，见根目录 `LICENSE`；第三方许可证不因项目许可证声明而改变。

## Release checklist

正式分发前必须补充：

1. 实际捆绑组件和精确版本；
2. 每个组件的许可证文本；
3. 需要提供的源代码或获取方式；
4. 本地修改和构建选项记录；
5. 许可证扫描和法律审查结果。