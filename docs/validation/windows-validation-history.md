# Windows Validation History

## Fresh Windows Desktop and record-import path boundaries — 2026-11-19

At source `5d1dfd5` on `codex/windows-validation-4ae46ed3`, `npm run build:tauri --workspace desktop -- --no-bundle --ci` built a fresh dev-channel optimized Desktop, SHA-256 `49b613db288f800b076bc46316be80a9abad9542f992bb6e3b804e89340a9a19`; build log and exit are `target/w6-current-release-5d1dfd5.{log,exit}`. Tauri rewrote `desktop/src-tauri/Cargo.toml` line endings; its Git-normalized hash still matched HEAD. The isolated Full assembly `target/w6-current-full-5d1dfd5/dist-portable/Full-current` reused the Native Host built at `67d1a3f`, worker at `7b7bd7e` and pinned downloader fixtures; assembly evidence is `target/w6-current-full-5d1dfd5.{ps1,log,exit}`. This is not a fresh build of every component or a released artifact.

On Windows 10.0.29708, WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, actual GUI preview rejected each of `../escape`, `..\escape` and `C:\escape`, reported zero acceptable records and kept acceptance disabled; a subsequent valid synthetic record preview was selectable. **PASS scoped 1/1** (`target/w7-path-boundary-5d1dfd5.{ps1,e2e.mjs}` and `target/w7-path-boundary-5d1dfd5/{runner.log,runner.exit,app.sha256}`). No import or real downloader was run. Reparse points, other path cases, full GUI acceptance and the W1/W2-gated lifecycle remain open.

## Windows record-import parent traversal — 2026-11-19

At clean tracked test source `65819d3`, the previously built Desktop (source `67d1a3f`, SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf`) ran in an isolated Full fixture on Windows 10.0.29708 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46. **PASS scoped 1/1:** actual GUI preview of one synthetic archive record with final path `../escape` reported zero acceptable records and an archive-root escape rejection; the acceptance button was disabled. No import was attempted. Evidence: `target/w7-traversal-65819d3.{ps1,e2e.mjs}` and `target/w7-traversal-65819d3/{runner.log,runner.exit,app.sha256}`. The test checks disabled acceptance and a reported preview outcome; the logged rejection reason was inspected separately. This single parent-path spelling does not establish all traversal, reparse, archive or GUI cases, and the executable predates later W1-only changes.

## Windows keeper admission interleaving — 2026-11-19

At source `c029f36` plus the change committed as `abcbcfe`, the keeper-start admission boundary was extracted into a private helper used by the production call, permitting a controlled concurrent test. On Windows 10.0.29708, a registered synthetic job paused inside the start callback while another thread attempted to close admission; closure did not complete during the 250 ms observation and did complete after release. Starting after closure was rejected before the callback. This is **PASS scoped** for that interleaving; the callback does not create a process and scheduling makes the timed observation bounded rather than an exhaustive proof. The native containment module passed **8/8**, including existing actual-keeper and successor cases, with the `67d1a3f` Desktop keeper SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf` and test binary SHA-256 `7bf6e5e4b99690252fcf1059a242be47998dc74f648f03b64307d6d40b7615cd`. Evidence: `target/w1-keeper-interleaving-c029f36.{log,exit}`. Two initial test compilations failed due to thread ownership and drop-order errors; both were corrected before the passing run. Formatting and architecture checks passed; the updated architecture baseline needs `CROSS_PLATFORM_REVIEW_REQUIRED`. Historical descendant exit proof and W2 alias handling remain open, so the production factory and publication remain gated.

## Windows keeper admission synchronization — 2026-11-19

At source `4f0ce4f` with the Windows containment change subsequently committed as `03a112d`, code review found that `start_desktop_keeper` released the shared writer admission mutex after checking `closed` and before creating the keeper; `close_admission` could interleave. The change holds that mutex through keeper creation, matching the synchronization already used for writer creation. The race was inferred from the code, not reproduced in a forced interleaving. On Windows 10.0.29708, `cargo test -p xarchive-workflow --lib windows_containment::tests:: -- --test-threads=1` passed **7/7**, including closed admission, controlled writer, keeper retention and owner-crash successor checks. The Desktop keeper was the `67d1a3f` build, SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf`; the test executable SHA-256 was `c65de6c99f4e4c1352dec7cb952098ec11388a7d0a3e6f3c98b69f1c45f5976d`. Evidence: `target/w1-keeper-admission-4f0ce4f.{log,exit}`. `cargo fmt --all -- --check` and architecture check passed; the architecture baseline was resealed only for the changed Windows file, requiring `CROSS_PLATFORM_REVIEW_REQUIRED`. These tests do not force the concurrent interleaving and do not prove all historical descendant handles have signaled. W1 production publication remains gated; W2 and the integrated lifecycle checks remain open.

## Windows record-import viewport — 2026-11-19

At clean tracked test source `5814475`, the previously built Desktop (source `67d1a3f`, SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf`) ran in the isolated Full fixture on Windows 10.0.29708 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46. **PASS scoped 1/1:** the record-import page was opened at reported 720×540 and 1080×760 viewports, each with device pixel ratio 2 and no document horizontal overflow. Captured visible text and controls appeared legible without overlap; the narrow page required vertical scrolling. Evidence: `target/w7-layout-record-retry-5814475/{runner.log,runner.exit,app.sha256}`, `target/w7-layout-record-5814475.{ps1,e2e.mjs}`, `target/w7-layout-record-retry-5814475.ps1`, and the two `target/w7-layout-record-5814475-{720,1080}.png` screenshots. The first script stopped before driver launch because it checked an unrelated existing fixture directory; the retry corrected that precondition. These checks do not cover off-screen content, interactions, other page states, system DPI changes or the full matrix.

## Offline Bundle prerequisite check — 2026-11-19

At clean tracked source `029c7107da692fc2f05a50e6448319088c1bcf77`, `node --test desktop/test/offline-bundle-package.test.mjs` passed 3/3 on Windows. This checks the manifest contract with synthetic component metadata only. `desktop/src-tauri/src/components.rs` still returns an empty embedded release catalog; ADR-013 explicitly leaves Bootstrap/release integration planned. The current Windows release workflow and portable builder do not assemble an Offline Bundle. Therefore actual six-component assembly, embedded/external catalog parity, startup/activation, signing and release acceptance remain `NOT_RUN`; Core/Full parity and the synthetic test are not substitutes. `CROSS_PLATFORM_CHANGE_REQUIRED`: Cross-platform Owner completes the already approved catalog and Bootstrap/release integration and supplies fixed versioned release inputs; Windows Platform Owner builds and validates the resulting exact artifact. Existing user registration and the two untracked user directories were preserved.

## Windows local release archive steps - 2026-11-19

At clean tracked source `8eb55430057cb090d1ecfff6b2fb903df5bfb662`, the actual PowerShell bodies of the Windows repository-dependencies and Full archive workflow steps were extracted and run in a new isolated workspace. The only workflow expression substituted was the local Desktop artifact path; the local `7z.exe` name referred to the previously verified standalone 7zr 26.06. Inputs reused the `67d1a3f` Desktop, `7b7bd7e` worker, Native Host and pinned external binaries; no real downloader was run. Both 7z archives were created and independently extracted: **PASS scoped**. Each extracted Extension contained the same 13 runtime files as source, each worker contained 15 byte-identical files, and package type and Extension origin matched. Full's Desktop and gallery binary hashes matched, all 21 gallery files and 8 aria2 files matched the supplied inputs. Repository archive SHA-256 `1cf7f031ebbd5680bd585c382cda60006b04d4dd8b8ccfa3bd0d479627f6a1ae`; Full archive SHA-256 `1ffe01b3d8d07e671ba41e6de6063f99d475fb1f7fcbf954617e4e8ad2225090`.

Both extracted Native Host manifests retained their original build-staging executable paths; registration/repair after extraction or relocation remains required and was not performed. The first verification script failed before product assertions because its `$host` variable conflicted with PowerShell's read-only `Host`; the corrected script used new extraction directories and passed. Evidence: `target/w6-release-archive-sim-8eb5543/` (`extract-workflow.cjs`, `repository.ps1`, `full.ps1`, `run.ps1`, per-step logs/exit codes, `inputs.sha256`, `verification-first.{log,exit}`, `verify-v2.ps1`, `verification-v2.{log,exit}`, `external-verification.{log,exit}` and extracted fixtures). This is a local workflow-step simulation with 7zr, not hosted Actions, full licence approval, installation/repair or complete W6/W7 acceptance. Shared packaging changes remain `CROSS_PLATFORM_REVIEW_REQUIRED`.

## Windows real Extension 7z verification - 2026-11-19

At clean tracked source `0f65c44ac3ffe365a632c0154d286b7eea7dbbab`, the actual Extension CLI planned and staged the 13 runtime files. The official [7-Zip 26.06 release](https://github.com/ip7z/7zip/releases/tag/26.06) `7zr.exe` created and extracted a real 7z archive; its 600,064 bytes matched the release metadata SHA-256 `e80db2dad715a6c5adabe292d31ed54e11f4e9b99ca349827b50573b175891bc`. Local Authenticode inspection reported `NotSigned`; this tool was used only in the disposable validation directory. The archive SHA-256 was `a1ef5667a18903c0d6d00719fc3c7f3293dd889f34d08773273713ef6c0fba4a`.

**PASS scoped:** the extracted 13-file Extension passed the actual inventory and source-byte verifier. After synthetic modification of `popup.js`, the verifier refused the mismatch; after restoring it and adding a synthetic `tests` directory, it refused the excluded entry. Evidence: `target/w6-7zr-{metadata,download}-0f65c44.{py,log,exit}`, `target/w6-real-7z-0f65c44.{ps1,log,exit}`, and `target/w6-real-7z-0f65c44/` (plan, stage, archive, extract, verify and negative-case logs/exit codes, `hashes.txt`, `tool-signature.txt`, `assertions.exit`). This uses the reduced standalone 7zr locally, not the hosted workflow's installed 7z; Full/repository-dependencies archives, hosted release, signing and complete W6/W7 acceptance remain open. Shared packaging changes remain `CROSS_PLATFORM_REVIEW_REQUIRED`.

## Windows Extension extracted-byte verification - 2026-11-19

At source `56656d014192fe384d51c3937f747ed6ea4ea1f0` plus changes committed unchanged as `117487f157e8e01839a4e429dbb11cc7146e23f5`, Extension verification accepted a same-name file with altered contents because it compared inventory names only. The new regression **FAILed before the fix** (`target/w6-extension-bytes-before-56656d0.{log,exit}`). The verifier now optionally compares each planned file with a supplied source directory; the Windows Extension archive workflow supplies that source after extraction and fails on mismatched bytes. Targeted release-assets, Extension, portable and release-security tests **PASS 53/53** (`target/w6-extension-bytes-after-56656d0.{log,exit}`); architecture and diff checks passed.

At clean source `117487f`, the actual CLI planned and staged the 13-file Extension, accepted matching contents and rejected a synthetic modification to `popup.js` (`target/w6-extension-bytes-117487f/{plan.log,plan.exit,stage.log,stage.exit,clean.log,clean.exit,rejected.log,rejected.exit,assertions.exit}`). This is a directory simulation; actual 7z extraction and hosted workflow remain unverified. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared verifier and workflow. W1/W2 publication gates and remaining W6/W7 acceptance remain open.

## Windows nested Extension exclusion - 2026-11-19

At source `17b0474` plus changes committed unchanged as `41872080705e54466e5202ef30f32ac199a966ca`, the Extension release filter accepted synthetic `tests`, `node_modules`, `secrets` and `cache` directories nested under `src`; the regression **FAILed before the fix**, with 17 files selected instead of 13 (`target/w6-extension-nested-before-17b0474.{log,exit}`). The existing excluded-directory rules now apply to every path component, so source planning skips those nested directories and artifact verification rejects them. Targeted release-assets, Extension, portable and release-security tests **PASS 52/52** (`target/w6-extension-nested-after-17b0474.{log,exit}`); architecture and diff checks passed.

At clean source `4187208`, an isolated Full fixture assembled with the current packaging scripts, reused `67d1a3f` Desktop and `7b7bd7e` worker, and its packaged Extension passed the actual release-inventory verifier for 13 files (`target/w6-extension-nested-full-4187208.{ps1,log,exit}` and `target/w6-extension-nested-full-4187208/{plan.log,plan.exit,verify.log,verify.exit}`). This is local assembly, not hosted workflow or 7z acceptance. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared release inventory rules; W1/W2 gates and remaining W6/W7 acceptance remain open.

## Windows Extension artifact verification - 2026-11-19

At source `3c985c8` plus changes committed unchanged as `2955227fefb3b776ca89182a0cb00ef3f5bdc399`, the Extension release verifier was found to filter excluded paths before comparing inventory, so a staged artifact containing a forbidden test file was incorrectly accepted. The new regression **FAILed before the fix** (`target/w6-extension-verify-before-3c985c8.{log,exit}`). Artifact verification now rejects excluded entries, including excluded directories; source planning still skips them. The regression checks a clean staged inventory, forbidden test and synthetic key files, and the actual verifier CLI. Targeted Extension, portable and release-security tests **PASS 48/48** (`target/w6-extension-verify-after-3c985c8.{log,exit}`); architecture and diff checks passed.

At clean source `2955227`, the real CLI planned and staged the current 13-file Extension, verified the clean directory, then refused the same directory after a synthetic test file was added (`target/w6-extracted-verify-2955227/{plan.log,plan.exit,stage.log,stage.exit,clean.log,clean.exit,rejected.log,rejected.exit,assertions.exit}`). This is a directory-level simulation; no 7z extraction or hosted workflow was run. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared verifier. W1/W2 gates, final release and remaining W6/W7 acceptance remain open.

## Windows release Extension staging - 2026-11-19

At source `448a9a8` plus changes committed unchanged as `8b7b6ae6673a8166b98897394b1c2d6f6380f34a`, the Windows release workflow's repository-dependencies and Full archive steps were found copying the entire Extension source tree, including tests and package metadata. Both steps now invoke the validated Extension inventory staging command and fail if staging fails. The command validates identity, release tag and required files before creating a new output directory; it refuses an existing destination or one inside the source. The targeted packaging/Extension/release-security suite **PASS 47/47** (`target/w6-release-inventory-final-448a9a8.{log,exit}`), including the actual staging CLI, existing-output preservation, missing required input and both workflow entry points. The first test invocation correctly failed before the stage command existed (`target/w6-release-inventory-before-448a9a8.{log,exit}`).

Local simulations of both workflow staging destinations each contained 13 runtime Extension files byte-identical to the tracked source, with no test files or package metadata (`target/w6-release-stage-448a9a8.{log,exit}` and its isolated directories). Architecture check and `git diff --check` passed. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared Extension packaging command and workflow change. This is local staging and workflow-shape evidence; hosted Actions, final archive extraction, signing, complete licence review and release acceptance remain open.

## Windows Full Extension inventory fix - 2026-11-19

At source `f2fe060` plus the changes committed unchanged as `d20eb3334d6aad208801b411928cf3b0b8cdc652`, a portable Full assembly reproduced a packaging defect: the prior Full fixture contained `extension/tests/` and `extension/package.json`, while the standalone Extension release inventory excludes them. The new end-to-end portable packaging assertion failed before the fix (`target/w6-extension-inventory-before-f2fe060.{log,exit}`). The Windows portable builder now computes and validates the existing standalone Extension inventory before replacing output, intersects it with the portable safety filter, and copies that inventory. Missing required Extension input is tested to preserve an existing output. Targeted packaging, portable and Extension inventory tests **PASS 33/33**; architecture check reported zero findings (`target/w6-extension-inventory-final-f2fe060.{log,exit}`).

An isolated Full fixture assembled with the final changes contained exactly 13 tracked Extension runtime files matching source bytes, with no Extension tests or package metadata; all 15 installation-manifest file references existed. It reused Desktop from `67d1a3f` (SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf`), Native Host SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`, and worker from `7b7bd7e` (SHA-256 `2a25f794749e221b914e68635e15da9a753f3a86e0900dd404ac8e8a8b6ca097`). At clean test source `d20eb33`, isolated Chromium status-triggered recovery and a synthetic status query **PASS scoped**; the temporary Chromium registration was removed and existing Chrome/Edge registrations were unchanged. Evidence: `target/w6-extension-clean-f2fe060.{ps1,log,exit}`, `target/w6-extension-clean-f2fe060-verify.log`, `target/w6-extension-clean-manifest-d20eb33.log`, and `target/w6-extension-clean-recovery-d20eb33/{runner.log,runner.exit,artifacts.sha256}`. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared packaging changes. Hosted release, complete dependency/licence review, real browser/account integration and full W6/W7 remain open.

## Windows current Core and Full boundary - 2026-11-19

At clean tracked source `6e5d58ae172b9ae7f97c26f1686a425dd131c739`, an isolated Core fixture was assembled with the `67d1a3f` Desktop and `7b7bd7e` worker. Comparison against the prior isolated Full fixture **PASS scoped**: the two manifests report their respective types, Full includes gallery-dl, Extension and Native Host, Core excludes them, Desktop bytes match, and all 15 worker files match. The optional aria2 input was reused. The Core Desktop (SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf`) then ran in a new isolated portable root on Windows 10.0.29708 with WebView2 153.0.4234.48 / EdgeDriver 153.0.4234.46; direct WebDriver dashboard startup and native preview input validation **PASS 4 / 0 / 1 skipped**. The record round-trip test was skipped by its environment condition. Evidence: `target/w6-core-current-6e5d58a.{ps1,log,exit}`, `target/w6-core-current-6e5d58a-verify.log`, `target/w6-core-startup-6e5d58a.ps1`, and `target/w6-core-startup-6e5d58a/{runner.log,runner.exit,app.sha256}`. This does not establish Offline Bundle parity, hosted release, real transfer or complete W6/W7 acceptance.

## Windows uppercase mapping volume comparison - 2026-11-19

At clean tracked source `51aeb244b270d4e127fee4e76a5503d529e53827`, a read/write diagnostic in newly created disposable directories compared the local `RtlUpcaseUnicodeChar` result against actual name lookup for 931 BMP alphabetic characters whose mapping changed the code unit. On the tested NTFS and ReFS volumes, each mapped spelling resolved to the created name: **931/931 per volume, scoped**, with no filesystem errors. Evidence: `target/w2-ordinal-fs-51aeb24.py`, `target/w2-ordinal-{ntfs,refs}-51aeb24.{log,exit}` and corresponding result JSON. This is a local lookup observation, not a production mapping implementation or a complete case-equivalence proof: it excludes unmapped characters, nonalphabetic characters, alternate volume tables, case-sensitive directories, future 8.3 aliases and cross-process locking. The Unicode false-contention failure and 8.3 race remain open; Windows publication remains fail-closed.

## Windows current-source NTFS record root replacement - 2026-11-19

At clean tracked source `75dbf1f193d10e7b70fed2ba97f42f3df9783431`, the `67d1a3f` Desktop (SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf`) and `7b7bd7e` worker used a separate Full fixture on Windows 10.0.29708, WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46. WebDriver-driven actual Desktop IPC **PASS scoped 1/1**: after a valid synthetic record preview, replacing its archive root caused acceptance to report a stale preview; a new preview accepted the replacement root. The disposable archive root was `C:\Users\Shiraishi\AppData\Local\Temp\tw2tg-w7-root-current-75dbf1f` on NTFS. Evidence: `target/w7-root-current-75dbf1f.ps1` and `target/w7-root-current-75dbf1f/{runner.log,runner.exit,app.sha256}`. This does not exercise production writer containment, destination publication or full W7 GUI acceptance.

## Windows Unicode alias guard - 2026-11-19

At source `8215937` plus the new test committed unchanged as `010ed4e0034ca429ac66776f97344072e9ebc919`, `absent_unicode_case_aliases_contend` **PASS 1/1 on NTFS and 1/1 on ReFS**. Each run first created a directory to verify that `archive-U+1F80` and `archive-U+1F88` resolve to the same name, removed it, then acquired the first destination guard while both names were absent and verified that the second contended. Test executable SHA-256 `b7f104499adbd6dc10fe09b4a0e12b934c7630c40a01e2799130f23858409ef7`; evidence `target/w2-unicode-alias-{ntfs,refs}-8215937.{log,exit}` and `target/w2-unicode-alias-8215937.sha256`. Formatting and architecture checks passed.

A separate bounded diagnostic selected 27 BMP character pairs using the local Windows uppercase routine, then checked filesystem lookup: all 27 pairs resolved as equivalent on each tested NTFS and ReFS volume (`target/w2-unicode-simple-8215937.py`, `target/w2-unicode-simple-{ntfs,refs}-8215937.{log,exit}` and result JSON). For the tested U+1F80/U+1F88 pair, Rust's full uppercase expands each to the same two characters (`target/w2-unicode-rust-8215937.{rs,log,exit}`). Consequently, a proposed shortcut that leaves every multi-character uppercase expansion unchanged would split the lock identity for this actual alias pair; the regression protects that invariant. This does not establish a complete filesystem case map, repair the `straße`/`STRASSE` collision, resolve 8.3 alias creation or authorize publication. Microsoft's [NTFS reserved-file documentation](https://learn.microsoft.com/openspecs/windows_protocols/ms-fscc/b04c3bd0-79dc-4e58-b8ed-74f19fc2ea0a) identifies a filesystem uppercase collation table; per-volume behavior still needs to be accounted for.

## Windows Unicode destination lock collision - 2026-11-19

At source `a1fc616` plus the new test committed unchanged except for its explicit ignore annotation as `7c64afb8f3c843551774f600b70545e3d10bbc76`, native Windows tests created separate existing directories named `straße` and `STRASSE`. On both the E: ReFS fixture and an independently checked NTFS temporary volume, both directories were successfully created, but the second destination guard returned `Contended` while the first was held: **FAIL**, because these distinct destinations must not serialize. `destination_lock_identity` applies Rust full-string uppercase; in this case it maps both names to `STRASSE`. The ignored regression was also explicitly run on NTFS and failed the same assertion. Test executable SHA-256 `0d4d2eb1d4010f70d0af8e077d5ac390a56d69f92ed8e12bca64f301aa491a6b`; evidence `target/w2-unicode-{before,ntfs-before,ntfs-ignored,module}-a1fc616.{log,exit}` and `target/w2-unicode-test-a1fc616.sha256`. Normal Windows native module **PASS 17 / 0 / 4 ignored**, with this known failure among the ignored tests; `cargo fmt --all -- --check` and architecture check passed.

This is independent of the previously recorded future 8.3 alias race. Both remain open, and publication stays fail-closed. Do not replace full Unicode uppercase with an unverified approximation: the replacement must preserve contention for all filesystem-equivalent spellings as well as independence for distinct destinations, including names absent at acquisition. Windows Platform Owner must implement and validate the filesystem-compatible identity; run the ignored regression on NTFS and ReFS along with existing case/8.3 and concurrent-publication checks before enabling the path.

## Windows current-source invalid record recovery - 2026-11-19

At clean tracked source `6608e1fed9c727b6d1abf8eba821237745236641`, the `67d1a3f` Desktop (SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf`) and `7b7bd7e` worker ran in a new isolated Full fixture on Windows 10.0.29708 with WebView2 153.0.4234.48 / EdgeDriver 153.0.4234.46. Actual record-import GUI **PASS scoped 1/1**: malformed input cleared the previous preview and disabled acceptance, preview reported failure, and after waiting for the preview button to re-enable, restoring valid input produced a selectable preview. No acceptance was requested. Evidence: `target/w7-invalid-current-6608e1f.{ps1,e2e.mjs}` and `target/w7-invalid-current-6608e1f/{runner.log,runner.exit,app.sha256}`. This does not explain the historical intermittent timeout or establish the full F3 failure matrix.

## Windows current-source configuration maintenance - 2026-11-19

At clean tracked source `bb44b79ce272892c329b3649c4d28140fc601aad`, three separate Full fixtures used the `67d1a3f` Desktop (SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf`) and the `7b7bd7e` worker on Windows 10.0.29708 with WebView2 153.0.4234.48 / EdgeDriver 153.0.4234.46. Actual Desktop IPC **PASS 3/3 scoped**: cancellation and timeout against a synthetic active workspace holder were correctly distinguished, preserved configuration/runtime state and reopened admission; an injected lock-path I/O failure was distinguished from cancellation/timeout, preserved state and allowed a later apply. The holder was test executable SHA-256 `67f1c8c29056d2bbcdeb9e45c6602b47634b92b20bbfff19d7d32f4a1b79f80d`. Evidence: `target/w3-current-{cancel,timeout,io}-bb44b79.ps1` and `target/w3-current-{cancel,timeout,io}-bb44b79/{runner.log,runner.exit,app.sha256}`. These use a synthetic holder rather than a production writer; W1/W2-gated lifecycle and cancellation while a contained writer drains remain open.

## Windows current-source worker and Full assembly - 2026-11-19

At clean tracked source `7b7bd7e53ae6285b1b1f61437a080c8daa1c0ae3`, Python 3.12.14 and PyInstaller 6.22.3 built the current worker in a new isolated directory with PATH restricted to the validation Python and Windows system directories. Worker SHA-256 `2a25f794749e221b914e68635e15da9a753f3a86e0900dd404ac8e8a8b6ca097`; the Analysis table resolves inspected project modules to this worktree. The exact bundled Python runtime was checked against its source before adding its licence, and the OpenSSL licence was checked and included. The inventory matched 11 binary entries to build-source bytes and separately inventoried `base_library.zip`. Help and a v2 download with the local synthetic gallery executable **PASS scoped**; synthetic gallery SHA-256 `37a68a72a659cec7a550fedb9df8291d9c33db230da75260caab4000316026c8`.

An isolated Full package combined this worker with the `67d1a3f` Desktop/Host and current extension; downloader inputs were reused. All 15 worker files matched their build outputs, and packaged worker help and synthetic v2 download **PASS scoped**. Evidence: `target/w6-current-worker-7b7bd7e/{build.log,build.exit,license.log,license.exit,license-inventory.json,license-inventory.log,license-inventory.exit,artifacts.sha256,probe.py,assertions.log,assertions.exit}` and `target/w6-fresh-worker-full-7b7bd7e/{probe.py,verify.log,assertions.log,assertions.exit}` plus `target/w6-fresh-worker-full-7b7bd7e.{ps1,log,exit}`. This does not establish complete dependency/licence obligations, hosted release behavior, a real downloader transfer or W1–W7 acceptance.

## Windows current-source record-import GUI - 2026-11-19

At clean tracked test source `009ee058b78c4e5265e7b36051a8c57e42f1891d`, the `67d1a3f` Desktop (SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf`) ran from a separate Full fixture and portable root on Windows 10.0.29708 with WebView2 153.0.4234.48 / EdgeDriver 153.0.4234.46. Actual record-import GUI **PASS 1/1 scoped**: previewed synthetic valid and missing records, disabled selection of the missing record, imported the selected valid record, and exported JSON/YAML via buttons to a driver-directed isolated download directory. Evidence: `target/w7-current-record-009ee05.ps1`, `target/w7-current-record-009ee05/{runner.log,runner.exit,app.sha256,archive-records.json,archive-records.yaml}`. The download destination was configured by WebDriver; a native save dialog and full W7 acceptance were not covered. Worker and downloaders were reused and not exercised.

## Windows current-source Desktop dashboard and records - 2026-11-19

At clean tracked test source `6078860bbf7e77cd633951248a291d21e55cc412`, the current-source `67d1a3f` Full fixture's Desktop (SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf`) ran in a new isolated portable root on Windows 10.0.29708, WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46. Direct WebDriver `dashboard.e2e.mjs` with `WDIO_F3_ARCHIVE=1` **PASS 5/5 scoped**: frontend startup contract and dashboard, native record preview input validation, and synthetic archive record preview/accept/YAML export/re-import, stale revision and modified-media checks. Evidence: `target/w7-current-dashboard-6078860.ps1`, `target/w7-current-dashboard-6078860/{runner.log,runner.exit,app.sha256,driver.log}`. This does not cover an active contained writer, real transfer, manual file picker or full GUI acceptance. The fixture reuses the worker and downloaders.

## Windows current-source release and Full recovery - 2026-11-19

At source `67d1a3f45200bb373e10e5af82d020c5ef527bed` on `codex/windows-validation-4ae46ed3`, with tracked tree clean and the two pre-existing untracked directories retained, `npm run build:tauri --workspace desktop` built an optimized Desktop with dev release channel, SHA-256 `8074207a04922a18445d35f77f6ede97fe3082459b3e859fea009596dc3162cf`; `cargo build --release -p xarchive-native-host` built the Host, SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`. An isolated Full fixture assembled those binaries, the current extension, and reused worker/downloaders. All 14 tracked runtime extension files and the Full manifest, Host path and allowed origin matched. In isolated Chromium, after Desktop absence, starting this Desktop and requesting status triggered authenticated recovery and a synthetic query round trip: **PASS scoped**. The first status after startup was unauthenticated; passive recovery was not established. The temporary Chromium host key was removed and existing Chrome/Edge registrations were unchanged.

Evidence: `target/w6-current-release-{67d1a3f,retry-67d1a3f}.{log,exit}`, `target/w6-npm-ci-{67d1a3f,retry-67d1a3f}.{log,exit}`, `target/w6-current-host-67d1a3f.{log,exit}`, `target/w6-current-release-artifacts-67d1a3f/`, `target/w6-current-full-67d1a3f.{ps1,log,exit}`, `target/w6-current-full-67d1a3f-verify.log`, and `target/w6-current-full-recovery-67d1a3f/{runner.log,runner.exit,artifacts.sha256}`. The first build failed before compiling because the worktree lacked Node dependencies; the first `npm ci` failed creating workspace symlinks in the restricted environment, and the elevated retry succeeded. Tauri rewrote line endings in `desktop/src-tauri/Cargo.toml` without a normalized Git diff; that generated-only change was restored. This is not a fresh worker, hosted workflow, real downloader or complete W6/W7 acceptance. W1/W2 publication gates remain closed.

## Windows Full package current-extension check - 2026-11-19

At clean tracked source `858b484`, the current packaging script assembled an isolated Full fixture with the current extension and reused components: Desktop built at `79447104ca8ac92e5a8424e41040534528db2322` (SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`), Native Host (SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`), worker and external binaries. **PASS scoped:** all 14 tracked runtime extension files matched the packaged bytes; Full manifest, Native Host path and extension origin matched. In isolated Chromium, the packaged extension and temporary Native Host displayed the Desktop-not-ready result in popup/settings; the temporary Chromium registration was removed and existing Chrome/Edge registrations were unchanged. Evidence: `target/w6-current-extension-858b484.{ps1,log,exit}`, `target/w6-current-extension-858b484-verify.log`, and `target/w6-chromium-packaged-ui-858b484/{runner.log,runner.exit,artifacts.sha256}`. This is a reused-component fixture and one unavailable-Desktop UI path, not a fresh release build or full W6/W7 acceptance. Existing gallery provenance review and other queued checks remain open.

At clean tracked source `ddca98c762eafd818b4f0e9a084cb37250edaf06`, the same packaged extension and reused Desktop/Native Host were exercised in a fresh isolated Chromium profile. **PASS scoped:** after an initial absent-Desktop period, starting Desktop and requesting extension status triggered authenticated recovery, followed by a successful synthetic status-query round trip. The first status observed after startup was unauthenticated; this evidence establishes status-triggered recovery, not passive recovery. The temporary Chromium registration was removed and Chrome/Edge registrations were unchanged. Evidence: `target/w6-chromium-packaged-recovery-ddca98c.{ps1,cjs}` and `target/w6-chromium-packaged-recovery-ddca98c/{runner.log,runner.exit,artifacts.sha256}`.

## Windows gallery build provenance follow-up - 2026-11-19

At clean tracked source `951c91f9d4401d9e502563cd8452d56ebfc937ef`, public GitHub metadata identified a successful scheduled [upstream executable workflow run](https://github.com/mikf/gallery-dl/actions/runs/35478973454), including Windows x64 job `105993148179`, from 00:30:55 to 00:32:41 UTC on 2026-09-20. The pinned release asset SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` was created at 00:32:34 UTC in that interval. Its [workflow at run head](https://github.com/mikf/gallery-dl/blob/19a64031b7e695dc76d7d689a1c9d7470fc0bfc7/.github/workflows/executables.yml) installs dependencies without version pins and does not reference the examined source snapshot's `requirements/windows`; its [pinned clone action](https://github.com/gdl-org/clone/blob/c54f55c32557e4755f08232bdb8a60f94a26fbdd/action.yml) clones the default Codeberg branch without an explicit ref. Workflow SHA-256 `dcd224f3e4d92e7a62676d4069718f81f37f3793080dfcc28e0bc5713ebf074f`; action SHA-256 `791a56b001908669c4c77669740c52fca26abcda54d19b0dea20ae7022f1c85c`.

The source requirements specify websockets 16.0, idna 3.19 and PyInstaller 6.22.0. Existing binary inspection found websockets 17.1 metadata and idna 3.20 constants; earlier selected loader comparisons matched a PyInstaller 6.22.3 candidate. The workflow and timing make dependency drift plausible, but do not prove this exact job produced these asset bytes or identify all actual inputs; the public job-log request returned HTTP 403. Reproducible read-only evidence and retained metadata: `target/w6-gallery-provenance-retry-951c91f.{py,log,exit}`, `target/w6-gallery-clone-951c91f.{log,exit}` and `target/w6-gallery-provenance-retry-951c91f/`. The first script attempt used an incorrectly transcribed source-archive hash and failed (`target/w6-gallery-provenance-951c91f.{log,exit}`); the retry used the independently verified existing archive hash. Full corresponding-source, dependency and redistribution review remains open under `CROSS_PLATFORM_REVIEW_REQUIRED`; no downloader was executed.

## Windows unregistered synthetic Native Host name - 2026-11-19

At clean tracked source `5b8bafdbd4d059b79cee4c4c9e80f2dc93de3069`, isolated Chromium 159.0.8154.6 loaded a disposable extension copy whose Native Host name was replaced with a unique synthetic name. The test confirmed that name absent from the checked current-user and local-machine Chromium/Chrome host keys; it did not modify registry entries. **PASS scoped:** the browser returned `NATIVE_HOST_DISCONNECTED`, and popup and settings displayed the Native Host unavailable message (`target/w6-chromium-roundtrip-missing-name-5b8bafd.cjs`, `target/w6-missing-name-5b8bafd.{log,exit}`). Fourteen tracked non-test extension files matched the copy, with only the host-name substitution in `background.js` after newline normalization (`target/w6-missing-name-compare-5b8bafd.{log,exit}`); modified background SHA-256 `68f95a02b663eb612cd79a42c466bc9074706b62bbdadce38722ec186d9452bb`. This exercises a simulated missing-name path; it does not establish behavior of the standard product name in an environment with all its actual registrations absent, or full browser acceptance.

## Windows pairing error display and extension architecture baseline - 2026-11-19

At clean source `bcac28d7a2ab96a710bd3fe4a328f2ec50be0259` plus the exact UI change committed as `66c4940`, popup and settings map known pairing errors to distinct user-facing messages without echoing unknown error input. With Desktop absent, isolated Chromium 159.0.8154.6 and the temporary fixture Native Host registration returned `DESKTOP_NOT_READY`; both actual extension pages displayed the Desktop-not-ready message: **PASS scoped** (`target/w6-chromium-roundtrip-error-ui-bcac28d/{runner.log,runner.exit,artifacts.sha256}`, harness `.ps1`/`.cjs` in `target/`). The inherited harness field named `host_absent_status` is misleading: a temporary Host registration was present. This is not a missing-Host test. Extension tests **55 PASS / 1 skipped**, syntax checks passed (`target/w6-pairing-error-bcac28d.{log,exit}`); the skipped legacy migration regression remains `CROSS_PLATFORM_CHANGE_REQUIRED`. Selected other error mappings have unit coverage only; their runtime paths remain unverified. Temporary Chromium registration was removed, Chrome/Edge registrations reported unchanged. `CROSS_PLATFORM_REVIEW_REQUIRED` for shared UI/error mapping.

Architecture check initially reported seven extension baseline deltas spanning this and prior shared changes. Following local diff review, baseline commit `f834d7f` updates only those seven entries, using committed Git blob hashes; `npm run arch:check` then **PASS** with zero findings (`target/w6-extension-arch-66c4940.{log,exit}`). The initial whole-baseline generator output included unrelated Windows line-ending hash changes; those were discarded before the selective update. Cross-platform Owner must review both the shared extension changes and baseline reseal. This does not close the skipped migration test or W1/W2 gates.

## Windows browser rejects unlisted Native Host origin - 2026-11-19

At clean tracked source `8b37b7678dea1d90b04cdf43ffb79cd986776be1`, an isolated Chromium 159.0.8154.6 profile loaded a disposable copy of the repository extension with its manifest key removed, producing synthetic origin `chrome-extension://mnkkbacbackdngigpbfnpfbnkldmjcdp/`. The temporary Chromium Native Host manifest allowed only the original extension ID. **PASS scoped:** the browser rejected the synthetic extension's direct `connectNative` request as forbidden (`target/w6-chromium-roundtrip-unauthorized-retry3-8b37b76/{runner.log,runner.exit,artifacts.sha256}`); synthetic manifest SHA-256 `d50360330ed2fe3290985f3355e8c17778735ee7dcc546966ffb917db54045b9`. The temporary Chromium registration was removed and existing Chrome/Edge registrations reported unchanged. This verifies one browser/manifest origin rejection, not missing-Host behavior, user profiles or the complete browser security matrix.

The initial harness attempt misparsed the extension URL; the next two selected a browser-internal worker and opened an error page instead of the target extension (`target/w6-chromium-roundtrip-unauthorized{,-retry,-retry2}-8b37b76/`). The final attempt selected the worker by its `/src/background.js` path. These are retained as tooling failures, not product failures. Harnesses are the corresponding `.ps1`/`.cjs` files in `target/`.

## Windows Chromium Native Host registration provenance - 2026-11-19

At clean tracked source `58e2811`, an elevated retry launched isolated Chromium 159.0.8154.6 with the Chromium-specific host key absent. Although the test harness labelled this `host_absent_status`, the extension returned `DESKTOP_NOT_READY`; a live process inspection confirmed it actually used the existing current-user Google Chrome registration, whose Native Host executable SHA-256 is `a8f39f2a742d32eac0a3da674a10970883a01203f982d12c100f5a18d882f8bb` (`target/w6-existing-host-58e2811.{log,exit}`; process inspection in validation conversation). **NOT_RUN:** missing-Host acceptance; absence of the Chromium key alone does not establish that the browser has no matching registration. Existing Chrome/Edge registrations were not changed.

Separately, reproducing the earlier temporary Chromium-registration setup while the Chrome registration existed and inspecting the live Native Host process showed the isolated fixture executable SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`, with authenticated synthetic query (`target/w6-chromium-roundtrip-host-origin-58e2811/{runner.log,runner.exit,artifacts.sha256}`; process inspection in validation conversation). This directly confirms that fixture's host identity for this reproduction and supports the earlier same-setup tests, but does not independently inspect every prior run. The temporary Chromium key was removed and Chrome/Edge registrations reported unchanged. The earlier restricted-sandbox launch failure remains an environment observation.

## Windows migration regression and unavailable sandbox browser - 2026-11-19

At source `75843da` plus the uncommitted regression in `extension/tests/browser-pairing.test.js`, the synthetic failed-migration assertion was run unskipped and **FAILed** because no retained synthetic legacy token remained (`target/w6-migration-regression-refined-75843da.{log,exit}`). It is marked `test.skip` with `CROSS_PLATFORM_CHANGE_REQUIRED` until the shared fix is available. The ordinary extension suite completed **54 PASS / 1 skipped** (`target/w6-migration-suite-75843da.{log,exit}`); the skipped case is not a pass. The test uses only in-memory synthetic storage.

An independent attempt to inspect the unregistered Native Host case without registry writes could not start isolated Chromium in the current sandbox: process exit `3221225506` (`0xc0000022`, access denied), before any extension observation (`target/w6-chromium-roundtrip-no-host-75843da.cjs`, `target/w6-no-host-75843da.{log,exit}`). **BLOCKED by environment** for this attempt, not a product failure. The Chromium-only test host key was absent before launch. The previous approved, registry-backed tests remain bound to their own conditions and artifacts.

## Windows automatic pairing migration failure - 2026-11-19

At clean tracked source `6c1b74ab9dc61cf62a46b91011e9d69c105aaac7`, isolated headless Chromium 159.0.8154.6 loaded the current extension with Desktop absent. The test created a synthetic explicit legacy setting using the actual extension management handler, verified its synthetic token existed in the isolated profile, then used the ordinary settings page to save enabled automatic mode. **FAIL observed:** storage changed to automatic mode and the synthetic legacy token was removed before automatic authentication; Desktop was not running. This conflicts with the [approved pairing plan](../development/browser-automatic-pairing-plan.md), which specifies clearing the legacy token after successful automatic pairing. Evidence: `target/w6-chromium-roundtrip-migration-failure-6c1b74a.{ps1,cjs}` and `target/w6-chromium-roundtrip-migration-failure-6c1b74a/{runner.log,runner.exit,artifacts.sha256}`. The log records booleans, not the synthetic token; no real credential or user browser profile was used. Temporary Chromium registration was removed and Chrome/Edge registrations reported unchanged. No Desktop executable was launched for this check.

The current `WebSocketBridge.saveSettings` persists normalized automatic settings before `TransportBridge.saveWebSocketSettings` attempts connection; the later successful-bootstrap path also writes automatic settings. Fixing failure, disabled mode, worker interruption and competing settings writes requires preserving the approved shared credential-migration contract, without silently reusing legacy authentication or exposing secrets. `CROSS_PLATFORM_CHANGE_REQUIRED`: Cross-platform Owner implements and reviews that shared lifecycle; Windows Platform Owner reruns isolated failure/success/upgrade/rollback and GUI validation against the returned revision. Prior automatic enable/disable PASS does not prove this legacy migration case.

## Windows automatic pairing settings UI - 2026-11-19

The approved [automatic pairing plan](../development/browser-automatic-pairing-plan.md) calls for removing ordinary token copy/paste and not requiring a stored port/token. At clean source `8a601428fc9e945d74ddaba101de67d0108244d2` the current settings page still presented both fields and incorrectly promised Native Messaging fallback; an actual restart had also shown a stale editable port. Shared UI fix `c700330` removes the ordinary manual fields, identifies automatic discovery, and removes the unsupported fallback promise. Saving the switch explicitly keeps automatic mode; the existing diagnostic compatibility implementation is not removed. `CROSS_PLATFORM_REVIEW_REQUIRED` for shared UI wording and migration behavior.

At source `8a60142` plus exactly that change, extension tests **PASS 54/54** and syntax checks passed (`target/w6-auto-settings-8a60142.{log,exit}`). Isolated headless Chromium 159.0.8154.6 with the actual repository extension, Native Host SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6` and Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322` (SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`) verified absent manual credential controls, saving disabled automatic mode with an empty stored token, and successful reauthentication after re-enabling: **PASS scoped** (`target/w6-chromium-roundtrip-auto-settings-8a60142/{runner.log,runner.exit,artifacts.sha256}`). A repeat also produced the visually inspected settings screenshot `target/w6-auto-settings-visual-8a60142.png`, SHA-256 `d5898d5692cd77bfa72e2b80857965d100f3fb58ce63b7564ca8dc4693a10e57` (`target/w6-chromium-roundtrip-auto-settings-visual-8a60142/{runner.log,runner.exit}`). Corresponding `.ps1`/`.cjs` harnesses are in `target/`; temporary Chromium registrations were removed and Chrome/Edge registrations reported unchanged. The screenshot and headless test do not establish full real-browser compatibility, upgrade/rollback or release acceptance.

## Windows bounded-retry toolbar revalidation - 2026-11-19

At clean tracked source `7c6511b3702a6a086f4926c63cdb5bd5e1a76f6f`, visible isolated Chromium 159.0.8154.6 loaded the current extension and connected through Native Host SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6` to Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322` (SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`). **PASS scoped:** the actual toolbar popup showed connected; after Desktop termination it showed disconnected. Desktop was restarted after 16 seconds; after readiness the still-open popup remained disconnected, and clicking its reconnect button restored authenticated/connected with the new runtime port. This observes bounded recovery and explicit manual reconnect, not unattended indefinite recovery. The previous `9126ec3` automatic recovery involved the superseded periodic polling and is historical only. Computer Use observations are in the validation conversation; harness and completion evidence are `target/w6-chromium-roundtrip-toolbar-bounded-7c6511b.{ps1,cjs}` and `target/w6-chromium-roundtrip-toolbar-bounded-7c6511b/{runner.log,runner.exit,artifacts.sha256}`. Temporary Chromium registration was removed, Chrome/Edge registrations reported unchanged and no Chromium/Chrome process appeared in the subsequent process check. Real browser profiles, full W7 and shared extension review remain open.

## Windows extension status observation and bounded retry - 2026-11-19

Review of the approved [automatic pairing plan](../development/browser-automatic-pairing-plan.md) found that the one-second page polling introduced by `99d4bb9` and `8daf887`, together with the exhausted-budget reset in `23bb414`, could repeatedly start new retry cycles while a page stayed open. Those changes are superseded by shared fix `d2e15f2`: pages receive extension-internal state-change notifications and perform passive status reads; opening a page or another explicit status request can still trigger one bounded recovery cycle. No periodic status polling or network heartbeat was added. `CROSS_PLATFORM_REVIEW_REQUIRED` for the private extension-message adjustment and shared state handling.

At clean baseline `9210ecec701bf6b6eea663c27fcf793ec88710e8` plus the exact changes committed as `d2e15f2`, extension tests **PASS 54/54** and syntax checks passed (`target/w6-event-status-9210ece.{log,exit}`). In isolated headless Chromium 159.0.8154.6, actual extension/Native Host/Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322` (Desktop SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`, Native Host SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`) showed event-driven popup disconnect and fast reconnect, and popup/settings recovery display **PASS scoped** (`target/w6-chromium-roundtrip-{events-restart,popup-event,options-event}-9210ece/{runner.log,runner.exit,artifacts.sha256}`). A separate retry-exhaustion scenario observed no recovery after passive reads and five seconds, then recovered after an explicit status read and completed a synthetic query (`target/w6-chromium-roundtrip-passive-bound-9210ece/{runner.log,runner.exit,artifacts.sha256}`). Corresponding `.ps1`/`.cjs` harnesses are in `target/`. Temporary Chromium registrations were removed, Chrome/Edge registrations reported unchanged. This tests selected timing and page states; actual toolbar and broader browser behavior must be revalidated on the changed extension. The earlier `9126ec3` open-page polling restart result remains historical, not evidence for `d2e15f2`.

## Windows toolbar popup across Desktop restart - 2026-11-19

At clean tracked source `9126ec30f744f2b06a9b7bfd9e3851f3190aaaad`, visible isolated Chromium 159.0.8154.6 loaded the repository extension and connected through Native Host SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6` to Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322` (SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`). **PASS scoped:** with the actual toolbar popup open, terminating Desktop made it display disconnected and waiting for the service; after 16 seconds Desktop restarted and the same open popup showed authenticated/connected without pressing reconnect. The popup and settings page were open and polled status, which can initiate retry; no unattended or suspended-worker claim follows. Computer Use observations are in the validation conversation; harness and completion evidence: `target/w6-chromium-roundtrip-toolbar-restart-9126ec3.{ps1,cjs}` and `target/w6-chromium-roundtrip-toolbar-restart-9126ec3/{runner.log,runner.exit,artifacts.sha256}`. Temporary Chromium registration was removed and Chrome/Edge registrations reported unchanged.

**Separate UI observation:** in the same session the settings page's editable port remained `57276` while its current-status line and popup showed the new live port `61162`. The existing `get_websocket_settings` handler returns runtime status, while the save path normalizes automatic settings; the exact intended semantics of the legacy-looking editable fields remain a Cross-platform Owner follow-up, rather than an inferred corruption or a change to the protocol. This reproduces the previously reported stale-form symptom with concurrent UI observations. `CROSS_PLATFORM_REVIEW_REQUIRED` remains for the shared extension changes.

## Windows Chromium toolbar popup - 2026-11-19

At clean tracked source `0726e6703b100897af06987146fc1620609deb87`, a visible isolated Chromium 159.0.8154.6 profile loaded the current repository extension and used Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322` (SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`) with Native Host SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`. **PASS scoped:** actual extension bootstrap, WebSocket authentication and a synthetic status query passed; Computer Use pinned XArchive in the isolated profile and opened its actual toolbar popup. The popup showed connected, WebSocket, Desktop authenticated and archive service available; after clicking reconnect it again showed connected. The selected page was the extension settings page, so its page-archive action was unavailable. UI observations are in the validation conversation; harness and completion evidence are `target/w6-chromium-roundtrip-toolbar-retry-0726e67.{ps1,cjs}` and `target/w6-chromium-roundtrip-toolbar-retry-0726e67/{runner.log,runner.exit,artifacts.sha256}`. Extension popup SHA-256 `8da2a11679c8c9f1ced8b9bf8d90d9b3e06004227fa5d7a94122424cbc5455f6`. Temporary Chromium registration was removed, Chrome/Edge registrations reported unchanged, and no Chromium/Chrome process remained in the subsequent process check. The first attempt reached authenticated query but its 180-second inspection deadline elapsed before the toolbar action; it is retained separately as tooling/incomplete evidence in `target/w6-chromium-roundtrip-toolbar-0726e67/`. A later CIM process inventory was unavailable in the sandbox; it is not a product failure. This does not prove product registration, an actual X page/archive, Desktop restart or the full W7 matrix. `CROSS_PLATFORM_REVIEW_REQUIRED` remains for the shared extension changes.

## Windows extension settings recovery display - 2026-11-19

At clean tracked source `f0f7d3f0a4d6c61b2566b97358d73cb236baab04`, isolated headless Chromium 159.0.8154.6 loaded the extension before Desktop, waited 16 seconds, then started Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322` (SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`) and opened its settings page. Background authentication recovered, but the page still showed `已断开`: **FAIL observed** (`target/w6-chromium-roundtrip-options-recovery-f0f7d3f/{runner.log,runner.exit}`). Shared fix `8daf887` refreshes the connection status while the page remains open, without refreshing editable settings. At source `f0f7d3f` plus that change, the corresponding scenario showed `已认证`: **PASS scoped** (`target/w6-chromium-roundtrip-options-fixed-f0f7d3f/{runner.log,runner.exit,artifacts.sha256}`). A separate run retained synthetic unsaved port and enabled-field edits through multiple status refreshes (`target/w6-chromium-roundtrip-options-edit-f0f7d3f/{runner.log,runner.exit}`). Options file SHA-256 `c3c4efb06f0d112b314aece641e60e842fad5b2d340ee8952c5c492cf7278a4b`; extension tests 53/53 and syntax checks passed (`target/w6-options-refresh-f0f7d3f.{log,exit}`). Corresponding harnesses are `.ps1`/`.cjs` files in `target/`; temporary Chromium registrations were removed, and existing Chrome/Edge registrations were reported unchanged. The status polling can initiate bounded retry. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared UI and retry policy; real browser acceptance remains open.

## Windows extension popup recovery display - 2026-11-19

At clean tracked source `160d6f015ce2ad0830460285ee50d37c02e637fd`, the isolated Chromium 159.0.8154.6 scenario started the extension with Desktop absent for 16 seconds, then started Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`, and opened the extension popup as a page. Five seconds later the background reported authenticated but the open page still showed `已断开`: **FAIL observed** (`target/w6-chromium-roundtrip-popup-recovery-160d6f0/{runner.log,runner.exit}`). Shared UI fix `99d4bb9` refreshes status once per second while the popup page is open. On source `160d6f0` plus that exact change, the same actual Chromium/Native Host/Desktop scenario reported authenticated and `已连接`: **PASS scoped** (`target/w6-chromium-roundtrip-popup-fixed-160d6f0/{runner.log,runner.exit,artifacts.sha256}`); popup file SHA-256 `8da2a11679c8c9f1ced8b9bf8d90d9b3e06004227fa5d7a94122424cbc5455f6`. Extension tests passed 53/53 (`target/w6-popup-refresh-160d6f0.{log,exit}`) and syntax checks passed. Harnesses are corresponding `.ps1`/`.cjs` files in `target/`; temporary Chromium registration was removed, and existing Chrome/Edge registrations were reported unchanged. The popup page was tested in headless Chromium, not as a toolbar popup. Status polling may trigger the bounded recovery sequence. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared popup refresh and retry behavior; real-browser acceptance remains open.

## Windows extension retry exhaustion and status recovery - 2026-11-19

At clean tracked baseline `c4868c8110e823399a2f38bef92606267918f18c`, an isolated Chromium 159.0.8154.6 profile loaded the packaged extension while Desktop was absent for 16 seconds. After starting Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322` (SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`) and checking readiness through Native Host SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`, status polling failed to restore authentication within 10 seconds: **FAIL observed** (`target/w6-chromium-roundtrip-retry-baseline-poll-c4868c8/{runner.log,runner.exit}`; first non-polling attempt `target/w6-chromium-roundtrip-retry-exhaust-c4868c8/`). A targeted unit regression failed at the exhausted retry budget (`target/w6-extension-retry-before-c4868c8.{log,exit}`).

Change `23bb414` permits a later status read to restart the existing bounded retry sequence after exhaustion, without changing authentication-failure, protocol-error or timeout gates. At baseline plus that exact change, the extension tests passed 53/53 (`target/w6-extension-retry-after-c4868c8.{log,exit}`), syntax checks passed, and the same isolated Chromium sequence using the changed repository extension recovered authentication during polling and completed a synthetic query: **PASS scoped** (`target/w6-chromium-roundtrip-retry-fixed-c4868c8/{runner.log,runner.exit,artifacts.sha256}`). Harnesses are the corresponding `.ps1` and `.cjs` files in `target/`. The changed `websocket-bridge.js` SHA-256 was `262def631206a554f18cb236976ddf7228111759606e348cd6581e592ba0ba3c`. Temporary Chromium registrations were removed; existing Chrome/Edge registrations were reported unchanged. This does not establish unaided indefinite retry or resolve the historical real-browser restart report. `CROSS_PLATFORM_REVIEW_REQUIRED`: review the shared extension repair and retry policy; Windows retains actual browser and lifecycle acceptance.

## Windows isolated Chromium before Desktop startup - 2026-11-19

At clean tracked test source `637bb6372cebe851225ca75da9d04d6086dfdbdc`, Windows 10.0.29708 and isolated headless Chromium 159.0.8154.6 loaded the packaged extension before launching Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322` (SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`); Native Host SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`. **PASS scoped 1/1:** the extension reported unauthenticated with Desktop absent; after Desktop launch and a direct Native Host readiness check, no extension status requests were sent for 45 seconds. Its first subsequent status response was authenticated and a synthetic query succeeded. Status reads can initiate repair, and the options page and debugging connection stayed open; no exact connection timing or suspended-worker behavior is established. Evidence: `target/w6-chromium-roundtrip-start-late-637bb63.{ps1,cjs}` and `target/w6-chromium-roundtrip-start-late-637bb63/{runner.log,runner.exit,artifacts.sha256}`. The script reported removal of its temporary Chromium registration and unchanged Chrome/Edge registrations. Real-browser startup and restart acceptance remain open.

## Windows isolated Chromium restart without status polling - 2026-11-19

At clean tracked test source `0481b40ffca3daffe547c0fb51bd85715124c592`, Windows 10.0.29708 and isolated headless Chromium 159.0.8154.6 exercised the packaged extension, Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322` (SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`) and Native Host SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`. **PASS scoped 1/1:** after observing the extension unauthenticated following Desktop termination, the test restarted Desktop, checked readiness through a direct Native Host invocation, and sent no extension status requests for 45 seconds. The first subsequent status response was authenticated and a synthetic query succeeded. Status reads can initiate connection repair; the options page and browser debugging connection remained open, so this does not prove continuous connectivity, exact reconnect timing, service-worker suspension behavior or the earlier real-browser result. Evidence: `target/w6-chromium-roundtrip-idle-0481b40.{ps1,cjs}` and `target/w6-chromium-roundtrip-idle-0481b40/{runner.log,runner.exit,artifacts.sha256}`. The script reported removal of its temporary Chromium registration and unchanged Chrome/Edge registrations.

## Windows isolated Chromium Desktop restart - 2026-11-19

At clean tracked test source `ad75cba00e195c7b9601bf977f44314a99905f41`, Windows 10.0.29708 and isolated headless Chromium 159.0.8154.6 exercised the packaged extension with Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, Desktop SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`, and Native Host SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`. **PASS scoped 1/1:** after a synthetic query, the test terminated Desktop, observed the extension become unauthenticated, restarted Desktop using the same isolated root and Named Pipe, and observed authenticated reconnection 648 ms after a direct Native Host readiness check. A second synthetic query returned the matching response. Status was polled throughout; `TransportBridge.getStatus()` calls `ensureConnected()`, so those reads can initiate repair and this run cannot establish unaided automatic reconnect. Evidence: `target/w6-chromium-roundtrip-restart-ad75cba.{ps1,cjs}` and `target/w6-chromium-roundtrip-restart-ad75cba/{runner.log,runner.exit,artifacts.sha256}`. The script reported removal of its temporary Chromium registration and unchanged Chrome/Edge registrations. This single controlled run does not supersede the earlier real-browser manual reconnect failure or close idle/restart acceptance.

## Windows isolated Chromium extension connection display - 2026-11-19

At clean tracked test source `5e695c530db86514ed26ea0926d6080163608c7d`, Windows 10.0.29708 and isolated Chromium 159.0.8154.6 exercised the packaged extension and Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, Desktop SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`, Native Host SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`. **PASS scoped:** the extension bootstrapped via the temporarily registered Native Host, authenticated WebSocket, completed a synthetic status query, and its popup page displayed the connected state, WebSocket channel and Desktop archive service availability. The captured page was opened as an extension page in headless Chromium, not as a browser toolbar popup. Screenshot `target/w6-chromium-popup-5e695c5.png` SHA-256 `9280328e071fa03e3a5ee0c53c478087fe04efdf1f82601cd81fd950b8bec70f`; harness `target/w6-chromium-roundtrip-popup-5e695c5.{ps1,cjs}` and evidence `target/w6-chromium-roundtrip-popup-5e695c5/{runner.log,runner.exit,artifacts.sha256}`. The script reported removal of its temporary Chromium key and unchanged existing Chrome/Edge registrations. Product registration, toolbar interaction, real account and transfer acceptance remain open.

## Windows isolated Chromium native messaging round trip - 2026-11-19

At clean tracked test source `0ea24445e5a44449949ffffaf9b28a98bbe3670a`, an isolated Chromium 159.0.8154.6 profile loaded the packaged extension from Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322` and Desktop executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. The Native Host executable SHA-256 was `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`; all 17 tracked extension files matched the packaged fixture (`target/w6-chromium-extension-check-0ea2444.{py,log,exit}`). A temporary Chromium-only Native Messaging registration pointed to a test manifest for that host; a fresh portable root and private Named Pipe were used.

**PASS scoped 1/1:** after Chromium and the extension loaded, the actual extension obtained a native-host bootstrap, authenticated its WebSocket to Desktop, and received a protocol-versioned response to a synthetic status query with its request ID (`target/w6-chromium-roundtrip-retry4-0ea2444/{runner.log,runner.exit,artifacts.sha256,cleanup-check.log}`; harness `target/w6-chromium-roundtrip-retry4-0ea2444.{ps1,cjs}`). The temporary Chromium key was deleted; existing Chrome and Edge registrations were unchanged; a follow-up found no Chromium/Chrome process or temporary host key, and no Desktop/Native Host process. A first attempt failed before launch from an invalid test-script escape; two attempts failed while reading the extension API, and a further attempt observed a browser error page after opening the extension URL during startup (`target/w6-chromium-roundtrip-{0ea2444,retry-0ea2444,retry2-0ea2444,retry3-0ea2444}`). The successful retry opened the page after browser startup. This validates a manually registered isolated Chromium path, not the product's Chrome/Edge registration UI, an actual account, real downloads, or full W6.

## Windows PyInstaller source archive reference - 2026-11-19

At clean tracked source `39f80de7b7f29430464c54971d26c03a2101aaac`, the [PyInstaller 6.22.3 source archive](https://pypi.org/pypi/pyinstaller/6.22.3/json) was downloaded and verified against its published SHA-256 `05eb2f5615503e72939a7224d68b4aff572c6b0438ee4a17d0a4b481f399362d`. Its eight loader and runtime-hook source files matched the previously verified Windows wheel byte-for-byte (`target/w6-pyi-source-39f80de.{py,log,exit}`). This links those files to the earlier scoped embedded-code comparison; it does not prove the native bootloader can be reproduced from that source, establish all build inputs, or resolve redistribution obligations. `CROSS_PLATFORM_REVIEW_REQUIRED` remains open. The downloader was not executed.

## Windows task removal during navigation - 2026-11-19

At clean tracked test source `0106d2fdbbe588a8e3f21e42427316c352636ec1`, WebDriver exercised Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`, with synthetic completed jobs in a fresh isolated root. **PASS scoped 1/1:** after the dashboard displayed a task, deleting that synthetic record before opening its detail caused the actual GUI/IPC path to show the missing-record message and not render that task's detail. Evidence: `target/w7-jobs-delete-0106d2f/{runner.log,runner.exit,app.sha256}`, `target/w7-jobs-delete-0106d2f.{ps1,py,e2e.mjs}` and `target/w7-jobs-seed-51bd696.py`. This checks one isolated removal ordering, not production writer concurrency or full lifecycle acceptance. The shell's `InitializeDefaultDrives` warning did not prevent the completed test.

## Windows gallery Microsoft runtime inventory - 2026-11-19

At clean tracked source `3c1251758e2509b8ac984b50ac22869a6ca178b4`, static extraction from pinned gallery executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` identified 45 `api-ms-win-*`, UCRT and VCRUNTIME DLLs. Local Authenticode verification reported `Valid` for all 45; extracted inventory and signatures are in `target/w6-ms-runtime-3c12517/inventory.json` and `target/w6-ms-runtime-signatures-3c12517.{ps1,json,log,exit}`. The UCRT file reports version 10.0.26100.1742; the two VCRUNTIME files report 14.51.36247.0. Comparison with this machine's `System32` found 42 same-named files absent and three present but byte-different (`target/w6-ms-runtime-3c12517.{py,log,exit}`). This is a scoped inventory and local signature result, not proof of distribution source, trust on another machine, or [Microsoft redistribution conditions](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files). Full provenance and terms remain open under `CROSS_PLATFORM_REVIEW_REQUIRED`; no downloader was executed. The shell emitted an `InitializeDefaultDrives` warning while the checks completed.

## Windows gallery PyInstaller loader and hook comparison - 2026-11-19

At clean tracked source `0b35fb036be799b3ffb9ce1df32e8ac02684172a`, all eight embedded PyInstaller loader and runtime-hook modules identified in the pinned gallery executable matched selected code-object fields compiled with Python 3.14.7 directly from the hash-verified PyInstaller 6.22.3 Windows wheel. Evidence: `target/w6-pyi-hook-{index,compile,compare}-0b35fb0.{py,log,exit}`. Line and exception tables, other executable sections and runtime behavior were not compared. This refines the preceding four-module check; full build provenance and redistribution obligations remain open under `CROSS_PLATFORM_REVIEW_REQUIRED`. No downloader was executed.

## Windows pinned gallery PyInstaller correspondence and licence - 2026-11-19

At clean tracked source `339ea97ed51e4aa9a7856668a7ed89b91d5c514a`, four embedded PyInstaller loader modules matched selected code-object fields compiled with Python 3.14.7 from the locally installed PyInstaller 6.22.3 loader; those four local source files matched the hash-verified [PyInstaller 6.22.3 Windows wheel](https://pypi.org/pypi/pyinstaller/6.22.3/json) byte-for-byte. Wheel SHA-256 `500bd58c7bf7e584a8435adccbd763a0b918d5c12b08d74ff50fd79b2915458b`; evidence `target/w6-pyi-{loader-compile,loader-compare,upstream,wheel-compare}-339ea97.{py,log,exit}`. At source `105531f563c7bdc6b20f1c4f4c056d400c0e2645`, the pinned gallery executable's PE `.text` section matched the wheel's `run.exe` `.text` bytes; the three other wheel bootloaders did not (`target/w6-pyi-bootloader-105531f.{py,log,exit}`). This establishes scoped section and loader-field correspondence, not all PE sections, the complete build or runtime behavior.

Packaging change `105531f563c7bdc6b20f1c4f4c056d400c0e2645` includes the wheel's candidate licence text SHA-256 `0598064c7d2718e38d7914a7d08343b2fa008e3bea9ebba2aa7a6ffa5900dd64` for the pinned gallery executable. **PASS scoped:** 40/40 packaging/security tests (`target/w6-pyi-package-339ea97.{log,exit}`), pinned-asset CLI, isolated Full portable assembly (`target/w6-pyi-portable-339ea97.{log,exit}`), simulated `core.autocrlf=true` checkout and CLI, architecture and diff checks. Reused Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. PowerShell reported an `InitializeDefaultDrives` environment warning while commands completed successfully. Complete source and terms, other dependencies and hosted workflow remain open; `CROSS_PLATFORM_REVIEW_REQUIRED`. No downloader was executed.

## Windows pinned gallery notice table - 2026-11-18

At source `f6ae4a2c84d6324cfe3e8e1e8bd68345273c28f2` plus changes committed as `55a966c9444ba22fc49b4820d4fa8257253defd9`, the pinned gallery helper's 19 positional notice arguments were replaced with a single internal notice-to-hash table and source directory. All notices and the asset remain verified before destination creation; non-pinned portable behavior is unchanged. **PASS scoped:** 40/40 packaging/security tests including each mismatched notice (`target/w6-notice-table-f6ae4a2.{log,exit}`), actual pinned-asset CLI, isolated Full portable assembly (`target/w6-notice-table-portable-f6ae4a2.{log,exit}`), all 19 assembled notices revalidated (`target/w6-notice-table-verify-f6ae4a2.{mjs,log,exit}`), simulated `core.autocrlf=true` checkout and CLI, architecture and diff checks. The Full fixture reused Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Hosted workflow and full redistribution review remain open; `CROSS_PLATFORM_REVIEW_REQUIRED`. No downloader was executed.

## Windows pinned gallery OpenSSL notice - 2026-11-18

At clean tracked source `8dfd68098363e734b687e0f1d2a7aebae3b5e580`, static extraction from pinned gallery executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` identified `libcrypto-3.dll` and `libssl-3.dll`, both declaring version 3.5.7. Their SHA-256 values are `53c529145339fb042a3dcd3a09c2d7753204f8b4fc79d99e0d31e69a33985958` and `b17a87979862d19241edc4318f967e24c3ec356ed6c2368f561179fab2311001`; both previously matched the official Python 3.14.7 embedded archive byte-for-byte (`target/w6-gallery-python-native-393bd81.{py,log,exit}`). The [OpenSSL 3.5.7 licence](https://github.com/openssl/openssl/blob/openssl-3.5.7/LICENSE.txt) matched the already vendored worker notice SHA-256 `7d5450cb2d142651b8afa315b5f238efc805dad827d91ba367d8516bc9d49e7a` (`target/w6-gallery-openssl-license-8dfd680.{py,log,exit}`). These observations do not establish complete native build provenance or redistribution terms.

Packaging change `b0f5441407cffb285a1ac02f45820afd4f86ceef` includes the verified OpenSSL licence alongside the pinned gallery executable. **PASS scoped:** 40/40 packaging/security tests (`target/w6-gallery-openssl-package-8dfd680.{log,exit}`), pinned-asset CLI, isolated Full portable assembly (`target/w6-gallery-openssl-portable-8dfd680.{log,exit}`), simulated `core.autocrlf=true` checkout and CLI, architecture and diff checks. Reused Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Other components, complete terms and hosted workflow remain open; `CROSS_PLATFORM_REVIEW_REQUIRED`. No downloader was executed.

## Windows pinned gallery Brotli correspondence and licence - 2026-11-18

At clean tracked source `98288150aed5ec2e91818b105afb269e99c35b2c`, the pinned gallery executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` contained a Brotli native extension with SHA-256 `44f02bcf2b15519a487cf653e16b5f9c65f26d0be559592878988ad47148fc5e`. It matched the [PyPI Brotli 1.2.0](https://pypi.org/pypi/Brotli/1.2.0/json) CPython 3.14 Windows x64 wheel byte-for-byte, while the 1.2.1 wheel did not (`target/w6-brotli-{static,wheel-index,wheels}-9828815.{py,log,exit}`). Selected code-object fields of the embedded Python wrapper also matched the 1.2.0 wheel when compiled using Python 3.14.7 (`target/w6-brotli-{compile,compare}-9828815.{py,log,exit}`). Line and exception tables, full native source provenance and runtime behavior were not established.

Packaging change `cc4b4be46bbe466c2399ad881a6479feb7b7cdf3` includes the wheel licence SHA-256 `60ae0f13e76cc2eaaa108677eefa4ce16b647f6bbe8cf0a1ac9429d82eca7248` for the pinned asset. **PASS scoped:** 40/40 packaging/security tests (`target/w6-brotli-package-9828815.{log,exit}`), pinned-asset CLI, isolated Full portable assembly (`target/w6-brotli-portable-9828815.{log,exit}`), simulated `core.autocrlf=true` checkout and CLI, architecture and diff checks. Reused Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Other components, complete terms and hosted workflow remain open; `CROSS_PLATFORM_REVIEW_REQUIRED`. No downloader was executed.

## Windows pinned gallery four Python dependencies - 2026-11-18

At clean tracked source `6841fa69aef8ecfb00773a315c6a11fbb27fc28e`, the pinned gallery executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` declared PyYAML 6.0.3, charset-normalizer 3.5.1, mutagen 1.48.1 and PySocks 1.7.1. Their PyPI source archives were checked against published hashes (`target/w6-python-deps-source-6841fa6.{py,log,exit}`). Selected code-object fields matched all embedded modules: YAML 17/17, charset-normalizer 7/7, mutagen 48/48 and PySocks 1/1, compiled using Python 3.14.7 (`target/w6-python-deps-{compile,compare}-6841fa6.{py,log,exit}`). The PyYAML native extension and both charset-normalizer native extensions matched their respective PyPI CPython 3.14 Windows x64 wheels byte-for-byte (`target/w6-python-deps-wheel-retry-6841fa6.{py,log,exit}`). The initial wheel-selection attempt failed because it selected both regular and free-threaded ABIs; the retry selected the regular ABI. Line and exception tables, runtime behavior, and full native dependency provenance remain outside these checks.

Packaging change `a628ebf8af8f8144c98cd6a98b7b1fba787b27b4` includes the four upstream licence texts for the pinned asset. **PASS scoped:** 40/40 packaging/security tests (`target/w6-python-deps-package-6841fa6.{log,exit}`), pinned-asset CLI, isolated Full portable assembly (`target/w6-python-deps-portable-6841fa6.{log,exit}`), simulated `core.autocrlf=true` checkout and CLI, architecture and diff checks. The first checkout attempt failed because a PowerShell pipe added carriage returns to path names; the direct-argument retry passed. Reused Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Other components, complete terms and hosted workflow remain open; `CROSS_PLATFORM_REVIEW_REQUIRED`. No downloader was executed.

## Windows pinned gallery idna and truststore notices - 2026-11-18

At clean tracked source `859e28ad666bfdf34a2102c45274e8034da065b9`, static inspection of pinned gallery executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` declared idna 3.20 and truststore 0.10.4. Their [idna source](https://pypi.org/pypi/idna/3.20/json) and [truststore source](https://pypi.org/pypi/truststore/0.10.4/json) archives matched the published SHA-256 values `a7db850025b95ded1eae8a46181a1a6c56c92c96f0e2b005d9ff8dc0210cab44` and `9d91bd436463ad5e4ee4aba766628dd6cd7010cf3e2461756b3303710eebc301`. Each package's six embedded modules matched selected source-compiled code-object fields using Python 3.14.7 (`target/w6-idna-truststore-{source,compile,compare}-859e28a.{py,log,exit}`). Line and exception tables and runtime behavior were not compared.

Packaging change `ad59c6e55ec1953ff4583018eefd5178ef487905` includes the idna and truststore source licences, SHA-256 `1a9a4f0e3d479a27240ddd59a9137a66ab4a0f9dfdc8ca6188cc0bfd85187f04` and `33be7b7e8fa4fd19b1760e1a8ed8a668bdab852c91b692dd41424bcb725a9fca`. **PASS scoped:** 40/40 packaging/security tests (`target/w6-idna-truststore-package-859e28a.{log,exit}`), pinned-asset CLI, isolated Full portable assembly (`target/w6-idna-truststore-portable-859e28a.{log,exit}`), simulated `core.autocrlf=true` checkout and CLI, architecture and diff checks. Reused Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Other dependencies, complete terms and hosted workflow remain open; `CROSS_PLATFORM_REVIEW_REQUIRED`. No downloader was executed.

## Windows pinned gallery requests source and licence - 2026-11-18

At clean tracked source `984bebd0f2e2959306886620cc6da16aa3573598`, static inspection of pinned gallery executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` identified `requests` 2.34.2. Its [PyPI source archive](https://pypi.org/pypi/requests/2.34.2/json) matched the published SHA-256 `f288924cae4e29463698d6d60bc6a4da69c89185ad1e0bcc4104f584e960b9ed`; all 19 embedded modules matched selected code-object fields compiled from that source with Python 3.14.7 (`target/w6-requests-{source,compile,compare}-984bebd.{py,log,exit}`). Line and exception tables and runtime behavior were not compared.

Packaging change `09d9c0a4a8f53ef08fe35b582ce69cdff6ac3037` includes the upstream licence SHA-256 `09e8a9bcec8067104652c168685ab0931e7868f9c8284b66f5ae6edae5f1130b` for the pinned asset. **PASS scoped:** 40/40 packaging/security tests (`target/w6-requests-package-984bebd.{log,exit}`), actual pinned-asset CLI, isolated Full portable assembly (`target/w6-requests-portable-984bebd.{log,exit}`), simulated `core.autocrlf=true` checkout and CLI, architecture and diff checks. Reused Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Other bundled components, complete terms and hosted workflow remain open; `CROSS_PLATFORM_REVIEW_REQUIRED`. No downloader was executed.

The same static inventory identified other candidate top-level packages; declared versions include PyYAML 6.0.3, charset-normalizer 3.5.1, idna 3.20, truststore 0.10.4, PySocks 1.7.1 and mutagen 1.48.1 (`target/w6-gallery-remaining-984bebd.{py,log,exit}`, `target/w6-gallery-remaining-versions-retry-984bebd.{log,exit}`, `target/w6-gallery-more-versions-984bebd.{py,log,exit}`). These declarations alone do not prove corresponding source or terms. The first version-inventory attempt failed on terminal encoding and was rerun with UTF-8; its failed log is retained.

## Windows pinned gallery EJS dependency notices - 2026-11-18

At clean tracked source `fdf4c0b9c9676f4d5994b82cefe120368e750a65`, the hash-verified yt-dlp-ejs 0.8.0 source lockfile named astring 1.9.0 and meriyah 6.1.4. Their npm archives were downloaded and verified against the lockfile's SHA-512 integrity values (`target/w6-ejs-npm-{lock-,}fdf4c0b.{py,log,exit}`). The astring licence SHA-256 is `d7e882d386050a13b3244078d95b1e3d943b4741fb1b2d7d936cf39a08f52a09`; meriyah licence SHA-256 is `e9a5df5a6ae5cd51ca430715f4c62fb5424946e8da07b7caa058a1ba0582a848`. Each full licence text appears in the pinned gallery's embedded EJS library header after whitespace normalization (`target/w6-ejs-npm-compare-fdf4c0b.{py,log,exit}`). This is a descriptive text comparison, not a legal determination or a full source/build correspondence check.

Packaging change `4d8d6abde421b4feb33ddb4a9d08997b8cf9f364` adds and hash-checks both independent notices for the exact pinned gallery asset. **PASS scoped:** 40/40 packaging/security tests (`target/w6-ejs-npm-package-fdf4c0b.{log,exit}`), pinned-asset CLI, isolated Full portable assembly (`target/w6-ejs-npm-portable-fdf4c0b.{log,exit}`), simulated `core.autocrlf=true` checkout and CLI, architecture and staged diff checks. Reused Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Hosted workflow, other components and complete redistribution obligations remain open; `CROSS_PLATFORM_REVIEW_REQUIRED`. No downloader was executed.

## Windows pinned gallery yt-dlp-ejs correspondence and licence - 2026-11-18

At clean tracked source `1443b47a3e512a2bd7c6b0d1a3c0c7d400956c13`, static inspection of pinned gallery executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` found `yt-dlp-ejs` version 0.8.0. Its [PyPI source and wheel](https://pypi.org/pypi/yt-dlp-ejs/0.8.0/json) were downloaded and checked against published hashes: source `d5fa1639f63b5c4af8d932495f60689d5370f1a095782c944f7f62a303eb104e`, wheel `79300e5fca7f937a1eeede11f0456862c1b41107ce1d726871e0207424f4bdb4`. All four embedded Python modules matched selected code-object fields compiled from the wheel with Python 3.14.7, and both embedded solver JavaScript files matched the wheel byte-for-byte (`target/w6-ejs-{version,upstream,compile,compare}-1443b47.{py,log,exit}`). Line and exception tables and runtime behavior were not compared.

The wheel's project licence SHA-256 `b5065838cbac452dfc855ba6e6e031481ad2c68406f70d21ead9321374653e6c` is included for the pinned executable by packaging change `3d8610e34ed53db52f1a3e0604c2cdbd2fa78651`. **PASS scoped:** packaging/security tests 40/40 (`target/w6-ejs-package-1443b47.{log,exit}`), actual pinned-asset CLI, isolated Full portable assembly (`target/w6-ejs-portable-1443b47.{log,exit}`), simulated `core.autocrlf=true` checkout and CLI, architecture and diff checks. The executable's bundled `lib.min.js` header names meriyah 6.1.4 under ISC and astring 1.9.0 under MIT (`target/w6-ejs-header-1443b47.{py,log,exit}`); wheel metadata declares `Unlicense AND MIT AND ISC`. This inventory and the included project licence do not determine all applicable notices or redistribution obligations. Other dependencies and hosted workflow remain open; `CROSS_PLATFORM_REVIEW_REQUIRED`. No downloader was executed. Reused Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`.

## Windows task navigation after history insertion - 2026-11-18

At clean tracked test source `c3a08612779125d31225f87a91b2f5467b1b87a4`, WebDriver exercised Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`, with synthetic completed jobs in a fresh isolated root. **PASS scoped 1/1:** after the dashboard loaded its 20 most recent jobs, one newer synthetic job was inserted into the database; selecting the previously visible oldest job opened that job on page 2 of 2 and displayed its focused-detail indicator. Evidence: `target/w7-jobs-shift-c3a0861/{runner.log,runner.exit,app.sha256}`, `target/w7-jobs-shift-c3a0861.{ps1,py,e2e.mjs}` and `target/w7-jobs-seed-51bd696.py`. This exercises one insertion ordering and actual GUI/IPC navigation, not concurrent production writers or full lifecycle acceptance.

## Windows task detail navigation and persisted metrics - 2026-11-18

At clean tracked test source `ce2fcb5eb66c1ff7e038ac525d9a9f8911277757`, WebDriver exercised the Full Desktop built at `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`, using 21 synthetic completed jobs and one synthetic metrics record in a fresh isolated root. **PASS scoped 1/1:** refreshing recent jobs, opening the latest task from the dashboard, and navigating through the real history-index and metrics IPC path displayed the task's `gallery_dl` backend, `2097152 B`, `1.00 MiB/s` and `3.0 秒`. Evidence: `target/w7-jobs-detail-ce2fcb5/{runner.log,runner.exit,app.sha256}`, `target/w7-jobs-detail-ce2fcb5.{ps1,py,e2e.mjs}` and `target/w7-jobs-seed-51bd696.py`. These values were seeded, not measured from a real transfer; active writer, cancellation and full W7 remain open.

## Windows terminal task state viewport - 2026-11-18

At clean tracked test source `ba7c0afe4fac7093453c54ddd5b955ec85589c4a`, WebDriver exercised the Full Desktop built at `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`, with three synthetic terminal-state jobs in a fresh isolated root. At requested 720×540, the document-width assertion passed and visual inspection found the visible cancelled and failed labels legible without overlap: **PASS scoped**. Evidence: `target/w7-jobs-state-visual-ba7c0af/{runner.log,runner.exit,app.sha256}`, `target/w7-jobs-state-visual-ba7c0af.{ps1,e2e.mjs}`, and screenshot `target/w7-jobs-state-visual-ba7c0af.png` SHA-256 `5c1d6f4f69d3435072f3cff8861a56a765038218e7be1e49a110e7dc75d81cc9`. This does not cover off-screen content, other window sizes, system DPI changes or actual task lifecycle.

## Windows populated task history viewport - 2026-11-18

At clean tracked test source `0901e4b509ba00467d59743c61ad3e959af52add`, WebDriver exercised 21 synthetic completed jobs in a fresh isolated root using Full Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. At the requested 720×540 window size, the document-width assertion and pagination scenario passed, and visual inspection of the screenshot found the visible heading, first task, fields and state label legible without overlap: **PASS scoped**. Screenshot `target/w7-jobs-visual-retry-0901e4b.png` SHA-256 `6810bf22db0646fe1e9dede74bdae9233f43d998b2132f331304fa21bd6fc08`; runner evidence `target/w7-jobs-visual-retry-0901e4b/{runner.log,runner.exit,app.sha256}`, with test and seed at `target/w7-jobs-visual-0901e4b.e2e.mjs` and `target/w7-jobs-seed-51bd696.py`. The initial run passed pagination but did not create a screenshot because the intended test edit was not applied; it is not visual evidence. Off-screen entries, other sizes and actual OS DPI changes remain open.

## Windows terminal task state display - 2026-11-18

At clean tracked test source `043b04e8cb273356eef4f859f6949f8b00bef5e0`, WebDriver exercised the Full Desktop built at source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. In a fresh isolated portable root, three synthetic database jobs in `COMPLETE`, `FAILED` and `CANCELLED` states appeared with their respective Chinese state labels and both pagination buttons disabled: **PASS scoped 1/1**. Evidence: `target/w7-jobs-state-043b04e/{runner.log,runner.exit,app.sha256}` and `target/w7-jobs-state-043b04e.{ps1,py,e2e.mjs}`. This checks stored-state display only, not actual execution, cancellation, failure classification, active writers or full W7.

## Windows populated task history pagination - 2026-11-18

At clean tracked test source `51bd696424bdd1820be5d413d6a163e020acdb64`, WebDriver exercised the Full Desktop built at source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. In a new `XARCHIVE_PORTABLE_ROOT`, the initialized database was seeded with 21 synthetic completed jobs. **PASS scoped 1/1:** the actual task-history UI displayed 20 entries on the first page and one on the second, correctly enabled or disabled the previous/next controls, and returned to the first page with the expected ordering. Evidence: `target/w7-jobs-page-51bd696/{runner.log,runner.exit,app.sha256}`, `target/w7-jobs-page-51bd696.{ps1,e2e.mjs}`, and `target/w7-jobs-seed-51bd696.py`. This is a synthetic completed-history and pagination check; it does not cover active writers, failed jobs, visual/DPI acceptance or full W7.

## Windows SQLite declared source identity - 2026-11-18

At clean tracked source `10a4b11d29ce708f9c4e4d882c00b10ceee89170`, static string inspection found that the pinned gallery and official Python 3.14.7 SQLite DLLs both contain source ID `2025-07-30 19:33:53 4d8adfb30e03f9cf27f800a2c1ba3c48fb4ca1b08b0f5ed59a4d5ecbf45e20a3`, corresponding to the [published SQLite 3.50.4 release identity](https://sqlite.org/releaselog/3_50_4.html). Evidence: `target/w6-sqlite-sourceid-10a4b11.{py,log,exit}`. This is a declared string, not proof of exact source, compiler options or equivalent behavior; the previously documented code-byte differences and provenance review remain open. No DLL or downloader was executed.

## Windows SQLite upstream binary check - 2026-11-18

At clean tracked source `347a68886e99cf19d4cb659601b919194adf0d4b`, the upstream [SQLite 3.50.4 x64 DLL archive](https://sqlite.org/2025/sqlite-dll-win-x64-3500400.zip) was downloaded as data. Its observed SHA3-256 `8454a8ef362b4b2d5a259a54948ed278ef943128bf1ba74b5cbd87ebc58e5b85` matches a historical value reproduced on the SQLite forum, not an independently retrieved contemporaneous release manifest. The archive DLL SHA-256 is `70450594d9f5743484723e8a27b08f76257e32854fbe6943c96b2aefcdf1ce3b` and it does not byte-match either previously extracted DLL. Evidence: `target/w6-sqlite-upstream-347a688.{py,log,exit,zip}`. This is not proof of the build origin or behavior of either installed DLL; provenance and terms remain open under `CROSS_PLATFORM_REVIEW_REQUIRED`.

## Windows gallery SQLite binary comparison - 2026-11-18

At clean tracked source `64b2c87687ba27c434ce69ee0cd0ec5f26af2075`, static inspection of the two previously compared SQLite DLLs found file version `3.50.4.0` in both and valid Authenticode signatures with the same signer certificate. Their SHA-256 values are `9a8fb79d6b7e581d127314028dcecf754b8d7fe4aca1247f82c1f33ee8fccc7e` (gallery) and `d3e60dd22e62c9fbdb64b31b1b8c48782e1d4958d04d0bf830cffd31ace3275f` (Python package). Excluding each PE checksum, security directory metadata and certificate table still leaves 147 differing bytes, including differences in `.text`; signature or timestamp differences alone cannot explain them. Evidence: `target/w6-sqlite-{64b2c87,pe-64b2c87,diff-64b2c87}.{py,log,exit}` and `target/w6-sqlite-signature-64b2c87.log`. This is not a determination of source, behavior, defect, or redistribution obligations; `CROSS_PLATFORM_REVIEW_REQUIRED` remains open. No downloader was executed.

## Windows gallery native Python package comparison - 2026-11-18

At clean tracked source `393bd815066652198b227ef340e4776dd3f21207`, the pinned gallery executable and the official, hash-verified Python 3.14.7 Windows x64 embedded archive were compared as data for identically named DLL and PYD entries. Of 26 common entries, 25 matched byte-for-byte and `sqlite3.dll` differed. Evidence: `target/w6-gallery-python-native-393bd81.{py,log,exit}`. This compares common names only; it does not establish that unmatched executable components came from that archive, explain the SQLite difference, or assess its behavior and distribution obligations. The downloader was not executed. `CROSS_PLATFORM_REVIEW_REQUIRED` for provenance and terms.

## Windows gallery project modules with Python 3.14.7 - 2026-11-18

At clean tracked source `f754bf0ffdc6b9a6f307a2ea0b33cf6fd9bfee94`, the previously hash-verified release-referenced gallery source archive was compiled with official Python 3.14.7, whose DLL matches the pinned executable. All 345 embedded gallery project modules had source entries; 344 matched selected code-object fields and only `gallery_dl.version` differed. Evidence: `target/w6-gallery-modules-f754bf0.json`, `target/w6-gallery-compile3147-f754bf0.py`, `target/w6-gallery-compare3147-f754bf0.{py,log,exit}`. The comparison excludes line and exception tables, data, native components and runtime behavior. The executable version module identifies the Windows build as recorded in the earlier comparison; the origin of the difference is not proven. Full provenance and redistribution remain under `CROSS_PLATFORM_REVIEW_REQUIRED`; the downloader was not executed.

## Windows gallery source comparison with matching Python - 2026-11-18

At clean tracked source `444d9fb6298b3f7a2268e19b3ad81f15e00a7821`, the official, hash-verified Python 3.14.7 embedded distribution whose DLL matches the pinned gallery executable compiled the two previously differing source modules. Selected code-object fields for `gallery_dl.extractor.steamgriddb` now match the executable, while `gallery_dl.version` remains different. This shows that the earlier steamgriddb selected-field mismatch under Python 3.14.10 can arise from the compiler version; it does not prove complete module or runtime equivalence. Only these two modules were recompiled with 3.14.7, so the earlier 343 matches were not re-established under that interpreter. Line and exception tables, data and runtime behavior remain outside the comparison. Evidence: `target/w6-gallery-compile3147-444d9fb.py`, `target/w6-gallery-compare3147-444d9fb.{py,log,exit}`; the downloader was not executed. Version provenance and complete redistribution review remain open, `CROSS_PLATFORM_REVIEW_REQUIRED`.

## Windows pinned gallery Python runtime and licence - 2026-11-18

At test source `5815a4952a11f8b7ddb25886dbbc7113e47358e0`, static inspection of pinned gallery executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` extracted `python314.dll` (file version 3.14.7, SHA-256 `0f9857ffdfe010fe6b99328d58c2e3c7472ce75f336bf9c2ad9bd5bca3bce700`). It matched the official Python 3.14.7 Windows x64 embedded package byte-for-byte. The downloaded package SHA-256 `d297e5ff019966817ad8502465176139f2d3d840fa4ed84b13bed399a6ab1f15` matched the [official release page](https://www.python.org/downloads/release/python-3147/); its licence SHA-256 is `935cf13e19f8c31b497d20b05d73623431a226b230c3599bc30fa3348979bc68`. Evidence `target/w6-python3147-5815a49.{py,log,exit,zip}`. This proves correspondence for the DLL, not every runtime component.

The packaging change committed as `125e065652792a522c9f4248ea23edfe97a0bfce` verifies and includes that licence for the pinned binary. **PASS scoped:** 40/40 packaging/security tests (`target/w6-gallery-python-package-real-5815a49.{log,exit}`), actual pinned-asset CLI, isolated Full portable assembly (`target/w6-gallery-python-portable-real-5815a49.{log,exit}`), simulated `core.autocrlf=true` checkout and CLI, architecture check and staged diff check. The licence retains upstream CRLF bytes. An initial write did not execute; the initial 40/40 run therefore tested unchanged code, and an initial portable command emitted missing-file errors despite its exit zero. Those are not acceptance evidence; the separately named real runs above passed after the changes were applied. Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Hosted workflow, other dependencies and full redistribution review remain open; `CROSS_PLATFORM_REVIEW_REQUIRED`. No downloader was executed.

## Windows keyboard navigation to logs - 2026-11-18

At clean tracked test source `4c58c73`, WebDriver on the same Full Desktop (source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`) passed 1/1: after the first main-navigation button was focused programmatically, six Tab presses reached the log navigation button, Enter opened the log page, and its button reported `aria-current=page`. Evidence: `target/w7-keyboard-nav-4c58c73.e2e.mjs` and `target/w7-keyboard-nav-4c58c73/{runner.log,runner.exit,app.sha256}`. This is one navigation path, not complete keyboard or accessibility acceptance.

## Windows log page keyboard interaction - 2026-11-18

At clean tracked test source `7dc38cc`, WebDriver on the previously built Full Desktop (source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`) passed a scoped log page keyboard check 1/1. The test focused the level selector programmatically, sent End (observed level `silent`), checked focus, used Tab to reach search, typed a marker, and cleared it with Ctrl+A and Backspace. Evidence: `target/w7-keyboard-log-7dc38cc.e2e.mjs` and `target/w7-keyboard-log-retry-7dc38cc/{runner.log,runner.exit,app.sha256}`. The initial script stopped before launching the driver because it checked an existing unrelated fixture; the retry used a new evidence directory and corrected fixture check. The checked-in opt-in WebDriver suite, with the added keyboard case subsequently committed as `2c12fd4c94dcce3ee13c41c3ca6e53549793f0fd`, passed both log filtering and keyboard cases 2/2 at source `16323d5` plus that test change. Evidence: `target/w7-keyboard-suite-16323d5/{runner.log,runner.exit,app.sha256}`. The test does not cover the complete keyboard path to the selector, full accessibility, or the DPI matrix.

## Windows pinned gallery project source comparison - 2026-11-18

At clean tracked source `36f1e067a9838e47618482d9ff6eadc6549344e8`, the previously retained release-referenced source archive SHA-256 `4dda5c5ead73951af9c9e247df52a66da4d5c850f4efbbdc73c1b5085a84605e` was compared using Python 3.14.10 with pinned gallery-dl executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a`. Selected code-object fields matched 343 of 345 embedded project modules; version and steamgriddb differed and none were missing (target/w6-gallery-source-36f1e06.py, .log, .exit). The executable version declares 1.32.13:2026.09.20 and dev/windows; the source declares 1.32.13. The steamgriddb icon extractor differs in bytecode and constants; cause and behavioral equivalence remain unestablished (target/w6-gallery-difference-36f1e06.py, .log, .exit). Line and exception tables, data, native components and runtime behavior were not compared. No downloader was executed. CROSS_PLATFORM_REVIEW_REQUIRED for provenance and redistribution review.

## Windows pinned gallery yt-dlp licence packaging — 2026-11-18

At source `93a89ed` plus changes committed as `b572933322121204e154a105d980ff508eb5306c`, Windows release packaging verifies and copies the yt-dlp licence for pinned gallery executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a`; portable packaging requires and verifies all six notices for that exact binary. The licence from the verified yt-dlp source archive has SHA-256 `7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c`. **PASS scoped:** Node packaging/security tests 40/40 (`target/w6-ytdlp-package-93a89ed.log`), actual pinned-asset CLI, isolated Full portable assembly (`target/w6-ytdlp-portable-93a89ed.log`), simulated `core.autocrlf=true` checkout and CLI, architecture check zero findings, and staged diff check. The first checkout attempt failed because its parent directory did not exist; after creating it the checkout and CLI passed. The reused Desktop was built at source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. No downloader was executed. Hosted workflows, other dependency provenance and redistribution obligations remain open; `CROSS_PLATFORM_REVIEW_REQUIRED` applies to the packaging and notice changes.

## Windows pinned gallery yt-dlp source comparison — 2026-11-18

At clean tracked test source `5086ef1fed1b8f6e4690fcc5d0b510f406e2cfe8`, the PyPI yt-dlp 2026.8.19 source archive was retrieved and checked against its published SHA-256 `9e213e48cea35c66b378e4447903f118f6392a5fa380a2b6d7070ec86f4e0af1`. Static comparison using Python 3.14.10 against pinned gallery-dl executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` matched selected code-object fields for all 1,043 embedded yt-dlp Python modules, with zero different or missing modules. Line and exception tables, non-code data and runtime behavior were not compared. The source `LICENSE` SHA-256 is `7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c`. Evidence `target/w6-ytdlp-{5086ef1,license-5086ef1}.{py,log,exit}`. No downloader was executed and no yt-dlp notice has yet been packaged; other components, complete build provenance and redistribution review remain open.

## Windows Job candidate source review — 2026-11-18

At clean tracked source `6758ff7`, crates.io metadata identified `cosca` 0.9.0, and the source archive was retrieved and checked against its published SHA-256 `c55d2fb20a5aa9758b390f0a71a367fc9de044f2a8a54e6b91d8d94750ad2b54` (`target/w1-cosca-{6758ff7,source-6758ff7}.{py,log,exit}`). Its public Job API creates a fresh job when assigning a process; the reviewed `job.rs` exposes no named-job reopen. Its own source distinguishes handles captured before kill from a child created during kill, for which it observes departure from the process list. Therefore this API cannot directly supply the existing keeper-backed successor recovery or the required historical-descendant exit proof. This is a bounded static API review, not a full dependency audit or native test; no dependency was added and W1 stays fail-closed.

## Windows gallery Cryptodome legal-material inventory — 2026-11-18

At clean tracked source `31e3cf3c69a32ac9af36e68b8568b208679a1a7d`, the already hash-verified pycryptodomex 3.23.0 source archive's `Doc/LEGAL` was inventoried (`target/w6-cryptodome-legal-31e3cf3.{py,log,exit}`). It contains a historical PyCrypto copyright explanation, contributor statements and distinct historical PyCrypto, LibTomCrypt and Python 2.2 licence texts. The copyright explanation SHA-256 is `8e563c767164faa0831a333b57d23d4311cf566eb1b15d93250f4606be4eb549`; hashes for the other inspected files are in the log. The official Windows wheel contains its main `LICENSE.rst`, whose text matches the source after CRLF normalization (`target/w6-cryptodome-{wheel-license,license-compare}-fb0a77c.{py,log,exit}`). This is a descriptive inventory, not a determination of which additional notices apply to the exact bundled binary. Those materials and full distribution obligations remain open for Cross-platform Owner review.

## Windows pinned gallery Cryptodome licence packaging — 2026-11-18

At source `fb0a77c22ce72c4b434dad2418c039459f9a5096` plus changes committed unchanged as `f45eba387f4419ec2264826317c8e40d8e485913`, Windows release packaging adds the upstream Cryptodome main licence after verifying the fixed gallery executable and five licence hashes; portable packaging requires and verifies the notice for that exact binary before replacing output. The source and official wheel licence texts match after CRLF normalization (`target/w6-cryptodome-{wheel-license,license-compare}-fb0a77c.{py,log,exit}`); separate upstream legal materials and other obligations remain open. Other binaries still require an operator-provided project licence and independent provenance review. **PASS scoped:** packaging/security Node tests 40/40, actual pinned-asset CLI, isolated Full assembly, simulated `core.autocrlf=true` checkout and CLI, architecture check zero findings and `git diff --check`. Evidence `target/w6-cryptodome-{package,cli,full,checkout}-fb0a77c.{log,exit}`; packaged licence SHA-256 `4e04660d77c1c64e89d79537919fb8240fa21484a7e3db29f358b2c7f84ea073`. Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Hosted workflows and complete dependency/source/terms review remain open; `CROSS_PLATFORM_REVIEW_REQUIRED` for shared packaging and notice changes. No downloader was executed.

## Windows pinned gallery Cryptodome correspondence — 2026-11-18

At clean tracked test source `e175fab8d41ace96f2b2fe52b697ec359b423db7`, the PyPI pycryptodomex 3.23.0 source archive SHA-256 `71909758f010c82bc99b0abf4ea12012c98962fbf0583c2164f8b84533c2e4da` and Windows x64 ABI3 wheel SHA-256 `52e5ca58c3a0b0bd5e100a9fbc8015059b05cffc6c66ce9d98b4b45e023443b9` were retrieved and hash-checked against PyPI metadata. Static comparison against pinned gallery-dl executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` matched selected code-object fields for 68 embedded Cryptodome Python modules (zero differences or missing modules), and all 42 embedded Cryptodome `.pyd` files matched the wheel byte-for-byte. Line and exception tables and runtime behavior were not compared. Source `LICENSE.rst` SHA-256 `4e04660d77c1c64e89d79537919fb8240fa21484a7e3db29f358b2c7f84ea073`; the source archive also contains distinct legal files, which require review. Evidence `target/w6-cryptodome-{e175fab,license-e175fab,wheels-e175fab,wheel-compare-e175fab}.{py,log,exit}`. No downloader was executed; no Cryptodome notice has yet been packaged, and full source and redistribution review remains open.

## Windows pinned gallery urllib3 licence packaging — 2026-11-18

At source `6380fa0530a472a1b669b26f9f2fcb739ee44e92` plus changes committed unchanged as `6e57da291aaa3bf5f45a63683de3febd7bba5066`, Windows release packaging adds the upstream urllib3 licence after verifying the fixed gallery executable and four licence hashes; portable packaging requires and verifies the notice for that exact binary before replacing output. Other binaries still require an operator-provided project licence and independent provenance review. **PASS scoped:** packaging/security Node tests 40/40, actual pinned-asset CLI, isolated Full assembly, simulated `core.autocrlf=true` checkout and CLI, architecture check zero findings and `git diff --check`. Evidence `target/w6-urllib3-{package,cli,full,checkout}-6380fa0.{log,exit}`; packaged urllib3 licence SHA-256 `130e3a64d5fdd5d096a752694634a7d9df284469de86e5732100268041e3d686`. Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Hosted workflows and complete dependency/source/terms review remain open; `CROSS_PLATFORM_REVIEW_REQUIRED` for shared packaging and notice changes. No downloader was executed.

## Windows pinned gallery urllib3 source comparison — 2026-11-18

At clean tracked test source `788173432e36a6c8b322b7250585a5ae45fb8f43`, the PyPI urllib3 2.8.0 source archive was retrieved and its SHA-256 checked against PyPI metadata (`63bf2ead4c879426ebf22ef2a781eeb4aa3b4ae798a0435506f8687fd5bb9b63`). Static comparison against the pinned gallery-dl executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` matched selected code-object fields for all 36 embedded urllib3 modules, with zero different or missing modules; line and exception tables were not compared. The retrieved source `LICENSE.txt` SHA-256 is `130e3a64d5fdd5d096a752694634a7d9df284469de86e5732100268041e3d686`. Evidence `target/w6-urllib3-{7881734,license-7881734}.{py,log,exit}`. The downloader was not executed; this scoped correspondence does not close other dependencies, binary build provenance or release terms. No urllib3 notice has yet been packaged.

## Windows gallery notice CLI regression — 2026-11-18

At source `eac5ce774f9bf1c769658a760b447caa8e057403` plus the test committed unchanged as `e16a98d7650b53cbb5e9c8c1d111167a747a0948`, the release-security suite **PASS 11/11**, including a new isolated child-process invocation of the actual gallery licence CLI with a synthetic pinned asset and three notices (`target/w6-gallery-cli-regression-eac5ce7.{log,exit}`). The test verifies that the CLI copies the verified project, websockets and certifi notices; architecture check reported zero findings. This closes the test gap that had failed to catch a missing CLI variable during development, but is not a hosted workflow or a complete dependency/redistribution acceptance.

## Windows pinned gallery certifi licence packaging — 2026-11-18

At source `e658d562bf52858a97a9e3a135cd67a9e75b34be` plus changes committed unchanged as `54a6daeca1788aaa29173caf00f22589c8ce0ca4`, Windows release packaging adds the upstream certifi licence after verifying the fixed gallery executable and all three licence hashes; portable packaging requires and verifies the certifi notice for that exact binary before replacing output. Other binaries still require an operator-provided project licence and independent provenance review. **PASS scoped:** packaging/security Node tests 39/39, actual pinned-asset CLI, isolated Full assembly, simulated `core.autocrlf=true` checkout and CLI, architecture check zero findings and `git diff --check`. Evidence `target/w6-certifi-{final,cli,full,checkout}-e658d56.{log,exit}`; packaged certifi licence SHA-256 `e93716da6b9c0d5a4a1df60fe695b370f0695603d21f6f83f053e42cfc10caf7`. Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Hosted workflows and complete dependency/source/terms review remain open; `CROSS_PLATFORM_REVIEW_REQUIRED` for shared packaging and notice changes. No downloader was executed.

## Windows pinned gallery certifi source comparison — 2026-11-18

At clean tracked test source `7dbdcd696fc63d8cca2496f6a3e27dc889770396`, the PyPI certifi 2026.7.22 source archive was retrieved and its SHA-256 checked against PyPI metadata (`741e2c3b351ddf169a738da9f2c048608ff7f2c5cc02f1ebc6b118bb090d5d55`). Static comparison of the pinned gallery-dl executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` against that source matched selected code-object fields for both embedded certifi modules, with zero differences or missing modules; line and exception tables were not compared. The bundled `certifi/cacert.pem` matched the source byte-for-byte, SHA-256 `9cc2a774b5198dcff14d9be1e66091f538975d867ce029a96bce15a55dfd730f`. The retrieved source `LICENSE` SHA-256 is `e93716da6b9c0d5a4a1df60fe695b370f0695603d21f6f83f053e42cfc10caf7`. Evidence `target/w6-certifi-{7dbdcd6,data-7dbdcd6}.{py,log,exit}`. The downloader was not executed; this scoped correspondence does not close other dependencies, binary build provenance or release terms. No additional notice has yet been packaged.

## Windows pinned gallery dependency version constants — 2026-11-18

At clean tracked test source `3e6d2858883c707e704cf4ccf780e78c584481c0`, the pinned gallery-dl executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` was inspected as data using the existing PyInstaller archive reader. Embedded Python code-object constants declare yt-dlp `2026.08.19` and release Git head `594bd50c2c78ac432f81600d309fdc4e0a92d82c`, urllib3 `2.8.0`, certifi `2026.07.22`, and Cryptodome version tuple `(3, 23, "0")`. Evidence `target/w6-gallery-{versions,crypto}-3e6d285.{py,log,exit}`. These are static declarations, not independently verified package source, licence completeness, binary provenance or runtime behavior. The downloader was not executed. W6's complete dependency and redistribution review remains open.

## Windows repeatable log filter test — 2026-11-18

At source `3048285597e49f341f1e918cb6624b6cc6db496a` plus the test committed unchanged as `62c8f8d7b5d2f6a5e3b30fb12ee9f00716b719ee`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. The repository WDIO test `desktop/e2e/specs/log-filter.e2e.mjs`, enabled with `WDIO_LOG_FILTER=1`, **PASS scoped 1/1**: it wrote a unique synthetic diagnostic through actual IPC, preserved the Error filter through the refresh interval, then observed the matching Info event and enabled copy after changing the filter. Evidence `target/w7-log-durable-3048285/{runner.log,runner.exit,app.sha256}` and `target/w7-log-durable-3048285.ps1`. Architecture check reported zero findings. This does not complete W7 or prove actual clipboard behavior.

## Windows Full log filter during refresh — 2026-11-18

At clean tracked test source `ece3fe4a3cde2029f4ae54a0d7ef8c4543bb2da1`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** after selecting Error, a synthetic Info diagnostic was written through real IPC; the selected level remained Error and the diagnostic was hidden after 2.5 seconds, then appeared after selecting Info. Evidence `target/w7-log-poll-ece3fe4/{runner.log,runner.exit,app.sha256}` and `target/w7-log-poll-ece3fe4.{ps1,e2e.mjs}`. This does not establish all levels or full W7 acceptance; the earlier startup-discovery failure remains recorded.

## Windows profile DPI observation — 2026-11-18

At clean tracked source `ce82f0ae410fbc9642cfeab64112e42fc38beb7e`, a read-only query of the current Windows account's `HKCU/Control Panel/Desktop/WindowMetrics/AppliedDPI` returned `192`; `Environment.OSVersion` reported `10.0.29708.0` (`target/w7-profile-dpi-ce82f0a.{log,exit}`). The preceding Full WebView check separately reported `devicePixelRatio=2`. The profile registry value and WebView value are consistent with 200% scaling, but do not independently prove per-monitor scaling or validate the 100% and 150% scenarios. No system setting was changed.

## Windows Full measured WebView pixel ratio — 2026-11-18

At clean tracked test source `098928d99f977817abba946f4d82d4edf7fa44c2`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** dashboard geometry and heading/navigation checks passed at reported 720×540 and 1080×760 CSS viewports; WebView reported `devicePixelRatio=2` at both, with CSS screen 1280×800. Evidence `target/w7-dpi-098928d/{runner.log,runner.exit,app.sha256}` and `target/w7-dpi-098928d.{ps1,e2e.mjs}`. The browser value alone does not establish the operating system's configured scale or independently validate 100%, 150% and 200% DPI scenarios.

## Windows Full runtime log level filter — 2026-11-18

At clean tracked test source `73ae53f3ec1e6f4834962142f5fdd2070ee28ff4`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** a synthetic info diagnostic written through real frontend IPC appeared under an info filter, was hidden under error, and appeared again after returning to info. Evidence `target/w7-log-level-73ae53f/{runner.log,runner.exit,app.sha256}` and `target/w7-log-level-73ae53f.{ps1,e2e.mjs}`. Other levels, clipboard and full W7 remain open; the earlier startup-discovery failure remains recorded.

## Windows Full runtime log positive filter — 2026-11-18

At clean tracked test source `115efe0882674e316cdcfef0cbec4646f18b026e`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** a synthetic diagnostic written through the real frontend IPC appeared under a matching log search with copy enabled, and a nonmatching search showed the empty result with copy disabled. Evidence `target/w7-log-positive-115efe0/{runner.log,runner.exit,app.sha256}` and `target/w7-log-positive-115efe0.{ps1,e2e.mjs}`. No actual clipboard operation or full W7 acceptance is implied; the previous startup-discovery failure remains recorded.

## Windows Full runtime log filter — 2026-11-18

At clean tracked test source `2fd906c0336b658c755cb05f4a457e9221585197`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. The first WebDriver run **FAILed before reaching the application page**: all sampled handles remained blank until startup discovery timed out (`target/w7-log-filter-2fd906c/{runner.log,runner.exit,wdio/startup/discovery.json}`). The cause is not established. One retry **PASS scoped 1/1**: after log loading, a synthetic nonmatching search showed the empty result and disabled copy, and the search field cleared (`target/w7-log-filter-retry-2fd906c/{runner.log,runner.exit,app.sha256}`, `target/w7-log-filter-retry-2fd906c.ps1`, `target/w7-log-filter-2fd906c.e2e.mjs`). No log contents were recorded. The initial automation failure remains part of the result; populated matching/filter behavior and full W7 remain open.

## Windows Full download configuration and storage screenshots — 2026-11-18

At clean tracked test source `33b352c167e0a9fc87f04f85988da79a1df00552`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. WebDriver captured and the agent inspected initial download-configuration and storage viewports at reported 720×540. **PASS scoped:** visible headings and controls were legible without overlap; storage was collapsed and download configuration continued below the viewport. Evidence `target/w7-download-storage-33b352c/{runner.log,runner.exit,app.sha256}`, `target/w7-download-storage-33b352c-{download,storage}.png` and `target/w7-download-storage-33b352c.{ps1,e2e.mjs}`. Expanded storage, off-screen content, interactions and the DPI matrix remain open.

## Windows Full account and settings screenshot review — 2026-11-18

At clean tracked test source `a8bf514c3408d955931e385abbce50b4db156e37`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. WebDriver captured and the agent inspected initial account-archive and settings viewports at reported 720×540. **PASS scoped:** visible headings and controls were legible without overlap. Evidence `target/w7-account-settings-a8bf514/{runner.log,runner.exit,app.sha256}`, `target/w7-account-settings-a8bf514-{accounts,settings}.png` and `target/w7-account-settings-a8bf514.{ps1,e2e.mjs}`. Off-screen content, expanded settings, interactions and the DPI matrix remain open.

## Windows Full settled empty task history — 2026-11-18

At clean tracked test source `b414b94d15b809f365843fe957f18f7890a8991d`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** at reported 720×540 the task page reached the empty state and both pagination buttons were disabled. The agent inspected the resulting screenshot; its visible heading, empty-state text and pagination did not overlap. Evidence `target/w7-settled-b414b94/{runner.log,runner.exit,app.sha256}`, `target/w7-settled-b414b94.png` and `target/w7-settled-b414b94.{ps1,e2e.mjs}`. Populated tasks, other DPI values and full W7 remain open.

## Windows Full task and import screenshot review — 2026-11-18

At clean tracked test source `8541f3b40ad2b68330ccbcc52f35257742574b6d`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. WebDriver captured and the agent inspected task-history and record-import initial viewports at reported 720×540. **PASS scoped:** the visible headings and controls were legible without overlap; task history was in a loading state and the import page continued below the viewport. Evidence `target/w7-pages-visual-8541f3b/{runner.log,runner.exit,app.sha256}`, `target/w7-pages-visual-8541f3b-{tasks,import}.png` and `target/w7-pages-visual-8541f3b.{ps1,e2e.mjs}`. This does not cover settled or populated task data, below-fold controls, interactions or the DPI matrix.

## Windows Full dashboard screenshot review — 2026-11-18

At clean tracked test source `64efb81`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. WebDriver captured and the agent inspected the initial dashboard viewport at reported 720×540 and 1080×760. **PASS scoped:** visible heading, setup text and buttons were legible and did not overlap; the narrow view displayed icon-only navigation and the wide view displayed navigation labels. Screenshots show loading state and do not cover off-screen content, interactions, real system DPI switching or other pages. Evidence `target/w7-visual-retry-64efb81/{runner.log,runner.exit,app.sha256}`, `target/w7-visual-retry-64efb81-{720,1080}.png` and `target/w7-visual-retry-64efb81.{ps1,e2e.mjs}`. First capture `target/w7-visual-64efb81-.png` was overwritten by the second size because the filename omitted width; it is not evidence for both sizes.

## Windows Full populated preview window sizes — 2026-11-18

At clean tracked test source `7fe0dbcd80114d45d942bb24533d69ec49930353`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** after previewing one synthetic valid record, the selection remained checked and accept enabled at reported viewports 720×540 and 1080×760, with no document horizontal overflow. Evidence `target/w7-populated-7fe0dbc/{runner.log,runner.exit,app.sha256}` and `target/w7-populated-7fe0dbc.{ps1,e2e.mjs}`. No actual acceptance, visual inspection, long data or DPI matrix was covered.

## Windows Full record import and logs window sizes — 2026-11-18

At clean tracked test source `91db8cf31a92259d9a4bf2a9bb820937979ea43d`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped:** record-import and runtime-logs pages displayed heading and navigation without document horizontal overflow at reported viewports 720×540 and 1080×760. Evidence `target/w7-more-sizes-91db8cf/{runner.log,runner.exit,app.sha256}` and `target/w7-more-sizes-91db8cf.{ps1,e2e.mjs}`. This DOM/geometry check covers default page state at current DPI; populated data, visual review and the DPI matrix remain open.

## Windows Full four page window sizes — 2026-11-18

At clean tracked test source `d6af045508916f6db40a6b5d979a2eb9c66aef09`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped:** account archive, download configuration, storage and settings each displayed their heading and navigation without document horizontal overflow at reported viewports 720×540 and 1080×760. Evidence `target/w7-pages-sizes-d6af045/{runner.log,runner.exit,app.sha256}` and `target/w7-pages-sizes-d6af045.{ps1,e2e.mjs}`. This DOM/geometry check covers default page state at current DPI; populated data, visual review and the DPI matrix remain open.

## Windows Full task history window sizes — 2026-11-18

At clean tracked test source `261664a60a4d59927e2229bd31c3425d48db0413`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** task-history heading and navigation were displayed without document horizontal overflow at reported viewports 720×540 and 1080×760. Evidence `target/w7-task-sizes-261664a/{runner.log,runner.exit,app.sha256}` and `target/w7-task-sizes-261664a.{ps1,e2e.mjs}`. This DOM/geometry check does not cover populated task data, visual review or the DPI matrix.

## Windows Full dashboard window sizes — 2026-11-18

At clean tracked test source `3c821c7db99f69adcd89f47ac7c856d7b96c2baa`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** WebDriver resized the dashboard to reported viewport sizes 720×540 and 1080×760; heading and navigation were displayed and the document had no horizontal overflow at either size. Evidence `target/w7-sizes-3c821c7/{runner.log,runner.exit,app.sha256}` and `target/w7-sizes-3c821c7.{ps1,e2e.mjs}`. This is a DOM/geometry check at the current DPI, not visual review or the complete DPI and page matrix.

## Windows Job accounting counter review — 2026-11-18

At clean tracked source `d2bf3542feb5c204a824c5e9cd6d841b587d5677`, the W1 adapter's use of `BasicAccountingInformation` was reviewed against [Microsoft's field definitions](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_accounting_information). `TotalProcesses` counts associations including failed limit-violation associations; `TotalTerminatedProcesses` counts processes terminated because of limit violations, rather than all exits. Therefore comparing those counters is not a complete historical-descendant exit proof. The documented `ActiveProcesses` description also does not supersede the already reproduced case where a process handle remained unsignaled after the active count reached zero; production drain evidence remains gated. This source review is not a native validation or a change to W1 status.

## Windows Full archive record round-trip — 2026-11-18

At clean tracked test source `f6561d068dbdcf7a6dfc8e913a82294ede274126`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. With `WDIO_F3_ARCHIVE=1` and `TEMP`/`TMP` set to the disposable workspace directory `target/w7-f3-temp-f6561d0`, the dashboard suite **PASS 5/5**. The previously skipped real IPC case previewed and accepted a synthetic archive, exported YAML, verified identical reimport and stale revision rejection, then rejected altered media from acceptance. Evidence `target/w7-dashboard-f6561d0/{runner.log,runner.exit,app.sha256}` and `target/w7-dashboard-f6561d0.ps1`. This does not establish production writer drain or the full GUI matrix.

## Windows Full dashboard smoke — 2026-11-18

At clean tracked test source `5e5f85d7c08797899d5556aa2b33d4befc3590c2`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Dashboard smoke **PASS 4, SKIP 1**: startup contract, native shell, dashboard regions and invalid record-preview rejection passed; archive round-trip was skipped because its opt-in was not set. Evidence `target/w7-dashboard-5e5f85d/{runner.log,runner.exit,app.sha256}` and `target/w7-dashboard-5e5f85d.ps1`. This is not full W7 or production lifecycle acceptance.

## Windows Full emulated theme check — 2026-11-18

At clean tracked test source `7b091014cb5d5252256c0986ab4a29b9ce132952`, Windows 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the Full fixture assembled with packaging fix `8f39b6c` used Desktop source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** the committed theme test emulated light and dark media preferences and checked the computed surface colour and explicit theme overrides. Evidence `target/w7-theme-7b09101/{runner.log,runner.exit,app.sha256}` and `target/w7-theme-7b09101.ps1`. This uses emulated WebView media preferences; OS theme switching, visual appearance and DPI matrix remain open.

## Windows worker Python licence source comparison — 2026-11-18

At clean tracked source `6778a0b2c0db5b0a6bdcf2031d904016960c275c`, the controlled Python 3.12.14 installation licence (SHA-256 `886a0ead2d89030ee62dbff52b04e47ab91998341295bb9c56fb952b4e081c7a`) was compared with [CPython v3.12.14 LICENSE](https://github.com/python/cpython/blob/v3.12.14/LICENSE), SHA-256 `3b2f81fe21d181c499c59a256c8e1968455d6689d269aa85373bfb6af41da3bf`. The complete files differ: after CRLF normalization the upstream file occurs at offset zero in the longer installation file. Evidence `target/w6-python-{upstream,license-detail}-6778a0b.{py,log,exit}`. This comparison supports the packaged text's upstream relationship; it does not prove binary build provenance or complete dependency and redistribution coverage.

## Windows controlled worker MSVC runtime identity — 2026-11-18

At clean tracked source `f4fd9a8`, the controlled worker's `VCRUNTIME140.dll` matched byte-for-byte the x64 file in this machine's Visual Studio Build Tools 2022 17.14.37906.3 `VC/Redist/MSVC/14.44.35112` directory, SHA-256 `d5e4d9a3e835fa679450145d6a7d94e36573a509317111904d9b3712c30d9066` (**PASS scoped**; `target/w6-msvc-redist-f4fd9a8.{py,log,exit}`). The `onecore` x64 file had a different hash. This establishes local file identity, not the release operator's licence eligibility or complete redistribution terms; see Microsoft's [redistribution documentation](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files).

## Windows Full record selection by keyboard — 2026-11-18

At clean tracked test source `0dc521cba73a4c1acaf07bbe64c9d01e93e55200`, Windows 29708.1003, WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the isolated Full assembly with packaging fix `8f39b6c` used Desktop build source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** Space deselected and reselected a valid preview record and acceptance availability followed the selection. The test populated inputs, opened preview and focused the checkbox using automation; it does not establish complete keyboard navigation or accessibility. Evidence: `target/w7-key-select-0dc521c/{runner.log,runner.exit,app.sha256}` and `target/w7-key-select-0dc521c.{ps1,e2e.mjs}`.

## Windows controlled worker ZIP — 2026-11-18

At clean tracked test source `e119202`, the Windows worker workflow's `Compress-Archive -Path <worker>\*` packaging form was applied locally to the previously validated controlled worker fixture. The ZIP's Python and OpenSSL notices, Python and OpenSSL DLLs and worker executable matched the five recorded SHA-256 values (**PASS 5/5**; `target/w6-worker-zip-e119202.{py,log,exit}`, archive `target/w6-worker-openssl-e119202.zip`). This local ZIP is not a hosted workflow or released artifact; complete worker provenance and redistribution review remain open.

## Windows worker OpenSSL 3 notice — 2026-11-18

At source `cfae09ad1737fcb7275cb9473559db2ad26f9c82` plus changes committed unchanged as `8f39b6cee200d6746f8818fbee4b18a1fbc440f7`, upstream OpenSSL 3.0.0, 3.5.8 and 3.6.0 licence files matched SHA-256 `7d5450cb2d142651b8afa315b5f238efc805dad827d91ba367d8516bc9d49e7a` (`target/w6-openssl-license-versions-cfae09a.{py,log,exit}`). The workflow's existing notice helper now copies this vendored and hash-checked text when the worker contains a `libcrypto-3*.dll`; portable Full preflight requires a nonempty notice for such a DLL before replacing output. For independently supplied workers, the DLL identity and notice provenance remain the operator's responsibility. The OpenSSL upstream licence is at [the 3.5.8 tag](https://github.com/openssl/openssl/blob/openssl-3.5.8/LICENSE.txt).

**PASS scoped:** Python tests 2/2, packaging and security Node tests 39/39, architecture check 0 findings, controlled worker helper, Windows autocrlf checkout/helper and isolated Full assembly (`target/w6-openssl-{unit,node,worker,checkout,portable}-cfae09a.{log,exit}`). The packaged OpenSSL licence hash matched the value above; bundled DLL SHA-256 `068eff510340d7c4f3ed7fc065e3189cfa55647dac2289c5cbe2b2b68244dd94` matched the controlled Python installation; Desktop source `79447104ca8ac92e5a8424e41040534528db2322` / SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. Hosted workflows are `NOT_RUN`. Filename detection and this notice do not prove full binary/source correspondence, other components' notices or redistribution eligibility. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared packaging script and notices.

## Windows Full missing archive-root recovery — 2026-11-18

At clean tracked test source `097adf4567abd6822b86e373bffe47122c7e8b0b`, Windows 29708.1003, WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the isolated Full package assembled with `c203bbe` packaging changes used Desktop build source `79447104ca8ac92e5a8424e41040534528db2322` and executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** an absent synthetic archive root produced a preview failure and disabled acceptance, then correcting the root allowed the preview and enabled acceptance. Evidence: `target/w7-missing-root-097adf4/{runner.log,runner.exit,app.sha256}` and `target/w7-missing-root-097adf4.{ps1,e2e.mjs}`. No actual import or native dialog interaction was tested.

## Windows Full record-import keyboard path — 2026-11-18

At clean tracked test source `5350b6f`, Windows 29708.1003, WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the isolated Full package assembled with `c203bbe` packaging changes used Desktop build source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** after preparing synthetic inputs and focusing the document field, native WebDriver Tab moved focus through archive root and preview, and Enter triggered an actual successful preview. Evidence: `target/w7-keyboard-5350b6f/{runner.log,runner.exit,app.sha256}` and `target/w7-keyboard-5350b6f.{ps1,e2e.mjs}`. Input population and initial focus used automation; this does not complete keyboard accessibility, other pages, native-dialog interaction or the DPI/window matrix.

## Windows Full invalid record input recovery — 2026-11-18

At clean tracked test source `9e53fe63f064fe6ef1395002be65a135727458d1`, Windows 29708.1003, WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the isolated Full assembly from `c203bbe` packaging changes was exercised with Desktop build source `79447104ca8ac92e5a8424e41040534528db2322` and executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. **PASS scoped 1/1:** an existing record preview cleared on invalid input, acceptance was disabled, the invalid preview failed, and valid input could subsequently be previewed again. Evidence: `target/w7-invalid-9e53fe6/{runner.log,runner.exit,app.sha256}` and `target/w7-invalid-9e53fe6.{ps1,e2e.mjs}`. The prior intermittent timeout remains recorded; this result does not cover the remaining GUI matrix or real writer lifecycle.

## Windows worker runtime provenance follow-up — 2026-11-18

At clean tracked source `da9b41e`, the controlled worker's OpenSSL `libcrypto-3-x64.dll` (file version 3.5.8, SHA-256 `068eff510340d7c4f3ed7fc065e3189cfa55647dac2289c5cbe2b2b68244dd94`) and Microsoft `VCRUNTIME140.dll` (file version 14.44.35211.0, SHA-256 `d5e4d9a3e835fa679450145d6a7d94e36573a509317111904d9b3712c30d9066`) each matched the corresponding file in the Python build installation byte-for-byte (`target/w6-worker-runtime-hashes-da9b41e.{py,log,exit}`). OpenSSL upstream's [3.5.8 licence](https://github.com/openssl/openssl/blob/openssl-3.5.8/LICENSE.txt) was retrieved, SHA-256 `7d5450cb2d142651b8afa315b5f238efc805dad827d91ba367d8516bc9d49e7a`; a guessed `NOTICE.md` returned 404 and the subsequent upstream root listing showed only `LICENSE.txt` among licence/notice-named root entries (`target/w6-openssl-upstream-da9b41e.{py,log,exit}`, `target/w6-openssl-list-da9b41e.{py,log,exit}`). The first retrieval command therefore failed after obtaining the licence. File version and installation matching do not establish the DLL's complete source/build provenance or distribution terms. Microsoft documents that redistribution of its Visual C++ runtime is subject to [licensing conditions](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files); applicability to the eventual distributor remains open. No additional notice has yet been packaged for these components; release inventory and terms remain open.

## Windows worker Python runtime notice — 2026-11-18

At source `e2374a833a5f82b8a205b845c18c5c08cc3e2c33` plus the changes committed unchanged as `c203bbe44842f370a44636a6ea971a61ba903dc7`, the controlled Windows worker executable SHA-256 `65bfce70fdee1b727c0d394454b0f2f6d73d44374c39f424735508c88b3e6615` contains Python DLL SHA-256 `43f622263b40e4894d06fdc34912bebe796ecbca2c395d09c5b2d0b84845f54b`, matching the build Python installation. The installation's `LICENSE.txt`, SHA-256 `886a0ead2d89030ee62dbff52b04e47ab91998341295bb9c56fb952b4e081c7a`, was copied into a separate worker fixture. Both worker workflows now perform that DLL identity check and licence copy; Full portable preflight requires a nonempty notice when the selected worker contains a Python DLL. For an independently supplied worker, the operator must verify notice provenance.

**PASS scoped:** Python unit 1/1, packaging/release Node tests 39/39, exact workflow fragments with matching and mismatching runtime fixtures, architecture check 0 findings and an isolated Full assembly containing the matching DLL and notice. Evidence: `target/w6-worker-license-{unittest,workflow,node-final,fragment-run,arch}-e2374a8.{log,exit}`, `target/w6-worker-license-e2374a8.{log,exit}` and `target/w6-portable-python-e2374a8.{log,exit}`. Desktop executable in that assembly is from source `79447104ca8ac92e5a8424e41040534528db2322`, SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`; pinned gallery executable and its two notices were also hash checked. Hosted GitHub Actions is `NOT_RUN`; this single notice does not close the worker dependency/source inventory or release obligations. No downloader was executed. `CROSS_PLATFORM_REVIEW_REQUIRED` applies to shared packaging and workflow integration.

## Windows gallery-dl websockets source and licence — 2026-11-18

At source `66012a051bfacb34c2d1f5d507864be9f508a186`, the upstream PyPI websockets 17.1 source archive was retrieved and verified against its published SHA-256 `acfea4c20bf54384883ea33b1240fc1db4f52e190823a4e2b334bc3e8bfca96a` (`target/w6-websockets-source-66012a0.{py,log,exit}`). Without executing downloader or dependency code, PyInstaller archive inspection and Python 3.14.10 compilation compared the pinned gallery-dl archive's 52 embedded websockets Python modules with that source: **52 matched, 0 different, 0 missing** on bytecode, names, variables, flags and nested constants; exception tables and line metadata were not compared (`target/w6-websockets-modules-66012a0.{py,log,exit}`). The embedded licence matched the source licence after line-ending normalization (`target/w6-websockets-compare-66012a0.{py,log,exit}`).

The PyPI 17.1 CPython 3.14 Windows x64 wheel SHA-256 `2a4dc6ef83f4559e0d05f313a375cb38f63c986096a9da99fe94fdd779d313e5` was separately verified against PyPI metadata. Its `speedups.cp314-win_amd64.pyd`, distribution METADATA and licence matched the three corresponding embedded files byte-for-byte (`target/w6-websockets-wheel-66012a0.{py,log,exit}`). This supports a scoped source/wheel correspondence for this one dependency, not the complete executable or its runtime behavior. The earlier discrepancy with the gallery-dl source snapshot's `websockets==16.0` build requirement remains unexplained.

Windows packaging fix `8e9b2cd6126ccdc261e66151dec30f6bd3cdc2f6` includes the verified upstream websockets licence alongside the project licence for the pinned gallery-dl Release asset. Portable Full packaging requires and hash-verifies both texts for that exact binary, refuses a stale licence-to-release pin binding before replacing output, and for other binary versions still requires an operator-provided project licence without claiming the dependency set is complete. A narrow `.gitattributes` rule preserves the websockets licence's LF bytes.

Targeted packaging/security tests **PASS 39/39** (`target/w6-websockets-package-final3-66012a0.{log,exit}`); the isolated Full portable package passed component identity checks (`target/w6-portable-websockets-final-66012a0.{log,exit}`, `target/w6-portable-websockets-verify-final-66012a0.{py,log,exit}`). The committed Release workflow fragment passed locally (`target/w6-gallery-run-committed-8e9b2cd.{ps1,log,exit}`; fragment identity `target/w6-gallery-fragment-check-8e9b2cd.{mjs,log,exit}`), and a simulated `core.autocrlf=true` checkout preserved both licence hashes and ran the helper (`target/w6-gallery-checkout-66012a0.{log,exit}`). Tests ran on `66012a0` with the fix uncommitted, then it was committed unchanged as `8e9b2cd`; the fragment was rerun after commit. `npm run arch:check` and `git diff --check` passed. Actual downloader acceptance and GitHub Actions were not run. The remaining bundled dependencies, complete corresponding source, release manifest and redistribution review remain open; `CROSS_PLATFORM_REVIEW_REQUIRED`.

## Windows gallery-dl static inventory and project licence — 2026-11-18

At source `57a83f032c00cd88272fef33b841cf26ea7883cc`, the pinned gallery-dl executable SHA-256 `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` was inspected as data using PyInstaller archive reader 6.22.3; the downloader was not executed. Its archive contains `websockets-17.1.dist-info/METADATA` declaring version 17.1 and websockets modules, while the upstream release's identified source snapshot lists `websockets==16.0` in `requirements/windows`. **Static dependency declaration mismatch:** this does not prove which code executes or why the versions differ, but the source requirements cannot be treated as a verified binary inventory. The archive reader also listed Python and other native/runtime components. Evidence `target/w6-gallery-static-inventory-57a83f0.{log,exit}`, `target/w6-gallery-metadata-57a83f0.{py,log,exit}`. The binary metadata SHA-256 is `ce1b2b1cde9c2681d3d0328a724193475436be238fd47d8bd866a8195b35563f`; its embedded websockets licence text SHA-256 is `0f44514998aca209d3482d10204a8adf2aa4296ff157a36a5c0922f2280632d3`. `CROSS_PLATFORM_REVIEW_REQUIRED` for build provenance, complete component source and terms.

Packaging fix `39308605a43971ebdaeee0d7bf9fe43b305aee7c` vendors the gallery-dl project licence from the release-referenced source, binds it to the pinned asset and licence hashes, and makes the Windows Full release workflow verify and copy it. Portable Full assembly requires a nonempty `LICENSE` beside its gallery executable before replacing an existing output; portable input provenance and correctness are not automatically verified. Core remains gallery-free. A narrowly scoped `.gitattributes` rule keeps the vendored licence LF so Windows `core.autocrlf=true` checkouts preserve the verified bytes.

Targeted Node tests **PASS 38/38** (`target/w6-gallery-licence-tests-57a83f0.{log,exit}`), including mismatched pins, binary/licence tampering and portable preflight preservation. The exact Full workflow gallery fragment **PASS scoped** with the pinned executable and verified licence (`target/w6-gallery-step-57a83f0.{ps1,log,exit}`); isolated Full portable assembly **PASS scoped** with both downloader binaries and their supplied texts verified by hash (`target/w6-portable-gallery-57a83f0.{log,exit}`, `target/w6-portable-gallery-verify-57a83f0.{py,log,exit}`). Simulated Git checkout with `core.autocrlf=true` retained licence SHA-256 `8177f97513213526df2cf6184d8ff986c675afb514d4e68a404010521b880643`, and the checked-out helper succeeded (`target/w6-gallery-checkout-57a83f0.{log,exit}`). Tests and assembly ran on `57a83f0` with the relevant diff subsequently committed unchanged as `3930860`; the checkout check used the staged files. `npm run arch:check` and `git diff --check` passed. No actual downloader was run and the hosted workflow remains NOT_RUN.

This supplies the gallery-dl project licence only. Its embedded libraries, the version discrepancy, complete corresponding source, release manifest and redistribution obligations remain open. Prior packages are unchanged; shared packaging, `.gitattributes` and source register require cross-platform review.

## Windows aria2 distribution companion files — 2026-11-17

The locally retained upstream Windows archive was rechecked against SHA-256 `67d015301eef0b612191212d564c5bb0a14b5b9c4796b76454276a4d28d9b288`. Its `aria2c.exe` is accompanied by `AUTHORS`, `COPYING`, `LICENSE.OpenSSL`, `README.mingw`, `README.html`, `NEWS` and `ChangeLog`; the earlier Full release workflow copied only the executable. The archive's README identifies statically linked libraries and versions, but does not close the corresponding-source or licence review.

Windows release fix `56cddcf72f7be9286acef0df1032dd942a719705` copies all seven nonempty companion files from the verified extracted archive into the Full bundle and fails the job if they cannot be included. Portable packaging fix `92b5611f467707a86b0e7c85fb72a00ac6e9341a` requires those files when an aria2 executable is present, before replacing any previous output; aria2 remains optional. These shared workflow/script changes require `CROSS_PLATFORM_REVIEW_REQUIRED`.

The targeted Node packaging/security tests **PASS 36/36** (`target/w6-aria-portable-final-56cddcf.{log,exit}`), including missing/empty companions and preservation of a previous output. The exact release workflow fragment, run locally using extracted pinned-archive bytes and the final helper, **PASS** with output file identities verified (`target/w6-aria-step-final-56cddcf.{log,exit}`, `target/w6-aria-step-verify-final-56cddcf.{py,log,exit}`). A separate isolated Full portable assembly using the final code and new Desktop **PASS scoped**, with executable and seven companions byte-matched (`target/w6-portable-aria-final-56cddcf.{log,exit}`, `target/w6-portable-aria-verify-56cddcf.{py,log,exit}`). The tests ran on `56cddcf` with the portable changes uncommitted, then those changes were committed unchanged as `92b5611`; the final release fragment was rerun with the committed helper. `npm run arch:check` and `git diff --check` passed. The actual downloader binary was not executed, and GitHub Actions was not run.

This closes only the identified aria2 companion-file omission for the current release workflow and portable packaging. Earlier packages were not retroactively changed; gallery-dl, worker dependencies, complete corresponding source, release manifest and redistribution review remain open. The historical legacy workflow and hosted artifacts have not been revalidated.

## Windows downloader source snapshots — 2026-11-17

On `bf7b05d7ee677aa3796d4b574eb42bf1567e742c`, the Codeberg gallery-dl archive request timed out (exit 1, `target/w6-source-archives-bf7b05d.{py,log,exit}`); the combined attempt did not reach aria2. An independent request retrieved the upstream aria2 source snapshot for commit `02f2d0d8472b3c38c29b4dba8c75ebd5fdd2899a`: **PASS scoped**, archive SHA-256 `fc3475683aa88df5764f6c5faa5f45b9d3dd23cca7a02de2ba6eee868edf5ce9`, 1,470 tar members and top-level COPYING matching the previously recorded licence text (`target/w6-aria-source-bf7b05d.{py,log,exit}`).

At `a5b124b5557857d43e0f9dc3cde57bb5c797cfcf` (comment and baseline changes only), the same gallery-dl commit `61070f02d387494ac3a2bb5bfd8345aba85b1303` was found on the upstream GitHub mirror and its archive retrieved: **PASS scoped**, observed SHA-256 `4dda5c5ead73951af9c9e247df52a66da4d5c850f4efbbdc73c1b5085a84605e`, 842 members and matching top-level LICENSE (`target/w6-gallery-mirror-source-bf7b05d.{py,log,exit}`). Archive hashes identify these retrieved bytes, not a promise that generated archives are reproducible. These snapshots do not establish the entire corresponding source, build dependencies or redistribution compliance; the release gate remains open and the shared source register requires `CROSS_PLATFORM_REVIEW_REQUIRED`. No real downloader was run.

Commit `a5b124b` also corrected the Windows containment module's stale comment: zero active processes does not mint drain evidence, and registration exists while production drain/recovery remain gated. It changed comments only and resealed the one corresponding architecture-baseline hash; `cargo fmt --all -- --check`, `npm run arch:check` and `git diff --check` passed.

## Windows pinned downloader upstream references — 2026-11-17

At test HEAD `4dabeb747a9dcb9ab30230ea3830899b3b260438`, tracked tree clean, read-only upstream metadata identified the gallery-dl 2026.09.20 release's stated source commit and the aria2 1.37.0 annotated tag's target commit; the two named upstream GPL licence files were retrieved and had identical SHA-256 `8177f97513213526df2cf6184d8ff986c675afb514d4e68a404010521b880643`. The gallery release metadata's Windows binary digest matched the existing repository pin. Exact URLs, commit IDs and remaining obligations are in the [external source register](../references/external-sources.md). Evidence: `target/w6-upstream-sources-4dabeb7.py`, `.log`, `.exit` and retained metadata/licence responses in `target/w6-upstream-sources-4dabeb7/`. This is a source-reference and text retrieval, not proof that all corresponding sources or bundled dependencies are available, or that distribution obligations are met. No downloader binary was executed. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared register and release-source/terms follow-up.

## Windows refreshed Full configuration cancellation — 2026-11-17

Test HEAD `da39d32770677d734cb58d1ec419919f660414c9`, tracked tree clean; Desktop build source `79447104ca8ac92e5a8424e41040534528db2322`, executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. On Windows build 29708.1003 with WebView2 153.0.4234.48 / EdgeDriver 153.0.4234.46, the controlled-worker Full ReFS fixture used a synthetic active workspace holder (test executable SHA-256 `67f1c8c29056d2bbcdeb9e45c6602b47634b92b20bbfff19d7d32f4a1b79f80d`). Actual Desktop IPC **PASS scoped 1/1**: waiting apply cancelled with a cancellation outcome, configuration file and exported runtime settings stayed unchanged, admission reopened, and a later apply succeeded. Evidence `target/w3-cancel-7944710/{runner.log,runner.exit,app.sha256}` and `target/w3-cancel-7944710.ps1`. This synthetic holder does not prove cancellation against a real contained production writer; W1 remains its prerequisite.

## Windows refreshed Full F3 root replacement — 2026-11-17

Test HEAD `371ba4376d41900bae821f0cccce8ed36935335d`, tracked tree clean; Desktop production build source `79447104ca8ac92e5a8424e41040534528db2322` (intervening commits are test/documentation only), executable SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`. On Windows build 29708.1003 with WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, the controlled-worker Full fixture on ReFS ran the real Desktop IPC via WebDriver: replacing the synthetic archive root after preview made acceptance report a stale preview; re-preview remained possible. **PASS scoped 1/1**. Evidence: `target/w7-root-7944710/{runner.log,runner.exit,app.sha256}` and `target/w7-root-7944710.ps1`. This does not cover the gated contained-writer lifecycle or actual browser registration.

## Windows guarded destination conflicts — 2026-11-17

At source `47e8335e110eaf8e93d197558818faa6fdaa5605` with the new test subsequently committed unchanged as `3e9155ac9ecaf21cda187b0db63e3e88b0e9b68e`, actual `ExecutionReservation` and `WindowsNoReplaceMover` were exercised against pre-existing destination files and directories. **PASS 1/1 on NTFS and 1/1 on ReFS**, each covering both target kinds: the mover returned `DestinationExists`, preserved the original contents and left the staging source intact while the destination pin was held. Test executable SHA-256 `67f1c8c29056d2bbcdeb9e45c6602b47634b92b20bbfff19d7d32f4a1b79f80d`; evidence `target/w2-guarded-conflict-{ntfs,refs}-47e8335.{log,exit}`. This scoped check does not resolve W2's future 8.3 alias-creation race or establish Desktop lifecycle acceptance.

## Windows guarded publication and refreshed Full artifact — 2026-11-17

At source `5d50e6b` plus the new test subsequently committed unchanged as `79447104ca8ac92e5a8424e41040534528db2322`, the native Windows test acquired the real `ExecutionReservation` for a synthetic destination, published a staging directory through `WindowsNoReplaceMover`, verified its contents, observed destination contention while reserved, then reacquired after dropping the reservation. **PASS 1/1 on NTFS and 1/1 on ReFS**; test executable SHA-256 `ebeb482b5556c281789eee80beda386347a676c8088858b31fe2df15ca896fb0`. Evidence: `target/w2-reservation-publication-{ntfs,refs}-5d50e6b.{log,exit}`. This tests the reservation/mover combination, not Desktop's gated production writer or the 8.3 alias race.

At clean tracked source `79447104ca8ac92e5a8424e41040534528db2322`, with the two earlier untracked user directories retained, `cargo build --release --locked --offline -p xarchive-desktop --features tauri/custom-protocol` **PASS**. Desktop SHA-256 `0e4e071d662d611613ed268b7f1d2c7685babc92cdd275e166c1d1292c49400d`; build evidence `target/w6-desktop-release-7944710.{log,exit,sha256}`. The existing frontend output was reused; tracked frontend sources did not change since the prior `a79566b` build.

An isolated portable Full fixture on ReFS was assembled using the new Desktop and existing controlled worker `65bfce70fdee1b727c0d394454b0f2f6d73d44374c39f424735508c88b3e6615`, Native Host `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`, pinned gallery-dl `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a` and pinned aria2 executable `be2099c214f63a3cb4954b09a0becd6e2e34660b886d4c898d260febfe9d70c2`. Fixture Extension and packaging scripts matched tracked inputs, and packaged component hashes were checked. Assembly **PASS scoped** (`target/w6-full-7944710.{log,exit}`). Isolated packaged Desktop/Native Host synthetic pairing **PASS**: untrusted origin rejected, bootstrap and WebSocket authentication succeeded, ticket replay rejected (`target/w6-full-native-7944710.{log,exit}`). This is not actual browser registration.

Using the same fixture after pairing, actual WebDriver-driven F3 valid/missing selection and import plus driver-directed JSON/YAML export **PASS 1/1** on Windows 29708.1003, WebView2 153.0.4234.48 / EdgeDriver 153.0.4234.46. Evidence: `target/w6-full-gui-7944710/{runner.log,runner.exit,app.sha256,archive-records.json,archive-records.yaml}` and `target/w6-full-gui-7944710.ps1`. No real downloader or Telegram acceptance was performed. W1 drain proof and W2 future short-name alias creation remain open and production publication stays fail-closed.

## Windows destination path lifetime — 2026-11-17

At source `ec54a44d8ec68041396772792d7957dd6c07d6e3` with the new regression test uncommitted, an NTFS targeted run **FAILed**: the destination parent could be renamed while its destination guard remained held (`target/w2-parent-pin-before-f90335f.{log,exit}`). That changes the path by which the same directory is reached while the old lock key remains held.

Windows adapter fix `2bd6168104a74af31ecedda97b5dc06e373ff86e` retains the pinned existing ancestor and, when present, destination leaf through the guard lifetime. It releases pins after an explicit guard release and on failed multi-scope acquisition. The final code and test contents were run as the uncommitted diff on `ec54a44` and then committed unchanged: native Windows NTFS module **PASS 15/15, 3 ignored**; targeted ReFS destination regressions **PASS 3/3, 3 ignored**. Test executable SHA-256 `f91b4b0f278a054859cba852224fc405f76c9b544cf52b36cba1e43cdb0324d5`; evidence `target/w2-parent-pin-final-{module,refs}.{log,exit}`. `cargo fmt --all -- --check`, `npm run arch:check` after the one-file baseline reseal, and `git diff --check` passed. An initial `npm run check-architecture` invocation failed because that script name does not exist; the correct script is `arch:check`.

This closes the observed parent/leaf rename gap only. The existing future 8.3 alias-creation race remains **FAIL**, and the three 8.3-dependent tests were not executed in the module run. Publication remains fail-closed. The shared architecture baseline reseal needs `CROSS_PLATFORM_REVIEW_REQUIRED`; Desktop artifacts and integrated lifecycle checks need a fresh build before their results can bind to this source.

## Windows portable fixture installer applicability — 2026-11-17

Source `f90335fca7d7fe9fcf9e44a37ffdcaebf9d98a58` on `codex/windows-validation-4ae46ed3`, tracked tree clean with the two previously retained untracked directories. The tested `Full-visual-6af046f` artifact is a portable `full` directory (`package-manifest.json`), with Desktop SHA-256 `cab55a9241178f33fa2fce3c6e5ef4c9b13e8dbcefdb28300fc556cd75f993da`. `desktop/src-tauri/tauri.conf.json` sets `bundle.active` to `false`; the inspected Windows release workflow defines executable and 7z package outputs.

Installer install/upgrade/uninstall checks are **NOT_APPLICABLE to this portable fixture**, because no installer is part of this fixture. This classification does not validate installation, upgrade, removal, signing, a hosted release run or any future installer. If an installer is added to the intended distribution, Windows Platform Owner must verify those behaviors on its exact source and artifact. The portable Full package's browser registration, license/source review, and other W6 acceptance remain open.

## Windows F3 invalid-input GUI recovery — 2026-11-17

Test source `a25db27577124b30bd45bb0c3547439706ff85ec`, tracked tree clean, controlled-worker Full fixture on ReFS with private Named Pipe; Desktop SHA-256 `cab55a9241178f33fa2fce3c6e5ef4c9b13e8dbcefdb28300fc556cd75f993da`, Windows build 29708.1003, WebView2 153.0.4234.48 / EdgeDriver 153.0.4234.46. Actual GUI retry **PASS scoped 1/1** (overall sequence intermittent): after a valid selectable preview, editing to malformed input cleared the old table and disabled acceptance; preview showed failure with acceptance still disabled; restoring valid input and previewing again showed a selectable record. No acceptance was requested in this check. Evidence: `target/w7-invalid-a25db27-retry.e2e.mjs`, `target/w7-invalid-a25db27-retry.ps1`, `target/w7-invalid-a25db27-retry/{runner.log,runner.exit,app.sha256}`.

The first run **FAILed** by timing out at the final valid preview, after its checks for error and disabled acceptance had passed (`target/w7-invalid-a25db27.e2e.mjs`, `target/w7-invalid-a25db27/{runner.log,runner.exit}`). The retry used a new fixture and waited until the preview button was enabled after the error before editing and clicking; it passed. This suggests a test timing issue but does not independently prove the cause of the first timeout. Other F3 failure cases remain open.

## Windows default-location record exports — 2026-11-17

Test source `0b01eab9c358783fda3526b47e85aa2346f3099b`, tracked tree clean; pre-existing untracked directories retained. Windows build 29708.1003, WebView2 153.0.4234.48 / EdgeDriver 153.0.4234.46, ordinary user, isolated controlled-worker Full on ReFS with private Named Pipe. Desktop executable SHA-256 `cab55a9241178f33fa2fce3c6e5ef4c9b13e8dbcefdb28300fc556cd75f993da` (production build source `a79566b`; subsequent changes do not alter that executable).

**PASS scoped 1/1:** actual WebView2 record-import GUI imported the synthetic selected record and exported JSON and YAML through the real buttons, without `Browser.setDownloadBehavior` or replacing the browser download implementation. Both files appeared in the current user's configured system Downloads (`E:\Shiraishi\Downloads`); JSON identity and YAML re-preview were verified and YAML was read again after a delay. The two target filenames were checked absent before running; saved evidence copies matched the generated files by SHA-256 before the two generated files were deleted from Downloads. JSON SHA-256 `5652caef69dca981eb6b0386c7a6972c52979e33b0ca694c9adae8f883e9270a`; YAML `959f2aa1f90601c9c7de661581142746a758f6722ad646ebe2f83129581b568c`. Evidence: `target/w7-system-download-0b01eab/{run.ps1,system-download.e2e.mjs,runner.log,runner.exit,app.sha256,archive-records.json,archive-records.yaml}`. No save-dialog interaction was required or verified in this configuration. Other F3 failures and the broader GUI, window-size and DPI matrix remain open.

## Windows worker runtime inventory — 2026-11-17

Test HEAD `347c142`, tracked tree clean; the worker was built at `0711a74` and packaged in the controlled-path `Full-w3-cancel` fixture. Its PyInstaller analysis listed 11 binary entries; all 11 packaged files matched their recorded build-source bytes, and `base_library.zip` was separately inventoried (`target/w6-worker-ci-sim-0711a74/license-inventory.json`, `.py`, `.log`, `.exit`). Windows file metadata identifies Python 3.12.14, OpenSSL `libcrypto` 3.5.8 and Microsoft `VCRUNTIME140` 14.44.35211.0; hashes and the exact observation are in `package-license-observation.log`, `.exit`. The fixture contained only the project `LICENSE` among filenames beginning LICENSE/COPYING, and its `THIRD_PARTY_NOTICES.md` had no component-specific Python Software Foundation, OpenSSL/libcrypto or VCRUNTIME matches. This is an inventory and text-scan finding, not a legal conclusion or a complete transitive dependency audit. Source, terms, licence inclusion and redistribution review remain open; the result does not generalize to other worker builds. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared external-source register and release notice follow-up.

## Windows keeper late-trigger control — 2026-11-17

Test source `d7a967b`, tracked tree clean. In the same disposable actual-Desktop-keeper setup, the owner was killed, the synthetic PowerShell writer was allowed to exit naturally and its retained handle signaled while the keeper stayed live. The successor reopened the recorded named Job, assigned a suspended synthetic CPU burner and its descendant, and applied the end-of-job time limit. The Job and all retained observed handles were signaled: **PASS scoped 1/1** (`target/w1-job-time-probe/desktop-keeper-late-trigger/probe.log`, `.exit`, `probe.py`). This demonstrates a trigger for a dormant keeper on this machine; it does not establish correctness for a previously detached process still terminating, production Rust interop, or publication authorization. W1 remains fail-closed.

## Windows actual keeper end-of-time experiment — 2026-11-17

Test HEAD `bb787f2`, branch `codex/windows-validation-4ae46ed3`, tracked tree clean. The existing Windows containment test owner (`xarchive_workflow-ac421d0704fc57ba.exe`, SHA-256 `9691e05f8ad5c51ffd2b25743c58cd08bcbc4a09caed2476e4257ac82a3d6f74`, built at `6630312`) launched the actual `a79566b` Release Desktop keeper (SHA-256 `cab55a9241178f33fa2fce3c6e5ef4c9b13e8dbcefdb28300fc556cd75f993da`) and a synthetic PowerShell writer in its recorded named Job. An external Python/Win32 probe retained handles to the observed Job members, killed the owner, reopened the recorded Job, assigned a suspended synthetic CPU burner, then applied the end-of-job time limit. In the first scoped run and six runs with an additional burner descendant, the Job signal and all retained observed process handles were signaled; the owner and observed members had been live before the owner was killed (`target/w1-job-time-probe/desktop-keeper`, `desktop-keeper-descendant-retry`, `desktop-keeper-repeat-1` through `-5`, each with `probe.log` and `.exit`). The probe uses no production wait bridge and does not mint publication evidence.

The first descendant harness attempt assumed exactly two initial members and **FAILed as a harness assertion** when the system console host was also in the Job (`desktop-keeper-descendant/probe.log`, `.exit`). The revised harness retained every reported member and required at least the real keeper and writer; the console host appeared in some runs. The result illustrates why a hard-coded process count is unsafe. Test processes were closed. This does not prove completeness of a member snapshot or coverage of a previously detached process still terminating; safe Rust wait interop, time-limit/trigger failure handling, actual successor adapter and the historical-descendant question remain open. Publication and startup recovery stay fail-closed.

## Windows Job end-of-time signal feasibility — 2026-11-17

Test source `a455cec`, native Windows 10.0.29704, isolated experimental helpers under `target/w1-job-time-probe`; no production code or dependency changed. Microsoft documents that a Job Object becomes signaled when its processes are terminated after its end-of-job time limit, and that the default limit action terminates processes and prevents new assignments until reset ([Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects), [end-of-job time action](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_end_of_job_time_information)). The time limit measures user CPU time and is checked periodically; a waiting/sleeping job needs an owned in-job CPU consumer to trigger it, and timeout must remain a refusal.

A disposable native parent/child process tree retained both process handles. With a short job user-time limit, six runs observed the Job handle and both retained process handles signaled (`probe.log` / `.exit`, `repeat-1` through `repeat-5`). In six separate runs, a synthetic owner was killed, a successor reopened the same named Job while both descendants survived, then applied the limit; the Job and both retained process handles signaled (`successor` and `successor-1` through `successor-5`). Six further runs requested child termination immediately before applying the limit: the child handle was initially unsignaled and was signaled when the Job wait completed (`prior-termination` and `prior-termination-1` through `-5`). A deliberately long limit produced `WAIT_TIMEOUT` on the Job and both live process handles (`timeout`). Synthetic executable SHA-256 `156a816e58e867837a4ff9c1fe41095f18ab4a4dbd37bcd3505bab3da6ad0efd`; scripts, logs, per-run exit codes and fixture roots are in the directory above. Test processes were closed.

These observations are **experimental and bounded**, not writer-drain acceptance: the probes do not drive the actual Desktop keeper or production publication/recovery, and they do not establish whether an earlier descendant already removed from Job membership but still terminating is covered by the later time-limit signal. The repository forbids unsafe Rust; the existing `firehazard` Job handle does not directly expose the standard `AsHandle` required by the reviewed safe `win-sync-sys` 0.1.1 wait API (limited source inspection and checksum at `target/w1-wait-api-audit`, not a full dependency audit). A safe wait bridge, bounded trigger, actual keeper/successor ownership, crash/timeout handling and the historical-descendant question must be resolved before minting evidence. W1 remains fail-closed.

## Windows Full configuration maintenance classification — 2026-11-17

Test HEAD `0765a23`, branch `codex/windows-validation-4ae46ed3`, tracked tree clean. Three distinct ReFS Full fixtures contained the `a79566b` Release Desktop (SHA-256 `cab55a9241178f33fa2fce3c6e5ef4c9b13e8dbcefdb28300fc556cd75f993da`), controlled-PATH worker built at `0711a74`, and pinned downloader inputs. On Windows 10.0.29704 / WebView2 153.0.4234.48 / EdgeDriver 153.0.4234.46, actual Desktop IPC tests **PASS 3/3 scoped**: cancellation and timeout while a synthetic active workspace holder blocked maintenance reported their distinct outcomes, preserved runtime/disk state and reopened admission; an injected lock-path I/O failure was distinguished from cancellation/timeout, preserved state and allowed a later apply. Evidence: `target/w3-cancel-0765a23`, `target/w3-timeout-0765a23`, `target/w3-io-0765a23-retry` (`runner.log`, `.exit`, `app.sha256`); runner scripts and assembly logs under `target/w6-worker-ci-sim-0711a74`. The holder executable was built at `fa4269e`, SHA-256 `3c10b85359b8b2a08eca8cd9c3d9c1277a6ea62ed2d6caa21f768d3fd9cb159c`.

The first I/O case stopped before any product assertion because its disposable-fixture safety marker was missing: a PowerShell positional argument error created a stray file named `w3-lock-failure-fixture` instead of the marker; the stray file was identified by its contents and removed (`target/w3-io-0765a23/runner.log`, `.exit`); after the marker was explicitly created and checked, the retry passed. This is a fixture preparation failure, not a product result. The holder is synthetic: cancellation against an actual contained production writer and the W1/W2-gated lifecycle remain open.

## Windows Full F3 root replacement on ReFS — 2026-11-17

Test HEAD `31ca098`, tracked tree clean, branch `codex/windows-validation-4ae46ed3`. On Windows 10.0.29704, WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, actual Desktop IPC driven by WebDriver **PASS 1/1** in the isolated controlled-worker/pinned-downloader Full fixture on ReFS. After preview, replacing the synthetic archive root made acceptance report a stale preview; re-preview still considered the record acceptable. The Desktop binary was built at `a79566b`, SHA-256 `cab55a9241178f33fa2fce3c6e5ef4c9b13e8dbcefdb28300fc556cd75f993da`; evidence `target/w7-root-31ca098/runner.log`, `.exit`, `app.sha256`, invocation `target/w6-release-a79566b/run-f3-root-31ca098.ps1`. Unlike the earlier debug-artifact WebDriver startup failures, this scoped Release Full attempt completed; the differing artifacts and fixtures do not establish their cause. This test invokes IPC through the WebView and does not establish the broader GUI or production writer lifecycle.

## Windows Full F3 cancellation with controlled worker — 2026-11-17

Test source `fa4269e`, branch `codex/windows-validation-4ae46ed3`, tracked tree clean. On Windows 10.0.29704 with WebView2 153.0.4234.48 / EdgeDriver 153.0.4234.46, the `a79566b` Release Desktop (SHA-256 `cab55a9241178f33fa2fce3c6e5ef4c9b13e8dbcefdb28300fc556cd75f993da`) ran in the isolated ReFS Full fixture containing the controlled-PATH worker and pinned downloader inputs. Actual GUI/IPC cancellation against a synthetic active workspace holder **PASS 1/1**: the UI reported cancellation, the record remained acceptable on re-preview, and acceptance succeeded after the holder exited. The holder was built from this test source, SHA-256 `3c10b85359b8b2a08eca8cd9c3d9c1277a6ea62ed2d6caa21f768d3fd9cb159c` (`target/w7-holder-fa4269e.log`, `.exit`). GUI evidence `target/w7-cancel-fa4269e/runner.log`, `.exit`, `app.sha256`; invocation `target/w6-release-a79566b/run-f3-cancel-fa4269e.ps1`. This is not W3 cancellation during an actual contained production writer, which still depends on W1.

## Windows Job member-query candidate review — 2026-11-17

At `2f488fb` and subsequent documentation-only revisions, a bounded source inspection considered `win32job` 2.1.0 as a replacement for the existing problematic PID-list query. The downloaded crate matched the crates.io checksum `21b25acac91041257d64f5625690b626133e0ca9e4e17814552b5803b1a82f7c` and declares MIT OR Apache-2.0 (`target/w1-win32job-audit/registry-check.log`). Its public `Job` constructor creates an anonymous Job Object; this version exposes no safe constructor for reopening the existing named object or adopting its handle. Its query also has a fixed-size list and reports an error above its capacity. Source inspected: `target/w1-win32job-audit/win32job-2.1.0/src/{job,query}.rs`. This is a limited API/source review, not a full dependency or security audit. A PID snapshot can still miss short-lived descendants, so this candidate alone neither proves complete drain nor provides successor ownership. No dependency or production change was made; W1 remains fail-closed.

## Windows worker installation failure injection — 2026-11-17

Test HEAD `0c4ac6f`, tracked tree clean. The two actual workflow steps were extracted into disposable directories and run with a synthetic `python.exe` that records calls and exits 37 at a chosen installation stage. For each of pip upgrade, worker installation and PyInstaller installation, both the release and dedicated worker step exited nonzero immediately; their recorded call sequences ended at the injected failure: **PASS 6/6 scoped**. Evidence: `target/w6-pip-fail-0c4ac6f/{release,worker}-{upgrade,project,pyinstaller}/output.log`, `result.exit`, `calls.txt`, and the fixture source/extractor. No real package install, hosted workflow, release or push occurred.

## Windows worker build failure gate — 2026-11-17

Production revision `b93a101`, branch `codex/windows-validation-4ae46ed3`, tracked tree clean at exact-source checks. The Windows main release step previously continued after a failed PyInstaller command and could replace its exit code with a later successful worker smoke check. Both Windows worker workflows now explicitly stop on failed pip upgrade, project install, PyInstaller install or worker build; this does not change the accepted worker protocol. Shared release test/integration is `CROSS_PLATFORM_REVIEW_REQUIRED`.

The actual workflow steps were extracted into disposable directories and driven by a synthetic `python.exe` which creates a runnable partial worker and then returns exit 37 for PyInstaller. A control with only the new build guard removed returned **0** and reached the end; the fixed release and dedicated worker steps each returned **1** at the guard with exit 37 (`target/w6-fail-gate-b93a101/{release-before,release-after,worker-after}/output.log` and `result.exit`; fixture and extractor in the same directory). This proves the injected build-failure case, not a hosted GitHub Actions run. At exact revision, release-security tests **PASS 5/5** (`target/w6-workflow-b93a101.log`, `.exit`), architecture check and diff check passed. No workflow dispatch or push occurred.

## Windows worker release workflow hardening — 2026-11-17

Revisions `a0212d4` and `0711a74` on `codex/windows-validation-4ae46ed3` fix two Windows packaging issues. The main release workflow installed unpinned PyInstaller while the dedicated worker workflow already used `6.22.3`; the new consistency test reproduced the unpinned failure (`target/w6-pyinstaller-pin-before.log`, `.exit`) and both now use the established pin. Both workflows select the first Python application and restrict the build process PATH to that Python directory plus Windows system directories before invoking PyInstaller. This limits inherited PATH contamination, but does not certify every DLL origin or licence. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared test and release integration.

At exact `0711a74`, release-security tests **PASS 4/4** (`target/w6-workflow-0711a74.log`, `.exit`). On this Windows machine, a local simulation of the workflow's Python selection and PATH setup built the unchanged worker **PASS**, SHA-256 `65bfce70fdee1b727c0d394454b0f2f6d73d44374c39f424735508c88b3e6615`; analysis found 11 Python-runtime binaries and no DLL from the unrelated tool directory; help and synthetic v2 fake-gallery download passed (`target/w6-worker-ci-sim-0711a74/build.log`, `.exit`, `origins.log`, `.exit`, `assertions.log`, `.exit`, individual probe logs). An earlier uncommitted simulation using all results from `Get-Command` failed before build because multiple Python installations yielded an array; selecting the first fixed it, and the retry built and passed the same scoped probe. These are local runs, not a GitHub Actions run, cross-machine portability proof, real downloader acceptance, or licence closure. No push or workflow dispatch performed.

## Windows worker collection provenance and controlled build — 2026-11-17

Source `a8629a5`, branch `codex/windows-validation-4ae46ed3`, tracked tree clean. Inspection of the `d1854c1` PyInstaller `Analysis-00.toc` found 43 Windows runtime DLLs collected from the installed Codex Primary Runtime `dependencies/native/libheif/libheif/bin`, in addition to 11 binaries from the Python runtime. Their packaged bytes matched their recorded source files (`target/w6-worker-d1854c1/runtime-origin-e008d7c.json`, `.log`, `.exit`). This is a concrete provenance and redistribution-review gap in that previously reused worker, not by itself a runtime product failure. Existing packages using it retain their historical scope.

A fresh worker was built from the current unchanged Sidecar source with PyInstaller 6.22.3 / Python 3.12.14 and PATH restricted to Windows system directories: **PASS**, SHA-256 `d4315bc571aec38f1114056fef34db2a76b9f43e5ea1e3f30e807ffd23d68b60` (`target/w6-worker-clean-e008d7c/build.log`, `.exit`). Its analysis TOC records 11 Python-runtime binaries and none from the other tool directory (`origin-comparison.py`, `.json`, `.log`, `.exit`). Help and synthetic v2 download using the previously recorded fake gallery executable passed for both the built worker and its copy in the Full package (`assertions.log`, `.exit`, `packaged-assertions.log`, `.exit`, with individual help/probe logs and exits). The controlled PATH run removed the observed collection, but it alone does not establish the unique cause or prove portability to other Windows environments.

A new isolated Full fixture at `target/w6-pinned-clean-a8629a5-fixture/dist-portable/Full` combined that worker with the `a79566b` Release Desktop, unchanged Native Host, current notices, and gallery-dl/aria2 binaries matching the recorded release pins: assembly and component/notice identity **PASS scoped** (`full.log`, `.exit`, `verify-full.py`, `.log`, `.exit`). It contains none of the 43 previously identified DLL names. Packaged Desktop/Native Host synthetic pairing **PASS scoped** (`full-native.log`, `.exit`). License and redistribution completeness, cross-machine runtime portability, browser registration, installer, GUI matrix and real downloader/Telegram acceptance remain open. `CROSS_PLATFORM_REVIEW_REQUIRED`: review the worker build environment and collected dependency/notice inventory before release; the observed difference has not been turned into a shared build-policy change.

## Windows release-source register reconciliation — 2026-11-17

Source `16b0d07` plus this documentation change: the external sources register and third-party notice previously said the Windows gallery-dl and aria2 versions and hashes were unpinned, contradicting the committed release asset manifest. They now point to the existing Windows pins and the local byte checks recorded below, while keeping corresponding source, licence text and redistribution review open. This is a factual documentation correction, not a license-completeness finding. `CROSS_PLATFORM_REVIEW_REQUIRED` for the shared notice and source register; reassemble any package intended to contain the updated notice.

## Windows Full assembly with pinned downloader inputs — 2026-11-17

Test source `e008d7c`, branch `codex/windows-validation-4ae46ed3`, tracked tree clean. The Release Desktop from `a79566b` (SHA-256 `cab55a9241178f33fa2fce3c6e5ef4c9b13e8dbcefdb28300fc556cd75f993da`), the previously built Native Host and worker, current committed packaging inputs, and locally retained downloader binaries were assembled into the new isolated `target/w6-pinned-e008d7c-fixture/dist-portable/Full`: **PASS scoped** (`target/w6-release-a79566b/full-pinned-e008d7c.log`, `.exit`). The Full manifest and required components/notices were checked, as were component hashes (`verify-pinned.py`, `.log`, `.exit` in that evidence directory). The gallery-dl executable matches the repository release pin `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a`; the retained aria2 archive matches its pin `67d015301eef0b612191212d564c5bb0a14b5b9c4796b76454276a4d28d9b288` and the packaged executable matches that archive member (SHA-256 `be2099c214f63a3cb4954b09a0becd6e2e34660b886d4c898d260febfe9d70c2`). These are local byte-to-pin checks using the previously documented upstream pin provenance, not a fresh upstream download or real downloader acceptance. The earlier Full fixture's different gallery-dl hash remains recorded above and is not promoted as pinned.

Actual packaged Desktop/Native Host with a private Named Pipe and synthetic client **PASS scoped**: wrong origin rejected, bootstrap and WebSocket authentication passed, replay rejected (`target/w6-release-a79566b/full-pinned-native.log`, `.exit`). The worker and native host are reused from their unchanged previously built sources. License and redistribution completeness, browser registration, installer, real Telegram and real downloader acceptance remain open; this fixture has not undergone the full GUI matrix.

## Windows native credential and volume alias checks — 2026-11-17

Test source `ec5f9f8` on `codex/windows-validation-4ae46ed3`, tracked tree clean; retained untracked user directories were not test inputs. On the ordinary Windows user account, the opt-in native Credential Manager test **PASS 1/1** (`cargo test --locked --offline -p xarchive-desktop --lib native_create_replace_read_delete -- --ignored`). It creates a unique synthetic credential, reads, replaces and deletes it, then verifies absence and idempotent deletion; a scope guard also attempts cleanup. Test executable SHA-256 `8a56a9a02b3c6e614844316e8d0262abb1670d940573865193052520d3f9156a`; evidence `target/w6-credentials-ec5f9f8.log`, `.exit`, `.sha256`. This is native adapter CRUD only: GUI/restart secrecy and real Telegram acceptance remain open.

A separate scoped W1 path probe on ReFS compared Rust `canonicalize` for the same test directory through its E: path and a temporary Q: `subst` mapping: both returned the same E: canonical path (`target/w1-canonical-volume-probe.log`, `.exit`, source `target/w1-canonical-volume-probe.rs`). Q: was absent before and the mapping was removed afterwards. This is not evidence for every mount/alias arrangement and does not prove descendant exit or enable publication.

## Windows current Release and Full revalidation — 2026-11-16

Source `a79566b` on `codex/windows-validation-4ae46ed3`, tracked tree clean; retained untracked directories not used as package inputs. Ordinary-user locked offline Release Desktop build **PASS**, SHA-256 `cab55a9241178f33fa2fce3c6e5ef4c9b13e8dbcefdb28300fc556cd75f993da` (`target/w6-release-a79566b/desktop.log`, `.exit`, `.sha256`). The Windows containment module **PASS 7/7** with this exact Desktop keeper (`target/w1-identity-module-a79566b.log`, `.exit`); descendant-proof gate remains closed.

A new isolated Full package `target/w6-full-09ca336-fixture/dist-portable/Full-a79566b` assembled **PASS scoped**, with the current Desktop and unchanged Native Host/worker, and previously reused external downloaders (`target/w6-release-a79566b/full.log`, `.exit`, `full-verify.log`, `.exit`). Component hashes and required notices/manifest were checked. Actual packaged Desktop and Native Host over a private Named Pipe plus synthetic native-messaging client **PASS scoped**: unauthorized origin rejected, bootstrap and WebSocket authentication passed, replay rejected (`full-native.log`, `.exit`). On Windows 10.0.29704, WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, Full F3 GUI preview/select/import plus driver-directed JSON/YAML exports **PASS 1/1** on ReFS; post-session files matched captured hashes (`target/w6-full-gui-a79566b/runner.log`, `.exit`, `app.sha256`, `post-session-hashes.txt`, `post-session.exit`; invocation `target/w6-release-a79566b/run-full-gui.ps1`). This scope does not establish browser registration, system default download location, real downloader, installer, Telegram or the broader GUI matrix. The existing Edge browser processes and registration were left intact.

## Windows writer workspace identity — 2026-11-16

Production revision `6630312`, branch `codex/windows-validation-4ae46ed3`, tracked tree clean at exact-source runs; retained untracked `dist-portable/` and `sidecar/xarchive-downloader/`. Previously the writer Job Object identity hashed the caller's lexical workspace path, so a case alias of the same directory derived a different identity: the new native test **FAIL** before the fix (`target/w1-identity-alias-before-filter.log`, `.exit`). An earlier `--exact` invocation selected zero tests (`target/w1-identity-alias-before.log`); it is not evidence of a pass. The fix pins an existing local workspace and rejects reparse traversal before canonicalizing its name; alternate case and the resolved path now produce the same identity and recovery opens the same recorded job. Missing/non-Unicode canonical workspaces fail closed. This changes the provisional Windows writer record/job name; the factory has not enabled production writer creation, and incomplete or absent records remain refusals.

At exact `6630312`, Windows containment module **PASS 7/7** on the NTFS temporary workspace, including owner crash/recovery and registered-writer publication refusal; test executable SHA-256 `9691e05f8ad5c51ffd2b25743c58cd08bcbc4a09caed2476e4257ac82a3d6f74` (`target/w1-identity-module-6630312.log`, `.exit`, `.sha256`). The desktop keeper helper used for this test was the previously built Release binary from `2bcd12c`, SHA-256 `aa6d2cb2685bf88babaa17c0b43d850f60e55e2ffd31c07f7e50faa70071af11`; keeper entry behavior was unchanged. Identity/record test **PASS 1/1** on E: ReFS (`target/w1-identity-refs-6630312.log`, `.exit`). Windows architecture baseline resealed only for the reviewed containment file. This is identity hardening, not complete descendant drain: factory and publication remain fail-closed, W1–W5 gates remain open. Current-source Desktop release packaging requires a new build; prior GUI/packaging results remain bound to `2bcd12c`. Cross-platform Owner should review the Windows adapter and reseal its platform-specific architecture baseline when integrating; Windows Platform Owner continues the writer proof and W2 alias race.

## Windows current Full F3 GUI — 2026-11-16

Test HEAD `965c43e`, tracked tree clean, branch `codex/windows-validation-4ae46ed3`; app source `2bcd12c`, Release executable SHA-256 `aa6d2cb2685bf88babaa17c0b43d850f60e55e2ffd31c07f7e50faa70071af11`. Windows 10.0.29704 x64, fixed WebView2 153.0.4234.48 and EdgeDriver 153.0.4234.46, ordinary user, new isolated Full package on ReFS with a unique Named Pipe. Actual GUI/IPC **PASS 1/1**: synthetic valid and missing records previewed, only valid record was selectable and imported, and JSON/YAML exports were written to a driver-selected disposable directory; JSON contents and YAML re-preview were checked. Both files remained after the session and their hashes matched the retained copies. Evidence: `target/w6-full-gui-965c43e/runner.log`, `.exit`, `app.sha256`, `archive-records.json`, `archive-records.yaml`, `post-session-hashes.txt`, `post-session.exit`; invocation `target/w6-release-2bcd12c/run-full-gui.ps1`. This scope does not cover the system default download location, actual browser registration, real downloader, or broader DPI/manual matrix.

## Windows current-artifact Full assembly — 2026-11-16

Test source `09ca336`, branch `codex/windows-validation-4ae46ed3`, tracked tree clean; retained untracked user directories were not used as inputs. Current Release Desktop was built at `2bcd12c` (SHA-256 `aa6d2cb2685bf88babaa17c0b43d850f60e55e2ffd31c07f7e50faa70071af11`), Native Host at the same source (SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6`); intervening commits are documentation only. The committed packaging script, notices, Extension and configuration were copied into a new isolated fixture, with explicit reuse of those executables, the `d1854c1` worker (SHA-256 `341134d90a1681f094dd1c0cb0727c6677921c0e3300f760a3e1df9a227efe4e`) and the previously assembled external downloader binaries. Full assembly **PASS scoped** in `target/w6-full-09ca336-fixture/dist-portable/Full-retry`; manifest, required files, notices presence and Desktop/Native Host/worker/gallery-dl file identity passed (`target/w6-release-2bcd12c/full-09ca336-retry.log`, `.exit`, `full-09ca336-verify.log`). The reused gallery-dl executable SHA-256 is `0b36ae6734ed41e12be6be1b33d3165a450b3e0a811fc1b8c664c032f7f13b2c`; provenance and license completeness remain open. First assembly failed after copying because the deliberately minimal fixture omitted `desktop/src-tauri/tauri.conf.json` (`full-09ca336.log`, `.exit`); after supplying that current configuration a distinct output succeeded. Ordinary-user isolated actual packaged Desktop and Native Host with a unique Named Pipe and synthetic native-messaging client **PASS scoped**: untrusted origin rejected, bootstrap and WebSocket authentication passed, ticket replay rejected (`target/w6-release-2bcd12c/full-native-09ca336.log`, `.exit`; existing probe `target/w6-full-native-roundtrip-d1854c1.js`). Executable hashes were rechecked after this probe. No browser registry, installer, real downloader or Telegram acceptance is implied; current Full browser integration and GUI remain open.

## Windows Core native folder selection — 2026-11-16

Branch `codex/windows-validation-4ae46ed3`, test HEAD `cf282cb` with one pending documentation-only W1 evidence entry; Release Desktop source `2bcd12c`, SHA-256 `aa6d2cb2685bf88babaa17c0b43d850f60e55e2ffd31c07f7e50faa70071af11`. On Windows 10.0.29704, ordinary-user Core package on ReFS with a private Named Pipe, the real native folder picker selected the newly created isolated `target/w7-picker-cf282cb` directory. Storage showed that exact path immediately and again after restarting the app: **PASS scoped**. Evidence: `target/w6-release-2bcd12c/core-picker-confirm.txt` and `core-picker-restart.txt`. The picker element had no cached bounds, so the observed select button was clicked using the current window screenshot after keyboard navigation; this was an automation issue, not a product failure. No real media transfer or system default Downloads selection was performed. Test processes were closed.

## Windows Core first-run GUI — 2026-11-16

Source `2bcd12c`; Release Desktop SHA-256 `aa6d2cb2685bf88babaa17c0b43d850f60e55e2ffd31c07f7e50faa70071af11`. Windows 10.0.29704 x64, ordinary-user session, isolated Core package `dist-portable/windows-2bcd12c-core` on ReFS and a private Named Pipe. Scoped GUI **PASS**: app opened the local Tauri dashboard, exposed the first-run archive-location choice, and after selecting portable directory, Storage displayed the package-local `download` directory; that directory exists. UI evidence `target/w6-release-2bcd12c/core-startup.txt` and `core-storage.txt`. SQLite was shown connected, Sidecar disconnected and Extension missing as expected for this isolated Core startup. After restart, the same portable directory was still displayed. The native folder picker opened and Escape cancelled it; Storage still displayed the same directory (`core-dialog-cancel.txt`). The UIA indexed cancel click was unavailable in the cached state; Escape supplied the same cancellation action. The test processes exited; this covers startup, portable directory persistence and picker cancellation, not system download location, picker confirmation, Full package, or real transfer acceptance. The package now contains its isolated runtime data, so assembly verification above describes its pre-launch state.

## Windows current-source Release build — 2026-11-16

Source `2bcd12c` on `codex/windows-validation-4ae46ed3`, tracked tree clean, with retained untracked `dist-portable/` and `sidecar/xarchive-downloader/`. Ordinary-user `cargo build --release --locked --offline -p xarchive-desktop --features tauri/custom-protocol` **PASS**; production frontend unchanged since `95114ae`. Release Desktop SHA-256 `aa6d2cb2685bf88babaa17c0b43d850f60e55e2ffd31c07f7e50faa70071af11`. Evidence: `target/w6-release-2bcd12c/desktop.log`, `desktop.exit`, `desktop.sha256`. The initial multi-package command used an invalid feature qualifier and failed before compilation (`build.log`, `build.exit`); the corrected single-package command passed. Build completion was read from the exit file after the terminal session remained open. Native Host Release build from the same source **PASS**, SHA-256 `95dcc0d9912572d5809609fd59534a88f8259e50a69637ab2507c95278f9d2a6` (`native-host.log`, `.exit`, `.sha256`). Core assembly **PASS scoped** in new `dist-portable/windows-2bcd12c-core`, using the freshly built Desktop and the previously built `d1854c1` worker (worker source unchanged), with explicit reuse; manifest, required files and Desktop/worker hashes checked (`core.log`, `core.exit`, `core-verify.log`). Core excludes gallery-dl, Extension and Native Host. Full assembly, GUI and integration of this new release artifact remain open; prior artifact results retain their original scope.

## Windows F3 full file identity — 2026-11-16

Owner: Windows Platform Owner. Branch: `codex/windows-validation-4ae46ed3`. Incoming source `4ae46ed3a2a26cd243b9b4366330966aec4d0fc0`; production revision `98d7ff7a62df9dc2626083b38734c86b5afd83ac`, IPC test revision `0c77dd151be8b45727b85c4e6dc6123c86e133d2`, separate fixture volume revision `775e9e6af81659d95fad90d345979e316bffeb29`. The return revision is the documentation commit containing this entry. Tracked tree clean before documentation; retained untracked `dist-portable/` and `sidecar/xarchive-downloader/` are outside this evidence. No push.

Windows F3 preview revisions now bind the pinned root handle to the full volume and 128-bit file identifier, encoded with an explicit domain and byte order. The previous `same-file` identity exposed only a 64-bit file index, a ReFS identity insufficiency risk; no actual collision was observed. The change preserves root locking and reparse-point refusal. An outstanding preview made by the old executable must be repeated. `fs-id` 0.2.0 is used for `FILE_ID_INFO`; its crate checksum is `f1648c1b89e98d042d61b5c1ed0fbcd9dbd7815163864d20f8e7976a9b3af938`, its zlib license is recorded in third-party notices, and upstream source details are in the external sources register.

Native Windows targeted results against production revision `98d7ff7`: root identity tests **PASS 3/3** on NTFS and **PASS 3/3** on ReFS; Desktop record acceptance tests **PASS 9/9**. Test executable SHA-256 `20a359a8968f91cad6f35f3c5faede6c718dbc171a2b39686c63aad5718833a2`; logs and exit codes: `target/windows-full-id-{ntfs,refs,records}-98d7ff7.*`. `cargo check --locked --offline -p xarchive-desktop` **PASS** with the normal user Cargo cache. The first two isolated-target builds failed in MSVC jitterentropy compilation with C1083 long-path handling; logs `target/windows-full-id-check-{dirty,short,user}.*` distinguish those environment/tool failures from the successful ordinary-user build.

NTFS actual Desktop IPC root replacement test **PASS 1/1** at test revision `0c77dd1`, evidence `target/w7-root-replacement-0c77dd1-NTFS/runner.log` and `.exit`. It previews a synthetic record, replaces the root, rejects acceptance as stale, and confirms a subsequent preview remains acceptable. The embedded-frontend dev-channel debug Desktop executable built at `98d7ff7` has SHA-256 `ff9a35774cc23d04af4782c1490e1a41b4ca1dcddb6a46c7771870a64293ab63`; build evidence `target/windows-full-id-debug-98d7ff7.*`.

ReFS actual GUI/IPC alternate **PASS scoped** at fixture/test revision `775e9e6` with that executable running from isolated NTFS storage and the synthetic archive root on ReFS: replacing the root after preview produced `record import preview is stale`; re-preview returned one acceptable record, demonstrating the rejected operation did not import it. A later isolated application session on the same fixture previewed and accepted that record, reporting one committed and zero identical. UI evidence: `target/w7-root-replacement-tauri-775e9e6/observed.txt`, `accepted.txt`, `app.sha256`, and `fixture.path`. The ReFS WebDriver attempts yielded blank startup pages (`target/w7-root-replacement-0c77dd1-ReFS`, its retry, and `target/w7-root-replacement-775e9e6-ReFS`); the Tauri automation connector's direct IPC helper also failed to resolve its helper. These are tooling observations, not a product failure or a WebDriver PASS; GUI interaction supplied the equivalent scoped acceptance evidence.

`CROSS_PLATFORM_REVIEW_REQUIRED`: review the Windows-only identity adapter and shared dependency/notice changes; shared contract unchanged. Current-code W6 release/packaging and broader W7 F3 GUI coverage require revalidation against a newly built artifact; earlier release and GUI results remain evidence only for their recorded artifacts. W1 containment and W2 future 8.3 alias race remain unresolved and publication remains fail-closed. Next Owner: Windows Platform Owner for remaining W1–W7 work; Cross-platform Owner for review when the local Git history is formally handed off.


## 2026-11-16 — Ordinary-user ReFS and Full Native Host follow-up

Source `9cf53222f47777998d03db57cf376544af2ce227`, tracked tree clean; the
GUI test is unchanged from `de3873e`. Windows 10.0.29704, WebView2
153.0.4234.48 and the dev-channel Release Desktop built at `d1854c1`,
SHA-256 `a66dd4b6f86ffeaa850aba7cdc201c7aa00ba46ccbacf713a0b36926fa9cbb4b`.
In the ordinary Windows user environment, a fresh isolated Full package on
E: ReFS passed the synthetic record GUI import and JSON/YAML download test
**1/1**. Both files remained after the session ended and their hashes matched
the in-session copies (`target/w6-full-e-user-gui-evidence`). The independent
synthetic WebView download control also passed **1/1** on E: ReFS in that
environment (`target/w6-synthetic-e-user-evidence`). The driver explicitly
selected a disposable download directory; the normal user download location,
dialogs and the broader W7 matrix remain unverified. These results supersede
the restricted-environment ReFS failure below only for this scoped target;
they do not identify the restricted environment's precise cause.

Using the fresh Full assembly, actual Desktop and Native Host executable
SHA-256 `a896766b7a8f29638cea5364ada2351e47bbb372097bb13a86845733815fac68`,
an isolated unique Named Pipe and synthetic native-messaging client in the
ordinary user environment, **PASS**: wrong origin rejected, bootstrap and
WebSocket authentication succeeded, and ticket replay rejected. The
restricted-environment attempt returned `DESKTOP_NOT_READY` and is retained
separately. Logs and exit codes: `target/w6-release-d1854c1/full-native-roundtrip*`.
This is not a browser extension registration or real-account acceptance.

## 2026-11-16 — Release Full GUI and ReFS download differential

Test source `377b5adf98a11cf5b1a96e655f4f9ec165923dbd`; the Desktop executable
built at `d1854c1` had SHA-256
`a66dd4b6f86ffeaa850aba7cdc201c7aa00ba46ccbacf713a0b36926fa9cbb4b`.
The dev Full assembly from `56befd1` was copied to a fresh C: NTFS temporary
fixture. With Windows 10.0.29704 and WebView2 153.0.4234.48, WDIO **PASS 1/1**:
synthetic record preview/import and isolated JSON/YAML downloads completed;
both files remained after the session ended and matched the in-session copies.
Evidence: `target/w6-full-temp-gui-377b5ad`. The same executable alone in a
fresh C: temporary fixture passed the same scope
(`target/w6-release-temp-gui-377b5ad`). No real downloader, account or
network integration was exercised.

Two runs with the same executable and driver-selected download directory on
E: ReFS **FAIL** at evidence copying: a Full fixture and a separate executable-
only fixture each read a JSON download successfully, but it was absent when
copied later in the session (`target/w6-release-full-gui-377b5ad` and
`target/w6-release-e-gui-377b5ad`). A synthetic WebView Blob/download control,
independent of the record-export command, also observed files and then failed
to read them after three seconds on E: ReFS
(`target/w6-synthetic-e-read-evidence`); an earlier version passed on C: NTFS
(`target/w6-synthetic-c-probe-evidence`). The observed difference does not
isolate ReFS from WebView2, driver or the restricted execution environment,
and is not attributed to product code. The control probe is retained at
`target/w6-synthetic-download-probe.e2e.mjs`, SHA-256
`07a5be9f77b3f0691eec6ad6bc0df6ade9c24b2cf7f8e2f83129581b568c`.
Normal-user ReFS confirmation is queued separately; the C: scoped PASS does
not close that item or the full W7 matrix.

## 2026-11-16 — Fresh worker and portable assembly

Windows 10.0.29704 x64; build source
`d1854c1635df2cbd2dc9cbc63c1a707a0f5d3c76`, tracked tree clean. The
dev-channel MSVC Release Desktop and Native Host built **PASS** using locked,
offline Cargo and the production frontend built at `95114ae`; relevant Desktop
frontend source and dependencies did not change in between. Desktop SHA-256
`a66dd4b6f86ffeaa850aba7cdc201c7aa00ba46ccbacf713a0b36926fa9cbb4b`;
Native Host SHA-256
`a896766b7a8f29638cea5364ada2351e47bbb372097bb13a86845733815fac68`.
Build log and exit code: `target/w6-release-d1854c1/build.*`.

Python 3.12.14 and PyInstaller 6.22.3 built a fresh worker from the same
source, SHA-256
`341134d90a1681f094dd1c0cb0727c6677921c0e3300f760a3e1df9a227efe4e`;
the 56-file worker manifest SHA-256 is
`5a54b2aedd64949e37035f66fa02dd22b8e30b873fcf29cb8a89de352b1f6851`.
Worker `--help` and synthetic v2 download **PASS** with the controlled fake
gallery executable SHA-256
`37a68a72a659cec7a550fedb9df8291d9c33db230da75260caab4000316026c8`.
Build, file manifest, probe outputs and exit codes are in
`target/w6-worker-d1854c1`; this does not test a real downloader or network.

The initial Full assembly **FAIL** before output creation: preflight checked
the default Native Host path despite an explicit existing override
(`target/w6-release-d1854c1/full-assembly.log`). Fix `0b91f78` uses the chosen
worker and host sources for both preflight and copying. The next assembly passed
at that scope but omitted `LICENSE` and `THIRD_PARTY_NOTICES.md`. Fix `56befd1`
includes both in Full and Core and checks them before deleting any output.
These Windows packaging-script changes are `CROSS_PLATFORM_REVIEW_REQUIRED`.

At packaging source `56befd187df3f0a7aeb5ebb9b9a119115487c045`, Full and
Core assembly **PASS**, using the above fresh build outputs. The two notices
matched their source bytes in both bundles; all 56 worker files matched in
each bundle; Desktop, worker and Full Native Host executable hashes matched
their inputs; Core omitted Extension and Native Host; the packaged worker
`--help` passed. Portable Node tests **29/29 PASS**, including overrides,
both package types and preservation of existing output on a missing notice.
The isolated fixture is `target/w6-full-d1854c1-fixture/dist-portable`;
assembly, test and verification logs are in `target/w6-release-d1854c1`.
These are local development assemblies, not a release candidate. The included
gallery-dl and aria2 binaries were reused from the prior `e903f7f` fixture,
SHA-256 respectively
`0b36ae6734ed41e12be6be1b33d3165a450b3e0a811fc1b8c664c032f7f13b2c` and
`be2099c214f63a3cb4954b09a0becd6e2e34660b886d4c898d260febfe9d70c2`.
Their provenance and license completeness, release manifest/parity, real
browser registration, installer and real Telegram send remain unverified.

## 2026-11-16 — F3 download after session shutdown

Test source `de3873eec855427e7f67495185639a106a886e54`, tracked tree clean;
Windows 10.0.29704, WebView2 153.0.4234.48. Reused the dev-channel Debug
Desktop built at `95114ae`, SHA-256
`a7bfc543372f84735fc892326ea0302f41f77744d7722eed0e52b6517d46cfee`;
Desktop source and lockfile did not change. With an isolated driver-selected
download directory, the GUI test **PASS 1/1** saved JSON and YAML, validated
the YAML through the application's preview, waited three seconds and read it
again. After the WebDriver session ended, the runner verified both original
files still existed and each hash matched the copy captured during the
session. Evidence: `target/w7-f3-yaml-settled-de3873e`, including the
post-session directory listing, synthetic files, runner output and exit code.

At unchanged source `bdaa37b`, a separate run without the wait read and
copied the YAML during the session, but the original was absent in the
post-session listing (`target/w7-f3-yaml-post-session`). This is a download
completion or test-teardown observation; it does not establish its cause or
a product defect. Normal download location/dialog and the rest of W7 remain
open. The earlier `60860f7` result below proves only its stated in-session
scope, not post-session persistence.

## 2026-11-16 — F3 isolated YAML download and re-preview

Test source `60860f79925e2448c28b22ba0ee3c1fd4cc026f5`, tracked tree clean;
Windows 10.0.29704 and WebView2 153.0.4234.48. The dev-channel Debug Desktop
built at `95114ae`, SHA-256
`a7bfc543372f84735fc892326ea0302f41f77744d7722eed0e52b6517d46cfee`,
was reused; no Desktop source or lockfile changed between those revisions.
With a driver-selected, newly created isolated download directory, the GUI
test **PASS 1/1** saved both JSON and YAML archive exports. It read each file
and used the application's real preview command to parse the saved YAML and
confirm its synthetic record identity. Copies taken by the test while its
session was active are in `target/w7-f3-export-yaml-60860f7`, with runner,
exit code, fixture and app identity. JSON SHA-256:
`5652caef69dca981eb6b0386c7a6972c52979e33b0ca694c9adae8f883e9270a`;
YAML SHA-256:
`959f2aa1f90601c9c7de661581142746a758f6722ad646ebe2f83129581b568c`.

The first dirty-tree experiment's WDIO assertions passed, but post-session
copying of the YAML from its temporary fixture failed
(`target/w7-f3-export-yaml-experiment`), so that attempt is not accepted as
retained-file evidence. An in-session copy succeeded in the subsequent dirty
experiment and the exact-source run above. Normal download location/dialog,
other record cases and the full W7 matrix remain open.

## 2026-11-16 — F3 isolated JSON download

Test source `9662e0a8bf75dcec13d03c0d962a1b4d0585f6fd`, tracked tree clean;
Windows 10.0.29704, WebView2 153.0.4234.48 and the dev-channel Debug Desktop
built at `95114ae`, SHA-256
`a7bfc543372f84735fc892326ea0302f41f77744d7722eed0e52b6517d46cfee`.
Desktop source and lockfile did not change between these revisions. The WDIO
GUI check **PASS 1/1**: a synthetic archive record was previewed and imported,
then the JSON export button created `archive-records.json` on disk. The test
configured an isolated, newly created download directory through the driver
before clicking; it read the saved document and checked the Tweet identity.
The downloaded synthetic document was copied to the evidence directory and
independently hashed as
`5652caef69dca981eb6b0386c7a6972c52979e33b0ca694c9adae8f883e9270a`.
Runner, exit code, app hash, fixture path and document are under
`target/w7-f3-export-disk-9662e0a`. This is not a test of the normal Downloads
location, a save dialog, YAML download, or the full W7 GUI matrix.

## 2026-11-16 — F3 record export interface

WDIO test source `ef24b7e8de66c8148d3ae66dffb93e9667bda8f4`, tracked tree
clean. On Windows 10.0.29704 with WebView2 153.0.4234.48, the dev-channel
Debug Desktop built at `95114ae` (no intervening Desktop source or lockfile
changes), SHA-256
`a7bfc543372f84735fc892326ea0302f41f77744d7722eed0e52b6517d46cfee`,
passed the scoped GUI test **1/1**. It previewed and accepted the valid member
of two synthetic archive records, then used the record export controls to
generate `archive-records.json` and verified the exported Tweet identity.
The test intercepted the generated Blob and download request at the WebView
boundary to avoid writing to the user's Downloads directory. Saving a file to
disk and the remaining GUI matrix are not established by this test. Runner,
exit code, fixture and executable hash are in `target/w7-f3-export-ui-ef24b7e`.

## 2026-11-16 — Two destination guards before alias creation

Source `3ebc9865d8ab9357ec823cb78974197fb02c2e3a`, tracked tree clean;
Windows 10.0.29704 and a new disposable C: temporary fixture. An explicitly
ignored native diagnostic **observed the W2 defect**: two independent
coordinators in one process acquired guards for an absent short name and an
absent long name; creating the long directory then made both names resolve to
that same directory while both guards were still live. The expected `LONGDE~1`
alias was verified on this fixture before the observation was accepted. The
test's **1/1 PASS** means the diagnostic reproduced the unsafe state; W2
remains **FAIL**. The fixture was removed after the two guards were dropped.

This rules out a repair that only acquires the existing short alias on a later
lock request: both admissions may already have occurred before alias creation.
Any proposed repair must account for that ordering and preserve independent
destination concurrency. Logs and exit code are
`target/windows-alias-dual-3ebc986.log` and `.exit`; test executable SHA-256
`f1c2b62f2692840581358a17c100d08215be42ad91364e27a8f14cb44be82804`.
The separate two-process and full Desktop publication acceptance remain open.

## 2026-11-16 — Windows F3 cancellation classification

Source `95114ae86748fe576a2aa1f04613fb723fa3be49`, tracked tree clean at
build and execution. Windows record-import waiting now distinguishes a
cancelled wait and timeout from a coordination I/O error, while preserving
I/O failure precedence; the interface reports a cancelled wait as no import.
The Linux command keeps its earlier mapping. The shared interface adjustment
requires Cross-platform Owner review; it does not change the shared lock type.

Native Desktop targeted test **PASS 1/1**, covering timeout, cancellation,
I/O error and reopened admission; test executable SHA-256
`578552908dfcec5ac566e1cfaef2643070aaa102af6dca502a174275f28f5685`.
Frontend targeted Node checks **PASS 8/8**. Architecture and diff checks
passed. The earlier F3 IPC result below remains historical and described the
old generic cancellation message.

Native Windows 10.0.29704, WebView2 153.0.4234.48: the dev-channel Debug
Desktop built with embedded production frontend from this source, executable
SHA-256 `a7bfc543372f84735fc892326ea0302f41f77744d7722eed0e52b6517d46cfee`,
passed WDIO **1/1** (`target/w7-f3-cancel-ui-95114ae-retry`): the real GUI
previewed a synthetic record, cancelled while a separate process held workspace
admission, reported no import, and the record could be accepted after release.
The holder executable SHA-256 was
`874ae33b65f9d1a585cb33ed47d27ab585174b0bb3bd3ccad211bbf56254dc65`.
The first run (`target/w7-f3-cancel-ui-95114ae`) failed to create a WebDriver
session because WebView2 157 was selected against driver 153. The retry set
the fixed-runtime environment before starting the driver and succeeded; the
failed attempt is retained as tooling evidence, not a product result.
Fixture paths, driver output, runner logs and exit codes are in those folders.
This does not establish W3 cancellation with a production writer or close W7.

## 2026-11-16 — Windows F3 record import integration

Windows adapter and shared wiring source `cda4f61966313d536ed01734aafcf0cd1bea50f9`,
adapted from the earlier Windows Git work at `775af5f` through `9acd84d` for
the current split command modules. The adapter pins the root and selected
media, binds preview revision to the Windows file identity, rejects reparse
evidence and exports archive paths with portable separators. The maintenance
wait runs off the IPC thread and releases the runtime mutex while waiting.
Windows-only `same-file` 1.0.6 was already in the lockfile; the direct use and
MIT redistribution obligation are recorded in external sources and notices.
Shared command, evidence and export changes are `CROSS_PLATFORM_REVIEW_REQUIRED`.

The first targeted Desktop module run on the uncommitted port was **7 PASS /
1 FAIL**: a Unix-rooted media path was not recognized as absolute on Windows.
The export validator now rejects a leading slash on both platforms; the rerun
at committed `cda4f61` was **8/8 PASS**. Windows adapter tests **2/2 PASS**;
Storage record evidence **4/4 PASS**. A Windows junction directory/media
regression at test commit `bd87efd` **PASS 1/1**; Storage import transaction
tests at `339299c` **9/9 PASS**. The native test logs and exit codes are in
`target/windows-f3-{adapter,records,storage}-cda4f61.*`,
`target/windows-f3-junction-bd87efd.*` and
`target/windows-f3-storage-import-339299c.*`. The Storage test executable for
the latter two checks had SHA-256
`daae9481c24ba9ddadda6bf7ab7516e2b94c605c9f9efd28bb084ec2197b4d26`.

Windows 10.0.29704 x64, C: NTFS, WebView2 153.0.4234.48: Release Desktop
from `cda4f61`, SHA-256
`94661cb4e04383c08a9cc73b37b486cedb19768d88235c49df3367ff9e5cb59c`,
passed native WDIO **5/5** with record preview, accept, YAML export,
idempotent repeat, stale revision and changed-media rejection exercised via
real IPC (`target/w7-f3-cda4f61`). With the same executable and a synthetic
active workspace holder, the waiting import was cancelled without a commit
and accepted after release **PASS 1/1** (`target/w7-f3-cancel-357e34f`). The
current F3 cancel outcome is the shared maintenance-unavailable error; this
test does not claim a cancellation-specific UI classification. The interface
test **PASS 1/1** (`target/w7-f3-gui-339299c`): it previewed two synthetic
archive records, kept the missing-directory entry unselectable, accepted the
valid entry and verified its IPC export. Test-owned processes exited.

These scoped checks do not close the full F3 manual matrix, W1 writer drain,
W2 alias race or W7 GUI/DPI acceptance. Linux review and validation of shared
changes, including the maintenance-wait refactor, remain with Cross-platform
Owner. No Git push or ownership transfer was performed.

## 2026-11-16 — NTFS no-replace conflict classification

Source `599e54d` plus an uncommitted test subsequently committed as
`bf465f3`; Windows 10.0.29704 x64 and a newly created C: NTFS temporary root.
Targeted native test **PASS 1/1**: file-to-file, file-to-directory,
directory-to-file and directory-to-directory moves each returned
`DestinationExists`, retained the source and preserved pre-existing target
bytes. Test executable SHA-256
`874ae33b65f9d1a585cb33ed47d27ab585174b0bb3bd3ccad211bbf56254dc65`.
Log, exit code and fixture path: `target/windows-no-replace-types-a512a32.*`
(the filename is a historical label; source identity is `599e54d`). This
scoped result does not resolve the W2 8.3 alias-creation race or establish
Desktop publication acceptance.

## 2026-11-16 — Current application Windows UI regression

Release Desktop built from `3117798`, SHA-256
`db1df080dc1a48db77a8263b1c57de32fc4fa89867b65ef5587ffb0157710907`;
test invocation at `a512a32`, with no intervening application or dependency
changes. Native Windows 10.0.29704 x64, isolated C: NTFS fixture, fixed
WebView2 153.0.4234.48 and matching msedgedriver. Direct-driver WDIO dashboard
**4 PASS / 1 SKIP**: startup contract, dashboard shell/regions and invalid record
preview IPC passed; the archive record round trip remains gated and was skipped.
Theme media-emulation regression **PASS 1/1**. Both runs exited 0; driver and
application processes exited. Evidence and fixture path:
`target/w7-regression-a512a32`. This is not full GUI, DPI, actual OS-theme or
record-import acceptance. W7's remaining manual and prerequisite-bound cases
stay open.

## 2026-11-16 — Configuration lock failure through Windows IPC

Application source `3117798`, Release Desktop SHA-256
`db1df080dc1a48db77a8263b1c57de32fc4fa89867b65ef5587ffb0157710907`;
test source `d7fcdba`, with no intervening application-source or dependency
changes. Windows 10.0.29704 x64, WebView2 153.0.4234.48, pinned msedgedriver,
and an explicitly marked, newly created C: NTFS temporary fixture.

Real Tauri IPC lock I/O failure **PASS 1/1**: replacing the fixture's empty
admission lock file with a directory caused `workspace maintenance unavailable`,
not cancellation or timeout. The persisted file and exported runtime
configuration stayed unchanged. After removing the injected directory, the
same document applied and persisted `max_files: 13`. Test processes exited.
Log, exit code and fixture path: `target/w3-ipc-failure-d7fcdba`. This checks
an injected filesystem failure, not cancellation against a production writer;
the full W3 lifecycle remains open.

## 2026-11-16 — Configuration maintenance timeout through Windows IPC

Application source `3117798`, Release Desktop SHA-256
`db1df080dc1a48db77a8263b1c57de32fc4fa89867b65ef5587ffb0157710907`;
test source `c356f4b`, with no intervening application-source or dependency
changes. Windows 10.0.29704 x64, WebView2 153.0.4234.48, pinned msedgedriver,
and a new C: NTFS temporary fixture. The separate synthetic active-workspace
holder executable SHA-256
`357170e36d5cdd62add023ff25c7c2bc9895571d79eaa88bdc051fe48fb11802`.

Real Tauri IPC timeout **PASS 1/1**: with the shared holder active, the apply
waited approximately 30 seconds and returned `timed out`, not `cancelled`;
persisted and exported runtime configuration remained unchanged. A second
holder entered while the first was still active, proving admission reopened;
after both exited the same document applied and persisted `max_files: 12`.
Log, exit code and fixture path: `target/w3-ipc-timeout-c356f4b`. Test-owned
processes exited. This exercises a synthetic holder rather than a production
writer; W3 production lifecycle and failure classification remain open.

## 2026-11-16 — Configuration cancellation through Windows IPC

Desktop application source `3117798` (clean tracked tree at Release build),
SHA-256 `db1df080dc1a48db77a8263b1c57de32fc4fa89867b65ef5587ffb0157710907`;
test source `45d1b05`, with no application-source or dependency changes between
the two revisions. Windows 10.0.29704 x64, WebView2 153.0.4234.48 and pinned
msedgedriver. A separate synthetic process held a real shared workspace
admission lock; its test executable SHA-256
`3e326f5d60bc1acaa88e625b4a5b3ac2d2910ed4be80303dfe15bc828ac656ce`.

Native direct-driver WDIO **PASS 1/1** on both E: ReFS and an isolated C: NTFS
temporary root: the actual Tauri IPC apply remained pending while the holder
was active; the cancel IPC returned a cancellation outcome; persisted bytes and
the exported runtime configuration stayed unchanged. While the original holder
was still active, a second holder acquired workspace admission. After holders
exited, the same document applied and persisted `max_files: 11`. Logs and exit
codes are in `target/w3-ipc-45d1b05` and `target/w3-ipc-ntfs-45d1b05`; the
NTFS root is recorded in the latter's `fixture.path`. Test-owned processes
and driver exited. An earlier version of the same probe passed on ReFS in
`target/w3-ipc-f56f729`.

The lock holder is a synthetic process, not a production Sidecar or a writer
being drained. This proves the IPC/cancellation and admission behavior in that
controlled state; active production-writer cancellation, timeout, failure and
the full W3 lifecycle remain open.

## 2026-11-16 — NTFS destination alias identity

Native Windows 10.0.29704 x64, source `8523ea7` plus an uncommitted targeted
test: **FAIL**, two coordinator instances acquired destination locks for the
same existing `C:\Program Files` directory through its long name and its
`C:\PROGRA~1` alias. The destination was only read; lock files were in the
isolated workspace fixture. DriveInfo reports the C: destination volume as NTFS
and the E: fixture volume as ReFS. Log: `target/windows-alias-before-8523ea7.log`.

Windows adapter repair `2e8d42c` pins and canonicalizes an existing leaf,
rejecting reparse leaves. The same alias test **PASS 1/1**. Native module
suite **10 PASS / 1 ignored**, including a destination acquired before and
after creation under the same long name, reparse guards and the independent
process no-replace race. The suite's fixture root was on E: ReFS; it does not
by itself establish NTFS behavior for those other cases. Test executable SHA-256
`1176e29f269a338d2d8132bec2db0744ffb8bcebdf9f601c39f5831d5172268a`.
Logs and exit codes: `target/windows-alias-after-2e8d42c.*` and
`target/windows-native-2e8d42c.*`. The ignored alias test was separately run
with its explicit existing-NTFS-alias prerequisite.

The native suite was then rerun in a newly created C: NTFS temporary root at
`2e8d42c` plus the uncommitted future-alias regression: **10 PASS / 2 ignored**.
This covers the suite's normal NTFS cases, not either ignored alias case.
Test executable SHA-256
`faa5d542b253deddb35feb07e87c737097beb010a8bbe487eeab2452180c39dc`;
logs, exit code and fixture root: `target/windows-native-ntfs-2e8d42c.*`.

A further disposable NTFS fixture on an 8.3-enabled volume revealed a second
case, **FAIL** at `2e8d42c` plus an uncommitted regression test: acquire a
previously absent short name; create its corresponding long directory; then a
second coordinator acquires the long name instead of contending. Both names
were confirmed to resolve to the same directory. Log and exit code:
`target/windows-alias-future-2e8d42c.*`; the fixture path is recorded there.
The test executable SHA-256 was
`9e1658dcde40df9184524f3104944c08642d4570f1bcc39535a4efdb88a4ea93`.
The checked-in regression is ignored until the race is fixed and requires an
explicit disposable fixture. Existing-leaf normalization does not establish
the full destination-lock contract. W2 stays `FAIL`; W1's current factory
gate continues to prevent production publication. Any remedy must cover
future alias creation and preserve the agreed coordination semantics.

## 2026-11-16 — Native Host to Desktop isolated pairing

Source `b18c972`, tracked tree clean at build; pre-existing untracked
directories retained. Windows 10.0.29704 x64 MSVC Release builds of Desktop
and Native Host **PASS** (exit 0, channel `dev`). The embedded frontend was
built at `1e29ebe`; its application source and dependencies were unchanged
through `b18c972`. Desktop SHA-256
`d52ffbef250aedccfc89954e8941f06167225f56e1f53c78bd4fd30cef5a2049`;
Native Host SHA-256
`a896766b7a8f29638cea5364ada2351e47bbb372097bb13a86845733815fac68`.

In `target/w6-native-roundtrip-b18c972-final`, an isolated Desktop and the
actual Native Host executable used a unique per-run Named Pipe. A synthetic
native-messaging client **PASS**: an untrusted extension origin was rejected;
the expected origin obtained a bootstrap response through the pipe; a loopback
WebSocket authenticated with its one-time ticket; reuse was rejected. The
probe records only outcomes, not the ticket. Test processes exited, and
existing Chrome/Edge registry entries were unchanged. This is an executable
transport round trip, not browser extension loading or a browser registration
test. Probe and exit records are under the fixture; script:
`target/w6-native-roundtrip.js`.

The initial restricted-environment probe did not receive a ready bootstrap;
the ordinary-user retry reached the Desktop but its WebSocket was rejected
with 403 because the probe incorrectly included a trailing slash in the
WebSocket Origin. The corrected origin passed above. These observations are
retained in the adjacent original and `-retry` fixture logs, not classified as
product failures. Actual browser integration and W1–W7 acceptance remain open.

## 2026-11-16 — Windows factory entry containment guard

Source `ef4f730b9b039f024f10c92317a76660fbcbaf05`, tracked tree clean at
test; the two existing untracked directories were retained. A code review
found that the registered-attempt factory entry refused uncontained Windows
writers, but the separate `create_for_attempt` entry could reach the generic
uncontained Sidecar spawn. No execution through that alternate entry was
observed. The Windows-only guard now refuses both entries with
`WRITER_CONTAINMENT_UNAVAILABLE` before opening the database or spawning a
writer; the existing registered path and recovery gates remain fail-closed.

Native Windows 10.0.29704 x64 targeted test **PASS 1/1**:
`cargo test -p xarchive-desktop --lib --locked --offline windows_factory_entries_refuse_before_starting_an_uncontained_writer`.
It exercises both entries and verifies the configured database was not created.
Test executable SHA-256
`ddeffd0eb562cef457110ca466678cbb464a69e9e12f10356701cbafee30fd87`;
log and exit code `target/windows-factory-ef4f730.log` and `.exit`.
`cargo fmt --all -- --check` and `git diff --check` passed. This closes an
uncontained entry, not W1: descendant drain proof, factory integration and
successor recovery remain open.

At documentation-only successor `8d69d19`, the factory test module also
**PASS 2/2**, including legacy unfenced creation refusal; executable hash is
unchanged. Log and exit code: `target/windows-factory-module-8d69d19.log`
and `.exit`.

## 2026-11-16 — System theme in native WebView2

The isolated Windows Release executable from `bfca728` (SHA-256
`33617ece1a7cbfefd173e28490607ba4ad9f0fb9c0f1299af18afdea7e13c23c`)
was driven through pinned WebView2/msedgedriver with emulated system color
preference. **FAIL:** in system mode, the WebView reported a dark preference
but `--surface` stayed `#fff`. Fixed light under dark preference and fixed dark
under light preference produced their expected surfaces. This is a WebView
media-emulation check, not a change to or test of the machine's OS theme.

Shared repair `1e29ebe` adds the system-dark tokens to the application and
startup fallback without changing explicit theme precedence. Theme unit tests
**5/5 PASS**, focused theme/startup wiring **2/2 PASS**, Vite and native MSVC
Release builds **PASS**. Corrected executable SHA-256
`6ed99db50e3e3087d79e2d6c2862a5d26633bec883fc2a3382d3808b5b06859e`.
The same native WebView2 probe **PASS**: system dark `#17191c`, system light
`#fff`, explicit light under dark `#fff`, explicit dark under light `#17191c`.
The before/after evidence and driver logs are under
`target/w7-system-theme-before` and `target/w7-system-theme-after`; the probe
and assertions are `target/w7-system-theme-probe.py` and
`target/w7-system-theme-assert.py`. Browser sessions and owned drivers exited.
The added Windows direct-driver WDIO regression produced the expected **FAIL**
against `bfca728` (`#fff` versus `#17191c`) and **1/1 PASS** against the
`1e29ebe` executable. Logs and exit codes are in
`target/w7-wdio-theme-before` and `target/w7-wdio-theme-1e29ebe`. The test
source was uncommitted during those runs; the application source was clean
at its build revision. The test belongs to the advanced Windows WDIO suite.
Actual OS theme switching, fallback failure rendering and full DPI/visual
coverage remain open. `CROSS_PLATFORM_REVIEW_REQUIRED` applies to these shared
theme changes.

## 2026-11-16 — Dark primary-button contrast

On the isolated Release Full fixture built from `aa4ffd3` (Desktop SHA-256
`4660d90afdaff85bda33ad211cee119e7f89016c1d03307dc47259889256f2d8`),
Windows GUI **FAIL**: after choosing dark mode, the enabled first-use
“创建便携目录” primary button rendered white text on a near-white background.
Dark mode remained active after closing and reopening that fixture. The
Extension status link also opened its expanded Settings panel; it reflected
the existing machine registration to a different artifact, not a current
browser round trip. An initial offscreen click and an option-list click were
automation failures; scrolling and keyboard selection worked.

Shared CSS repair source `bfca728`, tracked tree clean at build: primary
foreground now follows a light/dark token. `theme.test.mjs` **5/5 PASS**, the
focused theme wiring test **1/1 PASS**, Vite and native MSVC Release builds
**PASS**. Corrected Desktop SHA-256
`33617ece1a7cbfefd173e28490607ba4ad9f0fb9c0f1299af18afdea7e13c23c`.
Native GUI **PASS, scoped**: selecting dark mode and returning to the first-use
dashboard displayed dark button text on the light background. The test used a
fresh isolated executable-only fixture; it did not exercise package, DPI or
full-page acceptance. Both test windows were closed. Based on CSS color values,
the primary text/background contrast is 14.23:1 in dark mode (11.84:1 on
hover) and 17.93:1 in light mode (7.81:1 on hover).

The broader `ui-wiring.test.mjs` run was **56 PASS / 1 FAIL**: an unrelated
release-workflow test assumes LF line endings while this Windows checkout has
947 CRLF lines (`core.autocrlf=true`). The workflow and test were unchanged
from the incoming revision; this is recorded separately from the visual fix.
`CROSS_PLATFORM_REVIEW_REQUIRED` applies to the shared CSS change. Full W7
visual acceptance must be revalidated for the changed stylesheet.

## 2026-11-16 — Release executable and isolated Full assembly

Source `aa4ffd3`, tracked tree clean with the same pre-existing untracked
directories. Native Windows x64 Vite build and MSVC Release Desktop build
**PASS** (exit 0, release channel `dev`, `tauri/custom-protocol`). Desktop
SHA-256: `4660d90afdaff85bda33ad211cee119e7f89016c1d03307dc47259889256f2d8`.
The committed Full assembly script then **PASS** with that executable in an
isolated fixture, `target/w6-full-aa4ffd3-fixture/dist-portable/XArchive`;
the packaged Desktop hash matches the build. Native host SHA-256
`d38ffc5c20da31b19e1c7a62146af31d9951ba710807c18142820567e657c827`,
worker `1b6f6e1e194b4c925627763f040fb3048f317899a802fc1a86c31085e0f62a66`,
gallery-dl `0b36ae6734ed41e12be6be1b33d3165a450b3e0a811fc1b8c664c032f7f13b2c`,
aria2 `be2099c214f63a3cb4954b09a0becd6e2e34660b886d4c898d260febfe9d70c2`.
The host was built from `e903f7f`; worker and external tools were reused.
The manifest points to the isolated host with the fixed extension origin;
`download/` was absent before first launch. Logs and explicit exit codes are
under `target/windows-{vite,cargo-release,full-assembly}-aa4ffd3*`.

Native GUI startup **PASS, scoped** for this packaged Release executable:
the isolated XArchive window rendered `http://tauri.localhost/`, showed the
first-use directory prompt, an empty dashboard, available SQLite, gallery-dl
as the selected path, detected aria2 and a disconnected Extension. No download
or registration was attempted. The window was closed and the test process
confirmed exited. This observation does not establish the remaining W7 flows.

The packaged worker's synthetic v2 hello/shutdown probe **PASS**, exit 0:
`ready` reported the five required capabilities. A separate synthetic download
probe used a deliberately nonexistent gallery executable and a disposable
staging directory: it emitted `download_started`, then
`SIDECAR_DEPENDENCY_MISSING`, and the worker exited 0. The probe script itself
exited 1 because its additional empty-directory assertion failed: a zero-byte
`gallery-dl.jsonl` remained. No media or network request was produced. This
proves command recognition and missing-dependency classification only; a
successful `download_completed` and WQ-DL-01 as a whole remain unverified.
Probe output and exit records are in the isolated fixture.

Subsequent synthetic success probe **PASS**: the same packaged worker was
given a locally built fake gallery executable (SHA-256
`37a68a72a659cec7a550fedb9df8291d9c33db230da75260caab4000316026c8`)
that wrote a 15-byte synthetic media file and matching metadata into a new
staging directory. It emitted `download_started`, `download_completed`, and
matching file identity and size; worker and probe exited 0. This closes the
direct packaged-worker protocol portion of WQ-DL-01. Starting it through
Desktop Settings and the real downloader path remain unverified. Source,
binary and probe are in `target/w6-fake-gallery.rs`, `target/w6-fake-gallery.exe`
and `target/w6-worker-success-probe.py`; event logs are in the fixture.

The first build-script invocation failed because the isolated checkout lacked
local Node dependencies. Vite was run with the existing parent workspace
dependencies. The first offline Cargo invocation lacked its dependency index;
the retry using the deep local cache failed in MSVC on a long dependency include
path. The successful locked build used the standard user Cargo cache and network
access. These are build-environment observations, not product runtime results.
This local dev-channel assembly does not establish installer, browser round trip,
license-complete release or real downloader/Telegram acceptance.

## 2026-11-16 — Isolated Full package assembly

Source `e903f7f`, tracked tree clean; pre-existing untracked directories were
retained. On native Windows x64, the committed portable packaging script was
run in `target/w6-full-e903-fixture` with Full packaging and binary reuse.
Assembly **PASS, scoped**: the manifest listed the worker, gallery-dl,
extension and native host; copied executables matched their inputs; the worker
`--help` exited 0. The output did not pre-create `download/`. Log:
`target/windows-full-assembly-e903.log`. Desktop SHA-256
`99497ceb254350e7b75e8c9d4b90ec93e79a615389f1b412b8b3bdae67ea513b`;
native host SHA-256
`d38ffc5c20da31b19e1c7a62146af31d9951ba710807c18142820567e657c827`.
The Desktop is a reused Debug executable built from `57f8d00`; the native host
was built Release from `e903f7f`. Worker and downloader binaries were reused.
This fixture does not establish a fresh Release Full distribution, installer,
license-complete release, actual browser registration or real downloader
acceptance. Those W6 checks remain open.

Read-only registration inspection found existing Chrome and Edge current-user
keys pointing at a managed manifest for a different portable artifact
(`dist-portable/auto-pair-d65a01b`); its host exists and its origin matches
the fixed extension identity. This is existing configuration, not a test of
the current artifact. It was left intact. A current-artifact registration and
browser round trip still need an isolated user/profile or a controlled reversible
test that preserves the existing registration and manifest.

## 2026-11-16 — W1 completion-port evidence boundary

Microsoft documents that ordinary Job Object completion-port notifications are not guaranteed and that new-process and exit-process notifications can race ([Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects), [JOBOBJECT_ASSOCIATE_COMPLETION_PORT](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_associate_completion_port)). Consequently, treating a stream of these notifications alone as a complete census would not establish the existing descendant-handle drain requirement; this is an inference from the documented delivery limit, not a new architecture decision or a Windows runtime test. The keeper-backed factory remains fail-closed. Continue investigating a loss-detecting proof and successor ownership without weakening the existing acceptance gate.

## 2026-11-16 — Process census feasibility experiment

Source `8d403b5` plus an uncommitted, discarded census experiment. A local
prototype retained process handles and compared their count with Job Object
lifetime accounting before permitting publication. On the native Windows
synthetic parent/descendant fixture, five repeated runs produced **2 PASS and
3 FAIL**: the failed runs reported six lifetime associations but observed
only three, four or five handles. The mismatch correctly refused publication;
a sampler can miss short-lived processes and is not sufficient for production
acceptance. No positive publication change or new dependency from this
experiment remains in the tracked tree.

The existing `firehazard` process-ID query returned `ERROR_MORE_DATA` with
multiple members. A temporary local patch to try that query first hit a
`Vec::set_len` precondition abort (`0xc0000409`) before a further length check
prevented that specific failure. This was an experimental dependency fork,
not a failure of the committed application path. The scoped source and fork
are retained under `target/census-experiment/`; logs include
`target/windows-census-patched-test.log` and
`target/windows-census-repeat-1.log` through `-5.log`. Do not adopt the fork
or treat polling/count equality as an approved recovery proof without closing
its race, identity, owner-crash and dependency-audit gaps. The existing factory
and publication gates remain closed. Next: investigate a reliable process-event
and handle-retention path, including loss detection and successor ownership.

## 2026-11-16 — Current Windows GUI and driver checks

Source `57f8d009df31b7729951e206b1cb811675d02817`, branch
`codex/windows-validation-4ae46ed3`; tracked tree clean at build and run,
with pre-existing untracked `dist-portable/` and `sidecar/xarchive-downloader/`.
Windows build 29704. The MSVC Debug build with `tauri/custom-protocol` passed;
the isolated executable SHA-256 was
`99497ceb254350e7b75e8c9d4b90ec93e79a615389f1b412b8b3bdae67ea513b`.
Fixture: `target/w7-gui-57f8d00`; the selected media path is gallery-dl direct
(`use_aria2: false`), but no actual media download was run.

Native UI **PASS, scoped**: rendered dashboard; initialized the portable
folder; navigated Settings, Record Import, Download Configuration, Account
Batch, Jobs and Storage; opened and dismissed the date picker and native folder
chooser. Exported, previewed and applied the isolated configuration through
real UI/IPC; the UI reported success and readback showed `initialized: true`,
`directory: ./download`, and `use_aria2: false`. Folder chooser cancellation
left the displayed fixture path unchanged. No active-writer cancellation was
exercised. One initial Computer Use click targeted an off-screen element and
was retried after scrolling; it is an automation observation, not a product
failure. The test window was closed.

Direct WDIO against a second isolated copy with the same executable hash
**PASS 4, SKIP 1**: startup contract, dashboard shell/regions, and native record
preview validation. The P1 record import/accept/export case was skipped. The
session targeted `http://tauri.localhost/`, WebView2 `153.0.4234.48`, and
EdgeDriver `153.0.4234.46` (SHA-256
`8477aca216f2db3750ba12b4c2964edf1ee8459df2e51961a5af2a82f8b5d410`).
The driver used an OS-assigned port; the launched driver PID was stopped after
the run. Logs and session identity: `target/wdio-57f8d00/runner.log` and
`target/wdio-57f8d00/startup/session-start.json`. This supersedes the prior
blank-target blocker for this readiness scope, not the skipped import case.

Native driver ownership cleanup **PASS 3/3** using `node --test
 desktop/test/wdio-driver-native.test.mjs` with `WDIO_NATIVE_CLEANUP=1` on
the native account. The real test terminated its own three-driver tree while a
foreign listener remained alive. tauri-driver SHA-256
`668bb60829ea4862646eb7f76a138f918d234375c54071de032d2fac1cc508b6`;
log: `target/wdio-57f8d00/native-cleanup.log`. This supersedes the earlier
cleanup blocker at this scope. W7 remains open for the remaining manual queue
and skipped record acceptance, and W3 remains open for real cancellation while
a contained production writer is draining.

## 2026-11-15 — Descendant termination evidence correction

Source `7bbbd5c` plus this fail-closed correction. The earlier local positive
publication result below is **SUPERSEDED**: zero active jobs and exited handles
for the two owned processes do not identify or join every descendant. The
native experiment already observed job count zero before an owned process
handle signalled. The adapter now leaves the registered writer undrained and
publication refuses even after local cleanup. The prior example demonstrated
handshake and local termination, but was not sufficient evidence to authorize
the full writer tree.

Windows native adapter tests passed 6/6 (including one child-only helper), and
the contained supervisor handshake example passed with publication still
refused. Test executable SHA-256
`127abe2c5a8da44b18b0523c7ff2930490fc39d6667843f411e89ede2426b1f3`;
example SHA-256
`f80158d7756f7d24a10379b74a1f455dcf8530c67d8e409940bcccb89b9be481`.
Those hashes bind the run before the test name-only change in this writeback.
The exact method to prove all descendant process exits remains open; do not
wire positive recovery or remove the Desktop factory refusal without it.

## 2026-11-15 — Windows contained Sidecar supervision

Source `929d35b` plus the contained writer and Sidecar supervisor changes in
this writeback, Windows build 29704. The Windows adapter suite **PASS** (6/6,
including its child-only helper): the registered synthetic writer exchanges
stdin/stdout, remains unauthorized while live, and authorizes only after
termination and drain. Executable SHA-256
`da4077ed9a6701a9e315b2ab8c45688a307263db844bd07d6510693766019210`.
The first targeted run exposed an important ordering issue: active-job count
reached zero while the owned writer handle was still running. The adapter now
also waits for its owned writer and keeper process handles to exit; the rerun
passed. The recovered-owner case still has no individual writer handles, so
session-bound recovery proof remains open. The shared supervisor library suite
passed 11/11; those unit tests do not exercise the new Windows branch.

`cargo run -p xarchive-sidecar-supervisor --example windows_contained_probe`
with `TW2TG_PYTHON` set to the native Python executable **PASS**. The example
uses the actual contained supervisor method and a synthetic JSONL Sidecar to
exercise handshake, argument quoting (spaces, trailing backslash, quotes and
empty argument), ASCII case-insensitive environment removal, shutdown and the
publication gate. Example executable SHA-256
`0d4ba0dd58f3576402be50b42bbf2423a15812754330e4d256bd4ccdf0c0a2db`.
This uses a test helper as keeper and does not prove the Desktop factory or a
real downloader. `CROSS_PLATFORM_REVIEW_REQUIRED` applies to the shared
supervisor's boxed stdin change and lifecycle wiring.

## 2026-11-15 — Recorded Windows writer registration

Source `548a15120fcd26c64363a69bd84d8e36af5cfd13` plus the Windows
containment/registration changes in this writeback. On Windows build 29704,
`cargo test -p xarchive-workflow --lib windows_containment::tests --
--test-threads=1` **PASS** (5/5, including the child-only helper). The tests
cover record exclusivity, malformed records, absence refusal, mismatched and
duplicate writer registration, and refusal to authorize an unstarted writer.
The crash-owner case uses the real Desktop keeper executable from the preceding
entry and a synthetic writer, opens through the persisted record and drains
the job to zero. Test executable SHA-256
`832ca4d132bc1f827bc2d7f8c30e111f3fdb69833a318d6e026ad84ca7d1e6e7`.
The first test compile failed due to a fixture borrow; the corrected run passed.
Shared `writer_lifecycle` registration wiring is `CROSS_PLATFORM_REVIEW_REQUIRED`.
Sidecar/aria2 startup and session-bound recovery proof remain unwired, and an
unstarted registration cannot become a publication permit.

## 2026-11-15 — Windows Job Object keeper groundwork

Source `74ef946` plus the Windows Job Object adapter, helper entry and
manifest/lock changes in this writeback. On Windows build 29704, the scoped
`successor_opens_and_drains_job_after_owner_is_killed` native test **PASS**
(1/1). A separate owner process creates a job, starts a keeper with an inherited
job handle and a synthetic writer, then is terminated. The successor opens the
same global name and observes the active count reach zero after terminating the
job. Executable SHA-256
`456a217b5902097af2d3a3c82f710a964f51c221c0ef2d8ddf6fe91d2177ffb3`.
The same crash test also passed using the freshly built Desktop executable as
the keeper, SHA-256
`bc5102b2b7d236581b5657fb96e6d1f5a1088c0080d810c491a1b5396dc2c23e`;
the corresponding test executable SHA-256 was
`f357f5f8833c15555446694f7e784e2974788787b9160f45c5825977b6e78e03`
for the final three-test run (the child-only helper returns immediately in the
parent run).
The Desktop MSVC check and Debug build passed. The test-only fixture gained
additional assertions while the build was running; production source did not
change during that build. The first run failed because the fixture expected exactly one active process;
the job contained two. The corrected assertion requires both keeper and writer.
This is native adapter evidence, not Desktop production recovery acceptance.
A keeper failure can remove the job name while writers are still shutting down;
absence must remain a refusal, never an inferred drain. Durable writer records,
registration, real Sidecar/aria2 launch and successor evidence are not wired.

## 2026-11-15 — Named Job Object liveness follow-up

At `bb4893a` plus the discarded Windows Job Object prototype and its temporary
dependencies, a scoped native test **PASS** (1/1) confirmed that, immediately
after the creator dropped its last Job Object handle, reopening the same named
job returned not found while `GetExitCodeProcess` reported the child still
active. This is an in-process handle-drop experiment, not a crashed-owner or
successor acceptance test. The temporary test executable SHA-256 was
`dd9a1a4f9950055d46bd75bb3fa6d6db7adcd91d78c0f47b04034d2fa1ca662`;
the diagnostic source and output are `target/windows-job-liveness-prototype.rs`
and `target/windows-job-liveness-test-2.log`. A preceding variant failed
because `taskkill` was denied by the environment; the passing variant used a
short-lived child. The production code and dependencies were restored afterward.
A recoverable Windows containment design must not equate an absent named job
with drained writers. The factory continues to refuse starting writers.

## 2026-11-15 — Native publication race and containment experiment

Source `8d0da2ed9f69767860a6fd22d0e171b8276aa4b8` plus the Windows native
race test in this writeback, Windows build 29704. The targeted native
`independent_processes_cannot_replace_the_same_published_directory` test
**PASS** (1/1): two independent processes race the real no-replace mover;
one succeeds, one receives `DestinationExists`, and the loser's staging data
remains intact. Test executable SHA-256
`cb3450151dc41d3262af5a215f433b00d81fdfac9f562394a8990d414cca7156`.
This is adapter evidence, not Desktop publisher acceptance. The first build of
this new test failed on unequal byte-array types; after fixing the test the
targeted run passed. A subsequent Windows native module run was **FAIL**
(9 passed, 1 failed): junction fixture creation returned access denied before
its product assertions; this is an environment failure and does not supersede
prior junction evidence.

A separate, discarded Job Object prototype was tested against the same source
with uncommitted prototype and dependency changes. Its test **FAIL**: a child
was observed in the named job, but reopening the job after dropping the
creator's last handle returned not found. The test did not independently
check that the child was still active at that exact point, so its lifetime
explanation needs an improved probe before it can serve as a recovery proof.
The prototype is not integrated; no absence-of-name inference is authorized.
Its first unrestricted cargo test also failed to compile a Unix-only integration
test on Windows; the scoped `--lib` run produced the failure above. Temporary
probe source, dependency diff and logs are in `target/windows-job-prototype.rs`,
`target/windows-job-prototype.diff`, and `target/windows-job-test-lib.log`;
they are machine-local diagnostics, not Git handoff evidence.

## 2026-11-15 — Windows production session routing

Source baseline `cbe827d8c2d88cc5f0d93e9f751636e194093476` plus the local
writeback carrying this entry. The Windows scheduler transfers its existing
workspace admission into a production coordinator, then uses the owned
session boundary and configured archive root. Until Windows containment is
implemented, the factory refuses a registered attempt before opening its
database or starting a Sidecar. This intentionally prevents downloads from
starting on Windows in this intermediate state.

Windows MSVC targeted tests passed: two submit/schedule cases, the production
coordinator's destination and workspace locks, submission refusal under
maintenance, and the no-containment factory refusal. The test executable
SHA-256 was `f0700d88e58d347e3bf43566a4c13b6dcc86fd6995284241d1041df5bc0488c3`.
The failed initial invalid-database test revealed that opening a lock path
could create a missing database directory. Windows now refuses admission
before creating a Job when that directory is unavailable; the rerun passed.

## 2026-11-15 — Configuration replacement

Starting revision `bc5525b8d74cc4744b0066e00880cdb363a420a9` plus the
test in this writeback. The Windows apply-flow test passed after a second
apply replaced an existing configuration and the stored YAML and runtime
both reported the new value. The first cancelled apply still wrote nothing.
Test executable SHA-256
`b485147224b5b39302ff83be6264c1f607507a615f1a6994abef4d2d8a77e215`.

## 2026-11-15 — Windows admission at the submission entry point

Source baseline `8a83388bcdca30c715bb6f828068552890c841e4` plus the
test in this writeback. The native Windows
`maintenance_refuses_submission_before_persisting_the_job` test passed.
It holds exclusive maintenance, calls the production submission entry point,
and verifies admission is refused before the job is written. Test executable
SHA-256 `924da05c5314f4274010722742af8c70e143ee23714e52a6917488f27e639b38`.
W1 writer containment and actual IPC cancellation during a live writer remain
open.

## 2026-11-15 — Windows configuration apply

Source baseline `cbe62406e9bd7c5b6e9f84b7084231993c9755f4` plus the local
writeback carrying this entry. Windows uses the maintenance turnstile during
submission, execution and startup recovery. Configuration apply waits on
the exclusive guard and classifies cancellation, timeout and I/O errors.

`cargo test -p xarchive-desktop --lib --locked --offline configuration_exchange`
passed 6/6. Test executable SHA-256
`acf1cad84f5437ef7bc3a7565e30a0359f800a70328cf5078a07bf52a6d6803a`.
The native GUI previewed and applied a synthetic configuration, reported
success, and the isolated `config.yaml` contained `max_files: 11`. The
GUI executable SHA-256 was
`54402881a88f9a6c008b71a2a54ced865f26e22d90af8e03f322ca653c2acefc`.
Real IPC cancellation against a contained production writer remains open.

## 2026-11-15 — Release executable, GUI smoke and startup classification

Release and GUI source: `cbe62406e9bd7c5b6e9f84b7084231993c9755f4`.
The local MSVC Release command was `cargo build --release -p xarchive-desktop
--locked --offline --features tauri/custom-protocol`, with release channel
`dev`. It passed. The executable SHA-256 was
`45dd301e95db5335215090591e978e6e313b59fbe0c1ab371c3d0c74b3356539`.
Core portable assembly passed using that executable, with the same hash
in `dist-portable/W6-cbe6240`. The Sidecar worker was built from the same
Python source with PyInstaller 6.22.3 and passed the `--help` smoke.
Worker executable SHA-256
`1b6f6e1e194b4c925627763f040fb3048f317899a802fc1a86c31085e0f62a66`.

W7 manual smoke `PASS` for the embedded-frontend Debug executable
`ac7efffd46c06c78819fe18850c4fc0cc726a508a8863b80f75643b60c65b1f8`.
Computer Use observed the native window, rendered dashboard, Settings and
Record Import on `http://tauri.localhost/` in a new portable workspace.
The dashboard reported the gallery-dl path and aria2 disabled.
Direct WDIO on the same executable, EdgeDriver 153.0.4234.46 and fixed
WebView2 153.0.4234.48 established a session, then remained blank through
the 30-second readiness gate. WDIO remains `BLOCKED`. Local evidence:
`target/wdio-stage3-d43ec923bb0544cb8001f44ed176c52a/startup/discovery.json`.
Earlier setup errors from a shared log filename and occupied fixed port
were removed by using distinct logs and an OS-assigned loopback port.

Windows Desktop startup classification rerun used the local startup change
in this writeback. Six of seven tests passed. The remaining
`startup_recovery_completes_valid_internal_archive_and_repeats_idempotently`
returned `Refused` for the PREPARED candidate, with expected `Recovered`.
It remains an open W1/W5 production case. Three retained-drain policy
tests passed on `cbe6240`.
Startup rerun test executable SHA-256:
`0ee3593f8f0e72cbf4ea897ecfb373c3f9f7645e0e1603200fb3af907174e895`.

## 2026-11-15 — Desktop MSVC and native transport

Source baseline `b19db4a6226632d7de46877e7e4dfc9181844ccf` plus the
Desktop changes in the local writeback carrying this entry. The Windows
build uses the portable aria2 error and owner path, and refuses startup
recovery requiring a Windows predecessor-drain adapter before resuming
an attempt. The existing Linux containment path is preserved.

| Check | Result | Evidence |
|---|---|---|
| Desktop MSVC Debug | `PASS` | `cargo build -p xarchive-desktop --locked --offline`. Executable SHA-256 `8fcd6636d822057e1a331b09d271eb68c9aab0ab3b0fe088c07939b94c76b418`. |
| Desktop Windows transport | `PASS` | `windows_transport::tests`, 4/4 with the native Windows account. The sandbox denied Named Pipe connections with error 5. The native run covered stalled client shutdown, repeated framed requests and the two registration validation checks. |
| Desktop Credential Manager | `PASS` | `windows_credentials::tests -- --include-ignored`, 2/2 with the native account. The synthetic credential was created, replaced, read and deleted. |

Desktop native tests used executable SHA-256
`997bfdbc788a4fb746ff1f1eb182ceaeb43284654a396a111a456c211211bc48`.
Actual browser registration, installer and Telegram send remain open.

## 2026-11-15 — NTFS destination identity and long paths

Source baseline `e4f696ca6f94fcc80ddedc12ab44b35bae8368f7` plus the local
writeback carrying this entry. The Windows coordinator keys destination
locks from the canonical existing ancestor and normalized suffix. Tests
contend on case variants inside one process and across separate processes.
The native mover also published a directory with a destination longer than
260 UTF-16 code units on NTFS.

`cargo test -p xarchive-workflow --test windows_native --locked --offline`
passed 8/8. Test executable SHA-256:
`7ef63f20cf8786302df159b1e8b29ccd608686f5816f37c2aa5e154a646cb597`.
Desktop production integration and Job Object recovery remain open.

## 2026-11-15 — NTFS coordination and Storage publication

Source baseline `d0a83d2d6f5b44b6944e42304de2ba20ea1adaa3` plus the code
in the local writeback carrying this entry. Windows Workflow reuses the
NTFS coordinator and maintenance turnstile from `775af5f`. The Windows
no-replace mover pins both parents and uses
[`atomicwrites::move_atomic`](https://docs.rs/crate/atomicwrites/0.4.4/source/src/lib.rs)
for the native move. Storage uses it for staging publication and verified
file replay.

| Check | Result | Evidence |
|---|---|---|
| Windows NTFS integration fixture | `PASS` | `cargo test -p xarchive-workflow --test windows_native --locked --offline`, 7 passed. Destination contention and admission release were checked across a child termination; no-replace preserved existing bytes and junctions were refused. Test executable SHA-256 `10ff3acebb9c2b3f8133dd63186ab8595ea9eddc04375f52100e3682cc9a4913`. The sandbox denied junction creation, so the complete run used the native account and its NTFS temporary directory. |
| Windows Storage library | `PASS` | `cargo test -p xarchive-storage --lib --locked --offline`, 171 passed. Includes four D1 choices, exports-off recovery and the existing subprocess crash suite. Test executable SHA-256 `ae4f890ccf9ea075dd818a9dbce26f0c50cb4595a2b23c640e87f040aa86b1f4`. The rerun includes fixture changes to release SQLite handles before cleanup and compare Windows paths as paths. |

W2/W3 Desktop integration, W1 Job Object writer lifecycle, W4 Desktop
long paths, W5 executor replacement, W6 Desktop build and packaging, and
W7 exact-artifact GUI acceptance remain open.

## 2026-11-15 — Windows MSVC recovery and Native Host

Starting source: `4ae46ed3a2a26cd243b9b4366330966aec4d0fc0`. The local
writeback on `codex/windows-validation-4ae46ed3` corrects the workflow
re-export cfg and marks the three Linux-backed Storage test fixtures with
their actual platform requirement. The Windows test runs included these
changes. Linux fixture assertions and execution stay the same.

| Check | Result | Evidence |
|---|---|---|
| Windows Workflow MSVC check and library tests | `PASS` | `cargo check -p xarchive-workflow --locked --offline` and `cargo test -p xarchive-workflow --lib --locked --offline`; 13 tests passed. |
| Windows Storage recovery suite | `PASS` | `cargo test -p xarchive-storage --lib --locked --offline archive_recovery`; 29 tests passed. The Windows Storage test executable SHA-256 is `14f5b2fb1d8a9e8f105eebbc2f0d90f857f4d60a60bdbf9560cb77c566105885`. |
| W4 four output choices and exports-off publication | `FAIL` | Both `internal_v1_attempt_publish_writes_exactly_the_selected_metadata_exports` and `prepared_replay_converges_when_all_user_exports_are_disabled` failed at the production staging commit on the same test executable. Windows returned `atomic no-replace directory commit requires the platform adapter`. |
| W6 Windows Native Host | `PASS` | `cargo test -p xarchive-native-host --locked --offline`; 13 passed, including actual Windows process bootstrap frames over Named Pipe and the pipe deadline test. Host executable SHA-256 `6cedd5f54fe9ed9b7915abf397e6caaf04a032bc74026ba46ef61e2a9b78abcd`. |

The `aws-lc-sys` build in the long checkout path hit MSVC C1083. A
temporary drive mapping plus a short compiler include path for the bundled
entropy headers let the native dependency build without modifying sources.

Desktop target `FAIL`: `cargo test -p xarchive-desktop --lib --locked
--offline windows_transport::tests` stopped in Rust compilation with 39
errors. Examples include Linux-only startup coordinator and recovery
references in `executor/runtime/startup.rs`, inconsistent retained aria2
fields in `archive.rs`, and Linux-only transfer types referenced by
`production/transfer.rs`. The Desktop tests did not execute and there was
no new Desktop executable for W7.

## 2026-11-15 — Windows native follow-up

Validation source: `4ae46ed3a2a26cd243b9b4366330966aec4d0fc0`, from the
documentation-only checkout at `8ca558c3bff2b5dae859553e2fc4addb3cca0262`.
Computer Use and remote publication were excluded.

### MSVC build and native checks

`cargo test -p xarchive-desktop --lib --locked native_create_replace_read_delete`
failed in dependency compilation on `x86_64-pc-windows-msvc` with E0432.
`crates/xarchive-workflow/src/lib.rs:43` re-exports
`consume_predecessor_retirement`, which `writer_lifecycle.rs:40–41`
defines only on Linux. This is a source-level compile failure.
`cargo check -p xarchive-workflow --locked --offline` reproduced E0432
independently of the Desktop dependency build.
`CROSS_PLATFORM_REVIEW_REQUIRED` for the feature-gate correction.
The larger Desktop build also reported `aws-lc-sys` C compilation failures
and a NASM tool warning in the sandbox. Track that separately when rebuilding
after the shared fix.

Exports-off Storage replay and Desktop Named Pipe acceptance are `BLOCKED`
by that dependency. The four-combination production export fixture is
Linux-gated. The Desktop frontend `npm run build` passed with Vite 7.3.6
from the existing parent workspace dependencies.

Credential Manager adapter `PASS` in an isolated MSVC test harness.
The harness copied the unchanged `windows_credentials.rs` and used the
source `SecretStore` contract and timestamp helper, with `keyring 3.6.3`
and its `windows-native` feature. The executable SHA-256 was
`ff640800cc26db5b2ab2b9e383ecb0eb4409b905b49fbe64fd6254dbb9865434`.
Running the executable with `--include-ignored` on the native user account
outside the sandbox passed 2/2: input/error redaction and synthetic
credential create, replace, read and delete. The sandbox run returned
`Unavailable` on the initial read. Desktop startup injection and Telegram
send remain unverified.

### W7 real driver teardown

Ran `node --test desktop/test/wdio-driver-native.test.mjs` on Windows
with `WDIO_NATIVE_CLEANUP=1` and explicit paths to the two executables:

| Executable | SHA-256 |
|---|---|
| `tauri-driver.exe` | `668bb60829ea4862646eb7f76a138f918d234375c54071de032d2fac1cc508b6` |
| `msedgedriver.exe` (154.0.4258.24) | `85baba4548cce09ab1416816cf17047872e7fbf05b5899c0dfde454af63e42d4` |

Result: `BLOCKED` for native driver teardown. Both fixture tests passed;
the real test reported `taskkill` exit 1 with `The user name or password
is incorrect`. A direct `taskkill /PID <pid> /T /F` against a separately
spawned synthetic Node process reproduced the error without WDIO. The
outside-sandbox retry passed both fixture tests and cancelled the real
test at its 60-second timeout. The latter timeout has no established
root cause. Preserve the real-driver item in the manual queue. GUI
remains `NOT_RUN`.

## 2026-11-15 — Stage 3 Windows intake

Source: `4ae46ed3a2a26cd243b9b4366330966aec4d0fc0` (`origin/dev` at intake).
Validation started from an isolated clean checkout. The previous local Windows
branch at `9acd84d` had 12 commits outside `origin/dev` and retained local
untracked data. Environment: Windows NT 10.0.29704, Rust 1.98.0
`x86_64-pc-windows-msvc`, Node v26.13.1, checkout volume E: ReFS.

| Check | Result | Evidence |
|---|---|---|
| F2/F3 client wiring | `PASS` | `node --test desktop/test/record-import.test.mjs`, 7 passed in the combined run. This is headless test coverage. |
| WDIO teardown helpers | `PASS` | `desktop/test/wdio-driver-native.test.mjs` in the same run: 2 passed, including the injected ownership-snapshot failure. The real-driver test was skipped. Combined totals: 9 passed, 0 failed, 1 skipped. |
| W1/W2 entry review | Native acceptance `NOT_RUN` | The writer-drain test file uses Unix/Linux-only gates. `xarchive-workflow`'s coordinator adapter and cross-process tests are Unix-gated. The checkout volume cannot substitute for a disposable NTFS root. |
| W3 and F3 native entry review | Native acceptance `NOT_RUN` | `desktop/src-tauri/src/commands/records.rs` has explicit unsupported-platform returns for configuration apply and record-import accept. |
| W4–W7 artifact and GUI acceptance | `NOT_RUN` | No release executable or SHA-256 was used. This run excluded Computer Use and left real-driver/GUI and other artifact-bound checks in the [Windows queue](windows-queue.md). |

An initial Cargo probe hit registry-cache access errors and an offline cache
miss for locked `rustls 0.23.46`. These are tooling observations. The filtered
`writer_drain` tests cannot supply W1 evidence on Windows because of their cfg
gates.

## 2026-11-12 — restored historical receipts (evidence only, not current acceptance)

Five documents that were written on Windows return branches but never reached this
line are restored here verbatim from their original commits:

- `windows-a3a02228-results.md` (from `145f6ad1`)
- `windows-f3-20261018-results.md` (from `codex/windows-f3-native-import-20261018`)
- `windows-followup-20261018-results.md` (from `codex/windows-f3-c1-followup-20261018`)
- `../development/windows-f3-native-import-20261018-plan.md` and
  `../development/windows-f3-c1-followup-20261018-plan.md`, restored only because
  the receipts link to them.

These are **historical records bound to the revisions they name**. Restoring them
does not re-run anything, does not retarget them at the current tree, and does not
upgrade any Windows gate. Old PASS results stay bound to their original source,
artifact and harness identities. Their old cursors and queues were deliberately
**not** restored: the current state is still only
[`windows-queue.md`](windows-queue.md) and
[`../status/platform-handoff.md`](../status/platform-handoff.md).

2026-10-06 — 40858835 GUI consistency Windows return

Input `40858835`; Windows callback repair/final validation `60b76f3`, return
branch `codex/windows-validation-40858835`. [Exact results and manual queue](windows-40858835-results.md).
Sidebar input ReferenceError FAIL → repaired event regression PASS; frontend 83,
Storage 74, final Desktop 257 PASS/1 ignored. Interpreter fixture failures and
intermittent PAC failure preserved. Fresh Desktop Full corrected inventory/ZIP
PASS scoped; worker fresh-build provenance pending. Native UI subset PASS;
date/index-19 automation blocked and full DPI/state/real integration pending.
Next Owner Cross-platform for repair review/integration, Windows retains manual
execution. No old acceptance or historical FAIL overwritten.

## Full 2b98952 user manual return — 2026-10-06

[Exact manual results and diagnosis](../validation/windows-full-2b98952-feedback.md): user reports first automatic browser connection/task creation PASS scoped; re-detect/sidebar/current-port consistency FAIL; gallery-dl and aria2 download completion FAIL user-observed. Running Full executable hash verified. Restart/reconnect and cause-isolated download reproduction remain NOT_RUN; existing historical and scoped build results preserved.

Next Owner **Cross-platform Owner** for status-source semantics and configuration-change/executor/transport lifecycle policy (`CROSS_PLATFORM_CHANGE_REQUIRED`, narrow repairs may instead require review). Windows retains native reproduction and exact-build revalidation. State `WINDOWS_VERIFICATION_PENDING`; release blocked by these failures. Documentation input `a104000`; build source `2b98952`, implementation `44e60e3`, Linux handoff `5c3efb79`, return branch `codex/windows-validation-5c3efb79`; handoff revision is the commit containing this entry. Tracked tree clean after commit; local untracked/user data preserved.


## 2026-10-06 — Full 2b98952 fresh build

Source `2b98952`, Windows implementation `44e60e3`: canonical fresh optimized Full, fresh Native Host/worker and verified pinned externals; 80 files, complete 8 pyd runtime, ZIP SHA `06ab9d7973a413f2bbd28d7c81dd83d24daa2e77384f751714c7a4f7affa1e11`. Rust 512 PASS/1 ignored, Node 214+52 PASS, Python 61 PASS; package/component/bootstrap/auth/current-size GUI/download-mode restart subsets PASS. Local gallery trust mismatch FAIL avoided by verified official pin and original restored. WebSocket close-within-3s probe FAIL retained. [Exact results](windows-full-2b98952-results.md); real archive/browser/Telegram/PAC/DPI remain pending; no production code modified.

---

## 2026-10-06 — Windows 5c3efb79 return

Cross-platform `5c3efb79` reconciled prior `d41c59e` via merge `344ca10`, accepted fixture `8f7fde6` and Toggle `8e0169a`, and assigned native PAC integration to Windows. Windows implementation/validation `44e60e3`, branch `codex/windows-validation-5c3efb79`: one native adapter change, shared code unchanged. Desktop 257 PASS/1 ignored; debug native build and current-size Settings/proxy render PASS scoped. Initial module FAIL, Python prerequisite and loopback WinHTTP DIRECT limitation preserved in [exact result](windows-5c3efb79-results.md). Native fixtures are not Registry/WPAD discovery or whole actual-egress acceptance; pending matrix retained. Next Owner Cross-platform for integration/review, Windows for native/manual execution.
---

## 2026-10-05 — ca25e53 return follow-up and manual Full evidence

Return/handoff documentation revision: `0aa8d14be1f3d4d3e5aac0886def850e9536f1b7`
on `codex/windows-validation-ca25e53`; input UI source/handoff:
`ca25e5379302431a3130436896735b8be2bb4744`. Fresh Full Desktop artifact:
`37344cb7777152af1baf76eec85480e432e43ceaf44baae8ee31f8c598fe1b5e`, built
from source `ca25e53` with docs-only build HEAD `9f383c0f08d70079880a652c5b3203e6db1786f7`.
The separate native `--no-bundle` GUI artifact is
`d9457efe8154f165650d1bd18a49844aed9d7d19a37f8d6d6e12350980e67ccc`; it is not
the Full package. Windows return added documentation/evidence only; no production
code or shared contract changed. The Cross-platform Owner integrated this return
and retained the exact artifact identities.

### Scoped settings result

Targeted Desktop Node tests 47/47 and fresh native `--no-bundle` build PASS.
On current-size and maximized captures, concise summaries, Bootstrap/Telegram
spacing, icon appearance, no duplicate Sidecar/Extension status icons, mouse
disclosure, Sidecar Space/Enter, Extension sidebar navigation/refresh independence,
SQLite navigation and independent open panels passed scoped inspection.
No credentials/settings were saved and no real message was sent. Full
100/125/150/200% DPI, narrow breakpoint, all-panel keyboard/hidden Tab/focus,
busy/error, masking and independent backend-effect checks remain `NOT_RUN`.
Owner-reported 200% scaling was not independently queried or changed; document-level
automation focus does not prove WebView control focus. UIA/capture attempts required
bounded recovery; no product defect was established. See `platform-handoff.md` and
`windows-queue.md`; do not widen these scoped PASS results.

### Fresh Full package and real manual observations

Full inventory/component startup passed scoped; this does not imply integration
acceptance. Owner reports automatic browser connection, task creation, Telegram
authentication and explicit Channel test receipt PASS. The real XArchive task
download failed with `DOWNLOAD_TIMEOUT` (FAIL), with no completed archive/output
integrity evidence. Confirmation dialog ACL errors and action-unknown were observed.
Owner separately reports that the same packaged `gallery-dl.exe` succeeded against
the same URL standalone and produced a readable image. This narrows the difference
to invocation context but does not identify a cause; task-specific redacted stderr/
progress was not provided. The Full directory/root check and empty `_staging` are
not archive-output acceptance.

After Desktop restart, automatic Extension reconnect was observed failing; explicit
Extension “Reconnect” restored authenticated/connected state. No timestamps establish
a deadline. A displayed/actual port mismatch was observed, but captures may refer to
separate reconnect sessions and do not prove concurrent state corruption. Manual
reconnect PASS does not close automatic-recovery acceptance.

Next actions: Windows Owner captures task-scoped redacted extractor arguments,
stderr/progress; compares app invocation to standalone; reproduces confirmation
ACL/action-unknown and records the relevant authorization path; verifies a successful
archive, output integrity and duplicate behavior; timestamps automatic reconnect;
continues the settings GUI matrix. Cross-platform Owner reviews shared extraction
diagnostics and confirmation call/capability routing under
`CROSS_PLATFORM_REVIEW_REQUIRED`. There is no evidence yet for
`CROSS_PLATFORM_CHANGE_REQUIRED`, no production fix is claimed, and no real-action
retry was performed by the agent. Keep Telegram credentials/cookies out of shared
evidence. Detailed procedure: `windows-full-ca25e53-manual.md` and current queue.


## Windows security implementation and native follow-up — 2026-10-05

This continuation supersedes the earlier aa8dabd pending implementation summary.
Branch: codex/windows-validation-aa8dabd. Input return: 3c77a1341bca3ffe7cd51b3b21622f6081b68e8d.
Implementation: 39f54e5011b626827ee38b3ab06fadaf316b51b6. Return/handoff revision:
the documentation commit containing this entry. State: WINDOWS_VERIFICATION_PENDING.
Current Owner: Windows Platform Owner; next Cross-platform Owner integrates this
Git return, then Windows Owner performs remaining platform acceptance. No shared
production code or contract changed, and no new shared-defect marker is asserted.
Tracked worktree clean at start; existing untracked local data preserved.

Implementation: windows-release.yml now defaults to contents:read; checkout does
not persist credentials; the build reads only the public Extension ID variable;
publication is a separate contents:write job requiring successful build. All
seven verified files are copied into a flat staging directory before upload;
publication rechecks source/tag, inventory, sizes, manifest and SHA256SUMS. Existing
asset names fail before upload; clobber was removed. Dispatch upload defaults to
false. Existing tag/source parity and independent non-blocking WDIO remain intact.
No Release was created, overwritten, or dispatched during this continuation.

External gallery-dl 2026.09.20 and aria2 1.37.0 are pinned in
 desktop/scripts/windows-external-assets.json. gallery digest was independently
read from the official upstream GitHub Release API; aria digest retains the
existing reviewed Windows aria2.rs pin. Verification rejects missing/invalid pins,
missing/empty files and altered bytes before execution/extraction. Pin changes
require independent source review; downloaded bytes never establish their own pin.

PASS: local release/security module tests 9/9; all 31 workflow PowerShell scripts
parsed; both real upstream downloads matched fixed pins; after verification,
gallery-dl --version returned 1.32.13:2026.09.20 and aria2 verified extraction and
--version returned 1.37.0. Isolated read-only Windows Actions PASS:
https://github.com/15699122/Tw2Tg/actions/runs/37255273960 , exact implementation
39f54e5, including negative integrity/policy tests and real asset downloads.
Full production build-to-publish transfer/rehearsal and default-branch CodeQL
rescan remain NOT_RUN; this isolated run is not production release acceptance.

Packaged Sidecar controls PASS on the unchanged aa8dabd worker: 256 metadata
commands (16 KiB each), producer blocked in all cases; matching extraction cancel
CANCELLED / exit 0 in 0.359s; full shutdown INTERRUPTED / exit 0 in 0.313s;
EOF / exit 0 in 1.782s, without synthesizing a shutdown event. Source queue remains
32, read count 33 while busy. EOF fixture marker PID 92060 was observed exited.
Cancel/shutdown fake descendant startup/whole-tree reclamation is NOT_RUN:
marker was not observed before controls; worker exit does not prove subtree cleanup.
Evidence/probes: validation-artifacts/windows-aa8dabd/queue-probe.json and
queue_probe_controls.py; no network/media accessed. Earlier probe timeout included
a fixture shutdown command missing required job_id; fixed, not a product defect.
Original pre-control child-marker waits and independent source diagnosis remain
recorded as fixture limitations, not a shared implementation failure.

Scoped native GUI PASS on unchanged aa8dabd Desktop hash
CA02189CFAB3CBEED5E017010A85B5DB6D2CD895BD91BEC208BEE7281E49A8FA:
current-size collapsed panels; mouse Telegram expand/collapse and form layout;
unconfigured credential checks disabled; hidden form absent from accessibility
text after collapse; SQLite opens storage and Sidecar opens its own section.
Computer Use produced actual snapshots. Current scaling was Owner-reported 200%,
not independently measured or changed. Keyboard focus automation did not establish
web control focus, so keyboard/hidden Tab acceptance remains NOT_RUN. Other DPI,
narrow/wide matrix, Extension jump/refresh independence, masking with entered fake
credential, busy/error and backend-side-effect checks remain NOT_RUN. No settings
or credentials saved; no Telegram send. Worker SHA-256 remains
11CFE182B2C1389F5813466D7B69747AC940F61C8E237EDA2F7016749D3902BE.

Owner explicitly selected local/CI scope for this continuation; real Telegram and
Channel validation remain in the manual queue. Cross-platform follow-up: integrate
Windows-owned workflow/tooling and review evidence; continue existing Linux advisory
work. Windows follow-up: production upload=false rehearsal/CodeQL evidence, fake
child reclamation and the remaining settings GUI matrix. Existing WQ-SEC-PERMS-01
closure and old icon/keyboard artifact results are preserved at their original identity.


## Windows aa8dabd validation return — 2026-10-05

State: WINDOWS_VERIFICATION_PENDING. Current/next Owner: Windows Platform Owner.
Branch: codex/windows-validation-aa8dabd. Input/handoff/validation revision:
aa8dabda5be8050c68ff472eb1090fca160a9082; Sidecar implementation:
2da530c2165125649ca47d4996908464dd32a696; settings implementation:
94fbfd403f8053d5e5584001b7e242e5321ec456. Exact target fetched from origin
and checked out through Git. Return revision is the commit containing this entry.
No production implementation changes in this return. Tracked tree was clean at
checkout; Windows-local untracked dependencies/artifacts were preserved. Tauri
build produced only a non-semantic Cargo.toml line-ending change, restored before commit.

PASS on native Windows 10.0.29680.0, Python 3.12.14, Rust/Cargo 1.98,
Node 24.19: protocol pytest 15/15; supervisor cargo tests 11/11;
Desktop Node tests 197/197 (elevated rerun); fresh PyInstaller worker build;
fresh Tauri optimized --no-bundle build. Independent instrumented queue probe:
capacity/queued 32, reader consumed 33 while extraction blocked, clean EOF after
release. This source probe is not packaged-process acceptance.

Worker SHA-256: 11CFE182B2C1389F5813466D7B69747AC940F61C8E237EDA2F7016749D3902BE.
Desktop SHA-256: CA02189CFAB3CBEED5E017010A85B5DB6D2CD895BD91BEC208BEE7281E49A8FA.
Desktop path: target/release/xarchive-desktop.exe. Worker path:
validation-artifacts/windows-aa8dabd/worker-dist/xarchive-downloader/xarchive-downloader.exe.
Logs and local probes: validation-artifacts/windows-aa8dabd/ (local artifacts,
not committed). Commands: pytest sidecar/tests/test_protocol_v2.py;
cargo test -p xarchive-sidecar-supervisor --locked;
node --test desktop/test/*.test.mjs; PyInstaller sidecar/pyinstaller/xarchive-downloader.spec;
node desktop/scripts/build-tauri.mjs --no-bundle.

Tooling limitations: initial sandbox Node killTree attempt was denied; elevated
rerun passed all 197. Packaged queue probe emitted extraction_started but its
controlled fake child marker never appeared (3-second attempts and a bounded
10-second reset-environment retry). Packaged cancel/full shutdown/EOF/child-exit
acceptance is NOT_RUN: fixture startup blocker, no product defect established.
Later shell channel lost valid cwd/FileSystem access; node filesystem/process
channel remained available and was used for reading evidence and Git write-back.

Scoped static review FAIL against release-hardening criteria: windows-release.yml
runs gallery-dl before trusted digest verification and expands aria2 before such
verification; workflow contents:write is inherited by build and publication is
in the build job. WQ-SEC-RELEASE-ASSET-01 and WQ-SEC-RELEASE-PERMISSIONS-01 remain
implementation pending; isolated Actions and negative digest tests NOT_RUN.
No release dispatched. WQ-SEC-PERMS-01 remains closed.

New settings native GUI/DPI/accessibility matrix NOT_RUN on this artifact;
previous Full-package screenshots/keyboard results do not close this changed UI.
Real Telegram/channel and real archive/duplicate tests remain NOT_RUN; bot/channel
creation reported by Owner is not credential/service acceptance.

Next Windows Owner: repair/independently verify fake fixture, complete packaged
queue cases; implement trusted asset pins and separate read-only build/publish
jobs, then isolated Actions; execute new settings matrix. Cross-platform follow-up:
review this shared queue source evidence and retain unresolved Linux glib/WDIO
advisory work; no new shared-contract defect or shared-code change claimed.


## 2026-10-04 — Telegram Windows local return f57c4b2

Owner Windows Platform Owner; priority P0 credentials / P1 remaining acceptance.
Method automated native/module/static plus Computer Use read-only smoke.
Fetched origin first, then exact target. Source `542dc8ebd0bed69ea66afd575f8e8bdadb52fdff`;
handoff `f57c4b2090f02df3ba55c58a660e2eee143a9660` differs only in handoff/queue docs.
New branch `codex/windows-validation-f57c4b2`, implementation/build source
`b5dd8c95f2685e8d91adf2bb9c24f7d7caf50012`; return revision is this record's commit.
Initial tracked tree clean; existing untracked caches/tools/artifacts preserved.
No direct filesystem sync or production shared contract/schema changes.

Environment: native Windows x64, OS 10.0.29680.0, PowerShell, Rust/Cargo 1.98.0,
Node 24.19.0, npm 11.17.0, MSVC and WebView2 available through successful native
build/window. User's development display uses 200%; no new DPI matrix recorded.
Computer Use `@oai/sky` imported successfully and enumerated Edge and Unigram;
neither their versions nor real account availability were verified. User explicitly
confirmed bot/chat/api_id/api_hash not prepared and selected local-only checks.

Implementation: target-only pinned keyring 3.6.3, explicit Windows WinCredential,
sanitized errors, missing-entry semantics, no default/mock/plaintext fallback,
production RuntimeState provider seam. Existing shared Mutex serializes access.
Unique synthetic native test credential automatically cleaned after CRUD.

| Check / reproducible command | Result / evidence |
|---|---|
| `cargo test -p xarchive-desktop windows_credentials -- --include-ignored` | PASS 2 native/input/redaction tests; credentials.log |
| `cargo test -p xarchive-desktop telegram -- --nocapture` | PASS 48; desktop-telegram.log; explicitly set PYTHON to .venv-windows-validation/Scripts/python.exe |
| `cargo test -p xarchive-telegram -p xarchive-storage` | PASS 72 / 54; shared-modules.log; mock/module evidence only |
| `node --test desktop/test/telegram-state.test.mjs desktop/test/telegram-render.test.mjs` | PASS 6; stdout, exit 0 |
| `npm run check --workspace desktop` | PASS; ui-build.log |
| `node desktop/scripts/build-tauri.mjs --no-bundle` | PASS release-profile native build; native-build.log; development channel, not Full release package |
| `cargo fmt -p xarchive-desktop --check`; `node scripts/docs-audit.mjs`; `git diff --check` | PASS exit 0; docs-audit.log |
| Computer Use exact EXE launch → Settings | PASS scoped: dashboard rendered, credential service available, token not saved, sender stopped; tool accessibility/screenshot transcript |

Logs reside in `validation-artifacts/windows-tg-f57c4b2/` (local, untracked).
Artifact `target/release/xarchive-desktop.exe` SHA-256
`f5e59f8b4a47ed4ee0e5df22e30bf32f6739bcbd430b4171e3aa147ce2e4d826`.
Final native credential test EXE SHA-256
`1fffa8bb50375045896898a8378fe9c0c742518c612457e03420d43014e5f51f`.
The final input-neutral cfg binding adjustment was included in the committed build
and final two credential tests. Initial test binary was subsequently rebuilt.

Tool diagnostics, not product failures: sandbox Cargo fetch initially failed
Schannel SEC_E_NO_CREDENTIALS; elevated fetch passed. Incremental-cache finalization
reported Access denied but compilation/tests passed; OS CIM query denied, bounded
Environment.OSVersion alternative succeeded. Linker emitted informational library
creation warnings. No assertions/product behavior weakened to accommodate tools.

Acceptance limits: WQ-TG-001 full isolated-account GUI/restart/leakage acceptance
NOT_RUN; WQ-TG-002–009 and UNI-01–08 NOT_RUN except the above scoped subchecks.
WQ-TG-007 server deployment PLANNED because credentials/service target absent.
WQ-TG-008 full DPI/keyboard/narrow/cancel/retry/UNKNOWN matrix deliberately deferred.
Scheme B remainder NOT_RUN this round; historical pairing failures not closed.
Full regression, Full packaging and publication not run: this local native adapter
batch uses targeted Telegram/storage/UI checks and creates no release artifact.
No real Telegram client reception, bot verification, permanent credential rotation,
large file, crash recovery, proxy or diagnostics leakage PASS inferred from mocks.

Next Owner Windows Platform Owner; manual prerequisites/actions in §K1 of
windows-manual-steps.md. Cross-platform Owner reconciles commits/Windows-only
dependency through Git; no CROSS_PLATFORM_CHANGE_REQUIRED or shared fix requested.
This is WINDOWS_VERIFICATION_PENDING, not WINDOWS_PASS; no WINDOWS_BLOCKING declared.
Reuse only if native adapter, dependency, runtime setup and account capabilities
are unchanged; otherwise REVALIDATION_REQUIRED. Release/service acceptance remains
pending independently of these local PASS results.

## 2026-10-01 Automated release build — `v0.2.1-pre1` (tag target `3115c3b`)

- Owner: Cross-platform Owner invoked the release; the Windows job ran on a GitHub-hosted runner. This entry records a **machine-driven build/asset run**, not a Windows Platform Owner manual batch. Tag `v0.2.1-pre1` → `3115c3b50716be0155804ad4f94dd9d29e37d617`; `windows-release.yml` Run `36838400270` (2026-10-01T08:46:13Z → 09:10:23Z); dispatched by `pre-release.yml` Run `36837862702` (`success`).
- Build job: `success`. All seven assets uploaded to the Release: `…-windows-x64.exe` 19 812 864 B, `…-windows-x64.7z` 4 552 639 B, `…-repository-dependencies.7z` 6 079 660 B, `…-full.7z` 34 615 357 B, `…-extension.7z` 12 388 B, `…-release-manifest.json` 1 351 B, `SHA256SUMS-v0.2.1-pre1.txt` 538 B.
- G3 asset-integrity gate **PASS**: manifest `source_sha` = `3115c3b50716be0155804ad4f94dd9d29e37d617` = tag target, and the five `sha256` values in `SHA256SUMS-v0.2.1-pre1.txt` match the manifest entries (both files downloaded from the Release and compared). `catalog_version` is `unreleased`, matching the `v0.2.0` manifest.
- Independent WDIO job: **FAIL**, non-blocking, and **never promoted to GUI acceptance**. Failing step `Run final executable UI readiness gate`: `WebDriverError: session not created: DevToolsActivePort file doesn't exist` on `POST http://127.0.0.1:4445/session`. The executable had already been downloaded and identity-verified (job steps 4–6 `success`) and `msedgedriver` was listening (`[direct-driver] msedgedriver pid=1372 is listening on port 4445`). Diagnostics artifact: `XArchive-v0.2.1-pre1-wdio-diagnostics-36838400270` (ID `11150613263`, 662 629 bytes).
- Classification: **environment class** (`COMPUTER_USE_UNAVAILABLE` — a hosted runner has no interactive desktop session, so WebView2 cannot create a DevTools session). It is **not** a regression and **not** a product defect: `v0.2.0` Run `36705896154` (job `109862080081`) failed on the **same step** with the **same error text**, and every recorded `windows-release.yml` run to date ends in `failure` for this reason. Per the Tooling Failure Policy this produced no product change.
- Not validated by this run: GUI/DPI/keyboard matrix, extension install/pairing/archive, Named Pipe/ACL/reparse/long-path/Unicode, and Explorer/taskbar/Alt+Tab/tray icon rendering at each size. These remain `NOT_RUN` in [`windows-queue.md`](windows-queue.md). A green build is not acceptance.



## 2026-10-01 Windows batch — source 6be3269

- Owner: Windows Platform Owner; branch `codex/windows-validation-6be3269`; source/handoff `6be3269f73877651683e6165f0108ef2526efca0` (includes implementation `279d726`). Fetched main/dev and checked out through Git, tracked tree clean. Validation revision is the commit containing this record. No source changes, no release/tag dispatch. Previous `e844ffe` describes published 7910033 only and does not validate this source.
- Environment: native Windows x64, PowerShell, Node 24.19.0, npm 11.19.0 via npx, Rust 1.98.0, Python 3.12.14; existing WebView2/Edge environment retained. Computer Use initially found no hidden target; visible interactive launch provided equivalent real app-specific window evidence. Display scaling was not changed or certified; multi-DPI results are NOT_RUN.
- Build origin: `npm run build:portable:windows --workspace desktop` with XARCHIVE_RELEASE_TAG=v0.2.1-pre1; fresh optimized Desktop PASS. Initial output under validation-artifacts was rejected by the packaging path guard before build; corrected to isolated dist-portable and succeeded (operator configuration, not product defect). PyInstaller 6.22.3 fresh worker PASS; cargo build --locked -p xarchive-native-host --release PASS. Full assembled with fresh Desktop binary reuse only after successful fresh build, current worker override and refreshed Native Host. Full location: `dist-portable/windows-6be3269-20261001-prerelease`. Local validation build, not published pre-release; version files intentionally remain 0.2.0.
- Artifact SHA256: pre-release Desktop `3e310ef8a3350b7bd4829fb661db26c317a6d6416d18ec46e9fc328fa770cd8a`; stable-channel Desktop `d73c0174dbd49ff4938dddc58a540ec65ac9c195d13482823d97115e75b0bbc1`; Native Host `9f6c4c57ffadeac8a5af5c8123cf4e18b98c0d0379baf54c5bf74d7289813b84`; worker `1dfca7fdbdb95ca47b161925dc71baa170cedd5eb7957d625198fc652494eaf3`. Stable-channel optimized EXE built with tag v0.2.1 at `dist-portable/windows-6be3269-20261001-release`. Evidence directory: `validation-artifacts/windows-6be3269-20261001/` (ignored/local).
- Desktop npm module tests PASS 179/179; Python test_proxy_mode PASS 8/8. Rust changed-module scope: core 31/31, desktop 161/161, download 26/26, sidecar-supervisor 11/11 PASS with PYTHON pointing to `.venv-windows-validation/Scripts/python.exe`. First run desktop 158/161, three discovery fixtures failed because default python3 did not yield a functioning worker; existing PYTHON override fixed the environment, identical module scope rerun exit 0. Preserve both logs; no assertion/code weakening. Initial compilation also warned incremental cache finalization access denied, successful rebuild/test result retained.
- WQ-LOGS-020-02: backend channel/default subcheck PASS: real optimized startup log `level=debug`, channel=prerelease/channel_default=debug/effective_level=debug/user_override=none; runtime/network/transport/executor debug lines present. Full item FAIL: actual Logs page lowest-level ComboBox is Info despite effective Debug; screenshot and accessibility tree observed. Root cause `desktop/src/pages/logs-page.jsx:16` hardcodes useState("info"). Evidence `prerelease-logs-tree.txt`, `prerelease-logs-ui.txt`, package logs. CROSS_PLATFORM_CHANGE_REQUIRED: initialize/synchronize display filter from effective backend level with appropriate shared tests. Real child-process module coverage NOT_RUN, not inferred from unit tests.
- WQ-LOGS-020-03: stable channel/default subcheck PASS: independent startup `level=info`, channel=release/channel_default=info/effective_level=info; debug startup lines absent, info frontend diagnostics present. First simultaneous release launch had Named Pipe access-denied warning due to active pre-release instance; stopped it and separately relaunched release, clean startup log. This is isolated-instance fixture contamination, not new product failure. Real progress/error coverage NOT_RUN.
- WQ-LOGS-020-04: pre-release Error UI save + restart subcheck PASS: Settings initially Debug; selected Error and saved; config logging.level=error; restart new log header level=error. Silent, stable-channel override and old-config upgrade remain NOT_RUN. WQ-LOGS-020-01 current-size spacing and green checked checkbox observed PASS subchecks; three DPI/narrow/focus matrix NOT_RUN. WQ-LOGS-020-05 title-bar archive icon observed; Explorer/property/taskbar/all sizes NOT_RUN.
- Current new Full Dashboard/SQLite/native window observed. No new Sidecar GUI handshake/completed download/install acceptance is claimed. UI/settings/known-folder/cancel/proxy-real-boundary matrix remains manual as listed in windows-manual-steps.md; default Downloads redirection needs isolated user/VM, real archive dedicated non-personal account, long child-process fixture still required. System Proxy Batch B PLANNED, NOT_RUN/IMPLEMENTATION_NOT_READY; Telegram paused/out of scope. Historical Native Host installation WINDOWS_FAIL remains unclosed.
- Cleanup: exact batch app processes stopped, driver not started; final process and port inventory `cleanup-final.json`. No full regression or repeated WDIO recipe. Reuse unrelated historical PASS only; current shared changes invalidate old UI/behavior PASS. Blocks development=false; release acceptance still open. Next Owner Cross-platform Owner for log filter defect and Git reconciliation; Windows retains pending platform matrix for next concentrated batch.

### 2026-09-30 Windows candidate validation (`5f952d0`)

- Owner: Windows Platform Owner. Formal input/handoff: `5f952d0091644775ec633a9e62a1696fe71fd471`; underlying integration tree: `60110a6f000887d134bf5fc46a7b2d6da91635a8`. Started from clean tracked `release/v0.2.0` at `5a841a9`, fetched `main`/`dev`, and created `codex/windows-validation-5f952d0` through Git. Initial generic fetch did not acquire the new main ref; explicit `git fetch origin main dev` did. Existing untracked caches, user data and artifacts were preserved. No filesystem handoff.
- Workflow implementation: `951453cd55a06b913836e9fa6dd30314ffbf631c`; test-fixture implementation: `4d1d9148dd13f187a734aa994e8ba4c796a33bd0`. Validation record revision is the Git commit containing this section, to be resolved from history; no self-referential SHA is invented. Application/Native Host and CI worker derive from `951453c`; the only subsequent code change is the configuration test fixture, which was present as an uncommitted test-only diff during local tests/build. No runtime source changed after the input revision.
- Environment: Windows 11 Pro for Workstations Insider Preview `10.0.29671`, x64; Node `24.19.0`, system npm `11.17.0`, validation npm `11.19.0` via npx; Cargo `1.98.0`; CI Python 3.12 / PyInstaller `6.22.3`; Bandizip CLI `8.0 Beta x64`. Working directory: `E:\Shiraishi\VSCode Workspace\Tw2Tg`. Evidence: `validation-artifacts/windows-batch-20260930-5f952d0/` (ignored).

| Item | Result | Command / evidence / limit |
|---|---|---|
| Dependency resolution | PASS | `npx --yes --package npm@11.19.0 -c "npm ci --no-audit --no-fund"`, 546 packages; restricted first attempt had npm-cache EPERM, identical elevated command passed. npm install-script warnings retained in `npm-ci.log`. |
| WQ-REL-020-G1 disposition | PASS (review + targeted regression) | `5a1ecf1` allocated-port snapshot and `8a1714f` native-core shell/banner/pinned-driver behavior already exist in current launcher/patch/config. Disposition: **superseded; do not merge the old branch wholesale**. Current code has allocated port deduplication/fallback, snapshot before upstream teardown, direct driver spawning patch, path/runtime opt-ins and newer direct mode. Historical protocol/schema/docs diffs from the old branch must not be replayed. Native driver session acceptance is not implied. |
| Targeted regression | FAIL before fixture correction; PASS after | `node --test desktop/test/patch-wdio-tauri-service.test.mjs desktop/test/wdio-config.test.mjs desktop/test/wdio-tauri-service.test.mjs extension/tests/content.test.js`: 51/52 then **52/52**. Windows driver-directory fixture created no `msedgedriver.exe`, although config checks it before updating PATH. Added a zero-byte stand-in that is never executed; no production check weakened. `targeted-tests.log` / `targeted-tests-fixed.log`. Small shared test adjustment: `CROSS_PLATFORM_REVIEW_REQUIRED`. |
| Extension module | PASS | npm 11.19.0 `npm run test --workspace extension`: **33/33**; `npm run check --workspace extension`: PASS. New credentials/trailing-path assertions pass. |
| Installed dependency patch | PASS | `node desktop/scripts/patch-wdio-tauri-service.mjs`: all four files already patched, no missing-pattern warning (`patch-idempotency.log`). |
| WQ-SEC-PERMS-01 / G2 execution | PASS; default-branch closure pending | Explicit `permissions: contents: read` and checkout `persist-credentials: false`; no write permission required for artifact upload. [Worker run 36700506149](https://github.com/15699122/Tw2Tg/actions/runs/36700506149) SUCCESS at `951453c`, build/verify/package/upload succeeded. Downloaded artifact contains EXE and `_internal/python312.dll`; EXE SHA-256 `f7d51e7f2ec34a4adb49b4e816e8fdcc199fa242411916157823a0f6a96f72f2`. `--help`, v2 ready/capabilities, unknown field -> INVALID_COMMAND, shutdown/exit 0 PASS. Alert #2 remains open on `refs/heads/main`; default-branch integration/rescan is required. Full CI log download returned 403; run/step metadata and downloaded artifact were available and retained. |
| Fresh application / Native Host / Full directory | PASS (local build/assembly) | npm 11.19.0 `npm run build:portable:windows --workspace desktop`, no binary reuse; `cargo build --release --locked -p xarchive-native-host`. Final assembly uses this run's CI worker. Output: `dist-portable/windows-batch-20260930-5f952d0-full`. Initial output under validation-artifacts was rejected by the documented dist-portable namespace guard, corrected as an invocation error. Full script permits absent aria2; official aria2 `release-1.37.0` and gallery-dl `2026.09.20` were subsequently assembled as in release workflow. Final external probes: aria2 1.37.0, gallery-dl 1.32.13:2026.09.20. |
| Local four-shape archive rehearsal | PASS; formal release gate NOT_RUN | Bandizip `c -fmt:7z -l:1 -r`, `t`, `x` on application / full / dependencies / extension; all magic `377ABCAF271C`; SHA-256 roundtrip **1/77/74/12** files. Full includes 8 `.pyd`, python312.dll, external dependencies; filtering removes startup SQLite/log files. EXE FileVersion/ProductVersion and package manifest are `0.2.0`, installation tag `v0.2.0`. App SHA-256 `1b7c26e0e45422686682adf6a49b8979f8b5e2cecd6f0f36b9d191a4320b369b`; Native Host `0724c83e667c57322b8ad1642103941bc50b15eb06bbf47dbd7ab5b28e4941ea`. These are locally staged shape probes, **not seven-asset Release candidates**: dependency staging retains the Full manifest; no formal release-manifest/SHA256SUMS/tag/source publication gate was run. No v0.2.0 tag or release was created. |
| G4 native GUI | BLOCKED / COMPUTER_USE_UNAVAILABLE | Exact Full EXE launched, PID 53324; logs contain `react_mount_completed`, initial IPC started/settled. Computer Use found XArchive handle 1055818, but screenshot displayed another foreground app; activation returned `window is not a usable app window`, bounded fresh inventory no longer returned XArchive. No coordinate input was issued. Logs are startup evidence only, not Dashboard/interaction/restart PASS. |
| G5 browser/pairing/completed archive | BLOCKED / NOT_RUN | Stable observable application prerequisite unavailable; no isolated profile installation, real cookie, task/output or restart acceptance executed. Dedicated non-personal test accounts remain prerequisite. |
| G6 Telegram | NOT_RUN | Controlled account absent and advertised release scope has not been decided; Cross-platform Owner must record whether Telegram is included. |
| G7 filesystem/transport | NOT_RUN | No complete candidate-specific Named Pipe/ACL/reparse/long-path/Unicode harness was executed. Historical unrelated PASS is retained only for its original revision, not promoted to GA acceptance. |
| Native E2E / full regression | NOT_RUN | Old deterministic upstream capability recipe not retried. G1 changes are already present and targeted tests cover the disposition; no runtime/protocol/Rust/Python changes warrant rerunning the full suite. Final release scope still requires its separate gates. |
| Cleanup | PASS (this run) | Exact launched package PID stopped; final worker/msedgedriver/tauri-driver inventory empty. No native driver was launched, so no new driver listener was created. |

Native Host manifest in the local Full directory contains its build-directory absolute executable path. It is not installation acceptance and must be regenerated/registered through the supported installation flow after extraction; no Registry or browser settings were changed. Official 7zr download encountered transport EOF; existing Bandizip CLI successfully proved real 7z format instead. Original failure records remain visible.

Follow-up: Cross-platform Owner reviews the shared fixture adjustment and integrates Windows evidence/workflow fix; default-branch CodeQL rescan must close alert #2. Release SHA must be frozen after integration, then formal asset and manual gates executed. Windows retains GUI/install/filesystem execution responsibility. Manual steps and statuses are in windows-queue.md; release remains blocked by unmet acceptance gates, not by Linux development.

This document records completed Windows validation batches. It does not replace the current queue in windows-queue.md or the active state in ../status/platform-handoff.md.

### 2026-09-30 completed Windows pre-release migration

- Owner: Windows Platform Owner; branch: `release/v0.2.0`. Formal Linux handoff: `7a3374b7af1a647addb0b9ab4e50e315468b6d26`; batch: `ba0f8aa1fe4632917a3dde5990103508a44225b4`. Windows workflow fixes: `e6d71ad44f8ef8cd8539259896787426e389ad67` (vacant-target native exit handling) and `2ec1aba78579932e7788fb3b3bb9e5fca597f906` (publishing secret). All ten dry runs and the first four two-asset draft runs used the vacancy fix revision; the remaining six draft runs used the publishing-secret revision. No shared contract/schema/planner/ledger change. Existing untracked local caches and evidence preserved; no Linux state-changing action.
- **PASS**: all ten Windows dry runs, then all ten draft runs with local downloaded asset SHA-256/size/source checks and applicable external manifest/SHA256SUMS. All ten drafts were gated together before publication. All ten public pre-releases and all 41 public asset download URLs returned HTTP 200, with GitHub asset digests still matching the independently downloaded files. New tags point to the frozen historical source SHAs.
- **PASS**: all ten old Release objects retired only after public downloads passed. Before each deletion, old Notes linked the replacement and recorded origin/backup; exact Release IDs, asset IDs, downloads, hashes and Notes snapshots were saved. All 36 frozen old files were independently hash/size verified in backup. Every old Git tag remains at its frozen SHA. Deleted old Release/asset IDs, download counters and old download URLs cannot be restored losslessly.
- User configured repository secret `PRERELEASE_MIGRATION_TOKEN` at `2026-09-30T06:13:55Z`; publishing at revision `2ec1aba` subsequently passed. No credential value was read or recorded.
- PAT tag creation triggered ordinary `windows-release.yml` run `36677316346`; Windows Owner cancelled it before build/assets/upload and temporarily disabled that workflow to avoid its ordinary `--clobber` upload path interfering with migration. This run is **CANCELLED / build and upload NOT_RUN**, not product FAIL. After all ten drafts passed, workflow state was restored to its original `active` state and re-read. No ordinary build asset was uploaded.
- `windows-release.yml` same-source gates: **PASS (static review only)** for explicit tag checkout, dispatch ref equality, HEAD/tag/push parity, complete asset manifest/hash/size/SHA256SUMS and same-run WDIO executable identity. No fresh ordinary release acceptance was run. Migration used its separate collision-rejecting workflow and no clobber.
- Historical runs `36672663525` (vacancy native-exit FAIL), `36674672997` (HTTP 403 tag creation FAIL), old `36651747470` Rust FAIL and pre.16 WDIO FAIL remain failures. Earlier checkpoint sections below describe their then-current blockers; this completion supersedes pending/blocker status without rewriting those observations. All four registration CodeQL language checks eventually passed in run `36672108116`.
- `v0.2.0-pre5` used isolated artifact run `36655693790`, expiry `2026-12-29T01:33:36Z`, rechecked unexpired before dry and draft dispatch. Executable SHA-256: `4f8181459c30c682235573649c2c8713ea5bf191f4a65582837ab21607f25fa4`. The old pre.6 two polluted assets remain backup evidence only, never accepted publication input. ZIP-era pre6 popup/options and historical package development-file warnings remain documented frozen-payload warnings.
- GUI/startup, browser/Native Host installation and full product regression: **NOT_RUN**. Native Computer Use inventory was available; no migrated XArchive window or installation was exercised. These package migration results do not promote historical GUI/WDIO/installation outcomes to PASS. See `windows-queue.md`. Planner tests 7/7, YAML parse and final diff checks cover the Windows workflow-only corrections.
- Local detailed evidence: `E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\migration-20260930-7a3374b`; each run has dispatch/run/log/evidence/accepted records; draft assets were downloaded; `ten-draft-publication-gate.json`, per-target `published.json`, and retirement before/annotated/retired snapshots establish ordering. Backup: `/home/shiraishi/xarchive-pre-release-backup` (Windows read via `W:\home\shiraishi\xarchive-pre-release-backup`).

| Source tag | New pre-release | Source SHA | Dry run | Draft run | New / old Release ID | Assets |
|---|---|---|---|---|---|---|
| `v0.1.1-pre.1` | [v0.1.1-pre1](https://github.com/15699122/Tw2Tg/releases/tag/v0.1.1-pre1) | `5afc1b8289fbd38792280431159136531ae138da` | [36672977464](https://github.com/15699122/Tw2Tg/actions/runs/36672977464) | [36674300106](https://github.com/15699122/Tw2Tg/actions/runs/36674300106) | 399747460 / 390068354 | 2 |
| `v0.1.1-pre.2` | [v0.1.1-pre2](https://github.com/15699122/Tw2Tg/releases/tag/v0.1.1-pre2) | `a5f42ccc4b6d661e3cf80338b44859e5178e8480` | [36673063515](https://github.com/15699122/Tw2Tg/actions/runs/36673063515) | [36674393927](https://github.com/15699122/Tw2Tg/actions/runs/36674393927) | 399747967 / 390459522 | 2 |
| `v0.1.1-pre.3` | [v0.1.1-pre3](https://github.com/15699122/Tw2Tg/releases/tag/v0.1.1-pre3) | `de61eaabc2013aa2e9c90481acbf3ba0b7df5535` | [36673130141](https://github.com/15699122/Tw2Tg/actions/runs/36673130141) | [36674492512](https://github.com/15699122/Tw2Tg/actions/runs/36674492512) | 399748496 / 390833384 | 2 |
| `v0.2.0-pre.1` | [v0.2.0-pre1](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre1) | `0105ce9fdb4f8c6e9e260312e730804a72d1a6f0` | [36673250103](https://github.com/15699122/Tw2Tg/actions/runs/36673250103) | [36674586528](https://github.com/15699122/Tw2Tg/actions/runs/36674586528) | 399749017 / 391884476 | 2 |
| `v0.2.0-pre.2` | [v0.2.0-pre2](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre2) | `f2ae58db1f5f8be901e1c45f7629147056edeea9` | [36673316986](https://github.com/15699122/Tw2Tg/actions/runs/36673316986) | [36677235313](https://github.com/15699122/Tw2Tg/actions/runs/36677235313) | 399763919 / 392292514 | 4 |
| `v0.2.0-pre.3` | [v0.2.0-pre3](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre3) | `baf0b241237afbd9fb7435f96403af2de5598d91` | [36673405745](https://github.com/15699122/Tw2Tg/actions/runs/36673405745) | [36677392476](https://github.com/15699122/Tw2Tg/actions/runs/36677392476) | 399765204 / 392324859 | 4 |
| `v0.2.0-pre.4` | [v0.2.0-pre4](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre4) | `38e9a78a56260f7064b9ebf6a5230b0a9260002e` | [36673504341](https://github.com/15699122/Tw2Tg/actions/runs/36673504341) | [36677519304](https://github.com/15699122/Tw2Tg/actions/runs/36677519304) | 399765983 / 392350523 | 4 |
| `v0.2.0-pre.6` | [v0.2.0-pre5](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre5) | `ac586e609337947aeb51de8f5cce3185efc8995e` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | [36678289636](https://github.com/15699122/Tw2Tg/actions/runs/36678289636) | 399772991 / 392465704 | 7 |
| `v0.2.0-pre.7` | [v0.2.0-pre6](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre6) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | [36677644817](https://github.com/15699122/Tw2Tg/actions/runs/36677644817) | 399766680 / 392743241 | 7 |
| `v0.2.0-pre.16` | [v0.2.0-pre7](https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0-pre7) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | [36677777905](https://github.com/15699122/Tw2Tg/actions/runs/36677777905) | 399767938 / 399271706 | 7 |

#### Final published asset hashes

| Target | Asset | Bytes | SHA-256 |
|---|---|---:|---|
| v0.1.1-pre1 | XArchive-v0.1.1-pre1-windows-x64.7z | 4125276 | `6614c6b5551a248c46b0bb1bdf0d84c18b7a5b7d130f1caa755e70d53bd7744a` |
| v0.1.1-pre1 | XArchive-v0.1.1-pre1-windows-x64.exe | 17477632 | `8e8277555a945f19d4dbfca84a1ec772a62b28f931320a6a5307b1bbbb8f2b83` |
| v0.1.1-pre2 | XArchive-v0.1.1-pre2-windows-x64.7z | 4132005 | `a91e1b199c770b839cf6e12cfaf8b5454c2e20096d45eee04d8d8fa6ee9f8de8` |
| v0.1.1-pre2 | XArchive-v0.1.1-pre2-windows-x64.exe | 17507328 | `f74b9d89338aadc0601ea42f41153e5e8da0cb61fd83bff247a6ea491df15d28` |
| v0.1.1-pre3 | XArchive-v0.1.1-pre3-windows-x64.7z | 4226964 | `01a291d638bcd8ab54000399ced56495707e7f903ad54f216982f05816d4f8a3` |
| v0.1.1-pre3 | XArchive-v0.1.1-pre3-windows-x64.exe | 18023936 | `b5c6fdcce79fa3cb10635d4f9eaea6c436a5d5af5834ae24ecfde64403b0e8ab` |
| v0.2.0-pre1 | XArchive-v0.2.0-pre1-windows-x64.7z | 4232181 | `a493e3c2dc44428b6141f49c2f16b400689a9aaa0bffc32118e74e095e365dec` |
| v0.2.0-pre1 | XArchive-v0.2.0-pre1-windows-x64.exe | 18082304 | `9ca2686e9c8697db8f6dd8ac47c60762c0178e2d08daa8d7e8c599a01eebb156` |
| v0.2.0-pre2 | XArchive-v0.2.0-pre2-windows-x64-full.7z | 34253140 | `c10dbe5a1ff67fc31f5deef3c45f118d53d7bd7ed017010a1314b18626425aec` |
| v0.2.0-pre2 | XArchive-v0.2.0-pre2-windows-x64-repository-dependencies.7z | 5917419 | `e52b2650c62f5c9db96f09bf41656fe20ca3626ef8ae78a3aa1417c85fcd366e` |
| v0.2.0-pre2 | XArchive-v0.2.0-pre2-windows-x64.7z | 4270811 | `883224127a98f5d708cf2622ee6f7ebbd6b475d0296f3f0c326db29b61aaa530` |
| v0.2.0-pre2 | XArchive-v0.2.0-pre2-windows-x64.exe | 18249728 | `75d1c0dbf0d3dbbd8f136b8aefd219f228a973d1b615b0127c73a445ca70e7da` |
| v0.2.0-pre3 | XArchive-v0.2.0-pre3-windows-x64-full.7z | 34257241 | `dbbb38fe905717f97001e44250e9c53025e65e33df1ecec0f4e86cb281b799f2` |
| v0.2.0-pre3 | XArchive-v0.2.0-pre3-windows-x64-repository-dependencies.7z | 5916757 | `131c4292d9a49aa4be6e7c53ab5fec34ec857ad1ebb85bb514e739c9b9474dc4` |
| v0.2.0-pre3 | XArchive-v0.2.0-pre3-windows-x64.7z | 4268968 | `3b6c09911ce7072993e78ab4db649dc8d7a31d6684d05ec63d0eabc21ecfd6dd` |
| v0.2.0-pre3 | XArchive-v0.2.0-pre3-windows-x64.exe | 18250240 | `a25e7132e546045096b9acb96ea1c3c9f2e25353733871a905dcb8eee1743125` |
| v0.2.0-pre4 | XArchive-v0.2.0-pre4-windows-x64-full.7z | 34258098 | `98704ab4fdddfcd356dac9822dadcb9f31031713dee8a8e9b865d554a60dcb3b` |
| v0.2.0-pre4 | XArchive-v0.2.0-pre4-windows-x64-repository-dependencies.7z | 5918180 | `6fa1f9422af2faae987673d10fdb10be26ce98ca8fd42eeccf55db47cba96bc4` |
| v0.2.0-pre4 | XArchive-v0.2.0-pre4-windows-x64.7z | 4266292 | `b17ba34bbc21ff86d1bfc89fe4610f583af0e3ce99b6595e1f124274fb7475d8` |
| v0.2.0-pre4 | XArchive-v0.2.0-pre4-windows-x64.exe | 18250240 | `bf42e1456caec9f31d963f9bbd301ad3e16ccf65fa6462efaf67f13ee5481b9e` |
| v0.2.0-pre5 | SHA256SUMS-v0.2.0-pre5.txt | 538 | `ac29fc7e2286c1b2c9a0a426beb20d0013383871e2e87b5d95969cb758a894f3` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-extension.7z | 5280 | `7641105d80e61530c5a2c27c818c3f7404b0051ac44dfc80465e147f5016ebae` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-release-manifest.json | 1350 | `e64928a53a1a37d4e944cd881a3b348138f18f55e335e8bc2a652011171ac68a` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-windows-x64-full.7z | 34319492 | `4424993b6c0fbbb3df8cfb3359635bb11c52d644fd855c4d8afa0f5355643758` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-windows-x64-repository-dependencies.7z | 6053977 | `ad74ad738aeecc1e39b6a28e9d3fb499bcd445ec931f89167aab4c239a7a1df1` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-windows-x64.7z | 4406570 | `6b73ba6aaa1f233b81a6374e3be084195d9703dd965c79dd58ddf50d6a133bc3` |
| v0.2.0-pre5 | XArchive-v0.2.0-pre5-windows-x64.exe | 18258944 | `4f8181459c30c682235573649c2c8713ea5bf191f4a65582837ab21607f25fa4` |
| v0.2.0-pre6 | SHA256SUMS-v0.2.0-pre6.txt | 538 | `495b638e23f48f0e2e73a4d1265c00e6f2f666a7a18d7c5ef82af4d4b62f3b33` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-extension.7z | 5606 | `c616163037f3f9056a431cb0c51b3d5d12b170eaf24f56d43d203b27b0630341` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-release-manifest.json | 1350 | `f3ae37a8d85e962f6c95c7a5949893f10d1379a7e71e706ab2ea1bf96974c956` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-windows-x64-full.7z | 34322489 | `d95f5bfd43e8c554880114de6ae157daeddf40ef845bd1b79b6303082edcd236` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-windows-x64-repository-dependencies.7z | 6055952 | `d801df21e5cb68a6b4cdf592ea8341206c1219a6ed26fd8aa8ed6808b2d0685f` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-windows-x64.7z | 4270916 | `7cbc14a9dc20768360f5623e7acbbc91aa0175e7f57a203aecef9c647e8c8e00` |
| v0.2.0-pre6 | XArchive-v0.2.0-pre6-windows-x64.exe | 18258944 | `9ab7a616acfaf6ab88ecd5db055772fc42c7d27bd64b69e3e3e64c1302e893f4` |
| v0.2.0-pre7 | SHA256SUMS-v0.2.0-pre7.txt | 538 | `06605c923f008d8eb729629ab166f74f985790eb9c137e4786c1d509024c9f8c` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-extension.7z | 12380 | `87cf3e8eb78358e53cc8b74fa76853d8a53d8d92cf51d0ba50c14feb18b941f5` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-release-manifest.json | 1351 | `ea5f2dd9a1a015f1535fad70d3d4b9c04699988baf39543cb513352ec2f593ef` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-windows-x64-full.7z | 34535087 | `8a04c36f65d9cac3f8a5af8dc0a87d42c79a5aae1e4bc15d9877dc8d623921fd` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-windows-x64-repository-dependencies.7z | 6078141 | `74e11b431f3bc506c599449c085e8413c782567fc1a5210fd13491d2f5c77f8d` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-windows-x64.7z | 4471230 | `64b2440cba04f28e5afe5d33ab614614db0684e134248bf7c43c82fc10551e90` |
| v0.2.0-pre7 | XArchive-v0.2.0-pre7-windows-x64.exe | 19264512 | `615b60c16b38bf851c1efc65e28e8bb4bf757fc77af07d5f92b207d56f01121d` |

#### Old asset retirement inventory

| Old tag | Asset ID | Asset | Downloads at retirement | Backup |
|---|---:|---|---:|---|
| v0.1.1-pre.1 | 568310070 | XArchive-v0.1.1-pre.1-windows-x64.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.1/XArchive-v0.1.1-pre.1-windows-x64.7z` |
| v0.1.1-pre.1 | 568309988 | XArchive-v0.1.1-pre.1-windows-x64.exe | 3 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.1/XArchive-v0.1.1-pre.1-windows-x64.exe` |
| v0.1.1-pre.2 | 569572099 | XArchive-v0.1.1-pre.2-windows-x64.7z | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.2/XArchive-v0.1.1-pre.2-windows-x64.7z` |
| v0.1.1-pre.2 | 569572054 | XArchive-v0.1.1-pre.2-windows-x64.exe | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.2/XArchive-v0.1.1-pre.2-windows-x64.exe` |
| v0.1.1-pre.3 | 570548950 | XArchive-v0.1.1-pre.3-windows-x64.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.3/XArchive-v0.1.1-pre.3-windows-x64.7z` |
| v0.1.1-pre.3 | 570548900 | XArchive-v0.1.1-pre.3-windows-x64.exe | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.1.1-pre.3/XArchive-v0.1.1-pre.3-windows-x64.exe` |
| v0.2.0-pre.1 | 573929132 | XArchive-v0.2.0-pre.1-windows-x64.7z | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.1/XArchive-v0.2.0-pre.1-windows-x64.7z` |
| v0.2.0-pre.1 | 573929085 | XArchive-v0.2.0-pre.1-windows-x64.exe | 3 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.1/XArchive-v0.2.0-pre.1-windows-x64.exe` |
| v0.2.0-pre.2 | 575996364 | XArchive-v0.2.0-pre.2-windows-x64-full.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.2/XArchive-v0.2.0-pre.2-windows-x64-full.7z` |
| v0.2.0-pre.2 | 575996344 | XArchive-v0.2.0-pre.2-windows-x64-repository-dependencies.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.2/XArchive-v0.2.0-pre.2-windows-x64-repository-dependencies.7z` |
| v0.2.0-pre.2 | 575996290 | XArchive-v0.2.0-pre.2-windows-x64.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.2/XArchive-v0.2.0-pre.2-windows-x64.7z` |
| v0.2.0-pre.2 | 575996254 | XArchive-v0.2.0-pre.2-windows-x64.exe | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.2/XArchive-v0.2.0-pre.2-windows-x64.exe` |
| v0.2.0-pre.3 | 576353696 | XArchive-v0.2.0-pre.3-windows-x64-full.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.3/XArchive-v0.2.0-pre.3-windows-x64-full.7z` |
| v0.2.0-pre.3 | 576353612 | XArchive-v0.2.0-pre.3-windows-x64-repository-dependencies.7z | 8 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.3/XArchive-v0.2.0-pre.3-windows-x64-repository-dependencies.7z` |
| v0.2.0-pre.3 | 576353555 | XArchive-v0.2.0-pre.3-windows-x64.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.3/XArchive-v0.2.0-pre.3-windows-x64.7z` |
| v0.2.0-pre.3 | 576353495 | XArchive-v0.2.0-pre.3-windows-x64.exe | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.3/XArchive-v0.2.0-pre.3-windows-x64.exe` |
| v0.2.0-pre.4 | 576383855 | XArchive-v0.2.0-pre.4-windows-x64-full.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.4/XArchive-v0.2.0-pre.4-windows-x64-full.7z` |
| v0.2.0-pre.4 | 576383830 | XArchive-v0.2.0-pre.4-windows-x64-repository-dependencies.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.4/XArchive-v0.2.0-pre.4-windows-x64-repository-dependencies.7z` |
| v0.2.0-pre.4 | 576383788 | XArchive-v0.2.0-pre.4-windows-x64.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.4/XArchive-v0.2.0-pre.4-windows-x64.7z` |
| v0.2.0-pre.4 | 576383744 | XArchive-v0.2.0-pre.4-windows-x64.exe | 3 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.4/XArchive-v0.2.0-pre.4-windows-x64.exe` |
| v0.2.0-pre.6 | 577042844 | XArchive-v0.2.0-pre.6-windows-x64.7z | 3 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.6/XArchive-v0.2.0-pre.6-windows-x64.7z` |
| v0.2.0-pre.6 | 577042799 | XArchive-v0.2.0-pre.6-windows-x64.exe | 1 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.6/XArchive-v0.2.0-pre.6-windows-x64.exe` |
| v0.2.0-pre.7 | 578464042 | SHA256SUMS-v0.2.0-pre.7.txt | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/SHA256SUMS-v0.2.0-pre.7.txt` |
| v0.2.0-pre.7 | 578464017 | XArchive-v0.2.0-pre.7-extension.zip | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-extension.zip` |
| v0.2.0-pre.7 | 578464040 | XArchive-v0.2.0-pre.7-release-manifest.json | 6 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-release-manifest.json` |
| v0.2.0-pre.7 | 578463954 | XArchive-v0.2.0-pre.7-windows-x64-full.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-windows-x64-full.7z` |
| v0.2.0-pre.7 | 578463913 | XArchive-v0.2.0-pre.7-windows-x64-repository-dependencies.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-windows-x64-repository-dependencies.7z` |
| v0.2.0-pre.7 | 578463873 | XArchive-v0.2.0-pre.7-windows-x64.7z | 7 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-windows-x64.7z` |
| v0.2.0-pre.7 | 578463791 | XArchive-v0.2.0-pre.7-windows-x64.exe | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.7/XArchive-v0.2.0-pre.7-windows-x64.exe` |
| v0.2.0-pre.16 | 598557936 | SHA256SUMS-v0.2.0-pre.16.txt | 6 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/SHA256SUMS-v0.2.0-pre.16.txt` |
| v0.2.0-pre.16 | 598557860 | XArchive-v0.2.0-pre.16-extension.7z | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-extension.7z` |
| v0.2.0-pre.16 | 598557937 | XArchive-v0.2.0-pre.16-release-manifest.json | 6 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-release-manifest.json` |
| v0.2.0-pre.16 | 598557715 | XArchive-v0.2.0-pre.16-windows-x64-full.7z | 5 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-windows-x64-full.7z` |
| v0.2.0-pre.16 | 598557637 | XArchive-v0.2.0-pre.16-windows-x64-repository-dependencies.7z | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-windows-x64-repository-dependencies.7z` |
| v0.2.0-pre.16 | 598557564 | XArchive-v0.2.0-pre.16-windows-x64.7z | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-windows-x64.7z` |
| v0.2.0-pre.16 | 598557462 | XArchive-v0.2.0-pre.16-windows-x64.exe | 4 | `/home/shiraishi/xarchive-pre-release-backup/v0.2.0-pre.16/XArchive-v0.2.0-pre.16-windows-x64.exe` |

### 2026-09-30 pre-release migration Windows preflight — 7a3374b

#### Draft continuation: historical-source tag permission blocker

- All ten Windows `upload=false` runs passed their source/payload/manifest/asset gates at workflow revision `e6d71ad44f8ef8cd8539259896787426e389ad67`; evidence downloaded and accepted locally. Run IDs in order: `36672977464`, `36673063515`, `36673130141`, `36673250103`, `36673316986`, `36673405745`, `36673504341`, `36673591851`, `36673841917`, `36673929521`.
- Four two-asset `upload=true` runs PASS, with draft assets downloaded and SHA-256 checked: `v0.1.1-pre1`=`36674300106`, `v0.1.1-pre2`=`36674393927`, `v0.1.1-pre3`=`36674492512`, `v0.2.0-pre1`=`36674586528`. All four remain draft/unpublished; old Releases remain available.
- `v0.2.0-pre2`, run `36674672997`: **FAIL** at `Publish draft pre-release with the verified assets`, specifically POST `git/refs`, `Resource not accessible by integration (HTTP 403)` / `Tag creation failed: v0.2.0-pre2`. Source and migrated four-asset gates PASS before this step; no pre2 tag/draft exists. Evidence is archived under `validation-artifacts/migration-20260930-7a3374b/draft/v0.2.0-pre2-failed-36674672997/`. The verification record's `uploaded=true` means upload was requested, not that it succeeded; `job_status=failure` remains authoritative.
- Classification: GitHub credential/permission blocker, not an asset mismatch or GUI/product failure. Repository secrets/rulesets listing returned no existing publishing secret/ruleset. The workflow explicitly grants contents:write, but GitHub's historical-workflow-file permission guard also requires workflow authority when tagging commits with workflow files differing from the default branch. Primary references: https://docs.github.com/en/rest/git/refs#create-a-reference and https://docs.github.com/en/rest/releases/releases#create-a-release; GITHUB_TOKEN cannot request Workflows:write (https://docs.github.com/en/actions/tutorials/authenticate-with-github_token).
- Windows-owned fix prepared: `.github/workflows/pre-release-series-migration.yml` binds `PRERELEASE_MIGRATION_TOKEN` only to the draft publishing step and fails explicitly if missing. Required repository-scoped fine-grained token permissions: Contents:write and Workflows:write; no broad local gh OAuth token is copied into the runner. Planner, ledger, payload and non-clobber/collision gates are unchanged. Remote credential-dependent verification is BLOCKED until the user installs that secret.
- Independent follow-ups: original 36-file backup verification PASS. Computer Use `@oai/sky` list_apps/list_windows is available; no XArchive window was opened and GUI/installation tests are **NOT_RUN**, not COMPUTER_USE_UNAVAILABLE. Full regression NOT_RUN because only Windows workflow/credential plumbing changed and twenty intended asset migration runs are the relevant scope. No publish or retirement occurred.

#### Windows continuation: registration and first dry-run failure

- Windows gh login verified for account `15699122` (repo/workflow scope); no token was recorded. `workflow view --ref` and branch dispatch initially returned 404 because the workflow was absent from default-branch registration.
- Formal registration PR: https://github.com/15699122/Tw2Tg/pull/6, merged as `dd57861d93aaa94beb90c7774777c730b79919e5`. Three historical add/add conflicts were resolved by retaining the already handed-off release files; reconciliation merge `f1a2ba3c3eea0242b9e489fa8b205a40b4de27d0` is tree-identical to `66c89a9`. Actions/JavaScript/Python CodeQL checks passed at merge; Rust CodeQL was still in progress (not reported PASS).
- Old backup verification: 36/36 frozen Release asset files matched both sizes and SHA-256 when read from `W:\home\shiraishi\xarchive-pre-release-backup`; detailed file/path/digest evidence is `validation-artifacts/migration-20260930-7a3374b/old-backup-verification.json`. This includes the two polluted pre.6 files as historical evidence only.
- `v0.1.1-pre1`, `upload=false`, run `36672663525`, workflow SHA `f1a2ba3c3eea0242b9e489fa8b205a40b4de27d0`: **FAIL** at `Refuse an existing target tag or release`. It printed `target v0.1.1-pre1 is unused`, then Actions reported `Process completed with exit code 1`. Source material verification/repackaging was NOT_RUN; no upload/tag mutation occurred. Log and evidence artifact were downloaded under `validation-artifacts/migration-20260930-7a3374b/dry/v0.1.1-pre1-failed-36672663525/`.
- Classification/root cause: Windows PowerShell Actions wrapper propagates the final native command's nonzero `$LASTEXITCODE`; `gh release view` returning 1 for the expected missing Release was not reset by `Write-Host`. Windows-owned fix in `.github/workflows/pre-release-series-migration.yml` replaces that query with a successful paginated Release listing, checks API/fetch errors explicitly, and handles `git show-ref` absence code 1 separately. No shared planner/schema/contract change. Local real-vacancy check PASS with native exit 0. Remote rerun is required; prior FAIL remains unchanged.

- Owner: Windows Platform Owner.
- Linux source branch/revision: `release/v0.2.0`, `7a3374b7af1a647addb0b9ab4e50e315468b6d26`; read-only `wsl -d Ubuntu -- bash -lc` Git inspection confirmed clean source worktree. Batch content: `ba0f8aa1fe4632917a3dde5990103508a44225b4`.
- Windows native worktree: `E:\Shiraishi\VSCode Workspace\Tw2Tg`; Git checkout matches the handoff. Existing untracked local dependencies/artifacts were preserved; no uncommitted source changes were included. Current edits are validation/Plan documentation only.
- Environment: Windows 11 Pro for Workstations Insider Preview `10.0.29671`, 64-bit; Node `v24.19.0`; official portable GitHub CLI `2.102.0` in `validation-artifacts/migration-tools/bin/gh.exe`. Official ZIP digest verified: `ae64e556ecc240b200f7eba60d550e4bb60d78e860e69dd88c449405b86067f4`.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| Formal Git synchronization | PASS | `git fetch origin refs/heads/release/v0.2.0:refs/remotes/origin/release/v0.2.0`; `git checkout 7a3374b`; `git rev-parse HEAD`; `git status --short --branch` | HEAD equals handoff; tracked tree clean; later local branch `release/v0.2.0` created at the same SHA | Default fetch refspec only covers old branches; explicit fetch is required |
| Linux source state | PASS | Read-only WSL Git status/HEAD inspection | Branch `release/v0.2.0`; clean; same full SHA | No Linux state-changing operation performed |
| Ten frozen migration mappings | PASS | Node `readMigrationLedger` on `docs/release/migration/pre-release-asset-ledger.json` | Ten unique `pre.N -> preN` entries; nine `repackage`, one `artifact` | Ledger remains unchanged |
| New target tags free | PASS | `git ls-remote origin 'refs/tags/*pre[0-9]*'` | No matching remote tag; release branch is at handoff SHA | Repeat before mutations; workflow independently rejects collisions |
| Migration planner regression | PASS | `node --test desktop/test/pre-release-migration*.test.mjs` | 7 passed / 0 failed | Planner PASS is not asset migration PASS |
| Isolated pre5 artifact availability | PASS | GET `repos/15699122/Tw2Tg/actions/runs/36655693790/artifacts` | `legacy-pre6-source-build-36655693790`; `expired=false`; expiry `2026-12-29T01:33:36Z` | Recheck immediately before pre5 dispatch; keep old pre.6 available if expired |
| Windows GitHub CLI authentication | BLOCKED | Portable `gh auth status`; `gh workflow view ... --ref release/v0.2.0 --yaml` | Not logged into any GitHub hosts; workflow view cannot run without authentication | User must complete local `gh auth login --hostname github.com --web`; never send tokens in chat |
| Default-branch workflow registration | NOT_RUN | Public GET `repos/15699122/Tw2Tg/actions/workflows` | Six existing workflows; migration workflow absent | Inspect `view --ref` and branch dispatch after auth; use formal PR/merge if registration is needed |
| Ten upload=false Windows runs | BLOCKED | Sequential dispatch order recorded in current handoff Plan | No run dispatched, no migration run ID or asset verification artifact | Depends on authentication/registration; local Linux dry runs are not substituted |
| Ten upload=true draft runs | BLOCKED | Sequential dispatch only after all ten dry runs pass | No new draft/tag created by this batch | Depends on ten Windows dry-run gates |
| Publish and old Release retirement | BLOCKED | Publish only after ten draft gates; verify links; annotate/back up old Releases before deleting objects | No Release/tag mutation performed | Depends on verified drafts/new downloads; old tags must remain |
| windows-release.yml same-source review | PASS (static review only) | Read source/tag gate, complete-asset gate and independent WDIO identity gate at handoff | Checkout uses release ref; dispatch ref must equal tag; HEAD/tag parity; manifest size/hash and SHA256SUMS; WDIO downloads same-run exe and checks run/tag/source/hash | No ordinary release run dispatched. Existing ordinary workflow upload steps use `--clobber`; forbidden for migration, whose separate workflow has no clobber and rejects occupied targets |
| GUI and installation acceptance | NOT_RUN | Manual cases remain separate from static/package gates | No migrated package exists yet; no GUI/install result collected | See queue; historical WDIO FAIL and GUI NOT_RUN remain unchanged |

### Errors and unresolved issues

- Sandboxed Git initially failed at `.git/FETCH_HEAD` (permission denied), and sandboxed HTTPS Git returned `SEC_E_NO_CREDENTIALS`; approved Windows execution resolved both. Sandboxed WSL enumeration returned `E_ACCESSDENIED`; approved read-only enumeration/inspection succeeded.
- Default `git fetch origin` succeeded but did not retrieve the release branch because the local fetch refspec only includes two earlier branches. Explicit release ref fetch succeeded. `git switch -c ... --track origin/release/v0.2.0` could not infer tracking from that restricted refspec; a local branch at the verified SHA was created without changing the refspec.
- Actual dispatch blocker: Windows gh has no authenticated GitHub host. The official CLI was installed only as a machine-local validation artifact, without changing project dependencies or source. User authentication was requested; no credentials were read or emitted.
- Current Owner stays Windows. No product/shared contract change was made or justified by preflight. Linux documentation reconciliation waits for a Windows Git handoff; this batch does not write directly into the Linux worktree.

## Recording template

### [date] [source revision]

- Owner: Windows Platform Owner
- Source branch/revision:
- Working tree included:
- Scope:
- Environment/toolchain:
- Canonical integration status:

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
|  | PASS / FAIL / BLOCKED / NOT_RUN / NOT_APPLICABLE |  |  |  |

### Errors and unresolved issues

- Issue:
- Classification: Windows-specific / shared / environment / test / unknown
- Blocker:
- Reproduction and evidence:
- Owner and next action:

Do not delete historical failures when a later handoff succeeds. Record the later result as a new batch and state whether the previous conclusion remains reusable.

### 2026-09-23 Windows E5–E7 implementation and Full package validation

- Owner: Windows Platform Owner
- Source branch/revision: `feature/u7-desktop-production-integration` / input `bde6dc375bda68944b500714db8f9c39693bd06f`
- Working tree included: Windows E5 Named Pipe server, E6 current-user Native Host lifecycle, E7 status wiring, Full-package input-path fix, and the shared UI wiring pending Linux review.
- Scope: E5–E7 for the current U12/U17 Windows batch; Full package built from current Desktop/Native Host Rust source, current Sidecar Python source and repository Extension source. Existing local gallery-dl artifact was bundled as an external dependency.
- Environment/toolchain: Windows 11; x86_64-pc-windows-msvc; Cargo offline; Node 24.19.0; PyInstaller 6.22.3 / Python 3.12.14. Existing `target/release`, Sidecar artifact and prior validation artifacts were preserved; build and package outputs used new directories under `validation-artifacts/`.
- Local cache note: the PyInstaller build was invoked with `--clean`, which cleared its regenerable `%LOCALAPPDATA%\pyinstaller` cache. The project virtual environment, existing frozen Sidecar artifact, logs, and prior validation-artifact directories were not cleaned or replaced.
- Canonical integration status: Windows implementation committed and pushed; current-user browser registration and GUI/browser acceptance remain queued. Shared UI changes require `CROSS_PLATFORM_REVIEW_REQUIRED`.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| Windows formatting | PASS | `cargo fmt --all -- --check` | Clean | None |
| E5/E6 Windows library tests | PASS | `cargo test -p xarchive-desktop --lib --target x86_64-pc-windows-msvc --offline` | 89 passed, 0 failed; includes two sequential Named Pipe framed requests with echoed request IDs | Cross-user ACL and packaged Native Host integration remain manual |
| Windows Release build | PASS | `cargo build -p xarchive-desktop -p xarchive-native-host --release --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-full-e5-e7-build-20260923` | Desktop and Native Host built from the implementation source into an isolated target directory | Native GUI launch is queued |
| Sidecar current-source build | PASS | PyInstaller spec `sidecar/pyinstaller/xarchive-downloader.spec`, separate `--distpath`/`--workpath` | Fresh one-dir worker built from `sidecar/src`; `_internal/python312.dll` included | Bundled worker handshake probe PASS |
| Full package assembly | PASS | `node desktop/scripts/build-portable-windows.mjs` with isolated Desktop, Native Host, Sidecar and output paths | `validation-artifacts/windows-full-e5-e7-current-source-20260923`; required files, Full manifest, Extension ID and host executable path/origin verified | GUI/Native Host-to-Desktop live interaction queued |
| Packaged Sidecar v2 contract | PASS | Pipe `hello`, unknown-field `extract`, then `shutdown` to packaged worker | `ready` with protocol v2, `INVALID_COMMAND`, clean exit 0 | None |
| Extension/Native Host package contract | PASS | `node desktop/scripts/extension-identity.mjs verify ...`; targeted package Node tests | Derived Extension ID matches bundled public key and `allowed_origins`; 11/11 package tests passed | Live Edge/Chrome registration/load queued |
| Frontend build | PASS | `npm run check --workspace desktop` | Vite production build completed (51 modules); existing dynamic/static import chunk warning only | None |
| Desktop Node full suite | BLOCKED | `npm test --workspace desktop` | One existing `wdio-tauri-service` process-termination test hit Windows `taskkill /T /F` `Access denied` in the restricted process-control environment; no application assertion failure was reported | Re-run outside restricted process-control environment; see `MANUAL-WIN-WDIO-TEARDOWN-01` |
| Full package GUI / browser status / registry lifecycle | BLOCKED | Computer Use inventory and launch attempt | Native app inventory empty; `computer.launch_app` unavailable in the Computer Use surface (`COMPUTER_USE_UNAVAILABLE`) | Complete `MANUAL-WIN-E5-01` through `MANUAL-WIN-FULL-01` in `windows-queue.md`; no unexecuted UI test is PASS |

#### Errors and unresolved issues

- Issue: packaged Full Desktop GUI, live HKCU registration and Edge/Chrome Extension connection have not been exercised.
- Classification: test/automation boundary; not a product failure based on current evidence.
- Blocker: `COMPUTER_USE_UNAVAILABLE`; native app launch surface is not exposed. HKCU/browser live operations were not performed.
- Reproduction and evidence: `cua.getState()` returned no native apps; launch attempt failed because `cua.computer.launch_app` was unavailable. Build/package/Sidecar probe and Windows Named Pipe unit tests were independently completed.
- Owner and next action: Cross-platform Owner reviews the shared UI wiring; Windows Owner runs the Manual Windows Validation Queue once native GUI automation or manual access is available.

### 2026-09-23 Windows E5–E7 follow-up validation

- Owner: Windows Platform Owner
- Source branch/revision: `feature/u7-desktop-production-integration` / fetched handoff `26f8c37ac995cea7fce6667fcdee2d2966e6f77b`
- Working tree included: documentation-only Linux handoff update on top of the E5–E7 implementation at `2dcae6c9a04175d6aa7e2478d421b497934a039e`; all local dependencies, logs, and validation artifacts preserved.
- Scope: complete remaining non-GUI validation that can be run in this Windows environment; retry Computer Use once; reuse implementation/package validations because the fetched commit changes only `docs/status/platform-handoff.md`.
- Environment/toolchain: Windows 11; Node test runner; Computer Use native app inventory unavailable.
- Canonical integration status: Windows follow-up evidence and queue updated; docs commit to be pushed with this record.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| WDIO teardown lifecycle | PASS | `node --test desktop/test/wdio-tauri-service.test.mjs` | 8 passed, 0 failed; `killTree` terminated the test-owned child process with normal process-control access | Close `MANUAL-WIN-WDIO-TEARDOWN-01` |
| E5–E7 implementation/build/package/worker probes | PASS (reused) | Reuse the checks recorded in the preceding E5–E7 round | Fetched `26f8c37` changes only the handoff document; implementation, dependencies, package inputs, and contracts are unchanged | No rerun needed under validation policy |
| Computer Use retry | BLOCKED — `COMPUTER_USE_UNAVAILABLE` | `cua.getState()`; reset session and retry once | First observation returned `apps: []`; retry returned `apps: []` and `Unable to load browser request-header policy` | Keep GUI/browser/registry items in Manual Windows Validation Queue |
| Full Desktop Node suite / full regression | NOT_RUN | Not rerun | Current fetched diff is documentation-only; the previously blocked teardown test was isolated and passed | No broad regression required for this batch |
| GUI, live HKCU registry, real browser Native Host connection, packaged Sidecar UI, cross-user ACL | BLOCKED — `COMPUTER_USE_UNAVAILABLE` | Native-window observation/interaction unavailable | No product failure inferred; previous build, static package checks, worker protocol probe, and pipe loopback do not establish these outcomes | Manual queue remains with Windows Platform Owner |

#### Errors and unresolved issues

- Issue: live E5–E7 GUI, Registry, browser, and cross-user ACL scenarios remain unverified.
- Classification: automation boundary; no Windows product failure established.
- Blocker: native GUI inventory unavailable after one bounded retry; second observation also reported browser request-header policy initialization failure.
- Reproduction and evidence: current Computer Use retry returned no native apps; see the `2026-09-23 E5–E7 / current-source Full package` queue entries.
- Owner and next action: Windows Platform Owner completes `MANUAL-WIN-E5-01`, `MANUAL-WIN-E6-01`, `MANUAL-WIN-E7-01`, and `MANUAL-WIN-FULL-01` when native GUI automation or manual access is available. No Linux-owned follow-up remains.

### 2026-09-24 P1/P2/P3 account batch Windows validation

- Owner: Windows Platform Owner
- Source branch/revision: `feature/u7-desktop-production-integration` / `dac0a153d03fa174c600435c87afabf476aded34`
- Working tree included: fetched remote handoff revision; tracked tree was clean before validation. Existing local dependencies, logs, prior artifacts, and ignored user directories were preserved. Build outputs used fresh paths below `validation-artifacts/windows-account-batch-20260924/`.
- Scope: affected Windows-target Rust tests; Sidecar, Desktop, Extension, schema, Vite and frozen worker checks; isolated Tauri/Native Host release builds; Full and Core package assembly; static package contract checks; bounded Computer Use attempt.
- Environment/toolchain: Windows x64, `x86_64-pc-windows-msvc`, Cargo offline; Node.js 24.19.0; Python 3.12.14 / PyInstaller 6.22.3; existing `.venv-windows-validation`; local gallery-dl and aria2 artifacts. Ruff was not available in the preserved virtual environment and was not installed.
- Canonical integration status: no Windows production-code change was required. Windows validation results apply to source revision `dac0a153`; this history entry and queue update are committed separately below.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| Affected Windows Rust modules | PASS | `cargo test -p xarchive-core -p xarchive-download -p xarchive-protocol -p xarchive-sidecar-supervisor -p xarchive-storage -p xarchive-telegram -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-account-batch-20260924/target --quiet` | 219 passed, 0 failed across Core 18, Desktop 102, Download 21, Transfer Driver 7, Protocol 19, Supervisor 6, Storage 34, Telegram 12; includes Windows Named Pipe loopback test | Live GUI/ACL/Host integration remains in queue |
| Native Host Windows tests | PASS | `cargo test -p xarchive-native-host --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-account-batch-20260924/target --quiet` | 8 passed, 0 failed | Live registration and browser connection remain queued |
| Sidecar Python tests | PASS | `.venv-windows-validation\Scripts\python.exe -m pytest sidecar\tests -p no:cacheprovider --basetemp validation-artifacts\windows-account-batch-20260924\pytest-temp` with `PYTHONDONTWRITEBYTECODE=1` | 32 passed, 0 failed | Ruff `NOT RUN`: executable not installed in the preserved venv |
| Desktop Node tests | PASS | `npm test --workspace desktop` | 92 passed, 0 failed | No full regression rerun; relevant workspace suite passed |
| Extension Node tests | PASS | `npm test --workspace extension` | 21 passed, 0 failed | Live Edge/Chrome load remains queued |
| Vite frontend build | PASS | `npm run check --workspace desktop -- --outDir ../validation-artifacts/windows-account-batch-20260924/desktop-dist` | 52 modules built into isolated output; existing `desktop/dist` preserved | Known dynamic/static Tauri API import warning only |
| Modified schema JSON | PASS | Node `JSON.parse` over all three modified schema files | All three parsed | None |
| Current-source frozen Sidecar | PASS | PyInstaller `sidecar/pyinstaller/xarchive-downloader.spec` with isolated dist/work paths; run packaged `--help`, protocol-v2 handshake, invalid field and shutdown probes | `hello`/`ready` included `account_discovery`; unknown field returned `INVALID_COMMAND`; shutdown exit 0. Probe: `validation-artifacts/windows-account-batch-20260924/sidecar-probe.json`. Worker SHA-256: `CC7B65F3CD7CA32441F1FFD9291B290D7F7523E0CE7A394B3B3B9E5FD51EF114` | Real account/gallery-dl extraction remains blocked pending controlled session |
| Isolated Tauri release build | PASS | `node ..\node_modules\@tauri-apps\cli\tauri.js build --config ..\validation-artifacts\windows-account-batch-20260924\tauri-validation.json --target x86_64-pc-windows-msvc --no-bundle` with isolated Cargo target and frontend dist | `validation-artifacts/windows-account-batch-20260924/target/x86_64-pc-windows-msvc/release/xarchive-desktop.exe`, 19,002,880 bytes | GUI launch/first-run remains blocked by Computer Use surface |
| Isolated Native Host release build | PASS | `cargo build -p xarchive-native-host --release --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-account-batch-20260924/target` | Release executable built in isolated target | Live HKCU install/repair/unregister remains queued |
| Full portable package | PASS (assembly/static contract only) | `node desktop/scripts/build-portable-windows.mjs` using isolated app, Native Host and worker paths, derived Extension ID `iaajefkoanbkleojofoadeakelihbjne`, output `validation-artifacts/windows-account-batch-20260924/full-package-r1` | Required app, worker, `python312.dll`, gallery-dl, Extension and Native Host files present; manifest, Extension ID, Native Host executable path and allowed origin match | No package GUI launch or Edge/Chrome registration/connection claim |
| Core portable package | PASS (assembly/static boundary only) | Same package script with `PORTABLE_PACKAGE_TYPE=core` and output `validation-artifacts/windows-account-batch-20260924/core-package` | Core manifest says `package_type=core`, `gallery_dl_bundled=false`, Extension not bundled, Native Host null | No launch claim |
| Computer Use / account GUI | BLOCKED — `COMPUTER_USE_UNAVAILABLE` | Inventory showed `apps: []` and Edge browser tabs; attempted `cua.computer.launch_app` for isolated Full package | `launch_app` was undefined in the available Windows Computer Use surface. No native app was opened. No account request or registry change was attempted. This is not a product failure. | `MANUAL-WIN-BATCH-01`, `02`, `04`, `MANUAL-WIN-E5-01` through `E7-01`, and `FULL-01` remain in `windows-queue.md` |
| Real account discovery/archive, credential redaction, pause/resume/retry/restart, Windows filesystem fault/ACL/reparse, release signing/upload | BLOCKED / NOT_RUN | Not attempted | Requires a controlled account/browser session and GUI for account cases; requires controlled filesystem fault fixtures and actual signing/release target for the other cases | Keep `MANUAL-WIN-BATCH-02` through `05` open; no PASS inferred from synthetic tests or package builds |
| Full regression | NOT_RUN | Not run | Risk-based scope covered changed crates and affected Desktop/Extension/Sidecar modules, package build and worker contract. No dependency overhaul/release operation was part of this validation batch. | Reassess if remaining platform evidence reveals a shared defect |

#### Errors and unresolved issues

- Issue: native GUI, live browser/Native Host/Registry, real controlled-account extraction, and Windows filesystem fault scenarios are not validated.
- Classification: environment/automation prerequisites; no Windows-owned or shared-contract product failure was established.
- Blocker: Computer Use returned no native apps and exposed no `launch_app`; real account credentials/session, signing, and release target were not supplied or used.
- Reproduction and evidence: Computer Use inventory and launch attempt recorded above; static package evidence and isolated build outputs are under `validation-artifacts/windows-account-batch-20260924/`.
- Owner and next action: Windows Platform Owner completes the open queue when manual GUI access, controlled account data, and applicable release fixtures are available. `CROSS_PLATFORM_CHANGE_REQUIRED`: none. `CROSS_PLATFORM_REVIEW_REQUIRED`: none.

### 2026-09-24 Full package manual GUI and account-batch follow-up

- Owner: Windows Platform Owner
- Source branch/revision: `feature/u7-desktop-production-integration` / `dac0a153d03fa174c600435c87afabf476aded34`.
- Package: `validation-artifacts/windows-account-batch-20260924/full-package-r1` (Full package assembled from the source revision above).
- Test date: 2026-09-24. Evidence consists of user-provided screenshots and observations; screenshots are not copied into the repository.
- Privacy: account names, Tweet IDs, and batch identifiers are intentionally omitted/redacted. No account-identifying values are reproduced in this record.
- Scope: manual launch/exit, Sidecar state, Extension/Native Host connection, archive-button task propagation, account-batch display and pause/cancel behavior, and path handling.
- Environment limitations: Windows build and Edge/WebView2 exact versions were not recorded in this manual follow-up. No successful extraction/download occurred, so downstream file and batch acceptance could not be exercised.
- Source changes: none. This manual report supplements, and does not replace, the automated results already recorded for `dac0a153`.

| Item | Status | Observation | Follow-up / limits |
|---|---|---|---|
| Application launch and clean exit | PASS | Application launched and closed normally. After closing, `Get-CimInstance Win32_Process -Filter "Name = 'xarchive-desktop.exe'"` returned no remaining application process. | This is a user-observed manual result for this package. |
| Sidecar start, stop, and service indicator | PASS | Sidecar started and stopped; the lower-left service status indicator reflected the service state. The UI showed the Sidecar ready state during the successful run. | Does not establish successful real account extraction or download. |
| Account archive page rendering | PASS | Account archive page opened and rendered its controls and batch panel. | Batch discovery/processing results are separately recorded below. |
| Extension initial load and recognition | PASS | Edge loaded the Extension and the application initially recognized it as connected. | Later post-restart connection recovery failed; see below. |
| Extension archive action and task visibility | FAIL | Clicking the archive action on a Tweet did not immediately show a task in the main UI. Refreshing the main UI made the task appear, after which it showed download failure. The X page action changed to a retry state. | Delayed propagation and the failed download require diagnosis. Account name and Tweet ID omitted. |
| Download after manually starting aria2 | FAIL | Manually starting an aria2 process did not make the failed task download successfully. The settings page also displayed aria2 as not detected in a screenshot. | The UI state is a diagnostic clue only; this record does not assert aria2 detection as the root cause. No downloaded file was produced for inspection. |
| Extension reconnect after application restart | FAIL | After restarting the application, the UI showed Extension disconnected. Refresh, Native Host repair, unregister, and repair again did not restore the connection. | Explicit registry-path/value inspection and Native Host pipe-request tracing were not performed. |
| Account-batch discovery progress | FAIL | A batch remained at zero candidates and zero pending submissions for an extended period. The user then paused it; discovery still displayed `RUNNING` for about 30 seconds, after which the user manually cancelled the batch. | Batch identifier omitted. Resume, retry, and restart-recovery behavior were not tested. The observed delay is not a measured service-level threshold. |
| Chinese path handling | PASS | Chinese path was used successfully. | Long paths and cross-volume operations were not tested. |
| Downloaded file content, size, and checksum | NOT RUN | No successful download was produced. | Requires a working extraction/download flow and a controlled expected output. |
| Batch records and successful multi-item processing | NOT RUN | No normal batch completion was possible because discovery/download did not progress successfully. | Requires working account discovery and download. |
| File lock, abnormal exit, staging cleanup | NOT RUN | Not exercised. | Requires controlled Windows filesystem/process fixtures. |
| Credential/identifier leakage inspection | NOT RUN | No systematic log/database/package inspection was performed for this manual follow-up. | Any future evidence must continue to redact account names and Tweet IDs. |
| Native Host registry assertions and pipe ACL/cross-user access | NOT RUN | Repair actions were attempted, but registry values, exact executable registration, pipe requests, and cross-user ACL behavior were not inspected. | Requires targeted registry/pipe verification. |
| Signing and release gate | NOT RUN | No signing, release gate, or upload was performed. | Requires the release signing environment and release target. |
| Other filesystem acceptance | NOT RUN | Long path, cross-volume move, file lock, abnormal exit, and staging cleanup were not tested. | Keep each as a separate acceptance item. |

#### Triage and ownership

- `CROSS_PLATFORM_CHANGE_REQUIRED`: investigate account discovery/candidate progression, delayed task visibility from Extension to Desktop, and pause-to-terminal-state propagation. The evidence identifies failures at shared workflow boundaries, but does not establish a root cause or prove that a shared contract change is required; Linux should triage and decide the implementation owner after diagnosis.
- Windows-owned follow-up remains: investigate and retest Native Host/Extension reconnection after Desktop restart, including Windows registration and process/pipe lifecycle.
- Existing automated PASS results at source revision `dac0a153` remain valid for their recorded test scope; they do not override the manual FAIL results or imply successful account extraction/download.

### 2026-09-24 Windows batch revalidation at Linux handoff `7ed02b5`

- Owner: Windows Platform Owner
- Branch/source revision: `feature/u7-desktop-production-integration` / `7ed02b5e94e17171e26ae75000b555932e166216`.
- Working tree: canonical Windows branch fast-forwarded to the fetched handoff revision; tracked files were clean before this documentation closeout. Existing local dependencies, logs, manual-validation files, and prior artifacts were preserved. New generated output is isolated under `validation-artifacts/windows-batch-revalidation-7ed02b5/`.
- Scope: current-source Windows-target affected Rust tests, Native Host tests/build, Sidecar discovery tests, targeted Desktop/Extension tests, Vite build, frozen worker protocol probe, Tauri/Native Host release builds, Full package assembly/static contract, fixed-runtime dashboard WDIO attempt, and bounded Computer Use retry.
- Environment: Windows x64, `x86_64-pc-windows-msvc`, Cargo offline, Node 24.19.0, Python 3.12.14 / PyInstaller 6.22.3. Fixed WebView2 Runtime `153.0.4234.48`, EdgeDriver `153.0.4234.46`, and tauri-driver `2.0.6` were available.
- Implementation: no Windows production-code changes. Source/validation revision is `7ed02b5e94e17171e26ae75000b555932e166216`; this documentation closeout is committed separately.

| Item | Status | Command/steps | Evidence | Follow-up |
|---|---|---|---|---|
| Affected Windows Rust modules | PASS | `cargo test -p xarchive-download -p xarchive-storage -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --quiet` using an isolated target | 171 tests passed across Desktop (106), Download/Transfer Driver (29), and Storage (36) | Current-source real account/download GUI checks remain open |
| Native Host Windows tests and release build | PASS | `cargo test -p xarchive-native-host --target x86_64-pc-windows-msvc --offline --quiet`; release build in isolated target | 8 tests passed; release executable built | Live HKCU registration and restart reconnect remain queued |
| Sidecar discovery tests | PASS | `.venv-windows-validation\Scripts\python.exe -m pytest sidecar\tests\test_discovery.py -p no:cacheprovider --basetemp validation-artifacts\windows-batch-revalidation-7ed02b5\pytest-temp` | 8 passed. Initial invocation only lacked the basetemp parent; after creating it, the same test passed. | Real account discovery remains queued |
| Desktop targeted UI wiring tests | PASS | `node --test desktop/test/ui-wiring.test.mjs` | 20 passed | Full Desktop Node suite did not terminate after about two minutes and was interrupted; record as `BLOCKED`, not a passing suite |
| Extension tests | PASS | `npm test --workspace extension` | 21 passed | Real Edge behavior remains queued |
| Vite frontend build | PASS | `npm run check --workspace desktop -- --outDir ../validation-artifacts/windows-batch-revalidation-7ed02b5/desktop-dist` | 52 modules built; existing Tauri dynamic/static import warning | No claim of native GUI readiness from this build |
| Current-source frozen Sidecar | PASS | PyInstaller build in isolated dist/work dirs; `--help`, v2 `hello`/`ready`, unknown-field and shutdown probes | `account_discovery` capability present; invalid field returned `INVALID_COMMAND`; shutdown exit 0. Probe SHA-256: `d993cb9bce18760429a3093ff85754b9ce2273a20157f7e4fda428fcb4258bc0` | Real extraction and aria2 transfer not established |
| Isolated Tauri release build | PASS | Windows x64 Tauri release build with validation config and isolated Cargo target | `validation-artifacts/windows-batch-revalidation-7ed02b5/target/x86_64-pc-windows-msvc/release/xarchive-desktop.exe` built (19,017,216 bytes) | GUI launch remains unverified for this revision |
| Current-source Full package | PASS (assembly/static contract only) | Assembled to `validation-artifacts/windows-batch-revalidation-7ed02b5/full-package` | App, worker, `python312.dll`, gallery-dl, Extension and Native Host present; package manifest and host allowed origin match Extension ID `iaajefkoanbkleojofoadeakelihbjne` | No browser registration, task, download, or GUI PASS inferred |
| Fixed-runtime dashboard WDIO | BLOCKED (automation/environment) | `npm run test:e2e:windows --workspace desktop`, with WebView2 `153.0.4234.48`, EdgeDriver `153.0.4234.46`, tauri-driver `2.0.6`, and the newly built Full package | Worker failed before application launch: Node `uv_os_get_passwd returned ENOMEM`; spec retry failed identically. This is not an app assertion or product failure. | Retry when Node/WDIO worker environment is healthy; preserve pinned runtime/driver pairing |
| Computer Use native GUI | BLOCKED — `COMPUTER_USE_UNAVAILABLE` | Bounded native-app inventory and launch attempt | No native apps available; launch API unavailable. No Edge or XArchive window was controlled in this batch. | Keep `MANUAL-WIN-BATCH-REVAL-01`–`06` open |
| Full Desktop Node suite | BLOCKED (runner did not terminate) | `npm test --workspace desktop`; interrupted after about two minutes without a completion summary | Targeted 20/20 UI wiring tests passed separately; full-suite result incomplete | Re-run the workspace suite in a healthy runner; do not infer PASS |
| Real account discovery/download, package migration, reconnect, filesystem fault, signing/release gate | NOT RUN / BLOCKED | Not attempted or unavailable in this batch | Requires controlled browser/account, successful real download, manual GUI/registry access, filesystem fault fixtures, or signing/release target | Continue from `windows-queue.md`; prior manual FAIL remains bound to package/source `dac0a153`, not upgraded to current-source result |

#### Current triage and ownership

- Prior user-observed manual results remain evidence for Full package `full-package-r1` at `dac0a153`: task visible only after Dashboard refresh then failed; manually starting aria2 did not produce a download; Extension did not reconnect after Desktop restart; discovery remained at zero and still showed `RUNNING` for about 30 seconds after pause before user cancellation. Do not project these results onto the new source revision without retest.
- Current-source account task visibility, candidate progression, pause/cancel GUI state, aria2 transfer, v5 database copy migration, and Extension reconnect are `NOT RUN` or `BLOCKED` in this batch. Automated module tests and package assembly do not resolve those outcomes.
- `CROSS_PLATFORM_CHANGE_REQUIRED`: none newly established against `7ed02b5`; prior shared changes require manual revalidation. `CROSS_PLATFORM_REVIEW_REQUIRED`: none. Windows reconnect remains Windows-owned diagnosis.
- Next owner: Windows Platform Owner, to complete the open manual queue when native GUI access and controlled inputs are available.

### 2026-09-24 targeted WDIO and aria2 revalidation

- Owner: Windows Platform Owner
- Branch/source input: `feature/u7-desktop-production-integration`, source `7ed02b5e94e17171e26ae75000b555932e166216`; Windows WDIO/test changes were made after this input revision. Validation ran on the same current working tree as those changes.
- Implementation/validation revision: `e36705d` (`fix(windows): pass pinned runtime to WDIO service`).
- Scope: resolve the fixed-runtime Full-package dashboard startup failure, execute the complete Desktop Node suite, and validate the local aria2 runtime/transfer path without using account credentials or external services.
- Environment: Windows x64, Node `24.19.0`; fixed WebView2 Runtime `153.0.4234.48`; EdgeDriver `153.0.4234.46`; tauri-driver `2.0.6`; local aria2 `1.37.0` at `aria2/1.37.0/aria2-1.37.0-win-64bit-build1/aria2c.exe`.
- Windows-owned implementation: `desktop/wdio.conf.mjs` now forwards the fixed WebView2 folder to the Tauri service `env` option and honors an explicit `TAURI_DRIVER_PATH`; `desktop/test/ui-wiring.test.mjs` asserts both settings. This makes the configured driver/runtime pair reproducible and prevents the service from silently using another driver/runtime source.

| Item | Status | Command/steps | Evidence | Limits / follow-up |
|---|---|---|---|---|
| Full-package dashboard startup | PASS | `npm run test:e2e:windows --workspace desktop` with `WDIO_APP_BINARY` set to the assembled Full package; pinned WebView2, EdgeDriver, and tauri-driver paths; downloads disabled | Tauri smoke opened `http://tauri.localhost/`; actual session reported `webview2 153.0.4234.48`; 3/3 tests passed. Evidence under `validation-artifacts/windows-batch-revalidation-7ed02b5/wdio-config-fix` | This covers startup/readiness/dashboard shell, not Extension, real account submission, or download behavior. |
| Desktop full Node suite | PASS | `npm test --workspace desktop` in normal Windows process environment | 93 passed, 0 failed, 0 skipped. An earlier restricted execution had `uv_os_get_passwd returned ENOMEM`; the normal Windows run completed, confirming that was an execution-environment restriction. | WDIO GUI suite is separately reported above. |
| Windows-target aria2 transfer crate | PASS | `cargo test -p xarchive-download --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-batch-revalidation-7ed02b5/target-aria2 --quiet` | 22 unit tests and 7 integration tests passed. Cargo printed a non-fatal incremental-cache access warning after tests; command exit was 0. | Tests use the crate's fake backend; live binary check is recorded separately. |
| Local aria2 binary/RPC/range download | PASS | Isolated `aria2c.exe` 1.37.0 with loopback-only JSON-RPC and HTTP Range fixture; `getVersion`, `addUri`, `pause`, `unpause`, `tellStatus`, shutdown; Chinese/space output filename | 2 MiB completed; output SHA-256 matched fixture `4695B93998D2BE3CAE3354AD1D00A054ABC3DE0241A6FA2E632F7824108F45A2`; aria2 exited 0. Evidence under `validation-artifacts/windows-batch-revalidation-7ed02b5/aria2-runtime-20260924-r2/summary.json` | Does not prove Desktop's managed supervisor, account extraction, URL refresh, staging/commit, or app archive state machine. The source-tree `aria2` directory is not the Full package's default `sidecar/aria2` path; select/save its executable in Settings or package it at the expected relative location. A manually launched aria2 process does not substitute for the app's per-job loopback RPC child. |
| Desktop-managed real archive transfer | NOT RUN | No account/Extension submission or controlled extracted media URL was available for this run | No user account, external service, or media URL was used | Keep `MANUAL-WIN-BATCH-REVAL-04` open for a controlled end-to-end package run. |

The earlier fixed-runtime attempt that remained on `data:,` was caused by WDIO configuration not explicitly forwarding the runtime folder and driver path; it is superseded for dashboard startup by the passing rerun above. The earlier `uv_os_get_passwd` failure was a restricted-runner issue, not an app assertion. Other manual account-batch failures and Extension reconnect results remain bound to `full-package-r1` at `dac0a153` until retested.

### 2026-09-24 user-reported Extension and account-batch revalidation

- Owner: Windows Platform Owner (manual observations supplied by the user).
- Date: 2026-09-24.
- Artifact provenance: the user described the tested build as the current Extension/account-batch build, but did not include its exact source revision or package SHA-256. Treat the results below as user-reported observations; bind the artifact identity before using them as release acceptance evidence.
- Privacy: account names, Tweet IDs, batch IDs, and account URLs are omitted. Do not recover identifiers from screenshots into this history.

| Item | Result | Evidence / limit |
|---|---|---|
| Extension load and page action | PASS (user report) | Extension loaded; page action appeared and was clickable. |
| Extension-to-Dashboard submission | PASS for prompt visibility and idempotency; FAIL for execution | A task appeared in about 0.5 seconds without Dashboard refresh; repeating the same Tweet did not create a duplicate, and a different Tweet created another task. The tasks failed with `EXECUTOR_UNAVAILABLE: job executor is closed`. This is not a successful archive or download. |
| Account discovery | FAIL | Two tested accounts produced no candidates/tasks. The UI showed an active batch but zero candidates, submitted, completed, and failed entries. Pause and cancel controls worked in the observed UI. No SQLite timing or worker/Sidecar diagnostics were supplied. |
| v5-to-v6 package migration | PASS (user report) | User reports the migration test completed normally. No database hashes, row-count comparison, or foreign-key output were supplied. |
| Edge Extension refresh/reconnect | FAIL (user report) | Refresh in Settings did not reconnect Extension; UI showed Extension disconnected while Sidecar was connected. Further test is deferred at the user's request until Extension refactor. |
| Staging and Windows filesystem/recovery | NOT RUN | Long path, cross-volume, lock, abnormal-exit, recovery, and staging cleanup were skipped. User's “expected PASS” is recorded as an expectation only, not a result. |
| Application-managed aria2/extraction | BLOCKED / NOT RUN | The executor error prevented successful task execution. No successful app-managed aria2 transfer or file-integrity evidence was provided. Earlier local aria2 fixture PASS remains component-only. |
| Named Pipe and registry details | NOT RUN | No pipe endpoint/ACL, HKCU values, manifest path/origin, or unregister cleanup evidence was supplied. |

#### Routing

- `CROSS_PLATFORM_CHANGE_REQUIRED`: route job executor closure and non-producing account discovery to the Linux/Cross-platform Owner for diagnosis. This is an ownership classification for shared behavior; the report does not establish root cause or a required contract change.
- User recommends temporarily disabling account discovery pending further implementation. This is recorded as a recommendation only; no code or feature gate was changed in this validation entry.
- Windows Extension reconnect remains an observed Windows integration failure; resume that queue after Extension refactor.

### 2026-09-24 authenticated WebSocket / Extension UI Windows batch

- Owner: Windows Platform Owner.
- Branch: `feature/u7-desktop-production-integration`.
- Source and validation input revision: `084354a5ca433b52372aca4bc70ac5fc544104fc`.
- Windows implementation: none; no Windows production-source change was required.
- Working tree: fast-forwarded from `553e3788467041ef43ac5657c969064ce45d016c` to the fetched handoff. Tracked source was clean before this record; local dependencies, logs, manual-validation files, and existing validation artifacts were preserved. New output is under `validation-artifacts/windows-ws-084354a/`.
- Environment: Windows 11 x64; Node `24.19.0`; Python `3.12.14` / PyInstaller `6.22.3`; WebView2 Fixed Runtime `153.0.4234.48`; EdgeDriver `153.0.4234.46`; tauri-driver `2.0.6`; extension identity derived from the committed manifest key.

| Item | Status | Command / evidence | Limits |
|---|---|---|---|
| Affected Windows Rust targets | `PASS` | `cargo test -p xarchive-core -p xarchive-protocol -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-ws-084354a/target --quiet`; 146 passed (18 core, 19 protocol, 109 desktop) | Non-fatal linker/incremental-cache access warnings; exit code 0. |
| Desktop Node suite | `PASS` | `npm test --workspace desktop` with normal process-control permission; 93 passed, 0 failed, 0 skipped | The first restricted attempt hit the `killTree` child-process assertion and hung because process control was denied. The isolated teardown test then passed 8/8 and the full suite passed 93/93 when rerun with normal process control. |
| Extension tests | `PASS` | `npm test --workspace extension`; 25 passed | No real MV3 browser session. |
| Sidecar discovery/extraction affected tests | `PASS` | `.venv-windows-validation\Scripts\python.exe -m pytest sidecar\tests\test_discovery.py sidecar\tests\test_extraction_only.py -p no:cacheprovider --basetemp validation-artifacts\windows-ws-084354a\pytest-temp`; 15 passed | No real account or external media source. |
| Desktop production Vite build | `PASS` | `npm run check --workspace desktop`; 52 modules transformed | Existing Tauri API dynamic/static import advisory remains. |
| Extension ZIP package | `PASS` (inventory/extraction) | `validation-artifacts/windows-ws-084354a/XArchive-v0.0.0-pre.1-extension.zip`; 16,642 bytes; 12-file plan verified against extracted ZIP; SHA-256 `779F54F69528EFEE34EDD6F7FF8D6D412FEC7E5FEC9EC4BB008E587049944820` | Local synthetic pre-release tag only; no release publication. Loading the ZIP in Edge/Chrome remains `NOT RUN`. |
| Full package assembly/inventory | `PASS` (static contract) | Isolated current-source package at `validation-artifacts/windows-ws-084354a/full-package`; package type `full`, 11 required paths present, Extension directory 15 files. App SHA-256 `6B5284DBDD1BB843BCF49DEC296024F8391134B261D65D3CB2FE5C64303133EF`; worker SHA-256 `8F4FBBDB5AC084FCD73B5DD899695F67AEFC8540695FD8D87A0515C121EAEAD4`. | Fresh Tauri CLI release build, Native Host, and PyInstaller worker were isolated under this artifact root. aria2 is optional and was not included. No real task or transfer implied. |
| Full-package dashboard startup/readiness | `PASS` | `npm run test:e2e:windows --workspace desktop`, `WDIO_APP_BINARY` set to the isolated Full package; PATH prefixed with pinned EdgeDriver directory; pinned WebView2 and tauri-driver paths. Session URL `http://tauri.localhost/`; 3/3 passed. | A first attempt set `EDGEDRIVER_PATH` alone, but the service discovers EdgeDriver from PATH; after correcting PATH, it started. A direct Cargo release build without the Tauri CLI frontend step loaded the unused dev URL `localhost:1420` and failed; rebuilt via `tauri build --no-bundle --ci` with isolated `CARGO_TARGET_DIR`, after which the smoke passed. These setup failures are superseded, not source regressions. |
| WQ-WS-01 Edge/Chrome load and permissions | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | Two bounded inventory observations returned `apps: []`; no supported native-app launch target or isolated browser profile was available. Existing Edge profile contains user tabs and was not manipulated. | Re-run with a disposable Edge/Chrome profile; confirm MV3 worker, popup/options and permissions. |
| WQ-WS-02 live token pairing/auth | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | No controlled Desktop/browser pairing session or synthetic local request fixture was available. | Test wrong token rejection before valid token, authenticated status/archive requests, and token redaction. |
| WQ-WS-03 Desktop/Service Worker lifecycle | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | No controlled native GUI + browser profile to interrupt and restore a pending request. | Exercise Desktop/worker restart, pending cleanup, bounded reconnect, executor replacement, and Native fallback without replay. |
| WQ-WS-04 popup/options scaling and keyboard | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | Popup/options were not opened in a controlled browser profile. | Test 100/125/150% scaling, keyboard/focus, long status text, overflow, and settings save failure. |
| WQ-WS-05 packaging | `PASS` for ZIP/full assembly and static inventory; ZIP browser load `NOT RUN` | ZIP inventory/extraction passed; current Full package app/worker/Extension/Native Host and manifest inventory passed; dashboard smoke passed. | Package/static PASS is not Edge/Chrome Extension-load or live pairing acceptance. |
| Signing/release gate | `NOT RUN` | No signing credentials or release workflow invocation. | Requires release environment and target. |

#### Classification and next owner

- At the time of the automated validation record, `CROSS_PLATFORM_CHANGE_REQUIRED` had not been identified from automation/package evidence; no shared implementation was changed.
- The later manual follow-up below reports a live connection failure and establishes a cross-platform diagnosis follow-up; it does not isolate the failure to a code defect or a specific component.
- The user's previous account-discovery/executor-closure/reconnect results remain tied to their separately recorded artifact/revision; this dashboard smoke does not upgrade or clear them.

### 2026-09-24 Full package manual WebSocket follow-up

- Artifact: `validation-artifacts/windows-ws-084354a/full-package`; Extension directory: `validation-artifacts/windows-ws-084354a/full-package/extension`.
- Source/validation input: `084354a5ca433b52372aca4bc70ac5fc544104fc` on `feature/u7-desktop-production-integration`; this entry records user-supplied manual results against that previously assembled package. The report has not been independently reproduced in this turn.
- Privacy: account names, Tweet IDs, Extension identity, and token are omitted. The supplied options screenshot masks the token; no token value was recorded.

| Check | Result | Evidence and scope |
|---|---|---|
| Full package app startup, close, and content display | `PASS` (user-reported) | User confirmed normal startup/exit and normal main content display. |
| Extension load and basic settings/popup display | `PASS` (user-reported) | Extension from the Full package's `extension` directory loaded; settings/options and popup were displayed. No assertion is made for keyboard, scaling, or ZIP loading. |
| Extension/Desktop WebSocket integration | `FAIL` (user-reported) | Options displayed port `17321` and a masked pairing token while attempting to connect; the status section showed WebSocket disconnected. Popup reported Desktop disconnected and WebSocket connection closed. Desktop's Extension view reported no Native Host request and no active request. This demonstrates failed user-visible connection, not the underlying cause. |
| Reconnect/pending-request lifecycle | `NOT RUN` | No controlled pending request, Service Worker restart, network interruption, or reconnection sequence was reported. |
| ZIP installation, scale/keyboard accessibility | `NOT RUN` | The evidence covers the unpacked Full package Extension directory and basic rendering only. |

- Classification: `CROSS_PLATFORM_CHANGE_REQUIRED` for diagnosis and correction of the shared Desktop/Extension WebSocket integration. The Windows evidence establishes the failure on this package but does not isolate the responsible component; no Windows-specific root cause is established.
- No application or Extension source was changed in this follow-up. This record is based on the user's 2026-09-24 report and screenshots; no token, account name, Tweet ID, or browser identity has been copied into the repository.

### 2026-09-24 Extension ZIP load and token pairing retry

- Source/validation input remains `084354a5ca433b52372aca4bc70ac5fc544104fc`; the user report concerns the Extension ZIP generated in that batch (`validation-artifacts/windows-ws-084354a/XArchive-v0.0.0-pre.1-extension.zip`).
- `PASS` (user-reported): ZIP loaded in the browser and the Extension UI appeared.
- `FAIL` (user-reported): the Desktop-generated pairing token did not connect when transferred by either the automatic copy action or manual entry. The Popup screenshot reports Desktop `未配置`, WebSocket `未配置 · 端口 17321`, WebSocket closed, and Native Messaging fallback.
- `NOT RUN`: authenticated/unauthenticated response behavior, verification that the token persisted in Extension storage, Desktop receipt of a WebSocket handshake, and archive request delivery. The visible disconnected/unconfigured state does not establish token rejection or identify the faulty component.
- Privacy: no token, account name, Tweet ID, or Extension identity was copied from the screenshot. No independent GUI reproduction was performed.
- Classification remains `CROSS_PLATFORM_CHANGE_REQUIRED` for shared-path diagnosis. First isolate configuration persistence, browser WebSocket connection/handshake, listener reachability, and token authentication before selecting a code owner or claiming an auth defect.

### 2026-09-24 Full package WebSocket listener and authentication-frame follow-up

- Artifact/source: same `validation-artifacts/windows-ws-084354a/full-package` run and `084354a5ca433b52372aca4bc70ac5fc544104fc` input. This records later, user-supplied evidence; it does not erase the earlier `ERR_CONNECTION_REFUSED` snapshot or the ZIP run's UI state. Observations are point-in-time and were not independently reproduced.
- Record scope: account names, Tweet IDs, pairing-token values, and Extension identity are not reproduced in this historical note. The screenshot associated with this earlier observation masked the token. Per the user's clarification, token privacy/security review is skipped and is not classified as a finding.

| Check | Result | Evidence and limit |
|---|---|---|
| Desktop loopback listener reachability | `PASS` (user-reported, point-in-time) | PowerShell showed a listener on `127.0.0.1:17321` owned by the Full package `xarchive-desktop` process; `Test-NetConnection 127.0.0.1 -Port 17321` returned `True`. This does not establish WebSocket authentication or request handling. |
| WebSocket HTTP upgrade | `PASS` (user-supplied DevTools evidence, one selected request) | The selected `ws://127.0.0.1:17321/` request returned `101 Switching Protocols`. This proves that request reached a WebSocket endpoint and upgraded; it does not prove auth success or that every retry reached the same process. |
| Extension authentication and connection state | `FAIL` (user-reported) | Extension settings remained enabled on port `17321` with a nonempty masked token, while WebSocket status remained disconnected; Desktop showed Native Host requests arriving, but WebSocket unauthenticated and Desktop observation `not_loaded`. DevTools showed an outbound `authenticate` frame and no inbound server response in the supplied capture. The user reports that no requests received a response. |
| Browser-side connection errors | `FAIL` (user-supplied console evidence) | Captures include `WebSocket is closed before the connection is established`, `ERR_SOCKET_NOT_CONNECTED`, and earlier `ERR_CONNECTION_REFUSED`. Later listener/TCP and one `101` observation show that reachability differed across attempts or times; the earlier refusal alone does not describe the later state. |
| Native Messaging archive-button path | `PASS` (limited, user-reported) | Clicking the X-page archive button created a task and a Native Host request was visible. This only verifies task submission/Native Host receipt; task execution/download is not established by this observation. |
| Authentication response / request completion | `FAIL` (user-reported no response; protocol outcome `NOT RUN`) | No inbound `authentication_response` was visible in the supplied Messages capture and no request response was observed. Therefore the token is not proven rejected or accepted, and the failure point (client close, server receive/response, or other lifecycle race) remains unknown. |

- Diagnosis boundary: these observations make a permanently absent listener an insufficient explanation for the later capture. An outbound auth frame plus missing inbound response localizes the open question to after the browser sent the frame, but does not show whether Desktop received or processed it. Repeated reconnect/close behavior could be consistent with a client lifecycle race, but that remains a hypothesis, not a verified cause. Next diagnostics should correlate per-connection server accept/auth/close events with a client attempt, without recording the token, and distinguish client-initiated close from server failure to respond.
- Classification: `CROSS_PLATFORM_CHANGE_REQUIRED`; no component-level root cause or fix is established. Native Messaging and WebSocket evidence are separate; a working Native Host request does not imply WebSocket readiness.
- No code change or automated GUI reproduction was performed for this documentation update.

### 2026-09-25 Windows revalidation of authenticated WebSocket handoff

- Owner: Windows Platform Owner.
- Branch: `feature/u7-desktop-production-integration`.
- Handoff/source validation input: `98f16845b9e37f0dea419e7bffc890b1be3d2063` (includes shared implementation `de46a8ee7ac937f9ed64db5f16b9e8c4cb1c178d`).
- Windows implementation revision: none; the current diff was the shared auth-timeout/diagnostics change. This record is committed separately as Windows validation documentation.
- Working tree: fetched `origin` and fast-forwarded the clean tracked tree from `2d067cbf0412a8819fb52b14895f00e7c0b46bd2` to the handoff. Existing untracked dependencies, logs, local aria2/gallery-dl assets, manual-validation files, and artifacts were preserved. New build/test output is isolated under `validation-artifacts/windows-ws-de46a8e/`.
- Environment: Windows 10 Pro for Workstations; Node `24.19.0`, npm `11.17.0`, Rust/Cargo `1.98.0`, Python `3.12.14`, Windows target `x86_64-pc-windows-msvc`.

| Item | Status | Command / evidence | Limits |
|---|---|---|---|
| WebSocket-focused Windows Rust tests | `PASS` | `cargo test -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-ws-de46a8e/target websocket_transport::tests -- --nocapture`; 4 passed, 107 filtered. Covers defaults, wrong token, valid-token authentication response, and authenticated `query_status` route. | Automated Windows-target tests do not exercise a real browser or packaged runtime. Nonfatal linker/incremental-cache access warnings; exit code 0. |
| Extension Node suite | `PASS` | `npm test --workspace extension`; 26/26 passed. | No real MV3 browser session. |
| Desktop Node suite | `PASS` | `npm test --workspace desktop`; 93/93 passed when rerun with normal process-control permissions. | First restricted attempt failed/hung at child-process teardown because `killTree` was denied; successful rerun supersedes the sandbox artifact. |
| Windows Desktop crate suite | `PASS` | `cargo test -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-ws-de46a8e/target --quiet`; 111 passed, 0 failed with `PYTHON` set to `.venv-windows-validation\Scripts\python.exe`. | An initial run without `PYTHON` had 3 batch tests fail because the Sidecar did not start; the project Python prerequisite was then set and all 111 tests passed. This was setup, not a reproduced product failure. |
| Formatting and frontend checks | `PASS` | `cargo fmt --all -- --check`; `npm run check`; Desktop Vite transformed 52 modules and Extension syntax checks passed. | Existing nonfatal Tauri API dynamic/static import advisory remains. |
| Windows Tauri release executable | `PASS` | `npm run build:tauri --workspace desktop -- --no-bundle --ci` with isolated `CARGO_TARGET_DIR`; produced `validation-artifacts/windows-ws-de46a8e/tauri-target/release/xarchive-desktop.exe`. | Executable build only; no Full package assembly, signing, GUI launch, or live pairing implied. |
| Full package / Extension ZIP exact-revision assembly | `NOT RUN` | No packaging command was required by the narrow auth-timeout/diagnostics diff; package plan/static checks remain covered by existing valid evidence. | No current-revision ZIP/Full package artifact was built or browser-loaded. |
| Live browser GUI and authenticated pairing (WQ-WS-01/02/04) | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | Limited Computer Use attempts returned no native app targets (`apps: []`); the available Edge surface exposed an existing user profile, which was left untouched. `listWindows()` was unavailable in the tool surface. | Need a controlled/disposable Edge profile and current-revision Full package. Prior user-reported manual failure remains historical and is not upgraded by automated PASS. |
| Desktop/Service Worker lifecycle (WQ-WS-03) | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | No controlled GUI/browser target for pending request, restart, reconnection, or cleanup sequence. | Continue in a disposable profile with a synthetic/test-owned request. |
| Full workspace regression | `NOT RUN` | Current source diff is limited to Desktop WebSocket auth/diagnostics, Extension bridge/UI status, and focused tests; affected Desktop/Extension suites, Windows Rust target tests, frontend checks, formatter, and Windows executable build passed. | No unrelated Sidecar, aria2, account-discovery, filesystem, or complete workspace suite was run. |
| Signing/release gate | `NOT RUN` | No signing credentials or release workflow invoked. | Not required to validate this source handoff. |

#### Manual Windows Validation Queue

1. Build or obtain a Full package and Extension ZIP from source revision `98f16845b9e37f0dea419e7bffc890b1be3d2063`; verify the exact source revision before launch. Use a disposable Edge profile rather than the user's signed-in profile.
2. Launch the Full package. Open Extension settings and popup. Confirm the new bounded `auth_timeout` state is visible after a silent/nonresponsive peer and Desktop's auth/connection diagnostic counters update. Never copy the pairing token into screenshots, logs, or documentation.
3. With a local test token, attempt a wrong token and confirm explicit auth failure with no business request routed. Then pair using the correct token and confirm authenticated status.
4. Send a controlled `query_status` request and a synthetic/test-owned archive request. Confirm matching `request_id` responses and that no token appears in Browser payloads, Job specs, or logs.
5. With one request pending, stop/restart Desktop and restart/reload the Extension worker; verify pending cleanup, bounded reconnect, fresh authentication, no duplicate/replayed request, and accurate Native Messaging fallback state.
6. Separately check popup/options at 100%, 125%, and 150% zoom; Tab/Enter/Space/Escape behavior; long status text; and save-error state. Record each result against the exact source/artifact revision.

No current-revision GUI result is inferred from the earlier user-supplied Full package observations. No new Windows-specific or shared-contract root cause was established by this batch; the Linux-owned shared fix is Windows-automated-test verified, while live integration remains blocked.

### 2026-09-25 current-revision Full package and Extension ZIP preparation

- Package source revision: `fcde5943af2f6ad15c833fa5ab88d6b1758345c6`. It adds validation documentation only after application-source validation input `98f16845b9e37f0dea419e7bffc890b1be3d2063`; production source is unchanged. The Tauri executable was built at the prior source input and reused because the intervening commit changed documentation only.
- Full package directory: `E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\windows-ws-fcde594\full-package`.
- Extension ZIP: `E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\windows-ws-fcde594\XArchive-v0.0.0-pre.1-extension.zip`.
- Package metadata tag `v0.0.0-pre.1` is a local validation label, not a release tag. Bandizip CLI 8.0 (Beta, x64) generated `XArchive-v0.0.0-pre.1-windows-x64-full.7z` from the portable Full directory at compression level 9. Signing/publishing was not attempted.
- Full package `PASS` (assembly and static inventory): executable, `package-manifest.json`, Extension directory, Native Host executable and manifest, worker, and Full gallery-dl payload exist. `package_type=full`, platform `windows-x64`; package Extension identity and Native Host allowed origin match `iaajefkoanbkleojofoadeakelihbjne` from the manifest. This is not browser/GUI acceptance.
- Extension ZIP `PASS` (plan/build/extract/verify): generated a release-plan inventory, staged its 12 declared files, compressed using Windows `Compress-Archive`, expanded to an isolated directory, and passed `extension-package.mjs verify` (12/12). ZIP size 16,796 bytes; SHA-256 `32FDCB29AA847CFF4DDE48A1C0BE4FB10F44C92549782A1B87436BADD30A4FAB`.
- SHA-256: Desktop executable `AF07ED237538125E0ECA1689AE2D6295479CD50334318F76F8D421E4F888F074`; Native Host `76DC613744D83B65A3478FB6D72B534E7F72CDF983D4D83E5BE78C3279BCE153`; gallery-dl `0B36AE6734ED41E12BE6BE1B33D3165A450B3E0A811FC1B8C664C032F7F13B2C`.
- User provided aria2 directory `E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2`; source executable `1.37.0\aria2-1.37.0-win-64bit-build1\aria2c.exe`, aria2 1.37.0, SHA-256 `BE2099C214F63A3CB4954B09A0BECD6E2E34660B886D4C898D260FEBFE9D70C2`. Copied into this validation package at `full-package\sidecar\aria2\aria2c.exe` before archive creation. This is local artifact staging; the package builder and release workflow were not changed. Do not infer app-managed task/download success from binary presence or independent aria2 tests.
- Full `.7z` archive: `validation-artifacts/windows-ws-fcde594/XArchive-v0.0.0-pre.1-windows-x64-full.7z`, size 34,865,707 bytes, SHA-256 `35604266D32526632DF02145D2126310DDA7262C848189979ECF8F8E33C4F077`. Bandizip `t` result `All OK`; extracted with `bz x -target:name -o:<isolated-directory> -y <archive>`. Source/extracted inventory comparison: 77/77 files, missing 0, extra 0, SHA-256 differences 0. Archive creation/integrity/extract comparison `PASS`.
- Bandizip generation command used from the package directory: `bz c -fmt:7z -l:9 -r <absolute-archive-path> '*'`; Bandizip installed at `C:\Program Files\Bandizip\bz.exe` (8.0 Beta, x64). Archive input package directory is `validation-artifacts/windows-ws-fcde594/full-package`.
- Browser Extension load, Desktop GUI launch, WebSocket authentication, request/response, app-managed download, and lifecycle manual checks remain `NOT RUN` or `BLOCKED` according to the queue. Package assembly, aria2 presence, and archive integrity do not resolve the prior user-reported no-response failure or establish download success.
- Detailed numbered steps are in the “2026-09-25 package preparation for manual WebSocket verification” section of `windows-queue.md`.

### 2026-09-25 Full package Sidecar startup investigation

- User supplied a Full package Dashboard screenshot showing the first-run download-folder prompt, `Sidecar 未启动`, and `Sidecar 操作失败：sidecar is not running`.
- The original package at `validation-artifacts/windows-ws-fcde594/full-package` contained a stale frozen worker. Desktop's `start_sidecar` passes `--timeout-seconds 300 --discovery-timeout-seconds 600` from its Windows configuration. The old worker's `--help` omitted both arguments. Replaying this exact argument set against the original packaged worker reproduced exit code 2 with `error: unrecognized arguments: --timeout-seconds 300 --discovery-timeout-seconds 600`. Thus the worker quits before the protocol handshake and Desktop reports Sidecar not running. The prior isolated hello test omitted Desktop's actual arguments and was insufficient to validate the launch path.
- Rebuilt the worker from the current Sidecar source using Python 3.12.14 and PyInstaller 6.22.3. Fresh `--help` exposes both timeout arguments. A direct process-level probe with Desktop's exact arguments, packaged gallery-dl, and v2 JSONL `hello` returned `ready` plus the expected capabilities; graceful `shutdown` exited 0. Worker SHA-256: `B51566894CAA6C20AF5B81F2D93D1B8DFAC1B3EC3F24F6EBC8FF8B2222F1318A`.
- Reassembled a clean corrected package at `validation-artifacts/windows-ws-fcde594/full-package-sidecar-fix`; included aria2 from `E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2\1.37.0\aria2-1.37.0-win-64bit-build1\aria2c.exe` at `sidecar/aria2/aria2c.exe`. Bandizip 8.0 Beta x64 created `XArchive-v0.0.0-pre.1-windows-x64-full-sidecar-fix.7z` (34,877,156 bytes; SHA-256 `1DB74B9DEBF5CB6E72A098D871EE20C70582D28E8E4B495E0B1AEF16BC9552C5`). Bandizip archive test passed. Extracted inventory/hash comparison: 77 package files, 77 extracted files, 0 missing, 0 extra, 0 differing.
- Validation classification at the initial automation attempt: old Full package exact Desktop worker invocation `FAIL` (confirmed stale worker CLI); fresh worker exact-argument protocol handshake `PASS`; corrected archive integrity and extraction comparison `PASS`; corrected package GUI start was initially `BLOCKED` when the user stopped Computer Use with physical Escape. A later user screenshot on the same date supersedes that GUI block for the visible state: corrected-package Dashboard shows Sidecar connected. No archive task/download success is claimed.
- Remaining manual step: submit one controlled task from the corrected Full package while Sidecar reports `ready`; record task execution/download separately. The Extension pairing failure remains separate and is tracked in the subsequent dated entries.

### 2026-09-25 corrected Full package Extension WebSocket follow-up

- User-provided screenshots are from the Extension directory under `validation-artifacts/windows-ws-fcde594/full-package-sidecar-fix`. Options rendered, WebSocket was enabled on port `17321`, and its UI still showed disconnected. Desktop showed Sidecar connected, Extension files ready, Native Host registered, browser observation `not_loaded`, and all WebSocket counters at zero.
- DevTools Network showed six `127.0.0.1` WebSocket requests, all status `101`, displayed resource size `0 B`, durations 2–52 ms. These entries establish repeated HTTP Upgrade success only. The screenshots do not include WebSocket Messages, so they do not prove an `authenticate` frame or any server response.
- Read-only Windows process inspection during this investigation found the running `xarchive-desktop.exe` at the corrected package path and a listener on `127.0.0.1:17321`; local TCP `TIME_WAIT` entries were also present. This is a separate point-in-time observation, not per-request correlation with the six DevTools entries.
- Classification: Extension Options page load/display `PASS` (user screenshot); WebSocket Upgrade attempts `PASS` (user screenshot); authenticated session and request/response `FAIL` (Extension remained disconnected/no response); authentication outcome and component root cause `NOT RUN` / unresolved. Desktop diagnostics may be stale until refresh. If refreshed counters still show `accepted=0` after a newly generated 101, that conflicts with the inspected listener's accept path and should be investigated as a stale/different runtime or diagnostics defect before concluding the token is wrong.
- User clarification (2026-09-25): the token is for local validation and resets on Desktop restart. Per the user's request, token privacy/security review is `NOT APPLICABLE`; no compromise finding or rotation action is tracked. The literal token is omitted from committed records.
- Next evidence needed: refresh Desktop Extension diagnostics, create one reconnect, select that same newest DevTools request and inspect Messages for outbound `authenticate` plus inbound `authentication_response`; provide only frame type/boolean outcome and before/after counters. Do not include the token or full frame payload.

### 2026-09-25 WebSocket accepted but closed before authentication

- New user screenshots show refreshed Desktop counters: `accepted=100`, `auth_received=0`, `auth_succeeded=0`, `auth_failed=0`, `auth_response_failed=0`, `close_before_auth=99`, `close_after_auth=0`. The Extension popup remains disconnected and uses Native Messaging fallback. Edge's Extension error page reports `net::ERR_SOCKET_NOT_CONNECTED` at the WebSocket constructor call (`websocket-bridge.js:55`).
- For this aggregate snapshot only, WebSocket connections reached Desktop's accept loop, but no authentication envelope was counted. 99 connections closed before auth; the 100th was not yet counted closed in the screenshot. A wrong token would be evaluated only after receiving/parsing the auth envelope and incrementing `auth_failed`. HTTP 101 remains handshake-only evidence.
- The earlier six DevTools rows had 2–52 ms durations, suggesting early teardown rather than the listener's 5-second auth read timeout, but they were not correlated to the refreshed cumulative counters. At the time of this snapshot the receive/send/close stage was unresolved. No response-path or token defect was established.
- Result: live authenticated pairing `FAIL`; listener accepts WebSocket upgrades `PASS`; auth receipt `NOT RUN` (zero observed); auth outcome `NOT RUN`; exact client/server teardown cause unresolved. The previous screenshot's all-zero counters are superseded by this refreshed snapshot.
- This counter snapshot is point-in-time evidence and is not correlated with any later selected DevTools request. Do not treat it as proof that a later outbound auth frame was not received.
- Next observation: initiate one reconnect and correlate before/after Desktop counters with that exact request's Messages view. Report only whether `authenticate` and `authentication_response` appeared, the boolean auth outcome/error code, and relevant counter deltas.

### 2026-09-25 WebSocket authentication frame observed

- User supplied a new Edge DevTools screenshot for `ws://127.0.0.1:17321/`. The selected request's Messages pane visibly contains one outbound `authenticate` frame. Its payload includes the pairing token; the value is deliberately not copied here. No inbound `authentication_response` is visible in the capture.
- In the same evidence set, Extension Options remains `已断开`, the browser console reports `ERR_SOCKET_NOT_CONNECTED` at `websocket-bridge.js:55`, and repeated requests continue. This establishes that the Extension emitted an auth frame for the selected request, but it does not establish Desktop receipt, authentication outcome, or response delivery.
- The earlier Desktop aggregate snapshot (`accepted=100`, `auth_received=0`, `close_before_auth=99`) was not taken in a per-request-correlated manner. It cannot be applied to this selected DevTools connection; server receipt for this connection remains unknown.
- User instruction: the token is local-validation-only and resets on each Desktop restart; skip privacy/security review of this token. Classification: token privacy/security check `NOT APPLICABLE`; no compromise finding or rotation action. The literal value is omitted from committed records.
- Result: outbound authentication frame `PASS` (selected request, user screenshot); inbound authentication response `NOT OBSERVED`; live authenticated pairing remains `FAIL` (UI disconnected); exact transport/server outcome `NOT RUN` / unresolved. The next useful observation is one correlated reconnect with Desktop counter deltas and the same request's Messages pane.

### 2026-09-27 Windows batch at shared handoff `6d60429`

- Branch: `feature/u7-desktop-production-integration`. Fetched `origin` and fast-forwarded the formal Windows working tree from `f269efea64ebf6e656094ba3d2230962a83cb29f` to exact handoff input `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3`. Tracked files were clean before update; existing local `.codex`, `.venv-windows-validation`, aria2/gallery-dl, logs, manual-validation, build and validation-artifact directories were preserved.
- Scope: Windows revalidation of the accepted WebSocket stream blocking-mode fix and stage counters, plus Extension parsed-host URL validation. No Windows-owned production implementation change was required. The shared fix remains Linux-owned; it was not modified here.
- PASS: Windows-target regression `cargo test -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-batch-6d60429/target websocket_transport::tests::authenticates_when_the_accepted_stream_starts_non_blocking -- --exact --nocapture` (1/1; 111 filtered). This exercises a non-blocking accepted stream and delayed authentication frame on the Windows target.
- PASS: `npm test --workspace extension` (32/32), including exact parsed-host allowlist, credentials/ports/schemes, query/fragment and relative-link cases. `npm run check --workspace desktop` built 52 Vite modules; `cargo fmt --all -- --check` passed.
- PASS: `npm run build:tauri --workspace desktop -- --no-bundle --ci` built a fresh Windows release executable with isolated target `desktop/src-tauri/validation-artifacts/windows-batch-6d60429/target/release/xarchive-desktop.exe`; size 19,217,408 bytes; SHA-256 `BB597610902C23A3481E2FEAE4351170BA7B9C882ECAA65915864E284246FBDC`. The non-fatal MSVC linker messages reported import-library creation; exit status was 0.
- PASS: Full portable package assembled at `validation-artifacts/windows-batch-6d60429/full-package` using the fresh Desktop executable plus preserved local Native Host and worker/gallery-dl artifacts. Manifest type/platform are `full` / `windows-x64`; Extension ID and Native Host allowed origin agree (`iaajefkoanbkleojofoadeakelihbjne`). Static package contained 76 files, including `_internal/python312.dll` and bundled `gallery-dl.exe`.
- PASS: Extension ZIP plan/build/expand/verify at `validation-artifacts/windows-batch-6d60429/`: 12/12 files match plan `XArchive-v0.0.0-pre.2-extension.zip`; SHA-256 `244F30E27CF797CE02D4A384A94A3A4A1FFD9253ECC89803B5A385AE571644B9`.
- PASS: Bandizip 8.0 Beta created and tested `XArchive-v0.0.0-win-batch-6d60429-windows-x64-full.7z`; `bz t` returned `All OK`. Archive SHA-256 `F81C5777F17E98B084CA70E26DB95C32016CC28CAED4B17C3496539E79870CF1`. Extracted inventory matched 76/76 files, 0 missing, 0 extra, 0 SHA-256 differences. Both package names/tags are local validation metadata; nothing was signed or published.
- BLOCKED: Current exact-revision WQ-WS-01/02/03/04 browser load, authenticated pairing, request/response, reconnect/pending cleanup, and visual/keyboard checks. Computer Use was retried; both inventories returned no native app targets and only exposed the existing Edge profile. That profile was left untouched. Blocker: `COMPUTER_USE_UNAVAILABLE`; continue in a controlled/disposable Edge/Chrome profile using the package above. Prior `fcde594` user-reported pairing failure remains historical and is not reclassified as a current failure or pass.
- NOT RUN: full Desktop crate regression, full workspace regression, browser/GUI launch, real account/task/download, Native Host registration, code signing, and release publication. The diff was narrow; the new Windows socket-mode regression, affected Extension suite, frontend build, Rust formatting and fresh package build/inventory were selected. Full regression was not run for this change set.
- Cross-platform follow-up: `CROSS_PLATFORM_CHANGE_REQUIRED` none newly identified; `CROSS_PLATFORM_REVIEW_REQUIRED` none. Keep Windows ownership because only manual Windows GUI work remains and no cross-platform follow-up was found.

### 2026-09-27 user-reported Sidecar launch failure and corrected package probe

- Validation input: Windows Desktop source at `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3`; this revision has no Sidecar source changes relative to the previously built current-source worker input `fcde5943af2f6ad15c833fa5ab88d6b1758345c6`.
- The user screenshot from `validation-artifacts/windows-batch-6d60429/full-package` shows first-run download-folder setup still pending, Sidecar stopped, Extension disconnected, and `sidecar is not running` after a Sidecar action.
- `desktop/src-tauri/src/config.rs` adds `--timeout-seconds 300 --discovery-timeout-seconds 600` to the worker launch arguments. The worker in that package (SHA-256 `96C19695AC46E30AA23AF184ED41D0C8339FE4C98A2D771CF0781906402A60C2`, same hash as preserved `sidecar/dist` artifact) omits both options from `--help`. Replaying those arguments exits `2` with `unrecognized arguments`, before the protocol-v2 `hello -> ready` handshake. The package's earlier static inventory/archive PASS remains valid for file presence/integrity, but its Sidecar runtime-compatibility result is now `FAIL`.
- Corrected isolated package assembled with `desktop/scripts/build-portable-windows.mjs` at `validation-artifacts/windows-batch-revalidation-6d60429-sidecar-current`, using the same fresh Desktop executable and preserved Native Host/Extension, with the current-source worker from `validation-artifacts/windows-ws-fcde594/full-package-sidecar-fix`. The corrected worker accepts both timeout arguments; its package SHA-256 is `B51566894CAA6C20AF5B81F2D93D1B8DFAC1B3EC3F24F6EBC8FF8B2222F1318A`.
- PASS: exact-argument worker CLI and JSONL v2 probe against the corrected package, including `hello -> ready`, required `account_discovery` capability, `shutdown`, and exit code `0`. This verifies the worker/package handshake only; it does not prove the Desktop GUI starts the Sidecar.
- BLOCKED: corrected-package Dashboard Sidecar start and Extension connection were not re-run. The old package Desktop process (PID 29792) remains open at the stale package path; Computer Use reports no native app target. Do not replace its files or kill it. The new package's first-run download-folder choice requires the user. See WQ-WS-06 in `../validation/windows-queue.md`.
- Edge Computer Use reports an attached Edge extension instance but does not expose the profile name and currently lists multiple tabs, conflicting with the user's report that only the Codex profile tab remains. No tab was opened or navigated. Profile identity and XArchive Extension connection remain unverified; see WQ-WS-07.
- No shared contract/protocol failure is indicated: the cause is a stale local frozen package artifact, while a worker built from unchanged current Sidecar source accepts the current Desktop arguments. `CROSS_PLATFORM_CHANGE_REQUIRED`: none. `CROSS_PLATFORM_REVIEW_REQUIRED`: none.

### 2026-09-27 user retest: corrected package starts; Extension authentication rejected

- User-provided PowerShell process inventory identifies `xarchive-desktop.exe` PID `66608` and `xarchive-downloader.exe` PID `55608` under `validation-artifacts/windows-batch-revalidation-6d60429-sidecar-current`.
- User Dashboard screenshot: SQLite connected; Sidecar running with `hello -> ready`; Extension remains disconnected. This closes WQ-WS-06 as `PASS` for corrected-package GUI startup/Sidecar state.
- User Extension Options screenshot: WebSocket enabled at port `17321`, settings saved/connecting, current status `认证失败`; Native Messaging fallback is shown.
- User Extension diagnostics screenshot: Extension files ready and Native Host registered; Desktop had not observed an authenticated browser connection. The counters show one accepted connection, one auth received, one auth failure, no handshake failure, and one close before authentication (point-in-time single-attempt evidence).
- User DevTools screenshot shows an outbound protocol-v1 `authenticate` frame and an inbound `authentication_response` with `authenticated: false` and `error_code: AUTHENTICATION_FAILED`. This proves the request reached Desktop and was rejected at authentication; HTTP 101 alone is not the basis for this result. Authentication/request routing is `FAIL` for this attempt.
- Likely explanation (inference): the Extension's stored token does not match the current Desktop process token, which can change after Desktop restart. This has not been proven from the masked screenshots. Next step: use the current running Desktop's Extension settings copy action, replace the stored Extension token, save/reconnect once, and verify `authenticated: true`; never expose the token.
- The user's standalone worker probe exited `2` because `Start-Process -ArgumentList` split the `--gallery-dl` path at spaces. The error names the path remainder `Workspace\Tw2Tg\...`; this is a PowerShell argument-quoting issue, not a worker runtime failure. A local rerun passed after using one quoted argument string (`--timeout-seconds 300 --discovery-timeout-seconds 600 --gallery-dl "<full path>"`); the Dashboard and process screenshots independently confirm the corrected worker is running.
- Profile label is not visible in the supplied screenshots, so the exact Edge Profile identity remains unconfirmed. Do not record account/page contents or token values.
- Current results: stale package Sidecar start `FAIL`; corrected package Sidecar GUI startup `PASS`; corrected worker probe from prior run `PASS`; current Extension authentication `FAIL`; complete connected/request/response lifecycle `NOT RUN`; full regression/release/signing `NOT RUN`. No new shared contract issue identified; `CROSS_PLATFORM_CHANGE_REQUIRED`: none; `CROSS_PLATFORM_REVIEW_REQUIRED`: none.

### 2026-09-27 Windows job-ID path failure and fix

- Input revision: `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3`; Windows implementation and automated validation revision: `7eacb82`.
- User-provided evidence: after reconnect, the Extension Options page reported authenticated and two tasks appeared in Desktop. Both failed with `EXECUTOR_WORKER_FAILED` / `ARCHIVE_DOWNLOAD_FAILED`, Windows `os error 123` (invalid filename, directory name, or volume label). The generated archive job ID contains an ISO timestamp with a colon. This establishes authentication and task submission for that attempt, but archive execution failed.
- Implementation: `FileStore` maps opaque job IDs to Windows-safe staging directory components, escaping Windows-invalid characters, percent, trailing dots/spaces, and reserved device names. Creation, recovery lookup, and commit share the same mapping. The persisted job ID, database value, and protocol contract remain unchanged. This is a small shared implementation adjustment and is marked `CROSS_PLATFORM_REVIEW_REQUIRED`; no contract change was made.
- PASS: `cargo fmt --all -- --check`.
- PASS: `cargo test -p xarchive-storage --offline --target x86_64-pc-windows-msvc --target-dir validation-artifacts/windows-batch-os123-storage/target` (37/37; includes ISO-timestamp staging, recovery lookup, and commit regression).
- PASS: `npm run build:tauri --workspace desktop -- --no-bundle --ci` with isolated `CARGO_TARGET_DIR=validation-artifacts/windows-batch-os123/desktop-target`; fresh executable built at `validation-artifacts/windows-batch-os123/desktop-target/release/xarchive-desktop.exe`.
- PASS: Full package assembled at `validation-artifacts/windows-batch-os123/full-package` using that executable and the previously validated compatible worker/Native Host/Extension inputs. Required components and `windows-x64` package manifest are present. Package Desktop SHA-256: `2BA8861D56FC10C2EDC20B9B223641D56673F2471EA009F23090313F2FDC5039`; worker SHA-256: `B51566894CAA6C20AF5B81F2D93D1B8DFAC1B3EC3F24F6EBC8FF8B2222F1318A`.
- BLOCKED: post-fix GUI archive revalidation. Two Computer Use inventory attempts found no native app targets; Edge exposed multiple tabs and did not identify the intended Profile. No tab was touched. Manual steps are WQ-WS-08.
- Result classification: pre-fix archive execution `FAIL`; Windows storage regression and build/package `PASS`; post-fix GUI archive execution `BLOCKED — COMPUTER_USE_UNAVAILABLE`; unrelated complete regression, signing, and publication `NOT RUN`.
- Cross-platform follow-up: `CROSS_PLATFORM_CHANGE_REQUIRED` none; `CROSS_PLATFORM_REVIEW_REQUIRED` review the small shared FileStore mapping. Next owner: Linux Cross-platform Owner for review, then Windows for WQ-WS-08.

### 2026-09-27 user retest: executor runner saturation

- Validation input: current formal branch `0d3735a1ae8d2f3636acff489a933a1f3bc96e5b`; user tested the isolated Full package built from implementation `7eacb82` at `validation-artifacts/windows-batch-os123/full-package`.
- User screenshot/report: Sidecar and Extension connected. Four tasks appeared; two initially showed “Downloading” for more than 30 seconds, while later submissions failed with `EXECUTOR_WORKER_FAILED: job executor command queue is full`. The earlier `os error 123` did not recur; this is limited PASS evidence for the Windows staging path mapping and task submission, not archive completion.
- Read-only runtime evidence from the package: one Desktop process, one Dashboard Sidecar worker, one archive worker with gallery-dl child processes; SQLite snapshot at inspection time showed one `DOWNLOADING` job and three `FAILED` jobs. The package logs contain runtime initialization records only. No process was stopped and no task was cancelled.
- Source diagnosis: `JobExecutor::with_capacity_and_factory` creates a 32-entry command channel and a separate one-entry runner channel. `run_worker` forwards `RunJob` to the runner with `try_send`. `execute_persisted_from_factory` persists `DOWNLOADING` before calling `run_job`; a full runner channel returns `ExecutorError::QueueFull`, which is then persisted as `EXECUTOR_WORKER_FAILED`. Thus one job may be executing and one waiting while later concurrently scheduled jobs fail. The displayed “command queue” wording conflates the runner queue with the command channel.
- Timing limit: current package config sets extraction timeout to 300 seconds and transfer timeout to 1800 seconds. A 30-second wait does not prove an individual job is hung; the queue rejection itself is directly explained by the shared bounded runner behavior.
- Classification: current-token pairing/task submission `PASS` (user-reported); Windows path mapping `PASS` (limited, no `os error 123`); rapid scheduling behavior `FAIL` (user-reported and source-confirmed); full archive completion `NOT RUN`/unverified; post-fix burst GUI check `NOT RUN` until Linux returns an exact revision.
- Required follow-up: `CROSS_PLATFORM_CHANGE_REQUIRED` — Linux to define and implement durable scheduling/backpressure so accepted submissions remain queued instead of being failed on transient runner saturation. Preserve an accurate queued state until a runner slot is available, or otherwise return a defined retryable response without marking the durable Job failed. Add a deterministic regression test with at least three overlapping jobs. Windows revalidation is WQ-WS-09 after the exact revision is handed back.
- Ownership: Linux Cross-platform Owner; also review the small FileStore Windows-safe staging mapping from `7eacb82` (`CROSS_PLATFORM_REVIEW_REQUIRED`).

### 2026-09-27 user retest: extraction timeouts and final job states

- Validation input: current formal branch revision `57333ad3fa70702bbe9f889d128e5f230cf4a10e`; user continued testing the corrected Full package at `validation-artifacts/windows-batch-revalidation-6d60429-sidecar-current`.
- User screenshot: all four submitted jobs reached `FAILED`. Two report `EXECUTOR_WORKER_FAILED: job execution error [ARCHIVE_DOWNLOAD_FAILED]: DOWNLOAD_TIMEOUT: gallery-dl timed out`; two report `EXECUTOR_WORKER_FAILED: job executor command queue is full`. The screenshot also shows the Dashboard Sidecar running. The Extension currently appears disconnected in the Dashboard screenshot; no new authentication conclusion is drawn from it.
- Package configuration confirms `extraction_timeout_seconds: 300`, `discovery_timeout_seconds: 600`, and `transfer_timeout_seconds: 1800`. `ExtractionRunner` raises `DOWNLOAD_TIMEOUT` when its configured extraction deadline expires, then terminates the gallery-dl process tree. The earlier observation of more than 30 seconds was below this timeout; the later explicit timeout error establishes that the extraction deadline was eventually reached, but does not identify why gallery-dl failed to finish (network/service response, content/authentication requirements, or another runtime cause remain untested).
- Classification: Windows-safe staging ID mapping remains limited `PASS` (no `os error 123` is reported); the four-task outcome is `FAIL` (user-reported: two extraction timeouts and two shared runner queue rejections); root cause of the gallery-dl timeouts is `NOT DETERMINED`; real archive completion is `FAIL` for this attempt. No source change is justified by the timeout evidence alone.
- Required follow-up: retain `CROSS_PLATFORM_CHANGE_REQUIRED` for durable runner scheduling/backpressure and retest both queued-job progression and a controlled single archive after Linux returns the exact fix revision. WQ-WS-10 records the Windows extraction-timeout investigation. Compare an explicitly controlled public/synthetic URL or known-good fixture with the user-selected post, record only elapsed time, terminal code and output-file presence/hash, and do not capture account content or credentials.
- Ownership: Linux Cross-platform Owner for the already-confirmed executor queue correction and shared FileStore review; Windows Platform Owner resumes the queue and extraction retest on the exact returned package.

### 2026-09-27 user confirmation: X access and Extension controls

- User-provided screenshot/report: `https://x.com/home` loaded with page content, and the user confirms the Extension buttons/controls load. The capture shows the X Home page and visible Extension controls. No account/post identifiers were added to this record.
- Classification: `PASS` for X page access and Extension control rendering only. The screenshot does not identify the Edge Profile or exact package revision; it does not verify manifest permissions, service-worker health, authenticated Desktop connectivity, request/response, or archive execution. WQ-WS-01 is updated with this limited evidence; WQ-WS-02/03 and WQ-WS-09/10 remain open as previously classified.
- Ownership is unchanged: Linux Cross-platform Owner retains the shared executor queue fix and FileStore review. Windows resumes WQ-WS-09/10 after the exact shared-fix revision is returned.

### 2026-09-27 current Windows batch handoff check

- Fetch/status: `git fetch origin` completed; formal branch `feature/u7-desktop-production-integration` was at `d55ad583c969af4c2d3b4e801d0b767df51f8a1d`, equal to origin. The tracked worktree was clean. Existing user-local untracked dependencies, logs, manual files, and validation artifacts were preserved.
- Plan: no separate Plan markdown file was present in the repository or `.codex`; the current plan was taken from `docs/status/platform-handoff.md` and the active WQ-WS-09/10 items. No production-code diff exists since Windows implementation `7eacb82`; the intervening commits update validation/handoff documentation, so code-dependent historical PASS results remain reusable.
- Current browser observation: Computer Use exposed Edge browser tabs but no native app targets (`apps: []` on two inventory observations); the browser backend did not expose a profile name and showed multiple tabs. The existing X Home tab loaded successfully and its accessibility tree exposed injected `XArchive：保存` buttons on timeline items. No buttons were clicked and no task was submitted.
- Classification: `PASS` for current X Home access and Extension content-script button rendering. `BLOCKED` (`COMPUTER_USE_UNAVAILABLE`) for native Desktop window inspection and exact Edge Profile identification. Full manifest/permission and Service Worker checks remain `NOT RUN`; previous build, package, worker-protocol, and storage tests are reused because there is no new production-code diff.
- Revision scope: repository validation input `d55ad583c969af4c2d3b4e801d0b767df51f8a1d`; observed browser artifact source/package revision is not exposed. `git diff --check` over the changed documentation passed. No implementation or full regression was needed in this doc-only/current-GUI batch.
- Follow-up and ownership: shared executor queue/backpressure remains `CROSS_PLATFORM_CHANGE_REQUIRED`; the small shared Windows-safe FileStore mapping remains `CROSS_PLATFORM_REVIEW_REQUIRED`. Linux Cross-platform Owner receives the batch next; Windows resumes WQ-WS-09/10 when the exact shared-fix revision is handed back.

### 2026-09-27 repeated handoff fetch and validation-scope check

- `git fetch origin` completed. On branch `feature/u7-desktop-production-integration`, Windows `HEAD` and `origin/feature/u7-desktop-production-integration` both resolve to `f03a9963db9b92bdfef6fd4d2570634c7fe43392`; tracked working tree is clean. Existing untracked local dependencies, logs, manual files, and validation artifacts remain untouched.
- No separate current Plan markdown was found in the repository or `.codex`; current work remains defined by this handoff and WQ-WS-01 through WQ-WS-10. The diff from Windows implementation `7eacb82` contains documentation only.
- `git diff --check 7eacb82..HEAD` passed. No Windows-owned production failure or new Windows implementation need was found in the current diff. Reuse Windows storage tests 37/37, `cargo fmt --all -- --check`, Tauri release build, and corrected Full package assembly PASS from `7eacb82`; no unrelated tests were rerun.
- Reuse the immediately preceding read-only GUI evidence: X Home loaded and injected `XArchive：保存` buttons were visible. Native app inventory was empty twice, Profile identity and the exact loaded Extension revision remain unverified, and existing archive failures remain open. No task was resubmitted.
- Cross-platform routing is unchanged: queue/backpressure fix is `CROSS_PLATFORM_CHANGE_REQUIRED`, and the small FileStore mapping awaits `CROSS_PLATFORM_REVIEW_REQUIRED`. Current next owner remains Linux Cross-platform Owner; Windows resumes WQ-WS-09/10 after the exact returned revision.

### 2026-09-30 accepted Windows run and SHA-256 inventory (credential-blocked checkpoint)

Workflow tooling SHA for accepted runs: `e6d71ad44f8ef8cd8539259896787426e389ad67`. All rows below are actual accepted Windows evidence; no GUI/installation result is implied.

| Phase / target | Run ID | Source SHA | Asset | Bytes | SHA-256 |
|---|---|---|---|---:|---|
| dry / `v0.1.1-pre1` | [36672977464](https://github.com/15699122/Tw2Tg/actions/runs/36672977464) | `5afc1b8289fbd38792280431159136531ae138da` | `XArchive-v0.1.1-pre1-windows-x64.7z` | 4125276 | `6614c6b5551a248c46b0bb1bdf0d84c18b7a5b7d130f1caa755e70d53bd7744a` |
| dry / `v0.1.1-pre1` | [36672977464](https://github.com/15699122/Tw2Tg/actions/runs/36672977464) | `5afc1b8289fbd38792280431159136531ae138da` | `XArchive-v0.1.1-pre1-windows-x64.exe` | 17477632 | `8e8277555a945f19d4dbfca84a1ec772a62b28f931320a6a5307b1bbbb8f2b83` |
| dry / `v0.1.1-pre2` | [36673063515](https://github.com/15699122/Tw2Tg/actions/runs/36673063515) | `a5f42ccc4b6d661e3cf80338b44859e5178e8480` | `XArchive-v0.1.1-pre2-windows-x64.7z` | 4132005 | `a91e1b199c770b839cf6e12cfaf8b5454c2e20096d45eee04d8d8fa6ee9f8de8` |
| dry / `v0.1.1-pre2` | [36673063515](https://github.com/15699122/Tw2Tg/actions/runs/36673063515) | `a5f42ccc4b6d661e3cf80338b44859e5178e8480` | `XArchive-v0.1.1-pre2-windows-x64.exe` | 17507328 | `f74b9d89338aadc0601ea42f41153e5e8da0cb61fd83bff247a6ea491df15d28` |
| dry / `v0.1.1-pre3` | [36673130141](https://github.com/15699122/Tw2Tg/actions/runs/36673130141) | `de61eaabc2013aa2e9c90481acbf3ba0b7df5535` | `XArchive-v0.1.1-pre3-windows-x64.7z` | 4226964 | `01a291d638bcd8ab54000399ced56495707e7f903ad54f216982f05816d4f8a3` |
| dry / `v0.1.1-pre3` | [36673130141](https://github.com/15699122/Tw2Tg/actions/runs/36673130141) | `de61eaabc2013aa2e9c90481acbf3ba0b7df5535` | `XArchive-v0.1.1-pre3-windows-x64.exe` | 18023936 | `b5c6fdcce79fa3cb10635d4f9eaea6c436a5d5af5834ae24ecfde64403b0e8ab` |
| dry / `v0.2.0-pre1` | [36673250103](https://github.com/15699122/Tw2Tg/actions/runs/36673250103) | `0105ce9fdb4f8c6e9e260312e730804a72d1a6f0` | `XArchive-v0.2.0-pre1-windows-x64.7z` | 4232181 | `a493e3c2dc44428b6141f49c2f16b400689a9aaa0bffc32118e74e095e365dec` |
| dry / `v0.2.0-pre1` | [36673250103](https://github.com/15699122/Tw2Tg/actions/runs/36673250103) | `0105ce9fdb4f8c6e9e260312e730804a72d1a6f0` | `XArchive-v0.2.0-pre1-windows-x64.exe` | 18082304 | `9ca2686e9c8697db8f6dd8ac47c60762c0178e2d08daa8d7e8c599a01eebb156` |
| dry / `v0.2.0-pre2` | [36673316986](https://github.com/15699122/Tw2Tg/actions/runs/36673316986) | `f2ae58db1f5f8be901e1c45f7629147056edeea9` | `XArchive-v0.2.0-pre2-windows-x64-full.7z` | 34253138 | `f772b8d3f9d28b8876a0bbbede59683b9a35fe95821cb7be3ca5a3348c25eb0a` |
| dry / `v0.2.0-pre2` | [36673316986](https://github.com/15699122/Tw2Tg/actions/runs/36673316986) | `f2ae58db1f5f8be901e1c45f7629147056edeea9` | `XArchive-v0.2.0-pre2-windows-x64-repository-dependencies.7z` | 5917420 | `476b04246c64cd3b820b783274ef7cfc2096dd88222bc78f37f56ff60ecdf902` |
| dry / `v0.2.0-pre2` | [36673316986](https://github.com/15699122/Tw2Tg/actions/runs/36673316986) | `f2ae58db1f5f8be901e1c45f7629147056edeea9` | `XArchive-v0.2.0-pre2-windows-x64.7z` | 4270811 | `883224127a98f5d708cf2622ee6f7ebbd6b475d0296f3f0c326db29b61aaa530` |
| dry / `v0.2.0-pre2` | [36673316986](https://github.com/15699122/Tw2Tg/actions/runs/36673316986) | `f2ae58db1f5f8be901e1c45f7629147056edeea9` | `XArchive-v0.2.0-pre2-windows-x64.exe` | 18249728 | `75d1c0dbf0d3dbbd8f136b8aefd219f228a973d1b615b0127c73a445ca70e7da` |
| dry / `v0.2.0-pre3` | [36673405745](https://github.com/15699122/Tw2Tg/actions/runs/36673405745) | `baf0b241237afbd9fb7435f96403af2de5598d91` | `XArchive-v0.2.0-pre3-windows-x64-full.7z` | 34257239 | `7381333dbd6c00b791541f390c533cc5940fdb8e391e73d89aa8c2da15b2973a` |
| dry / `v0.2.0-pre3` | [36673405745](https://github.com/15699122/Tw2Tg/actions/runs/36673405745) | `baf0b241237afbd9fb7435f96403af2de5598d91` | `XArchive-v0.2.0-pre3-windows-x64-repository-dependencies.7z` | 5916756 | `d6960333ead108133fdeedc4cff9bb7eda6458c1a00b8cbcec84f0806b086799` |
| dry / `v0.2.0-pre3` | [36673405745](https://github.com/15699122/Tw2Tg/actions/runs/36673405745) | `baf0b241237afbd9fb7435f96403af2de5598d91` | `XArchive-v0.2.0-pre3-windows-x64.7z` | 4268968 | `3b6c09911ce7072993e78ab4db649dc8d7a31d6684d05ec63d0eabc21ecfd6dd` |
| dry / `v0.2.0-pre3` | [36673405745](https://github.com/15699122/Tw2Tg/actions/runs/36673405745) | `baf0b241237afbd9fb7435f96403af2de5598d91` | `XArchive-v0.2.0-pre3-windows-x64.exe` | 18250240 | `a25e7132e546045096b9acb96ea1c3c9f2e25353733871a905dcb8eee1743125` |
| dry / `v0.2.0-pre4` | [36673504341](https://github.com/15699122/Tw2Tg/actions/runs/36673504341) | `38e9a78a56260f7064b9ebf6a5230b0a9260002e` | `XArchive-v0.2.0-pre4-windows-x64-full.7z` | 34258100 | `a4008d6d69ce0b5e2bef69b66e06abd8b0d738783724c2e9f9108b9f4dbfbdaa` |
| dry / `v0.2.0-pre4` | [36673504341](https://github.com/15699122/Tw2Tg/actions/runs/36673504341) | `38e9a78a56260f7064b9ebf6a5230b0a9260002e` | `XArchive-v0.2.0-pre4-windows-x64-repository-dependencies.7z` | 5918182 | `f62bf33115a6e3997125040341b588ddf1283ea2794b10a132a2862360373d30` |
| dry / `v0.2.0-pre4` | [36673504341](https://github.com/15699122/Tw2Tg/actions/runs/36673504341) | `38e9a78a56260f7064b9ebf6a5230b0a9260002e` | `XArchive-v0.2.0-pre4-windows-x64.7z` | 4266292 | `b17ba34bbc21ff86d1bfc89fe4610f583af0e3ce99b6595e1f124274fb7475d8` |
| dry / `v0.2.0-pre4` | [36673504341](https://github.com/15699122/Tw2Tg/actions/runs/36673504341) | `38e9a78a56260f7064b9ebf6a5230b0a9260002e` | `XArchive-v0.2.0-pre4-windows-x64.exe` | 18250240 | `bf42e1456caec9f31d963f9bbd301ad3e16ccf65fa6462efaf67f13ee5481b9e` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `SHA256SUMS-v0.2.0-pre5.txt` | 538 | `8d9ed3d1e9006c04a7dd09d913905063c84765ec40e0be54c7c67aae06dd4087` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-extension.7z` | 5280 | `7641105d80e61530c5a2c27c818c3f7404b0051ac44dfc80465e147f5016ebae` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-release-manifest.json` | 1350 | `d79fe0e401795b743340b49040e8003a00197772dcec42d840d06c1a93de645a` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-windows-x64-full.7z` | 34319495 | `8c473b51d73ae11c4b73be94ffa297e9e4903bde7fdfe048dd15c377ed5aa75c` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-windows-x64-repository-dependencies.7z` | 6053977 | `32af3d952a05ca0c68421da8d704d3a296caf530d0199c6674e2f447d738fdcf` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-windows-x64.7z` | 4406570 | `6b73ba6aaa1f233b81a6374e3be084195d9703dd965c79dd58ddf50d6a133bc3` |
| dry / `v0.2.0-pre5` | [36673929521](https://github.com/15699122/Tw2Tg/actions/runs/36673929521) | `ac586e609337947aeb51de8f5cce3185efc8995e` | `XArchive-v0.2.0-pre5-windows-x64.exe` | 18258944 | `4f8181459c30c682235573649c2c8713ea5bf191f4a65582837ab21607f25fa4` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `SHA256SUMS-v0.2.0-pre6.txt` | 538 | `c05404bf2e03c7bb09d0524b9619bf899457df2d699116bb8c7ca416dd349880` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-extension.7z` | 5606 | `c616163037f3f9056a431cb0c51b3d5d12b170eaf24f56d43d203b27b0630341` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-release-manifest.json` | 1350 | `f73fb308f03daf7ee714cca707bb7a747e70003ec271c6cc4ac7b474733a5a08` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-windows-x64-full.7z` | 34322488 | `cc00274dea6c00cfe2f8980e792ea5d88131bc02a9606bca454dfde9c043632a` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-windows-x64-repository-dependencies.7z` | 6055960 | `de862ead603044360cddaa0132c24163e8cb9e6d4a72ae180f6eb1e2ab262714` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-windows-x64.7z` | 4270916 | `7cbc14a9dc20768360f5623e7acbbc91aa0175e7f57a203aecef9c647e8c8e00` |
| dry / `v0.2.0-pre6` | [36673591851](https://github.com/15699122/Tw2Tg/actions/runs/36673591851) | `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692` | `XArchive-v0.2.0-pre6-windows-x64.exe` | 18258944 | `9ab7a616acfaf6ab88ecd5db055772fc42c7d27bd64b69e3e3e64c1302e893f4` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `SHA256SUMS-v0.2.0-pre7.txt` | 538 | `c0a6294ace0d3693c14d9638d1e8d69d19df91ccede6acb193984968dda2cf6e` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-extension.7z` | 12380 | `87cf3e8eb78358e53cc8b74fa76853d8a53d8d92cf51d0ba50c14feb18b941f5` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-release-manifest.json` | 1351 | `8968ea9a031fde6e5dafb74bd9f5f5abd0cf2de574776fd2dee00be90cf444a2` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-windows-x64-full.7z` | 34535088 | `a4659a6c56c00796c871c7a9b41dccefb40c286e5dfadba489639cabfbaf312a` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-windows-x64-repository-dependencies.7z` | 6078140 | `ade857fa5bbae091270b42dc7165dcd9a19cfc94a0b3826d8c13cd092317026d` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-windows-x64.7z` | 4471230 | `64b2440cba04f28e5afe5d33ab614614db0684e134248bf7c43c82fc10551e90` |
| dry / `v0.2.0-pre7` | [36673841917](https://github.com/15699122/Tw2Tg/actions/runs/36673841917) | `1c72c2de73690b6c63fd31d0333deeccf1edee4c` | `XArchive-v0.2.0-pre7-windows-x64.exe` | 19264512 | `615b60c16b38bf851c1efc65e28e8bb4bf757fc77af07d5f92b207d56f01121d` |
| draft / `v0.1.1-pre1` | [36674300106](https://github.com/15699122/Tw2Tg/actions/runs/36674300106) | `5afc1b8289fbd38792280431159136531ae138da` | `XArchive-v0.1.1-pre1-windows-x64.7z` | 4125276 | `6614c6b5551a248c46b0bb1bdf0d84c18b7a5b7d130f1caa755e70d53bd7744a` |
| draft / `v0.1.1-pre1` | [36674300106](https://github.com/15699122/Tw2Tg/actions/runs/36674300106) | `5afc1b8289fbd38792280431159136531ae138da` | `XArchive-v0.1.1-pre1-windows-x64.exe` | 17477632 | `8e8277555a945f19d4dbfca84a1ec772a62b28f931320a6a5307b1bbbb8f2b83` |
| draft / `v0.1.1-pre2` | [36674393927](https://github.com/15699122/Tw2Tg/actions/runs/36674393927) | `a5f42ccc4b6d661e3cf80338b44859e5178e8480` | `XArchive-v0.1.1-pre2-windows-x64.7z` | 4132005 | `a91e1b199c770b839cf6e12cfaf8b5454c2e20096d45eee04d8d8fa6ee9f8de8` |
| draft / `v0.1.1-pre2` | [36674393927](https://github.com/15699122/Tw2Tg/actions/runs/36674393927) | `a5f42ccc4b6d661e3cf80338b44859e5178e8480` | `XArchive-v0.1.1-pre2-windows-x64.exe` | 17507328 | `f74b9d89338aadc0601ea42f41153e5e8da0cb61fd83bff247a6ea491df15d28` |
| draft / `v0.1.1-pre3` | [36674492512](https://github.com/15699122/Tw2Tg/actions/runs/36674492512) | `de61eaabc2013aa2e9c90481acbf3ba0b7df5535` | `XArchive-v0.1.1-pre3-windows-x64.7z` | 4226964 | `01a291d638bcd8ab54000399ced56495707e7f903ad54f216982f05816d4f8a3` |
| draft / `v0.1.1-pre3` | [36674492512](https://github.com/15699122/Tw2Tg/actions/runs/36674492512) | `de61eaabc2013aa2e9c90481acbf3ba0b7df5535` | `XArchive-v0.1.1-pre3-windows-x64.exe` | 18023936 | `b5c6fdcce79fa3cb10635d4f9eaea6c436a5d5af5834ae24ecfde64403b0e8ab` |
| draft / `v0.2.0-pre1` | [36674586528](https://github.com/15699122/Tw2Tg/actions/runs/36674586528) | `0105ce9fdb4f8c6e9e260312e730804a72d1a6f0` | `XArchive-v0.2.0-pre1-windows-x64.7z` | 4232181 | `a493e3c2dc44428b6141f49c2f16b400689a9aaa0bffc32118e74e095e365dec` |
| draft / `v0.2.0-pre1` | [36674586528](https://github.com/15699122/Tw2Tg/actions/runs/36674586528) | `0105ce9fdb4f8c6e9e260312e730804a72d1a6f0` | `XArchive-v0.2.0-pre1-windows-x64.exe` | 18082304 | `9ca2686e9c8697db8f6dd8ac47c60762c0178e2d08daa8d7e8c599a01eebb156` |

Evidence directories: `validation-artifacts/migration-20260930-7a3374b/<phase>/<target>/`; each contains run logs/status, downloaded `migration-plan.json` and `migration-verification.json`; accepted drafts also contain downloaded assets. Artifact names are `pre-release-migration-<target>-<run_id>`.

## 2026-10-01 Windows validation — 01c40db

- Git: fetched origin main/dev; selected exact requested source `01c40db7324f71f0971f3da4ef88ef4b180869d7` on `codex/windows-validation-01c40db` through Git, tracked tree initially clean. Existing ignored/untracked caches preserved; no filesystem synchronization.
- Scope: current density/icon Plan and preceding logs default-filter fix; Windows native PowerShell, Node 24.19.0, npm 11.19.0 through npx, Rust 1.98, Python 3.12.14. Computer Use @oai/sky available, actual packaged WebView2 window inspected at 1082x790; system DPI not certified by this run.
- Build PASS: fresh optimized Desktop/Vite and Full assembly, XARCHIVE_RELEASE_TAG / PORTABLE_RELEASE_TAG=v0.2.1-pre1; PORTABLE_OUTPUT_DIR=dist-portable/windows-01c40db-20261001-full; binary reuse fallback not enabled. Worker supplied from windows-6be3269-20261001/worker-dist/xarchive-downloader, Native Host from unchanged previous build: reused components, not newly compiled components. Version files remain 0.2.0 by plan.
- Desktop EXE SHA256 `08CC43B5BDBD677898EFC789ADDE1C1820379535F5C65C7B6F5FA83CE4DA2DEC` (19521536 bytes). Source identity, rather than displayed version, binds evidence.
- Desktop tests: first FAIL 186/188, icon render function extraction used LF-only blank-line regex on Windows CRLF; outer test also failed for the same cause. Test-only fix accepts CRLF and LF, assertions unchanged. Rerun PASS 188/188, exit 0. Implementation revision `28543edc10c432415794943736644f7c9d712f1e`, CROSS_PLATFORM_REVIEW_REQUIRED. No product implementation defect inferred.
- GUI subchecks: compact sidebar and distinct dashboard icon PASS at current viewport; empty/stopped settings-link gap PASS; Core Bootstrap missing top divider and retained next divider PASS; actual settings accessibility order Core Bootstrap / Sidecar / aria2 / Extension / Storage / Logging / Network Proxy PASS. Title-bar green archive-box icon observed from fresh path. Full DPI, keyboard, populated/running and all native icon surfaces NOT_RUN.
- Logs focused revalidation PASS: default select Debug, startup channel=prerelease/channel_default=debug/effective_level=debug/user_override=none, debug diagnostics visible. Manually selected Info via keyboard after pointer targeting error; remained Info on later polling. Prior Info default failure is resolved for this subcheck only; complete real module diagnostic coverage remains NOT_RUN.
- Reuse: unchanged Rust229/Python8 and prior stable channel backend subchecks from 6be3269; no new Rust/Python/WDIO/full regression. Existing Native Host installation FAIL not closed; Telegram paused; System Proxy Batch B not delivered.
- Evidence (ignored): validation-artifacts/windows-01c40db-20261001/build-full.log, desktop-tests.log, desktop-tests-r2.log, exe-hash.json, logs-default-debug.txt, logs-manual-info.txt, settings-headings.txt, cleanup.json. GUI screenshots observed in session; no token-bearing Settings tree or screenshot stored. Cleanup PASS: app/worker/driver processes and known listen ports empty.
- Manual queue: current windows-queue latest section. Results do not close release acceptance. Validation revision: Git commit containing this record. Next Owner Cross-platform for shared test review/reconcile, no WINDOWS_VERIFICATION_BLOCKING.

## 2026-10-01 Windows runtime icon follow-up

User screenshots show clear Explorer/properties icon but blurred running taskbar icon. Repository has no notification-area tray creation; screenshot is taskbar. Root cause confirmed from installed tauri-codegen2.6.3 src/context.rs and src/image.rs: Windows selects ICO, CachedIcon::new_ico decodes entries()[0]. Current ICO starts16x16; Explorer instead selects size-specific PE resources. Thus the runtime loses the remaining ICO resolutions. Windows-owned fix in lib.rs overrides Context default_window_icon with compile-time embedded256x256 PNG under cfg(windows), retaining multi-size EXE ICO and non-Windows behavior. No dependency/API/contract change.

Fresh optimized pre-release Full build PASS, exit0, desktop19784704bytes; directory dist-portable/windows-runtime-icon-20261001-full. Worker/Native Host reused unchanged as in preceding batch. Initial sandbox npm-cache EPERM logged separately, permitted build retry succeeded. Evidence runtime-icon-build.log/runtime-icon-build-r2.log/runtime-icon-hash.json under validation-artifacts/windows-01c40db-20261001. Runtime taskbar/Alt+Tab/titlebar multi-DPI acceptance NOT_RUN; build is not visual acceptance. Use fresh executable path/new shortcut without clearing global icon cache, compare taskbar at100/125/150/200% with Explorer and ensure small titlebar remains legible. No new tests added/run for this follow-up.

## 2026-10-01 Owner manual confirmation — runtime taskbar icon

Result: PASS for the reported runtime taskbar blur defect. The human Owner explicitly confirmed the fix and supplied codex-clipboard-0581c50a-3237-4f67-b6cd-14796c7c44da.png in this conversation; the cropped screenshot shows a sharp green archive-box taskbar icon. Evidence provenance: Owner manual observation, not agent automation. Implementation: 9cc9d5d5c6f62b9e87f6357a8acf913ddf450e55, Windows Context override to256x256 PNG; Full at dist-portable/windows-runtime-icon-20261001-full, EXE SHA256 B725122CC9B8E67DA57C7064A3F2536F43F22B6B633F72F746034EA711C76144. The screenshot alone does not expose path/hash or DPI; artifact association follows the preceding repair-package handoff and Owner response. Original reported blur defect closed. Full100/125/150/200% DPI and Alt+Tab/tray matrix not established by this cropped screenshot and remains NOT_RUN where applicable (notification-area tray not implemented). Prior NOT_RUN records are retained as historical facts.

## 2026-10-02 Windows Telegram shared-layer validation — 1f14cea

### Identity, capability and selected scope

Source/input/implementation/tested SHA: `1f14cea6859dc1c0ecec164509579cfe4eb15f1a`, fetched explicitly from `origin/dev`; execution branch `codex/windows-validation-1f14cea`. Default remote fetch refspec did not include dev. Initial checkout was `codex/windows-validation-01c40db` at `40b637b`, with no tracked modifications; it is preserved. Git switch aligned the formal NTFS workspace, not a filesystem sync. Validation documentation/handoff revision is the commit containing this record, pushed to the execution branch. No product/test code changed.

Capability detected: native Windows build `10.0.29671`, AMD64, PowerShell 7.6.6; restricted filesystem sandbox (Git metadata/network-cache operations used approved escalation), network available via native execution. Node 24.19.0, Cargo 1.98.0, rustc 1.98.0, toolchain `1.98.0-x86_64-pc-windows-msvc`, native Python 3.12.14 at `.venv-windows-validation/Scripts/python.exe`. Working directory for all commands: `E:\Shiraishi\VSCode Workspace\Tw2Tg`. GUI/browser/driver/display scaling not exercised: this diff delivers backend modules and the Telegram GUI entry point is absent. There is no new COMPUTER_USE_UNAVAILABLE claim. No external Telegram service/account tested.

Impact baseline: released source `3115c3b50716be0155804ad4f94dd9d29e37d617` through input SHA changes Telegram transport/outbox/cache/plans, storage migration 0007, Desktop config/clock/send core and related dependencies/docs. Selected Module/Subsystem scope: both Telegram and storage suites, plus Desktop lib as their direct consumer, including config, clock, mock sending and native compilation. Full workspace regression, unrelated frontend/Python/packaging and old release GUI matrix NOT_RUN: this is not a release, and the affected subsystem/migration tests cover the delivered changes; no executable runtime send path exists. Older acceptance results retain their original revisions and limits rather than being promoted to this commit.

### Results and evidence

Common fields for rows below: owner Windows Platform Owner; implementation IMPLEMENTED for tested shared modules; platform Windows x64; method automated (tests) / static (fmt/docs); priority P1 (docs P2); prerequisites native MSVC/Rust and locked dependency access, native Python for discovery fixtures; expected all selected tests pass without weakening assertions; blocks_development=no, blocks_release=no for this local validation batch. Revalidation requires unchanged relevant code, dependency/contract and environment; otherwise REVALIDATION_REQUIRED. Evidence root `validation-artifacts/windows-batch-1f14cea/` is local and intentionally untracked; results/hashes are committed here.

| ID | Target / reproducible command | Status | Evidence / result |
|---|---|---|---|
| WIN-TG-MODULE-01 | `cargo test --locked -p xarchive-telegram -p xarchive-storage --no-fail-fast` | PASS | `telegram-storage-native.log`, exit 0: Telegram 48/48 and storage 51/51; both doc-test sets 0 tests. Local mock transport/SQLite evidence, not Telegram E2E. |
| WIN-TG-DESKTOP-01 | Set `PYTHON` to the absolute native interpreter path, then `cargo test --locked -p xarchive-desktop --lib --no-fail-fast` | PASS after environment correction | `desktop-lib-tests-r2.log`, exit 0: 175/175. Includes 8 send-core tests, config/clock, migration consumers and Windows Named Pipe fixtures. First-run failure preserved below. This compiles a test executable, not a packaged GUI/release binary. |
| WIN-TG-FMT-01 | `cargo fmt --all -- --check` | PASS | `fmt.log`, exit 0. |
| WIN-TG-DOCS-01 | `node scripts/docs-audit.mjs`; `git diff --check` | PASS | initial/final docs audit logs, exit 0; final diff check exit 0. |
| WIN-TG-UNIX-01 | Three storage tests gated by `#[cfg(unix)]` | NOT_APPLICABLE | `file_store.rs` intermediate/final symlink tests and `lib.rs` sidecar symlink escape test; Linux 54 vs Windows 51. Windows junction/real-filesystem acceptance is a different existing queue item, not proven by exclusion. |
| `WQ-TG-001`–`009`, `WQ-TG-UNI-01`–`08` | Native credentials, real send/recovery/large files, GUI and Unigram acceptance | NOT_RUN | `IMPLEMENTATION_NOT_READY`: send core exists, but production enqueue/claim loop, Tauri commands, settings/task UI and Windows Credential Manager adapter absent. No runnable acceptance artifact (artifact_sha256/build_origin=N/A). Steps/prerequisites/expected evidence remain in windows-manual-steps §K. Follow-up Cross-platform Owner runtime handoff, then Windows native/real-account work. |

Test artifacts produced by the commands above (SHA-256):

| Artifact under `target/debug/deps/` | SHA-256 |
|---|---|
| `xarchive_telegram-88beb19c2af1469b.exe` | `75D3E9DEC8F08E5DF7281B9171492774DF92833E56D66CFD90DA54D193D789BF` |
| `xarchive_storage-b8f9486e9e316f45.exe` | `EB39EA079B3078BB7C54B581C5E124A41863FBAAAB70AB634FA5D44824E04ECD` |
| `xarchive_desktop_lib-772e0490fa15adbe.exe` | `268C6FD128DB5CD445C05E3C2ECF418E1DF8387F9A998312EBA606381AEC3A61` |

For static checks artifact_sha256=N/A (source check). `artifact-hashes.json` stores absolute paths. `changed-files.txt` and `workspace-status.txt` preserve scope/local state. Local untracked `.codex`, Python environment, aria2/gallery-dl/sidecar components, desktop logs, dist-portable/manual-validation/validation-artifacts were retained and not staged.

### Failed initial paths, diagnosis and recovery

1. Initial sandbox `cargo test -p xarchive-telegram -p xarchive-storage --no-fail-fast` exit 101 before tests: Schannel `AcquireCredentialsHandle` / `SEC_E_NO_CREDENTIALS` fetching mime_guess. `telegram-storage-tests.log`. Locked offline attempt exit 101: mime_guess missing in cache (`telegram-storage-offline.log`). Equivalent approved native locked command downloaded dependencies and passed. These are tool/environment failures, not product results. Native compile also logged one incremental-cache os error 5 note and MSVC import-library stdout warnings; compilation and tests completed successfully. No production change made.
2. Initial native Desktop lib run exit 101, 172/175 (`desktop-lib-tests.log`): `batch::tests::{records_a_failed_discovery_on_the_batch,runs_discovery_and_persists_streamed_candidates,persists_each_discovery_candidate_before_completion}` reported sidecar not running. `batch.rs:1166–1168,1213` selects PYTHON or python3; Get-Command resolved python3 to WindowsApps, and independent `python3 --version` could not launch that alias. Explicit native Python 3.12.14 restored all 3 tests. Repeated command, unchanged source/artifact, 175/175 exit 0. Preserve both runs; default launcher configuration is unreliable, and future Windows test recipes must bind PYTHON. No shared defect/review request raised by this environment correction.

### Manual queue and handoff

All 17 Telegram/Unigram acceptance rows stay NOT_RUN; unimplemented portions PLANNED. The manual index had incorrectly called missing implementation BLOCKED and had an old baseline; both corrected to NOT_RUN/IMPLEMENTATION_NOT_READY at input SHA. The queue also incorrectly said no send core existed; it now distinguishes implemented core from absent runtime. Old release/icon/DPI/Native Host/transfer evidence and gates remain unchanged.

Next Owner Cross-platform Owner: reconcile this result and complete the already planned runtime enqueue/claim loop, commands/settings/task projection before a runnable Windows handoff. Windows retains Credential Manager, lifecycle/GUI, server deployment and controlled real-send/Unigram verification afterward. CROSS_PLATFORM_CHANGE_REQUIRED and CROSS_PLATFORM_REVIEW_REQUIRED: none new; no shared code/assertion edits. WINDOWS_VERIFICATION_BLOCKING: none. No publication, version bump or release acceptance.

## 2026-10-02 Owner manual DPI screenshots — v0.2.1-pre1

Method: manual (Owner changed scaling/captured images), followed by agent visual review; not GUI automation. Environment reported by Owner: 14-inch display, 2560x1600, Windows scaling 100/125/150/200%. Windows build/browser/runtime version not supplied for the capture environment; do not import the agent host metadata. Images 1/3/5/7 were resized in chat, so visual clarity conclusions are limited to the presented evidence, not pixel measurements.

Artifact path supplied by Owner and read-only checked: `E:\Shiraishi\Downloads\Compressed\XArchive-v0.2.1-pre1-windows-x64-full\xarchive-desktop.exe`; 19,812,864 bytes, FileVersion 0.2.1, SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. Adjacent package-manifest records release_tag v0.2.1-pre1, platform windows-x64, package_type full. Build origin/source association: release record Run 36838400270 / `3115c3b50716be0155804ad4f94dd9d29e37d617`, with artifact association supplied by Owner and matching package metadata/version/size. Remote asset digest was not independently re-downloaded this round. This is a separate release-artifact result, not Windows GUI evidence for Telegram implementation 1f14cea.

Evidence is preserved locally in `validation-artifacts/manual-dpi-20261002/figure-1.png` through `figure-7.png`; screenshots.json contains original attachment paths and hashes. Screenshot files remain untracked; this committed record preserves their identities:

| Figure | Owner-reported scaling / window | SHA-256 |
|---|---|---|
| 1 | 200%, maximized | EDF5A23A579C604FAA0183D0C730AB3B6BA2FB307728FA06B22815F3D116E891 |
| 2 | 100%, windowed | D0D10EAC94E7FB151DC7660383FF89344F57970D35E95FD2C84F5B73E2843AC6 |
| 3 | 100%, maximized | A0C2B655DAA6E4FD2FB62D01941EAB93EF14651D7B4C8508F816AC77CBEF4FCA |
| 4 | 125%, windowed | B4B1C9DC6EF6AC3F6DA8A931AF418A78FA9F3382E60052AD3E1695126CBBF29A |
| 5 | 125%, maximized | AF461D0FDE2E1FB220325704A8EEC887F0BC7BA8EB5CDC629CE3B58B333C898F |
| 6 | 150%, windowed | E827CE39EB3C03BB3ED65FFFC793B28708B9DE40F005FF009D587ED96C9DA6F7 |
| 7 | 150%, maximized | 85118FDB9BF8850A32141603E1354AE5901B5C85012EA993ECEE5F41D0594002 |

Common acceptance fields: owner Windows Platform Owner; implementation IMPLEMENTED; priority P1; platform Windows x64; prerequisites identified Full artifact and reported display/scaling; expected legible icon and dashboard without overlap/clipping; blocks_development=no, blocks_release=no for these subchecks. Revalidate after icon/layout/dependency or artifact change. Documentation revision is the commit containing this record.

| ID / target | Result | Scope and remaining evidence |
|---|---|---|
| WQ-ICON-030-06 title-bar subcheck | PASS, screenshot-scoped | Green archive-box icon with distinguishable white mark visible in all 7 captures across reported 4 scales. Explorer, taskbar and Alt+Tab surfaces are absent: those current-artifact/multi-DPI subchecks remain NOT_RUN, defer reason missing surface evidence. No tray implemented. |
| WQ-UI-030-01 / 05 dashboard sidebar/glyph subchecks | PASS, screenshot-scoped | Service labels visible without apparent clipping/overlap; dashboard glyph distinct from sidecar waveform. Keyboard/focus and other pages not exercised. |
| WQ-UI-030-02 empty/stopped layout subcheck | PASS for visible gap/non-overlap only | Settings action follows start/stop row in all images; no stretched bottom alignment. Populated/running states not exercised. Button wrapping observation below is not closed by this limited PASS. |
| Settings/logs/keyboard/populated/running acceptance | NOT_RUN | Screenshots contain only the empty Dashboard with Sidecar stopped; prerequisites corresponding pages/actions/controlled fixtures still required. |

Visual observation for Cross-platform layout follow-up: figures 2/4/6 show start-button text split into two lines (Chinese start label above Sidecar), while stop remains one line; figures 1/3/5/7 show a single-line start label. Windowed captures are approximately the same logical width across scales, so this supports available-width sensitivity, not a scale-specific root cause. Current shared CSS uses a two-column dashboard with a 280px runtime-panel minimum, equal grow buttons and no dedicated label no-wrap rule (`desktop/src/style.css`, dashboard/control-panel/button-row; label in pages/dashboard-page.jsx). This is a plausible cause, not a reproduced/measured CSS diagnosis. The label is not visibly cut off or overlapping, and no single-line acceptance criterion was previously specified: record a layout review observation, not functional startup FAIL. Cross-platform Owner should review wrapping policy and, if adjusted, return the same window-width/scaling matrix for Windows revalidation. No code/assertions changed this round.

Next manual step: on the same artifact, capture Explorer file icon, actual running taskbar icon and Alt+Tab tile at the 4 scaling values, including the scale in evidence. Current screenshots do not complete WQ-ICON-030-06. No release gate or Telegram acceptance closes.

## 2026-10-02 Owner native-icon confirmation — 100/125/150%

WQ-ICON-030-06, priority P1, implementation IMPLEMENTED, owner Windows Platform Owner, method manual plus screenshot review. Same Owner-reported 14-inch 2560x1600 environment and artifact association as the preceding DPI record: `E:\Shiraishi\Downloads\Compressed\XArchive-v0.2.1-pre1-windows-x64-full\xarchive-desktop.exe`, FileVersion 0.2.1; read-only SHA-256 recheck unchanged at `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. Build origin release Run 36838400270; source association `3115c3b50716be0155804ad4f94dd9d29e37d617` per release/package record, not 1f14cea. No new remote digest verification. No product/test changes; documentation revision is the commit containing this section.

Owner supplied three images (100/125/150%, in order) showing the running taskbar icon, and explicitly reported Explorer file icon and Alt+Tab icon match the main-window top-left icon. Taskbar PASS is supported by direct image review (legible green archive box/white mark, no plain blurred block); Explorer/Alt+Tab PASS is Owner manual confirmation, without screenshots of those surfaces. The confirmation is scoped to the three supplied scales; 200% is not inferred.

| Scale | Taskbar | Explorer | Alt+Tab |
|---|---|---|---|
| 100% | PASS, screenshot | PASS, Owner confirmation | PASS, Owner confirmation |
| 125% | PASS, screenshot | PASS, Owner confirmation | PASS, Owner confirmation |
| 150% | PASS, screenshot | PASS, Owner confirmation | PASS, Owner confirmation |
| 200% | NOT_RUN | NOT_RUN | NOT_RUN |

Local preserved evidence: `validation-artifacts/manual-icons-20261002/figure-1.png` SHA-256 `5EFD2D025F4160C693258287B204546EBDA4FFC730565684F731F5F895866778`; figure-2.png `790C54C3C7D1618C4A2A468E2EB733611F3269437046FE81E1486AD9CEFAB02B`; figure-3.png `8FFC1C469076CC6AA6E254C8C370555CBEE507AD82C70589015498AE136D98C1`. screenshots.json maps original attachment names/paths. Raw images remain local/untracked; identities and results committed here. Evidence for Explorer/Alt+Tab is the Owner's message accompanying these attachments.

Expected/prerequisites: same EXE, each reported scale, legible icon on each surface; no tray exists (NOT_APPLICABLE). 200% defer_reason=missing execution/confirmation evidence. Follow-up Windows/Owner: only remaining 200% taskbar/Explorer/Alt+Tab, no need to repeat completed scales. Prior four-scale title-bar PASS remains; full WQ-ICON-030-06 is not closed. Other keyboard/settings/logs/task-state checks and the windowed start-button wrapping observation remain unchanged. blocks_development=no; blocks_release=no for these subchecks; revalidate if icon/runtime/artifact/environment changes. No release approval.

## 2026-10-02 Owner 200% evidence reuse — icon matrix complete

The Owner clarified that the current development environment normally uses 200% scaling and instructed reuse of prior screenshots for the remaining icon checks. In the context of the outstanding 200% taskbar/Explorer/Alt+Tab request, this is recorded as **Owner manual confirmation** for those surfaces; it is not a new agent GUI run. Reuse the preceding 200% title-bar image (`validation-artifacts/manual-dpi-20261002/figure-1.png`, SHA-256 `EDF5A23A579C604FAA0183D0C730AB3B6BA2FB307728FA06B22815F3D116E891`) and the Owner's prior icon-consistency observation plus this explicit reuse confirmation. That image itself does not show taskbar/Explorer/Alt+Tab: those three PASS results rely on Owner confirmation, not on visual inspection of absent surfaces. Keep the earlier 100/125/150% image labels as supplied; do not relabel them 200%.

WQ-ICON-030-06: **PASS for the identified v0.2.1-pre1 Full artifact at 100/125/150/200%**, title bar/Explorer/taskbar/Alt+Tab. Notification-area tray NOT_APPLICABLE (not implemented). Priority P1, implementation IMPLEMENTED, owner Windows Platform Owner, method manual; expected clear green archive-box icon on each surface, no blurred green block. Source association `3115c3b50716be0155804ad4f94dd9d29e37d617`, build origin Run 36838400270, artifact identity from prior read-only checks and Owner association: EXE SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`, FileVersion 0.2.1, supplied Downloads Full path. Environment: Owner-reported 14-inch 2560x1600, default 200%, temporary scale tests as labelled. No new executable hash confirmed this turn: the previously supplied path was unavailable during the attempted read-only recheck. The record reuses the established artifact identity; no current file presence or different artifact is inferred.

This closes only the icon matrix at that artifact. Historical NOT_RUN records remain as earlier facts; UI start-label wrapping, settings/logs/keyboard/populated/running matrices remain open. Revalidate if icon/runtime/artifact or relevant environment changes. blocks_development=no, blocks_release=no for this subcheck; no Telegram or general release approval. Documentation revision is the commit containing this section. Next manual item: settings-page layout/order and keyboard traversal, beginning at the normal 200% scaling, with pairing token excluded/redacted in screenshots.

## 2026-10-02 Computer Use settings subchecks — v0.2.1-pre1

Owner Windows Platform Owner; method gui-automated via computer-use plugin 26.928.40906 / node_repl @oai/sky; priority P1; implementation IMPLEMENTED. Native Windows desktop capability confirmed live: helper initialized, list_apps/list_windows and target launch/capture/input succeeded. Exact target `E:\Shiraishi\Downloads\Compressed\XArchive-v0.2.1-pre1-windows-x64-full\xarchive-desktop.exe`; read-only SHA-256 restored/verified this turn as `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`, same established Full artifact, source association `3115c3b50716be0155804ad4f94dd9d29e37d617`, build origin release Run 36838400270. Desktop target selected uniquely from returned window id 3473714 and process path. Initial XArchive app absent from list_apps; launched through sky.launch_app. Scaling 200% is Owner-reported default, not independently measured this run. No system scaling setting changed.

1. Opened Settings through returned button 36; first immediate refresh still showed Dashboard, next observation confirmed Settings. Clicked reload (88) once; did not select archive directory, change settings, start Sidecar, register/unregister Host or trigger external route requests.
2. Captured actual top: Core Bootstrap had no preceding separator, following Sidecar separator visible. Actual accessibility headings: Core Bootstrap, Sidecar, aria2, Extension, Storage, Logging, Network Proxy (last).
3. Keyboard actual screenshot focus: reload -> start Sidecar, disabled stop skipped. At bottom, focused max-log-files without changing value; successive Tab actions visibly reached save logging -> proxy mode -> save proxy -> inspect route, each with visible focus ring. UIA focused_element inaccurately returned pane 8 rather than browser control; no full-page traversal PASS inferred from it.
4. Bottom actual rendering confirmed Logging before Network Proxy, no visible overlap at 1082x790 tool-reported window pixels. First attempted right-edge drag actually hit scrollbar and did not narrow; second boundary drag at x1081 narrowed to 761x790. Top Bootstrap/Sidecar and bottom Logging/Proxy at this narrow width had no visible overlap/clipping; responsive sidebar collapsed to icons. This is current-scale/current-width evidence, not every scale or intermediate section.

| Target | Result | Remaining scope |
|---|---|---|
| WQ-UI-030-03 | PASS top separator/current-scale and 761px narrow subcheck | Other scales/sections remain NOT_RUN. |
| WQ-UI-030-04 | PASS actual heading order + bottom Tab segment and focus + bottom narrow rendering | Full top-to-bottom traversal, intermediate Extension/aria2/storage focus and other scales NOT_RUN. |
| Complete settings keyboard/scaling matrix | NOT_RUN | Only bounded top/bottom segments exercised; full focus attribution unavailable in UIA and token-bearing middle captures excluded from saved evidence. Follow-up visual full traversal, preserving token privacy; not a COMPUTER_USE_UNAVAILABLE blanket blocker. |

Evidence: live tool captures/actions in this chat; safe saved `validation-artifacts/computer-use-settings-20261002/narrow-top.png` SHA-256 `A90F9264565C4093F3C89D535768B818452CEFB176912A8817D7D3DB2AD7A1DD`; `narrow-bottom.png` `DB11138E82F57308B5FCC029F5440F67EA7DE725E645CD7FB19D83925CF85D1D`; redacted settings-headings.txt and tab-observations.txt. Saved evidence local/untracked. Expected separators/order/focus and non-overlap defined in manual §L before execution. Revalidate after relevant CSS/JSX/runtime/artifact/environment changes; blocks_development=no, blocks_release=no for these subchecks.

Tool diagnostics: off-screen indexed input (max-log-files) rejected at y825 outside 790px window; refreshed and used current visible coordinate successfully. User input detected once, refreshed before continuation. Width-restoration drag rejected because destination x1081 was outside current 761px bounds; no further resize retries, app left open at narrow Settings for user review. These are tool limits, not product FAIL. No production code/assertion changes; Full regression not run for GUI-only validation of this artifact.

Sensitive-output incident: initial accessibility-label filtering missed the Extension token embedded in a button name, and two intermediate captures displayed its pairing area. No raw token value persisted in these documents or saved evidence. Current token should be rotated by Owner; previous rotation is historical and does not cover this token. Future captures must keep pairing area off-screen or fully redact before sharing. This run did not rotate or alter pairing security settings.

Next Owner stays Cross-platform for planned Telegram runtime work and windowed start-label layout review; Windows retains complete Settings/keyboard/scale and real-account/native validation. No new shared-contract finding or release approval. Documentation revision is the commit containing this record.

## 2026-10-02 Owner manual keyboard traversal confirmation

Owner reports keyboard traversal manually verified PASS. Scope: current v0.2.1-pre1 Full Settings page in the established normal 200% environment; no new assertion of 100/125/150% keyboard coverage. WQ-UI-030-04 full Settings keyboard traversal/current-scale subcheck supersedes the preceding NOT_RUN for that subcheck only. Method manual, owner Windows Platform Owner, priority P1, implementation IMPLEMENTED; expected visible focus and correct order through the full settings page; evidence Owner message in this conversation (no new screenshot/log supplied). Source association `3115c3b50716be0155804ad4f94dd9d29e37d617`, build origin Run 36838400270, previously verified EXE SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. Artifact identity reused from preceding Computer Use run, not freshly checked this turn. blocks_development=no, blocks_release=no for this subcheck; revalidate after relevant layout/keyboard/runtime/artifact changes.

Keyboard traversal no longer a pending manual action at current scale. Other-scale Settings/Logs/Extension and populated/running Dashboard matrix remains NOT_RUN. Current token rotation is not confirmed by this message and remains pending. Other native/real-account/paths/recovery/proxy checks remain unchanged; use current protocol/production paths, not stale historical download/fallback recipes. No new functional tests for this documentation-only recording. Next priorities consolidated in windows-manual-steps §M; documentation revision is the commit containing this record.

## 2026-10-02 Computer Use follow-up: portable directory, logs and browser boundary

Revision: documentation branch `codex/windows-validation-1f14cea`, starting HEAD `63c44ca`; shared source baseline remains `1f14cea`. GUI target is the earlier release Full EXE, associated source `3115c3b50716be0155804ad4f94dd9d29e37d617`, not the Telegram baseline. Fresh EXE SHA-256: `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. Native Windows Computer Use sky and connected Edge browser interface available. Scaling is Owner-reported 200%, not independently measured; capture dimensions are tool logical pixels (1082x790 normal, 761x790 narrow).

| Check | Result | Observed evidence / limit |
|---|---|---|
| Portable archive initialization | PASS (subcheck) | Owner authorized app-adjacent directory; clicked create portable directory, first-use prompt disappeared, settings showed package `download`, directory exists, same path after two normal restarts. No real archive was submitted. |
| Native folder picker cancel | PASS (subcheck) | Opened real folder chooser, cancelled with Escape, unchanged `download` path. Cached cancel element failed; fresh observation and keyboard recovery succeeded. Selection/path rejection/migration tests remain NOT_RUN. |
| Settings middle/narrow and logs layout | PASS (observed subchecks) | aria2 controls, Extension upper/lower controls, Storage and Logging viewed at 761px; no observed overlap. Logs normal/narrow checked/unchecked follow control works, checked green. Header buttons wrap at narrow width. Multi-scale matrix and new/live log-follow behavior remain NOT_RUN. |
| Error and Silent save/restart persistence | PASS (subcheck) | Saved each, visible saved feedback, each survives normal app restart; Error config `level: error` and rotated log header `level=error` read. Original Debug restored; config confirms `level: debug`, `max_files: 5`. Does not prove old-config upgrade, controlled child-process filtering or stable release default info. |
| Extension loaded | PASS (Owner manual subcheck) | Owner loaded extension and supplied Edge card showing XArchive 0.2.1 enabled. No automated extension-install claim. |
| Browser pairing/real archive | NOT_RUN / waiting for Owner | Desktop shows unauthenticated/not_loaded; Owner says popup/options open but not connected, screenshot pending. Loading alone does not prove Native Host, authentication or real archive. |
| Browser extension management automation | BLOCKED | Browser tool rejects `edge://extensions/`: URL policy permits only http/https and forbids workaround. Owner performs extension-page operations; do not bypass using alternate browser surfaces. |

Tooling observations are not product FAIL: one helper initialization binding error recovered; simultaneous Owner file-explorer work caused wrong-surface capture, agent paused and resumed only after Owner returned desktop control; folder-picker cache error recovered via Escape; WebView dropdown coordinate target rejection recovered using fresh observation and keyboard. One invalid observation option corrected without product changes. No protocol, production code, package or security settings changed.

Owner clarified current Extension token is one-time and auto-rotates; manual replacement requirement withdrawn, not recorded as an independently tested rotation PASS. No token values saved in committed records or retained evidence. Existing historical exposure records retained. Current Manual queue should no longer ask Owner to rotate it.

Evidence: local untracked `validation-artifacts/computer-use-followup-20261002/`, SHA-256 manifest `screenshots.json`. Representative screenshots: `picker-cancel.png` SHA-256 `104815116A22C485FD8A866A1162766D8ADC293E3B2AF75EAE53BD0F91F98588`; `error-restart.png` `21F18B696A5A9B1110D8F2B70BAA568D134ED2F3F42E70756ED23B57CDD87708`; `silent-restart.png` `05C7CBA4213A557F5F7644AB28178092D0B1E21A87C87AFF06121B06890A3B8E`; `debug-restored.png` `60BC488DD8D8EBCB6E5ED22FFACBD563C39CE23E718EE04EF070F894085D8C03`. Image payloads remain local. Windows Owner retains native tests; Cross-platform runtime wiring and prior button-wrap review unchanged.

## 2026-10-02 Extension WebSocket pairing and Desktop restart semantics

Starting documentation revision `60f95f0`, branch `codex/windows-validation-1f14cea`; baseline shared commit `1f14cea` unchanged. GUI evidence binds to v0.2.1-pre1 Full, associated source `3115c3b50716be0155804ad4f94dd9d29e37d617`, EXE fresh SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. Owner manually enabled local WebSocket and saved pairing in Edge extension; browser tool remains unable to operate extension-scheme pages. Windows Computer Use verified Desktop accessibility state without emitting token.

- Initial WebSocket authentication: **PASS (subcheck, combined manual/automated evidence)**. Owner screenshot: WebSocket, authenticated, port 17321. Desktop: authenticated / connected, accepted 1, authentication received 1, success 1; handshake/authentication failure counters 0. This is authentication only, not real archive or Native Messaging acceptance.
- Normal Desktop exit and restart executed via Computer Use. New instance showed unauthenticated / not_loaded, accepted 0, success 0, initially and after a further 10-second observation. **Owner clarified restart resets the token; failure to reconnect with the previous token is expected**. Automatic reconnect with a stale token is not an acceptance requirement for this artifact. These counters do not prove an explicit old-token rejection: no new connection attempt was observed. Independent token rotation/rejection test remains NOT_RUN; Owner statement is expected-behavior provenance, not an automated rotation PASS.
- Current execution pauses before real archive: Owner must re-pair using the current token and supply a permitted public test-post URL. Same-session reconnect, browser restart, Native Messaging, actual media/SQLite consistency and duplicate submission remain NOT_RUN. No product defect or production change inferred from expected restart behavior.
- Tool capture had an extra 13x13 screenshot region; first indexed/coordinate clicks failed. Fresh observation identified actual 1280x760 region and recovered using its returned screenshot ID. Tool targeting failures are separate from product results. Source, branch and release acceptance unchanged.

Evidence local untracked `validation-artifacts/computer-use-pairing-20261002/`: `owner-extension-authenticated.png` SHA-256 `143E4B97E55CB6FFD5CED0AC66F1A58FBD8E6371FF4ABF21978A524C0C7C6276` (Owner token masked); `desktop-authenticated.txt` `3346653030DE32179E6167601EC92EDF58D45C2FC8141EABCE0C698AAF9AC6CF`; `desktop-after-restart.txt` `C044E68C9F3FA74B5741C90771B0EFC919DA3C4F3B0655F8C2019F8E287A9555`. No raw credential saved. Next execution Owner Windows; Cross-platform Telegram wiring and existing button-wrap review unchanged.

## 2026-10-02 Connection-status discrepancy: refreshed Extension and live socket

Starting documentation HEAD `b5b83cd`, same release Full artifact/source identity as preceding pairing record. Owner reports Desktop snapshot: accepted 3, handshake_failed 0, auth_read_failed 0, auth_received 3, auth_succeeded 1, auth_failed 2, auth_response_failed 0, close_before_auth 2, close_after_auth 1; unauthenticated / disconnected. This proves historical authentication success and closure, not the current connection state. Prior first-pair PASS is retained; current re-pair acceptance remains pending explicit Desktop refresh.

Owner refreshed the Extension options page without saving/reconnecting; it still shows authenticated on 17321. Screenshot retained locally as `validation-artifacts/computer-use-pairing-20261002/owner-options-after-reload.png`, SHA-256 `6EA6FEEF8005FDE5583811ECBA8A77970B65F74DF53A99B260D4654E50565347`. Token field blank on reload does not prove storage token deletion: options initialization loads enabled/port and status, not the saved token into the input. Token-bearing Desktop screenshot not copied into evidence or Git.

Read-only native endpoint check (approved escalation after sandbox CIM denied): exactly one Desktop process, PID 44384, target Full EXE path; listener 127.0.0.1:17321 owned by it and established loopback connection to client PID 26228, source port 54689. TCP Established alone does not prove WebSocket authentication. Multiple Desktop instances not observed.

Code inspection: packaged and checkout `extension/options.js` identical SHA-256 `0ED733F03A6BBAC76760AC22F0CC06C5C7B3122F851AC21BE6B5E403B60427A8`, snapshots update on load/save/reconnect, without live subscription/poll. Checkout `desktop/src/main.jsx:44,57-61` refreshes Extension explicitly, while periodic poll only refreshes jobs/batches; navigation preserves parent state and is not an equivalent refresh. Thus the earlier navigation-only Desktop re-pair check was insufficient. Desktop snapshot staleness is an inference, pending an explicit read; connection-close cause remains unclassified. No code change or completed shared-fix claim.

Computer Use indexed refresh click failed with coordinate geometry unavailable (tool failure, not product failure). Owner asked to click the actual Extension section refresh once after the options reload; current execution pauses for that result. Real archive/duplicates, browser restart and Native Messaging remain NOT_RUN. Windows Owner continues native evidence; any proposed shared connection-status implementation change routes to Cross-platform Owner review.

Owner then explicitly refreshed Desktop Extension section: still unauthenticated/disconnected, with accepted 8, auth_received 8, auth_succeeded 6, auth_failed 2, close_before_auth 2, close_after_auth 6; all handshake/auth-read/auth-response failure counters 0. Thus the navigation/snapshot-staleness hypothesis alone does not explain the refreshed observation. Initial authentication PASS remains historical; sustained/current re-pair connection is **FAIL (observed subcheck; root cause unclassified)**, real archive remains NOT_RUN. Do not equate failed authentication count 2 with all later closures: six authentications succeeded and six post-authentication sessions ended. No product implementation changed.

Candidate cause, not established: packaged Manifest V3 worker has no periodic heartbeat in inspected bridge/background code. Chromium guidance requires traffic inside a 30-second activity window to keep the extension WebSocket worker active: [Use WebSockets in service workers](https://developer.chrome.com/docs/extensions/how-to/web-platform/websockets). Actual Edge idle timing not measured; do not assert that Chrome guidance proves this Edge failure. Owner offered a bounded same-session test: reconnect once, explicit Desktop status reads around 5s and 45s, no Desktop restart or worker DevTools (which can change lifetime). Execution awaits Owner choice/manual browser action because extension-scheme automation is unavailable. Shared Extension/status changes, if pursued, route to Cross-platform Owner; Windows owns native timing evidence. Raw token-bearing new screenshot not retained in evidence/Git; numerical evidence comes from Owner message and screenshot observation.

## 2026-10-02 Current-token recovery, subsequent closure and Owner pause

Starting documentation revision `8beacbd406a21aac777a6b0dac84c7384bae2a51`, branch `codex/windows-validation-1f14cea`, tracked tree clean before this documentation-only update; untracked environments/artifacts preserved. GUI identity reused: Full v0.2.1-pre1, source `3115c3b50716be0155804ad4f94dd9d29e37d617`, build Run 36838400270, previously verified EXE SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`. No fresh executable hash or automated UI execution claimed in this follow-up. Native Windows, Owner 200% display/Edge environment unchanged as reported.

- **PASS, current-token authentication subcheck (Owner manual)**: Owner reports Save and Connect showed authentication failure before entering the new token; failure-state screenshot was not supplied and the exact submitted credential is not established. After new-token entry, screenshots show Extension authenticated on 17321 and Desktop authenticated/connected. Desktop accepted/auth_received 16, auth_succeeded 10, auth_failed 6, close_before_auth 6, close_after_auth 9; handshake/auth-read/auth-response failures 0. This supersedes the prior FAIL for current re-pair only; cumulative closure counts alone had not established sustained idle failure.
- **FAIL, observed connection persistence/state agreement subcheck (Owner manual)**: subsequent explicit Desktop section refresh showed unauthenticated/disconnected; close_after_auth increased from 9 to 10 with other counters unchanged. Extension options still showed authenticated. This newly observed delta documents closure after the successful authentication snapshot, independently of the older cumulative totals. It does not establish that refreshing caused closure, exact session duration, which peer initiated it, or the MV3 hypothesis. Authentication PASS remains distinct; production root cause unclassified.
- **NOT_RUN**: controlled same-session 5s/45s idle measurement, real media/SQLite archive and duplicate submission, browser restart and Native Messaging acceptance. Owner supplied authorized public test URL https://x.com/thsottiaux/status/2105039482013757749 , then explicitly selected pause and Cross-platform investigation. URL recorded only; no page access, submission or download in this follow-up.

Evidence: Owner chat and four supplied screenshots (two before/after recovery, two after refresh). Safe masked Extension recovery screenshot retained locally/untracked as `validation-artifacts/computer-use-pairing-20261002/owner-current-token-authenticated.png`, SHA-256 `B6A8E4BD16A406FB064A89F87BE2F3BF7E18C77064211D1A572D31AF74742BE4`. Raw token-bearing Desktop screenshots were not copied into retained evidence/Git; only state/counters transcribed. Prior observations and classifications remain historical, with scope correction above.

`CROSS_PLATFORM_REVIEW_REQUIRED`: next Owner Cross-platform Owner should investigate shared Extension worker/WebSocket lifecycle and options status snapshots together with Desktop observation; identify closure initiator/timing before proposing a fix. Windows retains native reproduction and acceptance after Git handoff. No shared contract change established, no production modification, no release approval; this investigation does not block unrelated Telegram work. Documentation checks only: docs audit and diff check; handoff revision is the commit containing this record pushed to the named branch.

## 2026-10-02 Windows targeted revalidation — b015fbe

Input/implementation/validation source `b015fbe81a0b47c2b486a5256bd81ac95fd98d25`; branch `codex/windows-validation-1f14cea`, remote `origin` (`dev` explicitly fetched because the default fetch selects the validation branch). Clean tracked start at `7799afc`, fast-forwarded through Git to the exact supplied handoff; local untracked environments, components, logs and validation artifacts preserved. The source handoff still says CROSS_PLATFORM_IN_PROGRESS but explicitly assigns the next Windows checks; this run accepts that concrete scope, not an assertion that Telegram wiring is complete. Current Plan remains Telegram Batch A with runtime/UI/credentials unwired. No Windows implementation changes or direct source sync.

Environment: native Windows/NTFS E: workspace, PowerShell, Node v24.19.0, Cargo 1.98.0, rustc 1.98.0. Computer Use sky window inventory works; initial inventory contained no XArchive window. Edge browser connector available (id 3); extension-scheme operations remain manual under browser URL restrictions. Owner manually confirmed loading the current checkout Extension. Scaling reused from Owner's 200% environment report, not independently measured.

Scope: Targeted transport/background tests plus affected UI/native build; Full regression not run because this is a bounded connection-status change without dependency/schema changes. Previous Telegram/storage/icon/layout results stay evidence for their recorded revisions, not promoted to new GUI acceptance.

| ID / target | Method | Result / expected evidence |
|---|---|---|
| WQ-WS-02/03 native transport regression subset | automated | PASS 6/6: live-socket state after recent request, defaults, correct/wrong token, request routing, accepted non-blocking stream. `cargo test -p xarchive-desktop --lib websocket_transport::tests -- --test-threads=1`, exit 0. Test EXE SHA-256 `B0D0C5795CA3B814D589F7A7B0590DFD6E0E25114A6BE362E523756F3E554809`. Loopback tests do not prove Edge lifecycle. |
| WQ-WS-02/03 Extension regression subset | automated | PASS 19/19: `node --test extension/tests/background.test.js`, exit 0, including stale live-state and status-read reconnect cases. Browser/socket fakes do not prove a real MV3 worker. |
| Affected syntax/build | static/build | PASS Extension `npm run check --workspace extension`; UI `npm run build --workspace desktop`; native `node desktop/scripts/build-tauri.mjs --no-bundle` (dev channel, release optimization), all exit 0. Linker informational warnings and Vite mixed-import warning retained, no build failures. |
| Formatting/docs | static | PASS `cargo fmt --all -- --check`, `node scripts/docs-audit.mjs`. |
| Status command test-name probe | automated discovery | NOT_APPLICABLE: filter `extension_status` matched 0 tests (176 filtered), exit 0; this is not a test PASS or additional coverage. No assertions changed. |

Build provenance: fresh current Desktop copied into `validation-artifacts/windows-batch-b015fbe/app/xarchive-desktop.exe`, SHA-256 `B3550F5CEB6764C2D8DD1E06B94D1E143B936B7B28ACA318F8549EDF1677A1F2`. Current Extension copied from the checkout; downloader/gallery-dl/aria2 components reused from the Owner's older Downloads Full package solely for a local integration fixture. This directory is not a new canonical Full build, release asset or packaging PASS. No user config/DB copied from the old package. GUI launch and portable download initialization observed via Computer Use; first-use prompt disappeared and new directory selected. One screenshot initially captured the foreground Edge surface despite Desktop AX state; activating the selected Desktop recovered before input (tooling observation, not product failure).

Evidence local/untracked: `validation-artifacts/windows-batch-b015fbe/`, logs `extension-tests.log`, `websocket-tests.log`, `extension-check.log`, `ui-build.log`, `tauri-build.log`, `status-command-tests.log`, SHA-256 manifest `log-hashes.json`. Native integration, timed idle and real archive results are recorded separately below. Priority P0 for connection acceptance, owner Windows Platform Owner, implementation IMPLEMENTED; blocks_development=no, no release approval. Revalidate after related bridge/transport/build/environment changes.

Shared follow-up identified by current source inspection: handoff asks the Owner to read `last_request_age_seconds` from Desktop diagnostics, but `desktop/src/pages/settings-page.jsx:40` renders only the earlier counters; the new Rust field is absent from JSX. `CROSS_PLATFORM_REVIEW_REQUIRED`: expose it or provide a supported equivalent readout before that exact manual evidence requirement can be completed. Backend unit test covers its value, not GUI visibility. No production edit made to compensate.

### Current native/browser observations and disposition

- **PASS authentication subcheck**: Owner manually loaded checkout Extension and entered the new Desktop's current token; reports authenticated on 17321. Computer Use independently read authenticated/connected at baseline; accepted/auth_received 7, success 1, failed 6, close_before 6, close_after 0, other failure counters 0. Prior wrong/stale attempts remain observed counters, not six product defects or erased retries.
- **FAIL observed connection-persistence subcheck**: later Desktop display was unauthenticated/disconnected, close_after 1, other counters unchanged. Owner explicitly refreshed Desktop then options: Desktop unauthenticated, options authenticated. To account for status-read self-healing, Owner refreshed only Desktop once more; still unauthenticated/disconnected, accepted/auth_received 8, succeeded 2, failed 6, close_before 6, close_after 2, all handshake/auth-read/auth-response failures 0. A new successful authentication and a later close occurred after the options read. No Desktop restart or token change was requested in this sequence. Cause/close initiator/duration unclassified; sequential observations cannot prove simultaneous live-state disagreement or failure of the two cached-flag fixes in isolation.
- **NOT_RUN complete controlled ~5s/~45s idle test**: initial safe display baseline `2026-10-02T05:54:18.204Z`, later safe snapshot `2026-10-02T05:58:14.788Z` (13:54:18.204 / 13:58:14.788 Asia/Shanghai), gap 236.584s. An explicit 45s tool wait was performed, but subsequent manual reading latency prevents a 45s close-timing claim. `last_request_age_seconds` absent from UI; no equivalent IPC read performed. Evidence `pairing-t0.txt`, `pairing-later-snapshot.txt`, Owner messages and final counters.
- **NOT_RUN real archive/duplicates**: authorized X URL was read through Edge; target main post and XArchive save button visible. No save click/download/SQLite completion occurred. Execution paused on reproduced connection closure under Owner's earlier choice to hand issues back to Cross-platform. Reused Sidecar components were copied as a fixture, not exercised or accepted as current-source workers. Native Messaging/worker/browser restart and pending cleanup remain NOT_RUN; Telegram entry points remain IMPLEMENTATION_NOT_READY.

Final follow-up: `CROSS_PLATFORM_REVIEW_REQUIRED` for post-auth closure/status-read reconnect investigation plus missing diagnostic readout. Existing backend and bridge regression PASS remain; native lifecycle acceptance is not closed. Next Owner Cross-platform Owner, Windows retains controlled real-browser/native reproduction after the next handoff. No production/security/credential code changes. Tauri CLI rewrote Cargo.toml line endings only (content diff empty), restored that tool-produced change before final documentation review. No Full regression or release approval; validation/handoff revision is this record's Git commit.

## 2026-10-02 automatic pairing native implementation continuation

*Restored verbatim from evidence commit `30fc575` (branch `codex/browser-automatic-pairing-windows`), where this record was originally written; the section was previously absent from this branch.*

Owner Windows; branch `codex/browser-automatic-pairing-windows`; source `0c74087cf26d7120dfe6bbabb7c66b37b879fe3e`; tested implementation is this commit's diff. Native Windows/PowerShell, Rust/Cargo 1.98, Node 24.19; existing E: caches/user data preserved, Git fetch/fast-forward only. Debug Host SHA-256 `dff2ecd767bf3d9b8d44c7724e732547466fffafaa365546cd61f40677de84ee`.

- PASS: Host library 11 + entry 1 + Windows process framing 1 (`cargo test -p xarchive-native-host --offline`). Stalled read/write deadline test checks real pipe backpressure. Unix integration NOT_APPLICABLE.
- PASS: Desktop Pipe/Registry targeted 4 and WebSocket targeted 12; `cargo test -p xarchive-desktop windows_transport --offline`, `cargo test -p xarchive-desktop websocket_transport --offline`. Existing business framing preserved; ticket consumption, exact manifest identity, duplicate listener and shutdown covered.
- Initial FAIL: bootstrap test received empty response before data arrived on nonblocking pipe; bounded retry of zero-byte reads repaired it. Initial Desktop module run hung in shutdown test and was terminated by its exact test process ID. First timeout fix still FAILed <1s at ~3s; stop-aware pre-auth polling and authenticated polling then PASS, unchanged assertion.
- Strict Clippy initially FAILed existing Registry collapsible-if; localized repair then PASS. Cargo incremental cache finalization AccessDenied notes and MSVC linker output warnings are tooling notes, not product failures. CIM process inventory was denied; Get-Process provided equivalent test-process identification.
- Desktop module rerun: 186/187 PASS, one config persistence test FAILed with AccessDenied under sandbox. The exact unchanged config test PASSed under permitted escalated execution, confirming an environment restriction; no config code changed. This is not a claim of a clean full-module run.
- Read-only Registry inspection: HKCU Edge and Chrome host keys ABSENT. NOT_RUN: installed registration/repair, cross-user ACL denial, Full package, native GUI, actual browser/MV3/E2E; prerequisites and current scope are in the current queue. No release publication or real archive performed.

`CROSS_PLATFORM_REVIEW_REQUIRED`: stop-aware shared WebSocket wrapper with Windows-only polling. Return through Git for Linux review/revalidation; no schema/protocol change.

## 2026-10-02 — exact 6060f5a Windows validation return

Source/handoff: 6060f5a2d554d32fa8400f7f5084319b19fabeb1; validation branch: codex/windows-validation-6060f5a. Native Windows PowerShell, Node and Rust/Cargo are available. Explicit Git fetch retrieved this SHA; tracked tree was clean at validation start. Existing untracked caches, artifacts and user data were preserved. No filesystem synchronization or production-code change.

Revision prerequisite: d65a01bfe2c6f93a071322bfef154fd947b0b70b is not an ancestor of this source (git merge-base --is-ancestor exits 1). Its Windows Host/Named Pipe/coordinator and stop-aware polling implementation, and 30fc575 evidence, remain on the prior Windows branch. Prior targeted PASS results do not apply to the missing implementation at 6060f5a.

| Check | Result | Evidence / limitation |
|---|---|---|
| Targeted package/UI Node tests | PASS 27/27 | node --test desktop/test/extension-package.test.mjs desktop/test/ui-state.test.mjs on Windows |
| Native Host release build | PASS | cargo build -p xarchive-native-host --release --offline |
| Canonical Full build | PASS | node desktop/scripts/build-portable-windows.mjs; fixed Extension ID; output dist-portable/validation-6060f5a; no binary-reuse fallback. Existing packaged worker dependencies reused; no fresh-worker or GUI acceptance claim. Rust unused pairing members and Vite chunk warnings are nonfatal. |
| Actual packaged Host framing diagnostic | PASS | Spawn packaged Host with fixed browser origin, one little-endian length-prefixed bootstrap frame, five-second process bound. Correlated response: BOOTSTRAP_NOT_IMPLEMENTED, retryable=false. This proves an implementation prerequisite failure, not successful pairing. |
| Actual Extension inventory | PASS scoped | extension-package.mjs plan + verify against Full extension directory: 13 files; src/browser-pairing.js exists. No archive extraction/install acceptance performed. |
| Full installation inventory | FAIL scoped | package-manifest.json installation.files still omits extension/src/browser-pairing.js, although the actual file ships. Shared native-host-package.mjs list is unchanged by 6060f5a. |
| Automatic pairing / UI mismatch reproduction / E2E / lifecycle | NOT_RUN | IMPLEMENTATION_NOT_READY on the exact tested 6060f5a source, which lacks Windows bootstrap. Do not replace working registration or claim historical d65a01b screenshots against this artifact. Subsequent Git integration is tracked separately and has no Windows result in this receipt. |

Artifact: E:/Shiraishi/VSCode Workspace/Tw2Tg/dist-portable/validation-6060f5a. SHA-256 Desktop 61cd19f0f9a192dbf21d1e494e360b30859134f7f8bbbda99c62809e2d3c6f86; Native Host 06d5fd6ff0296d5ec09d2771de08979cac5699c14214f47e6c495c0d114bbc81; browser-pairing.js ace9ba7defb8eb452f2f94e6da3c643787c2d3b721dbd13c09413e1b21efe351. Local probe and Extension inventory evidence: validation-artifacts/6060f5a/result.json and extension-plan.json. Extension per-file hashes: validation-artifacts/6060f5a/extension-hashes.json, inventory SHA-256 4c1c38f6bc76edafd19f82998d9d38851925f17d7774ae1e72975eb4721f3a6a. These local artifacts are not committed. Documentation audit and final diff whitespace check PASS.

Reconciliation clarification: earlier text describing a Full package as missing the helper must not be read as proof of an absent on-disk file. The prior d65a01b artifact shipped the helper but omitted it from installation.files; preserve the historical FAIL and this clarification. 6060f5a adds the standalone Extension required-file guard, not the Full installation-list correction.

Next owner: Cross-platform Owner. CROSS_PLATFORM_CHANGE_REQUIRED: integrate the Windows branch through Git and identify the resulting source revision before re-handoff. CROSS_PLATFORM_REVIEW_REQUIRED: review Windows stop-aware shared WebSocket wrapper, and fix Full installation inventory while preserving package abstraction. Shared UI/status and automatic-options semantics require their own review; do not infer the authoritative state from screenshots.

Manual queue after integrated source: rebuild and bind Desktop/Host/Extension hashes; register that Host; capture redacted browser_connection, websocket_connection, websocket_authenticated plus sidebar/detail at connect and 5s/45s idle; query status; execute the authorized archive fixture and duplicate/response-loss cases; MV3 worker/browser/Desktop restart and sleep; Edge/Chrome profiles; current-user/cross-user ACL; install/upgrade/uninstall/move and redaction. All remain NOT_RUN on this source. Previous automation BLOCKED and product FAIL records remain attached to their original artifacts.

## ca25e53 fresh Full verification directory — 2026-10-05

Build HEAD 9f383c0f08d70079880a652c5b3203e6db1786f7; code source ca25e5379302431a3130436896735b8be2bb4744. Fresh Native Host cargo --locked --release and worker PyInstaller build exit 0; canonical Full build exit 0 with no Desktop binary-reuse flag. Output dist-portable/XArchive-full-validation-ca25e53-20261005. Installation declared files all present, browser-pairing.js present, 79 nonempty package files before adding VALIDATION-STEPS.md. External gallery fixed SHA80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a and aria ZIP SHA67d015301eef0b612191212d564c5bb0a14b5b9c4796b76454276a4d28d9b288 verified before use; original local gallery preserved and restored. Packaged worker hello ready / shutdown process exit0 PASS smoke; gallery --version 1.32.13:2026.09.20 and aria --version1.37.0 exit0. Desktop SHA256 37344cb7777152af1baf76eec85480e432e43ceaf44baae8ee31f8c598fe1b5e. Component hashes/inventory/logs: validation-artifacts/windows-full-ca25e53/ (local). Full version metadata0.2.1, dev channel; not release publication or service acceptance. Automatic browser pairing, real archive/duplicate, Telegram Cloud/Channel and Local tests NOT_RUN; current handoff state WINDOWS_VERIFICATION_PENDING. Manual recipe: [Full validation steps](windows-full-ca25e53-manual.md). Local Bot API Server and credentials not packaged. Windows Owner executes manual queue; Cross-platform Owner reconciles evidence.

Static recipe follow-up: current Extension options retains port/token fields, but save does not request legacy mode; normalizeWebSocketSettings defaults automatic and discards token. Manual guide therefore tests fresh-profile automatic bootstrap first and does not promise UI legacy-token configuration. Cross-platform Owner should reconcile the legacy-looking options UI with supported modes; no product code changed and automatic pairing remains NOT_RUN.

## ca25e53 Full manual integration results — 2026-10-05

Code source ca25e5379302431a3130436896735b8be2bb4744; build HEAD 9f383c0f08d70079880a652c5b3203e6db1786f7; prior documentation b82138b4cdfab21eb9acad5603fed02754945e5d. Artifact dist-portable/XArchive-full-validation-ca25e53-20261005, Desktop SHA256 37344cb7777152af1baf76eec85480e432e43ceaf44baae8ee31f8c598fe1b5e. Evidence: Owner manual reports, three inline screenshots and supplied runtime log xarchive-1791174793868.log. These results supersede only the corresponding NOT_RUN subchecks for this artifact; historical results remain intact.

| Check | Result | Scope and evidence |
|---|---|---|
| Browser Extension load and initial automatic connection | PASS manual | Full Extension directory shown; dynamic port 53095, authenticated/connected; accepted/auth_received/succeeded 1, failures and closes 0. Sidebar services connected. Controlled 5s/45s persistence and restart recovery not established. |
| Real Extension task creation | PASS manual | Tweet 2105039482013757749; job archive-2105039482013757749-2026-10-05T04:35:22Z created. |
| Real archive download | FAIL observed | EXECUTOR_WORKER_FAILED / ARCHIVE_DOWNLOAD_FAILED / DOWNLOAD_TIMEOUT: gallery-dl timed out. Completed 0, failed 1. Root cause unclassified; not a browser pairing failure. |
| Telegram authentication and explicit test message receipt | PASS manual | Owner reports successful verification; XATest Channel screenshot contains XArchive explicit connection test at 12:39 and 12:42. Endpoint mode not captured; does not prove automatic archive/media delivery, rotation or Local Bot API acceptance. |
| Confirmation dialog | FAIL observed runtime | Three unhandled_rejection events: Command plugin:dialog\|confirm not allowed by ACL. Triggering user action not captured; cannot associate with download timeout or successful explicit sends. |

Supplied log: Sidecar hello ready; proxy_mode=system, proxy=None, proxy_configured=false, proxy_active=false; extraction timeout 300s, discovery 600s, transfer 1800s. This establishes that no active proxy was detected, not that network/proxy/authentication caused the timeout. No gallery stderr, extraction progress or task-correlated timeout event supplied. Do not extend the timeout or change production code solely from this evidence.

Remaining NOT_RUN: successful real download and output integrity; duplicate/response-loss handling; automatic Telegram archive/media upload and receipt; restart/rotation/Local Bot API matrix; controlled connection idle/restart tests; remaining GUI queue. Prerequisite for archive-to-Telegram checks is a successful authorized download. Next diagnostic: capture redacted task-specific gallery stderr/progress and verify X access, required authentication and intended proxy without exposing cookies/tokens. Separately reproduce the action invoking confirm and review Tauri capability authorization. CROSS_PLATFORM_REVIEW_REQUIRED for shared extraction diagnostics and confirmation call/capability routing; Windows Platform Owner retains native reproduction and remaining acceptance, Cross-platform Owner reviews shared findings. No production changes or real-message retry performed by the agent.

## ca25e53 follow-up manual evidence — 2026-10-05

The Owner confirmed the successful standalone `gallery-dl https://x.com/thsottiaux/status/2105039482013757749` run used the same `gallery-dl.exe` shipped in the Full artifact and produced `twitter/thsottiaux/2105039482013757749_1.jpg`. The supplied image is readable. This is PASS for standalone extraction/download with that binary and URL. XArchive still reports `DOWNLOAD_TIMEOUT` for the task recorded above. The result narrows the difference to invocation context (arguments, working directory, environment/configuration or child-process handling); it does not yet identify which factor causes the timeout. No task-specific gallery stderr/progress was supplied.

Owner also confirmed the chosen download root `E:\Shiraishi\Downloads\XArchive` exists and currently contains an empty `_staging` directory. This verifies directory setup only; no completed archive output is evidenced.

After Desktop restart, Extension did not reconnect by itself and showed disconnected. Owner clicked Extension “Reconnect”; subsequent screenshots show authenticated/connected and Desktop transport healthy without manually entering a token. One captured session showed actual port 49156 while the options form retained prior port 56646; a later capture showed actual port 64619 and one accepted/authenticated/successful connection with zero failures or closes. Record manual reconnect as PASS; automatic post-restart recovery as FAIL observed. Because capture delays were not provided, this does not establish a recovery deadline. The port mismatch is a separate stale-settings-display observation; captures may be from separate reconnect sessions and do not establish concurrent state corruption.

These follow-ups do not change the prior download FAIL or completion status. Remaining: inspect task-scoped redacted gallery invocation/stderr and reproduce confirmation-dialog ACL rejection; verify successful archive and duplicate behavior, automatic Telegram archive/media delivery, and timed automatic reconnection. CROSS_PLATFORM_REVIEW_REQUIRED remains for shared extraction diagnostics and confirm authorization. Next Owner: Cross-platform Owner for review; Windows Platform Owner for native reproduction and remaining manual acceptance. Evidence consists of Owner reports and inline screenshots in the validation conversation; no secrets or bot credentials are recorded here.


## Windows ca25e53 visual validation return — 2026-10-05

State: WINDOWS_VERIFICATION_PENDING. Branch: codex/windows-validation-ca25e53.
Input/handoff/validation source: ca25e5379302431a3130436896735b8be2bb4744;
Cross-platform source: 4f119ca76694d4ec1063435d88bd33d66d9fae00.
Return revision: the Git documentation commit containing this entry.
Current Owner: Windows Platform Owner; next Owner: Windows Platform Owner for
remaining GUI acceptance; Cross-platform Owner reconciles this Git result and
continues existing advisory work. No production code or shared contract changed.
Tracked tree clean before checkout; local untracked dependencies/data retained.

Scope: Targeted presentation tests and fresh native Desktop build/rendering.
PASS: node --test desktop/test/ui-wiring.test.mjs desktop/test/telegram-render.test.mjs
47/47; node desktop/scripts/build-tauri.mjs --no-bundle exit 0, including Vite
production build. Existing mixed static/dynamic Tauri import warning and MSVC
linker informational warnings retained. Full regression not run: only React/CSS
presentation/copy changed; Sidecar, shared Rust, lockfiles and workflows unchanged.

Artifact: target/release/xarchive-desktop.exe; SHA-256 d9457efe8154f165650d1bd18a49844aed9d7d19a37f8d6d6e12350980e67ccc.
Native environment: Windows 10.0.29680 x64, Node 24.19.0, Cargo 1.98.0;
local dev release channel. This is a fresh --no-bundle executable, not a Full package.
Evidence: validation-artifacts/windows-ca25e53/targeted-tests.log, native-build.log,
evidence.json, gui-disclosure-observation.txt, extension-refresh.png (local),
and inline Computer Use screenshots.

Scoped GUI PASS: current 1082x790 and maximized 1280x752 logical window captures;
concise collapsed summaries, shared Bootstrap/Telegram header spacing and
monochrome icon treatment, Extension puzzle icon; Sidecar and Extension expanded
status omit duplicate icons; detailed Sidecar/Telegram help remains in expanded
content; mouse expansion/collapse and Sidecar Space expand/Enter collapse;
Extension sidebar opens/expands/scrolls to matching panel; refresh leaves it open.
SQLite navigation also observed. Multiple Telegram/storage/Extension panels stayed
open independently. No credential/settings save or real message send performed.
DPI was not independently queried/changed (Owner previously reported 200%); these
window captures do not prove the full 100/125/150/200% matrix or a narrow breakpoint.

NOT_RUN/manual: full DPI/narrow-width matrix, all-panel Enter/Space and hidden Tab
order, focus visibility/navigation focus, fake-token masking/busy/error states,
and independent backend-effect checks for folding. Prior older-artifact GUI results
are not promoted to this changed presentation. Automation reported document-level
focus only, so rendered toggle behavior does not prove full focus acceptance.
Tooling: initial UIA Extension click landed on SQLite after maximize; first
coordinate attempt selected a 13x13 auxiliary screenshot and was rejected.
Recovered using the returned 1280x752 main screenshot; intended Extension jump
and refresh then succeeded. No product defect established or acceptance weakened.

Inherited remaining queue: cancel/shutdown fake-descendant subtree cleanup,
isolated upload=false release transfer/rehearsal and default-branch CodeQL, real
Telegram/Channel/archive acceptance. Deferred as unrelated to this presentation
change and existing Owner local/CI scope. Their prior states/evidence are retained.
No new CROSS_PLATFORM_CHANGE_REQUIRED or CROSS_PLATFORM_REVIEW_REQUIRED.
Cross-platform follow-up: reconcile this documentation return through Git and
continue unresolved Linux glib/WDIO advisory work; no shared implementation fix
requested by this batch.

## Windows 6cf4cc8 return — 2026-10-05

Exact input 6cf4cc8, Windows repair dc5ef1f, shared UI repair 297a237. Original compile/render FAIL and protocol fixture FAIL retained; repaired builds/module/scoped UI PASS. Commands, hashes, environment, tooling limits and manual queue: [batch record](windows-6cf4cc8-results.md). Next Owner Cross-platform Owner for Git integration/review; Windows retains manual acceptance.

## Windows 130affaf validation return

Exact input 130affaf, fixture repair 8f7fde6, Toggle repair/final source 8e0169a. Original Rust fixture and Enter FAIL retained; scoped repaired/module/build/native GUI results and activation blockers are in [batch record](windows-130affaf-results.md), including exact hashes and manual prerequisites. Native PAC failed-policy detection remains a static follow-up, actual routes NOT_RUN. Next Owner Cross-platform Owner for Git integration/review; Windows retains manual acceptance.

## 2026-11-17 — scoped Full window and keyboard check

- Source: `6af046f9fbfdd839cd37ac2e3617bf16806ba2c5`, tracked tree clean; pre-existing untracked `dist-portable/` and `sidecar/xarchive-downloader/` were preserved. The isolated Full package assembled successfully (exit 0) with the controlled worker and pinned downloader inputs. Desktop executable SHA-256: `cab55a9241178f33fa2fce3c6e5ef4c9b13e8dbcefdb28300fc556cd75f993da`.
- **PASS, scoped:** On the current Windows desktop, the first-run workspace rendered and created the package-local portable download directory. Two Tab presses moved the visible focus through the workspace refresh and all-tasks buttons; Enter opened the empty task-record page. Both observed pages rendered legibly at the current window size. The test process exited after Alt+F4. Screenshot: `target/w6-worker-ci-sim-0711a74/full-visual-6af046f-jobs.jpg`; assembly log and exit record: `target/w6-worker-ci-sim-0711a74/full-visual-6af046f.{log,exit}`.
- **NOT_RUN:** Full keyboard navigation, additional pages, alternate window sizes, system DPI values and the DPI matrix. This single-window observation does not close W7. No real downloader execution or browser registration was performed.
