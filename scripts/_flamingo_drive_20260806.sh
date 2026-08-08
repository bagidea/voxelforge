#!/usr/bin/env bash
# Flamingo — wait for a free box, build ONCE, then shoot + grade the 8 frames.
#
# WHY IT WAITS FIRST. The 07:59 attempt died at `bevy_image` with
# `exit code: 0xc0000142, STATUS_DLL_INIT_FAILED` while Yamamoto's own release
# build was live one minute away (his log at 07:58, and his own run died the
# same way). That exit code is not a code bug — it is the box out of process /
# commit headroom, and the only fix is to not stack the builds. So this polls
# until no cargo/rustc is running and refuses to start if the box never clears.
#
# WHY -j 2. Same reason. The release profile here uses codegen-units=1 +
# linker-plugin-lto, so each rustc is a big single-threaded memory hog; two at
# once is what this machine has headroom for alongside the office daemon.
#
# WHY `grep -c '^error'` DECIDES, NOT `tail`. cargo keeps printing `Compiling`
# lines after a crate has already failed ("waiting for other jobs to finish"),
# so the last line of a failed build looks like progress. It is also why the
# 07:59 log read as "still compiling" to a reader who only looked at the tail.
set -uo pipefail
cd "$(dirname "$0")/.."

OUT="${OUT:-_fl_recap_20260806}"
mkdir -p "$OUT"
LOG="$OUT/drive.log"
: > "$LOG"
say() { echo "$(date '+%H:%M:%S') $*" | tee -a "$LOG"; }

# ---- 1. wait for a free box -------------------------------------------------
say "waiting for cargo/rustc to clear (up to 45 min)..."
FREE=0
for i in $(seq 1 270); do
  n=$(tasklist 2>/dev/null | grep -ciE '^(cargo|rustc|link)\.exe' || true)
  if [ "${n:-0}" -eq 0 ]; then
    # Two clean polls in a row — cargo briefly shows 0 between crates on
    # some spawns, and starting into that gap is how builds get stacked.
    sleep 8
    n2=$(tasklist 2>/dev/null | grep -ciE '^(cargo|rustc|link)\.exe' || true)
    if [ "${n2:-0}" -eq 0 ]; then say "box free after ${i} polls"; FREE=1; break; fi
  fi
  [ $((i % 6)) -eq 0 ] && say "  still busy: $n proc(s) after ${i} polls"
  sleep 10
done
if [ "$FREE" -ne 1 ]; then
  say "STILL BUSY after 45 min — refusing to stack a third build. NOTHING BUILT."
  exit 3
fi

# ---- 2. build ONCE ----------------------------------------------------------
BUILD_LOG="$OUT/build.log"
say "building release into target-flamingo (-j 2, debug=0)..."
CARGO_PROFILE_RELEASE_DEBUG=0 cargo build --release -j 2 \
  -p voxelforge --target-dir target-flamingo > "$BUILD_LOG" 2>&1
BUILD_RC=$?
ERRS=$(grep -c '^error' "$BUILD_LOG" || true)
say "build exit=$BUILD_RC   grep '^error' count=${ERRS:-0}"
if [ "$BUILD_RC" -ne 0 ] || [ "${ERRS:-0}" -ne 0 ]; then
  say "BUILD FAILED — first errors:"
  grep -n '^error' "$BUILD_LOG" | head -20 | tee -a "$LOG"
  exit 1
fi

EXE=target-flamingo/release/voxelforge.exe
[ -s "$EXE" ] || { say "BUILD 'green' but no exe at $EXE"; exit 1; }
say "exe: $(stat -c '%y  %s bytes' "$EXE" | cut -d. -f1)"
# The 0.9.45 lesson: a build can report green while the exe on disk is the old
# one (denied access on a locked file). If the exe is older than look.rs, the
# frames would measure the OLD constants and read as "the patch did nothing".
if [ "client/src/look.rs" -nt "$EXE" ]; then
  say "STALE EXE — look.rs is newer than the binary. Refusing to shoot."
  exit 1
fi
say "exe is newer than look.rs — OK"

# ---- 3. shoot the 8 frames --------------------------------------------------
say "shooting 8 frames (4 stems x new/ctrl, one binary)..."
bash scripts/_flamingo_recap_20260806.sh >"$OUT/capture.log" 2>&1
CAP_RC=$?
say "capture exit=$CAP_RC"
tail -14 "$OUT/capture.log" | tee -a "$LOG"

# ---- 4. grade ---------------------------------------------------------------
say "grading..."
bash scripts/_flamingo_recap_grade_20260806.sh >"$OUT/grade.log" 2>&1
say "grade exit=$? -> $OUT/grade.log ($(wc -l <"$OUT/grade.log") lines)"
say "DONE"
