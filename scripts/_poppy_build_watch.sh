#!/usr/bin/env bash
# _poppy_build_watch.sh — wait out the in-flight target-combat build, archive its
# log (never overwrite), and if it dies on the machine-headroom markers the CEO
# named (0xc0000142 / STATUS_DLL_INIT_FAILED / LNK1102 / could not exec the linker)
# wait 2 minutes and retry with -j 1 exactly once.
#
# Every round writes to its OWN log file. Nothing is ever truncated with `>`.
#
# v2 — the v1 exit test was wrong: a failed/slow PowerShell query returned an
# empty string, which v1 read as "no build running" and broke out after 1s while
# cargo was very much alive. Now the ONLY thing that ends the wait is evidence in
# the log itself (Finished / error) plus the log going quiet; a broken process
# query is treated as "still running", never as "done".
set -uo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_DIR" || exit 1

LIVE_LOG="$PROJECT_DIR/_poppy_build.log"
ARCHIVE="$PROJECT_DIR/_poppy_build_r1.log"
J1_LOG="$PROJECT_DIR/_poppy_build_j1.log"
EXE="$PROJECT_DIR/target-combat/debug/voxelforge.exe"
SRC="$PROJECT_DIR/client/src/combat.rs"

# The machine-headroom markers. These are NOT code bugs — never edit .rs for them.
HEADROOM_RE='0xc0000142|STATUS_DLL_INIT_FAILED|LNK1102|could not exec the linker'

# cargo prints a terminal line either way: "Finished ..." or "error: ...".
settled() {
  local log="$1"
  grep -qE '^(    Finished|error)' "$log" 2>/dev/null
}

mtime_of() { stat -c %Y "$1" 2>/dev/null || echo 0; }

verdict() {
  local log="$1" tag="$2"
  echo "----- verdict: $tag -----"
  echo "errors(grep -c '^error'): $(grep -c '^error' "$log" 2>/dev/null)"
  echo "headroom markers: $(grep -Ec "$HEADROOM_RE" "$log" 2>/dev/null)"
  echo "finished line: $(grep -c '^    Finished' "$log" 2>/dev/null)"
  echo "--- error lines ---"
  grep -E "^error|$HEADROOM_RE" "$log" 2>/dev/null | head -8
  echo "--- exe vs src mtime ---"
  ls -la --time-style=+%m-%d_%H:%M:%S "$EXE" 2>/dev/null || echo "NO EXE"
  ls -la --time-style=+%m-%d_%H:%M:%S "$SRC"
}

# ── round 1: wait out whatever is already running ────────────────────────────
echo "[watch] $(date '+%H:%M:%S') waiting for the in-flight target-combat build"
last_mt=0
quiet=0
for i in $(seq 1 90); do
  if settled "$LIVE_LOG"; then
    echo "[watch] $(date '+%H:%M:%S') log shows a terminal line (round ${i})"
    break
  fi
  mt="$(mtime_of "$LIVE_LOG")"
  if [ "$mt" = "$last_mt" ]; then quiet=$((quiet + 1)); else quiet=0; last_mt="$mt"; fi
  # 6 × 20s = 120s of a completely silent log with no terminal line = dead build.
  if [ "$quiet" -ge 6 ]; then
    echo "[watch] $(date '+%H:%M:%S') log silent 120s with no terminal line — treating as dead"
    break
  fi
  sleep 20
done

cp -f "$LIVE_LOG" "$ARCHIVE" 2>/dev/null
verdict "$ARCHIVE" "round 1 (-j 2, archived to _poppy_build_r1.log)"

if ! grep -Eq "$HEADROOM_RE" "$ARCHIVE" 2>/dev/null; then
  echo "[watch] no headroom marker in round 1 — stopping here"
  exit 0
fi

# ── round 2: headroom failure → wait 2 min, retry single-threaded ────────────
echo "[watch] $(date '+%H:%M:%S') headroom marker found → sleeping 120s before -j 1 retry"
sleep 120
echo "[watch] $(date '+%H:%M:%S') starting -j 1 build → _poppy_build_j1.log"
CARGO_PROFILE_DEV_DEBUG=0 cargo build -j 1 --bin voxelforge --target-dir target-combat >"$J1_LOG" 2>&1
echo "[watch] $(date '+%H:%M:%S') -j 1 build exited: $?"
verdict "$J1_LOG" "round 2 (-j 1)"
