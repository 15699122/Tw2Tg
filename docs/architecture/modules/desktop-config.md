# desktop-config

- **Path / entry:** `desktop/src-tauri/src/config.rs` plus test modules
  `config/tests/{exchange,logging,network_proxy,telegram,paths}.rs`.
- **Responsibility:** define `AppConfig` and its sub-sections, the configuration
  exchange contract (export/parse/preview/persist), log level precedence, network
  and proxy settings, Telegram credential handling and portable path derivation.
- **Owner:** Cross-platform Owner. `config.rs` carries shared configuration
  behaviour, so it belongs to Linux under `docs/development/platform-ownership.md`.
  Windows-specific resolver and secret-store integration remains Windows-owned and
  was not touched by this change.
- **Depends on:** `portable::PortablePaths`, the secret store, `serde`, and the
  shared protocol types used by the exchange document.
- **Depended on by:** application bootstrap, settings commands and the proxy layer.
- **Public contract:** unchanged. No type, field, serialized name, default or
  validation rule changed.
- **Tests:** 34 integration-style unit tests now live in `config/tests/`, grouped as
  configuration exchange (3), logging (5), network/proxy (12), Telegram (11) and
  paths (3). `load_document` stays in the parent `mod tests` as the shared helper.
  Test names gained one module segment (`config::tests::<group>::<test>`); no
  registered test was added, removed or altered.
- **Gates to re-run:** Desktop lib regression (365/0/13).
- **Exceptions:** none.
- **Open questions:** none.
