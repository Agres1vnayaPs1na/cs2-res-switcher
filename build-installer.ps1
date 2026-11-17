# Builds the release exe and the Inno Setup installer into .\dist
# Usage: powershell -ExecutionPolicy Bypass -File build-installer.ps1
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value

cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $PSScriptRoot 'target' }
New-Item -ItemType Directory -Force dist | Out-Null
Copy-Item (Join-Path $targetDir 'release\cs2-res-switcher.exe') dist\ -Force

$iscc = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw "Inno Setup 6 not found (winget install JRSoftware.InnoSetup)" }

& $iscc "/DAppVersion=$version" installer\cs2-res-switcher.iss
if ($LASTEXITCODE -ne 0) { throw "ISCC failed" }

Get-ChildItem dist
