# Windows desktop E2E gate

This gate drives the real Tauri/WebView2 application through the W3C WebDriver protocol. It never launches Playwright or bundled Chromium. The 12 journeys and their CC-FR mappings live in `tests/desktop-e2e/journeys.json`.

## Prerequisites

1. Build the desktop binary: `npm run desktop:build` (or set `CAREERCRAFT_E2E_BINARY` to an installed/development EXE).
2. Install `tauri-driver`: `cargo install tauri-driver --locked`.
3. Put a Microsoft Edge Driver matching the installed WebView2/Edge version on `PATH`, or configure `tauri-driver` to use it. Tauri documents this Windows requirement in its official manual setup guide.

Run `npm run test:desktop:e2e`. Missing prerequisites produce JUnit failures and exit code 2; journey failures produce screenshots, driver logs, JUnit, and exit code 1 under `artifacts/desktop-e2e/`. A zero exit code is the only pass signal.

The current workstation has the application EXE but did not have `tauri-driver` or `msedgedriver` during infrastructure creation, so no desktop-pass claim has been made.

## Coverage isolation

Vitest writes every process to `coverage/run-<timestamp>-<random>`. `npm run coverage:aggregate` reads only completed `coverage-summary.json` files, writes a PID-unique aggregate, and removes only incomplete run directories older than `COVERAGE_MAX_AGE_HOURS` (default 168). It never deletes an active parallel run or writes a shared aggregate filename.
