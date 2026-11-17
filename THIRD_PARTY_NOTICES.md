# Third-party notices

本文件记录当前项目**开发过程中使用或计划使用**的第三方组件。正式发行包的许可证清单必须以实际捆绑文件、锁定版本和许可证扫描结果为准，本文件不替代该扫描。

逐项来源、固定版本、使用性质与许可证状态见 [`docs/references/external-sources.md`](docs/references/external-sources.md)。该文件区分「参考」「改编/补丁」与「再分发」，并显式记录未决义务；本文件只做面向读者的摘要。

## Runtime and downloader components

- **gallery-dl**：Sidecar 使用的 X extractor/downloader；当前通过运行环境提供，正式打包版本、许可证副本和源代码获取方式待发布阶段确认。
- **aria2**：可选的下载传输和 Windows artifact；当前默认下载路径仍为 gallery-dl，是否随安装包分发需要单独确认 GPL-2.0 义务。

### Planned distribution policy

目标发布策略（尚未完成）为：固定版本、固定 SHA-256、官方来源和随 Release 资产生成的许可证文件。

- gallery-dl：官方 Codeberg stable release；目标架构中仅负责 extraction；
- aria2：官方 GitHub stable release；目标架构中作为唯一媒体 transfer backend；
- Worker、Native Host、Extension：来自 `https://github.com/15699122/Tw2Tg/releases` 的版本化 Release assets；
- 每个正式版本必须同时记录精确版本、来源、SHA-256、许可证文本和源码获取方式。

Windows release 资产的版本与哈希已记录在 `desktop/scripts/windows-external-assets.json`；来源、许可证文本和再分发义务仍按 [`docs/references/external-sources.md`](docs/references/external-sources.md) 跟踪。本节不构成已发布组件清单。

## Application dependencies

- **same-file 1.0.6**: transitive dependency; Unlicense OR MIT
  (MIT selected), copyright (c) 2017 Andrew Gallant. Preserve its upstream
  MIT notice in distributions containing it; see `docs/references/external-sources.md`.

- **fs-id 0.2.0**: Windows record-import root identity; zlib licence. The
  upstream notice follows:

  > zlib License
  >
  > (C) 2024 Felice D'Angelo
  >
  > This software is provided 'as-is', without any express or implied
  > warranty.  In no event will the authors be held liable for any damages
  > arising from the use of this software.
  >
  > Permission is granted to anyone to use this software for any purpose,
  > including commercial applications, and to alter it and redistribute it
  > freely, subject to the following restrictions:
  >
  > 1. The origin of this software must not be misrepresented; you must not
  >    claim that you wrote the original software. If you use this software
  >    in a product, an acknowledgment in the product documentation would be
  >    appreciated but is not required.
  > 2. Altered source versions must be plainly marked as such, and must not be
  >    misrepresented as being the original software.
  > 3. This notice may not be removed or altered from any source distribution.

- **firehazard 0.0.0-2022-09-10**, **abibool 0.5.3**, **abistr
  0.2.0-rc3**, and **abistr-macros 0.2.0-rc3**: Windows Job Object
  integration dependencies, Apache-2.0 OR MIT (MIT selected). Copyright
  (C) 2022 MaulingMonkey for firehazard and (C) 2020 MaulingMonkey for the
  other three crates. Preserve their upstream MIT licence notices in any
  distribution containing them; see the pinned entries in
  `docs/references/external-sources.md`.
- **bytemuck_derive 1.12.1**: build-time dependency selected by firehazard;
  Zlib OR Apache-2.0 OR MIT (MIT selected), copyright (c) 2019 Daniel
  "Lokathor" Gee. Preserve the applicable upstream notice in the release
  dependency and licence inventory.

- **os-proxy-resolver (microsoft/os-proxy-resolver, commit `796b027`)**: Windows-only
  system proxy resolver covering the static system proxy, bypass lists, PAC, and
  WPAD; MIT. Copyright (c) Microsoft Corporation. The final Windows distribution
  must include the upstream MIT permission notice; see the pinned source and
  licence entry in `docs/references/external-sources.md`.

- **keyring 3.6.3**: Windows-only Credential Manager provider; MIT OR Apache-2.0
  (MIT selected). Copyright (c) 2016 keyring Developers. The final Windows
  distribution must include the upstream MIT permission notice; see the pinned
  source/license entry in `docs/references/external-sources.md`.

- Rust crates、Tauri、React、Vite、Node packages 和 Python packages 的许可证应按 `Cargo.lock`、`package-lock.json`、Python 环境和最终 SBOM 扫描结果维护。
- 本项目源代码使用 MIT License，见根目录 `LICENSE`；第三方许可证不因项目许可证声明而改变。

## Release checklist

正式分发前必须补充：

1. 实际捆绑组件和精确版本；
2. 每个组件的许可证文本；
3. 需要提供的源代码或获取方式；
4. 本地修改和构建选项记录；
5. 许可证扫描和法律审查结果。

未决项按风险排序列于 [`docs/references/external-sources.md`](docs/references/external-sources.md) 的「未决义务」小节；其中 gallery-dl、aria2 与 PyInstaller worker 的再分发义务在关闭前不应视为已解决。
