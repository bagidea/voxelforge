#!/usr/bin/env bash
# render_vfx_pairs.sh — render the VFX before/after PAIRS (lane: pixel / Flamingo).
#
# Why this exists next to render_vfx.sh
# -------------------------------------
# render_vfx.sh shoots the "before" plate as VOXELFORGE_VFX=off. That was fine
# while every beat shared one static pose, but it stopped being fine the moment a
# beat started STAGING something: `off` is its own branch of the timeline, so it
# never ran the stagger reel. The pair then read as
#
#     before : husk standing upright, no effects
#     after  : husk reeled back,      effects
#
# and a reader cannot tell which of the two differences is the thing being shown.
# Two variables moved at once, so the plate proved nothing about the VFX.
#
# Here the "before" plate is the SAME beat with the emitters muted
# (VOXELFORGE_VFX_MUTE=1). Same stage, same camera, same swing, same reel, same
# beat timestamps, same code path — the single difference is that nothing is
# emitted. Whatever you can see between the two images IS the effect layer.
#
# Determinism
# -----------
# The showcase runs on a real fixed step: shot_main.rs sets
# TimeUpdateStrategy::ManualDuration(1/60 s), so `Time` advances by exactly one
# step per frame regardless of how long the frame took, and the grab happens on a
# COUNTED frame (192) rather than at the first frame past a timestamp. Both plates
# of a pair therefore catch the beat at the same point whether the box is idle or
# loaded — CPU load cannot decide what the picture shows.
#
# What is still NOT bit-stable: SSAO and the shadow filter carry per-frame noise
# and this stage runs without TAA to average it out. Two runs of one identical
# command were measured (2026-07-31) to differ on ~5 % of pixels, up to 61/255,
# including on static geometry. So read these pairs by WHAT IS IN THEM, not
# pixel-for-pixel — a diff under roughly that floor is the renderer idling.
#
# Usage:  bash scripts/render_vfx_pairs.sh [target-dir]
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TARGET="${1:-target-vfx}"
EXE="$TARGET/debug/voxelforge_shot.exe"
[ -x "$EXE" ] || EXE="$TARGET/debug/voxelforge_shot"

if [ ! -x "$EXE" ]; then
  echo "FAIL no shot binary at $EXE — build first:"
  echo "  cargo build --bin voxelforge_shot --target-dir $TARGET"
  exit 1
fi

OUT="docs/assets/pairs"
mkdir -p "$OUT"

# Same framing constants as render_vfx.sh — keep the two in sync or the pairs stop
# being comparable with the shipped singles.
CAM_EYE="${VOXELFORGE_VFX_EYE:--7.90,4.50,0.30}"
CAM_AIM="${VOXELFORGE_VFX_AIM:--0.35,1.60,-0.55}"

# shoot <beat> <mute:0|1> <outfile>
shoot() {
  local beat="$1" mute="$2" png="$OUT/$3"
  # The runlog is named after the PLATE, not after beat+mute. The control is a
  # second shot with the SAME beat and the SAME mute as impact-a-before, so a
  # beat-mute name makes it overwrite that plate's log — and the frame number
  # that proves the pair grabbed on the same frame is exactly what gets lost.
  local runlog="${png%.png}.runlog"
  echo "--- $beat mute=$mute -> $png"
  VOXELFORGE_VFX="$beat" \
  VOXELFORGE_VFX_MUTE="$mute" \
  VOXELFORGE_VFX_EYE="$CAM_EYE" \
  VOXELFORGE_VFX_AIM="$CAM_AIM" \
  VOXELFORGE_SHOT="$png" "$EXE" >"$runlog" 2>&1
  local rc=$?
  # The bin prints the frame it grabbed on. Echo it: if the fixed step ever stops
  # working, two plates of a pair grab on different frames and it shows up here
  # rather than as an unexplained difference in the picture.
  grep -h 'SHOT saved' "$runlog" 2>/dev/null | tail -1
  if [ "$rc" -ne 0 ]; then
    echo "FAIL $beat/$mute: renderer exited $rc"
    return 1
  fi
  # A green exit code with no PNG is exactly the "gate that can't fail" trap
  # docs/LANES.md warns about — assert the artefact, not the exit code.
  if [ ! -s "$png" ]; then
    echo "FAIL $beat/$mute: no PNG written at $png"
    return 1
  fi
  echo "OK   $beat/$mute: $(ls -l "$png" | awk '{print $5}') bytes"
}

# pair <beat> <label>
pair() {
  shoot "$1" 1 "$2-a-before.png" || return 1
  shoot "$1" 0 "$2-b-after.png"  || return 1
}

rc=0
# impact  — the weapon trail + the contact sparks/debris
pair impact  impact  || rc=1
# parry   — the parry ring flash (short-lived by design, caught near its peak)
pair parry   parry   || rc=1
# stagger — the reel + the ground reaction; the reel is in BOTH plates
pair stagger stagger || rc=1

# ---- the control ----------------------------------------------------------
# A SECOND copy of the impact before-plate, same command, same env. It is the
# measuring stick: whatever this differs from impact-a-before.png by is this
# renderer's own per-frame noise (SSAO + shadow filter, no TAA on this stage) on
# THIS machine, on THIS run. A before/after difference is only worth reading if it
# is well clear of that floor, and quoting a floor measured on some other day is
# how a noisy renderer gets mistaken for a working feature.
shoot impact 1 "impact-a-before-control.png" || rc=1

if [ "$rc" -eq 0 ]; then
  # 7, not 6: three pairs plus the control. The control is a rendered plate that
  # can fail like any other, so it belongs in the count that says how many passed.
  echo "RENDER_VFX_PAIRS PASS 7/7 (3 pairs + control) -> $OUT"
else
  echo "RENDER_VFX_PAIRS FAIL"
fi
exit "$rc"
