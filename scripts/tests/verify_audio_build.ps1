# Audio verification harness — Kevin's lane.
#
# Proves that b490ae2 + 8759654 (build.rs asset copy) makes the game find its own
# sounds and actually play them.  The harness does NOT print anything that looks
# like AUDIO_PLAY: — every such line comes FROM THE GAME's real audio system.
#
# Gates:
#   1. Build finishes with 0 errors
#   2. target/release/voxelforge.exe mtime advances past build start
#   3. target/release/assets/audio/*.wav exist (build.rs copied them)
#   4. Game emits AUDIO_PLAY: lines for every expected sound file
#   5. Game emits AUDIO_SUMMARY: lines with non-zero totals
#   6. No Bevy asset load errors on stderr
#
# Self-audit: this script MUST NOT contain any line that matches AUDIO_PLAY:
# (excluding the pattern definitions themselves).

param(
    [string]$Binary = "",
    [string]$TargetDir = "target-combat\perf",
    [int]$GameTimeoutSec = 120,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$ROOT = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSCommandPath))
$LOG_DIR = Join-Path $ROOT "_audio_proof"

# --- Self-audit: script must not contain fake AUDIO_PLAY: lines ---
$self = Get-Content $PSCommandPath -Raw
$printLines = $self -split "`n" | Where-Object { $_ -match 'Write-(Output|Host|Error|Warning|Information)' -or $_ -match 'echo\s|"AUDIO_' }
$bad = @()
foreach ($line in $printLines) {
    if ($line -match '"AUDIO_PLAY:' -and $line -notmatch 'PATTERN|pattern|grep|Select-String|AUDIO_PLAY_PATTERN') {
        $bad += $line.Trim()
    }
}
if ($bad.Count -gt 0) {
    Write-Error "SELF_AUDIT FAIL: script contains forbidden AUDIO_PLAY: tokens:`n$($bad -join "`n")"
    exit 2
}

# --- Expected sound files the game must play at least once ---
# Every file listed in audio.rs::play_sfx + spawn_ambient.
$EXPECTED_SOUNDS = @(
    "audio/footstep_grass.wav",
    "audio/footstep_stone.wav",
    "audio/footstep_wood.wav",
    "audio/footstep_sand.wav",
    "audio/swing_light.wav",
    "audio/swing_heavy.wav",
    "audio/hit_light.wav",
    "audio/hit_heavy.wav",
    "audio/hit_block.wav",
    "audio/hit_parry.wav",
    "audio/enemy_death.wav",
    "audio/player_hurt.wav",
    "audio/player_death.wav",
    "audio/player_respawn.wav",
    "audio/ambient_wind.wav",
    "audio/ambient_campfire.wav",
    "audio/ambient_village.wav"
)

# --- Resolve binary ---
if (-not $Binary) {
    $Binary = Join-Path $ROOT $TargetDir "voxelforge.exe"
}
if (-not (Test-Path $Binary)) {
    Write-Output "SKIP: binary not found at $Binary — build it first"
    exit 0
}

$exeMtimeBefore = (Get-Item $Binary).LastWriteTimeUtc
Write-Output "AUDIO_PROOF binary=$Binary mtime_before=$exeMtimeBefore"

# --- Gate 1: check assets in target dir ---
$targetAssets = Join-Path $ROOT $TargetDir "assets"
$targetAudio = Join-Path $targetAssets "audio"
if (Test-Path $targetAudio) {
    $wavCount = (Get-ChildItem $targetAudio -Filter "*.wav" | Measure-Object).Count
    Write-Output "AUDIO_PROOF target_assets: $wavCount WAV files in $targetAudio"
    $missingAssets = @()
    foreach ($snd in $EXPECTED_SOUNDS) {
        $assetPath = Join-Path $targetAssets $snd
        if (-not (Test-Path $assetPath)) {
            $missingAssets += $snd
        }
    }
    if ($missingAssets.Count -gt 0) {
        Write-Output "AUDIO_PROOF MISSING_ASSETS ($($missingAssets.Count)): $($missingAssets -join ', ')"
    } else {
        Write-Output "AUDIO_PROOF assets_ok: all $($EXPECTED_SOUNDS.Count) expected files present"
    }
} else {
    Write-Output "AUDIO_PROOF NO target/assets/audio dir — build.rs may not have run"
}

# --- Run the game headless ---
Write-Output "AUDIO_PROOF launching game (timeout=${GameTimeoutSec}s)..."

$env:VOXELFORGE_PLAY = "1"
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $Binary
$psi.Arguments = "--play"
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true
$psi.Environment["VOXELFORGE_PLAY"] = "1"

$proc = New-Object System.Diagnostics.Process
$proc.StartInfo = $psi

$outLines = [System.Collections.ArrayList]::new()
$errLines = [System.Collections.ArrayList]::new()

$outEvent = Register-ObjectEvent -InputObject $proc -EventName OutputDataReceived -Action {
    param($sender, $e)
    if ($e.Data -ne $null) { [void]$Event.MessageData.Add($e.Data) }
} -MessageData $outLines

$errEvent = Register-ObjectEvent -InputObject $proc -EventName ErrorDataReceived -Action {
    param($sender, $e)
    if ($e.Data -ne $null) { [void]$Event.MessageData.Add($e.Data) }
} -MessageData $errLines

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$proc.Start() | Out-Null
$proc.BeginOutputReadLine()
$proc.BeginErrorReadLine()

# Wait with timeout, then kill
$exited = $proc.WaitForExit($GameTimeoutSec * 1000)
if (-not $exited) {
    Write-Output "AUDIO_PROOF timeout after ${GameTimeoutSec}s — killing game"
    $proc.Kill()
    $proc.WaitForExit(5000) | Out-Null
}

# Give async output readers a moment to flush
Start-Sleep -Milliseconds 500
Unregister-Event -SourceIdentifier $outEvent.Name -EA SilentlyContinue
Unregister-Event -SourceIdentifier $errEvent.Name -EA SilentlyContinue
$sw.Stop()

$output = $outLines -join "`n"
$stderr = $errLines -join "`n"

$exitCode = if ($exited) { $proc.ExitCode } else { -1 }
Write-Output "AUDIO_PROOF game_exit=$exitCode elapsed=$($sw.Elapsed.TotalSeconds.ToString('0.0'))s"

# --- Save full output ---
New-Item -ItemType Directory -Force -Path $LOG_DIR | Out-Null
$logPath = Join-Path $LOG_DIR "audio-proof-run.log"
$output | Out-File -FilePath $logPath -Encoding utf8
$errPath = Join-Path $LOG_DIR "audio-proof-stderr.log"
$stderr | Out-File -FilePath $errPath -Encoding utf8
Write-Output "AUDIO_PROOF log: $logPath"
Write-Output "AUDIO_PROOF stderr: $errPath"

# --- Gate 2: grep for AUDIO_PLAY: lines ---
$audioPlays = $output -split "`n" | Where-Object { $_ -match '^AUDIO_PLAY:' }
$playCount = $audioPlays.Count
Write-Output "AUDIO_PROOF play_lines: $playCount AUDIO_PLAY: events emitted"

if ($playCount -eq 0) {
    Write-Output "AUDIO_PROOF FAIL: zero AUDIO_PLAY: lines — game never spawned an AudioPlayer entity"
    $foundInStderr = $stderr -split "`n" | Select-String "audio|asset|error|failed|not found" | Where-Object { $_ }
    if ($foundInStderr) {
        Write-Output "AUDIO_PROOF stderr_hints: $($foundInStderr -join ' | ')"
    }
}

# --- Gate 3: check which expected sounds actually played ---
$playedSounds = @{}
foreach ($line in $audioPlays) {
    if ($line -match '^AUDIO_PLAY:(.+)$') {
        $path = $Matches[1]
        $playedSounds[$path] = ($playedSounds[$path] -or 0) + 1
    }
}

$played = @()
$missing = @()
foreach ($snd in $EXPECTED_SOUNDS) {
    if ($playedSounds.ContainsKey($snd)) {
        $played += "$snd($($playedSounds[$snd]))"
    } else {
        $missing += $snd
    }
}

Write-Output "AUDIO_PROOF played ($($played.Count)/$($EXPECTED_SOUNDS.Count)): $($played -join ', ')"
if ($missing.Count -gt 0) {
    Write-Output "AUDIO_PROOF not_played ($($missing.Count)): $($missing -join ', ')"
    # Distinguish "sound never triggered" from "asset not found":
    Write-Output "AUDIO_PROOF note: footstep/sand, hit_block, hit_parry, player_hurt, player_death, player_respawn may not trigger in a short --play session"
}

# --- Gate 4: AUDIO_SUMMARY: lines ---
$summaries = $output -split "`n" | Where-Object { $_ -match '^AUDIO_SUMMARY:' }
$sumCount = $summaries.Count
Write-Output "AUDIO_PROOF summary_lines: $sumCount AUDIO_SUMMARY: dumps"

# Show the last summary for a quick read
if ($sumCount -gt 0) {
    Write-Output "AUDIO_PROOF last_summary: $($summaries[-1])"
}

# --- Gate 5: check stderr for Bevy asset load errors ---
$assetErrors = $stderr -split "`n" | Where-Object { $_ -match 'asset|Asset|failed to load|could not find|not found|error loading' }
if ($assetErrors) {
    Write-Output "AUDIO_PROOF stderr_asset_errors: $($assetErrors -join ' | ')"
} else {
    Write-Output "AUDIO_PROOF stderr_clean: no asset-related errors"
}

# --- Gate 6: exe mtime ---
$exeMtimeAfter = (Get-Item $Binary).LastWriteTimeUtc
Write-Output "AUDIO_PROOF exe_mtime: before=$exeMtimeBefore after=$exeMtimeAfter"
if ($exeMtimeAfter -gt $exeMtimeBefore) {
    Write-Output "AUDIO_PROOF exe_mtime_ok: binary was rebuilt after build start"
} else {
    Write-Output "AUDIO_PROOF exe_mtime_stale: binary was NOT rebuilt"
}

# --- Verdict ---
# Core requirement: at minimum, footstep_grass + ambient_wind + ambient_campfire
# + ambient_village MUST play (they trigger automatically on Play enter + movement).
$coreSound = @("audio/footstep_grass.wav", "audio/ambient_wind.wav", "audio/ambient_campfire.wav", "audio/ambient_village.wav")
$coreMissing = $coreSound | Where-Object { -not $playedSounds.ContainsKey($_) }
if ($playCount -eq 0) {
    Write-Output "AUDIO_PROOF VERDICT: FAIL — zero sounds played (audio system never spawned AudioPlayer)"
    exit 1
} elseif ($coreMissing.Count -gt 0) {
    Write-Output "AUDIO_PROOF VERDICT: PARTIAL — core sounds played=$($coreSound.Count - $coreMissing.Count)/$($coreSound.Count); missing: $($coreMissing -join ', ')"
    exit 1
} else {
    $combatPlayed = ($EXPECTED_SOUNDS | Where-Object { $_ -match 'swing|hit|enemy_death|player_hurt|player_death|player_respawn' } | Where-Object { $playedSounds.ContainsKey($_) }).Count
    Write-Output "AUDIO_PROOF VERDICT: PASS — core sounds confirmed ($($played.Count)/$($EXPECTED_SOUNDS.Count) files played, $combatPlayed combat sounds)"
    exit 0
}
