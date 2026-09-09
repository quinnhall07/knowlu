# Resolves signtool.exe THE WAY THE BUNDLER DOES, and writes the full path to the pipeline
# (nothing at all when it cannot be found). Both sign.ps1 and release.ps1 use this, because a
# probe that disagrees with the bundler is worse than no probe: release.ps1 sets
# TAURI_SKIP_SIDECAR_SIGNATURE_CHECK when it believes signtool is missing, and that variable makes
# the bundler skip SIGNING the sidecar, not merely checking it.
#
# The trap this exists for (hit 2026-09-07): the Windows SDK does NOT put its bin on PATH - you are
# expected to use a Developer Command Prompt - so `Get-Command signtool.exe` fails on a machine
# that has it installed. tauri-bundler 2.9.4 src/bundle/windows/sign.rs:53 never looks at PATH: it
# reads TAURI_WINDOWS_SIGNTOOL_PATH, then the registry's KitsRoot10, then each installed kit
# version's bin\<arch>\signtool.exe. This mirrors that order and only then falls back to PATH.
#
# PowerShell 5.1: no &&, no ||, no ternary, no ??.
param([string]$Arch = "x64")
$ErrorActionPreference = "Stop"

if ($env:TAURI_WINDOWS_SIGNTOOL_PATH -and (Test-Path $env:TAURI_WINDOWS_SIGNTOOL_PATH)) {
  $env:TAURI_WINDOWS_SIGNTOOL_PATH
  return
}

# Both views are checked: the bundler opens the 32-bit hive, and a 64-bit PowerShell sees that as
# WOW6432Node. On this machine both hold the same KitsRoot10, but neither is guaranteed elsewhere.
foreach ($root in @("HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows Kits\Installed Roots",
                    "HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots")) {
  if (-not (Test-Path $root)) { continue }
  $props = Get-ItemProperty $root -ErrorAction SilentlyContinue
  if (-not $props) { continue }
  $kits = $props.KitsRoot10
  if (-not $kits) { continue }
  $bin = Join-Path $kits "bin"
  if (-not (Test-Path $bin)) { continue }
  # Newest kit first, then the un-versioned bin\<arch> the bundler also considers.
  $candidates = @()
  $versions = Get-ChildItem $root -ErrorAction SilentlyContinue | Select-Object -ExpandProperty PSChildName | Sort-Object -Descending
  foreach ($v in $versions) { $candidates += (Join-Path (Join-Path $bin $v) (Join-Path $Arch "signtool.exe")) }
  $candidates += (Join-Path $bin (Join-Path $Arch "signtool.exe"))
  foreach ($c in $candidates) { if (Test-Path $c) { $c; return } }
}

$cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
if ($cmd) { $cmd.Source; return }
