#!/usr/bin/env pwsh
# check-libmpv.ps1 - Dry-run the libmpv discovery rules from main.rs.
#
# Prints the first candidate that exists on disk and the path it would be
# loaded from. Exits 0 when at least one candidate is reachable, 1 otherwise.
#
# This script does not call LoadLibrary; it just confirms the same paths
# the runtime would try are available. CI can use the exit code to gate
# release builds.
#
# Usage:
#   pwsh ./scripts/check-libmpv.ps1

[CmdletBinding()]
param(
    [switch]$Help
)

$ErrorActionPreference = 'Stop'

if ($Help) {
    @"
check-libmpv.ps1 - Dry-run the libmpv discovery rules.

Exits 0 if a candidate is found, 1 otherwise. Prints the path it would
load from on stdout. This script is read-only and never loads the library.
"@
    exit 0
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir '..')

function Get-OsKind {
    if ($PSVersionTable.PSVersion.Major -ge 7) {
        if ($IsWindows) { return 'windows' }
        if ($IsMacOS) { return 'macos' }
        return 'linux'
    }
    if ($env:OS -eq 'Windows_NT') { return 'windows' }
    if ($env:HOME -and (Test-Path '/Applications')) { return 'macos' }
    return 'linux'
}

function Test-EnvOverride {
    $envPath = $env:MPV_LIBRARY_PATH
    if (-not $envPath) { return $null }
    if (Test-Path -LiteralPath $envPath) {
        return [pscustomobject]@{ Source = 'MPV_LIBRARY_PATH'; Path = (Resolve-Path $envPath).Path }
    }
    return [pscustomobject]@{ Source = 'MPV_LIBRARY_PATH (missing)'; Path = $envPath }
}

function Get-Candidates {
    param([string]$Os)
    switch ($Os) {
        'windows' { return @('mpv-2.dll', 'libmpv-2.dll', 'mpv-1.dll') }
        'macos' { return @('libmpv.2.dylib', 'libmpv.dylib') }
        default { return @('libmpv.so.2', 'libmpv.so') }
    }
}

$os = Get-OsKind
$exeDir = Join-Path $RepoRoot 'apps/desktop/src-tauri/target/release'

Write-Host "Platform: $os" -ForegroundColor Cyan
Write-Host "Executable directory: $exeDir" -ForegroundColor DarkGray
Write-Host ""

$override = Test-EnvOverride
if ($override) {
    if ($override.Source -like '*missing*') {
        Write-Host "MPV_LIBRARY_PATH is set but the file does not exist: $($override.Path)" -ForegroundColor Yellow
    }
    else {
        Write-Host "MPV_LIBRARY_PATH resolves to: $($override.Path)" -ForegroundColor Green
        exit 0
    }
}

$candidates = Get-Candidates -Os $os
Write-Host "Candidates tried in order:" -ForegroundColor Cyan
foreach ($candidate in $candidates) {
    $candidatePath = Join-Path $exeDir $candidate
    if (Test-Path -LiteralPath $candidatePath) {
        Write-Host "  FOUND   $candidatePath" -ForegroundColor Green
        exit 0
    }
    else {
        Write-Host "  miss    $candidatePath" -ForegroundColor DarkGray
    }
}

Write-Host ""
Write-Host "No libmpv candidate found next to the executable." -ForegroundColor Yellow
Write-Host "Install libmpv, place a build under the executable directory, or set MPV_LIBRARY_PATH." -ForegroundColor Yellow
exit 1
