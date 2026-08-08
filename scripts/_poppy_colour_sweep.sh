#!/usr/bin/env bash
# Scratch sweep for the magenta-cast fix (Poppy, 2026-08-01). Not a gate.
#
# Shoots one `--play` boot frame per candidate through the LookPlugin env hooks
# (`VOXELFORGE_LOOK_GRADE` / `VOXELFORGE_LOOK_LIGHT`) so the whole search runs on
# ONE release binary instead of a rebuild per candidate, then measures every
# frame with scripts/colour_gate.py + scripts/grade_axes.py.
#
# The first two rows are CONTROLS, and they matter more than the candidates:
#   ctrl-bug   temperature 0.10 + neutral lights = exactly what shipped in
#              db25758. If this does not reproduce the magenta, the env hook is
#              not reaching the shader and every other row is meaningless.
#   ctrl-cold  temperature 0.00 + neutral lights = the Jul-31 ungraded look.
#
# Usage: bash scripts/_poppy_colour_sweep.sh [label ...]
#
# With no arguments every row runs. With arguments only the named labels run,
# which is how the control gate above is actually enforced: shoot `ctrl-bug`
# and `ctrl-cold` first, read them, and only spend the other seven rows if
# ctrl-bug really did come back magenta.
set -uo pipefail

BIN="${BIN:-./target/release/voxelforge.exe}"
OUT="${OUT:-_poppy_colour_sweep}"
mkdir -p "$OUT"

# label            grade: temp,sat,mid,hi_gain     light: keyRGB,ambRGB
CANDIDATES=(
  "ctrl-bug        0.10,1.02,1.30,0.64             1.0,1.0,1.0,1.0,1.0,1.0"
  "ctrl-cold       0.00,1.02,1.30,0.64             1.0,1.0,1.0,1.0,1.0,1.0"
  "a-t000-L0       0.00,1.02,1.30,0.64             1.00,0.86,0.66,0.98,0.78,0.52"
  "a-t002-L0       0.02,1.02,1.30,0.64             1.00,0.86,0.66,0.98,0.78,0.52"
  "a-t004-L0       0.04,1.02,1.30,0.64             1.00,0.86,0.66,0.98,0.78,0.52"
  "b-t002-L1       0.02,1.02,1.30,0.64             1.00,0.80,0.55,1.00,0.72,0.42"
  "b-t002-L2       0.02,1.02,1.30,0.64             1.00,0.90,0.76,0.95,0.84,0.66"
  "c-t000-L1       0.00,1.02,1.30,0.64             1.00,0.80,0.55,1.00,0.72,0.42"
  "c-t004-L1       0.04,1.02,1.30,0.64             1.00,0.80,0.55,1.00,0.72,0.42"

  # Round 2. Round 1 showed the two levers are near-orthogonal — temperature
  # buys warmth, light colour buys blue and sat — but it also showed temperature
  # cannot be the warmth lever: sky R climbs ~+22 per +0.02 temp against a sky G
  # that is falling, so `B > G > R` (Gate B) dies at temp ~0.050, and the best
  # temperature-led row still lands at warmth 95.1 against a target of 110.
  # So temperature is pinned at or below 0.04 and the ambient/key pair does the
  # warmth work instead: warmth and blue are both measured on the midtone band,
  # which is exactly what AMBIENT_COLOR drives. L3→L5 walk ambient G and B down.
  # POST_SATURATION stays at 1.02 in every row on purpose — that knob is Rose's
  # sat sweep, and leaving it fixed keeps these rows orthogonal to theirs.
  "d-t004-L3       0.04,1.02,1.30,0.64             1.00,0.74,0.44,1.00,0.64,0.32"
  "d-t004-L4       0.04,1.02,1.30,0.64             1.00,0.70,0.38,1.00,0.58,0.24"
  "d-t004-L5       0.04,1.02,1.30,0.64             1.00,0.66,0.32,1.00,0.52,0.18"
  "e-t003-L4       0.03,1.02,1.30,0.64             1.00,0.70,0.38,1.00,0.58,0.24"
  "e-t002-L4       0.02,1.02,1.30,0.64             1.00,0.70,0.38,1.00,0.58,0.24"
  "f-t0045-L4      0.045,1.02,1.30,0.64            1.00,0.70,0.38,1.00,0.58,0.24"

  # Round 3. Round 2 dead-ended: with POST_SATURATION pinned at 1.02, warmth
  # plateaus near 92 no matter how far the ambient is walked down, because
  # darkening the ambient drops midtone R nearly as fast as midtone B. Rose's
  # sat sweep supplies the lever I was missing — saturation scales midtone R−B
  # directly, and unlike temperature it *widens* the Gate B sky margin (sky R
  # falls 122→93 from sat 1.02→1.80) instead of closing it. Their s170 reaches
  # warmth 110.1 on the shipped lights alone.
  #
  # These rows ask the one question their sweep and mine each leave open: the
  # warm-ambient L3/L1 rows already start ~7 warmth points ahead at sat 1.02,
  # so the same 110 should be reachable at a materially gentler saturation than
  # 1.70. Sat is Rose's constant to set — this is measurement for that call,
  # not a change to it.
  "g-t002-L3-s140  0.02,1.40,1.30,0.64             1.00,0.74,0.44,1.00,0.64,0.32"
  "g-t002-L3-s150  0.02,1.50,1.30,0.64             1.00,0.74,0.44,1.00,0.64,0.32"
  "g-t003-L3-s140  0.03,1.40,1.30,0.64             1.00,0.74,0.44,1.00,0.64,0.32"
  "g-t002-L4-s140  0.02,1.40,1.30,0.64             1.00,0.70,0.38,1.00,0.58,0.24"
  "g-t002-L1-s140  0.02,1.40,1.30,0.64             1.00,0.80,0.55,1.00,0.72,0.42"
  "g-t002-L0-s150  0.02,1.50,1.30,0.64             1.00,0.86,0.66,0.98,0.78,0.52"

  # Round 4. Round 3 refuted its own hypothesis: the warm ambient does not stack
  # with saturation, it fights it — at sat 1.40 the shipped lights read warmth
  # 103.6 and the warm-ambient L3 reads 98.5. Walking the ambient down darkens
  # the midtone band that warmth is measured on, and saturation then has less to
  # amplify. So the light colours stay exactly as shipped (L0 = KEY_COLOR /
  # AMBIENT_COLOR byte-for-byte) and only sat, plus at most a hair of
  # temperature, move.
  #
  # Rose's s170 clears warmth by 0.1 of a point, which is not a margin. These
  # rows buy margin two ways — more sat, or +0.01 temperature, which round 1
  # priced at roughly +6 warmth — and re-shoot their s170 verbatim to confirm
  # the number is reproducible before anyone tunes a constant to it.
  "k-t002-L0-s170  0.02,1.70,1.30,0.64             1.00,0.86,0.66,0.98,0.78,0.52"
  "k-t002-L0-s185  0.02,1.85,1.30,0.64             1.00,0.86,0.66,0.98,0.78,0.52"
  "k-t0025-L0-s170 0.025,1.70,1.30,0.64            1.00,0.86,0.66,0.98,0.78,0.52"
  "k-t003-L0-s160  0.03,1.60,1.30,0.64             1.00,0.86,0.66,0.98,0.78,0.52"
  "k-t003-L0-s165  0.03,1.65,1.30,0.64             1.00,0.86,0.66,0.98,0.78,0.52"
  "k-t003-L0-s170  0.03,1.70,1.30,0.64             1.00,0.86,0.66,0.98,0.78,0.52"
)

[ -f "$BIN" ] || { echo "✗ no binary at $BIN"; exit 1; }
echo "binary: $BIN ($(date -r "$BIN" '+%F %H:%M:%S'))"

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
  env VOXELFORGE_PLAY=1 \
      VOXELFORGE_LOOK_QUALITY=high \
      VOXELFORGE_LOOK_GRADE="$grade" \
      VOXELFORGE_LOOK_LIGHT="$light" \
      VOXELFORGE_SHOT="$png" \
      "$BIN" >"$OUT/$label.log" 2>&1
  rc=$?
  bytes=0; [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  echo "    exit=$rc bytes=$bytes"
  if [ "$rc" -ne 0 ] || [ "$bytes" -lt 2048 ] || ! grep -q 'SHOT saved' "$OUT/$label.log"; then
    echo "    ✗ shot failed — see $OUT/$label.log"
    grep -nE 'panicked|B0001|error' "$OUT/$label.log" | head -3
  fi
done

echo ""
echo "=== measurements ==="
python scripts/_poppy_sweep_report.py "${SHOT_PNGS[@]}"
