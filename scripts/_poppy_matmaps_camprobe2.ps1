# Poppy - second camera pass. Pose C won on material coverage (brick, plaster,
# glass, log, leaves, sand, grass, red_sand, lamp all in frame) but wastes the
# top third on the void above the horizon. These three pitch further down.
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root
$exe = Join-Path $root "_matmaps_ab_exe.exe"
$out = Join-Path $root "_poppy_matmaps\camprobe"

$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
Remove-Item Env:VOXELFORGE_MAT_MAPS, Env:VOXELFORGE_ATLAS_DIR, `
            Env:VOXELFORGE_LOOK_SUN, Env:VOXELFORGE_LOOK_LIGHT, `
            Env:VOXELFORGE_LOOK_EXPOSURE -ErrorAction SilentlyContinue

$poses = @(
  @{ n = "E"; cam = "44,18,40, 44,18,40, 29,3,20, 1" },
  @{ n = "F"; cam = "41,15,37, 41,15,37, 29,3,20, 1" },
  @{ n = "G"; cam = "38,12,34, 38,12,34, 29,3.5,20, 1" }
)
foreach ($p in $poses) {
  $env:VOXELFORGE_CINE = $p.cam
  $png = Join-Path $out ("pose_" + $p.n + ".png")
  $log = Join-Path $out ("pose_" + $p.n + ".log")
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  $env:VOXELFORGE_SHOT = $png
  & $exe --play *> $log
  $sz = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  Write-Output ("[{0}] pose {1}  png={2}B" -f (Get-Date -Format HH:mm:ss), $p.n, $sz)
}
Write-Output "PROBE2 DONE"
