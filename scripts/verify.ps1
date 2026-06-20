#!/usr/bin/env pwsh
# verify.ps1 - Run the Sprint 8 verification suite in order.
#
# Exits non-zero on the first failed step. The steps mirror the regression
# matrix in docs/QA_ACCEPTANCE_MATRIX.md and the verification block in
# README.md.
#
# Usage:
#   pwsh ./scripts/verify.ps1
#   pwsh ./scripts/verify.ps1 -SkipBundle
#
# The -SkipBundle switch skips `npm run desktop:build`, which is the
# slowest step. CI should run the full suite; local iteration can skip
# the bundle.

[CmdletBinding()]
param(
    [switch]$SkipBundle,
    [switch]$SkipClippy,
    [switch]$SkipTests,
    [switch]$Help
)

$ErrorActionPreference = 'Stop'

if ($Help) {
    @"
verify.ps1 - Run the Sprint 8 verification suite.

Options:
  -SkipBundle    Skip npm run desktop:build (fastest local feedback).
  -SkipClippy    Skip cargo clippy.
  -SkipTests     Skip cargo test.
  -Help          Show this help.

Exit code is the exit code of the first failing step, or 0 if every
step passes.
"@
    exit 0
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir '..')
Push-Location $RepoRoot
try {
    $failed = $false

    function Run-Step {
        param(
            [Parameter(Mandatory = $true)][string]$Name,
            [Parameter(Mandatory = $true)][string]$Command
        )
        Write-Host ""
        Write-Host "==> $Name" -ForegroundColor Cyan
        Write-Host "    $Command" -ForegroundColor DarkGray
        $start = Get-Date
        cmd /c "$Command"
        $code = $LASTEXITCODE
        $elapsed = (Get-Date) - $start
        if ($code -ne 0) {
            Write-Host "    FAILED ($([int]$elapsed.TotalSeconds)s, exit $code)" -ForegroundColor Red
            $script:failed = $true
            return $false
        }
        Write-Host "    OK ($([int]$elapsed.TotalSeconds)s)" -ForegroundColor Green
        return $true
    }

    Run-Step 'cargo fmt --all -- --check' 'cargo fmt --all -- --check'
    if ($failed) { exit 1 }

    if (-not $SkipClippy) {
        Run-Step 'cargo clippy --workspace --all-targets' 'cargo clippy --workspace --all-targets -- -D warnings'
        if ($failed) { exit 1 }
    }

    if (-not $SkipTests) {
        Run-Step 'cargo test --workspace' 'cargo test --workspace'
        if ($failed) { exit 1 }
    }

    Run-Step 'npm run typecheck' 'npm run typecheck'
    if ($failed) { exit 1 }

    Run-Step 'npm run lint' 'npm run lint'
    if ($failed) { exit 1 }

    if (-not $SkipBundle) {
        Run-Step 'npm run desktop:build' 'npm run desktop:build'
        if ($failed) { exit 1 }
    }

    Run-Step 'tauri info' 'npm run -w @watchparty/desktop tauri -- info'
    if ($failed) { exit 1 }

    Write-Host ""
    Write-Host "All verification steps passed." -ForegroundColor Green
    exit 0
}
finally {
    Pop-Location
}
