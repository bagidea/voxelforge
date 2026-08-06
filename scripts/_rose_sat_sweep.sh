#!/usr/bin/env bash
# Rose — saturation sweep for the magenta-cast fix (2026-08-01). Not a gate.
#
# PURPOSE. The shipped look fix (TEMPERATURE 0.02 + warm KEY/AMBIENT lights) was
# proven to clear the *sky* half of the magenta by scripts/wb_matrix.py, but the
# *midtone* axes — grade_axes.py measured playable frames at sat 57% (target
# >=90) — had not been rendered. This holds the shipped fix constant and walks
# POST_SATURATION up until midtone saturation crosses 90, so we know (a) whether
# the shipped default already clears sat and (b) the post_sat that does if not.
#
# LANE. Runs the ONE release exe Poppy built (no cargo). Before every shot it
# polls `tasklist` and WILL NOT launch while another voxelforge is running —
# Poppy owns the gate3 shoot, this never overlaps it. Output goes to
# scripts/_rose_sat_sweep/ — never docs/assets/gate3/.
#
# Usage: bash scripts/_rose_sat_sweep.sh
set -uo pipefail

BIN="${BIN:-./target/release/voxelforge.exe}"
OUT="${OUT:-_rose_sat_sweep}"
mkdir -p "$OUT"

[ -f "$BIN" ] || { echo "✗ no binary at $BIN"; exit 1; }
echo "binary: $BIN ($(date -r "$BIN" '+%F %H:%M:%S'))"
echo "out:    $OUT"

# label      grade=temp,sat,mid,hi_gain        light=keyRGB,ambRGB  (empty = no env = shipped constants)
SHIPPED_LIGHT="1.00,0.86,0.66,0.98,0.78,0.52"
ROWS=(
  "rose-base  -                                              -"
  "sat-s102   0.02,1.02,1.30,0.64                            $SHIPPED_LIGHT"
  "sat-s120   0.02,1.20,1.30,0.64                            $SHIPPED_LIGHT"
  "sat-s140   0.02,1.40,1.30,0.64                            $SHIPPED_LIGHT"
  "sat-s160   0.02,1.60,1.30,0.64                            $SHIPPED_LIGHT"
  "sat-s180   0.02,1.80,1.30,0.64                            $SHIPPED_LIGHT"
)

# Wait until NO voxelforge process is running before launching. Poppy's gate3
# shoot is the reason this exists: this lane never opens a second instance.
wait_free() {
  local waited=0
  while tasklist 2>/dev/null | grep -qi voxelforge; do
    if [ "$waited" -eq 0 ]; then printf '    …voxelforge running (Poppy?), waiting…\n'; fi
    sleep 5; waited=$((waited+5))
    if [ "$waited" -ge 600 ]; then printf '    ✗ waited 10min, still busy — aborting row\n'; return 1; fi
  done
  return 0
}

for row in "${ROWS[@]}"; do
  read -r label grade light <<<"$row"
  png="$OUT/$label.png"
  printf '=== %s' "$label"
  [ "$grade" != "-" ] && printf '  grade=%s' "$grade"
  [ "$light" != "-" ] && printf '  light=%s' "$light"
  printf '\n'

  wait_free || continue

  if [ "$grade" = "-" ]; then
    # Pure shipped constants — no env override. The headline measurement.
    env VOXELFORGE_PLAY=1 VOXELFORGE_LOOK_QUALITY=high VOXELFORGE_SHOT="$png" \
        "$BIN" >"$OUT/$label.log" 2>&1
  else
    env VOXELFORGE_PLAY=1 VOXELFORGE_LOOK_QUALITY=high \
        VOXELFORGE_LOOK_GRADE="$grade" VOXELFORGE_LOOK_LIGHT="$light" \
        VOXELFORGE_SHOT="$png" \
        "$BIN" >"$OUT/$label.log" 2>&1
  fi
  rc=$?
  bytes=0; [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  printf '    exit=%s bytes=%s\n' "$rc" "$bytes"
  if [ "$rc" -ne 0 ] || [ "$bytes" -lt 2048 ] || ! grep -q 'SHOT saved' "$OUT/$label.log"; then
    printf '    ✗ shot failed — see %s\n' "$OUT/$label.log"
    grep -nE 'panicked|B0001|error' "$OUT/$label.log" | head -3
  fi
done

printf '\n=== grade_axes (6-axis, sat is the one in question) ===\n'
python scripts/grade_axes.py "$OUT"/*.png

printf '\n=== colour_gate (Flamingo) per frame ===\n'
for f in "$OUT"/*.png; do
  printf '\n--- %s\n' "$(basename "$f")"
  python scripts/colour_gate.py "$f" 2>&1 | tail -20 || true
done
