#!/usr/bin/env bash
# Poppy sun-elevation sweep — the fg zone is DARK because the low 20-deg sun is blocked
# by the island, so its shadow falls onto the near-floor (grader fg zone). Raising the
# sun elevation shortens that shadow and rakes light onto the foreground => brighter,
# higher-contrast fg planks (fg-hf up => DOF axis) AND a real lit/shade gradient like
# golden (breaks the flat orange wash). Paired with pb3's pulled-back colour. Env-only.
# Verifies exit + mtime; new files only.
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

# baked sun = 20,195,12000 (low, shadows the near floor)
run se1.png $PB VOXELFORGE_SUN=35,195,12000
run se2.png $PB VOXELFORGE_SUN=45,200,14000
run se3.png $PB VOXELFORGE_SUN=30,180,12000
run se4.png $PB VOXELFORGE_SUN=40,165,13000
echo "ALL_DONE"
