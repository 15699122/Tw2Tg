# desktop-proxy

- **Path / entry:** `desktop/src-tauri/src/proxy.rs` plus test modules
  `proxy/tests/{routing,client_cache,redaction,socks}.rs`.
- **Responsibility:** resolve the effective proxy route from configuration and the
  environment, build and cache one HTTP client per route, and render decisions
  without exposing credentials.
- **Owner:** Cross-platform Owner. The route logic, caching and redaction are
  shared behaviour, so this file belongs to Linux. Windows-specific system-proxy
  resolution stays Windows-owned and was not exercised here.
- **Depends on:** the configuration section types, `reqwest` and the platform
  environment resolver.
- **Depended on by:** every outbound HTTP call in the desktop backend.
- **Public contract:** unchanged.
- **Tests:** 21 tests moved into routing (10), client cache (5), redaction (1) and
  socks (4). The parent `mod tests` keeps 165 lines of resolver doubles
  (`PerUrlResolver`, `UnresolvedResolver` and their `impl`s) as shared fixtures.
- **Gates to re-run:** Desktop lib regression (365/0/13).
- **Exceptions:** none.
- **Open questions:** a real Windows system-proxy resolution remains Windows-owned.
