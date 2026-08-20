# ===========================================================================
# Sky A/B, TWO binaries, ZERO levers.
#
#   before.png  0e54d85 (e4ceb80's parent) - target-poppy\release\voxelforge_SKY_BEFORE_e4ceb80parent.exe
#   after.png   e4ceb80 built from its own detached worktree
#
# The only thing that differs is the commit compiled into the exe. Same camera,
# same hour, same map, same quality, and the SAME asset tree is pushed next to
# both exes first (build.rs has silently skipped shaders/ + textures/ before, and
# the main working tree carries other lanes' uncommitted atlas/texture edits --
# neither is allowed to leak into a look A/B).
#
# The run REFUSES rather than shooting a meaningless pair when:
#   - either exe is missing
#   - BEFORE contains VOXELFORGE_LOOK_FOGCOOL (a string e4ceb80 introduced) or
#     AFTER does not -- i.e. the pair does not straddle the commit
# A pair that comes out md5-identical is reported as SUSPECT, never as a pass.
#
# USAGE  powershell -File scripts/_poppy_skyab_shoot.ps1
# ===========================================================================
param(
  [string]$BeforeExe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-poppy\release\voxelforge_SKY_BEFORE_e4ceb80parent.exe",
  [string]$AfterExe  = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-poppy-sky\release\voxelforge.exe",
  [string]$AssetSrc  = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_sky_wt\assets",
  [string]$Out       = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_skyab"
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

function Fail([string]$msg) { Write-Output "REFUSED  $msg"; exit 2 }

foreach ($p in @($BeforeExe, $AfterExe)) { if (-not (Test-Path $p)) { Fail "$p not found" } }
if (-not (Test-Path $AssetSrc)) { Fail "$AssetSrc not found" }
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# ---- straddle proof: scan the bytes, never the mtime ----------------------
function Count-Ascii([string]$file, [string]$needle) {
  $bytes = [IO.File]::ReadAllBytes($file)
  $text  = [Text.Encoding]::GetEncoding(28591).GetString($bytes)
  return ([regex]::Matches($text, [regex]::Escape($needle))).Count
}
$bFog = Count-Ascii $BeforeExe "VOXELFORGE_LOOK_FOGCOOL"
$aFog = Count-Ascii $AfterExe  "VOXELFORGE_LOOK_FOGCOOL"
$bSun = Count-Ascii $BeforeExe "VOXELFORGE_LOOK_SUN"
$aSun = Count-Ascii $AfterExe  "VOXELFORGE_LOOK_SUN"
Write-Output ("straddle  BEFORE FOGCOOL={0} SUN={1}   AFTER FOGCOOL={2} SUN={3}" -f $bFog,$bSun,$aFog,$aSun)
if ($bSun -lt 1 -or $aSun -lt 1) { Fail "one of these is not a world-render exe (LOOK_SUN missing)" }
if ($bFog -ne 0) { Fail "BEFORE already contains FOGCOOL - it is not pre-e4ceb80" }
if ($aFog -lt 1) { Fail "AFTER does not contain FOGCOOL - it is not e4ceb80" }

# ---- one asset tree for both arms ----------------------------------------
foreach ($exe in @($BeforeExe, $AfterExe)) {
  $dst = Join-Path (Split-Path $exe) "assets"
  robocopy $AssetSrc $dst /E /NFL /NDL /NJH /NJS /NP | Out-Null
}
function Tree-Manifest([string]$dir) {
  Get-ChildItem $dir -Recurse -File | Sort-Object FullName |
    ForEach-Object { "{0}|{1}" -f $_.FullName.Substring($dir.Length), $_.Length }
}
$bMan = @(Tree-Manifest (Join-Path (Split-Path $BeforeExe) "assets"))
$aMan = @(Tree-Manifest (Join-Path (Split-Path $AfterExe)  "assets"))
$diff = @(Compare-Object $bMan $aMan)
Write-Output ("assets    before={0} files  after={1} files  differing={2}" -f $bMan.Count, $aMan.Count, $diff.Count)
if ($diff.Count -gt 0) { $diff | Select-Object -First 10 | ForEach-Object { Write-Output ("  asset-diff {0} {1}" -f $_.SideIndicator, $_.InputObject) } }

# ---- shared capture env: identical for both arms, no levers ---------------
$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
$env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"
Remove-Item Env:VOXELFORGE_ATLAS_DIR, Env:VOXELFORGE_LOOK_SUN, `
            Env:VOXELFORGE_LOOK_LIGHT, Env:VOXELFORGE_LOOK_EXPOSURE, `
            Env:VOXELFORGE_LOOK_AMBIENT, Env:VOXELFORGE_LOOK_FILL, `
            Env:VOXELFORGE_LOOK_FOGCOOL, Env:VOXELFORGE_LOOK_SKYGRAD, `
            Env:VOXELFORGE_SKY_GAIN, Env:VOXELFORGE_SKY_CURVE `
            -ErrorAction SilentlyContinue

function Shoot([string]$tag, [string]$exe) {
  $png = Join-Path $Out "$tag.png"
  $log = Join-Path $Out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  $env:VOXELFORGE_SHOT = $png

  & $exe --play *> $log
  $ec = $LASTEXITCODE

  $err   = @(Select-String -Path $log -Pattern '^error').Count
  $dome  = @(Select-String -Path $log -Pattern 'sky-dome (PAINTED|spawned)').Count
  $size  = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $md5   = if ($size -gt 0) { (Get-FileHash $png -Algorithm MD5).Hash } else { "-" }
  $exeMd5= (Get-FileHash $exe -Algorithm MD5).Hash

  $note = ""
  if ($size -lt 1) { $note += "  !!NO-FRAME" }
  if ($err -gt 0)  { $note += "  !!ERRORS=$err" }
  if ($dome -lt 1) { $note += "  !!NO-DOME" }
  Write-Output ("{0,-7} ec={1} {2,8} bytes  dome={3}  png md5 {4}  exe md5 {5}{6}" -f `
    $tag, $ec, $size, $dome, $md5, $exeMd5, $note)
  return $md5
}

Write-Output ""
$bMd5 = Shoot "before" $BeforeExe
$aMd5 = Shoot "after"  $AfterExe
Write-Output ""
if ($bMd5 -eq "-" -or $aMd5 -eq "-") { Write-Output "VERDICT  UNMEASURABLE - an arm produced no frame"; exit 3 }
if ($bMd5 -eq $aMd5) { Write-Output "VERDICT  SUSPECT - both arms are byte-identical PNGs"; exit 4 }
Write-Output "VERDICT  pair OK - two distinct frames from two straddling binaries"
Write-Output "frames in $Out"
