#!/usr/bin/env bash
# Look-tier frame-time bench (Rose) — the numbers Shiba's Settings menu cites.
#
# This is Rose's measurement runner for the look lane. It drives the same probe
# Poppy's perf_look_probe.sh does, but:
#   - builds/runs out of target-rose (NOT target-poppy, NOT target/) per the
#     Director's GPU-lane partitioning,
#   - reads the probe's fps + fps_1pct_low fields (Poppy's probe printed p50/p95
#     only; this probe adds the 1%-low a settings menu needs),
#   - refuses to run until Yamamoto's Gate 3 is done (>=3 PNGs in
#     docs/assets/gate3) AND no voxelforge.exe is resident — the probe renders a
#     window, so it shares the GPU with the shoot and must not contend.
#
# Three phases (mirrors perf_look_probe.sh so the methodology doc's tables fill
# from the same run):
#   1. leave-one-out at Ultra  -> per-effect cost
#   2. tier ladder             -> THE table the Settings menu cites
#   3. sensor_height sweep     -> is look.rs's DoF constant a frame-time knob?
# Each phase runs TWO interleaved rounds (thermal-drift guard from Poppy's method).
#
# Output: raw PERF lines to the log, PLUS a parsed tier summary (mean of the two
# rounds: fps / fps_1pct_low / median_ms) printed to stdout and the log.
#
# Usage: scripts/rose_look_tier_bench.sh [out.log]
set -u

EXE="target-rose/perf/voxelforge_perf.exe"
OUT="${1:-_rose_look_tier_bench.log}"
MODES=(off full no_ssao no_vfog no_taa no_dof no_bloom no_pcss)
TIERS=(low medium high ultra)
SENSORS=(0.0186 0.10 0.20 0.35 0.50)

# --- GPU gate: do not contend with Gate 3 ----------------------------------
png_count=$(ls docs/assets/gate3/*.png 2>/dev/null | wc -l | tr -d '[:space:]')
if [ "${png_count:-0}" -lt 3 ]; then
  echo "REFUSING TO RUN — Gate 3 has only ${png_count:-0} PNG(s) in docs/assets/gate3" >&2
  echo "(need >=3; Yamamoto's shoot is still using the GPU). Re-run when it's done." >&2
  exit 4
fi
if tasklist 2>/dev/null | grep -qi "voxelforge.exe"; then
  echo "REFUSING TO RUN — voxelforge.exe is still running (GPU busy)." >&2
  exit 4
fi

if [ ! -x "$EXE" ]; then
  echo "MISSING $EXE — build it first:" >&2
  echo "  cargo build -p voxelforge --bin voxelforge_perf --profile perf --target-dir target-rose -j 2" >&2
  exit 2
fi

: > "$OUT"

run_probe() {  # $1=env assignments   echoes the PERF line or FAILED marker
  local assignments="$1"
  local line
  line=$(env $assignments "$EXE" 2>/dev/null | grep '^PERF ')
  if [ -z "$line" ]; then echo "FAILED (no PERF line)"; else echo "$line"; fi
}

echo "=== rose_look_tier_bench — $(date 2>/dev/null || echo 'undated') ===" | tee -a "$OUT"

# Phase 1 — leave-one-out, pinned Ultra (only tier with all six effects).
for round in 1 2; do
  for m in "${MODES[@]}"; do
    out=$(run_probe "VOXELFORGE_LOOK_QUALITY=ultra VOXELFORGE_PERF=$m")
    echo "round=$round phase=effect mode=$m $out" | tee -a "$OUT"
  done
done

# Phase 2 — the tier ladder (the Settings-menu table).
for round in 1 2; do
  for t in "${TIERS[@]}"; do
    out=$(run_probe "VOXELFORGE_LOOK_QUALITY=$t VOXELFORGE_PERF=full")
    echo "round=$round phase=tier tier=$t $out" | tee -a "$OUT"
  done
done

# Phase 3 — sensor_height sweep (Ultra).
for round in 1 2; do
  for sh in "${SENSORS[@]}"; do
    out=$(run_probe "VOXELFORGE_LOOK_QUALITY=ultra VOXELFORGE_PERF=full VOXELFORGE_PERF_SENSOR=$sh")
    echo "round=$round phase=sensor sensor=$sh $out" | tee -a "$OUT"
  done
done

# --- Parsed tier summary (mean of the two rounds) -------------------------
# Reads phase=tier lines, averages fps + fps_1pct_low + median_ms across rounds.
echo "" | tee -a "$OUT"
echo "--- tier summary (mean of 2 rounds; fps=1000/p50, fps_1pct_low=1000/mean-of-worst-1%) ---" | tee -a "$OUT"
awk '
/^round=[0-9]+ phase=tier/ {
  tier=""; fps=0; low=0; med=0;
  for (i=1;i<=NF;i++) {
    if ($i ~ /^tier=/)           tier=tolower(substr($i,6));
    if ($i ~ /^fps=/)            fps=substr($i,5)+0;
    if ($i ~ /^fps_1pct_low=/)   low=substr($i,14)+0;
    if ($i ~ /^median_ms=/)      med=substr($i,11)+0;
  }
  if (tier!="") {
    n[tier]++;  sumfps[tier]+=fps;  sumlow[tier]+=low;  summed[tier]+=med;
  }
}
END {
  printf "| tier | fps (1000/p50) | fps 1%% low | median ms |\n";
  printf "|---|---|---|---|\n";
  # preserve ladder order low->medium->high->ultra
  nt=split("low medium high ultra", order, " ");
  for (i=1;i<=nt;i++) {
    t=order[i];
    if (n[t]>0)
      printf "| %s | %.1f | %.1f | %.2f |\n", t, sumfps[t]/n[t], sumlow[t]/n[t], summed[t]/n[t];
    else
      printf "| %s | _no data_ | _no data_ | _no data_ |\n", t;
  }
}
' "$OUT" | tee -a "$OUT"

echo "--- wrote $OUT ---"
