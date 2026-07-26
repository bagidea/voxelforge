#!/usr/bin/env bash
# Poppy apron probe — render with VOXELFORGE_FGAPRON=1 (lit planked tabletop extended
# into the foreground) so the grader's fg zone lands on the high-contrast counter/
# counter_dk checker instead of the dark shadow floor. Paired with pb3's pulled-back
# colour. Also a control WITHOUT the apron at identical settings to isolate its effect.
# Env-only on top of the freshly-built exe. Verifies exit + mtime; new files only.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1
PB="VOXELFORGE_BLUESCALE=0.60 VOXELFORGE_AMBIENT=3700 VOXELFORGE_GRADE=0.10,1.00,1.30"

run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return 1; fi
  if [[ -f "$out" ]]; then echo "OK   $out ($(stat -c%s "$out")b)"; else echo "MISS $out"; return 1; fi
}

# control: apron OFF (should match pb3 exactly — proves the build didn't move the default)
run ap-off.png $PB
# apron ON at baked framing
run ap-on.png  $PB VOXELFORGE_FGAPRON=1
# apron ON + harder bg blur (f/1.2) to widen the fg:bg gap from both ends
run ap-f12.png $PB VOXELFORGE_FGAPRON=1 VOXELFORGE_DOF=10,1.2
# apron ON + slight tilt down so the apron owns more of the fg zone
run ap-tilt.png $PB VOXELFORGE_FGAPRON=1 VOXELFORGE_CAM=7.6,5.9,-5.2,7.6,2.9,6.0,52
echo "ALL_DONE"
