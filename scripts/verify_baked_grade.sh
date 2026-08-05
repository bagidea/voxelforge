#!/usr/bin/env bash
# Flamingo — build the grade patch and prove the BAKED DEFAULT reproduces it.
#
# Every number in commit bd3cde5 is an env-override measurement
# (`VOXELFORGE_LOOK_GRADE` / `_LIGHT` / `_HAZECOL` / `_HAZE`). An env override
# proves a value is good; it does not prove the binary ships it. This script
# closes that gap in one detached run so nothing depends on a session staying
# alive to babysit it:
#
#   build  ->  wait for the exe to actually be NEW and finished linking
#          ->  shoot the vista framing with NO look env set at all
#          ->  grade_axes.py + colour_gate.py on the de-HUDded frame
#
# Judged by `Finished` and by `grep '^error'`, never by tail: cargo prints
# `Compiling` lines AFTER an error, and a pipe into tail reports tail's exit
# status, not cargo's. Both mistakes have already produced a false "build green"
# on this project.
set -uo pipefail
cd "$(dirname "$0")/.."

OUT=_fl_grade2
LOG=$OUT/verify-build.log
RES=$OUT/verify-result.txt
mkdir -p "$OUT"
: >"$RES"

say() { echo "$@" | tee -a "$RES"; }

say "=== BUILD START $(date -Is) ==="
# -j 2 on purpose: this box has run out of commit headroom under a wider build
# before and answered with 0xc0000142 STATUS_DLL_INIT_FAILED, which reads like a
# code bug and is not one. Another lane's cargo may still hold the target lock;
# cargo blocks on it rather than failing, which is the wanted behaviour.
CARGO_PROFILE_RELEASE_DEBUG=0 cargo build --release -j 2 >"$LOG" 2>&1
RC=$?
ERRS=$(grep -c '^error' "$LOG")
FIN=$(grep -c '^ *Finished' "$LOG")
say "cargo exit=$RC  '^error' lines=$ERRS  Finished=$FIN"
if [ "$RC" -ne 0 ] || [ "$ERRS" -gt 0 ] || [ "$FIN" -eq 0 ]; then
  say "BUILD FAILED — first errors:"
  grep -m 20 -A 4 '^error' "$LOG" | tee -a "$RES"
  say "=== ABORT (no frame shot; no numbers to report) ==="
  exit 1
fi

EXE=target/release/voxelforge.exe
say "exe mtime $(date -r "$EXE" -Is)  size $(stat -c %s "$EXE")"

# Copy before running: a sweep that runs the linker's own output file holds a
# lock on it, and the next lane's build then fails on access-denied.
PROBE=$OUT/vfverify.exe
cp "$EXE" "$PROBE"

STEM=$OUT/vista-BAKED
say "=== SHOOT (no VOXELFORGE_LOOK_* overrides at all) ==="
env VOXELFORGE_PLAY=1 VOXELFORGE_LOOK_CAM=35,-18,26 VOXELFORGE_LOOK_QUALITY=ultra \
    VOXELFORGE_SHOT="$STEM.png" "$PROBE" >"$STEM.log" 2>&1
say "shot exit=$?  bytes=$(stat -c %s "$STEM.png" 2>/dev/null || echo 0)"

python scripts/_flamingo_dehud2.py "$STEM.png" >/dev/null 2>&1
say "=== P0 AXES (baked default) ==="
python scripts/grade_axes.py "${STEM}-nohud2.png" 2>&1 | tee -a "$RES"
say "=== COLOUR GATE (baked default) ==="
python scripts/colour_gate.py "${STEM}-nohud2.png" 2>&1 | tee -a "$RES"
say "=== DONE $(date -Is) ==="
