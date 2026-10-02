# Current Platform Handoff

Status: `CURRENT`. Prior Telegram Batch A record archived verbatim in [handoff history](platform-handoff-history.md).

## Batch and revisions

- Task: Windows validation of delivered Telegram shared modules, 2026-10-02. [Plan](../development/telegram-local-bot-api-plan.md).
- Branch: `codex/windows-validation-1f14cea`; input remote: `origin/dev`.
- Cross-platform source/handoff, Windows input/implementation and tested revision: `1f14cea6859dc1c0ecec164509579cfe4eb15f1a`. The inherited record's `8583d46` preceded the delivered album/send-core implementation; this batch binds to the actual fetched commit.
- Windows implementation changes: none. Validation-record/handoff revision: the Git commit containing this record, pushed to the branch above.
- Tracked working tree at test start: clean; only validation documentation changed afterward. Local untracked environments, components, logs and artifacts preserved. No direct sync.
- Current state: `CROSS_PLATFORM_IN_PROGRESS`; next Owner: **Cross-platform Owner** for the already planned runtime wiring. Windows verification of delivered shared modules is complete; product acceptance remains pending implementation.

## Windows results

Scope: Telegram/storage modules plus direct Desktop consumer, not a release/full-workspace regression.

- PASS: Telegram 48/48; storage 51/51 (three Unix-only symlink tests not applicable on Windows); Desktop lib 175/175 after binding `PYTHON` to the native Python 3.12.14 interpreter. Windows Desktop test executable compiled successfully. fmt and docs audit PASS.
- Preserved first executions: Cargo sandbox dependency fetch failed with Schannel `SEC_E_NO_CREDENTIALS`; offline dependency cache lacked `mime_guess`; permitted native execution succeeded. First Desktop run 172/175, three discovery stubs failed using the WindowsApps `python3` alias; unchanged tests passed with explicit native Python. No product defect demonstrated by these failures.
- NOT_RUN: `WQ-TG-001`–`009` and `WQ-TG-UNI-01`–`08`, defer reason `IMPLEMENTATION_NOT_READY`. Shared send core exists, but runtime scheduling, commands, UI and Windows credential adapter do not. No real-send/Unigram/large-file/GUI acceptance claimed.
- Manual Windows Validation Queue: [manual steps §K](../validation/windows-manual-steps.md), with entry-point prerequisites and evidence standards preserved; [current queue](../validation/windows-queue.md).
- Evidence, commands, environment and test artifact hashes: [Windows history](../validation/windows-validation-history.md), section 2026-10-02 / 1f14cea; local logs in `validation-artifacts/windows-batch-1f14cea/`.

## Cross-platform follow-up and next Owner

- `CROSS_PLATFORM_CHANGE_REQUIRED`: none newly identified. `CROSS_PLATFORM_REVIEW_REQUIRED`: none; no shared code or assertions changed.
- Cross-platform Owner: reconcile the Windows evidence, complete the planned archive enqueue/claim-loop and Tauri/settings/task wiring, then commit/push a runnable handoff for Windows credential/native work and real-send validation. Keep Bot API send, Unigram display and downloaded-file integrity separate.
- Windows Owner retains native Credential Manager, GUI/lifecycle, external Local API deployment, controlled real-send and receiving-side acceptance after that handoff.
- `WINDOWS_VERIFICATION_BLOCKING`: none. Existing pre-release GUI/Native Host/transfer gates and System Proxy Batch B remain open. No release publication, version change or acceptance approval in this batch.

## Owner manual evidence follow-up — 2026-10-02

Separate release artifact v0.2.1-pre1 was identified at the Owner-supplied Downloads path (EXE SHA-256 `9A29F57A091F6B0CBA75202843FEF19877C1C1FDB24E764BA67C25664F0BAC2A`); seven screenshots support title-bar icon and empty/stopped Dashboard subchecks at reported 100/125/150/200% scaling on 14-inch 2560x1600. This is not Telegram 1f14cea GUI evidence. Full Explorer/taskbar/Alt+Tab matrix remains NOT_RUN, next manual steps §L. Cross-platform Owner should also review start-button two-line wrapping in windowed captures 2/4/6; observation only, no functional FAIL or code change. Detailed source association, artifact/screenshot hashes and limits in windows-validation-history latest DPI section. Owner routing above remains unchanged; no release gate closed.

Follow-up native-icon evidence: Owner's three taskbar screenshots support 100/125/150% PASS; Explorer/Alt+Tab at these scales PASS by Owner manual confirmation (no corresponding surface screenshots). Same EXE SHA-256 rechecked unchanged. Only 200% Explorer/taskbar/Alt+Tab remains NOT_RUN for the icon matrix; full item not closed. Detailed provenance in Windows history; next manual steps §L updated. Other follow-up and ownership unchanged.

Owner 200% clarification completes WQ-ICON-030-06 for the previously identified v0.2.1-pre1 Full artifact: 200% remaining native surfaces PASS by Owner manual confirmation with prior evidence reuse, rather than a new screenshot/automation run. Four-scale icon matrix closed; no tray implemented. Exact reuse limits and unavailable current EXE path recheck recorded in Windows history. Other UI and Telegram work/ownership unchanged.

Computer Use now executed v0.2.1-pre1 Settings: top separator, actual section order, bottom Tab/focus and 761px narrow top/bottom subchecks PASS. Full-page/other-scale keyboard matrix remains NOT_RUN; helper available, bounded tool targeting failures recovered. Safe evidence/hashes in Windows history. Intermediate token-bearing output requires Owner pairing-token rotation; no code/security-setting changes performed. App left at narrow Settings due tool bounds on restore drag. Other Owner routing and planned follow-up unchanged.
