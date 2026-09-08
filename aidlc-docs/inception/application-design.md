# Application Design

## Components

```text
MV3 Extension → Rust Native Host → Named Pipe → Tauri/Rust Desktop
                                                     ├─ SQLite
                                                     ├─ Python gallery-dl Sidecar
                                                     ├─ optional aria2
                                                     └─ Telegram
```

## Design decisions

- Desktop/Rust owns all domain state。
- gallery-dl owns X-specific extraction and default download。
- aria2 can only be added after a dedicated technical spike。
- IDM is not a core backend because its public CLI lacks reliable structured job observability。
- Native Host is a minimal forwarding bridge。
- Files use staging-then-commit semantics。