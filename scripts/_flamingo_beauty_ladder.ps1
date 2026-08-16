<#
  Beauty-axes ladder (flamingo, 2026-08-16).

  Shoots the full pose x rung GRID on ONE binary.  Every knob under test is read
  from env at spawn (look.rs:1752-1801), which is the entire reason the ladder
  needs no relink between rungs -- and the reason a rung difference cannot be a
  build difference.

  The grid is the deliverable, not a highlight from it.  My own rule from 08-09,
  quoted in docs/flamingo-beauty-axes-2026-08-14.md section 3: a rung is reported
  for EVERY pose, and it only counts as won if it clears the target on every pose
  at once.  So this shoots every rung on every admitted pose -- no early exits, no
  "this one looked good so I stopped".

  Rungs (docs/flamingo-beauty-axes-2026-08-14.md section 3):
    r0            baseline, NO overrides at all -- the thing everything is judged against
    fog8-45 / fog6-55 / fog5-70   knob 3: the Linear{20,150} ramp never engages on a
                                  64-block map; these refit it to the map's real extents
    amb1700 / amb1400 / amb1100   knob 2: ambient_lux 2200 lifts the shade floor to
                                  19-21% of white against a 9.3% reference
    ev11.3 / ev12.0               knob 1's remaining handle.  exp_comp itself was
                                  already fixed at source (5eba1e8, in this binary),
                                  so what is left to steer the sky is ev100.
    win-*         the combination rung, filled in only after the singles are measured

  Every rung also re-shoots r0's exact pose and framing, so each before/after pair
  is the same frame with one knob group moved.

  Usage: powershell -File scripts\_flamingo_beauty_ladder.ps1 [-Poses poses.txt] [-Out DIR]
#>
param(
  [string]$Poses = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\docs\assets\look\beauty\_poses.txt",
  [string]$Out   = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\docs\assets\look\beauty\ladder",
  [string]$Exe   = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_flamingo_beauty_wt\target-pixel\perf\voxelforge.exe",
  [string[]]$OnlyRungs
)

if (-not (Test-Path $Out)) { New-Item -ItemType Directory -Force $Out | Out-Null }
Remove-Item "$Out\_ladder.done" -ErrorAction SilentlyContinue
$log = "$Out\_ladder.log"

# --- pinned on every single plate in the grid -------------------------------
$env:VOXELFORGE_PLAY          = "1"
$env:VOXELFORGE_NOHUD         = "1"
$env:VOXELFORGE_LOOK_QUALITY  = "ultra"
$env:VOXELFORGE_CINE_START    = "1.0"
$env:VOXELFORGE_LOOK_GEN      = "v3"

# --- the rungs: name -> the env deltas that define it ------------------------
$rungs = [ordered]@{
  "r0"       = @{}
  "fog8-45"  = @{ VOXELFORGE_LOOK_FOG = "8,45"  }
  "fog6-55"  = @{ VOXELFORGE_LOOK_FOG = "6,55"  }
  "fog5-70"  = @{ VOXELFORGE_LOOK_FOG = "5,70"  }
  "amb1700"  = @{ VOXELFORGE_LOOK_AMBIENT = "1700" }
  "amb1400"  = @{ VOXELFORGE_LOOK_AMBIENT = "1400" }
  "amb1100"  = @{ VOXELFORGE_LOOK_AMBIENT = "1100" }
  "ev11.3"   = @{ VOXELFORGE_LOOK_EXPOSURE = "11.3" }
  "ev12.0"   = @{ VOXELFORGE_LOOK_EXPOSURE = "12.0" }
  # Added 2026-08-16 AFTER the first 36-plate pass came back with a 0.00% sky
  # mask on all four poses.  Root cause is not a grade knob at all:
  # look.rs:3232 `sky_grad_enabled() = !atmos_enabled() && ...`, and
  # look.rs:3103-3118 defaults VOXELFORGE_LOOK_ATMOS to LookupTexture -- so the
  # shipped default DISABLES the hand-authored SkyDome and lets Bevy's
  # atmosphere own the sky, which renders black in these frames.  `off` is the
  # documented lever that hands the sky back to the dome, on this same binary.
  "atmos-off"      = @{ VOXELFORGE_LOOK_ATMOS = "off" }
  # The combination rung the brief reserves: the sky repair + the single knob
  # that moved value span the most (fog8-45, +22.8..+27.7 on every pose).
  "win-atmosfog"   = @{ VOXELFORGE_LOOK_ATMOS = "off"; VOXELFORGE_LOOK_FOG = "8,45" }
}
if ($OnlyRungs) {
  $keep = [ordered]@{}
  foreach ($k in $rungs.Keys) { if ($OnlyRungs -contains $k) { $keep[$k] = $rungs[$k] } }
  $rungs = $keep
}

$knobs = @("VOXELFORGE_LOOK_FOG","VOXELFORGE_LOOK_AMBIENT","VOXELFORGE_LOOK_EXPOSURE",
           "VOXELFORGE_LOOK_ATMOS")

$poseList = Get-Content $Poses | Where-Object { $_ -match '\S' -and -not $_.StartsWith('#') }
Write-Output "poses=$($poseList.Count)  rungs=$($rungs.Count)  plates=$($poseList.Count * $rungs.Count)"

$n = 0
foreach ($line in $poseList) {
  $parts = $line -split '\|', 2
  $pose  = $parts[0].Trim()
  $cine  = $parts[1].Trim()
  foreach ($rung in $rungs.Keys) {
    # clear every knob first, so a rung can never inherit the previous rung's env
    foreach ($k in $knobs) { Remove-Item "env:$k" -ErrorAction SilentlyContinue }
    foreach ($kv in $rungs[$rung].GetEnumerator()) {
      Set-Item -Path "env:$($kv.Key)" -Value $kv.Value
    }
    $env:VOXELFORGE_CINE = $cine
    $shot = "$Out\$pose-$rung.png"
    $env:VOXELFORGE_SHOT = $shot
    $set = ($rungs[$rung].GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join " "
    if (-not $set) { $set = "(no overrides)" }
    $n++
    Write-Output "[$n/$($poseList.Count * $rungs.Count)] $pose-$rung  $set"
    "[shoot] $pose-$rung  cine=$cine  $set" | Out-File -Append -Encoding utf8 $log
    & $Exe --play *>> $log
  }
}
"DONE" | Out-File -Encoding utf8 "$Out\_ladder.done"
Write-Output "ladder done -> $Out"
