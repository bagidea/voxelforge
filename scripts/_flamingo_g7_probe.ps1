# Flamingo G7 / A2 — depth + alpha probe. NO REBUILD: every row is an env hook
# that already exists in the shipped binary.
#
# Two ladders, both built out of `VOXELFORGE_LOOK_FOG=<start>,<end>` (linear):
#
#   slice_S   FOG=S,S+0.5      alpha is a STEP at distance S: every pixel nearer
#                              than S is byte-identical to the no-fog frame, every
#                              pixel further is 100% fog colour. Banding the frame
#                              across S gives an EXACT per-pixel depth map, which is
#                              the number the whole A2 argument rests on and which
#                              nobody has actually measured.
#
#   uni_A     FOG=-30000A,30000-30000A
#                              alpha is CONSTANT ~A over the entire depth range
#                              (it varies by 1% absolute across 0..300 blocks).
#                              A frame with a known, uniform alpha is the response
#                              curve needed to read a measured alpha back out of the
#                              real ExponentialSquared frame through the tonemap.
#
# Same shape as _flamingo_g7_sweep.ps1 (Start-Process + redirects; PS 5.1 wraps
# native stderr in ErrorRecords otherwise), minus the grade row so g7.tsv is not
# polluted with diagnostic frames.
$ErrorActionPreference = 'Continue'
Set-Location (Split-Path -Parent $PSScriptRoot)

$OUT = '_flamingo_g7probe'
$EXE = (Resolve-Path 'target-flamingo\perf\voxelforge.exe').Path
New-Item -ItemType Directory -Force $OUT | Out-Null
Write-Host "EXE = $EXE"
Write-Host "EXE mtime = $((Get-Item $EXE).LastWriteTime)  size = $((Get-Item $EXE).Length)"

$rows = @()
foreach ($s in 8, 12, 16, 20, 25, 30, 40, 55, 75, 110) {
  $rows += @{ label = "slice$s"; fog = "$s,$($s + 0.5)" }
}
foreach ($a in 0.05, 0.10, 0.20, 0.40, 1.00) {
  $start = -30000 * $a
  $end = 30000 - 30000 * $a
  $rows += @{ label = ("uni" + ($a * 100)); fog = "$start,$end" }
}

foreach ($r in $rows) {
  $label = $r.label
  $png = (Resolve-Path $OUT).Path + "\$label.png"
  if (Test-Path ((Resolve-Path $OUT).Path + "\$label-nohud2.png")) {
    Write-Host "$label already shot - skip"; continue
  }
  Remove-Item $png -ErrorAction SilentlyContinue

  [Environment]::SetEnvironmentVariable('VOXELFORGE_PLAY', '1')
  [Environment]::SetEnvironmentVariable('VOXELFORGE_SHOT', $png)
  [Environment]::SetEnvironmentVariable('VOXELFORGE_LOOK_CAM', '35,-18,26')
  [Environment]::SetEnvironmentVariable('VOXELFORGE_LOOK_FOG', $r.fog)

  $t0 = Get-Date
  $p = Start-Process -FilePath $EXE -NoNewWindow -Wait -PassThru `
    -RedirectStandardOutput "$OUT\$label.out.log" -RedirectStandardError "$OUT\$label.err.log"
  $dt = [int]((Get-Date) - $t0).TotalSeconds

  [Environment]::SetEnvironmentVariable('VOXELFORGE_LOOK_FOG', $null)
  [Environment]::SetEnvironmentVariable('VOXELFORGE_PLAY', $null)
  [Environment]::SetEnvironmentVariable('VOXELFORGE_SHOT', $null)

  if (-not (Test-Path $png)) {
    Write-Host "$label MISS (exit=$($p.ExitCode), ${dt}s)"
    continue
  }
  Start-Process -FilePath 'python' -ArgumentList "scripts\_flamingo_dehud2.py", "$png" `
    -NoNewWindow -Wait -RedirectStandardOutput "$OUT\$label.dehud.log" -RedirectStandardError "$OUT\$label.dehud.err"
  Write-Host "$label OK  fog=$($r.fog)  ${dt}s"
}
Write-Host "=== PROBE DONE $(Get-Date -Format o) ==="
