# External Sources

Owner: Cross-platform Owner.
Status: `ACTIVE` — registry of external material this repository refers to, adapts, patches or redistributes.

This file records **what was used, from where, under which licence**. It is not a legal opinion, and it does not replace a licence scan of a release artifact. Where a licence could not be verified from the repository itself, the entry says so instead of guessing.

Two obligations are tracked separately:

- **Development use** — a dependency, reference or tool used while building. Recorded in the root [`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md).
- **Redistribution** — anything shipped inside a published asset. Unresolved redistribution entries block the release checklist in [`../release/release-checklist.md`](../release/release-checklist.md).

## How to read an entry

| Field | Meaning |
|---|---|
| Source | Upstream project and maintainer |
| Pinned at | Tag, commit or doc version the reference was made against |
| Verified | Date the licence was checked against that pinned reference |
| Use | `reference`, `short-quote`, `copy`, `adapt`, `patch`, `redistribute` |
| Licence | Licence of the pinned reference |
| Obligation | What this repository must do, or `none identified` |

`Use` is the important column. Adapting or redistributing carries obligations that merely reading a design does not.

## Entries

### Microsoft OS proxy resolver

| Field | Value |
|---|---|
| Source | [microsoft/os-proxy-resolver](https://github.com/microsoft/os-proxy-resolver), Microsoft Corporation |
| Pinned at | Git commit `796b027c9361bb407f2a8d9d79c56b2dc4a42ee2`; the crate declares version 0.1.0 but **is not published on crates.io**, so a version requirement cannot be used |
| Verified | 2026-10-04; `Cargo.toml`, `src/lib.rs`, `src/types.rs`, and `src/resolver.rs` read at the pinned commit |
| Use | dependency; `redistribute` when linked into Windows artifacts |
| Licence | MIT (`LICENSE.txt` in the repository) |
| Obligation | Preserve the Microsoft copyright and MIT permission notice in distributed artifacts; the repository also ships `ThirdPartyNotices.txt` covering the optional PAC engines, which are not enabled here |
| Scope | Windows target only. A backend-less build (`default-features = false`) delegates PAC/WPAD evaluation to WinHTTP; on non-Windows a backend is mandatory, so this dependency is not usable on Linux |
| Notes | The crate is pre-1.0 and its default precedence is environment variables before the OS configuration. That precedence and its fall-through to `DIRECT` on PAC failure are reported to the user in the settings page rather than being hidden |

### keyring Windows Credential Manager provider

| Field | Value |
|---|---|
| Source | [keyring 3.6.3](https://docs.rs/crate/keyring/3.6.3/source/), upstream keyring-rs |
| Pinned at | Cargo package `=3.6.3`, checksum in Cargo.lock |
| Verified | 2026-10-04; downloaded crate Cargo.toml and LICENSE-MIT inspected |
| Use | dependency; `redistribute` when linked into Windows artifacts |
| Licence | MIT OR Apache-2.0; MIT selected for notice obligations |
| Obligation | Preserve copyright and MIT permission notice in distributed artifacts; final release SBOM/notice inclusion pending |
| Scope | Windows target only, `windows-native`, explicit WinCredential constructor; no default/mock provider |


### Microsoft Edge extension and Native Messaging documentation

| Field | Value |
|---|---|
| Source | Microsoft Learn — Edge extension API support, manifest format, Native Messaging, and Local Network Access |
| Pinned at | Documentation pages reviewed 2026-10-02; pages are rolling documentation, not version-pinned |
| Verified | 2026-10-02 — static compatibility review only |
| Use | `reference` |
| Licence | Microsoft Learn content; no content copied |
| Obligation | None identified for factual reference |

Used to check Edge compatibility of the existing Manifest V3 / `chrome.*` extension APIs and Native Messaging design. The official Native Messaging page documents `chrome-extension://<id>/` allowed origins and current-user Windows registration; the API support page enumerates Edge-supported extension APIs; the manifest page documents MV3 fields. Microsoft's Local Network Access guidance says the restrictions do not currently apply to extensions, while explicitly noting the evolving scope/version of the feature. This is static documentation evidence, not Windows/Edge runtime acceptance. No code or text is copied.

### x-spider-mod-2026

| Field | Value |
|---|---|
| Source | `hureyqi/x-spider-mod-2026` |
| Pinned at | commit `4fd46b66269761e1109309c6f21ba573f4836444` |
| Verified | 2026-10-01 — licence **not** verified from this repository |
| Use | `reference` |
| Licence | `LICENSE_UNVERIFIED` |
| Obligation | See note below |

Used as a design reference when scoping the P1–P3 plan. The plan in [`../development/roadmap.md`](../development/roadmap.md) records both what was borrowed (paginated discovery, filtering, download management) and what was deliberately **not** adopted (hardcoded GraphQL, skipping downloads based only on file existence, logging full RPC parameters). No source file was copied into this repository.

Because no code was copied, this is `reference`, not `adapt`. The licence was not read from the upstream repository during this pass, so the entry stays `LICENSE_UNVERIFIED`. It does not block development. If any code is ever copied rather than reimplemented, this entry must be resolved to a specific licence and an attribution obligation before that copy is merged.

### @wdio/tauri-service and @wdio/native-core

| Field | Value |
|---|---|
| Source | WebdriverIO (`@wdio/tauri-service@1.4.0`, `@wdio/native-core@1.2.0`) |
| Pinned at | pinned versions in `package-lock.json` |
| Verified | 2026-10-01 — versions pinned; per-package licences resolved from `node_modules` at install time, not re-read here |
| Use | `patch` |
| Licence | MIT for the WebdriverIO packages |
| Obligation | See note below |

[`desktop/scripts/patch-wdio-tauri-service.mjs`](../../desktop/scripts/patch-wdio-tauri-service.mjs) rewrites two files inside the installed dependency tree at `postinstall` to fix an Edge driver version regex and a `shell: true` argument-splitting defect. It touches no product code, assertion, capability or driver version, is idempotent, and only modifies `node_modules`, which is never tracked.

This modifies a locally installed copy, not a redistributed artifact: no patched dependency file is committed to this repository or shipped in a release asset. It therefore does not create a redistribution obligation for this project. It does mean the installed tree differs from the published package, so a future release that bundles `node_modules` would need to disclose the patch.

The upstream defects should still be reported to WebdriverIO so the patch can be removed when fixed.
### Cline

| Field | Value |
|---|---|
| Source | `cline/cline` |
| Pinned at | documentation and release notes read 2026-10-01; no source code vendored |
| Verified | 2026-10-01 |
| Use | `reference` |
| Licence | Apache-2.0 for the repository; documentation site terms apply to prose |
| Obligation | See note below |

Read to confirm how `AGENTS.md` is discovered, where skills live, and how release lines are versioned. Only behaviour was adopted, rewritten in this repository's own words. No Cline source or documentation text was copied verbatim into these files.

Apache-2.0 permits reuse with attribution and NOTICE handling if content is actually copied. Because nothing was copied, the obligation is currently `none identified`. If Cline documentation text, skill templates or source are ever copied rather than reimplemented, record the exact file, the licence file and the attribution in this entry before merging.

### OpenAI Codex

| Field | Value |
|---|---|
| Source | `openai/codex` and the official Codex documentation |
| Pinned at | documentation and changelog read 2026-10-01; no source code vendored |
| Verified | 2026-10-01 |
| Use | `reference` |
| Licence | Apache-2.0 for the repository; product documentation has separate terms |
| Obligation | See note below |

Read for the same purpose as Cline: `AGENTS.md` layering, `AGENTS.override.md` precedence, the combined size limit, and the distinction between the stable CLI line and pre-release builds. Behaviour was adopted; no text was copied.

Same position as Cline: reimplementation only, so `none identified` today. Copying documentation text or source would require recording the exact file and licence.

### gallery-dl

| Field | Value |
|---|---|
| Source | `mikf/gallery-dl` |
| Pinned at | not pinned to a version in this repository |
| Verified | 2026-10-01 — **status unresolved** |
| Use | `redistribute` (planned, not yet confirmed) |
| Licence | GPL-2.0 per the upstream repository; the upstream README notes active development moved to Codeberg |
| Obligation | Block release until version, source, hash and licence text are fixed |

gallery-dl is invoked by the Sidecar and is planned for bundling. GPL-2.0 is a strong copyleft licence, so redistribution obligations are real and cannot be discharged by a notice alone. The distribution plan in the root `THIRD_PARTY_NOTICES.md` targets a Codeberg stable release, but nothing is pinned, no hash is recorded, and no licence text has been captured.

This entry stays open. It is the single largest unresolved redistribution obligation in the project.

### aria2

| Field | Value |
|---|---|
| Source | `aria2/aria2` |
| Pinned at | not pinned in this repository |
| Verified | 2026-10-01 — **status unresolved** |
| Use | `redistribute` (planned) |
| Licence | GPL-2.0 per upstream |
| Obligation | Block release until version, source, hash and licence text are fixed |

Same position as gallery-dl: planned for bundling as the media transfer backend, with a version allowlist and SHA-256 verification referenced in the Desktop aria2 commands, but no pinned version, recorded hash or captured licence text in this repository.

### PyInstaller and the bundled worker

| Field | Value |
|---|---|
| Source | PyInstaller, plus the Python runtime and third-party packages collected into the worker |
| Pinned at | `pyinstaller==6.22.3` |
| Verified | 2026-10-01 — **status unresolved** |
| Use | `redistribute` (planned, bundled worker artifact) |
| Licence | PyInstaller GPL-2.0 with an exception; the bundled Python runtime and collected packages carry their own licences |
| Obligation | Block release until the collected set is enumerated and each licence recorded |

The worker ships as a PyInstaller onedir bundle containing a Python interpreter and collected packages. Enumerating what is actually inside a built worker is required before any licence claim, because the collected set can change when dependencies change.

### tdlib/telegram-bot-api

| Field | Value |
|---|---|
| Source | `tdlib/telegram-bot-api` (official Telegram Bot API server) |
| Pinned at | `master`; latest commit `e3e9dd8e5b3d7ab8537cd5a10dc31d5ffa8f82d1` (2026-08-25) |
| Verified | 2026-10-01 — licence `BSL-1.0` read from the repository |
| Use | `reference` |
| Licence | BSL-1.0 |
| Obligation | See note below |

Referenced by [`../development/telegram-local-bot-api-plan.md`](../development/telegram-local-bot-api-plan.md) as the authoritative protocol and server baseline. No source is copied or vendored; the plan does not manage the server's lifecycle. A future version that bundles or redistributes the binary must re-read the BSL-1.0 terms, which are **not** an OSI-approved open-source licence in the general case.

### unigramdev/unigram

| Field | Value |
|---|---|
| Source | `unigramdev/unigram` (Windows Telegram client) |
| Pinned at | `develop`; recent push 2026-09-24 |
| Verified | 2026-10-01 — licence `GPL-3.0` read from the repository |
| Use | `reference` |
| Licence | GPL-3.0 |
| Obligation | See note below |

Used as the Owner's Windows receiving-client acceptance target and as a source of **test scenarios** (Issues #3244, #3453, #3299, #2490, #3306, #3292). No Unigram source is copied. GPL-3.0 makes any future code reuse a deliberate licence decision; behaviour and issue reproduction are not code reuse.

### Telegram Bot API container images

| Field | Value |
|---|---|
| Source | `aiogram/telegram-bot-api`, `bots-house/docker-telegram-bot-api`, `avbor/docker-telegram-bot-api` |
| Pinned at | aiogram latest commit `76970f45dfd2eb9d66af543526cd7b116b35558d` (2025-04-15); bots-house recent push 2024-07-16; avbor recent push 2026-08-28 |
| Verified | 2026-10-01 — `aiogram` licence **not detected via the GitHub API**; `bots-house` and `avbor` MIT |
| Use | `reference` |
| Licence | `LICENSE_UNVERIFIED` for `aiogram`; MIT for `bots-house` and `avbor` |
| Obligation | See note below |

Deployment references only. The plan requires a fixed server version and digest, and does not treat any image as automatically trusted or automatically maintained. The `aiogram` entry stays `LICENSE_UNVERIFIED` until its licence is read directly. `avbor` patches the upstream server to add proxy options, so it is a **server variant**, not official behaviour.

## Unresolved obligations

Ordered by risk. None of these are closed by this document.

| # | Item | Why it matters | Owner |
|---|---|---|---|
| 1 | gallery-dl licence and pinning | GPL-2.0 redistribution, not yet pinned or hashed | Release + Windows |
| 2 | aria2 licence and pinning | GPL-2.0 redistribution, not yet pinned or hashed | Release + Windows |
| 3 | PyInstaller worker collected licences | Bundle contents unknown until a real build is inspected | Release + Windows |
| 4 | x-spider-mod licence | Only matters if code is ever copied; currently reference-only | Cross-platform |
| 5 | Per-dependency licence scan | Root `THIRD_PARTY_NOTICES.md` still describes a plan, not a scan result | Release |

## Rules for this file

1. Add an entry before relying on external material, not after.
2. `Use` must be the strongest word that applies. `adapt` and `redistribute` are not downgraded to `reference` for convenience.
3. An unverified licence is written `LICENSE_UNVERIFIED`. It is never replaced by an assumption, and never by a link alone.
4. A pinned reference means a tag or commit. "Latest" is not a pin.
5. Redistribution entries that are still open appear in the release checklist until closed with evidence.
6. Changing an entry's `Use` to a weaker word requires a reason in the commit that makes the change.


## Windows release asset pins — 2026-10-05

Release-only Windows asset pins are recorded in `desktop/scripts/windows-external-assets.json` (implementation 39f54e5). gallery-dl 2026.09.20 Windows asset SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` was read independently from [official GitHub release API](https://api.github.com/repos/gdl-org/builds/releases/tags/2026.09.20). aria2 1.37.0 Windows x64 build1 ZIP retains existing reviewed `desktop/src-tauri/src/aria2.rs` pin `67d015301eef0b612191212d564c5bb0a14b5b9c4796b76454276a4d28d9b288`, downloaded from the official aria2/aria2 release. Actual downloaded bytes matched; hashes were not bootstrapped from those bytes. Runtime pins are committed and never replaced automatically by fetched metadata. Upstream project/license records above remain authoritative.
