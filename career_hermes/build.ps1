# CareerCraft Agent - Windows Build Script
# Run with: powershell -ExecutionPolicy Bypass -File D:\workplace_for_hermes\career-agent\build.ps1

$ErrorActionPreference = "Continue"
$ProjectRoot = "D:\workplace_for_hermes\career-agent"
$VenvPython = "$ProjectRoot\.venv\Scripts\python.exe"
$VenvPip = "$ProjectRoot\.venv\Scripts\python.exe -m pip"

Set-Location $ProjectRoot

Write-Host "=========================================="
Write-Host "CareerCraft Agent - Windows EXE Build"
Write-Host "=========================================="
Write-Host ""

# 1) Create venv if missing
if (!(Test-Path -Path $VenvPython)) {
    Write-Host "Creating virtual environment..."
    python -m venv "$ProjectRoot\.venv"
}

# 2) Upgrade pip
Write-Host "Upgrading pip..."
& $VenvPython -m pip install --upgrade pip

# 3) Install dependencies
Write-Host "Installing dependencies..."
$packages = @(
    "PySide6",
    "jinja2",
    "httpx",
    "pydantic",
    "pyyaml",
    "sqlalchemy",
    "keyring",
    "fpdf2",
    "aiofiles",
    "pyinstaller"
)
foreach ($pkg in $packages) {
    Write-Host "Installing $pkg ..."
    & $VenvPython -m pip install $pkg
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to install $pkg" -ForegroundColor Red
    }
}

# 4) Run build
Write-Host ""
Write-Host "Building EXE (this may take 3-5 minutes)..."
& $VenvPython "$ProjectRoot\build.py"

if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

# 5) Verify
$dist = "$ProjectRoot\dist\CareerCraftAgent"
Write-Host ""
Write-Host "=========================================="
Write-Host "Build completed!"
Write-Host "=========================================="

$exe = "$dist\CareerCraftAgent.exe"
if (Test-Path $exe) {
    $size = (Get-Item $exe).Length / 1MB
    Write-Host "EXE: $exe"
    Write-Host "Size: $([Math]::Round($size, 2)) MB"
}

$proto = "$dist\prototype"
if (Test-Path $proto) {
    $files = Get-ChildItem $proto | Select-Object -ExpandProperty Name
    Write-Host "Prototype files: $($files -join ', ')"
} else {
    Write-Host "WARNING: prototype/ not found in dist" -ForegroundColor Red
}

Write-Host ""
Write-Host "Run with: $exe"
