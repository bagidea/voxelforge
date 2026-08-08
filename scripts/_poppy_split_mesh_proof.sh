#!/usr/bin/env bash
# Poppy — proof that the split mesher is actually in the render path.
#
# Queues behind whatever build owns the box (a second rustc on this machine =
# 0xc0000142), then does ONE --release build and shoots an A/B pair from that
# single binary. `VOXELFORGE_ATLAS_MESH=1` pins the pre-fix single-atlas mesher
# back on, so both frames share seed, map, camera and timing — the mesher is the
# only variable.
#
# Never pipes cargo (a pipe launders a failed build into exit 0).
set -uo pipefail
cd "$(dirname "$0")/.."

OUT=docs/assets/split-mesh
LOGS=_poppy_proof
mkdir -p "$OUT" "$LOGS"

# ---- 1. wait for the box ---------------------------------------------------
echo "=== waiting for a free box (no cargo/rustc/link) ==="
CLEAR=0
for i in $(seq 1 360); do   # 360 x 20s = 2 h cap
  if tasklist 2>/dev/null | grep -qiE 'cargo\.exe|link\.exe'; then
    CLEAR=0
  else
    CLEAR=$((CLEAR + 1))
    [ "$CLEAR" -ge 2 ] && break
  fi
  sleep 20
done
if [ "$CLEAR" -lt 2 ]; then
  echo "TIMEOUT — a build still held the box after 2 h"
  exit 1
fi
echo "BOX_FREE after ${i} checks"

# ---- 2. one release build --------------------------------------------------
echo "=== cargo build --release --bin voxelforge ==="
bash scripts/build_safe.sh build --release --bin voxelforge 2>&1 | tee "$LOGS/split-mesh-build.log"
rc=${PIPESTATUS[0]}
echo "BUILD_EXIT=$rc"
if [ "$rc" -ne 0 ]; then
  echo "BUILD FAILED — first errors:"
  grep -n '^error' "$LOGS/split-mesh-build.log" | head -20
  exit "$rc"
fi
grep -c '^warning' "$LOGS/split-mesh-build.log" | sed 's/^/warnings=/'

BIN=./target/release/voxelforge.exe
[ -x "$BIN" ] || { echo "NO BINARY at $BIN"; exit 1; }

# ---- 3. the A/B shots ------------------------------------------------------
unset VOXELFORGE_MAP_LOAD
fails=0

shot() { # <png> <look_cam|-> <atlas 0|1>
  local png="$1" cam="$2" atlas="$3"
  local log="$LOGS/$(basename "$png").log"
  echo "=== $png  cam=$cam atlas=$atlas ==="
  (
    export VOXELFORGE_SHOT="$png"
    [ "$cam" != "-" ] && export VOXELFORGE_LOOK_CAM="$cam"
    [ "$atlas" = "1" ] && export VOXELFORGE_ATLAS_MESH=1
    "$BIN" --play
  ) >"$log" 2>&1
  local rc=$?
  grep -E 'MESH_PATH|MAP_LOAD ok|SPAWN_GROUND|SCENE_READY|SHOT saved' "$log" || true
  local bytes=0
  [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  echo "exit=$rc bytes=$bytes"

  local bad=""
  [ "$rc" -ne 0 ]                    && bad="$bad exit=$rc"
  [ "$bytes" -lt 1024 ]              && bad="$bad no-png"
  grep -qE 'panicked|B0001' "$log"   && bad="$bad PANIC"
  grep -q  'SCENE_READY'    "$log"   || bad="$bad no-SCENE_READY"
  grep -qE 'SPAWN_GROUND.*FAIL' "$log" && bad="$bad spawn-over-air"
  if [ -n "$bad" ]; then
    echo "  x FAIL:$bad"
    grep -nE 'panicked|B0001|error' "$log" | head -5
    fails=$((fails + 1))
  else
    echo "  ok PASS"
  fi
}

# Standing pose (the framing playable-boot.png uses) + a short-boom close-up of
# the ground/wall right in front of the avatar, where a stretched tile is
# unmissable.
#
# NOTE on the close-up: in `--play`, `scene::place_player` re-poses the boom to
# the campsite yaw/WAKE_PITCH after `setup`, so only LOOK_CAM's THIRD number
# (the boom length, which survives via `OrbitCam::want_dist`) changes the frame.
# The yaw/pitch fields are there to satisfy the 3-float parse, not to aim.
shot "$OUT/stand-split.png" -           0
shot "$OUT/stand-atlas.png" -           1
shot "$OUT/close-split.png" "0,0,2.4"   0
shot "$OUT/close-atlas.png" "0,0,2.4"   1

# A pinned mesher that renders the same bytes as the split path means the env
# lever (or the wiring) is dead — that is the exact failure this proof exists
# to catch, so it is a hard fail, not a note.
for pair in stand close; do
  a="$OUT/$pair-split.png"; b="$OUT/$pair-atlas.png"
  if [ -f "$a" ] && [ -f "$b" ]; then
    if cmp -s "$a" "$b"; then
      echo "  x FAIL: $pair split/atlas byte-identical — mesher swap had no effect"
      fails=$((fails + 1))
    else
      echo "  ok $pair split != atlas ($(wc -c <"$a" | tr -d ' ') vs $(wc -c <"$b" | tr -d ' ') bytes)"
    fi
  fi
done

echo
[ "$fails" -eq 0 ] && echo "SPLIT_MESH_PROOF: PASS" || echo "SPLIT_MESH_PROOF: FAIL ($fails)"
exit "$fails"
