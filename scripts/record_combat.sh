#!/usr/bin/env bash
# record_combat.sh — rebuild + record --combat-demo as MP4 via ffmpeg gdigrab.
#
# CAPTURE POLICY (v2 — 2026-07-31):
#   ffmpeg -f gdigrab -i title="<exact window title>" — captures ONLY the game
#   window. No desktop, no other apps, no Office Feed, no billing pages.
#   The window title is resolved at runtime from the live process because
#   Bevy/winit appends a dynamic "(NNv0)" suffix.
#
# START POLICY:
#   Game starts FIRST, ffmpeg starts AFTER the window title is confirmed
#   non-empty. This guarantees zero frames of desktop-only content.
#
# VERIFY POLICY:
#   After capture, extracts first/mid/last frames as PNG for manual review
#   before the MP4 is considered complete.
#
# Safeguards (unchanged from v1):
#   - Rebuild before every recording + exe-mtime-vs-.rs check
#   - timeout on every game/ffmpeg command
#   - Kill lingering voxelforge.exe at start and end
#   - Git provenance stamped into output directory
#
# Output: _combat_recordings/combat-YYYYMMDD-HHMMSS.mp4
#         _combat_recordings/verify_<timestamp>_first.png|mid.png|last.png
#
# Usage:
#   bash scripts/record_combat.sh
#   VOXELFORGE_RECORD_FPS=60 bash scripts/record_combat.sh
set -uo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$PROJECT_DIR/target-combat"
EXE="$TARGET_DIR/debug/voxelforge.exe"
OUTPUT_DIR="$PROJECT_DIR/_combat_recordings"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
RAW_MKV="$OUTPUT_DIR/raw_${TIMESTAMP}.mkv"
FINAL_MP4="$OUTPUT_DIR/combat-${TIMESTAMP}.mp4"
LOG="$OUTPUT_DIR/run_${TIMESTAMP}.log"
FPS="${VOXELFORGE_RECORD_FPS:-30}"
GAME_TIMEOUT=28
FFMPEG_TIMEOUT=35
WINDOW_POLL_MAX=20   # seconds to wait for the game window to appear

mkdir -p "$OUTPUT_DIR"

# ── helpers ──────────────────────────────────────────────────────────────────
die() { echo "FATAL: $*" >&2; exit 1; }

# Resolve the exact window title of the running voxelforge.exe.
# Bevy sets "Voxelforge — Phase 0 spike" but winit appends " (NNv0)".
# Returns empty string if no window yet.
resolve_title() {
  powershell.exe -NoProfile -Command '
    $p = Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($p -and $p.MainWindowTitle) { Write-Output $p.MainWindowTitle } else { Write-Output "" }
  ' 2>/dev/null | tr -d '\r'
}

# ── check ffmpeg + gdigrab ───────────────────────────────────────────────────
command -v ffmpeg &>/dev/null || die "ffmpeg not found in PATH"
echo "ffmpeg: $(ffmpeg -version 2>&1 | head -1 | cut -d' ' -f1-3)"

FFMPEG_HAS_GDIGRAB=$(ffmpeg -hide_banner -devices 2>&1 | grep -c "gdigrab" || true)
[ "$FFMPEG_HAS_GDIGRAB" -gt 0 ] || die "ffmpeg built without gdigrab"
echo "gdigrab: available ✓"

# ── kill stale voxelforge ────────────────────────────────────────────────────
if tasklist /FI "IMAGENAME eq voxelforge.exe" 2>/dev/null | grep -q "voxelforge.exe"; then
  echo "WARNING: voxelforge.exe already running — killing stale instance"
  taskkill /F /IM voxelforge.exe 2>/dev/null || true
  sleep 1
fi

# ── rebuild ──────────────────────────────────────────────────────────────────
echo "=== BUILD ==="
bash "$PROJECT_DIR/scripts/build_safe.sh" build --bin voxelforge --target-dir "$TARGET_DIR" 2>&1
RC=$?
EXE_EXISTS=false
[ -f "$EXE" ] && EXE_EXISTS=true

if [ "$RC" -ne 0 ]; then
  if $EXE_EXISTS; then
    echo "BUILD FAILED (exit=$RC) — but existing exe found; will use it"
    echo "WARNING: recording from PRE-BUILD exe — may not reflect latest .rs changes"
  else
    die "BUILD FAILED exit=$RC and no existing exe"
  fi
else
  echo "BUILD OK"
fi

# ── mtime guard: exe must be >= newest .rs ───────────────────────────────────
EXE_MTIME=$(stat -c %Y "$EXE" 2>/dev/null || echo "0")
NEWEST_RS=$(find "$PROJECT_DIR/client/src" -name '*.rs' -exec stat -c %Y {} \; 2>/dev/null | sort -rn | head -1)
NEWEST_RS="${NEWEST_RS:-0}"
NEWEST_RS_NAME=$(find "$PROJECT_DIR/client/src" -name '*.rs' -exec stat -c "%Y %n" {} \; 2>/dev/null | sort -rn | head -1 | cut -d' ' -f2-)
if [ "${EXE_MTIME:-0}" -lt "${NEWEST_RS:-0}" ]; then
  echo "WARNING: exe is STALE — older than at least one .rs file"
  echo "  exe mtime : $EXE_MTIME ($(date -d @"$EXE_MTIME" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || echo '?'))"
  echo "  newest .rs: $NEWEST_RS ($(date -d @"$NEWEST_RS" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || echo '?'))"
  echo "  newest file: $NEWEST_RS_NAME"
else
  echo "mtime check: exe ($EXE_MTIME) >= newest .rs ($NEWEST_RS) ✓"
fi

# ── git provenance ───────────────────────────────────────────────────────────
GIT_REF=$(git -C "$PROJECT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_DIRTY=$(git -C "$PROJECT_DIR" diff --stat 2>/dev/null | tail -1 || echo "")
echo "commit: $GIT_REF${GIT_DIRTY:+ (dirty)}"
echo "commit=$GIT_REF dirty=${GIT_DIRTY:-clean}" > "$OUTPUT_DIR/provenance.txt"

# ══════════════════════════════════════════════════════════════════════════════
# PHASE 1 — Launch game FIRST, resolve window title
# ══════════════════════════════════════════════════════════════════════════════
echo "=== LAUNCH GAME ==="
cd "$PROJECT_DIR"
timeout "$GAME_TIMEOUT" "$EXE" --combat-demo >"$LOG" 2>&1 &
GAME_PID=$!
echo "game PID: $GAME_PID"

# Poll for the window title. Bevy takes ~2-4 s to create the window on this
# machine (Vulkan init + asset loading). We give it up to WINDOW_POLL_MAX.
echo "waiting for game window title (max ${WINDOW_POLL_MAX}s)..."
WIN_TITLE=""
for ((i=0; i<WINDOW_POLL_MAX; i++)); do
  sleep 1
  WIN_TITLE=$(resolve_title)
  if [ -n "$WIN_TITLE" ]; then
    echo "window found after ${i}s: \"$WIN_TITLE\""
    break
  fi
  # Check if game already died
  if ! kill -0 "$GAME_PID" 2>/dev/null; then
    echo "game exited before window appeared (check $LOG)"
    break
  fi
done

if [ -z "$WIN_TITLE" ]; then
  echo "FATAL: could not resolve game window title within ${WINDOW_POLL_MAX}s"
  echo "Game may have crashed. Log tail:"
  tail -20 "$LOG" 2>/dev/null || true
  kill "$GAME_PID" 2>/dev/null || true
  taskkill /F /IM voxelforge.exe 2>/dev/null || true
  exit 1
fi

# ══════════════════════════════════════════════════════════════════════════════
# PHASE 2 — Start ffmpeg capturing ONLY the game window
# ══════════════════════════════════════════════════════════════════════════════
echo "=== RECORD ==="
echo "ffmpeg gdigrab → window \"$WIN_TITLE\" @ ${FPS}fps → MKV"

# draw_mouse=0 — no cursor in combat footage
timeout "$FFMPEG_TIMEOUT" ffmpeg -y -hide_banner -loglevel warning \
  -f gdigrab -framerate "$FPS" -draw_mouse 0 \
  -i title="$WIN_TITLE" \
  -c:v libx264 -preset ultrafast -crf 23 -pix_fmt yuv420p \
  "$RAW_MKV" &
FFMPEG_PID=$!
echo "ffmpeg PID: $FFMPEG_PID"

# ══════════════════════════════════════════════════════════════════════════════
# PHASE 3 — Wait for game to finish, then stop capture
# ══════════════════════════════════════════════════════════════════════════════
echo "=== WAIT FOR GAME ==="
wait "$GAME_PID" 2>/dev/null || true
GAME_EXIT=$?
echo "game exited with code: $GAME_EXIT"

# Let ffmpeg capture a few post-combat frames (respawn, fade), then stop
sleep 3
kill "$FFMPEG_PID" 2>/dev/null || true
wait "$FFMPEG_PID" 2>/dev/null || true
echo "ffmpeg stopped"

# ══════════════════════════════════════════════════════════════════════════════
# PHASE 4 — Verify capture exists
# ══════════════════════════════════════════════════════════════════════════════
if [ ! -f "$RAW_MKV" ] || [ ! -s "$RAW_MKV" ]; then
  die "no video captured ($RAW_MKV missing or empty). Log: $LOG"
fi

RAW_SIZE=$(stat -c %s "$RAW_MKV" 2>/dev/null || echo 0)
echo "raw mkv: $RAW_SIZE bytes"

DURATION=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$RAW_MKV" 2>/dev/null || echo "0")
echo "raw duration: ${DURATION}s"

# ══════════════════════════════════════════════════════════════════════════════
# PHASE 5 — Extract verification frames (first / mid / last)
# ══════════════════════════════════════════════════════════════════════════════
echo "=== VERIFY FRAMES ==="
VERIFY_FIRST="$OUTPUT_DIR/verify_${TIMESTAMP}_first.png"
VERIFY_MID="$OUTPUT_DIR/verify_${TIMESTAMP}_mid.png"
VERIFY_LAST="$OUTPUT_DIR/verify_${TIMESTAMP}_last.png"

# First frame at 0.5s (skip possible black init frame)
ffmpeg -y -hide_banner -loglevel error \
  -ss 0.5 -i "$RAW_MKV" -vframes 1 "$VERIFY_FIRST"
echo "  first frame (t=0.5s): $VERIFY_FIRST"

# Mid frame
MID_T=$(awk "BEGIN { printf \"%.1f\", $DURATION / 2.0 }")
ffmpeg -y -hide_banner -loglevel error \
  -ss "$MID_T" -i "$RAW_MKV" -vframes 1 "$VERIFY_MID"
echo "  mid frame   (t=${MID_T}s): $VERIFY_MID"

# Last frame (1s before end, avoid fade-to-black)
LAST_T=$(awk "BEGIN { t = $DURATION - 1.0; printf \"%.1f\", (t > 0.5 ? t : 0.5) }")
ffmpeg -y -hide_banner -loglevel error \
  -ss "$LAST_T" -i "$RAW_MKV" -vframes 1 "$VERIFY_LAST"
echo "  last frame  (t=${LAST_T}s): $VERIFY_LAST"

echo ""
echo ">>> VERIFY these frames before sending the MP4 to CEO:"
echo "    frame 1: $VERIFY_FIRST"
echo "    frame 2: $VERIFY_MID"
echo "    frame 3: $VERIFY_LAST"
echo "    Confirm: game content only, no desktop, no other apps."

# ══════════════════════════════════════════════════════════════════════════════
# PHASE 6 — Trim to 10–18 s MP4 (game content only)
# ══════════════════════════════════════════════════════════════════════════════
echo "=== FINAL MP4 ==="
# Trim: start at 0s (no desktop lead-in), keep up to 18s
TRIM_DUR=18
ffmpeg -y -hide_banner -loglevel warning \
  -i "$RAW_MKV" \
  -t "$TRIM_DUR" \
  -c:v libx264 -preset fast -crf 23 -pix_fmt yuv420p \
  "$FINAL_MP4"

FINAL_SIZE=$(stat -c %s "$FINAL_MP4" 2>/dev/null || echo 0)
FINAL_DUR=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$FINAL_MP4" 2>/dev/null || echo "?")

# If we're under 10s, slow down to hit the 10-20s target
SLOW=1.0
if [ "${FINAL_DUR%.*}" -lt 10 ] 2>/dev/null; then
  SLOW=$(awk "BEGIN { printf \"%.2f\", 10.0 / $FINAL_DUR }")
  echo "  video is short (${FINAL_DUR}s) — slowing down ${SLOW}x to hit 10s..."
  SLOWED_MP4="${FINAL_MP4%.mp4}_slow.mp4"
  ffmpeg -y -hide_banner -loglevel warning \
    -i "$FINAL_MP4" \
    -vf "setpts=${SLOW}*PTS" \
    -c:v libx264 -preset fast -crf 23 -pix_fmt yuv420p \
    "$SLOWED_MP4"
  FINAL_MP4="$SLOWED_MP4"
  FINAL_SIZE=$(stat -c %s "$FINAL_MP4" 2>/dev/null || echo 0)
  FINAL_DUR=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$FINAL_MP4" 2>/dev/null || echo "?")
fi

RES=$(ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=s=x:p=0 "$FINAL_MP4" 2>/dev/null || echo "?x?")
echo "final video: $FINAL_MP4"
echo "  resolution: $RES  fps: $FPS  size: $FINAL_SIZE bytes  duration: ${FINAL_DUR}s"

# ══════════════════════════════════════════════════════════════════════════════
# PHASE 7 — Cleanup lingering processes
# ══════════════════════════════════════════════════════════════════════════════
echo "=== CLEANUP ==="
HANGING=$(tasklist /FI "IMAGENAME eq voxelforge.exe" 2>/dev/null | grep -c "voxelforge.exe" || true)
if [ "${HANGING:-0}" -gt 0 ]; then
  echo "killing $HANGING lingering voxelforge.exe process(es)..."
  taskkill /F /IM voxelforge.exe 2>/dev/null || true
  sleep 1
  STILL=$(tasklist /FI "IMAGENAME eq voxelforge.exe" 2>/dev/null | grep -c "voxelforge.exe" || true)
  if [ "${STILL:-0}" -gt 0 ]; then
    echo "WARNING: $STILL process(es) still running — manual check needed"
  else
    echo "all voxelforge.exe processes killed ✓"
  fi
else
  echo "no lingering voxelforge.exe ✓"
fi

echo ""
echo "=== DONE ==="
echo "MP4: $FINAL_MP4"
echo "Verify frames: $VERIFY_FIRST | $VERIFY_MID | $VERIFY_LAST"
