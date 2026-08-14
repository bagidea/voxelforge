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

RUN_END = 12.5  # AUDIO_PROOF_DONE, real t=

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

# ---- Ambience lane: three real zone segments (AUDIO_ZONE: lines)
y = lane_y["Ambience"]
zones = [
    ("ambient_wind.wav (Wilds)", 0.00, 5.47),
    ("ambient_village.wav (Village)", 5.47, 8.30),
    ("ambient_campfire.wav (Campfire)", 8.30, RUN_END),
]
for label, t0, t1 in zones:
    ax.add_patch(Rectangle((t0, y - LANE_H / 2), t1 - t0, LANE_H,
                            facecolor=C_AMBIENCE, alpha=0.28, edgecolor=C_AMBIENCE, linewidth=1.5, zorder=1))
    ax.text((t0 + t1) / 2, y, label, va="center", ha="center", fontsize=8, color=INK, zorder=3)
for t in (5.47, 8.30):
    ax.axvline(t, color=C_AMBIENCE, linewidth=1, linestyle=":", ymin=0.02, ymax=0.98, zorder=0)
    ax.text(t, lane_y["Ambience"] + LANE_H / 2 + 0.06, f"AUDIO_ZONE t={t:.2f}",
            fontsize=7, color=INK_SECONDARY, ha="center")

# ---- Footstep lane: real per-hit rug (every AUDIO_PLAY:audio/footstep_*.wav
# line in the runlog), plus the real AUDIO_SUMMARY counts.
y = lane_y["Footstep"]
import re
runlog = ROOT / "docs" / "assets" / "audio-proof" / "voxelforge-audio-proof.runlog"
stone_ts, sand_ts = [], []
for line in runlog.read_text().splitlines():
    m = re.match(r"AUDIO_PLAY:audio/footstep_(\w+)\.wav t=([\d.]+)", line)
    if m:
        surface, t = m.group(1), float(m.group(2))
        (stone_ts if surface == "stone" else sand_ts).append(t)
for t in stone_ts:
    ax.plot([t, t], [y - LANE_H / 2, y + LANE_H / 2], color=C_FOOTSTEP, linewidth=0.6, alpha=0.55, zorder=1)
for t in sand_ts:
    ax.plot([t, t], [y - LANE_H / 2, y + LANE_H / 2], color=C_FOOTSTEP, linewidth=0.6, alpha=0.9, zorder=1)
ax.text(0.15, y + LANE_H / 2 + 0.10,
        f"footstep_stone.wav x{len(stone_ts)} + footstep_sand.wav x{len(sand_ts)} "
        f"— real per-hit ticks from the runlog (grass/wood not triggered this run)",
        va="bottom", ha="left", fontsize=8, color=INK)

# ---- Combat lane: real cluster, all logged at the same real t=9.00
# (audio_proof_main.rs fires the whole SfxEvent batch in one scripted tick —
# see that file's doc header). Drawn as tight ticks here; the numbered list
# with the full file combos lives in the side panel so nothing overlaps.
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
n = len(combat_events)
for i in range(n):
    tx = 9.00 + (i - (n - 1) / 2) * 0.045
    ax.plot([tx, tx], [y - LANE_H / 2, y + LANE_H / 2], color=C_COMBAT, linewidth=1.4, zorder=2)
ax.axvline(9.00, color=C_COMBAT, linewidth=1, alpha=0.35, zorder=1)
ax.text(9.00, y - LANE_H / 2 - 0.22, "real t=9.00 — see 1-11 at right",
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
ax_list.text(0, 1.0, "Combat cluster — real t=9.00 (1 scripted tick)",
             fontsize=10, color=INK, fontweight="bold", va="top", ha="left",
             transform=ax_list.transAxes)
for i, label in enumerate(combat_events):
    ax_list.text(0, 0.90 - i * 0.082, f"{i + 1}. {label}",
                 fontsize=8.6, color=INK, va="top", ha="left", transform=ax_list.transAxes)
ax_list.text(0, 0.90 - n * 0.082 - 0.03,
             "File+gain pairs match audio.rs play_sfx / extra_layers() 1:1.",
             fontsize=7.5, color=INK_SECONDARY, va="top", ha="left", transform=ax_list.transAxes, style="italic")

fig.suptitle("Voxelforge audio proof — SFX/ambience/music event timeline",
             x=0.06, y=0.965, fontsize=14.5, color=INK, ha="left", fontweight="bold")
fig.text(0.06, 0.905,
         "Every marker is a real AUDIO_PLAY:/AUDIO_ZONE: line logged by the real, unmodified audio.rs\n"
         "(voxelforge-audio-proof.runlog, AUDIO_SUMMARY:total=436 plays). Not a mockup.",
         fontsize=8.5, color=INK_SECONDARY, ha="left", va="top")

fig.savefig(OUT, facecolor=SURFACE)
print(f"OK wrote {OUT}")
