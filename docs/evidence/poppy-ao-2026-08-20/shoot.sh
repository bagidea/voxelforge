#!/usr/bin/env bash
# AO before/after plates — ONE exe (target-poppy/perf/voxelforge.exe, Build Sentinel
# run 1787233325847), one camera, one seed per pair. The ONLY thing that changes
# inside a pair is VOXELFORGE_AO: `0` = every vertex fully lit (the frame the CEO
# called flat), unset = the shipped strength (corner + contact terms).
#
# BEVY_ASSET_ROOT is set because the perf profile resolves `assets/` next to the
# exe, and target-poppy/perf has no assets dir — without it the block art silently
# falls back to procedural tiles and the pair would prove nothing about the art set.
set -u
cd "$(dirname "$0")/../../.."
EXE=target-poppy/perf/voxelforge.exe
OUT=docs/evidence/poppy-ao-2026-08-20
export BEVY_ASSET_ROOT="$PWD" VOXELFORGE_NOHUD=1 VOXELFORGE_SEED=1

# shot <name> <map> <look_cam> <ao-env-or-empty>
shot() { local out="$OUT/$1.png" map="$2" cam="$3" ao="$4"; rm -f "$out"
  env ${ao:+VOXELFORGE_AO=$ao} \
      VOXELFORGE_MAP_LOAD="$map" VOXELFORGE_LOOK_CAM="$cam" \
      VOXELFORGE_SHOT="$PWD/$out" "$EXE" >"$OUT/$1.log" 2>&1
  local code=$?
  grep -h "^AO strength" "$OUT/$1.log"
  if [[ $code -ne 0 ]]; then echo "FAIL $1 exit=$code"; return 1; fi
  [[ -f "$out" ]] && echo "OK   $1 ($(stat -c%s "$out")b)" || { echo "MISS $1"; return 1; }
}

# Pair 1 — castle inner corner: two walls meeting, the wall→grass junction along
# the whole courtyard, and the notches between crenellations.
shot pair1-corner-BEFORE-ao-off maps/castle.json 35,-30,10 0
shot pair1-corner-AFTER-ao-on  maps/castle.json 35,-30,10 ""

# Pair 2 — river_sunset canopy: broad leaf slabs with open grass underneath and
# trunks meeting the ground. This is the eave case the contact term exists for —
# the corner term alone cannot see a roof three blocks up.
shot pair2-canopy-BEFORE-ao-off maps/river_sunset.json 40,-10,8 0
shot pair2-canopy-AFTER-ao-on   maps/river_sunset.json 40,-10,8 ""

# Pair 3 — edhari village, wider: many building corners at once, the sanity check
# that the terms read across a whole scene and not just one hero corner.
shot pair3-village-BEFORE-ao-off maps/edhari.json 90,-15,9 0
shot pair3-village-AFTER-ao-on   maps/edhari.json 90,-15,9 ""
echo ALL_DONE
