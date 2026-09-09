# Copies a vault's data folders from -Source into a fresh scratch folder under
# %LOCALAPPDATA%\knowlu\scratch\<stamp>, makes it a local-only git repository (no remote), and
# prints the launch line. The console is developed and looked at against THIS, never the live
# vault, until G2 (Knowlu spec §2). Read-only against the source.
param([string]$Source)
if (-not $Source) { throw "pass -Source <vault folder>: this repository holds no vault of its own" }
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$dest = Join-Path $env:LOCALAPPDATA "knowlu\scratch\$stamp"
New-Item -ItemType Directory -Force $dest | Out-Null
foreach ($f in @('tasks','approvals','archive','courses','issues','info','state','config','profile')) {
    $src = Join-Path $Source $f
    if (Test-Path $src) { Copy-Item -Recurse -Force $src (Join-Path $dest $f) }
}
Push-Location $dest
git init -q --initial-branch=main; git add .; git -c user.name=scratch -c user.email=scratch@localhost commit -q -m "scratch copy $stamp"
Pop-Location
Write-Output "scratch vault: $dest"
Write-Output "launch: target\release\knowlu.exe --vault `"$dest`""
