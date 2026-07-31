#!/usr/bin/env bash
# Look post-stack frame-time probe (Poppy's perf lane).
#
# Three phases, each answering a different question Rose has to decide with:
#   1. leave-one-out at Ultra  -> what does each individual effect cost?
#   2. tier ladder             -> what does each shipped tier cost, end to end?
#   3. sensor_height sweep     -> is look.rs's DoF constant a frame-time knob?
#
# Every phase runs TWO ROUNDS in interleaved order, because a single pass down
# the list lets the GPU heat up under the later entries and quietly charges
# thermal drift to whichever one happened to run last. Reporting both rounds
# makes that visible instead of averaging it away.
#
# VSync is forced off inside the binary — see docs/perf-vsync-cliff-rootcause.md
# for why a VSync'd A/B of this stack would read "free".
#
# Method + how to read the output: docs/look-perf-methodology.md
#
# Usage: scripts/perf_look_probe.sh [out.log]
set -u

EXE="target-poppy/perf/voxelforge_perf.exe"
OUT="${1:-_poppy_look_perf.log}"
MODES=(off full no_ssao no_vfog no_taa no_dof no_bloom no_pcss)
TIERS=(low medium high ultra)

if [ ! -x "$EXE" ]; then
  echo "MISSING $EXE — build it first:" >&2
  echo "  bash scripts/perf_look_build.sh" >&2
  echo "  (that script refuses to start while another lane's cargo is running —" >&2
  echo "   a concurrent build is what killed rustc with STATUS_DLL_INIT_FAILED twice)" >&2
  exit 2
fi

: > "$OUT"

# Phase 1 — leave-one-out, pinned to Ultra.
#
# NOT the default tier. look.rs defaults to High, and High carries no
# VolumetricFog — a `no_vfog` run there strips nothing and would report the most
# expensive pass in the stack as free. Ultra is the only tier holding all six
# effects, so it is the only tier where "full minus X" is a real subtraction.
# `VOXELFORGE_LOOK_QUALITY` is look.rs's own override (look.rs:139), so pinning
# it costs no edit to Rose's file.
for round in 1 2; do
  for m in "${MODES[@]}"; do
    line=$(VOXELFORGE_LOOK_QUALITY=ultra VOXELFORGE_PERF="$m" "$EXE" 2>/dev/null | grep '^PERF ')
    if [ -z "$line" ]; then
      echo "round=$round phase=effect mode=$m FAILED (no PERF line)" | tee -a "$OUT"
    else
      echo "round=$round phase=effect $line" | tee -a "$OUT"
    fi
  done
done

# Phase 2 — the tier ladder.
#
# Phase 1 prices each effect; this prices each rung as it actually ships. They
# are not the same question: tiers differ in SSAO *quality level* as well as in
# which effects are present, so a sum of per-effect deltas would miss the
# Low->Medium->Ultra SSAO quality steps entirely.
for round in 1 2; do
  for t in "${TIERS[@]}"; do
    line=$(VOXELFORGE_LOOK_QUALITY="$t" VOXELFORGE_PERF=full "$EXE" 2>/dev/null | grep '^PERF ')
    if [ -z "$line" ]; then
      echo "round=$round phase=tier tier=$t FAILED (no PERF line)" | tee -a "$OUT"
    else
      echo "round=$round phase=tier $line" | tee -a "$OUT"
    fi
  done
done

# Phase 3 — the sensor_height sweep.
#
# This is NOT a look question dressed up as a perf one. Bevy's Bokeh DoF computes
# `coc_scale_factor = focal_length² / (sensor_height · N)` with
# `focal_length = 0.5 · sensor_height / tan(fov/2)`, which reduces to
# `0.25 · sensor_height / (tan²(fov/2) · N)` — LINEAR in sensor_height. The blur
# then runs `support = round(coc/2)` taps per direction (dof.wgsl box_blur_a/_b).
# So sensor_height directly scales the DoF tap count until it hits the 64 px
# `max_circle_of_confusion_diameter` clamp. 0.35 is ~19x the physical default.
#
# Ultra again, for the same reason as phase 1 — and because High and Ultra share
# an identical DoF block, so the shape of this curve transfers to both.
for round in 1 2; do
  for sh in 0.0186 0.10 0.20 0.35 0.50; do
    line=$(VOXELFORGE_LOOK_QUALITY=ultra VOXELFORGE_PERF=full VOXELFORGE_PERF_SENSOR="$sh" "$EXE" 2>/dev/null | grep '^PERF ')
    if [ -z "$line" ]; then
      echo "round=$round phase=sensor sensor=$sh FAILED (no PERF line)" | tee -a "$OUT"
    else
      echo "round=$round phase=sensor sensor=$sh $line" | tee -a "$OUT"
    fi
  done
done

echo "--- wrote $OUT ---"
