#!/usr/bin/env bash
# Rose — re-capture gate3 boot/walk/combat NOHUD frames from the rebuilt exe for
# the AAA look regrade (ambient_lux 2200 / ambient B 0.45 / PCSS 4.0 / ev100 10.9).
#
# Why this exists separately from gate3_shoot.sh:
#   - Director: capture WITH NOHUD. gate3_shoot.sh omits VOXELFORGE_NOHUD (its
#     combat frame intentionally shows HUD bars). Adding it would change the
#     shared script's contract, so this is a NOHUD-only variant in Rose's lane.
#   - Names frames *-new-nohud2.png in a recap dir so the scorecard baseline
#     frames in docs/assets/gate3/ stay untouched (they are the "before").
#
# Each shot: boot → settle → screenshot_once at t=3.2s → AppExit at t=4.4s.
set -uo pipefail

BIN="${BIN:-./target/debug/voxelforge.exe}"
OUT="${OUT:-_rose_recap_20260807}"
LOGS="$OUT/logs"
mkdir -p "$OUT" "$LOGS"

if [ ! -f "$BIN" ]; then echo "✗ binary missing: $BIN"; exit 2; fi

shoot() {
  local png="$1" label="$2" mode_env="$3"
  local log="$LOGS/$label.log"
  echo "=== $label → $png  (bin=$BIN, quality=high, NOHUD=1) ==="
  # mode_env is the VOXELFORGE_* demo/play flag name; =1 turns it on.
  env VOXELFORGE_NOHUD=1 VOXELFORGE_LOOK_QUALITY=high "${mode_env}=1" \
      "VOXELFORGE_SHOT=$png" "$BIN" >"$log" 2>&1
  local rc=$?
  local bytes=0; [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  echo "  exit=$rc  bytes=$bytes"
  grep -E 'SHOT saved|panicked|B0001|error\[' "$log" || true
  if [ "$rc" -eq 0 ] && [ "$bytes" -ge 2048 ] && grep -q 'SHOT saved' "$log" \
     && ! grep -qE 'panicked|B0001' "$log"; then
    echo "  ✓ PASS"
  else
    echo "  ✗ FAIL (see $log)"
  fi
}

shoot "$OUT/gate3-boot-new-nohud2.png"   boot   VOXELFORGE_PLAY
shoot "$OUT/gate3-walk-new-nohud2.png"   walk   VOXELFORGE_PLAY_DEMO
shoot "$OUT/gate3-combat-new-nohud2.png" combat VOXELFORGE_COMBAT_DEMO

echo ""
echo "=== capture done ==="
ls -la "$OUT"/*.png 2>/dev/null
