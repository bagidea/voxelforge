# Poppy - camera probe for the MAT_MAPS A/B plate.
# beach_dusk.json is the only map carrying all 12 file-art block kinds at once
# (grass sand wood dirt leaves stone limestone brick red_sand snow glass lamp),
# so the frame has to come from there. This shoots a few poses so I can LOOK at
# them before spending the real pair on a camera that misses half the materials.
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root
$exe = Join-Path $root "_matmaps_ab_exe.exe"
$out = Join-Path $root "_poppy_matmaps\camprobe"
New-Item -ItemType Directory -Force -Path $out | Out-Null

$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
Remove-Item Env:VOXELFORGE_MAT_MAPS  -ErrorAction SilentlyContinue
Remove-Item Env:VOXELFORGE_ATLAS_DIR -ErrorAction SilentlyContinue

$poses = @(
  @{ n = "A"; cam = "52,20,46, 52,20,46, 28,3,22, 1";   sun = "1" },
  @{ n = "B"; cam = "44,13,40, 44,13,40, 29,4,20, 1";   sun = "1" },
  @{ n = "C"; cam = "44,13,40, 44,13,40, 29,4,20, 1";   sun = "0" },
  @{ n = "D"; cam = "38,9,36, 38,9,36, 29,4.5,20, 1";   sun = "0" }
)

foreach ($p in $poses) {
  $env:VOXELFORGE_CINE = $p.cam
  if ($p.sun -eq "1") {
    $env:VOXELFORGE_LOOK_SUN      = "66,205,20000"
    $env:VOXELFORGE_LOOK_LIGHT    = "1.00,0.98,0.93,0.84,0.88,1.00"
    $env:VOXELFORGE_LOOK_EXPOSURE = "10.6"
  } else {
    Remove-Item Env:VOXELFORGE_LOOK_SUN, Env:VOXELFORGE_LOOK_LIGHT, Env:VOXELFORGE_LOOK_EXPOSURE -ErrorAction SilentlyContinue
  }
  $png = Join-Path $out ("pose_" + $p.n + ".png")
  $log = Join-Path $out ("pose_" + $p.n + ".log")
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  $env:VOXELFORGE_SHOT = $png
  & $exe --play *> $log
  $px  = @(Select-String -Path $log -Pattern 'BLOCK_ART file-backed.*tile_px=64').Count
  $pbr = @(Select-String -Path $log -Pattern '^BLOCK_PBR authored').Count
  $sz  = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  Write-Output ("[{0}] pose {1}  png={2}B tile_px64={3} authored_pbr={4}" -f (Get-Date -Format HH:mm:ss), $p.n, $sz, $px, $pbr)
}
Write-Output "PROBE DONE"
