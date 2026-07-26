param(
    [Parameter(Mandatory = $true)]
    [string]$ExePath
)

$resolved = (Resolve-Path -LiteralPath $ExePath).Path
$mt = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Recurse -Filter mt.exe -ErrorAction Stop |
    Where-Object { $_.FullName -match '\\x64\\mt\.exe$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $mt) { throw 'Windows SDK mt.exe was not found.' }

$manifest = Join-Path $env:TEMP 'careercraft-artifact.manifest'
& $mt.FullName "-inputresource:$resolved;#1" "-out:$manifest"
if ($LASTEXITCODE -ne 0) { throw 'The executable does not contain an embedded manifest.' }
$xml = Get-Content -LiteralPath $manifest -Raw
if ($xml -notmatch 'Microsoft\.Windows\.Common-Controls' -or $xml -notmatch 'version="6\.0\.0\.0"') {
    throw 'The executable does not request Common Controls v6.'
}
Write-Output 'Windows manifest OK: Common Controls 6.0.0.0'
