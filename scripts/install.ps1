# rat-squad installation script for Windows PowerShell
# Installs rat-squad as a ratterm extension

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

Write-Host "rat-squad Extension Installer" -ForegroundColor Cyan
Write-Host "=============================" -ForegroundColor Cyan
Write-Host ""

# Get script directory (where Cargo.toml is)
$ScriptDir = Split-Path -Parent $PSScriptRoot
$CargoToml = Join-Path $ScriptDir "Cargo.toml"

if (-not (Test-Path $CargoToml)) {
    Write-Host "Error: Must run from rat-squad repository root" -ForegroundColor Red
    exit 1
}

# Build release binary
Write-Host "[1/4] Building release binary..." -ForegroundColor Yellow
Push-Location $ScriptDir
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed"
    }
} finally {
    Pop-Location
}
Write-Host "Build successful!" -ForegroundColor Green

# Create extension directory
$ExtDir = "$env:USERPROFILE\.ratterm\extensions\rat-squad"
Write-Host "[2/4] Creating extension directory: $ExtDir" -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $ExtDir | Out-Null
Write-Host "Directory created!" -ForegroundColor Green

# Copy files
Write-Host "[3/4] Copying extension files..." -ForegroundColor Yellow
$BinaryPath = Join-Path $ScriptDir "target\release\rat-squad.exe"
$ManifestPath = Join-Path $ScriptDir "extension.toml"
$ConfigExamplePath = Join-Path $ScriptDir "config.yaml.example"

Copy-Item $BinaryPath $ExtDir -Force
Copy-Item $ManifestPath $ExtDir -Force
Copy-Item $ConfigExamplePath $ExtDir -Force
Write-Host "Files copied!" -ForegroundColor Green

# Create data directory
$DataDir = "$env:USERPROFILE\.rat-squad"
Write-Host "[4/4] Creating configuration directory: $DataDir" -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

# Copy config example if no config exists
$ConfigPath = Join-Path $DataDir "config.yaml"
if (-not (Test-Path $ConfigPath)) {
    Copy-Item $ConfigExamplePath $ConfigPath
    Write-Host "Created default configuration at $ConfigPath" -ForegroundColor Green
} else {
    Write-Host "Configuration already exists at $ConfigPath" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. Restart ratterm to discover the extension"
Write-Host "  2. Approve the extension when prompted"
Write-Host "  3. Use 'rat-squad' commands in ratterm"
Write-Host ""
Write-Host "Configuration file: $ConfigPath"
Write-Host "Extension directory: $ExtDir"
