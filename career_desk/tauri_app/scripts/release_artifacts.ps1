param([string]$ReleaseDir = 'src-tauri/target/release', [string]$OutputDir = 'artifacts/release')
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force $OutputDir | Out-Null
$app = Get-Item (Join-Path $ReleaseDir 'careercraft-desktop.exe')
$installer = @(Get-ChildItem (Join-Path $ReleaseDir 'bundle/nsis') -Filter *.exe -File)
$updater = @(Get-ChildItem (Join-Path $ReleaseDir 'bundle') -Recurse -File | Where-Object { $_.Name -match '\.(sig|zip)$' })
$latest = @(Get-ChildItem $ReleaseDir -Recurse -Filter latest.json -File)
if (-not $app -or $installer.Count -eq 0) { throw 'Application EXE and NSIS installer are required' }
if ($updater.Count -eq 0) { throw 'Updater archive/signature assets are required' }
if ($latest.Count -eq 0) { throw 'latest.json is required' }
$files = @($app) + $installer + $updater + $latest
$hashes = foreach ($file in $files) { $h = Get-FileHash $file.FullName -Algorithm SHA256; "$($h.Hash.ToLower())  $($file.Name)" }
$hashes | Set-Content (Join-Path $OutputDir 'SHA256SUMS.txt') -Encoding utf8
$npm = npm ls --all --json | ConvertFrom-Json
$cargo = cargo metadata --manifest-path src-tauri/Cargo.toml --format-version 1 --locked | ConvertFrom-Json
$components = @()
function Add-Npm($node, $name) { if ($node.version) { $script:components += @{ type='library'; name=$name; version=$node.version; purl="pkg:npm/$name@$($node.version)" } }; if ($node.dependencies) { $node.dependencies.psobject.Properties | ForEach-Object { Add-Npm $_.Value $_.Name } } }
Add-Npm $npm $npm.name
$cargo.packages | ForEach-Object { $components += @{ type='library'; name=$_.name; version=$_.version; purl="pkg:cargo/$($_.name)@$($_.version)" } }
$sbom = @{ bomFormat='CycloneDX'; specVersion='1.5'; serialNumber="urn:uuid:$([guid]::NewGuid())"; version=1; metadata=@{ timestamp=(Get-Date).ToUniversalTime().ToString('o'); component=@{ type='application'; name='CareerCraft'; version='0.1.0' } }; components=$components }
$sbom | ConvertTo-Json -Depth 20 | Set-Content (Join-Path $OutputDir 'careercraft.cdx.json') -Encoding utf8
$validated = Get-Content (Join-Path $OutputDir 'careercraft.cdx.json') -Raw | ConvertFrom-Json
if ($validated.bomFormat -ne 'CycloneDX' -or $validated.specVersion -ne '1.5' -or $validated.serialNumber -notmatch '^urn:uuid:') { throw 'CycloneDX metadata validation failed' }
if (-not $validated.metadata.timestamp -or -not $validated.metadata.component.name -or $validated.components.Count -eq 0) { throw 'CycloneDX required fields are missing' }
foreach ($component in $validated.components) { if (-not $component.type -or -not $component.name -or -not $component.version -or $component.purl -notmatch '^pkg:') { throw "Invalid CycloneDX component: $($component.name)" } }
Copy-Item $files.FullName $OutputDir -Force
Write-Output "release evidence generated in $OutputDir"
