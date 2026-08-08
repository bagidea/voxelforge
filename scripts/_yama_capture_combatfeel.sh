#!/usr/bin/env bash
# _yama_capture_combatfeel.sh — one-shot capture rig for the parry-ring /
# riposte-ring / cam-kick-through-hitstop combat-feel patch.
#
# Runs the real game (--play, VOXELFORGE_DODGE_PROBE=1) which scripts a real
# fight through real ButtonInput and drives every one of: a landed parry, a
# missed parry, a riposte (Critical), a dodge on time, a dodge late — see
# client/src/dodge_parry.rs::dodge_parry_probe / evidence_complete.
#
# While it runs:
#   - stdout (the FEEL_* lines) is tailed and every line gets a wall-clock
#     timestamp, so a FEEL_PARRY / FEEL_RIPOSTE line can be mapped to a video
#     offset later (there is no timestamp in the line itself, only a frame
#     counter with no fixed fps mapping).
#   - the window is captured continuously via ffmpeg gdigrab at 60fps, so the
#     ~100ms ring / ~170ms hitstop windows land on multiple frames each.
#
# Output: _yama_combatfeel/capture_<ts>.mkv + probe_<ts>.log + probe_<ts>.ts.log
set -uo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_DIR"
EXE="$PROJECT_DIR/target-yamamoto/debug/voxelforge.exe"
OUT="$PROJECT_DIR/_yama_combatfeel"
mkdir -p "$OUT"
TS=$(date +%Y%m%d-%H%M%S)
RAWLOG="$OUT/probe_${TS}.log"
TSLOG="$OUT/probe_${TS}.ts.log"
RAW_MKV="$OUT/capture_${TS}.mkv"
FPS=60
RUN_SECS=90
WINDOW_POLL_MAX=20

die() { echo "FATAL: $*" >&2; exit 1; }

[ -f "$EXE" ] || die "$EXE does not exist — build first"

resolve_title() {
  powershell.exe -NoProfile -Command '
    $p = Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($p -and $p.MainWindowTitle) { Write-Output $p.MainWindowTitle } else { Write-Output "" }
  ' 2>/dev/null | tr -d '\r'
}

if tasklist /FI "IMAGENAME eq voxelforge.exe" 2>/dev/null | grep -q "voxelforge.exe"; then
  echo "WARNING: voxelforge.exe already running — killing stale instance"
  taskkill /F /IM voxelforge.exe 2>/dev/null || true
  sleep 1
fi

: > "$RAWLOG"

echo "=== LAUNCH GAME (--play, VOXELFORGE_DODGE_PROBE=1) ==="
( VOXELFORGE_DODGE_PROBE=1 VOXELFORGE_FEEL_LOG=1 timeout "$RUN_SECS" "$EXE" --play >"$RAWLOG" 2>&1 ) &
GAME_PID=$!
echo "game PID: $GAME_PID"

# Timestamp every line as it lands, concurrently.
( tail -n +1 -F "$RAWLOG" 2>/dev/null | while IFS= read -r line; do
    printf '%s %s\n' "$(date +%s.%N)" "$line"
  done > "$TSLOG" ) &
TAIL_PID=$!

echo "waiting for game window title (max ${WINDOW_POLL_MAX}s)..."
WIN_TITLE=""
for ((i=0; i<WINDOW_POLL_MAX; i++)); do
  sleep 1
  WIN_TITLE=$(resolve_title)
  if [ -n "$WIN_TITLE" ]; then
    echo "window found after ${i}s: \"$WIN_TITLE\""
    break
  fi
  if ! kill -0 "$GAME_PID" 2>/dev/null; then
    echo "game exited before window appeared (check $RAWLOG)"
    break
  fi
done
[ -n "$WIN_TITLE" ] || { kill "$TAIL_PID" 2>/dev/null; die "could not resolve game window title"; }

echo "=== RECORD (ffmpeg gdigrab @ ${FPS}fps) ==="
FFMPEG_T0=$(date +%s.%N)
echo "ffmpeg_t0=$FFMPEG_T0" > "$OUT/timing_${TS}.txt"
timeout $((RUN_SECS + 10)) ffmpeg -y -hide_banner -loglevel warning \
  -f gdigrab -framerate "$FPS" -draw_mouse 0 \
  -i title="$WIN_TITLE" \
  -c:v libx264 -preset ultrafast -crf 18 -pix_fmt yuv420p \
  "$RAW_MKV" &
FFMPEG_PID=$!
echo "ffmpeg PID: $FFMPEG_PID  t0=$FFMPEG_T0"

echo "=== WAIT FOR GAME (probe stops itself on evidence_complete) ==="
wait "$GAME_PID" 2>/dev/null || true
GAME_EXIT=$?
echo "game exited with code: $GAME_EXIT"

sleep 2
kill "$FFMPEG_PID" 2>/dev/null || true
wait "$FFMPEG_PID" 2>/dev/null || true
kill "$TAIL_PID" 2>/dev/null || true
echo "ffmpeg stopped"

[ -f "$RAW_MKV" ] && [ -s "$RAW_MKV" ] || die "no video captured"
echo "capture: $RAW_MKV ($(stat -c %s "$RAW_MKV") bytes)"
echo "raw log: $RAWLOG"
echo "ts log:  $TSLOG"
echo "ffmpeg_t0: $FFMPEG_T0"
echo "DONE ts=$TS"
