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

本轮没有发起新的旧 `pre.6` 构建；暂停依据是历史同源构建失败以及旧 workflow 的覆盖风险，不能将其写作本轮构建失败。拟议编号 `pre.5` 与 `pre.6` 当前仍被保留的历史 tag 占用，`pre.7` 当前也对应有资产的旧 Release；迁移前必须先设计不覆盖历史身份和下载链接的 tag/Release 过渡方案，不能直接创建同名 tag。

解除阻断需由 Windows Platform Owner 在隔离、不覆盖现有 Release 的流程中用旧 `pre.6` 精确源码完成 Windows 测试、Native Host/打包和来源校验；若历史源码/工具链无法重建，先决定是否放弃该版的连续序列目标。之后每个迁移版本需检查 tag/source SHA、包内文件名、资产清单、manifest、SHA256SUMS、下载文件实测哈希及 Release Notes 的旧→新对应关系；保留旧 run ID、历史失败状态与 GUI 未验收事实。不要重写历史 tag 指向。