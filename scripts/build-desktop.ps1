#!/usr/bin/env pwsh
# build-desktop.ps1 - Build the desktop bundle for the current platform.
#
# Builds the React frontend and the Tauri Rust shell. Produces unsigned MSI
# and NSIS installers on Windows, APP and DMG bundles on macOS. The output
# directory is apps/desktop/src-tauri/target/release/bundle/.
#
# Usage:
#   pwsh ./scripts/build-desktop.ps1
#   pwsh ./scripts/build-desktop.ps1 -Platform windows
#   pwsh ./scripts/build-desktop.ps1 -NoLibmpv

[CmdletBinding()]
param(
    [ValidateSet('auto', 'windows', 'macos', 'linux')]
    [string]$Platform = 'auto',

    [switch]$NoLibmpv,

    [switch]$Help
)

$ErrorActionPreference = 'Stop'

if ($Help) {
    @"
build-desktop.ps1 - Build the desktop bundle.

Options:
  -Platform      auto | windows | macos | linux. Defaults to the current OS.
  -NoLibmpv      Skip the libmpv bundling step (smoke builds).
  -Help          Show this help.
"@
    exit 0
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir '..')
Push-Location $RepoRoot
try {
    if ($Platform -eq 'auto') {
        if ($PSVersionTable.PSVersion.Major -ge 7) {
            if ($IsWindows) { $Platform = 'windows' }
            elseif ($IsMacOS) { $Platform = 'macos' }
            else { $Platform = 'linux' }
        }
        elseif ($env:OS -eq 'Windows_NT') { $Platform = 'windows' }
        elseif ($env:HOME -and (Test-Path '/Applications')) { $Platform = 'macos' }
        else { $Platform = 'linux' }
    }

    Write-Host "Building desktop bundle for $Platform" -ForegroundColor Cyan

    Write-Host ""
    Write-Host "==> npm install" -ForegroundColor Cyan
    cmd /c 'npm install'
    if ($LASTEXITCODE -ne 0) { throw 'npm install failed' }

    Write-Host ""
    Write-Host "==> npm run build (frontend)" -ForegroundColor Cyan
    cmd /c 'npm run build --workspace @watchparty/desktop'
    if ($LASTEXITCODE -ne 0) { throw 'frontend build failed' }

    Write-Host ""
    Write-Host "==> npm run desktop:build (tauri)" -ForegroundColor Cyan
    cmd /c 'npm run desktop:build'
    if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }

    if (-not $NoLibmpv) {
        Write-Host ""
        Write-Host "==> bundle-libmpv" -ForegroundColor Cyan
        & "$ScriptDir/bundle-libmpv.ps1" -Platform $Platform
        if ($LASTEXITCODE -ne 0) { throw 'libmpv bundling failed' }
    }

    $bundleRoot = Join-Path $RepoRoot 'apps/desktop/src-tauri/target/release/bundle'
    if (Test-Path $bundleRoot) {
        Write-Host ""
        Write-Host "Bundle outputs:" -ForegroundColor Cyan
        Get-ChildItem -Path $bundleRoot -Recurse -File |
            Where-Object { $_.Extension -in '.msi', '.exe', '.app', '.dmg' } |
            ForEach-Object { Write-Host "  $($_.FullName.Substring($RepoRoot.Path.Length + 1))" }
    }

    Write-Host ""
    Write-Host "Desktop build complete." -ForegroundColor Green
    exit 0
}
finally {
    Pop-Location
}
