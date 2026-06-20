#!/usr/bin/env pwsh
# bundle-libmpv.ps1 - Copy a known libmpv build next to the Tauri executable.
#
# The desktop shell's dynamic loader checks well-known names relative to the
# executable directory (see apps/desktop/src-tauri/src/main.rs and
# docs/release/LIBMPV_BUNDLING.md). This script copies the library from a
# source path into the bundle directory so the loader can find it without
# MPV_LIBRARY_PATH.
#
# Usage:
#   pwsh ./scripts/bundle-libmpv.ps1
#   pwsh ./scripts/bundle-libmpv.ps1 -Platform windows -Source ./libmpv/mpv-2.dll

[CmdletBinding()]
param(
    [ValidateSet('auto', 'windows', 'macos', 'linux')]
    [string]$Platform = 'auto',

    [string]$Source,

    [switch]$Help
)

$ErrorActionPreference = 'Stop'

if ($Help) {
    @"
bundle-libmpv.ps1 - Copy libmpv next to the Tauri executable.

Options:
  -Platform   auto | windows | macos | linux. Defaults to the current OS.
  -Source     Path to the libmpv binary to copy. If omitted, the script
              looks under ./libmpv/<platform>/ for the first candidate.
  -Help       Show this help.
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

    $bundleDir = Join-Path $RepoRoot 'apps/desktop/src-tauri/target/release'
    if (-not (Test-Path $bundleDir)) {
        throw "Bundle directory not found. Run `npm run desktop:build` first: $bundleDir"
    }

    $candidates = switch ($Platform) {
        'windows' { @('mpv-2.dll', 'libmpv-2.dll', 'mpv-1.dll') }
        'macos' { @('libmpv.2.dylib', 'libmpv.dylib') }
        default { @('libmpv.so.2', 'libmpv.so') }
    }

    $sourcePath = $null
    if ($Source) {
        if (-not (Test-Path -LiteralPath $Source)) {
            throw "Source libmpv not found: $Source"
        }
        $sourcePath = (Resolve-Path $Source).Path
    }
    else {
        $searchDir = Join-Path $RepoRoot "libmpv/$Platform"
        if (Test-Path $searchDir) {
            foreach ($name in $candidates) {
                $probe = Join-Path $searchDir $name
                if (Test-Path -LiteralPath $probe) {
                    $sourcePath = (Resolve-Path $probe).Path
                    break
                }
            }
        }
    }

    if (-not $sourcePath) {
        Write-Host "No libmpv source found under ./libmpv/$Platform. Skipping bundle step." -ForegroundColor Yellow
        Write-Host "Drop a libmpv build there or pass -Source." -ForegroundColor Yellow
        exit 0
    }

    foreach ($target in $candidates) {
        $targetPath = Join-Path $bundleDir $target
        Copy-Item -LiteralPath $sourcePath -Destination $targetPath -Force
        Write-Host "Copied $sourcePath -> $targetPath" -ForegroundColor Green
    }

    exit 0
}
finally {
    Pop-Location
}
