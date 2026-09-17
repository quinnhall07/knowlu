# Puts Tauri's own prebuilt cargo-tauri.exe on a GitHub runner, for exactly one pinned version, and
# refuses to install it unless the archive's bytes match the pinned SHA-256.
#
# It is downloaded, never compiled: `cargo install tauri-cli --locked` spent 21 of the first release
# run's 40 minutes building the CLI from source (run 35179886527, 2026-09-17). The CLI is a tool that
# invokes cargo, not the product - it does not decide what the product is built for (the toolchain
# step does, and `assert gnu host toolchain` proves it), so the MSVC build of the CLI is the right
# one to take.
#
# $Dest defaults to cargo home's bin, and that matters. `cargo tauri` resolves the external
# subcommand `cargo-tauri.exe` by looking in cargo home's bin FIRST, and Swatinem/rust-cache caches
# ~\.cargo\bin - so a cargo-installed cargo-tauri.exe from an older run can be restored there, and a
# binary dropped anywhere later on PATH would lose to it. Installing into cargo home's bin overwrites
# whatever the cache restored, so the verified binary is the one `cargo tauri build` runs.
#
# A bump is the version AND the hash, and a wrong archive throws before anything is built - the same
# contract app\src\inference.rs holds for llama.cpp runtimes. This script runs unchanged on Windows
# PowerShell 5.1 and on pwsh 7: no &&, no ternary, no ??.
param(
  [Parameter(Mandatory = $true)][string]$Version,
  [Parameter(Mandatory = $true)][string]$Sha256,
  [string]$Dest = ""
)
$ErrorActionPreference = "Stop"
if ($Sha256 -notmatch "^[0-9a-f]{64}$") { throw "-Sha256 must be 64 lowercase hex characters, got '$Sha256'" }
if ([string]::IsNullOrWhiteSpace($Dest)) {
  $cargoHome = $env:CARGO_HOME
  if ([string]::IsNullOrWhiteSpace($cargoHome)) { $cargoHome = Join-Path $env:USERPROFILE ".cargo" }
  $Dest = Join-Path $cargoHome "bin"
}
# Windows PowerShell 5.1's Invoke-WebRequest is an order of magnitude slower with the progress bar on.
$ProgressPreference = "SilentlyContinue"
# The version is the only variable part of the URL.
$url = "https://github.com/tauri-apps/tauri/releases/download/tauri-cli-v$Version/cargo-tauri-x86_64-pc-windows-msvc.zip"
$tmp = $env:RUNNER_TEMP
if ([string]::IsNullOrWhiteSpace($tmp)) { $tmp = [IO.Path]::GetTempPath() }
$zip = Join-Path $tmp "cargo-tauri-$Version.zip"
Invoke-WebRequest -Uri $url -OutFile $zip
$got = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant()
if ($got -ne $Sha256) {
  Remove-Item $zip -Force -ErrorAction SilentlyContinue
  throw "tauri-cli $Version archive mismatch: expected SHA-256 $Sha256, got $got - nothing was installed"
}
$stage = Join-Path $tmp "cargo-tauri-$Version"
Expand-Archive $zip -DestinationPath $stage -Force
$exe = Join-Path $stage "cargo-tauri.exe"
if (-not (Test-Path $exe)) { throw "no cargo-tauri.exe inside $url" }
if (-not (Test-Path $Dest)) { New-Item -ItemType Directory -Force $Dest | Out-Null }
Copy-Item $exe $Dest -Force
$installed = Join-Path $Dest "cargo-tauri.exe"
$printed = (& $installed --version | Out-String)
if (($LASTEXITCODE -ne 0) -or ($printed -notmatch [regex]::Escape($Version))) {
  throw "cargo-tauri.exe at $installed printed '$($printed.Trim())', not tauri-cli $Version"
}
Write-Output "tauri-cli $Version installed at $Dest (SHA-256 verified)"
Remove-Item $zip -Force -ErrorAction SilentlyContinue
Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
exit 0
