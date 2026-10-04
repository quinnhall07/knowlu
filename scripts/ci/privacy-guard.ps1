# D18's release guard (Quinn's ruling, 2026-10-04). Email forwarding to Knowlu may merge to main
# before privacy bump #1, but no release may ship it until the privacy page discloses it. A release
# from a tree that carries the mail code while app\src\account.rs's PRIVACY_VERSION is still on or
# before 2026-09-24 - the version the email forwarding lane was signed against - fails here, before
# anything is built. Bump #1 moves PRIVACY_VERSION with the page, and the guard then lets it through.
#
# Three markers, one per layer, and any one of them is enough: the receiver that accepts a
# student's mail (cloud\supabase\functions\mail-inbound), the app's mail step (app\src\mail_steps.rs)
# and the engine's mail reconciler (engine\src\mailreconcile). Renaming one cannot switch the guard
# off while the other two stand. A PRIVACY_VERSION that cannot be read blocks: the guard fails
# closed. engine/tests/workflows.rs runs this script against synthetic trees and this one.
#
# Releases are CI-only, so release.yml runs this; scripts\release.ps1 -DryRun stays a laptop build
# check that works on any branch.
# PowerShell 5.1: no &&, no ||, no ternary, no ??.
param([string]$Root = ".")
$ErrorActionPreference = "Stop"
$preBump = "2026-09-24"
$markers = @("cloud\supabase\functions\mail-inbound", "app\src\mail_steps.rs", "engine\src\mailreconcile")
$present = @($markers | Where-Object { Test-Path -LiteralPath (Join-Path $Root $_) })
if ($present.Count -eq 0) {
  Write-Output "privacy guard (D18): no mail code in this tree"
  exit 0
}
Write-Output ("privacy guard (D18): mail code present: " + ($present -join ", "))
$account = Join-Path $Root "app\src\account.rs"
if (-not (Test-Path -LiteralPath $account -PathType Leaf)) {
  Write-Output "RELEASE BLOCKED (D18): $account is missing, so PRIVACY_VERSION cannot be read"
  exit 1
}
$m = [regex]::Match([System.IO.File]::ReadAllText($account), 'pub const PRIVACY_VERSION: &str = "(\d{4}-\d{2}-\d{2})";')
if (-not $m.Success) {
  Write-Output "RELEASE BLOCKED (D18): no YYYY-MM-DD PRIVACY_VERSION in $account"
  exit 1
}
$version = $m.Groups[1].Value
# Equal-length ISO dates compare correctly as ordinal strings.
if ([string]::CompareOrdinal($version, $preBump) -le 0) {
  Write-Output "RELEASE BLOCKED (D18): this tree carries email forwarding and PRIVACY_VERSION is $version, on or before $preBump. Privacy bump #1 must merge first: the page, its Effective date and PRIVACY_VERSION move together."
  exit 1
}
Write-Output "privacy guard (D18): PRIVACY_VERSION $version is past $preBump"
exit 0
