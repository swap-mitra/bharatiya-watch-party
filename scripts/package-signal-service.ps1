#!/usr/bin/env pwsh
# package-signal-service.ps1 - Package the signal service binary for release.
#
# Produces a zip (Windows) or tar.gz (macOS/Linux) containing the release
# binary plus a short README. The artifact is intended to be uploaded
# alongside the desktop bundle.
#
# Usage:
#   pwsh ./scripts/package-signal-service.ps1 -Platform windows
#   pwsh ./scripts/package-signal-service.ps1 -Platform macos
#   pwsh ./scripts/package-signal-service.ps1 -Platform linux
#   pwsh ./scripts/package-signal-service.ps1 -OutDir ./dist

[CmdletBinding()]
param(
    [ValidateSet('windows', 'macos', 'linux')]
    [string]$Platform,

    [string]$OutDir = './dist',

    [switch]$Help
)

$ErrorActionPreference = 'Stop'

if ($Help -or -not $Platform) {
    @"
package-signal-service.ps1 - Package the signal service binary.

Options:
  -Platform   windows | macos | linux. Required.
  -OutDir     Output directory. Defaults to ./dist.
  -Help       Show this help.
"@
    exit 0
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir '..')
Push-Location $RepoRoot
try {
    Write-Host "Building signal-service in release mode" -ForegroundColor Cyan
    cmd /c 'cargo build --release -p signal-service'
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

    $binaryName = if ($Platform -eq 'windows') { 'signal-service.exe' } else { 'signal-service' }
    $binaryPath = Join-Path $RepoRoot "target/release/$binaryName"
    if (-not (Test-Path $binaryPath)) {
        throw "Expected binary not found: $binaryPath"
    }

    $version = (Get-Content "$RepoRoot/services/signal-service/Cargo.toml" |
        Select-String -Pattern '^version' |
        Select-Object -First 1).ToString().Trim()
    $version = ($version -split '=', 2)[1].Trim().Trim('"')
    if (-not $version) { $version = '0.1.0' }

    $resolvedOut = Resolve-Path -Path $OutDir -ErrorAction SilentlyContinue
    if (-not $resolvedOut) {
        New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
        $resolvedOut = (Resolve-Path $OutDir).Path
    }

    $targetTriple = switch ($Platform) {
        'windows' { 'x86_64-pc-windows-msvc' }
        'macos' { if ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -eq 'Arm64') {
                'aarch64-apple-darwin'
            } else { 'x86_64-apple-darwin' } }
        'linux' { 'x86_64-unknown-linux-gnu' }
    }

    $staging = Join-Path $resolvedOut "signal-service-$version-$targetTriple"
    if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
    New-Item -ItemType Directory -Path $staging -Force | Out-Null

    Copy-Item -LiteralPath $binaryPath -Destination $staging
    Copy-Item -LiteralPath (Join-Path $RepoRoot 'docs/release/SIGNAL_SERVICE_DEPLOYMENT.md') `
        -Destination (Join-Path $staging 'README.md')

    if ($Platform -eq 'windows') {
        $archive = Join-Path $resolvedOut "signal-service-$version-$targetTriple.zip"
        if (Test-Path $archive) { Remove-Item -Force $archive }
        Compress-Archive -Path $staging -DestinationPath $archive
        Write-Host "Wrote $archive" -ForegroundColor Green
    }
    else {
        $archive = Join-Path $resolvedOut "signal-service-$version-$targetTriple.tar.gz"
        if (Test-Path $archive) { Remove-Item -Force $archive }
        $parent = Split-Path $staging
        $leaf = Split-Path $staging -Leaf
        Push-Location $parent
        try {
            cmd /c "tar -czf `"$archive`" `"$leaf`""
        }
        finally {
            Pop-Location
        }
        Write-Host "Wrote $archive" -ForegroundColor Green
    }

    exit 0
}
finally {
    Pop-Location
}
