#!/usr/bin/env bash
# render_vfx.sh — render the VFX before/after beauty shots (lane: pixel / Flamingo).
#
# Renders the SAME stage, from the SAME camera, four times; only the VFX beat
# differs. That is the whole point of the set: a "before" plate that shares every
# other variable is the only way a reader can tell what this round actually added.
#
#   before   VOXELFORGE_VFX=off        VFX layer loaded, nothing fired
#   impact   VOXELFORGE_VFX=impact     sparks + voxel debris + ash + hit flash + trail
#   dissolve VOXELFORGE_VFX=dissolve   the Unravelling (staggered voxel death)
#   fire     VOXELFORGE_VFX=fire       campfire coals + embers + flicker
#
# The showcase TIMELINE is deterministic (fixed xorshift seed + fixed fire times),
# so the same beat is caught every run. The FRAME is not bit-identical, though:
# measured 2026-07-31, two back-to-back runs of the identical command differ on
# 5.1 % of pixels, up to 61/255 — and not only on the particles, but on static
# geometry (the hero column, the ruined wall) too. SSAO + the shadow filter carry
# per-frame noise and this stage runs without TAA to average it out.
#
# So: compare these plates by WHAT IS IN THEM, not pixel-for-pixel. A diff under
# roughly that 5 % / 61 is this renderer idling, not a change worth reading.
#
# Usage:  bash scripts/render_vfx.sh [target-dir]
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TARGET="${1:-target-vfx}"
EXE="$TARGET/debug/voxelforge_shot.exe"
[ -x "$EXE" ] || EXE="$TARGET/debug/voxelforge_shot"

if [ ! -x "$EXE" ]; then
  echo "FAIL no shot binary at $EXE — build first:"
  echo "  bash scripts/build_safe.sh build --bin voxelforge_shot --target-dir $TARGET"
  exit 1
fi

OUT="docs/assets"
mkdir -p "$OUT"

# ---- framing --------------------------------------------------------------
# The stage's baked default (vfx.rs: eye -7.9,3.5,1.7 / aim 0.15,1.45,-0.95) put
# the camera 27° off the hero→husk line and only 14° above the horizon. That is
# close enough to the swing plane that the blade's TRAIL RIBBON lands flat across
# the husk's chest — i.e. across the exact spot the hit is supposed to be legible
# (measured: the ribbon covered 15.8 % of the husk's screen box). The bodies never
# overlapped; the hero's *effect* was the occluder, which is why nudging the two
# characters apart would not have fixed it.
#
# 38.5° off-axis + 21° of lift swings the ribbon clear of the silhouette (5.0 %
# coverage — enough to read as contact, not as a mask) while keeping everything
# the shipped frame got right: the debris still flies across the wall's shadow
# (it travels -Z, so a +Z camera would hide it behind the husk instead), the
# ruined wall stays as the dark backdrop, and the campfire holds the right edge.
#
# Overridable, but these ARE the shipped framing — keep them in sync with the
# PNGs in docs/assets.
CAM_EYE="${VOXELFORGE_VFX_EYE:--7.90,4.50,0.30}"
CAM_AIM="${VOXELFORGE_VFX_AIM:--0.35,1.60,-0.55}"

shoot() {
  local mode="$1" name="$2"
  local png="$OUT/$name"
  echo "--- rendering $mode -> $png"
  # The bin waits 3.2 s (post stack settles), grabs the frame, exits at 4.4 s.
  VOXELFORGE_VFX="$mode" VOXELFORGE_VFX_EYE="$CAM_EYE" VOXELFORGE_VFX_AIM="$CAM_AIM" \
    VOXELFORGE_SHOT="$png" "$EXE"
  local rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "FAIL $mode: renderer exited $rc"
    return 1
  fi
  if [ ! -s "$png" ]; then
    # A green exit code with no PNG is exactly the "gate that can't fail" trap
    # docs/LANES.md warns about — assert the file, not the exit code.
    echo "FAIL $mode: no PNG written at $png"
    return 1
  fi
  echo "OK   $mode: $(ls -l "$png" | awk '{print $5}') bytes"
}

# NOTE (2026-07-31): only 01 + 02 have been re-rendered at the framing above —
# vfx-00-before.png and vfx-03-campfire.png on disk are still the old default
# camera, so the before/after PAIR does not currently share a viewpoint. Running
# this script re-shoots all four and puts the set back on one camera.
rc=0
shoot off      vfx-00-before.png   || rc=1
shoot impact   vfx-01-impact.png   || rc=1
shoot dissolve vfx-02-dissolve.png || rc=1
shoot fire     vfx-03-campfire.png || rc=1

if [ "$rc" -eq 0 ]; then
  echo "RENDER_VFX PASS 4/4"
else
  echo "RENDER_VFX FAIL"
fi
exit "$rc"
