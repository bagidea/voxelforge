# Flamingo G7 / A2 — pass 2 of the alpha probe: a FINE uniform-alpha ladder.
#
# Pass 1 read alpha back by interpolating the ship frame against uniform-alpha
# frames at 0.05/0.10/0.20/0.40/1.00. Between a=0 and a=0.05 that chord is a
# straight line under a CONCAVE tonemap response, so it over-reads alpha exactly
# in the 0-5% near-field bucket the whole A2 argument is about -- i.e. the
# instrument would manufacture the defect it is being used to measure.
#
# Fix: sample the ladder where the answer lives. 0.5% .. 12% in eight steps makes
# the interpolation error second-order instead of the signal.
$ErrorActionPreference = 'Continue'
Set-Location (Split-Path -Parent $PSScriptRoot)

$OUT = '_flamingo_g7probe'
$EXE = (Resolve-Path 'target-flamingo\perf\voxelforge.exe').Path
New-Item -ItemType Directory -Force $OUT | Out-Null
Write-Host "EXE mtime = $((Get-Item $EXE).LastWriteTime)  size = $((Get-Item $EXE).Length)"

$alphas = 0.005, 0.01, 0.02, 0.03, 0.05, 0.08, 0.12, 0.16, 0.25, 0.60
foreach ($a in $alphas) {
  $label = "u" + [int]($a * 1000)      # u5 = 0.5%, u120 = 12%
  $start = -30000 * $a
  $end = 30000 - 30000 * $a
  $png = (Resolve-Path $OUT).Path + "\$label.png"
  if (Test-Path ((Resolve-Path $OUT).Path + "\$label-nohud2.png")) {
    Write-Host "$label already shot - skip"; continue
  }
  Remove-Item $png -ErrorAction SilentlyContinue

  [Environment]::SetEnvironmentVariable('VOXELFORGE_PLAY', '1')
  [Environment]::SetEnvironmentVariable('VOXELFORGE_SHOT', $png)
  [Environment]::SetEnvironmentVariable('VOXELFORGE_LOOK_CAM', '35,-18,26')
  [Environment]::SetEnvironmentVariable('VOXELFORGE_LOOK_FOG', "$start,$end")

  $p = Start-Process -FilePath $EXE -NoNewWindow -Wait -PassThru `
    -RedirectStandardOutput "$OUT\$label.out.log" -RedirectStandardError "$OUT\$label.err.log"

  [Environment]::SetEnvironmentVariable('VOXELFORGE_LOOK_FOG', $null)
  [Environment]::SetEnvironmentVariable('VOXELFORGE_PLAY', $null)
  [Environment]::SetEnvironmentVariable('VOXELFORGE_SHOT', $null)

  if (-not (Test-Path $png)) { Write-Host "$label MISS (exit=$($p.ExitCode))"; continue }
  Start-Process -FilePath 'python' -ArgumentList "scripts\_flamingo_dehud2.py", "$png" `
    -NoNewWindow -Wait -RedirectStandardOutput "$OUT\$label.dehud.log" -RedirectStandardError "$OUT\$label.dehud.err"
  Write-Host "$label OK  a=$a  fog=$start,$end"
}
Write-Host "=== PROBE2 DONE $(Get-Date -Format o) ==="
