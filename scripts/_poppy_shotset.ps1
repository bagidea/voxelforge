# _poppy_shotset.ps1 -- ONE command shoots the whole canonical plate set off ONE exe,
# de-HUDs every frame, builds the before/after pair dirs regrade.py eats verbatim,
# and writes a side-by-side sheet + manifest.
#
# ASCII ONLY (Windows PowerShell 5.1 reads .ps1 as ANSI; a UTF-8 dash inside a string
# literal is a parse error). Keep this file ASCII.
#
#   # 1. baseline, off the frozen 19:53 exe -- run once, or after any map/content edit
#   powershell -File scripts/_poppy_shotset.ps1 -BeforeSide -NoRegrade
#   # 2. the real pass, the moment Rose says green. Pairs + sheets + regrades on its own.
#   powershell -File scripts/_poppy_shotset.ps1 -Exe target-flamingo/release/voxelforge.exe
#
# WHAT IT SHOOTS (the canonical set; -Only takes a subset by key)
#   gate3-boot     VOXELFORGE_PLAY=1                     1280x720
#   gate3-combat   VOXELFORGE_COMBAT_DEMO=1              1280x720
#   gate3-walk     VOXELFORGE_PLAY_DEMO=1                1280x720
#   grade-vista    VOXELFORGE_LOOK_CAM=35,-18,26 ultra   1280x720
#   s1-vista       CINE 32,20,54 -> 33,5,20              1600x900
#   hero           CINE 34.2,2.9,28.8 -> 32.5,2.0,32.4   1600x900
#   s4-raking      CINE 54,12,44 + LOOK_SUN=6,140,9000   1600x900
#   s3-clash       VOXELFORGE_ANIM_POSE=clash            1600x900
#
# Every camera line above is copied from the `CINE eye ... aim ...` the engine printed
# into the matching .log of the 19:53 pass, recorded in _poppy_beauty/final/SHOTS.md --
# not retyped from memory. The gate3 / grade-vista env lines are the ones their published
# assets were shot with (scripts/gate3_shoot.sh, scripts/verify_baked_grade.sh). Both
# sides run through this one table, so a pair changes the binary and nothing else.
#
# BUILD LANE: this script never invokes cargo and never writes inside a target dir. It
# takes a FROZEN COPY of the exe first, so it cannot hold a file lock on the linker's
# live output while Rose's build is running.
#
# STALE / OLD-BINARY GATE: refuses to shoot with the 19:53 binary (its SHA-256 is pinned
# below) and refuses an exe older than the newest client/src/*.rs. -AllowOldExe /
# -AllowStale override, and the override is stamped into every runlog. -DryRun downgrades
# both to warnings -- a dry run never launches the exe, so there is nothing to mis-shoot.
param(
    [string]$Exe       = "target-flamingo\release\voxelforge.exe",
    [string]$OutDir    = "_poppy_shotset",
    [string[]]$Only    = @(),
    [string]$ClashEnv  = "",        # "K=V;K=V" merged into s3-clash's env (override only)
    [string]$ExtraEnv  = "",        # "K=V;K=V" merged into EVERY selected plate's env
    [switch]$BeforeSide,            # shoot the BASELINE set (frozen 19:53 exe + BeforeEnv)
    [switch]$AllowMissingBefore,    # shoot AFTER frames that cannot be paired, on purpose
    [switch]$DryRun,
    [switch]$AllowOldExe,
    [switch]$AllowStale,
    [switch]$NoRegrade,
    [int]$ShotTimeoutMs = 45000
)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $Root
try {

# The 19:53:36 target-flamingo build the previous beauty pass shot on. Every BEFORE
# frame in _poppy_beauty/final came off this exe, so re-shooting the AFTER side with it
# would produce a pair with zero variables changed.
$OLD_EXE_SHA = "BF344EE77A050BA486391D00DF68B8F9C6561F9ABC3A3FDD17558123F5A4D3A3"
# A copy of that exe already frozen OUTSIDE every target dir. -BeforeSide runs this one,
# so re-shooting a baseline can never read target-flamingo out from under a live link.
$OLD_EXE_FROZEN = "_rose_look_beauty\vfprobe_grade-vista-new.exe"

# -BeforeSide is the baseline pass: the old binary is the POINT, not a mistake, and its
# output belongs in the dir every plate's Before field points at.
if ($BeforeSide) {
    if (-not $PSBoundParameters.ContainsKey('Exe'))    { $Exe = $OLD_EXE_FROZEN }
    if (-not $PSBoundParameters.ContainsKey('OutDir')) { $OutDir = "_poppy_shotset_before" }
    $AllowOldExe = $true
    $AllowStale  = $true
}

# ---------------------------------------------------------------------------
# shot table
# ---------------------------------------------------------------------------
# Key       key used for the regrade pair (before/<Key>-nohud2.png <-> after/<Key>-nohud2.png)
# CliArgs   argv handed to the exe ("" = none)
# Env       VOXELFORGE_* knobs for this plate (NOHUD/SHOT are added by the runner)
# Cine      VOXELFORGE_CINE string ("" = no scripted camera; gameplay/boom framing)
# W,H       client size forced before the 3.2s shot timer -- DECLARED, never inherited.
#           "the engine default" is not a constant: the gate3 baselines are 1280x720 but
#           Rose's vista off the SAME 19:53 exe came out 2560x1440
#           (_rose_look_beauty/grade-vista-new.png). A silent size change moves the
#           resolution-sensitive axes (micro-contrast, edge energy, penumbra px) on its
#           own, which reads as a look change that never happened.
# Before    the baseline frame this plate is paired against -- ALWAYS a frame this same
#           script shot from the frozen 19:53 exe into -OutDir _poppy_shotset_before
# BeforeRaw $true if that baseline still has its HUD band (needs dehud2 before pairing)
# BeforeEnv env deltas applied ONLY on the baseline side, for a knob the old exe does not
#           have (see s3-clash); absent means both sides run the identical Env
# Ref       the published asset this plate corresponds to -- recorded for context, NOT
#           graded (see the note below on why none of them can be)
# Expect    stdout markers that MUST appear or the plate fails -- a green exit with a
#           saved PNG still lies if the thing the plate exists to show never spawned.
#           Only set where a real log proves the marker fires for that plate: s1/hero/
#           s4/s3 (_poppy_beauty/final/*.log) and grade-vista
#           (_poppy_shotset_before/raw/grade-vista.log) all print both actors. The three
#           gate3 plates have no log on disk, so they get no hard Expect -- a guessed
#           marker that never fires would block the whole GO run. They still print the
#           RIGS/capsule line, which is the warning without the false-fail.
#
# WHY EVERY BASELINE IS RE-SHOT AND NO PUBLISHED ASSET IS PAIRED AGAINST
#   Measured 2026-08-07, same exe (sha BF344EE7...), same camera, s1-vista:
#     run1 vs run2 of THIS script   mean |d| R 0.39 G 0.52 B 0.18   2.6% px moved
#     published 19:53 frame vs run1 mean |d| R 32.1 G 16.0 B  7.6  81.5% px moved
#   So the harness is deterministic and the 32-unit gap is real, not noise. The engine
#   logs say what it is: `MAP_LOAD ok path=maps/edhari.json blocks=8513` in the published
#   frame's log vs `blocks=8838` today. maps/edhari.json gained 325 blocks after those
#   frames were taken. Pairing them with a new binary would bill a CONTENT change as a
#   look change. Same for docs/assets/gate3/* (2026-08-01, older map still).
#   Separately, docs/assets/grade-vista-2026-08-05-nohud2.png -- the before frame Sun's
#   pairing map (scripts/_sun_after_scorecard.sh) uses -- is not even a frame: it is a
#   936x1726 CONTACT SHEET, three stacked renders plus black caption bands and coloured
#   metric text. Grading it grades the captions.
#   The map_blocks count is captured per plate and compared across the pair, so if the
#   map moves again between the two sides the run says so instead of quietly averaging it in.
#
#   Re-make the whole baseline set with:
#     powershell -File scripts/_poppy_shotset.ps1 -BeforeSide -NoRegrade
$SHOTS = @(
    @{ Key='gate3-boot';   CliArgs=''; Cine=''; W=1280; H=720;
       Env=@{ VOXELFORGE_PLAY='1'; VOXELFORGE_LOOK_QUALITY='high' };
       Before='_poppy_shotset_before\after\gate3-boot-nohud2.png'; BeforeRaw=$false;
       Ref='docs/assets/gate3/gate3-after-boot-nohud2.png';
       Note='spawn pose, no input -- the static G3 interior plate' }

    @{ Key='gate3-combat'; CliArgs=''; Cine=''; W=1280; H=720;
       Env=@{ VOXELFORGE_COMBAT_DEMO='1'; VOXELFORGE_LOOK_QUALITY='high' };
       Before='_poppy_shotset_before\after\gate3-combat-nohud2.png'; BeforeRaw=$false;
       Ref='docs/assets/gate3/gate3-after-combat-nohud2.png';
       Note='husk 2.2 blocks ahead, player closing at t=3.2s' }

    @{ Key='gate3-walk';   CliArgs=''; Cine=''; W=1280; H=720;
       Env=@{ VOXELFORGE_PLAY_DEMO='1'; VOXELFORGE_LOOK_QUALITY='high' };
       Before='_poppy_shotset_before\after\gate3-walk-nohud2.png'; BeforeRaw=$false;
       Ref='docs/assets/gate3/gate3-after-walk-nohud2.png';
       Note='play-demo drives real ButtonInput -- mid-stride frame' }

    @{ Key='grade-vista';  CliArgs=''; Cine=''; W=1280; H=720;
       Env=@{ VOXELFORGE_PLAY='1'; VOXELFORGE_LOOK_CAM='35,-18,26'; VOXELFORGE_LOOK_QUALITY='ultra' };
       Before='_poppy_shotset_before\after\grade-vista-nohud2.png'; BeforeRaw=$false;
       Ref='docs/assets/grade-vista-2026-08-05-nohud2.png (CONTACT SHEET -- not gradeable)';
       Expect=@('ANIM_RIG_WEAPON spawn actor=Player');
       Note='canonical vista boom -- the framing every look audit/sweep has used' }

    @{ Key='s1-vista';     CliArgs='--play'; W=1600; H=900;
       Cine='32,20,54, 32,20,54, 33,5,20, 4'; Env=@{};
       Before='_poppy_shotset_before\after\s1-vista-nohud2.png'; BeforeRaw=$false;
       Ref='_poppy_beauty/final/s1-village-wide.png';
       Expect=@('ANIM_RIG_WEAPON spawn actor=Player');
       Note='aerial over the campfire quad -- shipped GOLDEN look, no env override' }

    @{ Key='hero';         CliArgs='--play'; W=1600; H=900;
       Cine='34.2,2.9,28.8, 34.2,2.9,28.8, 32.5,2.0,32.4, 4'; Env=@{};
       Before='_poppy_shotset_before\after\hero-nohud2.png'; BeforeRaw=$false;
       Ref='_poppy_beauty/final/s2-hero-medium.png';
       # THE capsule-proof plate, so it must not fail open. The fix is "dodge_ghost_flash
       # leaves the placeholder alone once the player is Rigged" -- if the player never
       # rigs, the capsule is back and this frame is worthless, but every other check
       # (PNG saved, no panic, exit 0) still passes. Demand the rig marker.
       Expect=@('ANIM_RIG_WEAPON spawn actor=Player');
       Note='hero medium -- the capsule fix is always-on (dodge_ghost_flash skips a Rigged player), no flag to pass' }

    @{ Key='s4-raking';    CliArgs='--play'; W=1600; H=900;
       Cine='54,12,44, 54,12,44, 26,3,20, 4';
       Env=@{ VOXELFORGE_LOOK_SUN='6,140,9000' };
       Before='_poppy_shotset_before\after\s4-raking-nohud2.png'; BeforeRaw=$false;
       Ref='_poppy_beauty/final/s4-vista-raking.png';
       Expect=@('ANIM_RIG_WEAPON spawn actor=Player');
       Note='eye-level ruin field, raking sun via VOXELFORGE_LOOK_SUN' }

    # ON by default since 2026-08-07: Yamamoto shipped the two-actor pose. `clash` parks
    # the player at the light-1 contact frame AND drives the nearest Actor::Husk to its
    # own contact frame via override_husk_beat() -- so the blades meet without pinning
    # the shot to a lucky moment in the demo. `attack` (the old value) only ever posed
    # the player, which is exactly why the BEFORE frame has no contact.
    # -ClashEnv "K=V" still merges extra knobs in on top, for one-off variants.
    #
    # BeforeEnv: the 19:53 exe has no `clash` arm in pose_override(), so passing it there
    # matches no branch and yields None -- the baseline would be "no pose at all", which
    # is not the state anything was ever in. `attack` is what that binary could actually
    # do and what the published s3 frame used, so the pair reads old-lane vs new-lane
    # (one actor posed -> two), which is the change being demonstrated.
    @{ Key='s3-clash';     CliArgs='--combat-demo'; W=1600; H=900;
       Cine='34.9,3.4,31.4, 34.9,3.4,31.4, 33.4,1.7,26.4, 4';
       Env=@{ VOXELFORGE_ANIM_POSE='clash' };
       BeforeEnv=@{ VOXELFORGE_ANIM_POSE='attack' };
       Before='_poppy_shotset_before\after\s3-clash-nohud2.png'; BeforeRaw=$false;
       Ref='_poppy_beauty/final/s3-husk-clash.png';
       # Both rigs must actually exist or "clash" is one actor swinging at nothing.
       Expect=@('ANIM_RIG_WEAPON spawn actor=Player', 'ANIM_RIG_WEAPON spawn actor=Husk');
       Note='husk clash -- baseline is the old attack pose (player only); after is clash (player + husk)' }
)

# ---------------------------------------------------------------------------
# window helper -- the exe hardcodes 1280x720; the beauty plates want 1600x900.
# The process also owns a 16x16 helper window and MainWindowHandle picks whichever
# existed first, so pick the render window by geometry instead.
# ---------------------------------------------------------------------------
if (-not ("W32S" -as [type])) {
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W32S {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowThreadProcessId(IntPtr h, out int pid);
  public static IntPtr FindRenderWindow(int pid, int minW, int minH) {
    IntPtr hit = IntPtr.Zero;
    EnumWindows(delegate(IntPtr h, IntPtr l) {
      int wpid; GetWindowThreadProcessId(h, out wpid);
      if (wpid != pid || !IsWindowVisible(h)) return true;
      RECT c; GetClientRect(h, out c);
      if (c.R - c.L >= minW && c.B - c.T >= minH) { hit = h; return false; }
      return true;
    }, IntPtr.Zero);
    return hit;
  }
}
"@
}

function Clear-VoxelEnv {
    Get-ChildItem env: | Where-Object { $_.Name -like 'VOXELFORGE_*' } |
        ForEach-Object { Remove-Item ("env:" + $_.Name) -ErrorAction SilentlyContinue }
}

function Parse-KvString {
    param([string]$s)
    $h = @{}
    foreach ($kv in ($s -split ";")) {
        if ($kv -match "^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=(.*)$") { $h[$matches[1]] = $matches[2] }
    }
    return $h
}

function Png-Size {
    param([string]$p)
    if (-not (Test-Path $p)) { return "-" }
    $img = [System.Drawing.Image]::FromFile((Resolve-Path $p).Path)
    $d = "$($img.Width)x$($img.Height)"
    $img.Dispose()
    return $d
}

# Run python WITHOUT letting PS 5.1 turn its stderr into a terminating error.
# On 5.1, a native exe writing to stderr under $ErrorActionPreference='Stop' raises
# NativeCommandError even when the exe exits 0 -- and dehud2 emits a Pillow
# DeprecationWarning on every run, so redirecting to a file is not enough. Drop to
# 'Continue' for the call, merge both streams, and judge the run by $LASTEXITCODE.
function Invoke-Py {
    param([string]$Script, [string[]]$PyArgs)
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $out = & python $Script @PyArgs 2>&1 | Out-String
        $rc = $LASTEXITCODE
    } finally { $ErrorActionPreference = $prev }
    return @{ Rc = $rc; Out = $out }
}

# dehud2 crops the top 80px HUD band and inpaints the [E] prompt glyphs. Every BEFORE
# baseline in docs/assets was graded after this pass, so the AFTER side must get it too
# or the band axes do not line up. With NOHUD live the inpaint steps are a no-op and
# only the crop fires -- but running it keeps the geometry identical either way.
function Invoke-Dehud2 {
    param([string]$Png)
    $errFile = "$Png.dehud2.err"
    $r = Invoke-Py "scripts\_flamingo_dehud2.py" @($Png)
    if ($r.Rc -ne 0) {
        $r.Out | Out-File -FilePath $errFile -Encoding utf8
        throw "dehud2 exited $($r.Rc) on $Png -- see $errFile"
    }
    $out = Join-Path ([System.IO.Path]::GetDirectoryName($Png)) `
                     ([System.IO.Path]::GetFileNameWithoutExtension($Png) + "-nohud2.png")
    if (-not (Test-Path $out)) { throw "dehud2 produced no -nohud2.png from $Png" }
    Remove-Item $errFile -ErrorAction SilentlyContinue
    return $out
}

# ---------------------------------------------------------------------------
# 0. resolve the shot list
# ---------------------------------------------------------------------------
$clashKv = if ($ClashEnv -ne "") { Parse-KvString $ClashEnv } else { @{} }
# -ExtraEnv is the "same binary, one knob moved" hook: an env sweep (e.g. an
# VOXELFORGE_LOOK_EXPOSURE ladder) has to be provable as a pure env delta, and the
# only alternative was editing the shot table — which makes the harness itself a
# variable in the very comparison it is measuring. Applied to EVERY selected plate,
# AFTER -ClashEnv, so a ladder value wins over a plate default and shows up verbatim
# in each runlog's shot_env line.
$extraKv = if ($ExtraEnv -ne "") { Parse-KvString $ExtraEnv } else { @{} }

# `powershell -File ... -Only a,b` hands the whole thing over as ONE string (only -Command
# splits it into an array), so every key silently misses and the run dies with
# "no shots selected". Split on comma here so both invocation styles behave the same.
$Only = @($Only | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })

$plan = @()
foreach ($s in $SHOTS) {
    if ($Only.Count -gt 0 -and ($Only -notcontains $s.Key)) { continue }
    if ($s.Key -eq 's3-clash' -and $clashKv.Count -gt 0) {
        foreach ($k in $clashKv.Keys) { $s.Env[$k] = $clashKv[$k] }
    }
    # BeforeEnv exists for knobs the old binary does not have; applying it on the
    # baseline side is what keeps the pair a comparison of two real states.
    if ($BeforeSide -and $s.BeforeEnv) {
        foreach ($k in $s.BeforeEnv.Keys) { $s.Env[$k] = $s.BeforeEnv[$k] }
    }
    foreach ($k in $extraKv.Keys) { $s.Env[$k] = $extraKv[$k] }
    $plan += $s
}
if ($plan.Count -eq 0) { throw "no shots selected (-Only $($Only -join ',')) " }

"=== POPPY SHOTSET -- $($plan.Count) plate(s): $(($plan | ForEach-Object { $_.Key }) -join ', ')"

# ---------------------------------------------------------------------------
# 1. binary gate + provenance
# ---------------------------------------------------------------------------
if (-not (Test-Path $Exe)) { throw "EXE NOT FOUND: $Exe -- is the build green yet?" }
$exeItem = Get-Item $Exe
$sha     = (Get-FileHash -Path $Exe -Algorithm SHA256).Hash
$head    = try { (git rev-parse --short=9 HEAD) } catch { "unknown" }
$newest  = Get-ChildItem "client\src\*.rs" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$isStale = $exeItem.LastWriteTime -lt $newest.LastWriteTime
$isOld   = ($sha -eq $OLD_EXE_SHA)

"EXE      $Exe"
"  mtime  $($exeItem.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss'))"
"  sha256 $sha"
"  commit $head"
"  newest client/src: $($newest.Name) @ $($newest.LastWriteTime.ToString('MM-dd HH:mm'))"
"  stale=$isStale  is-19:53-binary=$isOld"

# The default -Exe is target-flamingo's, but a green build may land in a different lane's
# target dir (Rose's runs in plain target\release). Shooting the stale default while a
# fresh exe sits one directory over is the quiet way to publish a null A/B, so say so.
$freshest = Get-ChildItem -Path . -Filter voxelforge.exe -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\target[^\\]*\\release\\voxelforge\.exe$' } |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($freshest -and $freshest.LastWriteTime -gt $exeItem.LastWriteTime) {
    $relPath = $freshest.FullName.Substring($Root.Length + 1)
    "  HINT   a newer release exe exists: $relPath @ $($freshest.LastWriteTime.ToString('MM-dd HH:mm'))"
    "         shoot it with:  -Exe $relPath"
}

if ($isOld -and -not $AllowOldExe) {
    $msg = @"
REFUSING TO SHOOT: this is the 19:53:36 binary (sha $($sha.Substring(0,12))...).
Every BEFORE frame in _poppy_beauty/final came off it, so the pair would change nothing --
and it predates VOXELFORGE_ANIM_POSE=clash and the always-on capsule fix entirely.
Wait for the green build, or pass -AllowOldExe if you really want a null A/B.
"@
    # A dry run resolves the plan and writes runlogs without ever launching the exe,
    # so the gate has nothing to protect -- warn and keep going.
    if ($DryRun) { Write-Warning ($msg -replace "`r?`n", " ") } else { throw $msg }
}
if ($isStale -and -not $AllowStale) {
    $msg = @"
REFUSING TO SHOOT: exe ($($exeItem.LastWriteTime.ToString('HH:mm'))) is older than
$($newest.Name) ($($newest.LastWriteTime.ToString('HH:mm'))) -- it does not contain the
source on disk. Wait for the link to finish, or pass -AllowStale.
"@
    if ($DryRun) { Write-Warning ($msg -replace "`r?`n", " ") } else { throw $msg }
}

# ---------------------------------------------------------------------------
# 1b. BEFORE-side pre-flight -- run BEFORE a single exe is launched
# ---------------------------------------------------------------------------
# A plate with no baseline produces an AFTER frame that can never be paired, and the
# old accounting let that pass: `$fails` counted shot failures only, so a run could
# print MISS on every plate, write DONE=FINISH_PARTIAL and still exit 0. That is the
# same fail-open the AFTER-side content gate exists to close, so close it here too --
# and close it up front, because discovering it after 8 shots wastes the whole pass.
#
# -BeforeSide is the one legitimate exception: that mode EXISTS to create the baselines,
# so nothing is expected on disk yet. -AllowMissingBefore covers a deliberate partial.
if (-not $BeforeSide) {
    $missingBefore = @($plan | Where-Object { -not (Test-Path (Join-Path $Root $_.Before)) })
    if ($missingBefore.Count -gt 0) {
        "`n=== BEFORE pre-flight -- $($missingBefore.Count) of $($plan.Count) baseline(s) NOT on disk"
        foreach ($m in $missingBefore) { "  MISS $($m.Key) <- $($m.Before)" }
        $msg = @"
REFUSING TO SHOOT: $($missingBefore.Count)/$($plan.Count) BEFORE baselines are missing, so those
plates could only ever produce an unpaired AFTER frame. Note the baseline set lives in
_poppy_shotset_before\, which .gitignore excludes ("_*/") -- it is scratch, not tracked,
and a clean checkout or a cleanup sweep takes it with no warning.
Re-make the whole baseline set first (frozen 19:53 exe, ~4 min):
  powershell -File scripts/_poppy_shotset.ps1 -BeforeSide -NoRegrade
or pass -AllowMissingBefore to shoot the AFTER side unpaired on purpose.
"@
        if ($DryRun -or $AllowMissingBefore) { Write-Warning ($msg -replace "`r?`n", " ") } else { throw $msg }
    } else {
        "`n=== BEFORE pre-flight -- all $($plan.Count) baseline(s) present"
    }
}

# ---------------------------------------------------------------------------
# 2. output tree
# ---------------------------------------------------------------------------
$RawDir    = Join-Path $Root "$OutDir\raw"
$AfterDir  = Join-Path $Root "$OutDir\after"
$BeforeDir = Join-Path $Root "$OutDir\before"
foreach ($d in @($RawDir, $AfterDir, $BeforeDir)) { New-Item -ItemType Directory -Force -Path $d | Out-Null }

# ---------------------------------------------------------------------------
# 3. BEFORE side -- copy each baseline in under its pair key, dehud2 if still raw
# ---------------------------------------------------------------------------
"`n=== BEFORE side"
$beforeOk = 0
foreach ($s in $plan) {
    if ($BeforeSide) { $s.BeforePng = $null; continue }   # this run IS the baseline pass
    $src = Join-Path $Root $s.Before
    if (-not (Test-Path $src)) { "  MISS $($s.Key) <- $($s.Before) (baseline not on disk)"; $s.BeforePng = $null; continue }
    $dst = Join-Path $BeforeDir "$($s.Key)-nohud2.png"
    if ($s.BeforeRaw) {
        # dehud2 writes next to its input, so stage the copy inside before/ first.
        $stage = Join-Path $BeforeDir "$($s.Key).png"
        Copy-Item $src $stage -Force
        $graded = Invoke-Dehud2 $stage
        if ($graded -ne $dst) { Move-Item $graded $dst -Force }
        Remove-Item $stage -ErrorAction SilentlyContinue
    } else {
        Copy-Item $src $dst -Force
    }
    $s.BeforePng = $dst
    $beforeOk++
    "  OK   $($s.Key)-nohud2.png  $(Png-Size $dst)  <- $($s.Before)"
}

# ---------------------------------------------------------------------------
# 4. AFTER side -- frozen copy of the exe, then one plate at a time
# ---------------------------------------------------------------------------
$Probe = Join-Path $Root "$OutDir\vfprobe.exe"
if (-not $DryRun) {
    Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Stop-Process -Force
    Copy-Item $Exe $Probe -Force
}

$metaHead = @"
exe:            $Exe
exe_mtime:      $($exeItem.LastWriteTime.ToString('o'))
exe_sha256:     $sha
commit:         $head
overrides:      AllowOldExe=$AllowOldExe AllowStale=$AllowStale
extra_env:      $(if ($ExtraEnv -ne '') { $ExtraEnv } else { '(none)' })
"@

"`n=== $(if ($BeforeSide) { 'BASELINE plates (frozen 19:53 exe)' } else { 'AFTER side' })"
$fails = 0
foreach ($s in $plan) {
  # The engine grabs the swapchain at a fixed t=3.2s. The window only exists once wgpu
  # has finished init, and on a cold exe that took 3.8s in testing -- i.e. the resize
  # landed AFTER the shutter and the plate came out at the engine's own size instead of
  # the declared one, with nothing in the log saying so. Second launch of the same exe is
  # warm and the window shows up in well under a second, so: gate on the raw PNG's actual
  # size and re-shoot once if it missed. A wrong-size plate is not a smaller plate, it is
  # a different reading on every resolution-sensitive axis.
  for ($attempt = 1; $attempt -le 2; $attempt++) {
    $raw    = Join-Path $RawDir "$($s.Key).png"
    $log    = Join-Path $RawDir "$($s.Key).log"
    $runlog = Join-Path $RawDir "$($s.Key).runlog"

    $shotEnv = @{}
    foreach ($k in $s.Env.Keys) { $shotEnv[$k] = $s.Env[$k] }
    $shotEnv['VOXELFORGE_NOHUD'] = '1'
    $shotEnv['VOXELFORGE_SHOT']  = $raw
    if ($s.Cine -ne '') {
        $shotEnv['VOXELFORGE_CINE'] = $s.Cine
        # Locked-off compositions, not dollies: park at the opening pose from frame 0
        # instead of burning a settle window the shot timer would have to outlast.
        $shotEnv['VOXELFORGE_CINE_START'] = '0'
    }
    $envLine = (($shotEnv.Keys | Sort-Object | ForEach-Object { "$_=$($shotEnv[$_])" }) -join ' ')

    "`n--- $($s.Key)  argv='$($s.CliArgs)'  client=$($s.W)x$($s.H)$(if ($attempt -gt 1) { "  [re-shoot $attempt/2]" })"
    "    $envLine"

    if ($DryRun) {
        "$metaHead`nplate:          $($s.Key)`nargv:           $($s.CliArgs)`nshot_env:       $envLine`n---`n[DryRun -- exe not launched]" |
            Out-File -FilePath $runlog -Encoding utf8
        $s.AfterPng = $null
        "    DRYRUN ok (runlog written, no exe launched)"
        break
    }

    Clear-VoxelEnv
    foreach ($k in $shotEnv.Keys) { Set-Item -Path ("env:" + $k) -Value $shotEnv[$k] }
    Remove-Item $raw -ErrorAction SilentlyContinue

    $spArgs = @{ FilePath = $Probe; PassThru = $true; RedirectStandardOutput = $log; RedirectStandardError = "$log.err" }
    if ($s.CliArgs -ne '') { $spArgs['ArgumentList'] = $s.CliArgs }
    $proc = Start-Process @spArgs

    # The resize has to land before the engine's t=3.2s shutter. It usually does by a
    # wide margin, but a cold exe took 3.8s once -- so the timestamp is printed and the
    # PNG's real size is gated below rather than assumed.
    $hwnd = [IntPtr]::Zero
    $deadline = (Get-Date).AddSeconds(25)
    while ((Get-Date) -lt $deadline) {
        if ($proc.HasExited) { break }
        $hwnd = [W32S]::FindRenderWindow($proc.Id, 320, 200)
        if ($hwnd -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 20
    }
    if ($hwnd -ne [IntPtr]::Zero) {
        $wr = New-Object W32S+RECT; $c0 = New-Object W32S+RECT
        [W32S]::GetWindowRect($hwnd, [ref]$wr) | Out-Null
        [W32S]::GetClientRect($hwnd, [ref]$c0) | Out-Null
        $chromeW = ($wr.R - $wr.L) - ($c0.R - $c0.L)
        $chromeH = ($wr.B - $wr.T) - ($c0.B - $c0.T)
        $scr = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
        $wantW = [Math]::Min($s.W + $chromeW, $scr.Width)
        $wantH = [Math]::Min($s.H + $chromeH, $scr.Height)
        [W32S]::SetWindowPos($hwnd, [IntPtr](-1), 0, 0, $wantW, $wantH, 0) | Out-Null
        # t is PROCESS time; the engine's 3.2s shutter is on the Bevy clock, which only
        # starts once the window exists -- so a t of 3-4s here is normal, not late. The
        # only trustworthy answer is the PNG's own size, gated below.
        $t = [math]::Round(((Get-Date) - $proc.StartTime).TotalSeconds, 2)
        "    WINDOW resized at t=${t}s (process clock) want=${wantW}x${wantH}"
    } else {
        "    WINDOW not found -- the plate keeps whatever size the engine chose"
    }

    $timedOut = -not $proc.WaitForExit($ShotTimeoutMs)
    if ($timedOut) { $proc | Stop-Process -Force; "    TIMEOUT after ${ShotTimeoutMs}ms -- killed" }
    Start-Sleep -Milliseconds 300
    Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Stop-Process -Force
    Clear-VoxelEnv

    $stdout = if (Test-Path $log) { Get-Content $log -Raw } else { "" }
    $stderr = if (Test-Path "$log.err") { Get-Content "$log.err" -Raw } else { "" }
    "$metaHead`nplate:          $($s.Key)`nargv:           $($s.CliArgs)`nshot_env:       $envLine`nexit:           $($proc.ExitCode)  timed_out=$timedOut`n---`n$stdout`n--- stderr ---`n$stderr" |
        Out-File -FilePath $runlog -Encoding utf8

    # Mechanical gate, same rule the gate3 shooters use: PNG on disk >= 2 KiB,
    # "SHOT saved" in the log, no panic. A green exit with no PNG is the silent-fail trap.
    $bytes = if (Test-Path $raw) { (Get-Item $raw).Length } else { 0 }
    $all   = $stdout + $stderr
    $saved = $all -match 'SHOT saved'
    $panic = $all -match 'panicked|B0001'
    $bad = @()
    if ($timedOut)      { $bad += "timeout" }
    if ($bytes -lt 2048){ $bad += "no-png" }
    if (-not $saved)    { $bad += "no-SHOT" }
    if ($panic)         { $bad += "PANIC" }

    # Geometry gate. The resize is a race against the engine's fixed 3.2s shutter, and
    # losing it produces a perfectly valid PNG at the wrong size -- which then reads as a
    # look change on micro-contrast, edge energy and penumbra px when nothing in the
    # render moved. Ask the file, not the SetWindowPos return value.
    $rawSize = Png-Size $raw
    $wantSize = "$($s.W)x$($s.H)"
    if ($bytes -ge 2048 -and $rawSize -ne $wantSize) { $bad += "size-mismatch[$rawSize want $wantSize]" }

    # Per-plate content gate: a saved PNG proves the swapchain was grabbed, not that
    # the subject was in it. anim.rs prints one ANIM_RIG_WEAPON line per rigged actor,
    # so s3-clash can insist both blades are on stage before the frame counts.
    foreach ($m in @($s.Expect)) {
        if ($m -and ($all -notmatch [regex]::Escape($m))) { $bad += "missing[$m]" }
    }

    # Which actors got a rig. ANIM_RIG_WEAPON is anim.rs:1038, one line per rigged actor.
    #
    # This does NOT tell you whether the placeholder capsule is in the frame, and an
    # earlier version of this line claimed it did. ANIM_RIG_WEAPON has been printed since
    # well before the fix; the `if rigged { return }` guard that actually keeps the
    # capsule hidden is dodge_parry.rs:291 and is much newer. Proof: every baseline plate
    # here logs `RIGS Husk,Player` and the red pill is plainly in the frame. The capsule
    # verdict comes from the before/after pair on the sheet, not from stdout.
    $rigLines = ($all -split "`n" | Select-String -Pattern 'ANIM_RIG_WEAPON spawn actor=' |
                 ForEach-Object { ($_ -replace '.*actor=(\w+).*', '$1') } | Sort-Object -Unique)
    $s.Rigs = @($rigLines)
    "    RIGS $(if ($rigLines) { $rigLines -join ',' } else { 'none' })"

    # World provenance. maps/edhari.json is edited by other lanes and a block-count change
    # moves far more pixels than any look tweak (it is what made the published 19:53
    # frames unusable as baselines: 8513 blocks then, 8838 now). Capture it per plate so
    # the pair can refuse to blame the binary for a content edit.
    $s.MapBlocks = if ($all -match 'MAP_LOAD ok[^\r\n]*blocks=(\d+)') { [int]$matches[1] } else { $null }

    if ($bad.Count -gt 0) {
        # Only the geometry race is worth a second launch: the exe's shader cache and
        # page-ins are warm now, so the window comes up in well under a second. Every
        # other failure (panic, no-SHOT, a missing Expect marker) is deterministic --
        # re-running it just burns 10s to fail identically.
        $onlySize = ($bad.Count -eq 1 -and $bad[0] -like 'size-mismatch*')
        if ($onlySize -and $attempt -lt 2) {
            "    ! $($bad[0]) -- window lost the race to the 3.2s shutter; re-shooting warm"
            continue
        }
        "    X FAIL: $($bad -join ' ')  (bytes=$bytes exit=$($proc.ExitCode)) -- see $log"
        ($stdout + $stderr) -split "`n" | Select-String -Pattern 'panicked|B0001|error' | Select-Object -First 4 | ForEach-Object { "      $_" }
        $s.AfterPng = $null
        $fails++
        break
    }

    $graded = Invoke-Dehud2 $raw
    $dst = Join-Path $AfterDir "$($s.Key)-nohud2.png"
    Copy-Item $graded $dst -Force
    $s.AfterPng = $dst

    # A CINE plate that silently fell back to the gameplay boom is the trap the s4
    # re-shoot was about -- so echo the camera line the engine actually parked on.
    $cineLine = (($stdout + $stderr) -split "`n" | Select-String -Pattern 'CINE ' | Select-Object -First 1)
    if ($cineLine) { "    $($cineLine.ToString().Trim())" }
    "    OK raw=$rawSize graded=$(Png-Size $dst) ${bytes}b"
    break
  }
}

# ---------------------------------------------------------------------------
# 5. manifest + sheet + regrade
# ---------------------------------------------------------------------------
$paired = @($plan | Where-Object { $_.BeforePng -and $_.AfterPng })

# The baseline pass wrote its own map_blocks per plate. Pull them back in so the pair can
# say "the world changed under us" instead of billing a content edit to the binary.
$beforeManifest = Join-Path $Root "_poppy_shotset_before\manifest.json"
$beforeMap = @{}
if ((-not $BeforeSide) -and (Test-Path $beforeManifest)) {
    try {
        (Get-Content $beforeManifest -Raw | ConvertFrom-Json).shots |
            ForEach-Object { if ($null -ne $_.map_blocks) { $beforeMap[$_.key] = [int]$_.map_blocks } }
    } catch { Write-Warning "could not read $beforeManifest -- map drift unchecked" }
}

$rows = @()
$drift = @()
foreach ($s in $plan) {
    $bBlocks = if ($beforeMap.ContainsKey($s.Key)) { $beforeMap[$s.Key] } else { $null }
    $mapDrift = ($null -ne $bBlocks -and $null -ne $s.MapBlocks -and $bBlocks -ne $s.MapBlocks)
    if ($mapDrift) { $drift += "$($s.Key): before $bBlocks -> after $($s.MapBlocks) blocks" }
    $rows += [ordered]@{
        key      = $s.Key
        note     = $s.Note
        argv     = $s.CliArgs
        cine     = $s.Cine
        env      = (($s.Env.Keys | Sort-Object | ForEach-Object { "$_=$($s.Env[$_])" }) -join ' ')
        expect   = @($s.Expect | Where-Object { $_ })
        rigs     = @($s.Rigs | Where-Object { $_ })
        before   = if ($s.BeforePng) { (Resolve-Path $s.BeforePng).Path.Substring($Root.Length + 1).Replace('\','/') } else { $null }
        after    = if ($s.AfterPng)  { (Resolve-Path $s.AfterPng ).Path.Substring($Root.Length + 1).Replace('\','/') } else { $null }
        before_src = $s.Before.Replace('\','/')
        ref      = $s.Ref
        before_size = if ($s.BeforePng) { Png-Size $s.BeforePng } else { "-" }
        after_size  = if ($s.AfterPng)  { Png-Size $s.AfterPng  } else { "-" }
        map_blocks  = $s.MapBlocks
        before_map_blocks = $bBlocks
        map_drift   = $mapDrift
        paired   = [bool]($s.BeforePng -and $s.AfterPng)
    }
}
$manifest = [ordered]@{
    exe        = $Exe.Replace('\','/')
    exe_mtime  = $exeItem.LastWriteTime.ToString('o')
    exe_sha256 = $sha
    commit     = $head
    dry_run    = [bool]$DryRun
    extra_env  = $ExtraEnv
    out_dir    = $OutDir.Replace('\','/')
    shots      = $rows
}
$manifestPath = Join-Path $Root "$OutDir\manifest.json"
$manifest | ConvertTo-Json -Depth 5 | Out-File -FilePath $manifestPath -Encoding utf8

$regradeCmd = "python scripts/regrade.py --before $($OutDir.Replace('\','/'))/before/ --after $($OutDir.Replace('\','/'))/after/ --profile gameplay --json $($OutDir.Replace('\','/'))/regrade.json"
$sheetCmd   = "python scripts/_poppy_shotset_sheet.py $($OutDir.Replace('\','/'))/manifest.json $($OutDir.Replace('\','/'))/before-after-sheet.png"
"$regradeCmd" | Out-File -FilePath (Join-Path $Root "$OutDir\REGRADE.cmd") -Encoding ascii

# An unpaired plate is a failed plate. DONE already said FINISH_PARTIAL for it, but the
# exit code did not -- so a caller chaining on `&&` read a half-empty pass as a clean one.
$unpaired = @($plan | Where-Object { -not ($_.BeforePng -and $_.AfterPng) })
# -BeforeSide has no BEFORE side to pair against by construction, so "every plate shot
# clean" IS its FINISH_OK. Reporting PARTIAL there would make a perfectly good baseline
# set look broken to anything gating on DONE (Sun's scorecard does exactly that).
$status = if ($DryRun) { "DRYRUN" }
          elseif ($fails -eq 0 -and ($BeforeSide -or $paired.Count -eq $plan.Count)) { "FINISH_OK" }
          else { "FINISH_PARTIAL" }
$status | Out-File -FilePath (Join-Path $Root "$OutDir\DONE") -Encoding ascii -NoNewline

if ($drift.Count -gt 0) {
    Write-Warning "MAP DRIFT -- maps/edhari.json changed between the two sides, so these pairs measure a world edit, not the binary:"
    $drift | ForEach-Object { "    $_" }
    "    fix: re-shoot the baseline -- powershell -File scripts/_poppy_shotset.ps1 -BeforeSide -NoRegrade"
}

"`n=== SUMMARY"
if ($BeforeSide) {
    "  baseline plates: $($plan.Count)   shot-fails: $fails   -> $OutDir/after/  (this dir is what every plate's Before points at)"
} else {
"  plates:  $($plan.Count)   shot-fails: $fails   pairs ready: $($paired.Count)   unpaired: $($unpaired.Count)   map-drift: $($drift.Count)"
}
if ($unpaired.Count -gt 0 -and -not $DryRun -and -not $BeforeSide) {
    foreach ($u in $unpaired) {
        $why = if (-not $u.BeforePng) { "no BEFORE ($($u.Before))" } else { "no AFTER (shot failed)" }
        "  UNPAIRED $($u.Key): $why"
    }
}
"  before:  $OutDir/before/    after: $OutDir/after/"
"  manifest:$OutDir/manifest.json   DONE=$status"

if ($paired.Count -gt 0) {
    "`n=== SHEET"
    $r = Invoke-Py "scripts\_poppy_shotset_sheet.py" @($manifestPath, (Join-Path $Root "$OutDir\before-after-sheet.png"))
    $r.Out.Trim()
    if ($r.Rc -ne 0) { Write-Warning "sheet script exited $($r.Rc)" }

    if (-not $NoRegrade -and -not $DryRun) {
        "`n=== REGRADE  ($regradeCmd)"
        $r = Invoke-Py "scripts\regrade.py" @("--before", "$OutDir\before\", "--after", "$OutDir\after\",
                                              "--profile", "gameplay",
                                              "--json", (Join-Path $Root "$OutDir\regrade.json"))
        $r.Out | Out-File -FilePath (Join-Path $Root "$OutDir\regrade_console.txt") -Encoding utf8
        $r.Out
        if ($r.Rc -ne 0) { Write-Warning "regrade exited $($r.Rc)" }
    }
}

"`nNEXT"
"  $regradeCmd"
"  $sheetCmd"
# Exit non-zero on a shot failure OR an unpaired plate. -BeforeSide has no BEFORE side by
# definition, and -AllowMissingBefore says the caller chose the unpaired run deliberately.
if ($fails -gt 0) { exit 1 }
if (-not $DryRun -and -not $BeforeSide -and -not $AllowMissingBefore -and $unpaired.Count -gt 0) { exit 1 }

} finally { Pop-Location }
