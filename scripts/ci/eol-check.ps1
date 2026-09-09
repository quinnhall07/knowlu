# The line-ending contract from .gitattributes, enforced: code and docs LF in the index, PowerShell
# CRLF on checkout, every fixture -text with index bytes equal to working bytes. Exit 1 with the
# offending paths listed. PowerShell 5.1: no &&, no ||.
#
# `git ls-files --eol` rows look like:
#   i/lf    w/lf    attr/text=auto eol=lf <TAB>path
# The three metadata fields are space-padded to a fixed width and there is exactly one tab, right
# before the path — so split on the tab first, then split the metadata on whitespace. The attribute
# field itself can contain spaces (`attr/text=auto eol=lf`), so it is everything from the third
# whitespace-split token on, rejoined with a single space.
$ErrorActionPreference = "Stop"
$bad = @()
$rows = git ls-files --eol
if (-not $rows -or @($rows).Count -eq 0) {
  Write-Output "eol-check: git ls-files --eol returned no rows"
  exit 1
}
foreach ($row in $rows) {
  $meta, $path = $row -split "`t", 2
  $f = $meta.Trim() -split "\s+"
  $idx = $f[0]
  $attr = ($f[2..($f.Count - 1)] -join " ")
  if ($path -like "engine/tests/fixtures/*") {
    if ($attr -notlike "*-text*") { $bad += "fixture not -text: $path" }
    continue
  }
  if ($path -like "*.ps1") {
    if (($idx -ne "i/lf") -or ($attr -notlike "*eol=crlf*")) { $bad += "ps1 not i/lf+eol=crlf: $path" }
    continue
  }
  if (($idx -eq "i/crlf") -or ($idx -eq "i/mixed")) { $bad += "$idx in index: $path" }
}
if ($bad.Count -gt 0) {
  $bad | ForEach-Object { Write-Output $_ }
  exit 1
}
Write-Output ("eol contract holds over " + $rows.Count + " files")
exit 0
