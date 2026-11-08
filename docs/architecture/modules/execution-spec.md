## <execution_spec>

- **Path / entry:** `desktop/src-tauri/src/execution_spec.rs`; entered through the
  re-exports in `desktop/src-tauri/src/executor.rs`.
- **Responsibility:** Decode a persisted archive execution specification into
  validated business inputs, refusing any version or argument snapshot this
  build cannot interpret faithfully.
- **Owner:** Cross-platform
- **Depends on:** `xarchive_storage` (`BatchOutputSettings`, `BatchNamingMode`,
  `Database`), `xarchive_download::downloader_args`, `serde`, `serde_yaml`.
- **Depended on by:** `crate::executor` (submission, persisted dispatch, recovery).
- **Public contract:** `decode_execution_spec(u32, &str) -> Result<DecodedExecutionSpec, ExecutionSpecError>`
  and `validate_downloader_argument_snapshot(&DownloaderArgumentSnapshot)`. The
  `ExecutionSpecV2/V3/V4` field names and `#[serde(deny_unknown_fields)]` are
  persisted-payload contracts and must not change without an ADR; `Display`
  strings are unchanged by the R1 move.
- **Compatibility entry:** `crate::executor::{decode_execution_spec,
  validate_downloader_argument_snapshot, ExecutionSpecV2, ExecutionSpecV3,
  ExecutionSpecV4, MetadataOutputSettings, DownloaderArgumentSnapshot,
  ToolCompatibility, DecodedExecutionSpec}`.
- **State and ownership:** none. No I/O, session, owner, permit, writer or cgroup
  state is held or produced here.
- **Failure behaviour:** fail closed. An unsupported version, malformed JSON or a
  policy-version mismatch returns `ExecutionSpecError`; callers must not fall
  back to reinterpreting a snapshot.
- **Tests:** `execution_spec::d1_contract_tests` (V4 dispatch, independent export
  formats, refusal of legacy TXT), plus the decode regression tests in
  `executor::tests` and `executor::d1_*` cases.
- **Gates to re-run:** V1-01, V1-02b, V1-03, V1-07 (persisted-spec path reaches
  submission and recovery), and the Desktop library suite.
- **Exceptions:** none in `architecture-exceptions.json`.
- **Open questions:** the D1 production metadata-export caller is still not
  wired; `MetadataOutputSettings::serialize_metadata` is exercised by tests only
  and carries a commented `#[allow(dead_code)]` until that call site exists.
