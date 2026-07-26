# CareerCraft Windows release build

Build date: 2026-07-22. Target: Windows x64, Tauri 2 system WebView2, NSIS current-user installer.

## Reproducible build

From `tauri_app`:

```powershell
npm.cmd run build
$env:CARGO_TARGET_DIR="$PWD\src-tauri\target-final"
npx.cmd tauri build --features desktop
```

The dedicated Cargo target directory prevents release linking from blocking the normal test target. The first NSIS build downloads Tauri's pinned NSIS 3.11 package and `nsis_tauri_utils` DLL from official GitHub Releases; subsequent builds reuse the verified cache.

## Verified artifacts

| Artifact | Size | SHA-256 |
|---|---:|---|
| `release/CareerCraft-Setup-0.1.0-x64.exe` | 4,447,595 bytes (4.24 MiB) | `B4710E5CB12071FFE3D8960296FF8AC8157FA83C6EE707089D33118097A7C18F` |

Both artifacts are currently unsigned (`Authenticode: NotSigned`). Public distribution requires an Authenticode certificate and CI signing step; signing will change the hashes above.

## Gates completed

- `cargo test --all-features --offline`: 169/169 passed.
- `cargo check --features desktop --offline`: passed.
- Frontend: 23 files/113 tests passed; 100% statement/line coverage across the configured API, state, App, pages, and actions scope.
- `npm.cmd run build`: TypeScript and Vite production build passed twice.
- `npm.cmd run desktop:build`: application and NSIS bundle passed.
- Windows resource manifest contains `Microsoft.Windows.Common-Controls` `6.0.0.0`; this prevents the `TaskDialogIndirect` loader error.
- Silent NSIS install and uninstall returned exit code 0 on the local Windows host.
- Uninstall preserved `%USERPROFILE%\.careercraft\career.db` byte-for-byte.
- Installed application opened a responsive `CareerCraft` window with a nonzero native window handle; this is a launch smoke, not selector-ready WebView E2E evidence.
- Existing 9-table database migrated to schema 1–4; integrity `ok`, FK errors `0`, and the pre-migration backup was created.

## Cold-start and health smoke

For installed-build acceptance, use a clean Windows 10 and Windows 11 VM:

1. Start a stopwatch immediately before launching CareerCraft from the Start menu.
2. Stop it when the primary window is visible and its first local dashboard state is rendered; record p50/p95 over five cold launches after reboot. Target: <=2 seconds on the release reference machine.
3. From the WebView test harness call Tauri command `health`; require `{ "success": true, "data": { "healthy": true, "service": "careercraft-core" } }`.
4. Call `version`; require `0.1.0`.
5. Verify `%USERPROFILE%\.careercraft\career.db` opens, migrations 1/2/3/4 are recorded, and offline persona/experience CRUD works.
6. Close the window and verify the process exits. Reopen from the installed shortcut, then uninstall and confirm user data follows the documented retention policy.

The local interactive smoke proves primary-window creation. Cold-start p50/p95, WebView2 missing/damaged scenarios, SmartScreen, and Defender must still be run against the signed installer on clean VMs.
