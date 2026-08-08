# Flamingo G7 — sweep the aerial-perspective / AO / vegetation levers on ONE build.
#
# Same shape as the G6 sweep (that one worked): read a plan file, shoot the same
# pinned frame once per row through look.rs's env hooks, de-HUD it, and append a
# numeric row carrying BOTH the locked gates and the new G7 axes. One relink,
# many data points — the only way to tune three coupled levers without spending
# a 20-minute link per candidate.
#
# Plan line format:  label|KEY=value;KEY=value        (env may be empty)
#
# Start-Process -Redirect* for every native call: PS 5.1 wraps native stderr in
# ErrorRecords and writes UTF-16, which makes the logs unmatchable otherwise.
$ErrorActionPreference = 'Continue'
Set-Location (Split-Path -Parent $PSScriptRoot)

$PLAN = if ($args.Count -ge 1) { $args[0] } else { '_flamingo_g7\plan.txt' }
$OUT  = if ($args.Count -ge 2) { $args[1] } else { '_flamingo_g7' }
# The lane's OWN target dir, never the shared `target\` — G7 is under an explicit
# "target-flamingo only" instruction. `perf` profile: identical pixels, no fat-LTO link.
$EXE  = if ($env:FL_EXE) { (Resolve-Path $env:FL_EXE).Path }
        else { (Resolve-Path 'target-flamingo\perf\voxelforge.exe').Path }
$TSV  = "$OUT\g7.tsv"
New-Item -ItemType Directory -Force $OUT | Out-Null
if (-not (Test-Path $TSV)) {
  "label`tenv`tG3_p05`tG5_spread`tG6_RB`tG6_L`tG3`tG5`tG6`tp95`tmicro`tvegSat`tvegHue`tC" |
    Out-File -Encoding utf8 $TSV
}
Write-Host "EXE = $EXE"

foreach ($line in (Get-Content $PLAN | Where-Object { $_ -and -not $_.StartsWith('#') })) {
  $parts = $line -split '\|', 2
  $label = $parts[0].Trim()
  $envs  = if ($parts.Count -gt 1) { $parts[1].Trim() } else { '' }

  $png = (Resolve-Path $OUT).Path + "\$label.png"
  Remove-Item $png -ErrorAction SilentlyContinue

  $saved = @{}; $applied = @()
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
    Write-Host "$label MISS"
    continue
  }

  # de-HUD is MANDATORY per the rubric — a `--play` frame burns HUD text that the
  # graders otherwise read as scene pixels.
  $nh = (Resolve-Path $OUT).Path + "\$label-nohud2.png"
  Start-Process -FilePath 'python' -ArgumentList "scripts\_flamingo_dehud2.py","$png" `
    -NoNewWindow -Wait -RedirectStandardOutput "$OUT\$label.dehud.log" -RedirectStandardError "$OUT\$label.dehud.err"
  Start-Process -FilePath 'python' -ArgumentList "scripts\_flamingo_g7_row.py","$nh","$label","$envs","$TSV" `
    -NoNewWindow -Wait -RedirectStandardOutput "$OUT\$label.grade.log" -RedirectStandardError "$OUT\$label.grade.err"
  Write-Host (Get-Content "$OUT\$label.grade.log" -Raw)
}

"=== SWEEP DONE $(Get-Date -Format o) ===" | Out-File -Encoding utf8 -Append $TSV
