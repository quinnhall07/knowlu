# Puts a GNU host toolchain on PATH for the x86_64-pc-windows-gnu Rust target on a GitHub runner.
# -Flavor image   : whatever mingw-w64 the runner image already carries (probed, not assumed).
# -Flavor winlibs : the pinned WinLibs POSIX MSVCRT release the dev machine uses, verified by SHA-256.
# Never installs anything outside $env:RUNNER_TEMP. Exit 1 on any failure - a silently missing `as`
# reads as `dlltool ... CreateProcess` failures deep inside a windows-* crate build (CLAUDE.md).
param([ValidateSet("image", "winlibs")][string]$Flavor = "image")
$ErrorActionPreference = "Stop"
if ($Flavor -eq "image") {
  $onPath = Get-Command gcc.exe -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($onPath) {
    Write-Output ("toolchain: image mingw at " + (Split-Path $onPath.Source -Parent))
    exit 0
  }
  $candidates = @("C:\mingw64\bin", "C:\ProgramData\mingw64\mingw64\bin", "C:\msys64\mingw64\bin")
  foreach ($dir in $candidates) {
    if (Test-Path (Join-Path $dir "gcc.exe")) {
      Add-Content $env:GITHUB_PATH $dir
      Write-Output "toolchain: image mingw at $dir"
      exit 0
    }
  }
  Write-Output ("no mingw gcc.exe on this image (looked on PATH, then " + ($candidates -join ", ") + ")")
  exit 1
}
# WinLibs: the exact asset the laptop has (winget BrechtSanders.WinLibs.POSIX.MSVCRT), gcc 16.1.0 /
# binutils 2.47.20260726, POSIX threads, SEH, msvcrt CRT. The runner verifies the bytes it downloads.
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
