#!/usr/bin/env bash
# _sun_capture_combo.sh — capture the 3-hit light chain + finisher as MP4.
#
# Runs the real game (--play, VOXELFORGE_COMBO_PROBE=1) which drives the combo
# state machine through real ButtonInput<KeyCode>: L1 -> L2 -> L3 -> finisher,
# on repeat. See client/src/combat.rs::feel_probe (combo_mode) for the cadence.
#
# While it runs, the window is captured via ffmpeg gdigrab at 60fps. The probe
# auto-exits at its own PROBE_END (30s), so the game closes itself.
#
# Output: _sun_combo/capture_<ts>.mkv + probe_<ts>.log
set -uo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_DIR"
EXE="$PROJECT_DIR/target-sun/release/voxelforge.exe"
OUT="$PROJECT_DIR/_sun_combo"
mkdir -p "$OUT"
TS=$(date +%Y%m%d-%H%M%S)
RAWLOG="$OUT/probe_${TS}.log"
RAW_MKV="$OUT/capture_${TS}.mkv"
FPS=60
WINDOW_POLL_MAX=25

die() { echo "FATAL: $*" >&2; exit 1; }

[ -f "$EXE" ] || die "$EXE does not exist — build first (Sentinel, --target-dir target-sun)"

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

echo "=== LAUNCH GAME (--play, VOXELFORGE_COMBO_PROBE=1) ==="
( VOXELFORGE_COMBO_PROBE=1 VOXELFORGE_FEEL_LOG=1 "$EXE" --play >"$RAWLOG" 2>&1 ) &
GAME_PID=$!
echo "game PID: $GAME_PID"

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
[ -n "$WIN_TITLE" ] || die "could not resolve game window title (see $RAWLOG)"

echo "=== RECORD (ffmpeg gdigrab @ ${FPS}fps) ==="
timeout 60 ffmpeg -y -hide_banner -loglevel warning \
  -f gdigrab -framerate "$FPS" -draw_mouse 0 \
  -i title="$WIN_TITLE" \
  -c:v libx264 -preset ultrafast -crf 18 -pix_fmt yuv420p \
  "$RAW_MKV" &
FFMPEG_PID=$!
echo "ffmpeg PID: $FFMPEG_PID"

echo "=== WAIT FOR GAME (probe auto-exits at its PROBE_END) ==="
wait "$GAME_PID" 2>/dev/null || true
GAME_EXIT=$?
echo "game exited with code: $GAME_EXIT"

sleep 2
kill "$FFMPEG_PID" 2>/dev/null || true
wait "$FFMPEG_PID" 2>/dev/null || true
echo "ffmpeg stopped"

[ -f "$RAW_MKV" ] && [ -s "$RAW_MKV" ] || die "no video captured"

DUR=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$RAW_MKV" 2>/dev/null | tr -d '\r')
SIZE=$(stat -c %s "$RAW_MKV")
echo "capture: $RAW_MKV ($SIZE bytes, ${DUR}s)"
echo "raw log: $RAWLOG"
echo "DONE ts=$TS"
