#!/usr/bin/env python3
"""Compile a single audible proof reel from the REAL asset/audio wav files,
placed at timestamps derived from a REAL run of voxelforge_audio_proof.exe
(see docs/assets/audio-proof/voxelforge-audio-proof.runlog for the raw
AUDIO_PLAY:/AUDIO_ZONE:/AUDIO_SUMMARY: log this reel is built from).

This is NOT a live capture of the game's audio device output (headless CI/
proof bins have no such capture path here) — it is a deterministic ffmpeg
mixdown of the exact same .wav files audio.rs loads, laid out on a
COMPRESSED timeline so a human can listen to one ~15s clip instead of
scrubbing a 12.5s run where hundreds of footstep_stone.wav hits bury
everything else. Every (file, gain) pair is lifted from audio.rs's
SfxEvent->path table and extra_layers() combos.

Unlike a hand-typed timestamp table, the FACTS below (which footstep hits
are real vs synthetic, their counts, the zone-crossfade instants, the
combat-trigger instant) are parsed from the runlog every time this script
runs — the same way gen_audio_proof_timeline.py does it. If a future run's
runlog no longer matches the shape this reel's layout assumes (e.g. grass/
wood start getting real movement-triggered hits, or a zone stops crossing),
the script refuses to render rather than silently shipping a reel whose
caption no longer matches the data.

Usage: python scripts/gen_audio_proof_reel.py
"""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AUDIO_DIR = ROOT / "assets" / "audio"
RUNLOG = ROOT / "docs" / "assets" / "audio-proof" / "voxelforge-audio-proof.runlog"
OUT = ROOT / "docs" / "assets" / "audio-proof" / "voxelforge-audio-proof-reel.wav"
LOG = ROOT / "docs" / "assets" / "audio-proof" / "voxelforge-audio-proof-reel.ffmpeg.log"

_runlog_text = RUNLOG.read_text()

# sfx_proof_driver (audio.rs's TEMPORARY synthetic-catalog system) fires the
# whole SfxEvent catalog once/frame for its startup burst; every hit from
# that system lands at t<=0.05 in every run so far. Anything with t>0.05 is
# footstep_tracker (real, movement-triggered).
SYNTH_CUTOFF = 0.05


def _play_ts(fname):
    return sorted(
        float(m.group(1))
        for m in re.finditer(rf"AUDIO_PLAY:audio/{re.escape(fname)} t=([\d.]+)", _runlog_text)
    )


stone_ts = _play_ts("footstep_stone.wav")
sand_ts = _play_ts("footstep_sand.wav")
grass_ts = _play_ts("footstep_grass.wav")
wood_ts = _play_ts("footstep_wood.wav")
swing_light_ts = _play_ts("swing_light.wav")

stone_real = [t for t in stone_ts if t > SYNTH_CUTOFF]
sand_real = [t for t in sand_ts if t > SYNTH_CUTOFF]
grass_real = [t for t in grass_ts if t > SYNTH_CUTOFF]
wood_real = [t for t in wood_ts if t > SYNTH_CUTOFF]

if len(stone_real) < 4 or len(sand_real) < 3:
    raise SystemExit(
        f"runlog only has {len(stone_real)} real stone / {len(sand_real)} real sand "
        f"footstep hits (t>{SYNTH_CUTOFF}) — need >=4/>=3 for this reel's footstep "
        "layout; update the layout or get a fuller run"
    )
if grass_real or wood_real:
    raise SystemExit(
        f"runlog now has REAL movement-triggered footstep hits for grass "
        f"({len(grass_real)}) and/or wood ({len(wood_real)}) — this is new evidence "
        "footstep_tracker reaches those surfaces; update LAYERS and the doc header "
        "to add real grass/wood layers instead of only the synthetic driver burst"
    )

zone_events = [(m.group(1), float(m.group(2))) for m in re.finditer(r"AUDIO_ZONE:(\w+) t=([\d.]+)", _runlog_text)]
village_t = next((t for z, t in zone_events if z == "Village"), None)
campfire_t = next((t for z, t in zone_events if z == "Campfire"), None)
if village_t is None:
    raise SystemExit(
        "runlog no longer crosses into Village — this reel's layout assumes it does"
    )
# Campfire is deliberately NOT required: as of the 23:32 run the walk script
# never re-enters it (only AUDIO_ZONE:Village then back to AUDIO_ZONE:Wilds
# appear) — ambient_campfire.wav's one AUDIO_PLAY: hit is the t=0.00
# sfx_proof_driver startup burst, not a real crossfade. A prior hand-typed
# version of this LAYERS list claimed a "Campfire zone crossfade-in" here
# that never happened in the run it shipped against; parsing catches that
# instead of re-asserting it.
if campfire_t is not None:
    raise SystemExit(
        f"runlog now has a real AUDIO_ZONE:Campfire crossfade (t={campfire_t:.2f}) — "
        "this reel's layout has no Campfire section; add one instead of silently "
        "dropping real data"
    )

if not swing_light_ts:
    raise SystemExit("no swing_light.wav hits in runlog — can't place the combat cluster")
combat_t = max(swing_light_ts)

village_stone = [t for t in stone_real if t >= village_t][:2]
if len(village_stone) < 2:
    raise SystemExit(
        "not enough real stone hits after the Village crossfade to fill this reel's "
        "layout — update the layout"
    )

if not (stone_real[0] < village_t < combat_t):
    raise SystemExit(
        f"runlog anchor order broke the assumed sim-time story (first real stone hit "
        f"{stone_real[0]:.2f}, Village {village_t:.2f}, combat {combat_t:.2f} must be "
        "strictly increasing) — update the layout"
    )

summary_totals = re.findall(r"AUDIO_SUMMARY:total=(\d+)", _runlog_text)
summary_total = summary_totals[-1] if summary_totals else "?"

# sim-time anchors (all parsed above, real) -> fixed reel-local slots. The
# REEL side is a design/compression choice (how many reel-seconds each phase
# of the run gets); the SIM side is never invented.
SIM_ANCHORS = [0.00, stone_real[0], village_t, combat_t]
REEL_ANCHORS = [0.00, 0.80, 3.50, 7.00]


def to_reel(sim_t):
    for i in range(len(SIM_ANCHORS) - 1):
        s0, s1 = SIM_ANCHORS[i], SIM_ANCHORS[i + 1]
        if sim_t <= s1 or i == len(SIM_ANCHORS) - 2:
            r0, r1 = REEL_ANCHORS[i], REEL_ANCHORS[i + 1]
            frac = 0.0 if s1 == s0 else (sim_t - s0) / (s1 - s0)
            return r0 + frac * (r1 - r0)
    return REEL_ANCHORS[-1]


__doc__ += (
    f"\n\nThis run: {len(stone_real)} real footstep_stone.wav hits, "
    f"{len(sand_real)} real footstep_sand.wav hits (footstep_tracker, "
    f"movement-triggered). footstep_grass.wav/footstep_wood.wav fired "
    f"{len(grass_ts)}/{len(wood_ts)} times, all at t<={SYNTH_CUTOFF} — 100% "
    f"sfx_proof_driver (synthetic-catalog burst at the player's start-of-run "
    f"position), 0 real movement-triggered hits. Village zone crossfade at real "
    f"t={village_t:.2f} (no Campfire crossfade this run). Combat cluster's real "
    f"trigger instant t={combat_t:.2f}. AUDIO_SUMMARY:total={summary_total}.\n"
)

# (file, start_sec, volume) — start times are reel-local (compressed), not
# the real sim-clock t= values from the runlog; derived via to_reel() above
# except where noted as an artistic stagger. Volumes/offsets for combat
# layers are copied from audio.rs::extra_layers() base_volume args.
LAYERS = [
    # background music — real spawn_music, plays for the whole run
    ("music_theme.wav", 0.00, 0.22),
    # Wilds ambience — real spawn_ambient at Play-enter (runlog t=0.00)
    ("ambient_wind.wav", 0.00, 0.45),
    # proof-driver catalog burst (runlog t<=0.05, sfx_proof_driver only —
    # see doc header) — NOT footstep_tracker/movement-triggered, staggered
    # here purely so grass and wood are each distinguishable by ear.
    ("footstep_grass.wav", to_reel(grass_ts[0]) + 0.15, 0.9),
    ("footstep_wood.wav", to_reel(wood_ts[0]) + 0.40, 0.9),
    # real footstep_stone.wav hits (footstep_tracker, Wilds leg)
    ("footstep_stone.wav", to_reel(stone_real[0]), 0.9),
    ("footstep_stone.wav", to_reel(stone_real[1]), 0.9),
    # real footstep_sand.wav hits (footstep_tracker, Wilds leg)
    ("footstep_sand.wav", to_reel(sand_real[0]), 0.9),
    ("footstep_sand.wav", to_reel(sand_real[1]), 0.9),
    # Village zone crossfade-in (real AUDIO_ZONE:Village; this run never
    # crosses into Campfire — see doc header)
    ("ambient_village.wav", to_reel(village_t), 0.5),
    ("footstep_stone.wav", to_reel(village_stone[0]), 0.9),
    ("footstep_stone.wav", to_reel(village_stone[1]), 0.9),
    # combat cluster — real runlog fires all of these at the same sim
    # instant t=combat_t (scripted trigger, see audio_proof_main.rs doc);
    # here staggered ~0.35-0.40s apart purely so each is distinguishable by
    # ear. File+gain pairs per event match audio.rs's play_sfx/extra_layers
    # 1:1.
    ("swing_light.wav", to_reel(combat_t) + 0.00, 1.0),
    ("swing_whoosh.wav", to_reel(combat_t) + 0.00, 0.55),
    ("swing_heavy.wav", to_reel(combat_t) + 0.35, 1.0),
    ("swing_whoosh.wav", to_reel(combat_t) + 0.35, 0.75),
    ("hit_light.wav", to_reel(combat_t) + 0.70, 1.0),
    ("impact_tail.wav", to_reel(combat_t) + 0.70, 0.35),
    ("hit_splatter.wav", to_reel(combat_t) + 0.70, 0.45),
    ("hit_heavy.wav", to_reel(combat_t) + 1.10, 1.0),
    ("impact_thump.wav", to_reel(combat_t) + 1.10, 0.9),
    ("impact_tail.wav", to_reel(combat_t) + 1.10, 0.55),
    ("hit_splatter.wav", to_reel(combat_t) + 1.10, 0.7),
    ("hit_block.wav", to_reel(combat_t) + 1.55, 1.0),
    ("impact_thump.wav", to_reel(combat_t) + 1.55, 0.6),
    ("hit_parry.wav", to_reel(combat_t) + 1.95, 1.0),
    ("impact_tail.wav", to_reel(combat_t) + 1.95, 0.5),
    ("enemy_death.wav", to_reel(combat_t) + 2.35, 1.0),
    ("impact_thump.wav", to_reel(combat_t) + 2.35, 0.7),
    ("enemy_growl.wav", to_reel(combat_t) + 2.80, 1.0),
    ("player_hurt.wav", to_reel(combat_t) + 3.25, 1.0),
    ("player_death.wav", to_reel(combat_t) + 3.70, 1.0),
    ("player_respawn.wav", to_reel(combat_t) + 4.20, 1.0),
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
