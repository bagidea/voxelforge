# _yamamoto_shadowbias_sweep.ps1 -- ONE build, THREE VOXELFORGE_LOOK_SHADOW_BIAS
# arms, off the SAME exe, at TWO camera framings -- proves whether the
# depth/normal bias fix from commit f1d6dc2 (client/src/look.rs) actually moves
# the shadow off the base of a wall/tree without inviting acne back in.
#
# WHY THREE ARMS, NOT TWO. A plain before/after only shows "moved". It cannot
# show whether it moved TOO FAR and let the low-frequency raking key acne on
# open flat ground (normal_bias 1.8 -> 0.3 is a big cut -- see look.rs's own
# doc comment on SHADOW_NORMAL_BIAS for why that risk is real, not decorative).
# The "mid" arm is a deliberately conservative third point so a human looking
# at the three plates side by side can see the whole slope, not just the ends.
#
#   before  0.02,1.8   Bevy's own engine defaults -- what every frame this lane
#                       has ever shipped actually ran on, per f1d6dc2's commit msg.
#   after   0.01,0.3   the constants shipped in look.rs by f1d6dc2.
#   mid     0.015,0.8  halfway point, safety margin if "after" turns out to acne.
#
# WHY TWO CAMERAS. The brief named three subjects -- wall base, tree trunk
# base, open flat ground -- and no single proven camera in this repo's shot
# history frames all three at once. Rather than invent a blind, unverified
# VOXELFORGE_CINE string (which would burn the one queued build on a shot
# that might not even see geometry), this script reuses TWO cameras that are
# already known-good, straight out of _poppy_beauty/final/SHOTS.md and
# scripts/_poppy_shotset.ps1 (their CINE strings are copied from the `CINE eye
# ... aim ...` line the engine itself printed on a real run, not retyped from
# memory):
#
#   wallbase  eye 54,12,44 -> aim 26,3,20, VOXELFORGE_LOOK_SUN=6,140,9000
#             = "s4-vista-raking": eye-level across the ruin field, sun
#             dropped to ~6deg (harder raking test than the 17-22deg the
#             commit message reasons from), open ground + standing walls
#             in one frame.
#   closeup   eye 34.2,2.9,28.8 -> aim 32.5,2.0,32.4, default sun (Hour::GOLDEN)
#             = "s2-hero-medium": eye-level, close range, near the village
#             centre where edhari.json's wood/leaves density is highest --
#             the better bet for a tree-trunk base filling real frame area.
#
# Neither camera has been probed FOR this bias fix specifically -- if the
# closeup frame does not happen to catch a trunk base in view, say so in the
# report and let a follow-up sweep add a purpose-probed camera. Do not
# pretend a miss is a hit.
#
# BUILD LANE: this script never invokes cargo. It only reads an exe that
# someone else already built (queued through poppy per the build-lock rule --
# see notes.md). -Exe overrides the default path if poppy's build lands
# somewhere else.
#
# ACNE CHECK IS MANUAL. A ray-march/bias artifact reads as fine parallel
# banding on the OPEN FLAT GROUND plane, not as a single obviously-wrong
# pixel -- no script in this repo can grade that reliably (see the sky-sweep
# script's own precedent for why: "no single-frame ... detector separates our
# frames -- A/B only"). Open the three PNGs per camera side by side and look
# at the open-ground region specifically before calling "after" a win.
#
# USAGE
#   powershell -File scripts/_yamamoto_shadowbias_sweep.ps1
#   powershell -File scripts/_yamamoto_shadowbias_sweep.ps1 -Exe target-flamingo\release\voxelforge.exe
#   powershell -File scripts/_yamamoto_shadowbias_sweep.ps1 -DryRun
param(
    [string]$Exe    = "target-poppy\release\voxelforge.exe",
    [string]$OutDir = "_yamamoto_shadowbias",
    [switch]$DryRun
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

if (-not (Test-Path $Exe)) {
    Write-Output "REFUSED  $Exe not found -- build not landed yet, or wrong -Exe path"
    exit 2
}

# Binary-lever gate: this exe must carry the new lever BY NAME or a sweep of
# it measures nothing (the atlas-tilepx and sky-lever lessons: a stale exe
# still runs, still writes a PNG, and still lies about what it tested).
$bytes = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($Exe))
$missing = @()
foreach ($s in @("VOXELFORGE_LOOK_SHADOW_BIAS", "SHADOW_DEPTH_BIAS", "SHADOW_NORMAL_BIAS")) {
    if ($bytes.IndexOf($s) -lt 0) { $missing += $s }
}
if ($missing.Count -gt 0) {
    Write-Output ("REFUSED  exe missing: {0} -- this is not a build off f1d6dc2 or later" -f ($missing -join ", "))
    exit 2
}
$exeItem = Get-Item $Exe
$exeHash = (Get-FileHash $Exe -Algorithm SHA256).Hash
Write-Output ("binary-lever gate: PASS   mtime {0}   sha256 {1}" -f $exeItem.LastWriteTime, $exeHash)

if ($DryRun) {
    Write-Output "DRY RUN -- gate checked only, no exe launched, no files written"
    exit 0
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$gitHead = (git rev-parse HEAD).Trim()
$manifest = Join-Path $OutDir "manifest.txt"
"exe        $Exe" | Out-File -Encoding utf8 $manifest
"exe mtime  $($exeItem.LastWriteTime)" | Out-File -Encoding utf8 -Append $manifest
"exe sha256 $exeHash" | Out-File -Encoding utf8 -Append $manifest
"git HEAD   $gitHead (script authored against f1d6dc2)" | Out-File -Encoding utf8 -Append $manifest
"" | Out-File -Encoding utf8 -Append $manifest

$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
Remove-Item Env:VOXELFORGE_ATLAS_DIR, Env:VOXELFORGE_LOOK_LIGHT, `
            Env:VOXELFORGE_LOOK_EXPOSURE, Env:VOXELFORGE_MAT_MAPS, `
            Env:VOXELFORGE_LOOK_ATMOS, Env:VOXELFORGE_LOOK_SUN `
            -ErrorAction SilentlyContinue

# tag -> depth,normal bias (see header for why these three points)
$arms = @(
    @{ tag = "before"; bias = "0.02,1.8";  note = "Bevy engine default (unset before f1d6dc2)" },
    @{ tag = "after";  bias = "0.01,0.3";  note = "shipped in f1d6dc2" },
    @{ tag = "mid";    bias = "0.015,0.8"; note = "safety midpoint" }
)

# cam -> cine string + per-cam env delta (LOOK_SUN etc.) + what it is proving
$cams = @(
    @{ tag = "wallbase"; cine = "54,12,44, 54,12,44, 26,3,20, 4";
       env = @{ VOXELFORGE_LOOK_SUN = "6,140,9000" };
       note = "s4-vista-raking cam, ~6deg sun -- ruin walls + open ground" }
    @{ tag = "closeup";  cine = "34.2,2.9,28.8, 34.2,2.9,28.8, 32.5,2.0,32.4, 4";
       env = @{};
       note = "s2-hero-medium cam, default Hour::GOLDEN sun -- village centre, best tree-trunk bet" }
)

$results = @()
foreach ($cam in $cams) {
    foreach ($e in $cam.env.Keys) { Set-Item -Path "Env:$e" -Value $cam.env[$e] }
    if ($cam.env.Keys -notcontains "VOXELFORGE_LOOK_SUN") {
        Remove-Item Env:VOXELFORGE_LOOK_SUN -ErrorAction SilentlyContinue
    }
    $env:VOXELFORGE_CINE = $cam.cine

    foreach ($a in $arms) {
        $tag = "{0}_{1}" -f $cam.tag, $a.tag
        $png = Join-Path $OutDir ("shadowbias_{0}.png" -f $tag)
        $log = Join-Path $OutDir ("shadowbias_{0}.log" -f $tag)
        Remove-Item $png -Force -ErrorAction SilentlyContinue

        $env:VOXELFORGE_LOOK_SHADOW_BIAS = $a.bias
        $env:VOXELFORGE_SHOT             = $png

        & $Exe --play *> $log

        $size    = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
        $biasLine = @(Select-String -Path $log -Pattern 'bias=(\S+)') | Select-Object -Last 1
        $biasSeen = if ($biasLine) { $biasLine.Matches[0].Groups[1].Value } else { "MISSING" }
        $mapLine  = @(Select-String -Path $log -Pattern 'MAP_LOAD ok') | Select-Object -Last 1

        $note = ""
        if ($size -lt 1)          { $note += "  !!NO-FRAME" }
        if ($biasSeen -eq "MISSING") { $note += "  !!NO-BIAS-LINE" }
        if (-not $mapLine)        { $note += "  !!NO-MAP-LOAD" }

        $row = [pscustomobject]@{
            Tag  = $tag
            Bias = $a.bias
            Seen = $biasSeen
            PNG  = "$size B"
            Note = $note
        }
        $results += $row
        Write-Output ("[{0,-16}] want={1,-9} saw={2,-9} png={3,-9}B{4}" -f `
                      $tag, $a.bias, $biasSeen, $size, $note)
        ("{0}  want-bias={1}  saw-bias={2}  png={3}B{4}" -f $tag, $a.bias, $biasSeen, $size, $note) |
            Out-File -Encoding utf8 -Append $manifest
    }
}

Write-Output "SWEEP DONE"
Write-Output ("manifest: {0}" -f $manifest)
Write-Output "NEXT: open the 3 PNGs per camera side by side. Check (1) the shadow sits"
Write-Output "tighter against the wall/trunk base from before -> after -> and (2) the open"
Write-Output "flat ground in 'after' has no new banding/acne that 'before' did not have."
