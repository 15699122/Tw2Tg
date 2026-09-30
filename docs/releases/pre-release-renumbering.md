# Pre-release 清理与重编号状态（2026-09-29）

## 已完成：移除空 Release

核对 GitHub Release API 中的 `prerelease=true` 和资产数量为零后，删除了以下六个 **GitHub Release 对象**：`v0.1.1-pre.4`、`v0.2.0-pre.5`、`v0.2.0-pre.8`、`v0.2.0-pre.10`、`v0.2.0-pre.13`、`v0.2.0-pre.15`。保留对应 Git tag 和原始 `docs/releases/` 记录，以便追溯失败的构建与验证；旧 Release 链接不再有效，不得把历史记录解释为可下载资产。其他只有 tag、从未创建 GitHub Release 的 `v0.2.0-pre.9`、`.11`、`.12`、`.14` 也未被当作有资产 Release。

## 待执行：v0.2.0 有资产 Release 的连续编号

| 现有 Release | 拟议新编号 | 当前资产数 | 状态 |
|---|---|---:|---|
| `v0.2.0-pre.1` | `v0.2.0-pre.1` | 2 | 保持 |
| `v0.2.0-pre.2` | `v0.2.0-pre.2` | 4 | 保持 |
| `v0.2.0-pre.3` | `v0.2.0-pre.3` | 4 | 保持 |
| `v0.2.0-pre.4` | `v0.2.0-pre.4` | 4 | 保持 |
| `v0.2.0-pre.6` | `v0.2.0-pre.5` | 2 | **阻断：现有资产来源与 tag 不一致** |
| `v0.2.0-pre.7` | `v0.2.0-pre.6` | 7 | 待同源构建与校验 |
| `v0.2.0-pre.16` | `v0.2.0-pre.7` | 7 | 待同源构建与校验 |

**表中“拟议新编号”均尚未发布或写入 tag；现有编号仍是唯一有效的资产身份。** 旧 `pre.6` tag 源码为 `ac586e609337947aeb51de8f5cce3185efc8995e`；现有两个资产来自 `main` 手动 run `35518832674`，不能作为该 tag 的同源构建证据。历史同源 Windows run `35567742785` 的 Rust 测试与 Tauri build 通过，但旧 workflow 只从 Secret 读取 Extension ID，当前值位于 Repository Variable，Native Host 构建失败；完整资产和 manifest 未生成。该 workflow 的上传采用 `--clobber`，直接重试有覆盖现有资产的风险。依据用户指定的“先同源重建，失败则暂停”条件，**重编号暂停**；不得重命名污染资产、改动 manifest 中的 tag，或把旧 `pre.16` 的 WDIO FAIL 写成 PASS。

上一轮没有发起新的旧 `pre.6` 构建；当时的暂停依据是历史同源构建失败以及旧 workflow 的覆盖风险，不能将其写作该轮构建失败。拟议编号 `pre.5` 与 `pre.6` 当前仍被保留的历史 tag 占用，`pre.7` 当前也对应有资产的旧 Release；迁移前必须先设计不覆盖历史身份和下载链接的 tag/Release 过渡方案，不能直接创建同名 tag。

### 后续隔离构建验证（2026-09-29）

先前“没有发起新的构建”仅描述上一轮。本轮在 `release/v0.2.0` 提交 `078bdfbf7995fc16ea9cb58314931703ef5283dc` 增加无 Release 上传权限的 `.github/workflows/legacy-pre6-rebuild.yml`，并在默认分支 `main` 注册同一 workflow（注册提交 `1596d086da8011423514dbc0ae7c551893aa8ab6`）。Windows Actions run `36596395444` / job `109502297535` **PASS**：检出旧 `pre.6` 精确 tag 源码 `ac586e609337947aeb51de8f5cce3185efc8995e` 并校验 parity，Rust workspace tests、Tauri 和 Native Host 构建通过。Actions artifact `legacy-pre6-source-build-36596395444` 含桌面 exe、Native Host exe 和身份 JSON；下载后桌面 exe 实测 SHA-256 `4fcc100e63262fe9bdf50bf75e016aa6580ba146c020e23838b01724376eef4a`，与身份 JSON 一致。**这是隔离构建 PASS，不是完整资产/Release PASS**：未执行 worker、7z/extension 打包、manifest、SHA256SUMS 或 Windows GUI；原有污染资产未覆盖。

连续重编号仍未执行。后续先在无 `--clobber`、不上传现有 Release 的隔离 workflow 中完成剩余组件和五资产同源构建及逐文件/包内验证，然后规划占用中的历史 tag 和 Release 的保留方式；在新旧身份、下载链接及历史 run 可追溯方案落实之前不得复用旧编号或发布未经验证的迁移资产。

### 2026-09-30 隔离打包尝试：暂停（非发布）

分支提交 `57aae9fcccd0c9f33b9642cf3568dbaf83dd500e` 的隔离 Windows run `36650561763` 成功生成五个候选资产、manifest 和 SHA256SUMS，下载后五个哈希/大小与 manifest 匹配、四个 7z 可解压测试通过；但包内检查发现 Extension 含 `tests/` 和 `package.json`，Full/依赖包也含测试目录，而且 Full 缺少 gallery-dl/aria2。因此 **run PASS 不等于资产合格**；这批 Actions artifact 不可发布，桌面 exe SHA-256 为 `530f33c8dade50439013c2a29b004e07da6dc2f2131427d4a2da9ab359528ad4`，与上一轮 run 的 exe 哈希不同，不能混用身份记录。

提交 `f2915313f47e1fe53c52c0b6f1d9af935ba50fcf` 修正 Extension 打包过滤并增加外部组件及包内排除检查；但对应隔离 run `36651747470` 在 `cargo test --workspace` 的 `xarchive-sidecar-supervisor` 两项 hello handshake 测试超时 **FAIL**，尚未进入 Tauri、Native Host 或修正后的打包步骤。不能声称修正版五资产通过或把该失败写成打包失败。依照“失败则暂停”，未重新编号、未修改 Release Notes/tag/Release、未覆盖现有资产；后续由 Windows Owner 诊断并重试旧源码的间歇性测试失败，再逐项检查修正版包内结构、同源 SHA、下载哈希和历史 tag/链接迁移。Windows GUI 仍 NOT_RUN。

解除阻断需由 Windows Platform Owner 在隔离、不覆盖现有 Release 的流程中用旧 `pre.6` 精确源码完成 Windows 测试、Native Host/打包和来源校验；若历史源码/工具链无法重建，先决定是否放弃该版的连续序列目标。之后每个迁移版本需检查 tag/source SHA、包内文件名、资产清单、manifest、SHA256SUMS、下载文件实测哈希及 Release Notes 的旧→新对应关系；保留旧 run ID、历史失败状态与 GUI 未验收事实。不要重写历史 tag 指向。