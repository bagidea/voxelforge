#!/usr/bin/env bash
# Poppy — before/after proof for the magenta-wash fix (26b2ae6), ONE binary.
#
# Both states come out of the SAME freshly-built exe: the "after" run uses the
# shipped constants, the "before" run reproduces the pre-fix look through the
# documented sweep hooks (`VOXELFORGE_LOOK_GRADE` / `VOXELFORGE_LOOK_LIGHT`),
# set to exactly what `git show 26b2ae6^:client/src/look.rs` had:
#   TEMPERATURE 0.10 · POST_SATURATION 1.02 · MIDTONE_CONTRAST 1.30 · HIGHLIGHT_GAIN 0.64
#   key + ambient light colours untouched by look.rs -> main.rs's default WHITE
# Same build, same scene, same seed — the only difference is the constants, which
# is the point: a rebuilt old binary would also carry every other 12:12 delta.
#
# Usage:  bash scripts/_poppy_ba_shoot.sh before|after [angle ...]
set -u

STATE="${1:-}"; shift || true
[ -n "$STATE" ] || { echo "usage: $0 before|after [angle ...]"; exit 2; }

BIN="${BIN:-./target/debug/voxelforge.exe}"
OUT="_poppy_ba/shots/$STATE"
LOGS="_poppy_ba/logs/$STATE"
mkdir -p "$OUT" "$LOGS"

# The pre-fix look, reproduced through the sweep hooks.
BEFORE_ENV=(VOXELFORGE_LOOK_GRADE=0.10,1.02,1.30,0.64 VOXELFORGE_LOOK_LIGHT=1,1,1,1,1,1)

# angle -> the env that frames it (mirrors scripts/gate3_shoot.sh + render_castle.sh)
angle_env() {
  case "$1" in
    # NOTE: the look stack is gated on `cfg.play` (look.rs::look_enabled), so a
    # fly-cam map-load overview renders the SAME pixels before and after — it
    # never sees the grade. Verified: both states hashed f9ac1351 identically.
    # The far angle therefore has to be a PLAY frame; the open arena is the one
    # with a horizon, which is what puts the flat ClearColor sky — the surface
    # the magenta bug hits hardest — across ~45% of the frame.
    far)    echo "VOXELFORGE_PLAY=1 VOXELFORGE_MAP_LOAD=maps/arena.json" ;;
    boot)   echo "VOXELFORGE_PLAY=1" ;;                      # player standing at spawn
    walk)   echo "VOXELFORGE_PLAY_DEMO=1" ;;                 # mid-stride, camera orbited
    combat) echo "VOXELFORGE_COMBAT_DEMO=1" ;;               # closing on a husk, HUD up
    *)      echo "" ;;
  esac
}

ANGLES=("$@")
[ ${#ANGLES[@]} -gt 0 ] || ANGLES=(far boot walk combat)

for a in "${ANGLES[@]}"; do
  env_extra=$(angle_env "$a")
  [ -n "$env_extra" ] || { echo "  ✗ unknown angle: $a"; continue; }
  png="$OUT/$a.png"
  log="$LOGS/$a.log"
  pre=()
  [ "$STATE" = "before" ] && pre=("${BEFORE_ENV[@]}")
  t0=$(date +%s)
  # $env_extra is deliberately unquoted — an angle may carry more than one
  # KEY=VALUE and none of them contain spaces.
  # shellcheck disable=SC2086
  env "${pre[@]}" $env_extra VOXELFORGE_SHOT="$png" VOXELFORGE_LOOK_QUALITY=high \
    "$BIN" >"$log" 2>&1
  rc=$?
  t1=$(date +%s)
  bytes=0; [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  ok="✓"
  [ "$rc" -ne 0 ] && ok="✗"
  [ "$bytes" -lt 2048 ] && ok="✗"
  grep -q 'SHOT saved' "$log" || ok="✗"
  grep -qE 'panicked|B0001' "$log" && ok="✗"
  echo "  $ok $STATE/$a  exit=$rc  bytes=$bytes  $((t1 - t0))s"
done
