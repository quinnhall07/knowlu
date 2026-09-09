# Puts the GNU host toolchain for the x86_64-pc-windows-gnu Rust target on PATH on a GitHub runner.
#
# It downloads the same WinLibs POSIX MSVCRT release the dev machine runs (winget
# BrechtSanders.WinLibs.POSIX.MSVCRT) and refuses to extract it unless the bytes match the pinned
# SHA-256. Nothing is installed outside $env:RUNNER_TEMP. Exit 1 on any failure - a silently missing
# `as` reads as `dlltool ... CreateProcess` failures deep inside a windows-* crate build (CLAUDE.md).
#
# The runner image does carry a mingw-w64 of its own, and C0 Task 1 proved the workspace green on it
# (run 34341678223). We do not use it: it is a UCRT build we do not target, the image readme does not
# document it, its version is not ours to pin, and it is on PATH only ahead of Strawberry Perl's much
# older gcc. Pinning the laptop's exact compiler is what makes a CI build and a laptop build the same
# build. Bumping it is a one-line change here, with a new digest.
$ErrorActionPreference = "Stop"
$url = "https://github.com/brechtsanders/winlibs_mingw/releases/download/16.1.0posix-14.0.0-msvcrt-r4/winlibs-x86_64-posix-seh-gcc-16.1.0-mingw-w64msvcrt-14.0.0-r4.zip"
$sha = "3e1627cada82e8ad18b20dc6456d8fb23da221a29eacc26bdcc9576b8043770f"
$zip = Join-Path $env:RUNNER_TEMP "winlibs.zip"
Invoke-WebRequest -Uri $url -OutFile $zip
$got = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant()
if ($got -ne $sha.ToLowerInvariant()) { Write-Output "winlibs digest mismatch: $got"; exit 1 }
$dst = Join-Path $env:RUNNER_TEMP "winlibs"
Expand-Archive $zip -DestinationPath $dst -Force
$bin = Join-Path $dst "mingw64\bin"
if (-not (Test-Path (Join-Path $bin "gcc.exe"))) {
  $bin = Get-ChildItem $dst -Recurse -Filter gcc.exe | Select-Object -First 1 | ForEach-Object { $_.DirectoryName }
}
if (-not $bin) { Write-Output "no gcc.exe in the WinLibs archive"; exit 1 }
Add-Content $env:GITHUB_PATH $bin
Write-Output "toolchain: winlibs at $bin"
exit 0
