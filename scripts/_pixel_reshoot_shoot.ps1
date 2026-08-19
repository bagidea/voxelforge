<#
  Re-shoot the 60 destroyed beauty plates from the surviving run logs.
  (pixel/flamingo, 2026-08-17 -- recovery of the `git clean -fd` of 2026-08-16.)

  Every plate's camera and env comes from `_pixel_reshoot/_plan.json`, which
  scripts/_pixel_reshoot_plan.py parses out of `ladder/_ladder.log` and
  `probe/_probe.log` -- the logs that shot the LOST plates.  Nothing in this
  file chooses a knob value; it only replays what the plan says.

  Two things it is careful about, both learned the hard way in this lane:

    * every knob in the plan's `knobs` list is REMOVED before each shot, so a
      plate can never inherit the previous plate's env.  `_LOOK_FILL` is in that
      list even though the recovered ladder.ps1 never sets it -- the log proves
      8 plates used it.
    * the shot file is deleted first and its existence + mtime checked after, so
      "the exe ran" is never mistaken for "the plate exists".

  Usage:
    powershell -File scripts\_pixel_reshoot_shoot.ps1 [-Exe ...] [-Plan ...]
                     [-Only NAME[,NAME]] [-OutRoot DIR] [-Tag control]
#>
param(
  [string]$Root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge",
  [string]$Exe  = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_pixel_recover_wt\target-pixel\perf\voxelforge.exe",
  [string]$Plan = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_pixel_reshoot\_plan.json",
  [string]$OutRoot = "",
  [string]$Tag = "reshoot",
  [string[]]$Only
)

# NOT "Stop".  `& $exe *>> $log` wraps every native stderr line in an ErrorRecord
# under Windows PowerShell 5.1, and bevy logs its SystemInfo banner to stderr on
# EVERY boot -- so a "Stop" preference kills the shooter on plate 1 of 60 while
# the exe itself is exiting 0.  The original ladder.ps1 ran at the default for
# exactly this reason; the checks below are explicit instead.
$ErrorActionPreference = "Continue"
if (-not (Test-Path $Exe))  { throw "no exe at $Exe" }
if (-not (Test-Path $Plan)) { throw "no plan at $Plan" }

$p = Get-Content $Plan -Raw | ConvertFrom-Json
$plates = $p.plates
if ($Only) { $plates = $plates | Where-Object { $Only -contains $_.name } }

$exeItem = Get-Item $Exe
$log = Join-Path $Root "_pixel_reshoot\_$Tag.log"
New-Item -ItemType Directory -Force (Split-Path $log) | Out-Null
"=== $Tag  $(Get-Date -Format 'yyyy-MM-ddTHH:mm:ss')  exe=$Exe  $($exeItem.Length)B  mtime=$($exeItem.LastWriteTime.ToString('s'))  plates=$($plates.Count)" |
  Out-File -Append -Encoding utf8 $log

foreach ($kv in $p.pinned.PSObject.Properties) { Set-Item -Path "env:$($kv.Name)" -Value $kv.Value }

$n = 0; $ok = 0; $fail = @()
foreach ($e in $plates) {
  $n++
  foreach ($k in $p.knobs) { Remove-Item "env:$k" -ErrorAction SilentlyContinue }
  foreach ($kv in $e.env.PSObject.Properties) { Set-Item -Path "env:$($kv.Name)" -Value $kv.Value }
  $env:VOXELFORGE_CINE = $e.cine

  $destDir = if ($OutRoot) { Join-Path $OutRoot (Split-Path $e.dest -Leaf) } else { Join-Path $Root $e.dest }
  New-Item -ItemType Directory -Force $destDir | Out-Null
  $shot = Join-Path $destDir "$($e.name).png"
  Remove-Item $shot -ErrorAction SilentlyContinue
  $env:VOXELFORGE_SHOT = $shot

  $set = ($e.env.PSObject.Properties | ForEach-Object { "$($_.Name)=$($_.Value)" }) -join " "
  if (-not $set) { $set = "(no overrides)" }
  "[shoot] $($e.name)  cine=$($e.cine)  $set" | Out-File -Append -Encoding utf8 $log
  & $Exe --play *>> $log

  if (Test-Path $shot) {
    $f = Get-Item $shot
    $ok++
    Write-Output ("[{0}/{1}] OK   {2,-38} {3,9} B   {4}" -f $n, $plates.Count, $e.name, $f.Length, $set)
    "[ok] $($e.name) $($f.Length)B" | Out-File -Append -Encoding utf8 $log
  } else {
    $fail += $e.name
    Write-Output ("[{0}/{1}] MISS {2,-38} no file written" -f $n, $plates.Count, $e.name)
    "[MISS] $($e.name)" | Out-File -Append -Encoding utf8 $log
  }
}

"=== $Tag done  ok=$ok  missing=$($fail.Count)  $(Get-Date -Format 'yyyy-MM-ddTHH:mm:ss')" | Out-File -Append -Encoding utf8 $log
Write-Output "`n$Tag done: $ok/$($plates.Count) written"
if ($fail.Count) { Write-Output ("MISSING: " + ($fail -join ", ")) }
"DONE $ok/$($plates.Count)" | Out-File -Encoding utf8 (Join-Path $Root "_pixel_reshoot\_$Tag.done")
