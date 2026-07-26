param([string]$ReleaseExe = '', [string]$ReleaseFeatures = 'desktop')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$base = Get-Content (Join-Path $root 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
$test = Get-Content (Join-Path $root 'src-tauri/tauri.wdio.conf.json') -Raw | ConvertFrom-Json
if ($base.app.windows[0].devtools -ne $false) { throw 'Production devtools must be false' }
if (($base.app.security.capabilities -join ',') -ne 'default') { throw 'Production must load only the default capability' }
if (($base | ConvertTo-Json -Depth 20) -match 'wdio') { throw 'Production Tauri config contains WDIO capability' }
if ($test.app.windows[0].devtools -ne $true) { throw 'E2E overlay must explicitly enable devtools' }
if (($test | ConvertTo-Json -Depth 20) -notmatch 'wdio-webdriver:default') { throw 'E2E overlay is missing embedded WebDriver capability' }
$dist = Join-Path $root 'dist'
if (Test-Path $dist) {
  $markers = Get-ChildItem $dist -Recurse -File | Select-String -Pattern 'e2e-ipc-bridge|E2E IPC bridge|wdio-webdriver'
  if ($markers) { throw 'Production frontend contains E2E/WDIO markers' }
}
if ($ReleaseExe) {
  if (-not (Test-Path $ReleaseExe)) { throw "Release EXE not found: $ReleaseExe" }
  $binary = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes((Resolve-Path $ReleaseExe)))
  foreach ($marker in @('wdio-webdriver','TAURI_WEBDRIVER_PORT','e2e-ipc-bridge')) { if ($binary.Contains($marker)) { throw "Release EXE contains forbidden marker: $marker" } }
  $tree = cargo tree --manifest-path src-tauri/Cargo.toml --locked --no-default-features --features $ReleaseFeatures
  if ($LASTEXITCODE -ne 0) { throw 'cargo feature tree failed' }
  if (($tree -join "`n") -match 'tauri-plugin-wdio-webdriver') { throw 'Release feature tree includes WDIO WebDriver plugin' }
}
Write-Output 'production isolation verified'
