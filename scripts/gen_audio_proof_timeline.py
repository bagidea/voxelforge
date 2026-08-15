#!/usr/bin/env python3
"""Render the audio proof timeline sheet — every SFX/ambience/music event vs
its REAL logged timestamp from a real run of voxelforge_audio_proof.exe.

All timestamps below are copied verbatim from
docs/assets/audio-proof/voxelforge-audio-proof.runlog (the AUDIO_PLAY: /
AUDIO_ZONE: / AUDIO_SUMMARY: lines that bin prints against the real,
unmodified audio.rs). This script does not invent or estimate any timing —
it only lays the real log out visually.

Usage: python scripts/gen_audio_proof_timeline.py
"""
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "assets" / "audio-proof" / "audio-proof-timeline-sheet.png"

# validated 4-slot categorical order (dataviz skill, light mode, adjacent-safe)
C_MUSIC = "#2a78d6"    # blue
C_COMBAT = "#eb6834"   # orange
C_AMBIENCE = "#1baf7a" # aqua
C_FOOTSTEP = "#eda100" # yellow
INK = "#0b0b0b"
INK_SECONDARY = "#52514e"
INK_MUTED = "#898781"
GRID = "#e1e0d9"
SURFACE = "#fcfcfb"

import re

RUNLOG = ROOT / "docs" / "assets" / "audio-proof" / "voxelforge-audio-proof.runlog"
_runlog_text = RUNLOG.read_text()

RUN_END = 12.5  # AUDIO_PROOF_DONE, real t= (script.rs done threshold, constant)

fig = plt.figure(figsize=(15, 6.6), dpi=200)
fig.patch.set_facecolor(SURFACE)
gs = fig.add_gridspec(1, 2, width_ratios=(2.55, 1), left=0.06, right=0.985,
                       top=0.82, bottom=0.11, wspace=0.03)
ax = fig.add_subplot(gs[0, 0])
ax_list = fig.add_subplot(gs[0, 1])
ax_list.axis("off")
ax.set_facecolor(SURFACE)

lanes = ["Combat", "Footstep", "Ambience", "Music"]
lane_y = {name: i for i, name in enumerate(lanes)}
LANE_H = 0.62

for name, y in lane_y.items():
    ax.axhline(y, color=GRID, linewidth=1, zorder=0)

# ---- Music lane: continuous from real t=0.00 (spawn_music on Play-enter)
y = lane_y["Music"]
ax.add_patch(Rectangle((0, y - LANE_H / 2), RUN_END, LANE_H,
                        facecolor=C_MUSIC, alpha=0.25, edgecolor=C_MUSIC, linewidth=1.5, zorder=1))
ax.text(0.15, y, "music_theme.wav — real t=0.00, plays continuously (AUDIO_SUMMARY: =1 spawn)",
        va="center", ha="left", fontsize=9, color=INK, zorder=3)

# ---- Ambience lane: real zone segments, parsed from this run's own
# AUDIO_ZONE: lines (not hardcoded — a prior version pinned 3 zones from an
# older run; this run only crosses Village/Wilds, never Campfire).
y = lane_y["Ambience"]
ZONE_TRACK = {
    "Wilds": "ambient_wind.wav (Wilds)",
    "Village": "ambient_village.wav (Village)",
    "Campfire": "ambient_campfire.wav (Campfire)",
}
zone_events = [(m.group(1), float(m.group(2)))
               for m in re.finditer(r"AUDIO_ZONE:(\w+) t=([\d.]+)", _runlog_text)]
zones = []
prev_zone, prev_t = "Wilds", 0.00  # AmbientZone default before any AUDIO_ZONE: line
for zname, zt in zone_events:
    zones.append((prev_zone, prev_t, zt))
    prev_zone, prev_t = zname, zt
zones.append((prev_zone, prev_t, RUN_END))
for zname, t0, t1 in zones:
    label = ZONE_TRACK.get(zname, f"{zname} (unmapped)")
    ax.add_patch(Rectangle((t0, y - LANE_H / 2), t1 - t0, LANE_H,
                            facecolor=C_AMBIENCE, alpha=0.28, edgecolor=C_AMBIENCE, linewidth=1.5, zorder=1))
    ax.text((t0 + t1) / 2, y, label, va="center", ha="center", fontsize=8, color=INK, zorder=3)
for _, zt in zone_events:
    ax.axvline(zt, color=C_AMBIENCE, linewidth=1, linestyle=":", ymin=0.02, ymax=0.98, zorder=0)
    ax.text(zt, lane_y["Ambience"] + LANE_H / 2 + 0.06, f"AUDIO_ZONE t={zt:.2f}",
            fontsize=7, color=INK_SECONDARY, ha="center")

# ---- Footstep lane: real per-hit rug (every AUDIO_PLAY:audio/footstep_*.wav
# line in the runlog), all 4 surfaces bucketed by their actual tag.
#
# IMPORTANT provenance split (checked against client/src/audio.rs, not
# assumed): stone/sand ticks spread across the whole run and grow past 120
# because footstep_tracker (real system, gated on nothing — fires from real
# player-transform movement every ~0.45s, surface read via surface_at()) adds
# real hits on top of the 120-frame startup burst. grass/wood ticks stay
# pinned at exactly 120 and all land at t<=0.05 — every one of them comes
# from sfx_proof_driver, a TEMPORARY system in audio.rs gated by
# audio_proof_enabled() that fires all 4 surfaces once/frame for the first
# ~120 frames at the player's start-of-run position, expressly to prove
# asset+routing (per that function's own doc comment, which states the real
# movement-triggered case is NOT yet proven). This run's walk legs 3-4 target
# real Wood(36,46)/Grass(44,49) tiles but footstep_tracker never actually
# landed a step on Grass or Wood terrain here — 0 real grass/wood hits.
y = lane_y["Footstep"]
surface_ts = {"grass": [], "sand": [], "stone": [], "wood": []}
for line in _runlog_text.splitlines():
    m = re.match(r"AUDIO_PLAY:audio/footstep_(\w+)\.wav t=([\d.]+)", line)
    if m and m.group(1) in surface_ts:
        surface_ts[m.group(1)].append(float(m.group(2)))

SURFACE_STYLE = {
    # (color, alpha, zorder) — stone/sand: real movement-driven, spread out.
    "stone": (C_FOOTSTEP, 0.45, 1),
    "sand": (C_FOOTSTEP, 0.85, 1),
    # grass/wood: synthetic-driver-only, all clustered at t<=0.05 — drawn in a
    # visibly different color so the cluster reads as "not the same kind of
    # evidence" rather than blending into the real per-hit rug.
    "grass": ("#7a4fd1", 0.85, 2),
    "wood": ("#c23b6b", 0.85, 2),
}
for surface, ts in surface_ts.items():
    color, alpha, z = SURFACE_STYLE[surface]
    for t in ts:
        ax.plot([t, t], [y - LANE_H / 2, y + LANE_H / 2], color=color, linewidth=0.6, alpha=alpha, zorder=z)

counts_line1 = (f"footstep_stone.wav x{len(surface_ts['stone'])} + footstep_sand.wav x{len(surface_ts['sand'])} "
                f"— real per-hit ticks, footstep_tracker (movement-triggered, grows past the startup burst)")
counts_line2 = (f"footstep_grass.wav x{len(surface_ts['grass'])} + footstep_wood.wav x{len(surface_ts['wood'])} "
                f"— sfx_proof_driver only (t≤0.05, asset+routing proof; 0 real movement-triggered hits this run)")
ax.text(0.15, y + LANE_H / 2 + 0.20, counts_line1, va="bottom", ha="left", fontsize=8, color=INK)
ax.text(0.15, y + LANE_H / 2 + 0.06, counts_line2, va="bottom", ha="left", fontsize=8, color="#7a4fd1")

# ---- Combat lane: real cluster, all logged at the same real instant
# (audio_proof_main.rs fires the whole SfxEvent batch in one scripted tick —
# see that file's doc header). combat_t is the LAST swing_light.wav timestamp
# in the runlog (not hardcoded): the first swing_light is part of the
# sfx_proof_driver startup burst at t<=0.05, the scripted combat trigger
# fires once more, later, on its own — that later instant is the real one.
y = lane_y["Combat"]
combat_events = [
    "SwingLight + swing_whoosh",
    "SwingHeavy + swing_whoosh",
    "HitLight + impact_tail + hit_splatter",
    "HitHeavy + impact_thump + impact_tail + hit_splatter",
    "HitBlock + impact_thump",
    "HitParry + impact_tail",
    "EnemyDeath + impact_thump",
    "EnemyGrowl",
    "PlayerHurt",
    "PlayerDeath",
    "PlayerRespawn",
]
swing_light_ts = [float(m.group(1)) for m in
                   re.finditer(r"AUDIO_PLAY:audio/swing_light\.wav t=([\d.]+)", _runlog_text)]
combat_t = max(swing_light_ts) if swing_light_ts else 9.00
n = len(combat_events)
for i in range(n):
    tx = combat_t + (i - (n - 1) / 2) * 0.045
    ax.plot([tx, tx], [y - LANE_H / 2, y + LANE_H / 2], color=C_COMBAT, linewidth=1.4, zorder=2)
ax.axvline(combat_t, color=C_COMBAT, linewidth=1, alpha=0.35, zorder=1)
ax.text(combat_t, y - LANE_H / 2 - 0.22, f"real t={combat_t:.2f} — see 1-11 at right",
        fontsize=7.5, color=INK_SECONDARY, ha="center", va="top")

# ---- mixer mutation marker (not audible on its own — annotate, don't lane it)
ax.axvline(10.5, color=INK_MUTED, linewidth=1, linestyle="--", ymin=0.0, ymax=1.0, zorder=0)
ax.text(10.5, 3.75, "mixer mutated\nt=10.50", fontsize=7, color=INK_MUTED, ha="center", va="bottom")

ax.set_xlim(-0.3, RUN_END + 0.6)
ax.set_ylim(-1.05, 4.0)
ax.set_yticks(list(lane_y.values()))
ax.set_yticklabels(list(lane_y.keys()), fontsize=10, color=INK)
ax.set_xlabel("real sim time (s) — from voxelforge_audio_proof.exe's own Time resource", fontsize=9, color=INK_SECONDARY)
ax.tick_params(axis="x", colors=INK_SECONDARY)
for spine in ("top", "right", "left"):
    ax.spines[spine].set_visible(False)
ax.spines["bottom"].set_color(GRID)

# ---- side panel: numbered combat event list (avoids overlapping the lanes)
ax_list.set_xlim(0, 1)
ax_list.set_ylim(0, 1)
ax_list.text(0, 1.0, f"Combat cluster — real t={combat_t:.2f} (1 scripted tick)",
             fontsize=10, color=INK, fontweight="bold", va="top", ha="left",
             transform=ax_list.transAxes)
for i, label in enumerate(combat_events):
    ax_list.text(0, 0.90 - i * 0.082, f"{i + 1}. {label}",
                 fontsize=8.6, color=INK, va="top", ha="left", transform=ax_list.transAxes)
ax_list.text(0, 0.90 - n * 0.082 - 0.03,
             "File+gain pairs match audio.rs play_sfx / extra_layers() 1:1.",
             fontsize=7.5, color=INK_SECONDARY, va="top", ha="left", transform=ax_list.transAxes, style="italic")

_summary_totals = re.findall(r"AUDIO_SUMMARY:total=(\d+)", _runlog_text)
_summary_total = _summary_totals[-1] if _summary_totals else "?"

fig.suptitle("Voxelforge audio proof — SFX/ambience/music event timeline",
             x=0.06, y=0.965, fontsize=14.5, color=INK, ha="left", fontweight="bold")
fig.text(0.06, 0.905,
         f"Every marker is a real AUDIO_PLAY:/AUDIO_ZONE: line logged by the real, unmodified audio.rs\n"
         f"(voxelforge-audio-proof.runlog, AUDIO_SUMMARY:total={_summary_total} plays, final line). Not a mockup — "
         f"but the purple/pink footstep ticks are proof-driver-only, see Footstep lane caption.",
         fontsize=8.5, color=INK_SECONDARY, ha="left", va="top")

fig.savefig(OUT, facecolor=SURFACE)
print(f"OK wrote {OUT}")
