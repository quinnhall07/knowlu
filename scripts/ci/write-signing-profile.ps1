# Writes the Trusted Signing metadata file sign.ps1 reads (no secret in it: endpoint, account,
# profile). The three values come from repository VARIABLES, not secrets - they are names and a
# URL, and keeping them out of the secret store keeps the log readable when something is wrong.
#
# ExcludeCredentials is the point of this file. Azure.CodeSigning.Dlib.dll authenticates with
# DefaultAzureCredential, which walks a chain; the excluded five are the ones that either hang
# (InteractiveBrowserCredential wants a browser on a headless runner) or silently pick up the
# wrong identity on a machine that has ever had Visual Studio or a managed identity attached.
#
# AzureCliCredential is the one that actually signs, and it must stay in the chain.
# `azure/login` at the SHA release.yml pins does an Azure CLI login and nothing else: it exchanges
# the workflow's OIDC token for a token in `az`'s own cache. It does not write
# AZURE_FEDERATED_TOKEN_FILE or the other variables EnvironmentCredential and
# WorkloadIdentityCredential read, so neither of those can authenticate here - the dlib reaches
# AzureCliCredential, which shells out to `az account get-access-token`. That is why the signing
# step must come after the login step, and why no client secret exists anywhere.
#
# PowerShell 5.1: no &&, no ||, no ternary, no ??.
param([string]$Path = (Join-Path $env:USERPROFILE ".knowlu\trusted-signing.json"))
$ErrorActionPreference = "Stop"
foreach ($n in @("TS_ENDPOINT", "TS_ACCOUNT", "TS_PROFILE")) {
  # Both halves matter. A repository variable that was never set reaches the step as an EMPTY
  # value, not as an absent one, and an empty Endpoint fails much later inside the dlib with a
  # message that names none of this. Fail here, where the name of the missing variable is known.
  $item = Get-Item "Env:$n" -ErrorAction SilentlyContinue
  if ((-not $item) -or [string]::IsNullOrWhiteSpace($item.Value)) { Write-Output "missing env $n"; exit 1 }
}
$dir = Split-Path $Path -Parent
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force $dir | Out-Null }
$obj = [ordered]@{
  Endpoint = $env:TS_ENDPOINT
  CodeSigningAccountName = $env:TS_ACCOUNT
  CertificateProfileName = $env:TS_PROFILE
  ExcludeCredentials = @("ManagedIdentityCredential", "SharedTokenCacheCredential", "VisualStudioCredential", "VisualStudioCodeCredential", "InteractiveBrowserCredential")
}
# UTF-8 WITHOUT a BOM, like every other JSON this repo writes: a BOM in front of `{` makes the
# file fail to parse in the dlib, in jq and in a browser.
[System.IO.File]::WriteAllText($Path, ($obj | ConvertTo-Json -Depth 4), (New-Object System.Text.UTF8Encoding($false)))
Write-Output "signing profile written to $Path"
exit 0
