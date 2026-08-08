# Flamingo - G6 exposure/sky sweep on ONE build.
#
# The gate that fails is G6's sunlit-luminance floor (grade_gate.py: L >= 55). The
# lever is lighting, not post, so this shoots the SAME frame repeatedly through the
# look lane's env sweep hooks (VOXELFORGE_LOOK_EXPOSURE / _SUN / _SKY / _AMBIENT)
# and grades each one. One relink, many data points.
#
# Reads a plan file: one line per run, "label|KEY=value;KEY=value" (env may be empty).
# Writes <OUT>\<label>.png, de-HUDs it, and appends the numeric gate row to sweep.tsv.
#
# Start-Process -Redirect* for every native call (PS 5.1 wraps native stderr in
# ErrorRecords and writes UTF-16 - unmatchable logs otherwise).
$ErrorActionPreference = 'Continue'
Set-Location (Split-Path -Parent $PSScriptRoot)

$PLAN = if ($args.Count -ge 1) { $args[0] } else { '_flamingo_g6\plan.txt' }
$OUT  = '_flamingo_g6'
# Which binary to sweep. The lane's own target dir when it has one, else the
# shared `target\` — set FL_EXE to pin it. (target-flamingo never linked: the
# cold build died at bevy_asset, so the honest thing is to sweep the binary that
# actually exists rather than a path that does not.)
$EXE  = if ($env:FL_EXE) { (Resolve-Path $env:FL_EXE).Path }
        else { (Resolve-Path 'target\release\voxelforge.exe').Path }
$TSV  = "$OUT\sweep.tsv"
New-Item -ItemType Directory -Force $OUT | Out-Null
if (-not (Test-Path $TSV)) {
  "label`tenv`tG3_p05`tG5_minGB`tG5_spread`tG6_RGB`tG6_L`tG3`tG5`tG6" | Out-File -Encoding utf8 $TSV
}

foreach ($line in (Get-Content $PLAN | Where-Object { $_ -and -not $_.StartsWith('#') })) {
  $parts = $line -split '\|', 2
  $label = $parts[0].Trim()
  $envs  = if ($parts.Count -gt 1) { $parts[1].Trim() } else { '' }

  $png = (Resolve-Path $OUT).Path + "\$label.png"
  Remove-Item $png -ErrorAction SilentlyContinue

  # apply env
  $saved = @{}
  $applied = @()
  if ($envs) {
    foreach ($kv in ($envs -split ';')) {
      if (-not $kv.Trim()) { continue }
      $k, $v = ($kv -split '=', 2)
      $k = $k.Trim(); $v = $v.Trim()
      $saved[$k] = [Environment]::GetEnvironmentVariable($k)
      [Environment]::SetEnvironmentVariable($k, $v)
      $applied += $k
    }
  }
  [Environment]::SetEnvironmentVariable('VOXELFORGE_PLAY', '1')
  [Environment]::SetEnvironmentVariable('VOXELFORGE_SHOT', $png)

  $p = Start-Process -FilePath $EXE -NoNewWindow -Wait -PassThru `
       -RedirectStandardOutput "$OUT\$label.out.log" -RedirectStandardError "$OUT\$label.err.log"

  [Environment]::SetEnvironmentVariable('VOXELFORGE_PLAY', $null)
  [Environment]::SetEnvironmentVariable('VOXELFORGE_SHOT', $null)
  foreach ($k in $applied) { [Environment]::SetEnvironmentVariable($k, $saved[$k]) }

  if (-not (Test-Path $png)) {
    "$label`t$envs`tMISS(exit=$($p.ExitCode))" | Out-File -Encoding utf8 -Append $TSV
    continue
  }

  # de-HUD (mandatory per rubric) then grade
  $nh = (Resolve-Path $OUT).Path + "\$label-nohud2.png"
  Start-Process -FilePath 'python' -ArgumentList "scripts\_flamingo_dehud2.py","$png" `
    -NoNewWindow -Wait -RedirectStandardOutput "$OUT\$label.dehud.log" -RedirectStandardError "$OUT\$label.dehud.err"
  Start-Process -FilePath 'python' -ArgumentList "scripts\_flamingo_g6_row.py","$nh","$label","$envs","$TSV" `
    -NoNewWindow -Wait -RedirectStandardOutput "$OUT\$label.grade.log" -RedirectStandardError "$OUT\$label.grade.err"
}

"=== SWEEP DONE $(Get-Date -Format o) ===" | Out-File -Encoding utf8 -Append $TSV
