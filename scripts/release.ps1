# Knowlu release (plan 4a, Task 9): both binaries from ONE commit, signed by the bundler as it
# packs (bundle.windows.signCommand -> scripts\sign.ps1), manifest written, artefacts dropped in
# site\releases\.
#
# PowerShell 5.1: no &&, no ||, no ternary, no ?? - chains are `A; if ($?) { B }`.
#
# NO SECRET IS EVER WRITTEN TO DISK. The Tauri updater's private key is read from Windows
# Credential Manager into an environment variable for the length of one `cargo tauri build` and
# cleared in a finally (R-P4a-14). Authenticode signing needs no secret at all: Trusted Signing
# authenticates through Quinn's own Azure login, and sign.ps1 reads the profile from outside the
# repo.
#
# This script NEVER uploads and never signs after the fact. `site\` goes to Cloudflare Pages by
# hand (plan 4a Task 10).
param(
  [switch]$SkipBuild,
  [switch]$StageOnly,
  [string]$UpdaterCredential = "knowlu/updater-key",
  [string]$UpdaterPasswordCredential = "knowlu/updater-key-password"
)
$ErrorActionPreference = "Stop"

# Every native exe is run through this. In PowerShell 5.1 a native command's stderr becomes
# ErrorRecords the moment anything merges the streams, and under $ErrorActionPreference = "Stop"
# that TERMINATES the script - on output that is not an error at all. Both offenders are here:
# `git diff` prints "LF will be replaced by CRLF" on this repo every single run, and cargo writes
# its entire progress log to stderr. So errors are made non-terminating for the call and the exit
# code is the only thing judged, which is also more honest than $? for a native command.
#
# What it hands back is the native command's own stdout, and nothing else - PowerShell's implicit
# output, which the `rustc -vV` call site captures and reads a `host:` line out of. It deliberately
# adds no status of its own: a function that both writes output and returns a value hands the caller
# an array with a Boolean stuck on the end of it. $LASTEXITCODE is global, so every caller judges
# the exit code by reading that after the call, never by what came back.
function Invoke-Native {
  param([Parameter(Mandatory = $true)][string]$Exe, [string[]]$Arguments = @())
  # Cleared FIRST. If the exe cannot be launched at all, PowerShell writes a non-terminating error
  # and leaves $LASTEXITCODE holding the PREVIOUS native command's value - so a cargo that is not
  # on PATH would read as the last `git diff --cached --quiet`'s 0, and the script would stage a
  # stale engine and publish a stale installer as this release. $null afterwards means "never ran",
  # and every caller tests for it.
  $global:LASTEXITCODE = $null
  $prev = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try { & $Exe @Arguments } finally { $ErrorActionPreference = $prev }
}
$repo = Split-Path $PSScriptRoot -Parent
$conf = Get-Content (Join-Path $repo "app\tauri.conf.json") -Raw | ConvertFrom-Json
$version = $conf.version
Write-Output "Knowlu $version"

# The sidecar's stem, in ONE place. This line and `bundle.externalBin` in app\tauri.conf.json are
# the only two spellings (app\build.rs's placeholder is pinned to the config by a static test), and
# the check below refuses to build if they ever disagree.
$sidecar = "knowlu-engine"
# Required by Tauri on an externalBin file and stripped at install time, so the installed name is
# `$sidecar.exe` beside `knowlu.exe` - exactly where scheduler::engine_exe() looks.
# m2: derived, not hard-coded. rustc's own `host:` line is the triple cargo will actually build,
# so a toolchain change cannot leave the staged file under a name the bundler will not look for.
$null = Get-Command rustc -ErrorAction Stop
$rustcVv = Invoke-Native rustc @("-vV")
if (($null -eq $LASTEXITCODE) -or ($LASTEXITCODE -ne 0)) { throw "rustc -vV failed - is the Rust toolchain on PATH?" }
$hostLine = $rustcVv | Where-Object { $_ -like "host:*" } | Select-Object -First 1
if (-not $hostLine) { throw "rustc -vV printed no host: line, so the sidecar target triple cannot be derived" }
$triple = ($hostLine -replace "^host:", "").Trim()
if ($triple -eq "") { throw "rustc -vV host: line was empty" }
$declared = [string]$conf.bundle.externalBin[0]
if ($declared -ne ("binaries/" + $sidecar)) {
  throw "bundle.externalBin is '$declared' but this script stages 'binaries/$sidecar' - the sidecar is named in one place, and the two have drifted"
}

# --- Credential Manager, read-only, in-memory ------------------------------------------------
# The engine reads credentials in Rust and the app writes them in Rust; PowerShell has no cmdlet
# for generic credentials, so this is the same CredReadW through Add-Type. The blob is UTF-16LE
# and CredentialBlobSize counts BYTES, not code units - the one trap wincred.rs documents.
# Guarded: Add-Type throws if the type is already defined, which a second run in one session hits.
# No -UsingNamespace here: -MemberDefinition already emits `using System.Runtime.InteropServices;`
# itself, and Add-Type compiles with warnings-as-errors, so naming it again is a hard build error
# ("The using directive ... appeared previously in this namespace"). ComTypes.FILETIME below is
# spelled out in full for the same reason.
if (-not ("Knowlu.Cred" -as [type])) {
  Add-Type -Namespace Knowlu -Name Cred -MemberDefinition @'
[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
public struct CREDENTIAL {
  public uint Flags; public uint Type; public string TargetName; public string Comment;
  public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
  public uint CredentialBlobSize; public IntPtr CredentialBlob; public uint Persist;
  public uint AttributeCount; public IntPtr Attributes; public string TargetAlias; public string UserName;
}
[DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
public static extern bool CredReadW(string target, uint type, uint flags, out IntPtr credential);
[DllImport("advapi32.dll")] public static extern void CredFree(IntPtr buffer);
public static string Read(string target) {
  IntPtr p;
  if (!CredReadW(target, 1, 0, out p)) { return null; }
  try {
    CREDENTIAL c = (CREDENTIAL)Marshal.PtrToStructure(p, typeof(CREDENTIAL));
    return Marshal.PtrToStringUni(c.CredentialBlob, (int)(c.CredentialBlobSize / 2));
  } finally { CredFree(p); }
}
'@
}

# --- the toolchain, resolved before anything is judged by an exit code -------------------------
# A missing exe is a launch failure, not an exit code, and PowerShell reports it without touching
# $LASTEXITCODE. Resolving cargo and git here turns "not installed" into one clear message instead
# of a build that silently looks like it succeeded. `cargo tauri` is a SUBCOMMAND, so Get-Command
# cannot see it - it is probed by running it, and only when this run will actually bundle.
$null = Get-Command cargo -ErrorAction Stop
$null = Get-Command git -ErrorAction Stop
if (-not $StageOnly) {
  Invoke-Native cargo @("tauri", "--version")
  if (($null -eq $LASTEXITCODE) -or ($LASTEXITCODE -ne 0)) { throw "cargo tauri is not available - install it with: cargo install tauri-cli" }
}

# --- one commit, both binaries ----------------------------------------------------------------
# `git diff --quiet`, NOT `git status --porcelain`: `cargo tauri build` invalidates app\Cargo.toml's
# stat cache without changing a byte, so `status` reports a modification a content diff does not
# (Task 1 spike, concern 1). Untracked files are build products (app\binaries\, site\releases\) and
# are ignored here by design.
Push-Location $repo
Invoke-Native git @("diff", "--quiet")
$treeClean = (($null -ne $LASTEXITCODE) -and ($LASTEXITCODE -eq 0))
if ($treeClean) {
  Invoke-Native git @("diff", "--cached", "--quiet")
  $treeClean = (($null -ne $LASTEXITCODE) -and ($LASTEXITCODE -eq 0))
}
Pop-Location
# -StageOnly produces no installer, so there is nothing for a commit to match, and a developer
# staging a real sidecar beside a plain cargo build is expected to have a dirty tree.
if ((-not $StageOnly) -and (-not $treeClean)) { throw "working tree has uncommitted changes - commit or stash before releasing, so the installer matches a commit" }

if (-not $SkipBuild) {
  Push-Location $repo
  Invoke-Native cargo @("build", "--release", "-p", "knowlu-engine")
  $engineOk = (($null -ne $LASTEXITCODE) -and ($LASTEXITCODE -eq 0))
  Pop-Location
  if (-not $engineOk) { throw "engine build failed" }
}

$bin = Join-Path $repo "app\binaries"
if (-not (Test-Path $bin)) { New-Item -ItemType Directory -Force $bin | Out-Null }
$engineExe = Join-Path $repo ("target\release\" + $sidecar + ".exe")
if (-not (Test-Path $engineExe)) { throw "no engine at $engineExe - run without -SkipBuild" }
$staged = Join-Path $bin ($sidecar + "-" + $triple + ".exe")
Copy-Item $engineExe $staged -Force

# R-P4a-26: never bundle the placeholder. app\build.rs drops a ZERO-BYTE file here when the sidecar
# is missing, so a plain cargo build / cargo test works on a checkout that has never released; that
# file must never reach an installer, where it would be an engine that cannot run.
$stagedLen = (Get-Item $staged).Length
if ($stagedLen -lt 1MB) { throw "sidecar at $staged is $stagedLen bytes; stage the real engine (a zero-byte file is the build.rs placeholder)" }

if ($StageOnly) {
  Write-Output ("staged " + $staged + " (" + [math]::Round($stagedLen / 1MB, 2) + " MB)")
  Write-Output "-StageOnly: nothing was bundled. A plain cargo build in app\ now copies the real engine beside knowlu.exe."
  exit 0
}

# --- does this build want updater artefacts? ---------------------------------------------------
# `createUpdaterArtifacts` and `plugins.updater` are ONE decision, not two: tauri-cli 2.11.4
# (src\interface\rust.rs, get_bundle_settings) reads `plugins > updater` whenever
# createUpdaterArtifacts is anything but false, and fails the whole build with "failed to get
# updater configuration: plugins > updater doesn't exist" when it is missing. The two therefore
# land together, in plan 4a Task 8's key-gated step. Until then this says so once, loudly, and
# ships the installer alone.
$cua = $conf.bundle.createUpdaterArtifacts
$wantsUpdater = $false
if ($null -ne $cua) {
  if ($cua -isnot [bool]) { $wantsUpdater = $true } elseif ($cua) { $wantsUpdater = $true }
}
$hasUpdaterPlugin = $false
if ($conf.PSObject.Properties.Name -contains "plugins") {
  if ($null -ne $conf.plugins) {
    if ($conf.plugins.PSObject.Properties.Name -contains "updater") { $hasUpdaterPlugin = $true }
  }
}
if ($wantsUpdater -and (-not $hasUpdaterPlugin)) {
  throw "bundle.createUpdaterArtifacts is set but plugins.updater is missing from app\tauri.conf.json - cargo tauri build fails before it bundles. Land plan 4a Task 8's gated step, or set createUpdaterArtifacts to false."
}
$needsKey = ($wantsUpdater -and $hasUpdaterPlugin)
if (-not $needsKey) {
  Write-Output "UPDATER OFF: createUpdaterArtifacts is false / plugins.updater is absent - this build ships an installer only, and friends will not be offered an in-app update (plan 4a Task 8)."
}

# --- the updater key, for exactly one build ----------------------------------------------------
$key = $null
$pw = $null
if ($needsKey) {
  $key = [Knowlu.Cred]::Read($UpdaterCredential)
  if (-not $key) {
    throw "updater artefacts are enabled but $UpdaterCredential is not in Credential Manager. Create it with: cmdkey /generic:$UpdaterCredential /user:knowlu /pass"
  }
}
# --- can anything be signed at all? ------------------------------------------------------------
# Before the bundler decides to sign the sidecar it asks signtool whether the sidecar is ALREADY
# signed (tauri-bundler 2.9.4, bundle.rs: `verify(&path)` over settings.external_binaries()), and
# that probe needs signtool.exe even though the signing itself is a custom command. So on a machine
# with no Windows SDK the whole bundle dies with "SignTool not found" BEFORE sign.ps1 is ever
# reached - which would defeat R-P4a-16's own rule that an unsigned dev build must still bundle.
#
# TAURI_SKIP_SIDECAR_SIGNATURE_CHECK is the bundler's escape hatch, and it is BLUNT: its `continue`
# skips the whole sidecar branch (bundle.rs), so it does not merely skip the probe - it disables
# sidecar SIGNING outright. Never reach for it on a signing machine, or the sidecar ships unsigned
# inside a signed installer. It
# is therefore set ONLY when signtool is missing, which is exactly when the sidecar could not have
# been signed anyway - so it never suppresses signing that would otherwise happen. With an SDK
# installed the variable is never set, verify() runs, and the sidecar is signed through sign.ps1
# like every other binary. Nothing else in this config needs signtool: the main binary, the
# WebView2 loader, the NSIS plugin DLLs and the installer all go through try_sign -> sign_custom,
# and should_sign() is only reached for bundle.resources, which this config does not declare.
$signToolPresent = [bool]([string](& (Join-Path $PSScriptRoot "find-signtool.ps1")))
if (-not $signToolPresent) {
  Write-Output "UNSIGNED BUILD: no signtool.exe on PATH or under the Windows Kits registry root, so the bundler's sidecar signature check is skipped and NOTHING in this installer is signed. Install the Windows SDK (winget install Microsoft.WindowsSDK.10.0.26100) and a Trusted Signing profile before this reaches anyone."
}

try {
  if (-not $signToolPresent) { $env:TAURI_SKIP_SIDECAR_SIGNATURE_CHECK = "true" }
  if ($key) {
    $env:TAURI_SIGNING_PRIVATE_KEY = $key
    $pw = [Knowlu.Cred]::Read($UpdaterPasswordCredential)
    if ($pw) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $pw }
  }
  Push-Location (Join-Path $repo "app")
  Invoke-Native cargo @("tauri", "build")
  $built = (($null -ne $LASTEXITCODE) -and ($LASTEXITCODE -eq 0))
  Pop-Location
  if (-not $built) { throw "cargo tauri build failed - if it stopped at 'Verifying NSIS package' the bundler could not reach GitHub/Microsoft for NSIS, nsis_tauri_utils.dll and the WebView2 bootstrapper, which it downloads once and caches under %LOCALAPPDATA%\tauri\" }
} finally {
  # Cleared whatever happened, including on the throw above.
  if (Test-Path Env:\TAURI_SIGNING_PRIVATE_KEY) { Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY }
  if (Test-Path Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD }
  $key = $null
  $pw = $null
  if (Test-Path Env:\TAURI_SKIP_SIDECAR_SIGNATURE_CHECK) { Remove-Item Env:\TAURI_SKIP_SIDECAR_SIGNATURE_CHECK }
}

$bundle = Join-Path $repo "target\release\bundle\nsis"
# -like, not -Filter: the FileSystem provider's -Filter uses Win32 wildcards, where a 3-letter
# extension can also match a longer one through 8.3 aliasing (the classic *.htm matching .html).
# Here that would let "*.nsis.zip" pick up a .nsis.zip.sig. -like is plain PowerShell globbing.
$setup = Get-ChildItem $bundle -File | Where-Object { $_.Name -like "*-setup.exe" } | Sort-Object LastWriteTime | Select-Object -Last 1
if (-not $setup) { throw "no installer in $bundle" }
Write-Output ("installer: " + $setup.FullName + " (" + [math]::Round($setup.Length / 1MB, 2) + " MB)")
Write-Output "authenticode: whatever scripts\sign.ps1 printed above - an UNSIGNED: line means Windows will warn on install"

# --- artefacts and the manifest ----------------------------------------------------------------
$rel = Join-Path $repo "site\releases"
if (-not (Test-Path $rel)) { New-Item -ItemType Directory -Force $rel | Out-Null }
Copy-Item $setup.FullName $rel -Force
# M5: a stable name the download page can link forever, beside the versioned file.
Copy-Item $setup.FullName (Join-Path $rel "Knowlu-setup.exe") -Force

if ($needsKey) {
  # Tauri 2's updater artefact IS the installer, with a detached minisign signature beside it:
  # Knowlu_<v>_x64-setup.exe plus .sig. The .nsis.zip pair this script first looked for is the v1
  # shape, emitted only by createUpdaterArtifacts: "v1Compatible", which tauri.conf.json does not
  # set. The bundler said "Finished 1 updater signature" and this still threw. Found by the first
  # release rehearsal, 2026-09-07, which is what a rehearsal is for.
  $sig = Get-ChildItem $bundle -File | Where-Object { $_.Name -like "*-setup.exe.sig" } | Sort-Object LastWriteTime | Select-Object -Last 1
  if (-not $sig) { throw "no .sig beside the installer - the updater key produced nothing" }
  if ($sig.BaseName -ne $setup.Name) { throw ("signature " + $sig.Name + " does not belong to installer " + $setup.Name) }
  $manifest = [ordered]@{
    version   = $version
    notes     = "Knowlu $version"
    pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    platforms = [ordered]@{
      "windows-x86_64" = [ordered]@{
        signature = (Get-Content $sig.FullName -Raw).Trim()
        # The VERSIONED name, never the stable Knowlu-setup.exe: a signature is valid for exactly
        # these bytes, and the stable name is overwritten by the next release.
        url       = "https://knowlu.com/releases/" + $setup.Name
      }
    }
  }
  # S10: UTF-8 WITHOUT a BOM. Out-File -Encoding utf8 writes one in 5.1, and a BOM in front of `{`
  # makes the manifest fail to parse in the updater, in a browser, and in jq.
  $json = $manifest | ConvertTo-Json -Depth 6
  [System.IO.File]::WriteAllText((Join-Path $rel "latest.json"), $json, (New-Object System.Text.UTF8Encoding($false)))
  Write-Output ("manifest: " + (Join-Path $rel "latest.json"))
} else {
  Write-Output "no updater artefacts (see UPDATER OFF above) - installer only, and no latest.json was written"
}
Write-Output "upload site\ to Cloudflare Pages by hand (plan 4a Task 10; no account yet)"
