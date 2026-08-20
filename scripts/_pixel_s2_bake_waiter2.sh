#!/usr/bin/env bash
# ===========================================================================
# Flamingo/pixel - waiter v2. Wait for a genuinely free build slot, build
# target-pixel THROUGH BUILD SENTINEL, then shoot the B ~= C proof frames.
# Detached, so a session cut cannot orphan the work.
#
# What changed vs v1 (and why):
#
#  * cap 50min -> 120min. v1's cap was sized for "two builds finish". The real
#    queue is not two builds: Poppy's A/B driver fires arm after arm by itself,
#    so the cargo count does NOT fall in a straight line. A 50min cap would
#    have written GAVE-UP for a reason that was never true.
#
#  * GAVE-UP now says WHICH kind. "queue busy" (never got a slot, nothing was
#    built, nothing is known about the code) is a completely different fact
#    from "build failed" (it built and the code is red). v1 blurred them.
#
#  * release gate = cargo.exe count EXACTLY 0. Never squeeze in beside a
#    running child: this box has run out of commit headroom before and killed
#    a link with 0xc0000142, which reads exactly like a code error.
#
#  * the build is no longer fired here. `build-sentinel status` must be idle,
#    then `build-sentinel build` runs it - office rule as of 16:10. Sentinel
#    also gives the 3-layer verdict (real exit code + whole-output error scan
#    + artifact mtime), which is stricter than v1's grep.
#
#  * target-dir is target-pixel. Never target-kevin / target-poppysky.
# ===========================================================================
set -u

# Launched detached from PowerShell, this shell inherits PowerShell's PATH - which
# does NOT contain Git's /usr/bin. grep/date/sed/stat then silently do not exist,
# every `$(...)` returns "", and the cargo-count gate reads as "0 running" and
# waves the build straight through. That already happened once at 16:52 and put a
# third cargo on the box. So: fix PATH first, then PROVE the tools work (preflight
# below) before any number from them is trusted.
export PATH="/usr/bin:/bin:/mingw64/bin:/c/WINDOWS/system32:/c/WINDOWS:$PATH"

ROOT="E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge"
SCRIPTS="$ROOT/scripts"
OUT="$ROOT/_pixel_hero"
STATE="$ROOT/_pixel_s2_waiter.state"
PROG="$ROOT/_pixel_s2_waiter.progress"
EXE="$ROOT/target-pixel/release/voxelforge_shot.exe"
HERO_CAM="7.6,5.9,-5.2,7.6,3.2,6.0,52"
BS="http://127.0.0.1:8787/plugin/build-sentinel/cmd"
JSON="content-type: application/json"

WAIT_CAP=7200      # 120 min waiting for a free slot
BUILD_CAP=7200     # 120 min for the build itself (fat LTO link is slow)

mkdir -p "$OUT"
cd "$ROOT" || exit 2

say() {  # one line -> state file (current) + progress log (history)
  printf '%s\n' "$1" > "$STATE"
  printf '[%s] %s\n' "$(date +%H:%M:%S)" "$1" >> "$PROG"
}

# Fail-safe counters: a probe that cannot answer reports 999 (= "busy"), never
# "" and never 0. An unreadable process list must block the build, not open it.
ncargo() { c=$(tasklist 2>/dev/null | grep -ci 'cargo\.exe'); case "$c" in ''|*[!0-9]*) echo 999 ;; *) echo "$c" ;; esac; }
nrustc() { c=$(tasklist 2>/dev/null | grep -ci 'rustc\.exe'); case "$c" in ''|*[!0-9]*) echo 999 ;; *) echo "$c" ;; esac; }

# --- 0. preflight: refuse to run on instruments that are not there ----------
# Every number this script gates on comes from one of these. A missing tool
# returns "" - which compares as "not busy" and is the most dangerous possible
# failure mode here. Prove them, or build nothing.
for t in tasklist grep sed date stat curl sleep python; do
  command -v "$t" >/dev/null 2>&1 || { say "REFUSED - preflight: '$t' not found on PATH. Measured nothing, built nothing."; exit 2; }
done
probe=$(tasklist 2>/dev/null | grep -ci '\.exe')
case "$probe" in
  ''|*[!0-9]*) say "REFUSED - preflight: tasklist|grep gave '$probe', not a number. The cargo-count gate is blind; built nothing."; exit 2 ;;
esac
[ "$probe" -lt 5 ] && { say "REFUSED - preflight: tasklist sees only $probe processes - implausible, the process probe is broken. Built nothing."; exit 2; }
n=$(ncargo)
[ "$n" -eq 999 ] && { say "REFUSED - preflight: the cargo counter returned its blind-sentinel value. It cannot see the process list; built nothing."; exit 2; }

say "v2 start - preflight ok ($probe processes visible) - waiting for a free build slot (cap ${WAIT_CAP}s), cargo now $n"

# --- 1. wait until NOT ONE cargo.exe is left --------------------------------
waited=0
n=$(ncargo)
while [ "$n" -ne 0 ] && [ "$waited" -lt "$WAIT_CAP" ]; do
  say "WAITING ${waited}s/${WAIT_CAP}s - $n cargo.exe / $(nrustc) rustc.exe still running"
  sleep 20
  waited=$((waited + 20))
  n=$(ncargo)
done
n=$(ncargo)
if [ "$n" -ne 0 ]; then
  say "GAVE-UP: queue busy - ${waited}s elapsed, $n cargo.exe still running. NO BUILD WAS RUN, so nothing is known about the code. This is NOT a build failure."
  exit 3
fi
say "slot free after ${waited}s - cargo.exe = 0 (rustc.exe = $(nrustc))"

# --- 2. Build Sentinel must be idle before we claim it ----------------------
bs_status() { curl -s -X POST "$BS" -H "$JSON" --data-binary "@$SCRIPTS/_pixel_bs_status.json"; }
bs_field() { printf '%s' "$1" | grep -o "\"$2\":\"[^\"]*\"" | head -1 | sed 's/.*:"//;s/"$//'; }
bs_id()    { printf '%s' "$1" | grep -o "\"id\":[0-9]*" | head -1 | sed 's/.*://'; }

sw=0
st=$(bs_field "$(bs_status)" status)
# An unparseable status is not "finished". Refuse rather than guess.
[ -z "$st" ] && { say "REFUSED - Build Sentinel status did not parse (daemon down or reply changed shape). Built nothing."; exit 2; }
while [ "$st" = "running" ] && [ "$sw" -lt 1800 ]; do
  say "sentinel busy (${sw}s) - someone else's guarded build is running"
  sleep 20; sw=$((sw + 20))
  st=$(bs_field "$(bs_status)" status)
done
if [ "$st" = "running" ]; then
  say "GAVE-UP: queue busy - Build Sentinel still running another build after ${sw}s. NO BUILD WAS RUN. This is NOT a build failure."
  exit 3
fi

# last look before we fire: the gate is 0, not 'nearly 0'
n=$(ncargo)
if [ "$n" -ne 0 ]; then
  say "GAVE-UP: queue busy - a new cargo ($n) appeared in the moment the slot opened; refused to build beside it. NO BUILD WAS RUN."
  exit 3
fi

# --- 3. fire it through Build Sentinel --------------------------------------
before=$( [ -f "$EXE" ] && stat -c '%Y/%s' "$EXE" || echo none )
say "sentinel idle - firing guarded build (target-pixel, -j 2). exe before: $before"
fire=$(curl -s -X POST "$BS" -H "$JSON" --data-binary "@$SCRIPTS/_pixel_bs_build.json")
printf 'fire reply: %s\n' "$fire" >> "$PROG"
case "$fire" in
  *'"ok":true'*) : ;;
  *) say "GAVE-UP: sentinel refused the build - $fire. NO BUILD WAS RUN."; exit 3 ;;
esac

runid=$(bs_id "$fire")
bw=0
st=$(bs_field "$(bs_status)" status)
while [ "$st" = "running" ] && [ "$bw" -lt "$BUILD_CAP" ]; do
  say "BUILDING ${bw}s/${BUILD_CAP}s via Build Sentinel (cargo now $(ncargo))"
  sleep 20; bw=$((bw + 20))
  st=$(bs_field "$(bs_status)" status)
done
last=$(curl -s -X POST "$BS" -H "$JSON" --data-binary "@$SCRIPTS/_pixel_bs_last.json")
printf 'last: %s\n' "$last" >> "$PROG"
verdict=$(bs_field "$last" verdict)
lastid=$(bs_id "$last")
after=$( [ -f "$EXE" ] && stat -c '%Y/%s' "$EXE" || echo none )

if [ "$st" = "running" ]; then
  say "BUILD TIMED OUT after ${bw}s - sentinel still running. No frames shot."
  exit 4
fi
# `last` is whatever finished most recently - not necessarily MY run. Reading
# someone else's verdict as mine is exactly how a stale record gets reported as
# a fresh result, so the ids must match or nothing is proven.
if [ "$lastid" != "$runid" ]; then
  say "REFUSED - build-sentinel last is run $lastid, not mine ($runid). Refusing to report another run's verdict as my build."
  exit 4
fi
if [ "$verdict" = "red" ]; then
  say "BUILD FAILED - sentinel verdict RED (this IS a build failure, not a queue problem). No frames shot. See build-sentinel last."
  exit 1
fi
if [ "$verdict" != "green" ]; then
  say "BUILD UNPROVEN - sentinel verdict '${verdict:-none}' (it ran, but at least one layer could not answer - NOT the same as red). No frames shot. See build-sentinel last."
  exit 4
fi
if [ "$before" = "$after" ]; then
  say "REFUSED - sentinel said green but the exe never moved ($after). Not shooting frames off a stale binary."
  exit 4
fi
say "BUILD GREEN in ${bw}s - exe $before -> $after. Shooting proof frames."

# --- 4. shoot A / B / B2 / C ------------------------------------------------
# Clear every look lever so nothing leaks in from this shell.
unset VOXELFORGE_CAM VOXELFORGE_SUN VOXELFORGE_DOF VOXELFORGE_FOG VOXELFORGE_DFOG \
      VOXELFORGE_EXPOSURE VOXELFORGE_GRADE VOXELFORGE_AMBIENT VOXELFORGE_AMBCOLOR \
      VOXELFORGE_SHOULDER VOXELFORGE_BLUESCALE VOXELFORGE_BOUNCE VOXELFORGE_BOUNCE2 \
      VOXELFORGE_BOUNCE1COLOR VOXELFORGE_BOUNCE2COLOR VOXELFORGE_RIM VOXELFORGE_DUST \
      VOXELFORGE_WIDE VOXELFORGE_CLEAR VOXELFORGE_SUNCOLOR VOXELFORGE_PANEHI VOXELFORGE_PANELO

# A - the real proof: NO look env at all. Only the output path, which is not a
#     look lever. This is the frame a fresh clone gets.
say "shooting A (no env at all)"
VOXELFORGE_SHOT="$OUT/baked_s2_noenv.png" "$EXE" > "$OUT/baked_s2_noenv.log" 2>&1

# B - baked default, framed on the hero cam so it is comparable with the s2 plate
say "shooting B (hero cam, baked default)"
VOXELFORGE_CAM="$HERO_CAM" VOXELFORGE_SHOT="$OUT/baked_s2_herocam.png" \
  "$EXE" > "$OUT/baked_s2_herocam.log" 2>&1

# B2 - B again, unchanged. This is the capture's own noise floor: without it the
#      B-vs-C tolerance would be a number I invented.
say "shooting B2 (repeat of B - noise floor)"
VOXELFORGE_CAM="$HERO_CAM" VOXELFORGE_SHOT="$OUT/baked_s2_herocam_r2.png" \
  "$EXE" > "$OUT/baked_s2_herocam_r2.log" 2>&1

# C - hero cam + the s2 env set EXPLICITLY. B ~= C means the bake is the real
#     default and not an env crutch.
say "shooting C (hero cam, explicit s2 env)"
VOXELFORGE_CAM="$HERO_CAM" \
VOXELFORGE_AMBIENT="1700" VOXELFORGE_AMBCOLOR="0.38,0.51,0.80" \
VOXELFORGE_BOUNCE="2.6" VOXELFORGE_BOUNCE2="1.1" \
VOXELFORGE_SUN="19,196,32000" VOXELFORGE_RIM="0.46,0.62,1.0,3400" \
VOXELFORGE_GRADE="-0.01,1.00,1.32" VOXELFORGE_SHOULDER="0.72" \
VOXELFORGE_EXPOSURE="8.80" VOXELFORGE_SHOT="$OUT/baked_s2_envcheck.png" \
  "$EXE" > "$OUT/baked_s2_envcheck.log" 2>&1

sz() { [ -f "$1" ] && stat -c %s "$1" || echo 0; }
say "frames: A=$(sz "$OUT/baked_s2_noenv.png") B=$(sz "$OUT/baked_s2_herocam.png") B2=$(sz "$OUT/baked_s2_herocam_r2.png") C=$(sz "$OUT/baked_s2_envcheck.png") bytes - measuring"

# --- 5. the gate: B ~= C, judged against B-vs-B2 --------------------------
python "$SCRIPTS/_pixel_bc_compare.py" \
  "$OUT/baked_s2_herocam.png" "$OUT/baked_s2_herocam_r2.png" "$OUT/baked_s2_envcheck.png" \
  > "$ROOT/_pixel_s2_gate.txt" 2>&1
g=$?
cat "$ROOT/_pixel_s2_gate.txt" >> "$PROG"
if [ "$g" -eq 0 ]; then
  say "DONE build-green, GATE PASS (B ~= C). See _pixel_s2_gate.txt"
  exit 0
elif [ "$g" -eq 2 ]; then
  say "DONE build-green, GATE REFUSED TO MEASURE. See _pixel_s2_gate.txt"
  exit 6
else
  say "DONE build-green, GATE FAIL (B != C - the bake is not the whole recipe). See _pixel_s2_gate.txt"
  exit 5
fi
