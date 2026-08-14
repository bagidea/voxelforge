#!/usr/bin/env python3
"""Compile a single audible proof reel from the REAL asset/audio wav files,
placed at timestamps derived from a REAL run of voxelforge_audio_proof.exe
(see docs/assets/audio-proof/voxelforge-audio-proof.runlog for the raw
AUDIO_PLAY:/AUDIO_ZONE:/AUDIO_SUMMARY: log this reel is built from).

This is NOT a live capture of the game's audio device output (headless CI/
proof bins have no such capture path here) — it is a deterministic ffmpeg
mixdown of the exact same .wav files audio.rs loads, laid out on a
COMPRESSED timeline so a human can listen to one ~15s clip and hear every
category instead of scrubbing a 12.5s run where 371 footstep_stone.wav hits
bury everything else. Every (file, gain) pair below is lifted directly from
audio.rs's SfxEvent->path table and extra_layers() combos, and every zone/
footstep file choice matches what the real runlog actually logged (only
footstep_stone/footstep_sand fired this run — grass/wood did not, so they
are not claimed here). Timestamps are reel-local, not sim-local; the runlog
sidecar is the source of truth for real sim timestamps.

Usage: python scripts/gen_audio_proof_reel.py
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AUDIO_DIR = ROOT / "assets" / "audio"
OUT = ROOT / "docs" / "assets" / "audio-proof" / "voxelforge-audio-proof-reel.wav"
LOG = ROOT / "docs" / "assets" / "audio-proof" / "voxelforge-audio-proof-reel.ffmpeg.log"

# (file, start_sec, volume) — start times are reel-local (compressed), not
# the real sim-clock t= values from the runlog. Volumes/offsets for combat
# layers are copied from audio.rs::extra_layers() base_volume args.
LAYERS = [
    # background music — real spawn_music, plays for the whole run
    ("music_theme.wav", 0.00, 0.22),
    # Wilds ambience — real spawn_ambient at Play-enter (runlog t=0.00)
    ("ambient_wind.wav", 0.00, 0.45),
    # real footstep_stone.wav hits (runlog first fired t=1.02, Wilds leg)
    ("footstep_stone.wav", 0.80, 0.9),
    ("footstep_stone.wav", 1.15, 0.9),
    # real footstep_sand.wav hits (runlog first fired t=1.56, Wilds leg)
    ("footstep_sand.wav", 1.50, 0.9),
    ("footstep_sand.wav", 1.85, 0.9),
    # Village zone crossfade-in (runlog AUDIO_ZONE:Village t=5.47)
    ("ambient_village.wav", 3.00, 0.5),
    ("footstep_stone.wav", 3.30, 0.9),
    ("footstep_stone.wav", 3.60, 0.9),
    # Campfire zone crossfade-in (runlog AUDIO_ZONE:Campfire t=8.30)
    ("ambient_campfire.wav", 5.50, 0.5),
    ("footstep_sand.wav", 5.80, 0.9),
    # combat cluster — real runlog fires all of these at the same sim
    # instant t=9.00 (scripted trigger, see audio_proof_main.rs doc); here
    # staggered ~0.35-0.40s apart purely so each is distinguishable by ear.
    # File+gain pairs per event match audio.rs's play_sfx/extra_layers 1:1.
    ("swing_light.wav", 7.00, 1.0),
    ("swing_whoosh.wav", 7.00, 0.55),
    ("swing_heavy.wav", 7.35, 1.0),
    ("swing_whoosh.wav", 7.35, 0.75),
    ("hit_light.wav", 7.70, 1.0),
    ("impact_tail.wav", 7.70, 0.35),
    ("hit_splatter.wav", 7.70, 0.45),
    ("hit_heavy.wav", 8.10, 1.0),
    ("impact_thump.wav", 8.10, 0.9),
    ("impact_tail.wav", 8.10, 0.55),
    ("hit_splatter.wav", 8.10, 0.7),
    ("hit_block.wav", 8.55, 1.0),
    ("impact_thump.wav", 8.55, 0.6),
    ("hit_parry.wav", 8.95, 1.0),
    ("impact_tail.wav", 8.95, 0.5),
    ("enemy_death.wav", 9.35, 1.0),
    ("impact_thump.wav", 9.35, 0.7),
    ("enemy_growl.wav", 9.80, 1.0),
    ("player_hurt.wav", 10.25, 1.0),
    ("player_death.wav", 10.70, 1.0),
    ("player_respawn.wav", 11.20, 1.0),
]

TOTAL_DURATION = 14.5  # seconds — covers the respawn chime's tail


def build_filter():
    inputs = []
    chains = []
    labels = []
    for i, (fname, start, vol) in enumerate(LAYERS):
        path = AUDIO_DIR / fname
        if not path.exists():
            raise SystemExit(f"missing asset: {path}")
        inputs += ["-i", str(path)]
        delay_ms = int(round(start * 1000))
        label = f"a{i}"
        chains.append(
            f"[{i}:a]adelay={delay_ms}:all=1,volume={vol}[{label}]"
        )
        labels.append(f"[{label}]")
    mix = "".join(labels) + f"amix=inputs={len(LAYERS)}:duration=longest:normalize=0[mixed]"
    post = f"[mixed]alimiter=limit=0.9:attack=5:release=50,atrim=0:{TOTAL_DURATION}[out]"
    filter_complex = ";".join(chains) + ";" + mix + ";" + post
    return inputs, filter_complex


def main():
    inputs, filter_complex = build_filter()
    OUT.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        "ffmpeg", "-y",
        *inputs,
        "-filter_complex", filter_complex,
        "-map", "[out]",
        "-ar", "44100", "-ac", "2",
        str(OUT),
    ]
    with open(LOG, "w", encoding="utf-8") as log_f:
        result = subprocess.run(cmd, stdout=log_f, stderr=subprocess.STDOUT)
    if result.returncode != 0:
        print(f"ffmpeg FAILED exit={result.returncode}, see {LOG}", file=sys.stderr)
        sys.exit(1)
    print(f"OK wrote {OUT}")


if __name__ == "__main__":
    main()
