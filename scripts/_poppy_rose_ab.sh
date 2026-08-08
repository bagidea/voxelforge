#!/usr/bin/env bash
# What did Rose's quality push cost in frame time? (bb5c21e: ambient_lux
# 1100 -> 2200, PCSS_WIDTH 3.0 -> 4.0)
#
# ONE BINARY, BOTH SIDES. `look.rs` already exposes both knobs at runtime
# (`VOXELFORGE_LOOK_AMBIENT`, `VOXELFORGE_LOOK_PCSS`), so the before/after does
# not need two builds -- and more importantly it does not need me to edit
# look.rs, which is Rose's file (docs/LANES.md). Every number below comes out of
# the same exe on the same scene at the same resolution; only the two knobs move.
#
# PCSS IS TIER-GATED AND THE ENV OVERRIDE IS NOT. `look.rs:1353` turns PCSS on
# for Ultra only, but `pcss_width()` returns `Some(v)` for ANY tier once the env
# var parses. So passing `3`/`4` at High would measure a stack the game never
# ships. Below Ultra both sides therefore pass `off`, which is what the tier
# itself does -- the only thing that moves at those tiers is ambient_lux.
#
# WHY ROUND 1 IS THROWN AWAY: the 560.94 driver compiles Vulkan shaders on first
# use of each permutation, and a contaminated round 1 once read Medium as 4x
# slower than High -- i.e. slower than a strict superset of its own work, which
# is impossible. Rounds 2 and 3 are the reported ones; round 1 stays in the log
# so the discard is visible instead of quiet.
#
# Method + how to read the output: docs/look-perf-methodology.md
# Usage: bash scripts/_poppy_rose_ab.sh [out.log] [exe]
set -u
cd "$(dirname "$0")/.."

OUT="${1:-_poppy_look_perf_rose_ab.log}"
EXE="${2:-target/release/voxelforge_perf.exe}"
ROUNDS=3

if [ ! -x "$EXE" ]; then
  echo "MISSING $EXE" >&2
  exit 2
fi

# A render probe fighting another render probe for the GPU is not a measurement.
others=$(powershell -NoProfile -Command \
  "@(Get-CimInstance Win32_Process -Filter \"Name='cargo.exe' or Name='voxelforge.exe' or Name='voxelforge_perf.exe'\").Count" \
  2>/dev/null | tr -d '[:space:]')
if [ "${others:-0}" != "0" ]; then
  echo "REFUSING TO MEASURE — $others cargo/voxelforge process(es) running." >&2
  exit 3
fi

: > "$OUT"
echo "# exe=$EXE sha256=$(sha256sum "$EXE" | cut -d' ' -f1)" | tee -a "$OUT"
echo "# before = ambient 1100 + PCSS 3.0 (HEAD~1) · after = ambient 2200 + PCSS 4.0 (HEAD, bb5c21e)" | tee -a "$OUT"

run() {  # run <round> <label> <tier> <ambient> <pcss>
  local round="$1" label="$2" tier="$3" amb="$4" pcss="$5"
  local line
  line=$(VOXELFORGE_LOOK_QUALITY="$tier" \
         VOXELFORGE_LOOK_AMBIENT="$amb" \
         VOXELFORGE_LOOK_PCSS="$pcss" \
         VOXELFORGE_PERF=full "$EXE" 2>/dev/null | grep '^PERF ')
  if [ -z "$line" ]; then
    echo "round=$round cfg=$label tier=$tier amb=$amb pcss=$pcss FAILED (no PERF line)" | tee -a "$OUT"
  else
    echo "round=$round cfg=$label amb=$amb pcss=$pcss $line" | tee -a "$OUT"
  fi
}

for round in $(seq 1 "$ROUNDS"); do
  # Interleaved, not "all of before then all of after": a single pass down the
  # list lets the GPU heat under whatever runs last and charges it the drift.
  for tier in low medium high ultra; do
    if [ "$tier" = "ultra" ]; then
      run "$round" before "$tier" 1100 3
      run "$round" after  "$tier" 2200 4
      # Attribution: which of the two knobs actually costs anything.
      run "$round" amb_only  "$tier" 2200 3
      run "$round" pcss_only "$tier" 1100 4
    else
      run "$round" before "$tier" 1100 off
      run "$round" after  "$tier" 2200 off
    fi
  done
done

echo "# done" | tee -a "$OUT"
