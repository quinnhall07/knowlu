<#
.SYNOPSIS
    Is the cloud routine's enrichment step starved yet? Read-only.

.DESCRIPTION
    Knowlu spec 5.1: the app does the routine's work first, at its own slots, and the routine finds
    nothing left each time it wakes. When it has done nothing for a week, it is turned off ONCE, by
    hand, at claude.ai/code/routines. This script is the instrument that says when that week has
    passed - and, just as importantly, when it has NOT.

    It reads two things and writes nothing:

      - state/journal/*.jsonl, for `op: set` records on the enrichment fields, split by actor:
        `agent:routine.enrich` (the cloud routine's step 3) against `agent:knowlu.enrich` (this
        app's judge step). Both producers running is the expected state for weeks.
      - tasks/*.md, for how many notes still carry `needs_enrichment: true` - because "the routine
        wrote nothing" means one thing when the queue is empty and quite another when it is not.

    THIS SCRIPT EDITS NOTHING. Not the routine, not its prompt, not its config, not a note. Turning
    the routine off is a deliberate act by Quinn in a browser; this only says whether the evidence
    supports it. The one thing it can be wrong about is a routine that is failing rather than
    starved, which is why it also prints the last date the routine wrote ANYTHING.

    PowerShell 5.1: no &&, no ||, no ternary, no ??.

.PARAMETER Vault
    The vault to read. Required - there is no default, because the interesting answer is about a
    specific vault and a wrong guess would read the wrong one.

.PARAMETER Days
    How many days back to tabulate. Default 14.

.EXAMPLE
    .\scripts\starvation.ps1 -Vault .
    .\scripts\starvation.ps1 -Vault <path-to-a-vault> -Days 21
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$Vault,
  [int]$Days = 14
)
$ErrorActionPreference = "Stop"

$ENRICHMENT_FIELDS = @("course", "effort_hours", "effort_confidence", "importance", "importance_reason", "needs_enrichment")
$ROUTINE = "agent:routine.enrich"
$KNOWLU  = "agent:knowlu.enrich"

if (-not (Test-Path $Vault)) { Write-Output "no vault at $Vault"; exit 0 }
$journal = Join-Path $Vault "state\journal"

# day -> @{ routine = n; knowlu = n }
$byDay = @{}
$lastRoutine = $null
$lastKnowlu = $null
if (Test-Path $journal) {
  foreach ($file in Get-ChildItem $journal -Filter "*.jsonl" -File) {
    foreach ($line in Get-Content $file.FullName) {
      if ($line.Trim() -eq "") { continue }
      $rec = $null
      # A torn or hand-edited line is skipped, never fatal: this is a report.
      try { $rec = $line | ConvertFrom-Json } catch { continue }
      if ($rec.op -ne "set") { continue }
      if ($ENRICHMENT_FIELDS -notcontains $rec.field) { continue }
      $actor = [string]$rec.actor
      if (($actor -ne $ROUTINE) -and ($actor -ne $KNOWLU)) { continue }
      $day = ([string]$rec.ts).Substring(0, 10)
      if (-not $byDay.ContainsKey($day)) { $byDay[$day] = @{ routine = 0; knowlu = 0 } }
      if ($actor -eq $ROUTINE) {
        $byDay[$day].routine += 1
        if (($null -eq $lastRoutine) -or ($day -gt $lastRoutine)) { $lastRoutine = $day }
      } else {
        $byDay[$day].knowlu += 1
        if (($null -eq $lastKnowlu) -or ($day -gt $lastKnowlu)) { $lastKnowlu = $day }
      }
    }
  }
}

# m6 (final fix wave): $byDay's keys come from the journal's `ts`, which is UTC - so $today must be
# too, or an 18:00 CT enrichment (already tomorrow in UTC) lands in a bucket this table prints as
# "tomorrow" and the seven-day gap can be a day out, in the direction that declares the routine
# starved a DAY EARLY - the wrong direction, since the verdict's whole purpose is to authorise
# turning the live cloud routine off.
$today = [datetime]::UtcNow.Date
Write-Output "day          routine  knowlu"
for ($i = $Days - 1; $i -ge 0; $i--) {
  $day = $today.AddDays(-$i).ToString("yyyy-MM-dd")
  $r = 0; $k = 0
  if ($byDay.ContainsKey($day)) { $r = $byDay[$day].routine; $k = $byDay[$day].knowlu }
  Write-Output ("{0}   {1,7}  {2,6}" -f $day, $r, $k)
}

# Still owed. A queue of zero means neither producer had anything to do, which is NOT evidence that
# the app beat the routine to it.
$flagged = 0
$tasks = Join-Path $Vault "tasks"
if (Test-Path $tasks) {
  foreach ($note in Get-ChildItem $tasks -Filter "*.md" -File) {
    $head = Get-Content $note.FullName -TotalCount 40
    if ($head -match "^needs_enrichment:\s*true\s*$") { $flagged += 1 }
  }
}
Write-Output ""
Write-Output ("{0} task(s) still flagged needs_enrichment: true" -f $flagged)
if ($null -ne $lastKnowlu) { Write-Output ("knowlu last enriched on {0}" -f $lastKnowlu) } else { Write-Output "knowlu has never enriched anything here" }

if ($null -eq $lastRoutine) {
  Write-Output "the routine has never enriched anything in this journal - starved, or it was never running against this vault"
  exit 0
}
$gap = [int]($today - [datetime]::ParseExact($lastRoutine, "yyyy-MM-dd", $null)).TotalDays
Write-Output ("the routine last enriched on {0}, {1} day(s) ago" -f $lastRoutine, $gap)
if ($gap -ge 7) {
  Write-Output "STARVED: seven or more days with no routine enrichment. Spec 5.1 says the routine is now turned off ONCE, by hand, at claude.ai/code/routines - never by editing its prompt or its config."
  Write-Output "Before doing it, check state/runner-log.md: a routine that is FAILING looks exactly like one that is starved from here."
} else {
  Write-Output "NOT STARVED: the routine is still doing enrichment work. Leave it alone - turning it off now would drop judgments nobody else is making yet."
}
exit 0
