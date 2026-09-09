# Signs ONE binary, called by the Tauri bundler through bundle.windows.signCommand with the
# binary's path as the only argument (plan 4a Task 9, R-P4a-16).
#
# It must EXIT 0 when there is no signing profile: an unsigned dev build still has to bundle, and a
# non-zero exit here fails the whole `cargo tauri build`. It says UNSIGNED: once, loudly, per file.
#
# No signing secret is in this repo or in this script: Trusted Signing authenticates through
# Quinn's own Azure login, and the profile file lives outside the checkout.
# PowerShell 5.1: no &&, no ||, no ternary, no ??.
param(
  [Parameter(Mandatory = $true)][string]$Path,
  [string]$SigningProfile = (Join-Path $env:USERPROFILE ".knowlu\trusted-signing.json"),
  [string]$SignDlib = "C:\Program Files\Microsoft\Azure Code Signing\bin\x64\Azure.CodeSigning.Dlib.dll",
  [string]$SignTool = ""
)
$ErrorActionPreference = "Stop"
if ($SignTool -eq "") {
  # NOT Get-Command: the Windows SDK leaves its bin off PATH, so a machine with signtool installed
  # would still report UNSIGNED. find-signtool.ps1 resolves it the way the bundler does.
  $SignTool = [string](& (Join-Path $PSScriptRoot "find-signtool.ps1"))
}
if (-not (Test-Path $SigningProfile)) { Write-Output "UNSIGNED: no Trusted Signing profile at $SigningProfile - $Path"; exit 0 }
if ($SignTool -eq "") { Write-Output "UNSIGNED: no signtool.exe on PATH or under the Windows Kits registry root (winget install Microsoft.WindowsSDK.10.0.26100) - $Path"; exit 0 }
if (-not (Test-Path $SignDlib)) { Write-Output "UNSIGNED: no Azure Code Signing dlib at $SignDlib - $Path"; exit 0 }
& $SignTool sign /v /fd SHA256 /tr "http://timestamp.acs.microsoft.com" /td SHA256 /dlib $SignDlib /dmdf $SigningProfile $Path
if (-not $?) { throw "signtool failed on $Path" }
Write-Output "signed $Path"
exit 0
