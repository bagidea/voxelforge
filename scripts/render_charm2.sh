#!/usr/bin/env bash
# Charm pass 2 — env-only, chase golden's TONAL SEPARATION (not global desaturation).
# Golden's sunlit patch is punchy (R-B+133,L88) while its shade stays neutral-warm,
# giving contrast; ours reads flat mono-orange (R-B+50,L75). So: punch the key sun +
# trim the ambient wash a touch (we have headroom: p05 11.5% vs golden 9.9%) to widen
# the tonal range WITHOUT crushing the warm shadow floor. Zero geometry/identity change.
# Verifies exit code + PNG mtime. New files only — never touches converged-final.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1
CAM=7.6,5.9,-5.2,7.6,3.2,6.0,52
COMMON="VOXELFORGE_CAM=$CAM VOXELFORGE_FOG=0.032 VOXELFORGE_DFOG=0.008 VOXELFORGE_DOF=11,3.2"

run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return 1; fi
  [[ -f "$out" ]] && echo "OK   $out ($(stat -c%s "$out")b, $(stat -c%y "$out"))" || { echo "MISS $out"; return 1; }
}

# A: punch sun, trim ambient wash for tonal range
run charm2-A.png $COMMON VOXELFORGE_SUN=20,195,17000 VOXELFORGE_AMBIENT=2500 VOXELFORGE_EXPOSURE=9.7
# B: same punch, hold ambient near golden p05, slightly brighter key still
run charm2-B.png $COMMON VOXELFORGE_SUN=18,196,20000 VOXELFORGE_AMBIENT=2600 VOXELFORGE_EXPOSURE=9.8
# C: strongest key + lowest wash (max separation, guard against crush)
run charm2-C.png $COMMON VOXELFORGE_SUN=18,196,22000 VOXELFORGE_AMBIENT=2400 VOXELFORGE_EXPOSURE=9.9
echo "ALL_DONE"
