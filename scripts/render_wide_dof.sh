#!/usr/bin/env bash
# WIDE DOF resolve (Pixel). Focus on the NEAR planked tabletop (grader fg zone) at a
# MODERATE aperture (f3-4) so tabletop+bowl stay sharp while the far wall melts →
# fg hi-freq up, bg down. Keep bottom-center = planked WOOD (high-contrast), bowl
# just above center. Grade temp 0.05->0.08 to pull midtone blue under 10.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1 VOXELFORGE_WIDE=1
export VOXELFORGE_BLUESCALE=0.42 VOXELFORGE_AMBIENT=4000 VOXELFORGE_EXPOSURE=9.0
export VOXELFORGE_GRADE=0.08,0.95,1.24
run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?; [[ $code -ne 0 ]] && { echo "FAIL $out exit=$code"; return 1; }
  [[ -f "$out" ]] && echo "OK   $out ($(stat -c%s "$out")b)" || { echo "MISS $out"; return 1; }
}
run wf1.png VOXELFORGE_CAM=8.4,4.8,-0.5,6.9,2.4,9.0,63 VOXELFORGE_DOF=3.5,3.5
run wf2.png VOXELFORGE_CAM=8.4,4.8,-0.5,6.9,2.4,9.0,63 VOXELFORGE_DOF=4.0,3.0
run wf3.png VOXELFORGE_CAM=7.6,4.8,-0.8,7.6,2.4,9.0,64 VOXELFORGE_DOF=3.8,3.5
run wf4.png VOXELFORGE_CAM=8.8,5.0,-1.2,6.8,2.5,9.0,62 VOXELFORGE_DOF=4.2,4.0
echo ALL_DONE
