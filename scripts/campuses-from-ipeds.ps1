# scripts/campuses-from-ipeds.ps1 -- regenerate app/campuses.json from IPEDS.
#
# Run by hand, about once a year, when NCES publishes the next HD file. The output is COMMITTED: the
# wizard's school typeahead has to work on a first run with no network, which is most first runs.
#
# US federal data (NCES), public domain. The header line records which file this came from and when,
# so "is this current?" is answerable without re-downloading it.
#
# PowerShell 5.1. `Import-Csv -Encoding Default` reads the ANSI code page, which is what latin-1 means
# on a US Windows install -- the file has accented institution names and reading it as UTF-8 mangles
# them into replacement characters that then ship to every user.
[CmdletBinding()]
param(
  [string]$Url = "https://nces.ed.gov/ipeds/datacenter/data/HD2024.zip",
  [string]$Csv = "HD2024.csv",
  [string]$Out = "app/campuses.json"
)
$ErrorActionPreference = "Stop"

$work = Join-Path $env:TEMP ("ipeds-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory $work | Out-Null
try {
  $zip = Join-Path $work "hd.zip"
  Write-Output "downloading $Url"
  Invoke-WebRequest -Uri $Url -OutFile $zip
  Expand-Archive -Path $zip -DestinationPath $work -Force
  $csvPath = Join-Path $work $Csv
  if (-not (Test-Path $csvPath)) { throw "$Csv is not in that zip" }

  $rows = Import-Csv -Path $csvPath -Encoding Default
  Write-Output ("read {0} rows from {1}" -f $rows.Count, $Csv)

  # The first column's name carries the file's BOM, so `$_.UNITID` misses on some hosts. Bind the
  # property by position once instead of trusting the name.
  $unitidName = ($rows[0].PSObject.Properties | Select-Object -First 1).Name

  $keep = $rows | Where-Object {
    $_.CYACTIVE -eq "1" -and ($_.ICLEVEL -eq "1" -or $_.ICLEVEL -eq "2")
  }
  Write-Output ("keeping {0} active two- and four-year institutions" -f $keep.Count)

  # A generic list, not `@()` with `+=`: the latter reallocates the whole array on each of six
  # thousand iterations. It finishes either way and this runs once a year — but one line removes the
  # only quadratic thing in the script.
  $campuses = New-Object System.Collections.Generic.List[object]
  foreach ($r in $keep) {
    # `WEBADDR` is inconsistent: some rows carry a scheme, some a path, some a trailing slash, some
    # nothing at all. Keep the HOST and nothing else -- partly because that is all the wizard shows,
    # and partly because `app/tests/onboarding.rs::the_campus_list_is_bundled_headed_and_small`
    # asserts the whole file carries no `http(s)://`. (`app/tests/static_assets.rs` cannot: its
    # `read()` helper resolves against `app/static/`, and this file is `app/campuses.json`.)
    # `$webhost`, not `$host`: `$Host` is a PowerShell automatic variable (the PSHost object), and
    # assigning to it shadows it in this scope. It usually works and PSScriptAnalyzer flags it, which
    # is one rename too many arguments.
    $webhost = ""
    $raw = ($r.WEBADDR + "").Trim()
    if ($raw) {
      $h = $raw -replace '^\s*https?://', ''
      $h = $h -replace '^www\.', ''
      $h = ($h -split '[/?#]')[0]
      $webhost = $h.ToLowerInvariant().Trim()
      if ($webhost -match '\s') { $webhost = "" }
    }
    [void]$campuses.Add(@(
      [int]$r.$unitidName,
      ($r.INSTNM + "").Trim(),
      ($r.CITY + "").Trim(),
      ($r.STABBR + "").Trim(),
      $webhost
    ))
  }

  # One row per line, compact: about 400 KB, diffable, and a new school shows up as one added line
  # rather than a reflowed file.
  $sb = New-Object System.Text.StringBuilder
  # **No URL in the header.** `app/tests/onboarding.rs::the_campus_list_is_bundled_headed_and_small`
  # asserts the whole file carries no `http(s)://` — the same rule that makes this script strip
  # schemes off `WEBADDR` — and a header naming the download would break it on line one. The source is
  # named in words; the URL lives in this script's own `-Url` default, which is where somebody looking
  # to regenerate it will look.
  [void]$sb.AppendLine(('{{"source":"NCES IPEDS {0}","retrieved":"{1}","count":{2},"campuses":[' -f `
    $Csv, (Get-Date -Format "yyyy-MM-dd"), $campuses.Count))
  for ($i = 0; $i -lt $campuses.Count; $i++) {
    $line = ConvertTo-Json $campuses[$i] -Compress
    if ($i -lt $campuses.Count - 1) { $line += "," }
    [void]$sb.AppendLine($line)
  }
  [void]$sb.AppendLine("]}")

  $full = Join-Path (Get-Location) $Out
  # LF, and UTF-8 without a BOM: `.gitattributes` says `* text=auto eol=lf`, and a BOM in a file the
  # page fetches is a parse error in some webviews.
  $text = $sb.ToString() -replace "`r`n", "`n"
  [System.IO.File]::WriteAllText($full, $text, (New-Object System.Text.UTF8Encoding($false)))
  $kb = [math]::Round((Get-Item $full).Length / 1KB)
  Write-Output ("wrote {0} ({1} schools, {2} KB)" -f $Out, $campuses.Count, $kb)
  if ($kb -gt 600) { throw "campuses.json is ${kb} KB; the guard in app/tests/onboarding.rs is 600" }
}
finally {
  Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
}
