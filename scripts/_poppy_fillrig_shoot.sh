#!/usr/bin/env bash
# _poppy_fillrig_shoot.sh — the before/after set for the v2 fill rig (look.rs).
#
# ONE BINARY, ENV ONLY. That is the whole point and it is why `LookGen` exists:
#   before (v1) — VOXELFORGE_LOOK_GEN=v1 → flat AMBIENT_LUX_V1 fill, no sky-fill,
#                 no ground-bounce, PCSS at Ultra only and PCSS_WIDTH_V1 wide.
#                 This is the rig this lane shipped before 2026-08-14.
#   after  (v2) — env UNSET. The shipped default, byte for byte the same exe.
# A commit sha does not date a binary and two builds are not one change, so the
# A side is a code path in the SAME file on disk, not a second exe and not a claim.
#
# TIER=high BY DEFAULT, and that is deliberate: High is `LookQuality::default()`,
# i.e. the tier a real session runs. The v1→v2 PCSS move is Ultra-only → High-and-up,
# so shooting at Ultra would hide half of what changed behind a tier nobody plays.
#
# THREE FRAMINGS, each answering a different half of the claim:
#   vista   35,-18,26 — the framing every gate row in this repo is shot on
#                       (vista_grade_sweep.sh:107). Wall-and-campfire close-up:
#                       shaded wall + contact shadows. Regression guard.
#   horizon 0,1,18    — pitch ~level, sees the ruins and the sky.
#   shade  128,-24,15 — NEW for this lane. Yaw swung round to the sun's far side
#                       and pitched down 24°, so both a block's TOP face and its
#                       own SIDE face are in frame and both in shade. That split
#                       is the one thing a flat AmbientLight cannot produce, so
#                       this is the framing the fill rig lives or dies on.
#
# THREE GATES THAT FAIL, not three gates that print. Every one of them exists
# because a print-only version of it would have let a wrong plate through:
#   1. FRESHNESS. `_poppy_fillrig_build.cmd` reuses `target-poppy` across retries,
#      and a 0xc0000142 DLL-init death writes NO `^error` line (my own scar). So a
#      stale exe from an earlier retry could be shot as if it were the new build.
#      Gate: the exe must be NEWER than the build script that produced this run,
#      and the build's own `EXIT=` must be 0.
#   2. PROVENANCE PER ARM. `LOOK_FILL` was being echoed and never checked — a v1
#      shot that came back reporting `sky=1400lux` would have passed silently.
#      Gate: each arm must print the exact fill line that arm is defined by.
#   3. THE ARMS MUST DIFFER. Same exe + same camera means a dead env hook hands
#      back two identical PNGs under a "look how much better" caption.
#
# Usage: bash scripts/_poppy_fillrig_shoot.sh      [BIN=... OUT=... TIER=...]
set -uo pipefail
cd "$(dirname "$0")/.."

BIN="${BIN:-./target-poppy/perf/voxelforge.exe}"
OUT="${OUT:-_poppy_fillrig}"
TIER="${TIER:-high}"
BUILD_CMD="${BUILD_CMD:-_poppy_fillrig_build.cmd}"
BUILD_DONE="${BUILD_DONE:-_poppy_fillrig_build.done}"
mkdir -p "$OUT"

# --- GATE 1: freshness -----------------------------------------------------
[ -f "$BIN" ] || { echo "GATE1 FAIL: no binary $BIN"; exit 2; }
if [ -f "$BUILD_CMD" ] && [ ! "$BIN" -nt "$BUILD_CMD" ]; then
  echo "GATE1 FAIL: $BIN is NOT newer than $BUILD_CMD — this is a stale exe from an"
  echo "            earlier retry, not the build that just ran. Refusing to shoot it."
  exit 2
fi
if [ -f "$BUILD_DONE" ]; then
  exitline=$(tr -d '\r' <"$BUILD_DONE")
  echo "GATE1: build $exitline"
  case "$exitline" in
    EXIT=0*) ;;
    *) echo "GATE1 FAIL: build did not exit 0 ($exitline) — a DLL-init death writes no ^error line"; exit 2;;
  esac
fi
echo "GATE1 OK  $BIN  $(ls -l --time-style=+%Y-%m-%d\ %H:%M:%S "$BIN" | awk '{print $5" bytes  "$6" "$7}')"
echo "TIER $TIER"

# The fill line each arm is DEFINED by, straight off look.rs:
#   v1 → hour() zeroes sky/bounce and restores AMBIENT_LUX_V1 (look.rs:1123-1131)
#   v2 → Hour::GOLDEN as shipped: 1150 flat + 1400 sky @76° + 1100 bounce @28°
expect_for() {
  case "$1" in
    v1) echo 'LOOK_FILL gen=V1 ambient=2200 sky=0lux@76deg bounce=0lux@28deg';;
    v2) echo 'LOOK_FILL gen=V2 ambient=1150 sky=1400lux@76deg bounce=1100lux@28deg';;
  esac
}

fails=0
shoot() {                          # shoot <framing> <arm> <cam> [EXTRA=env ...]
  local framing=$1 arm=$2 cam=$3; shift 3
  local png="$OUT/${framing}-${arm}.png"
  local log="$OUT/${framing}-${arm}.log"
  echo "--- $framing/$arm  cam=$cam  ${*:-(no extra env)}"
  env VOXELFORGE_PLAY=1 \
      VOXELFORGE_NOHUD=1 \
      VOXELFORGE_LOOK_CAM="$cam" \
      VOXELFORGE_LOOK_QUALITY="$TIER" \
      "$@" \
      VOXELFORGE_SHOT="$png" \
      timeout 240 "$bin_or_default" >"$log" 2>&1
  local rc=$?
  if [ ! -s "$png" ]; then
    echo "   MISS (exit=$rc)"; tail -4 "$log"; fails=$((fails+1)); return 1
  fi
  echo "   exit=$rc  $(wc -c <"$png" | tr -d ' ') bytes"
  if grep -aqE 'panicked|B0001' "$log"; then
    echo "   !! PANIC in log"; fails=$((fails+1))
  fi
  # --- GATE 2: provenance, asserted -----------------------------------------
  local got exp
  got=$(grep -aoE 'LOOK_FILL gen=[A-Za-z0-9]+ ambient=[0-9]+ sky=[0-9]+lux@[0-9]+deg bounce=[0-9]+lux@[0-9]+deg' "$log" | head -1)
  exp=$(expect_for "$arm")
  echo "   fill: ${got:-(no LOOK_FILL line at all)}"
  if [ "$got" != "$exp" ]; then
    echo "   GATE2 FAIL: this arm printed the wrong fill rig"
    echo "     expected: $exp"
    echo "     got     : ${got:-<nothing>}"
    fails=$((fails+1))
  fi
}

bin_or_default="$BIN"
for f in "vista 35,-18,26" "horizon 0,1,18" "shade 128,-24,15"; do
  set -- $f
  shoot "$1" v1 "$2" VOXELFORGE_LOOK_GEN=v1
  shoot "$1" v2 "$2"
done

# --- GATE 3: the arms must differ -----------------------------------------
echo
echo "=== GATE3 differ check (v1 vs v2 must NOT be byte-identical) ==="
for framing in vista horizon shade; do
  a="$OUT/${framing}-v1.png"; b="$OUT/${framing}-v2.png"
  if [ ! -s "$a" ] || [ ! -s "$b" ]; then echo "$framing  MISSING A SIDE"; fails=$((fails+1)); continue; fi
  if [ "$(md5sum <"$a" | cut -d' ' -f1)" = "$(md5sum <"$b" | cut -d' ' -f1)" ]; then
    echo "$framing  GATE3 FAIL — identical bytes, the GEN hook did nothing"; fails=$((fails+1))
  else
    echo "$framing  OK differs"
  fi
done

echo
echo "=== files ==="
ls -l --time-style=+%H:%M:%S "$OUT"/*.png 2>/dev/null | awk '{print $5"  "$6"  "$7}'
echo
echo "GATE FAILURES: $fails"
exit $(( fails > 0 ? 1 : 0 ))
