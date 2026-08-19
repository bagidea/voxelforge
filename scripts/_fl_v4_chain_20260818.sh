#!/usr/bin/env bash
# ===========================================================================
# Flamingo — v4 look pass: the whole chain, detached.
#
#   wait for a FRESH exe  ->  shoot 3 plates (v3 vs v4, one binary)
#   ->  de-HUD  ->  side-by-side sheets  ->  measured table
#
# WHY DETACHED AND NOT "poll from the session"
#   I already lost one session to sitting on a poll loop for 300 s with nothing
#   on stdout — the watchdog cut it and the work died with it. So the waiting
#   lives in a script that prints a timestamped line every 30 s and survives the
#   session, and the session only reads its log.
#
# FRESHNESS = mtime AND size-stability. A link that is still writing gives a
#   file that exists, has a new mtime, and is truncated — running it is how a
#   "before/after" ends up being a crash log. Two identical sizes 30 s apart is
#   the cheapest honest proof the linker let go.
#
# USAGE  scripts/_fl_v4_chain_20260818.sh [max_wait_secs]
# ===========================================================================
set -u
cd "$(dirname "$0")/.."
ROOT="$PWD"
OUT="$ROOT/_fl_v4_20260818"
mkdir -p "$OUT"
MAXW="${1:-5400}"

stamp() { date "+[%H:%M:%S]"; }
mt() { stat -c %Y "$1" 2>/dev/null || echo 0; }
sz() { stat -c %s "$1" 2>/dev/null || echo 0; }

newest_src=0
for f in client/src/look.rs client/src/hero.rs client/src/voxel.rs; do
  t=$(mt "$f"); [ "$t" -gt "$newest_src" ] && newest_src=$t
done
echo "$(stamp) newest edited source: $(date -d @"$newest_src" '+%H:%M:%S')  (an exe older than this cannot contain the pass)"

pick_play_exe() {
  local best="" bestt=0 c t
  for c in target-poppy/release/voxelforge.exe target/release/voxelforge.exe; do
    [ -f "$c" ] || continue
    t=$(mt "$c")
    [ "$t" -lt "$newest_src" ] && continue
    if [ "$t" -gt "$bestt" ]; then bestt=$t; best=$c; fi
  done
  echo "$best"
}

# ---- wait, loudly ---------------------------------------------------------
start=$(date +%s)
prev_size=-1
EXE=""
while :; do
  now=$(date +%s)
  cand=$(pick_play_exe)
  if [ -n "$cand" ]; then
    s=$(sz "$cand")
    if [ "$s" -gt 0 ] && [ "$s" -eq "$prev_size" ]; then
      EXE="$cand"
      echo "$(stamp) FRESH+STABLE: $cand  $s bytes"
      break
    fi
    echo "$(stamp) waiting — $cand exists, size $s (prev $prev_size), need one stable reading"
    prev_size=$s
  else
    echo "$(stamp) waiting — no voxelforge.exe newer than the source yet"
    echo "$(stamp)   target-poppy/release/deps touched $(date -d @"$(mt target-poppy/release/deps)" '+%H:%M:%S')  rustc procs: $(tasklist //FI 'IMAGENAME eq rustc.exe' 2>/dev/null | grep -c rustc.exe)"
  fi
  if [ $((now - start)) -ge "$MAXW" ]; then
    echo "$(stamp) GAVE UP after ${MAXW}s — no fresh voxelforge.exe. Nothing shot."
    exit 3
  fi
  sleep 30
done

# ---- shoot ----------------------------------------------------------------
echo "$(stamp) === SHOOT ==="
bash scripts/_fl_v4_shoot_20260818.sh "$EXE" || { echo "$(stamp) shoot failed"; exit 4; }

# ---- sheets + numbers -----------------------------------------------------
echo "$(stamp) === SHEETS + MEASURE ==="
python scripts/_fl_v4_sheet_20260818.py --plates "$OUT/plates"
P="$OUT/plates"
python scripts/_fl_v4_grade_20260818.py measure \
  "outdoor-noon_v3=$P/outdoor-noon_before-nohud2.png" \
  "outdoor-noon_v4=$P/outdoor-noon_after-nohud2.png" \
  "evening-raking_v3=$P/evening-raking_before-nohud2.png" \
  "evening-raking_v4=$P/evening-raking_after-nohud2.png" \
  "night-firelit_v3=$P/night-firelit_before-nohud2.png" \
  "night-firelit_v4=$P/night-firelit_after-nohud2.png" \
  --out v4-vs-v3

echo "$(stamp) === CHAIN DONE ==="
