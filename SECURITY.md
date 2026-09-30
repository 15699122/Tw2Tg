# Security Policy

## Supported versions

XArchive is currently a pre-release project. Only the current development line
on the default branch (`main`) and the latest published pre-release receive
security fixes. Older pre-releases are frozen historical artifacts and are not
patched.

| Version | Supported |
|---|---|
| `main` (current development line) | Yes |
| Latest published pre-release | Yes |
| Any older release or pre-release | No |

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability, and do not include
real credentials in any report.

Use GitHub's private vulnerability reporting for this repository:

**Security → Report a vulnerability** on `https://github.com/15699122/Tw2Tg/security/advisories/new`

If private reporting is unavailable to you, open a regular issue that contains
only a description of the issue class and no exploit details, or contact the
maintainer through their public GitHub profile.

Please include:

- affected version, commit or asset;
- platform (Windows or Linux) and component (Desktop, Extension, Sidecar, worker, release workflow);
- reproduction steps or a proof of concept;
- the expected and observed behavior;
- the impact you believe is reachable, and whether it needs a real X/gallery-dl
  or Telegram account.

## What to report

- Remote or local code execution, including archive extraction escape.
- Credential, cookie or token exposure through logs, SQLite, archives or
  release assets.
- Path traversal, symlink/junction escape or unsafe archive handling.
- Desktop IPC or Native Host boundary bypass.
- Release workflow or packaging integrity issues.

## What not to report

- Findings that require an already-compromised developer machine or CI runner.
- Self-XSS, or issues that need the reporter's own credentials to work.
- Denial of service that only affects a developer's own local test run.
- Missing hardening in a dependency that is not reachable from this project's
  configuration, unless you can show a reachable path.
- Missing hardening in test-only tooling, unless the affected path runs in a
  release or CI job that holds credentials.

## Response targets

These are targets, not a service-level agreement:

| Stage | Target |
|---|---|
| Acknowledgement | 7 days |
| Triage and severity assessment | 14 days |
| Fix or mitigation plan for high severity | 30 days |

The project is maintained by two platform owners. Reports affecting shared
architecture, protocols or cross-platform behavior are handled by the
Cross-platform Owner (Linux); reports affecting Windows-native behavior,
packaging, installers or release workflows are handled by the Windows Platform
Owner. A fix that changes a shared contract is marked
`CROSS_PLATFORM_CHANGE_REQUIRED` before it is merged.

## Disclosure

Please do not publicly disclose an unfixed issue. Report it privately first so
a fix or an explicit risk acceptance can be recorded. Fixes are disclosed in
the commit history and, when they affect a published asset, in the release
notes for that version.
