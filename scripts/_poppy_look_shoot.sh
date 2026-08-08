#!/usr/bin/env bash
# Poppy — re-shoot the two vista frames of the look audit through the exe that
# was just rebuilt, with NO look overrides at all.
#
# The previous round's numbers were void because the frames predated the sky /
# exposure commit. Two rules follow from that:
#   1. every VOXELFORGE_LOOK_* tuning knob is explicitly unset, so what lands on
#      disk is the SHIPPED default, not a sweep row that happens to look good;
#   2. the env the binary actually saw is dumped by the very process that execs
#      it (`env | grep ^VOXELFORGE` from inside that process), so the claim
#      "no exposure override" is evidence rather than a promise.
set -uo pipefail
cd "$(dirname "$0")/.."
EXE=$(cygpath -w "$PWD/target/release/voxelforge.exe" 2>/dev/null || echo "$PWD/target/release/voxelforge.exe")
OUT="$PWD/_flamingo_look_audit"
[[ -f "target/release/voxelforge.exe" ]] || { echo "NO EXE"; exit 2; }

# Knobs that would make the frame something other than the shipped look.
UNSET=(-u VOXELFORGE_LOOK_EXPOSURE -u VOXELFORGE_LOOK_SKY -u VOXELFORGE_LOOK_AMBIENT
       -u VOXELFORGE_LOOK_SUN -u VOXELFORGE_LOOK_GRADE -u VOXELFORGE_LOOK_LIGHT
       -u VOXELFORGE_LOOK_FOG -u VOXELFORGE_LOOK_NIGHT -u VOXELFORGE_LOOK_FORCE
       -u VOXELFORGE_LOOK_DISABLE -u VOXELFORGE_BOOM_TRACE -u VOXELFORGE_BOOM_WALK)

shoot() { # name  extra-env...
  local name=$1; shift
  local png="$OUT/$name.png"
  rm -f "$png"
  echo "--- $name"
  env "${UNSET[@]}" VOXELFORGE_PLAY=1 VOXELFORGE_LOOK_CAM=35,-18,26 \
      VOXELFORGE_SHOT="$png" "$@" \
      bash -c 'env | grep "^VOXELFORGE" | sort > "$1"; exec "$2"' _ \
        "$OUT/$name.env.txt" "$EXE" \
        > "$OUT/$name.out.log" 2> "$OUT/$name.err.log"
  local code=$?
  if [[ -f "$png" ]]; then
    echo "OK   $png ($(stat -c%s "$png")b) exit=$code"
    python - "$png" <<'PY'
import sys
from PIL import Image
print("     size:", Image.open(sys.argv[1]).size)
PY
  else
    echo "MISS $png exit=$code"; tail -5 "$OUT/$name.err.log"
  fi
  echo "     env the binary saw:"; sed 's/^/       /' "$OUT/$name.env.txt"
}

shoot look-on-vista
shoot look-ultra-vista VOXELFORGE_LOOK_QUALITY=ultra

echo "=== de-HUD ==="
python scripts/_flamingo_dehud2.py "$OUT/look-on-vista.png" "$OUT/look-ultra-vista.png"
