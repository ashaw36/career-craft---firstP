param([ValidateSet('win10','win11')][string]$Profile, [switch]$RunDesktopProbe)
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root "artifacts/vm-$Profile"
New-Item -ItemType Directory -Force $out | Out-Null
$cases = @()
function Run-Case([string]$Name,[scriptblock]$Command) { $start=Get-Date; & $Command *> (Join-Path $out "$Name.log"); $code=$LASTEXITCODE; $script:cases += @{name=$Name;passed=($code -eq 0);exitCode=$code;elapsedMs=[int]((Get-Date)-$start).TotalMilliseconds} }
Run-Case 'npm-ci' { npm ci }
Run-Case 'npm-audit' { npm audit --audit-level=high }
Run-Case 'frontend-tests' { npm test }
Run-Case 'production-build' { npm run build }
Run-Case 'production-isolation' { npm run verify:production-isolation }
Run-Case 'rust-tests' { cargo test --manifest-path src-tauri/Cargo.toml --all-features --locked }
Run-Case 'nsis-build' { npm run desktop:build }
if ($RunDesktopProbe) { Run-Case 'desktop-e2e-upstream-probe' { npm run test:desktop:e2e } }
$result=@{profile=$Profile;os=(Get-CimInstance Win32_OperatingSystem | Select-Object Caption,Version,BuildNumber);timestamp=(Get-Date).ToUniversalTime().ToString('o');desktopProbeRequested=[bool]$RunDesktopProbe;cases=$cases;passed=($cases.passed -notcontains $false)}
$result | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $out 'result.json') -Encoding utf8
$result | ConvertTo-Json -Depth 10
if (-not $result.passed) { exit 1 }
