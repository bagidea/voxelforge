#!/usr/bin/env bash
# Poppy colour-pullback — pd-baked overshoots golden's global warmth (+138 vs golden
# +118) => monochrome-orange collapse (grey fridge / green / cream all dyed one hue).
# The midtone axes have big margin (warmth 126 vs 110, blue 1.2 vs 10, sat 98 vs 90),
# so pull the global orange DOWN toward golden's level to restore material variety
# WITHOUT dropping below the midtone gates. Levers: BLUESCALE up (let blue back so
# steel/green survive), AMBIENT down (less honey dye), GRADE sat down toward 1.0.
# Reviewer-signed framing kept. Env-only. Verifies exit + mtime; new files only.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1

run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return 1; fi
  if [[ -f "$out" ]]; then echo "OK   $out ($(stat -c%s "$out")b)"; else echo "MISS $out"; return 1; fi
}

# baked: BLUESCALE 0.50, AMBIENT 4200, GRADE 0.10,1.02,1.30, EXPOSURE 9.5
run pb1.png VOXELFORGE_BLUESCALE=0.68 VOXELFORGE_AMBIENT=3400 VOXELFORGE_GRADE=0.08,0.98,1.28
run pb2.png VOXELFORGE_BLUESCALE=0.78 VOXELFORGE_AMBIENT=3000 VOXELFORGE_GRADE=0.06,0.96,1.26
run pb3.png VOXELFORGE_BLUESCALE=0.60 VOXELFORGE_AMBIENT=3700 VOXELFORGE_GRADE=0.10,1.00,1.30
run pb4.png VOXELFORGE_BLUESCALE=0.88 VOXELFORGE_AMBIENT=2700 VOXELFORGE_GRADE=0.05,0.94,1.24
echo "ALL_DONE"
