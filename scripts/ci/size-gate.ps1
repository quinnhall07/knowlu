# The size budget, enforced on the release runner and nowhere else. Knowlu is a small desktop app
# for students on whatever laptop they own; the crate budget in the rewrite design is what keeps it
# that way, and a dependency that quietly doubles the download is exactly the change nobody notices
# in a diff. A breach fails the release so it becomes a decision, never a silent ship.
#
# The ceilings are deliberately loose - roughly double today's engine and well above today's
# installer - so this fires on a regression, not on ordinary growth. Raising one is a code change
# here, in a commit that says why.
#
# 1MB in PowerShell is 1048576 (MiB), which is what the messages say.
# PowerShell 5.1: no &&, no ||, no ternary, no ??.
param([Parameter(Mandatory=$true)][string]$Engine, [Parameter(Mandatory=$true)][string]$Installer)
$ErrorActionPreference = "Stop"
$e = (Get-Item $Engine).Length; $i = (Get-Item $Installer).Length
Write-Output ("engine " + [math]::Round($e/1MB,2) + " MB, installer " + [math]::Round($i/1MB,2) + " MB")
if ($e -gt 6MB) { Write-Output "engine over 6 MiB"; exit 1 }
if ($i -gt 15MB) { Write-Output "installer over 15 MiB"; exit 1 }
exit 0
