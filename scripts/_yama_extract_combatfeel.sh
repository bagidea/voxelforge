#!/usr/bin/env bash
# _yama_extract_combatfeel.sh <ts> — pull before/after evidence frames out of
# the capture produced by _yama_capture_combatfeel.sh.
#
# Correlates FEEL_PARRY (regular parry, HitFlavor::Parry power=1.2 -> ring
# born=0.22, grow_to 3.4) and FEEL_RIPOSTE (Critical, redirected as a Parry
# ring with power=2.6 -> ring born=0.29, ~30% bigger) wall-clock timestamps
# (from the .ts.log) to video offsets (video started at ffmpeg_t0), then pulls
# a burst of frames around each event (the ring's alpha/emissive fade
# (1-t)^2 over its 0.10s ttl means it is brightest in the first ~30-40ms and
# still growing after that — a burst, not a single guessed frame, is what
# survives that timing slop).
set -uo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$PROJECT_DIR/_yama_combatfeel"
TS="${1:?usage: _yama_extract_combatfeel.sh <ts>}"
RAW_MKV="$OUT/capture_${TS}.mkv"
TSLOG="$OUT/probe_${TS}.ts.log"
TIMING="$OUT/timing_${TS}.txt"
FRAMES_DIR="$OUT/frames_${TS}"
mkdir -p "$FRAMES_DIR"

[ -f "$RAW_MKV" ] || { echo "FATAL: $RAW_MKV missing"; exit 1; }
[ -f "$TSLOG" ] || { echo "FATAL: $TSLOG missing"; exit 1; }
[ -f "$TIMING" ] || { echo "FATAL: $TIMING missing"; exit 1; }

T0=$(sed -nE 's/^ffmpeg_t0=//p' "$TIMING")
echo "ffmpeg_t0=$T0"

# burst_around <label> <event_epoch> <lead_s> <span_s> <fps>
burst_around() {
  local label="$1" ev="$2" lead="$3" span="$4" fps="$5"
  local off
  off=$(awk -v ev="$ev" -v t0="$T0" -v lead="$lead" 'BEGIN{ v=ev-t0-lead; if (v<0) v=0; printf "%.3f", v }')
  echo "--- $label: event=$ev offset=${off}s span=${span}s @ ${fps}fps ---"
  ffmpeg -y -hide_banner -loglevel error -ss "$off" -i "$RAW_MKV" -t "$span" -vf "fps=$fps" \
    "$FRAMES_DIR/${label}_%03d.png"
}

single_at() {
  local label="$1" ev="$2" offset_from_event="$3"
  local off
  off=$(awk -v ev="$ev" -v t0="$T0" -v d="$offset_from_event" 'BEGIN{ v=ev-t0+d; if (v<0) v=0; printf "%.3f", v }')
  echo "--- $label: offset=${off}s ---"
  ffmpeg -y -hide_banner -loglevel error -ss "$off" -i "$RAW_MKV" -frames:v 1 "$FRAMES_DIR/${label}.png"
}

# ── first regular parry (FEEL_PARRY, not FEEL_PARRY_FAIL / not riposte) ──
PARRY_LINE=$(grep -a ' FEEL_PARRY ' "$TSLOG" | head -1)
if [ -n "$PARRY_LINE" ]; then
  PARRY_T=$(echo "$PARRY_LINE" | awk '{print $1}')
  echo "PARRY_EVENT t=$PARRY_T :: $PARRY_LINE"
  single_at parry_before "$PARRY_T" -0.15
  burst_around parry_after "$PARRY_T" 0.0 0.30 60
else
  echo "WARNING: no FEEL_PARRY line found in $TSLOG"
fi

# ── first riposte (FEEL_RIPOSTE, weight=critical) ──
RIPOSTE_LINE=$(grep -a 'FEEL_RIPOSTE' "$TSLOG" | head -1)
if [ -n "$RIPOSTE_LINE" ]; then
  RIPOSTE_T=$(echo "$RIPOSTE_LINE" | awk '{print $1}')
  echo "RIPOSTE_EVENT t=$RIPOSTE_T :: $RIPOSTE_LINE"
  single_at riposte_before "$RIPOSTE_T" -0.15
  burst_around riposte_after "$RIPOSTE_T" 0.0 0.30 60
  # cam-kick: crit hitstop=0.170s. Capture the kick right as it lands and
  # again right as hitstop should be releasing, to show the offset held
  # rather than decayed while the game was frozen.
  single_at camkick_atland "$RIPOSTE_T" 0.02
  single_at camkick_athitstopend "$RIPOSTE_T" 0.18
  single_at camkick_after "$RIPOSTE_T" 0.35
else
  echo "WARNING: no FEEL_RIPOSTE line found in $TSLOG"
fi

echo "DONE — frames in $FRAMES_DIR"
ls -la "$FRAMES_DIR"
