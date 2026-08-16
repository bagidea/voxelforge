#!/usr/bin/env bash
# Shoot the four-set gear ladder twice out of ONE binary — pre-sculpt and
# post-sculpt — then compose the contact sheets.
#
# Both passes use the same exe, the same stage, the same camera (AB_CAM, baked),
# the same sun (the stage default) and the same frame counter. The ONLY thing that
# differs between them is VOXELFORGE_SCULPT, which truncates every gear table to
# its pre-sculpt prefix and swaps Auren's body back to AUREN_V2. See
# `equipment::sculpt_on` for why this is not two binaries.
#
# usage: scripts/_fl_char_quad.sh [resolution]     (default 1440,1080)
set -euo pipefail

cd "$(dirname "$0")/.."
EXE=target-pixel/release/voxelforge_charshot.exe
OUT=docs/assets/characters
RES="${1:-1440,1080}"

[ -x "$EXE" ] || { echo "no exe at $EXE — build it first"; exit 1; }
mkdir -p "$OUT"

echo "== exe: $(ls -l "$EXE" | awk '{print $5" bytes  "$6" "$7" "$8}')"
echo "== res: $RES"

shoot() {           # $1 = sculpt flag, $2 = output stem
  echo "--- VOXELFORGE_SCULPT=$1 -> $2"
  VOXELFORGE_CHARSHOT=quad \
  VOXELFORGE_SCULPT="$1" \
  VOXELFORGE_RES="$RES" \
  VOXELFORGE_SHOT="$OUT/$2.png" \
    "$EXE" 2>&1 | tee "$OUT/$2.runlog" | grep -E 'SCULPT_PREFIX|CAST_SPAWN|CAST_WEAR|CAST_MATERIALS|SWAP_(ROOT|CAPTURE|APPLY|DONE)|CHARSHOT' || true
}

shoot 0 gear-ladder-a-before
shoot 1 gear-ladder-b-after

echo "== composing sheets"
python scripts/_fl_char_sheet.py

echo "== plates on disk"
ls -l "$OUT" | grep -E 'gear-ladder' || true
