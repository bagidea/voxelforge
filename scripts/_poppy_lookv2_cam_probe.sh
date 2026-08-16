#!/usr/bin/env bash
# Poppy — camera scouting for the look-v2 plates.
#
# The first castle framings came back with no horizon in them: `35,-8,60` fills
# the top third with the hillside BEHIND the castle, so "does the new sky read?"
# was unanswerable from the plate. Pitch is signed the intuitive way in this
# camera (`fwd = (0, sin p, -cos p)`, main.rs:1463) — positive looks UP — so this
# sweeps pitch/boom and prints one contact sheet to pick from BY EYE, instead of
# me guessing a second time.
#
# v2 only, one binary. This decides framing, not the look.
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
EXE="${VOXELFORGE_AB_EXE:-$ROOT/_poppy_lookv2_ab.exe}"
OUT="${PROBE_OUT:-$ROOT/_poppy_lookv2_cam}"
mkdir -p "$OUT"
[[ -f "$EXE" ]] || { echo "NO EXE at $EXE"; exit 2; }

# name : LOOK_CAM : extra env
CANDS=("${@}")

for c in "${CANDS[@]}"; do
  name=${c%%:*}; rest=${c#*:}
  cam=${rest%%:*}; extra=${rest#*:}
  [[ $extra == "$cam" ]] && extra=""
  png="$OUT/$name.png"
  rm -f "$png"
  printf '%-22s cam=%-14s ' "$name" "$cam"
  # shellcheck disable=SC2086
  env -u VOXELFORGE_LOOK_GEN -u VOXELFORGE_LOOK_ATMOS -u VOXELFORGE_LOOK_SKYGRAD \
      VOXELFORGE_PLAY=1 VOXELFORGE_NOHUD=1 VOXELFORGE_SEED=42 \
      VOXELFORGE_MAP_LOAD=maps/castle.json VOXELFORGE_LOOK_CAM="$cam" \
      VOXELFORGE_SHOT="$png" $extra \
      "$EXE" > "$OUT/$name.out.log" 2> "$OUT/$name.err.log"
  code=$?
  if [[ -f $png ]]; then echo "OK exit=$code"; else echo "MISS exit=$code"; tail -3 "$OUT/$name.err.log" | sed 's/^/      /'; fi
done

python scripts/_poppy_lookv2_contact.py "$OUT"
