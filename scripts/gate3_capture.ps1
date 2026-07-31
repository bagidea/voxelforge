<#
.SYNOPSIS
    Gate 3 capture harness — one-shot VFX pair renderer for regression testing.

.DESCRIPTION
    Replicates the exact shot set `docs/assets/pairs` uses (3 VFX pairs + 1 control =
    7 plates), writing outputs to `docs/assets/gate3/` with a structured `.runlog`
    per image. The runlog includes the EXE's mtime, SHA-256, commit hash, and every
    flag the shot was launched with — so you can prove which binary a plate came from.

    Modeled on `scripts/render_vfx_pairs.sh` (Flamingo's renderer, 2026-07-31) and
    extended with binary-provenance metadata.

.PARAMETER ExePath
    Path to the voxelforge_shot executable. Default: target/release/voxelforge.exe.

    NOTE: The default `voxelforge.exe` (main binary) cannot render VFX pairs — it
    does not register `VfxPlugin` in the hero path. Use `voxelforge_shot.exe` for
    the VFX showcase. This default matches the task spec; swap it at the command line.

.PARAMETER DryRun
    Validate arg parsing and print what would be captured — do NOT launch the exe.
    Use this to test the harness itself without a running binary.

.EXAMPLE
    # Dry-run the harness (no exe launched)
    powershell -File scripts/gate3_capture.ps1 -DryRun

.EXAMPLE
    # Full capture with the shot binary
    powershell -File scripts/gate3_capture.ps1 target-vfx/debug/voxelforge_shot.exe

.NOTES
    Flag verification (grep against client/src/main.rs read_cfg):
      VOXELFORGE_SHOT        ✅ main.rs:152
      VOXELFORGE_VFX         ❌ vfx.rs:1188 / shot_main.rs:133  (mod vfx; in main.rs, no literal)
      VOXELFORGE_VFX_MUTE    ❌ vfx.rs:1219
      VOXELFORGE_VFX_EYE     ❌ vfx.rs:1475
      VOXELFORGE_VFX_AIM     ❌ vfx.rs:1476
#>

param(
    [string]$ExePath = "target/release/voxelforge.exe",
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

# Resolve project root relative to this script
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Root = Resolve-Path "$ScriptDir\.."
Push-Location $Root

# ---- output dir -----------------------------------------------------------
$OutDir = "docs\assets\gate3"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# ---- binary metadata -------------------------------------------------------
function Get-BinaryMeta {
    param([string]$Path)

    if (-not (Test-Path $Path)) {
        if ($DryRun) {
            return @{ Mtime = "(dry-run)"; Sha256 = "(dry-run)"; Commit = "(dry-run)" }
        }
        Write-Error "EXE NOT FOUND: $Path"
        exit 2
    }

    $item = Get-Item $Path
    $mtime = $item.LastWriteTime.ToString("o")
    $sha256 = (Get-FileHash -Path $Path -Algorithm SHA256).Hash

    $commit = try { (git rev-parse HEAD).Substring(0, 9) } catch { "unknown" }

    return @{ Mtime = $mtime; Sha256 = $sha256; Commit = $commit }
}

$meta = Get-BinaryMeta -Path $ExePath

# ---- framing defaults (mirrors render_vfx_pairs.sh) -----------------------
$CamEye = if ($env:VOXELFORGE_VFX_EYE) { $env:VOXELFORGE_VFX_EYE } else { "-7.90,4.50,0.30" }
$CamAim = if ($env:VOXELFORGE_VFX_AIM) { $env:VOXELFORGE_VFX_AIM } else { "-0.35,1.60,-0.55" }

# ---- helper: one shot ------------------------------------------------------
function Shoot {
    param(
        [string]$Beat,      # impact | parry | stagger
        [int]$Mute,         # 1 = emitters muted (before plate), 0 = live (after)
        [string]$PngName    # output filename stem, e.g. "impact-a-before.png"
    )

    $png = "$OutDir\$PngName"
    $runlog = "$png.runlog"

    # Build the flags string for the runlog header
    $flags = "VOXELFORGE_VFX=$Beat VOXELFORGE_VFX_MUTE=$Mute VOXELFORGE_VFX_EYE=$CamEye VOXELFORGE_VFX_AIM=$CamAim VOXELFORGE_SHOT=$png"

    Write-Host "--- $Beat mute=$Mute -> $png"

    if ($DryRun) {
        # Dry path: write a synthetic runlog proving arg plumbing works
        $dryBody = @"
[DryRun — exe not launched]
exe: $ExePath
exe_mtime: $($meta.Mtime)
exe_sha256: $($meta.Sha256)
commit: $($meta.Commit)
flags: $flags
---
"@
        $dryBody | Out-File -FilePath $runlog -Encoding utf8
        Write-Host "DRYRUN SHOT saved to $png (not written)"
        return @{ Ok = $true; Png = $png; Frame = "(dry-run)" }
    }

    # Set env vars for this shot
    $env:VOXELFORGE_VFX        = $Beat
    $env:VOXELFORGE_VFX_MUTE   = "$Mute"
    $env:VOXELFORGE_VFX_EYE    = $CamEye
    $env:VOXELFORGE_VFX_AIM    = $CamAim
    $env:VOXELFORGE_SHOT       = $png

    # Launch the exe, capture stdout+stderr
    $output = & $ExePath 2>&1 | Out-String
    $rc = $LASTEXITCODE

    # ---- write runlog (metadata header + raw exe output) -------------------
    $header = @"
exe: $ExePath
exe_mtime: $($meta.Mtime)
exe_sha256: $($meta.Sha256)
commit: $($meta.Commit)
flags: $flags
---
"@
    "$header`n$output" | Out-File -FilePath $runlog -Encoding utf8

    # The bin prints the frame number it grabbed on — echo it
    $frameLine = $output | Select-String "SHOT saved" | Select-Object -Last 1
    if ($frameLine) { Write-Host $frameLine.Line }

    # Exit-code check
    if ($rc -ne 0) {
        Write-Error "FAIL $Beat/$Mute : renderer exited $rc"
        return @{ Ok = $false; Png = $png; Frame = "" }
    }

    # Artefact check: a green exit code with no PNG is the silent-fail trap
    if (-not (Test-Path $png)) {
        Write-Error "FAIL $Beat/$Mute : no PNG written at $png (exit code was 0)"
        return @{ Ok = $false; Png = $png; Frame = "" }
    }

    $size = (Get-Item $png).Length
    if ($size -eq 0) {
        Write-Error "FAIL $Beat/$Mute : zero-byte PNG at $png"
        return @{ Ok = $false; Png = $png; Frame = "" }
    }

    Write-Host "OK   $Beat/$Mute : $size bytes"
    $frameText = if ($frameLine) { $frameLine.Line } else { "" }
    return @{ Ok = $true; Png = $png; Frame = $frameText }
}

# ---- 3 pairs + 1 control = 7 plates (same set as docs/assets/pairs) -------
$results = @()
$results += Shoot -Beat impact  -Mute 1 -PngName "impact-a-before.png"
$results += Shoot -Beat impact  -Mute 0 -PngName "impact-b-after.png"
$results += Shoot -Beat parry   -Mute 1 -PngName "parry-a-before.png"
$results += Shoot -Beat parry   -Mute 0 -PngName "parry-b-after.png"
$results += Shoot -Beat stagger -Mute 1 -PngName "stagger-a-before.png"
$results += Shoot -Beat stagger -Mute 0 -PngName "stagger-b-after.png"
# Control: second impact before-plate (same command) — renders the noise floor
$results += Shoot -Beat impact  -Mute 1 -PngName "impact-a-before-control.png"

# ---- report ----------------------------------------------------------------
Pop-Location

$ok = ($results | Where-Object { $_.Ok }).Count
$total = $results.Count

if ($DryRun) {
    Write-Host "GATE3_CAPTURE DRYRUN $ok/$total -- arg plumbing verified (no exe launched)"
    Write-Host "Runlog metadata keys: exe, exe_mtime, exe_sha256, commit, flags"
    exit 0
}

if ($ok -eq $total) {
    Write-Host "GATE3_CAPTURE PASS ${ok}/${total}  [3 pairs + 1 control] -> $OutDir"
    exit 0
} else {
    $failedPngs = ($results | Where-Object { -not $_.Ok }).Png -join ' '
    Write-Error "GATE3_CAPTURE FAIL ${ok}/${total} failed plates: $failedPngs"
    exit 1
}
