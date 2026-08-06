#!/usr/bin/env bash
# Rose — build voxelforge into target-rose (NOT target/, NOT target-flamingo),
# gated on a free box, then leave it for the recap capture/grade scripts.
#
# WHY IT WAITS FIRST. Same STATUS_DLL_INIT_FAILED (0xc0000142) trap the other
# lanes hit: stacking a release build on top of a live one fails the link with a
# code that reads like a bug and isn't one. Poll until cargo/rustc/link clear.
#
# WHY -j 2 + debug=0 + target-rose. -j 2 is the box's commit headroom;
# CARGO_PROFILE_RELEASE_DEBUG=0 drops /DEBUG from the link line (buys headroom);
# target-rose is Rose's own dir — never target/ (Poppy), never target-flamingo.
#
# WHY grep '^error' DECIDES, NOT exit/pipe. cargo keeps printing `Compiling`
# after a crate has failed; a pipe's $? is tail's, not cargo's. Both have
# produced a false "build green" on this project before.
set -uo pipefail
cd "$(dirname "$0")/.."

OUT="${OUT:-_rose_recap_20260806}"
mkdir -p "$OUT"
LOG="$OUT/drive-build.log"
: > "$LOG"
say() { echo "$(date '+%H:%M:%S') $*" | tee -a "$LOG"; }

proc_count() {
  powershell.exe -NoProfile -Command \
    "(Get-CimInstance Win32_Process -Filter \"Name='cargo.exe' OR Name='rustc.exe' OR Name='link.exe'\" | Measure-Object).Count" \
    2>/dev/null | tr -d '\r ' | tr -dc '0-9'
}

# ---- 1. wait for a free box -------------------------------------------------
say "waiting for cargo/rustc/link to clear (up to 45 min)..."
FREE=0
for i in $(seq 1 270); do
  n=$(proc_count); n=${n:-0}
  if [ "$n" -eq 0 ]; then
    # Two clean polls in a row — cargo briefly shows 0 between crates.
    sleep 8
    n2=$(proc_count); n2=${n2:-0}
    if [ "$n2" -eq 0 ]; then say "box free after ${i} polls"; FREE=1; break; fi
  fi
  [ $((i % 6)) -eq 0 ] && say "  still busy: $n proc(s) after ${i} polls"
  sleep 10
done
if [ "$FREE" -ne 1 ]; then
  say "STILL BUSY after 45 min — refusing to stack. NOTHING BUILT."; exit 3
fi

# ---- 2. build ONCE into target-rose -----------------------------------------
BUILD_LOG="$OUT/build-cargo.log"
say "building release into target-rose (-j 2, debug=0)..."
CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_TARGET_DIR=target-rose \
  cargo build --release -j 2 -p voxelforge > "$BUILD_LOG" 2>&1
BUILD_RC=$?
ERRS=$(grep -c '^error' "$BUILD_LOG" || true)
FIN=$(grep -c 'Finished' "$BUILD_LOG" || true)
say "build exit=$BUILD_RC   grep '^error' count=${ERRS:-0}   Finished=${FIN:-0}"
if [ "$BUILD_RC" -ne 0 ] || [ "${ERRS:-0}" -ne 0 ] || [ "${FIN:-0}" -eq 0 ]; then
  say "BUILD FAILED — first errors:"
  grep -n '^error' "$BUILD_LOG" | head -20 | tee -a "$LOG"
  exit 1
fi

EXE=target-rose/release/voxelforge.exe
[ -s "$EXE" ] || { say "BUILD 'green' but no exe at $EXE"; exit 1; }
say "exe: $(stat -c '%y  %s bytes' "$EXE" | cut -d. -f1)"
# A build can report green while the exe on disk is the old one (denied access
# on a locked file). If the exe is older than look.rs the frames would measure
# the OLD constants and read as "the patch did nothing".
if [ "client/src/look.rs" -nt "$EXE" ]; then
  say "STALE EXE — look.rs is newer than the binary. Refusing to shoot."; exit 1
fi
say "exe is newer than look.rs — OK"
say "BUILD GREEN: $EXE"
say "DONE"
