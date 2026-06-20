#!/usr/bin/env pwsh
# run-signal-service.ps1 - Run the signal service in the foreground.
#
# Mirrors `cargo run -p signal-service` but reads environment variables
# from a .env file if one exists. The .env file is git-ignored and never
# committed.
#
# Usage:
#   pwsh ./scripts/run-signal-service.ps1
#   pwsh ./scripts/run-signal-service.ps1 -EnvFile ./.env.production

[CmdletBinding()]
param(
    [string]$EnvFile,

    [switch]$Help
)

$ErrorActionPreference = 'Stop'

if ($Help) {
    @"
run-signal-service.ps1 - Run the signal service.

Options:
  -EnvFile   Optional path to a .env file. Variables are loaded into the
             process environment before cargo runs.
  -Help      Show this help.
"@
    exit 0
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir '..')
Push-Location $RepoRoot
try {
    if ($EnvFile) {
        $resolved = Resolve-Path $EnvFile -ErrorAction SilentlyContinue
        if (-not $resolved) {
            throw "Env file not found: $EnvFile"
        }
        Get-Content $resolved | ForEach-Object {
            $line = $_.Trim()
            if (-not $line -or $line.StartsWith('#')) { return }
            $eq = $line.IndexOf('=')
            if ($eq -lt 1) { return }
            $name = $line.Substring(0, $eq).Trim()
            $value = $line.Substring($eq + 1).Trim().Trim('"', "'")
            Set-Item -Path "Env:$name" -Value $value
            Write-Host "  $name=$value" -ForegroundColor DarkGray
        }
    }

    Write-Host "Running signal-service..." -ForegroundColor Cyan
    cargo run -p signal-service
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
