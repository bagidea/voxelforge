#!/usr/bin/env bash
# P0/P1/P2 charm-layer proof render (Flamingo note): warm floor + bowl-on-counter
# margin + tighter stainless streak + softer god-ray. Uses the ISOLATED shot bin so
# it builds hero.rs without main.rs. OOB = baked hero defaults, ZERO env crutch, so
# the frame proves the charm changes reproduce out of the box. Verifies exit + mtime.
set -u
cd "$(dirname "$0")/.."
EXE=target/release/voxelforge_shot.exe

run() {
  local out="$1"; shift
  rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"logs_$out.txt" 2>&1
  local code=$?
  if [[ $code -ne 0 ]]; then echo "FAIL $out exit=$code"; return 1; fi
  if [[ -f "$out" ]]; then echo "OK   $out  ($(stat -c%s "$out") bytes, mtime $(stat -c%y "$out"))";
  else echo "MISS $out — no png (see logs_$out.txt)"; return 1; fi
}

# 1) pure OOB — baked hero defaults, nothing overridden
run charm-oob.png
echo "ALL_DONE"
