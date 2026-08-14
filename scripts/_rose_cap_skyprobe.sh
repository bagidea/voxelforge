#!/usr/bin/env bash
# _rose_cap_skyprobe.sh — capture A1 dome green-probe frames to find a sky-visible path.
# TEMP harness for task (4); safe to delete after. Uses MAIN binary + PLAY (dome lives
# in look.rs, playable-session only). Not voxelforge_shot (no look plugin).
set -uo pipefail
BIN="${BIN:-./target/release/voxelforge.exe}"
OUT="${OUT:-docs/assets/gate3}"
mkdir -p "$OUT"
[ -f "$BIN" ] || { echo "no binary $BIN"; exit 1; }

run() {
  local name="$1"; shift
  local png="$OUT/_rose_probe_${name}.png"
  local log="_rose_cap_${name}.log"
  echo "=== CAP $name -> $png ==="
  env "$@" \
      VOXELFORGE_PLAY=1 \
      VOXELFORGE_LOOK_QUALITY=high \
      VOXELFORGE_LOOK_SKYPROBE=1 \
      VOXELFORGE_SHOT="$png" \
      timeout 100 "$BIN" >"$log" 2>&1
  local rc=$?
  local bytes=0; [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  echo "  exit=$rc bytes=$bytes"
  grep -aE 'LOOK-A1-PROOF|LOOK sky-dome spawned|SHOT saved|panicked|wgpu|Error' "$log" | head -8 || true
}

# 1) default camera — reproduce the Director's no-sky baseline with the fresh post-fix exe
run default
# 2/3) pitch the boom UP to expose sky (pitch>0 = up; range -1.35..+1.20), longer boom
run camup   VOXELFORGE_LOOK_CAM=0,0.7,14
run camup2  VOXELFORGE_LOOK_CAM=0,1.0,18
# 4) the REAL dome (no green probe), camup, to see the gradient if it renders
env VOXELFORGE_PLAY=1 VOXELFORGE_LOOK_QUALITY=high VOXELFORGE_LOOK_CAM=0,0.7,14 \
    VOXELFORGE_SHOT="$OUT/_rose_probe_realdome.png" \
    timeout 100 "$BIN" >"_rose_cap_realdome.log" 2>&1
echo "=== CAP realdome -> bytes=$([ -f "$OUT/_rose_probe_realdome.png" ] && wc -c <"$OUT/_rose_probe_realdome.png") ==="
grep -aE 'LOOK sky-dome spawned|SHOT saved|panicked' "_rose_cap_realdome.log" | head -5 || true
echo "=== DONE ==="
