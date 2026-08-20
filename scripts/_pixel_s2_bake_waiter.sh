#!/usr/bin/env bash
# ===========================================================================
# Flamingo/pixel - wait for a free build slot, build target-pixel, then shoot
# the proof frames. Detached so a session cut cannot orphan the work.
#
# Why a waiter: two builds were already running (target/ and target-poppysky/).
# A third concurrent cargo on this box runs the machine out of commit headroom
# and the link dies with 0xc0000142 STATUS_DLL_INIT_FAILED - a FALSE fail that
# reads exactly like a code error. So: wait, then -j 2.
#
# Verdict rule: `grep -c '^error'` on the build log. NEVER tail - cargo keeps
# printing `Compiling` lines after an error, so the tail of a failed build
# looks green.
# ===========================================================================
set -u
ROOT="E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge"
OUT="$ROOT/_pixel_hero"
LOG="$ROOT/_pixel_s2_build.log"
STATE="$ROOT/_pixel_s2_waiter.state"
EXE="$ROOT/target-pixel/release/voxelforge_shot.exe"
HERO_CAM="7.6,5.9,-5.2,7.6,3.2,6.0,52"

cd "$ROOT" || exit 2
echo "waiting for a free build slot" > "$STATE"

# --- 1. wait for every other cargo to finish (cap 50 min) -------------------
waited=0
while [ $waited -lt 3000 ]; do
  n=$(tasklist 2>/dev/null | grep -ci 'cargo\.exe')
  [ "$n" -eq 0 ] && break
  echo "waited ${waited}s - $n cargo still running" > "$STATE"
  sleep 20
  waited=$((waited + 20))
done
n=$(tasklist 2>/dev/null | grep -ci 'cargo\.exe')
if [ "$n" -ne 0 ]; then
  echo "GAVE-UP after ${waited}s - $n cargo still running, did NOT build" > "$STATE"
  exit 3
fi

# --- 2. build ---------------------------------------------------------------
echo "slot free after ${waited}s - building target-pixel -j 2" > "$STATE"
exe_before=""
[ -f "$EXE" ] && exe_before=$(stat -c '%Y %s' "$EXE")
export CARGO_TARGET_DIR="$ROOT/target-pixel"
cargo build --release -p voxelforge --bin voxelforge_shot -j 2 > "$LOG" 2>&1
ec=$?
errs=$(grep -c '^error' "$LOG")
echo "build finished ec=$ec errors=$errs" > "$STATE"
if [ "$errs" -ne 0 ]; then
  echo "BUILD RED - $errs error lines, no frames shot" > "$STATE"
  exit 1
fi
exe_after=""
[ -f "$EXE" ] && exe_after=$(stat -c '%Y %s' "$EXE")
if [ "$exe_before" = "$exe_after" ]; then
  echo "REFUSED - exe unchanged ($exe_after), build produced nothing new" > "$STATE"
  exit 4
fi

# --- 3. shoot the proof frames ---------------------------------------------
# Clear every look lever so nothing leaks in from this shell.
unset VOXELFORGE_CAM VOXELFORGE_SUN VOXELFORGE_DOF VOXELFORGE_FOG VOXELFORGE_DFOG \
      VOXELFORGE_EXPOSURE VOXELFORGE_GRADE VOXELFORGE_AMBIENT VOXELFORGE_AMBCOLOR \
      VOXELFORGE_SHOULDER VOXELFORGE_BLUESCALE VOXELFORGE_BOUNCE VOXELFORGE_BOUNCE2 \
      VOXELFORGE_BOUNCE1COLOR VOXELFORGE_BOUNCE2COLOR VOXELFORGE_RIM VOXELFORGE_DUST \
      VOXELFORGE_WIDE VOXELFORGE_CLEAR VOXELFORGE_SUNCOLOR VOXELFORGE_PANEHI VOXELFORGE_PANELO

# A - the real proof: NO look env at all. Only the output path is set, which is
#     not a look lever. This is the frame someone gets from a fresh build.
echo "shooting A (no env)" > "$STATE"
VOXELFORGE_SHOT="$OUT/baked_s2_noenv.png" "$EXE" > "$OUT/baked_s2_noenv.log" 2>&1

# B - same baked default, but framed on the ladder's hero cam so it is
#     comparable pixel-for-pixel with the s2 plate.
echo "shooting B (hero cam, baked default)" > "$STATE"
VOXELFORGE_CAM="$HERO_CAM" VOXELFORGE_SHOT="$OUT/baked_s2_herocam.png" \
  "$EXE" > "$OUT/baked_s2_herocam.log" 2>&1

# C - hero cam + the s2 env set EXPLICITLY. If B and C match, the bake is the
#     real default and not an env crutch.
echo "shooting C (hero cam, explicit s2 env)" > "$STATE"
VOXELFORGE_CAM="$HERO_CAM" \
VOXELFORGE_AMBIENT="1700" VOXELFORGE_AMBCOLOR="0.38,0.51,0.80" \
VOXELFORGE_BOUNCE="2.6" VOXELFORGE_BOUNCE2="1.1" \
VOXELFORGE_SUN="19,196,32000" VOXELFORGE_RIM="0.46,0.62,1.0,3400" \
VOXELFORGE_GRADE="-0.01,1.00,1.32" VOXELFORGE_SHOULDER="0.72" \
VOXELFORGE_EXPOSURE="8.80" VOXELFORGE_SHOT="$OUT/baked_s2_envcheck.png" \
  "$EXE" > "$OUT/baked_s2_envcheck.log" 2>&1

sa=$( [ -f "$OUT/baked_s2_noenv.png" ]    && stat -c %s "$OUT/baked_s2_noenv.png"    || echo 0 )
sb=$( [ -f "$OUT/baked_s2_herocam.png" ]  && stat -c %s "$OUT/baked_s2_herocam.png"  || echo 0 )
sc=$( [ -f "$OUT/baked_s2_envcheck.png" ] && stat -c %s "$OUT/baked_s2_envcheck.png" || echo 0 )
echo "DONE build-green A=$sa B=$sb C=$sc bytes" > "$STATE"
exit 0
