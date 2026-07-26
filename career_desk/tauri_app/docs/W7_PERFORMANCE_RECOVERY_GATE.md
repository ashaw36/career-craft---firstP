# W7 performance and recovery gate

Backend/offline gate (debug profile, fixed local inputs, no provider network):

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --bin w7-gate -- target/w7-gate
```

It exits non-zero on failure and writes `w7-gate.json` plus `w7-gate.junit.xml`. It measures backend database startup, 1,000-experience reads, persona switching, local JD parsing, side-effect-free resume preview, offline CRUD, and WAL reopen durability. Thresholds are deliberately tested in debug builds so passing does not depend on release optimization.

After W6 embedded WDIO produces selector-ready evidence for the exact binary (at least five `startupMs` samples, `readySelector`, `processId`, and preferably `binarySha256`), measure desktop startup distribution and the complete CareerCraft/WebView2 process-tree working set:

```powershell
./scripts/w7-desktop-gate.ps1 -Executable ./src-tauri/target/release/careercraft-desktop.exe -ReadyEvidenceJson ./target/wdio/ready-evidence.json
```

The script never treats a window handle or `Responding` as page readiness. It consumes W6's real selector-ready timestamps, calculates startup P50/P95, and samples the root executable plus descendant WebView2 renderer/GPU processes until five consecutive totals stabilize. JSON/JUnit include successful values, peak and idle-median memory, binary SHA-256, OS/WebView2 versions, process tree, raw samples, and UTC timestamp. Until W6 can provide the selector evidence, this desktop portion is explicitly blocked rather than reporting a false pass.

Migration backup/restore, failed-migration restoration, pending-restore validation, WAL mode, transaction rollback, and conversion snapshot recovery remain covered by the Rust test suite (`cargo test --lib`).
