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

### Windows worker OpenSSL 3 licence

The worker build includes the [upstream OpenSSL 3.5.8 licence](https://github.com/openssl/openssl/blob/openssl-3.5.8/LICENSE.txt) when it detects a `libcrypto-3*.dll`. The 3.0.0 and 3.6.0 upstream licence files matched the same SHA-256. See [Windows validation history](../validation/windows-validation-history.md) for the controlled DLL identity and the limits of this filename-based inclusion. Other bundled dependencies and complete corresponding source and release obligations remain open.

### Windows worker Python runtime

The controlled worker evidence in [Windows validation history](../validation/windows-validation-history.md) binds its Python 3.12 DLL to the build installation and records the adjacent Python `LICENSE.txt` hash. Windows worker build workflows copy that notice after checking DLL identity. The exact hosted-runner installation and complete worker dependency and redistribution inventory remain to be verified for each release; an externally supplied worker's notice provenance remains the operator's responsibility.

### Windows record-import identity (historical)

| Field | Value |
|---|---|
| Source | [BurntSushi/same-file](https://github.com/BurntSushi/same-file) |
| Pinned at | crates.io `same-file` 1.0.6, already present in `Cargo.lock` |
| Verified | 2026-11-16; installed package manifest and MIT licence |
| Use | Former direct Windows Desktop dependency; it may remain transitive |
| Licence | Unlicense OR MIT; MIT selected |
| Obligation | Include the upstream MIT copyright and permission notice in distributed artifacts; the full release dependency inventory remains open |

### Windows full-width file identity

| Field | Value |
|---|---|
| Source | [Maiori44/fs-id](https://github.com/Maiori44/fs-id) |
| Pinned at | crates.io `fs-id` 0.2.0, SHA-256 `f1648c1b89e98d042d61b5c1ed0fbcd9dbd7815163864d20f8e7976a9b3af938` |
| Verified | 2026-11-16; crate manifest, Windows implementation and zlib licence |
| Use | Windows Desktop record-import root identity from a pinned file handle; `redistribute` when linked |
| Licence | zlib |
| Obligation | Preserve the upstream notice in source distributions; do not misrepresent origin or altered source |
| API reference | [Microsoft `FILE_ID_INFO`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info) supplies a volume serial and 128-bit file identifier |

### Windows Job Object bindings

| Field | Value |
|---|---|
| Source | [MaulingMonkey/firehazard](https://github.com/MaulingMonkey/firehazard), [abibool](https://github.com/MaulingMonkey/abibool), and [abistr](https://github.com/MaulingMonkey/abistr) |
| Pinned at | crates.io `firehazard` `0.0.0-2022-09-10`, `abibool` `0.5.3`, `abistr` and `abistr-macros` `0.2.0-rc3` as locked in `Cargo.lock` |
| Verified | 2026-11-16; installed crate manifests and MIT licence files |
| Use | Windows-only dependency; `redistribute` when linked into Windows artifacts |
| Licence | Apache-2.0 OR MIT; MIT selected |
| Obligation | Include the relevant upstream MIT copyright and permission notices in distributed artifacts; release packaging and final dependency scan remain open |
| Notes | `bytemuck_derive` `1.12.1` is a transitive build-time dependency, Zlib OR Apache-2.0 OR MIT, MIT selected; its manifest and MIT licence were also checked. |

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
| Pinned at | documentation and release notes read 2026-10-01; Memory Bank and Cline Rules pages read 2026-11-11; no source code vendored |
| Verified | 2026-11-11 |
| Use | `reference` |
| Licence | Apache-2.0 for the repository; documentation site terms apply to prose |
| Obligation | See note below |

Read to confirm how `AGENTS.md` is discovered, where skills live, and how release lines are versioned. The 2026-11-11 session also read the Memory Bank best-practice page and the Cline Rules and `/newtask`/`/smol` command pages to decide whether this repository needs a separate memory-bank structure. Only behaviour was adopted, rewritten in this repository's own words; no file structure, heading set or template was reproduced. No Cline source or documentation text was copied verbatim into these files.

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
| Pinned at | Windows release asset `2026.09.20`, SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a`, in `desktop/scripts/windows-external-assets.json` |
| Verified | release asset pin recorded below; licence/source obligations **unresolved** |
| Use | `redistribute` (planned, not yet confirmed) |
| Licence | GPL-2.0 per the upstream repository; the upstream README notes active development moved to Codeberg |
| Obligation | Block release until corresponding source, licence text and redistribution obligations are resolved |

gallery-dl is invoked by the Sidecar and is planned for bundling. The 2026-11-17 Windows fixture matched the pinned asset. The upstream [2026.09.20 release metadata](https://api.github.com/repos/gdl-org/builds/releases/tags/2026.09.20) associates that asset with gallery-dl 1.32.13 at [source commit `61070f02d387494ac3a2bb5bfd8345aba85b1303`](https://codeberg.org/mikf/gallery-dl/commit/61070f02d387494ac3a2bb5bfd8345aba85b1303); its [LICENSE](https://codeberg.org/mikf/gallery-dl/raw/commit/61070f02d387494ac3a2bb5bfd8345aba85b1303/LICENSE) text SHA-256 is `8177f97513213526df2cf6184d8ff986c675afb514d4e68a404010521b880643`. A [GitHub mirror archive of that exact commit](https://api.github.com/repos/mikf/gallery-dl/tarball/61070f02d387494ac3a2bb5bfd8345aba85b1303) was retrieved on 2026-11-17 (observed archive SHA-256 `4dda5c5ead73951af9c9e247df52a66da4d5c850f4efbbdc73c1b5085a84605e`), and its LICENSE matched the referenced text. The current Windows Full release packaging includes that project licence only after verifying its hash and the pinned executable; portable Full verifies notices for the pinned asset and requires an operator-provided project licence for other assets. Static inspection found bundled `websockets` 17.1 while this source snapshot specifies 16.0 in `requirements/windows`. The [PyPI 17.1 source and Windows wheel](https://pypi.org/pypi/websockets/17.1/json) were independently hash-verified; 52 embedded Python modules matched selected source-compiled bytecode fields, and the native extension and metadata matched the wheel. The Windows release and portable flows now include the verified websockets licence for the pinned executable.
The pinned asset also contains certifi 2026.7.22: its two Python modules matched selected upstream source code-object fields, and its certificate bundle matched byte-for-byte. The Windows packaging now includes the upstream certifi licence for that exact asset; see [Windows validation history](../validation/windows-validation-history.md).
The pinned asset also contains urllib3 2.8.0; 36 embedded modules matched selected upstream source code-object fields. Windows packaging includes its upstream licence for that exact asset; see [Windows validation history](../validation/windows-validation-history.md).
The pinned asset also declares Cryptodome 3.23.0: 68 embedded modules matched selected source code-object fields and 42 native extensions matched the official Windows wheel. Windows packaging includes the upstream main licence for the exact asset; distinct legal files and other obligations remain under review. See [Windows validation history](../validation/windows-validation-history.md).

The pinned asset embeds Python 3.14.7: its `python314.dll` matches the
[official 64-bit embeddable package](https://www.python.org/downloads/release/python-3147/)
byte-for-byte. Packaging includes that package's licence for the pinned asset;
other components and complete redistribution review remain open.

The pinned asset's `yt-dlp-ejs` 0.8.0 modules and solver files have scoped
correspondence with its [PyPI wheel](https://pypi.org/pypi/yt-dlp-ejs/0.8.0/json).
Packaging includes the wheel's project licence and the astring 1.9.0 and
meriyah 6.1.4 notices. Their full texts match the embedded library header after
whitespace normalization; full build provenance and terms remain under review.

The pinned asset's 19 `requests` 2.34.2 modules match selected fields compiled
from its [PyPI source](https://pypi.org/pypi/requests/2.34.2/json); packaging
includes its upstream licence. Other dependencies and terms remain under review.

The pinned asset's six idna 3.20 and six truststore 0.10.4 modules each match
selected source fields; packaging includes both upstream licences. See
[Windows validation history](../validation/windows-validation-history.md).

PyYAML 6.0.3, charset-normalizer 3.5.1, mutagen 1.48.1 and PySocks 1.7.1
also have scoped Python source correspondence; PyYAML and charset-normalizer
native extensions match their PyPI Windows wheels. Their source licences are
included; see [Windows validation history](../validation/windows-validation-history.md).

The pinned asset's 1,043 yt-dlp 2026.8.19 modules match selected source code-object fields.
Windows packaging includes the upstream licence for this exact asset; other files,
complete build provenance and redistribution obligations remain under review.
The build-requirement discrepancy and other components remain open; see [Windows validation history](../validation/windows-validation-history.md). This identifies a source snapshot and project licence text, not the complete corresponding source, build inputs or a redistribution determination; those obligations remain open.

This entry stays open. It is the single largest unresolved redistribution obligation in the project.

### aria2

| Field | Value |
|---|---|
| Source | `aria2/aria2` |
| Pinned at | Windows `release-1.37.0` x64 build1 archive, SHA-256 `67d015301eef0b612191212d564c5bb0a14b5b9c4796b76454276a4d28d9b288`, in `desktop/scripts/windows-external-assets.json` |
| Verified | release archive pin recorded below; licence/source obligations **unresolved** |
| Use | `redistribute` (planned) |
| Licence | GPL-2.0 per upstream |
| Obligation | Block release until corresponding source, licence text and redistribution obligations are resolved |

aria2 is planned for bundling as the media transfer backend. The 2026-11-17 Windows fixture matched the pinned archive member. The upstream [release tag](https://github.com/aria2/aria2/releases/tag/release-1.37.0) resolves through annotated tag object `eb7232465119e1a596cb424e2b6c3c9f4e2abb30` to [source commit `02f2d0d8472b3c38c29b4dba8c75ebd5fdd2899a`](https://github.com/aria2/aria2/tree/02f2d0d8472b3c38c29b4dba8c75ebd5fdd2899a); its [COPYING](https://raw.githubusercontent.com/aria2/aria2/02f2d0d8472b3c38c29b4dba8c75ebd5fdd2899a/COPYING) text SHA-256 is `8177f97513213526df2cf6184d8ff986c675afb514d4e68a404010521b880643`. A [source archive for that commit](https://api.github.com/repos/aria2/aria2/tarball/02f2d0d8472b3c38c29b4dba8c75ebd5fdd2899a) was retrieved on 2026-11-17 (observed SHA-256 `fc3475683aa88df5764f6c5faa5f45b9d3dd23cca7a02de2ba6eee868edf5ce9`), and its COPYING matched the referenced text. The pinned archive also contains `AUTHORS`, `COPYING`, `LICENSE.OpenSSL`, `README.mingw`, `README.html`, `NEWS` and `ChangeLog`; its Windows README lists statically linked gmp 6.3.0, expat 2.5.0, sqlite 3.43.1, zlib 1.3, c-ares 1.19.1 and libssh2 1.11.0. The Windows release workflow copies these files from the verified archive; portable packaging requires same-named nonempty files beside the binary but does not verify their provenance or bytes. See [Windows validation history](../validation/windows-validation-history.md). The release reference and accompanying files do not establish the build1 binary's full corresponding source or dependencies, or a redistribution determination; review remains open.

### PyInstaller and the bundled worker

| Field | Value |
|---|---|
| Source | PyInstaller, plus the Python runtime and third-party packages collected into the worker |
| Pinned at | `pyinstaller==6.22.3` |
| Verified | 2026-10-01 — **status unresolved** |
| Use | `redistribute` (planned, bundled worker artifact) |
| Licence | PyInstaller GPL-2.0 with an exception; the bundled Python runtime and collected packages carry their own licences |
| Obligation | Block release until the collected set is enumerated and each licence recorded |

The worker ships as a PyInstaller onedir bundle containing a Python interpreter and collected packages. One Windows worker built at `0711a74` has a recorded onedir inventory, including Python 3.12.14, OpenSSL `libcrypto` and Microsoft `VCRUNTIME140`; its Full fixture notice did not have component-specific entries for these (`docs/validation/windows-validation-history.md`). This is a scoped inventory, not a licence review: collected contents can change with the build environment, and the corresponding terms and source/redistribution obligations remain open.

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
| 1 | gallery-dl licence and source | GPL-2.0 redistribution; Windows binary pinned, source and licence obligations open | Release + Windows |
| 2 | aria2 licence and source | GPL-2.0 redistribution; Windows archive pinned, source and licence obligations open | Release + Windows |
| 3 | PyInstaller worker collected licences | One Windows build inventoried; component terms and other builds remain unreviewed | Release + Windows |
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
