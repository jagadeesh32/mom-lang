# install.ps1 — Install mom programming language on Windows
#
# Usage (run in PowerShell as regular user):
#   irm https://raw.githubusercontent.com/jagadeesh32/mom/main/scripts/install.ps1 | iex
#
# With options:
#   $env:MOM_VERSION="v0.2.0"; irm https://raw.githubusercontent.com/jagadeesh32/mom/main/scripts/install.ps1 | iex
#
# The installer:
#   1. Detects your Windows architecture (x86_64 or aarch64)
#   2. Downloads the matching .zip from GitHub releases
#   3. Extracts to $env:LOCALAPPDATA\mom\
#   4. Adds the bin directory to your User PATH
#
# Requirements: PowerShell 5.1+ or PowerShell Core 7+

[CmdletBinding()]
param(
    [string]$Version    = $env:MOM_VERSION,
    [string]$InstallDir = "$env:LOCALAPPDATA\mom",
    [switch]$NoPath
)

$ErrorActionPreference = 'Stop'
$ProgressPreference    = 'SilentlyContinue'

$REPO = "jagadeesh32/mom"

function Write-Info  { Write-Host "[mom] $args" -ForegroundColor Green }
function Write-Warn  { Write-Host "[mom] $args" -ForegroundColor Yellow }
function Write-Err   { Write-Host "[mom] $args" -ForegroundColor Red; exit 1 }

# ── Detect architecture ───────────────────────────────────────────────────────
function Get-Platform {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch -Wildcard ($arch) {
        "AMD64" { return "mom-windows-x86_64" }
        "ARM64" { return "mom-windows-aarch64" }
        default { Write-Err "Unsupported architecture: $arch" }
    }
}

# ── Resolve latest version ────────────────────────────────────────────────────
function Get-LatestVersion {
    $url = "https://api.github.com/repos/$REPO/releases/latest"
    try {
        $resp = Invoke-RestMethod -Uri $url -Headers @{ 'User-Agent' = 'mom-installer' }
        return $resp.tag_name
    } catch {
        Write-Err "Failed to fetch latest version: $_"
    }
}

# ── Main ──────────────────────────────────────────────────────────────────────
$platform = Get-Platform

if (-not $Version) {
    Write-Info "Checking latest release..."
    $Version = Get-LatestVersion
    Write-Info "Latest version: $Version"
}

$asset    = "$platform.zip"
$url      = "https://github.com/$REPO/releases/download/$Version/$asset"
$BinDir   = "$InstallDir\bin"
$LibDir   = "$InstallDir\lib"

Write-Info "Platform:  $platform"
Write-Info "Version:   $Version"
Write-Info "Directory: $InstallDir"

# ── Download ──────────────────────────────────────────────────────────────────
$tmpDir  = Join-Path $env:TEMP "mom-install-$(Get-Random)"
$zipPath = Join-Path $tmpDir "$asset"
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null

Write-Info "Downloading $asset ..."
try {
    Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
} catch {
    Write-Err "Download failed: $_"
}

# ── Verify checksum ───────────────────────────────────────────────────────────
$checksumUrl = "https://github.com/$REPO/releases/download/$Version/SHA256SUMS.txt"
try {
    $checksums = (Invoke-WebRequest -Uri $checksumUrl -UseBasicParsing).Content
    $expectedLine = $checksums -split "`n" | Where-Object { $_ -match [regex]::Escape($asset) }
    if ($expectedLine) {
        $expected = ($expectedLine -split '\s+')[0].ToLower()
        $actual   = (Get-FileHash $zipPath -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) {
            Write-Err "Checksum mismatch!`n  Expected: $expected`n  Actual:   $actual"
        }
        Write-Info "Checksum OK ✓"
    }
} catch {
    Write-Warn "Could not verify checksum (continuing)"
}

# ── Extract ───────────────────────────────────────────────────────────────────
Write-Info "Extracting..."
$extractDir = Join-Path $tmpDir "extract"
New-Item -ItemType Directory -Force -Path $extractDir | Out-Null
Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force
$extracted = Get-ChildItem $extractDir -Directory | Select-Object -First 1

# ── Install ───────────────────────────────────────────────────────────────────
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $LibDir | Out-Null

Copy-Item "$($extracted.FullName)\mom.exe" "$BinDir\mom.exe" -Force
Write-Info "Installed binary to $BinDir\mom.exe"

if (Test-Path "$($extracted.FullName)\compiler") {
    Remove-Item "$LibDir\compiler" -Recurse -Force -ErrorAction SilentlyContinue
    Copy-Item "$($extracted.FullName)\compiler" "$LibDir\compiler" -Recurse -Force
}
if (Test-Path "$($extracted.FullName)\std") {
    Remove-Item "$LibDir\std" -Recurse -Force -ErrorAction SilentlyContinue
    Copy-Item "$($extracted.FullName)\std" "$LibDir\std" -Recurse -Force
}

# ── PATH ──────────────────────────────────────────────────────────────────────
if (-not $NoPath) {
    $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($userPath -notlike "*$BinDir*") {
        [Environment]::SetEnvironmentVariable("PATH", "$BinDir;$userPath", "User")
        Write-Info "Added $BinDir to User PATH"
        $env:PATH = "$BinDir;$env:PATH"
    } else {
        Write-Info "$BinDir is already in PATH"
    }
}

# ── Cleanup ───────────────────────────────────────────────────────────────────
Remove-Item $tmpDir -Recurse -Force -ErrorAction SilentlyContinue

# ── Done ──────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  ╔══════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "  ║   mom $Version installed successfully!   ║" -ForegroundColor Cyan
Write-Host "  ╚══════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Try it (open a new terminal):"
Write-Host "    mom version"
Write-Host "    mom run examples\hello.mom"
Write-Host ""
