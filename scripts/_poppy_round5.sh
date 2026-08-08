#!/usr/bin/env bash
# Round 5 of the magenta/warmth sweep (Poppy, 2026-08-01). Not a gate.
#
# Exists to close the two holes a review found in rounds 1-4, and to re-shoot
# the rows that matter on a binary that is actually NEWER than look.rs (the
# round 1-4 frames were shot with a release exe from 12:12:10 against a look.rs
# last written 12:17:21 — the numbers were right but not reproducible):
#
#   HOLE 1 — "Gate B dies at temp ~0.050" was EXTRAPOLATED, not measured. The
#   only high-temperature row in rounds 1-4 (`f-t0045-L4`) came back
#   `[SKIP] B sky order — flat top region too dark to be sky`, i.e. the sky was
#   not measurable at all, so it could not confirm or deny anything about sky
#   ordering. The `B > R > G` seen on it was Gate C (SUNLIT) ordering, a
#   different axis. Rows `m-t005-L0` / `m-t006-L0` re-ask the question on the
#   SHIPPED lights, where every round-1 frame proved the sky IS measurable.
#
#   HOLE 2 — the candidate was only ever measured on the BOOT frame. Gate 3
#   ships three, and `gate3-after-walk` is the one that was worst when broken
#   (73.73% magenta), so it is the one a warmth push must not regress. `MODE`
#   switches the shoot to the `--play-demo` walk path used by
#   scripts/gate3_shoot.sh:184.
#
# Usage:
#   bash scripts/_poppy_round5.sh                     # boot frames
#   MODE=walk OUT=_poppy_round5_walk bash scripts/_poppy_round5.sh
set -uo pipefail

BIN="${BIN:-./target/release/voxelforge.exe}"
MODE="${MODE:-boot}"
OUT="${OUT:-_poppy_round5}"
mkdir -p "$OUT"

case "$MODE" in
  boot) PLAY_ENV="VOXELFORGE_PLAY=1" ;;
  walk) PLAY_ENV="VOXELFORGE_PLAY_DEMO=1" ;;
  *) echo "MODE must be boot|walk"; exit 2 ;;
esac

# label          grade: temp,sat,mid,hi_gain    light: keyRGB,ambRGB
CANDIDATES=(
  # Controls first — same rule as the round-1 gate. If `ctrl-bug` does not come
  # back magenta, the env hook is not reaching the shader and nothing below
  # means anything.
  "ctrl-bug       0.10,1.02,1.30,0.64            1.0,1.0,1.0,1.0,1.0,1.0"
  "shipped        0.02,1.02,1.30,0.64            1.00,0.86,0.66,0.98,0.78,0.52"

  # HOLE 1 — the Gate B sky crossing, measured on lights that leave the sky
  # measurable. wb_matrix.py puts the CPU-side R-passes-B crossing at ~0.099;
  # these ask what the real post stack does at 0.05 / 0.06 / 0.08.
  "m-t005-L0      0.05,1.02,1.30,0.64            1.00,0.86,0.66,0.98,0.78,0.52"
  "m-t006-L0      0.06,1.02,1.30,0.64            1.00,0.86,0.66,0.98,0.78,0.52"
  "m-t008-L0      0.08,1.02,1.30,0.64            1.00,0.86,0.66,0.98,0.78,0.52"

  # HOLE 2 — the leading candidate from round 4, re-shot on a fresh binary and
  # (with MODE=walk) on the frame it was never measured on.
  "cand-s160      0.03,1.60,1.30,0.64            1.00,0.86,0.66,0.98,0.78,0.52"
  "cand-s150      0.03,1.50,1.30,0.64            1.00,0.86,0.66,0.98,0.78,0.52"
  "cand-s170      0.03,1.70,1.30,0.64            1.00,0.86,0.66,0.98,0.78,0.52"
)

[ -f "$BIN" ] || { echo "x no binary at $BIN"; exit 1; }
echo "binary: $BIN ($(date -r "$BIN" '+%F %H:%M:%S'))  mode=$MODE"
echo "look.rs: $(date -r client/src/look.rs '+%F %H:%M:%S')"

SHOT_PNGS=()
for row in "${CANDIDATES[@]}"; do
  read -r label grade light <<<"$row"
  if [ "$#" -gt 0 ]; then
    hit=0
    for w in "$@"; do [ "$w" = "$label" ] && hit=1; done
    [ "$hit" -eq 1 ] || continue
  fi
  png="$OUT/$label.png"
  SHOT_PNGS+=("$png")
  echo "=== $label  grade=$grade  light=$light"
  env "$PLAY_ENV" \
      VOXELFORGE_LOOK_QUALITY=high \
      VOXELFORGE_LOOK_GRADE="$grade" \
      VOXELFORGE_LOOK_LIGHT="$light" \
      VOXELFORGE_SHOT="$png" \
      "$BIN" >"$OUT/$label.log" 2>&1
  rc=$?
  bytes=0; [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  echo "    exit=$rc bytes=$bytes"
  if [ "$rc" -ne 0 ] || [ "$bytes" -lt 2048 ] || ! grep -q 'SHOT saved' "$OUT/$label.log"; then
    echo "    x shot failed — see $OUT/$label.log"
    grep -nE 'panicked|B0001|error' "$OUT/$label.log" | head -3
  fi
done

echo ""
echo "=== measurements ==="
python scripts/_poppy_sweep_report.py "${SHOT_PNGS[@]}"
