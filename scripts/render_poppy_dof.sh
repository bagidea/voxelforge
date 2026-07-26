#!/usr/bin/env bash
# Poppy DOF-verify — render the CURRENT baked source with NO env crutch so we see
# what VOXELFORGE_HERO=1 actually ships (focus 10 / f/1.4 / planked tabletop already
# baked in client/src/hero.rs). Then a couple of controlled DOF sweeps around it.
# Verifies exit code + PNG mtime; new files only (never touches signed frames).
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge.exe
export VOXELFORGE_HERO=1

run() { local out="$1"; shift; rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return 1; fi
  if [[ -f "$out" ]]; then echo "OK   $out ($(stat -c%s "$out")b, $(stat -c%y "$out"))"; else echo "MISS $out"; return 1; fi
}

# 1) Pure baked default — the honest "what ships" frame.
run pd-baked.png
# 2) Tighter aperture sweep around baked focus 10 (does f/1.0 melt bg harder?)
run pd-f1.0.png VOXELFORGE_DOF=10,1.0
run pd-f1.2.png VOXELFORGE_DOF=10,1.2
# 3) Focus pulled a touch nearer the bowl plane (bowl ~ eye-depth; try 9.2 / 9.6)
run pd-fo9.2.png VOXELFORGE_DOF=9.2,1.2
run pd-fo9.6.png VOXELFORGE_DOF=9.6,1.2
echo "ALL_DONE"
