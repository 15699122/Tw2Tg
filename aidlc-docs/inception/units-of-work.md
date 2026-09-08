# Units of Work

| Unit | Scope | Dependency | Size |
|---|---|---|---:|
| U0 | Monorepo/toolchain/scaffold | None | M |
| U1 | JSON Schema and protocol fixtures | U0 | M |
| U2 | Python Fake/Real gallery-dl Worker | U1 | L |
| U3 | Rust Sidecar Supervisor | U1/U2 | L |
| U4 | Archive Core and Job state machine | U3 | XL |
| U5 | SQLite and FileStore | U4 | XL |
| U6 | Telegram/TagEngine | U5 | XL |
| U7 | Named Pipe Server | U4 | M |
| U8 | Native Messaging Host | U7 | L |
| U9 | MV3 Extension | U8 | XL |
| U10 | Tauri GUI | U5/U6 | XL |
| U11 | aria2 technical spike | U5 | M |
| U12 | Installer and release | U9/U10 | XL |

推荐顺序：`U0 → U1 → U2/U3 → U4/U5 → U11/U6/U7 → U8/U9 → U10 → U12`。