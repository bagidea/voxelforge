# Poppy - the null pair. `after` shot TWICE under byte-identical env.
# The campfire, the foliage and the husk all animate, so two identical runs
# already disagree. That disagreement is this session's capture floor, and the
# A/B delta is worth nothing until it is measured against it.
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root
$exe = Join-Path $root "_matmaps_ab_exe.exe"

$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
$env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"
Remove-Item Env:VOXELFORGE_MAT_MAPS, Env:VOXELFORGE_ATLAS_DIR, Env:VOXELFORGE_LOOK_SUN, `
            Env:VOXELFORGE_LOOK_LIGHT, Env:VOXELFORGE_LOOK_EXPOSURE -ErrorAction SilentlyContinue

foreach ($t in @("nullA", "nullB")) {
  $png = Join-Path $root "_matmaps_$t.png"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  $env:VOXELFORGE_SHOT = $png
  & $exe --play *> (Join-Path $root "_matmaps_$t.log")
  $sz = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  Write-Output ("[{0}] {1} png={2}B" -f (Get-Date -Format HH:mm:ss), $t, $sz)
}
Write-Output "NULL DONE"
