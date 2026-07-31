#!/usr/bin/env bash
# Gate 3 — LookPlugin ON/OFF, same binary, same scene, one variable changed.
#
# WHY THIS EXISTS (vs scripts/gate3_shoot.sh): that script only ever shoots
# LookPlugin-ON frames and reuses old pre-LookPlugin PNGs as the "before" —
# those were captured on a different binary, map state and camera pose, so
# they are not a controlled before/after. This script captures BOTH sides of
# the comparison off the exact same freshly-built voxelforge.exe, with the
# only difference being VOXELFORGE_LOOK_DISABLE. It reuses the same
# screenshot mechanism (main.rs screenshot_once, t=3.2s -> save -> exit).
#
# GATE (mechanical, no by-eye grading — matches gate3_shoot.sh's rule):
#   exit 0 · PNG on disk >= 2 KiB · "SHOT saved" in stdout · no panic.
#
# Every plate gets a `.runlog` recording the exe's mtime + SHA-256 so a
# reviewer can prove which binary produced it — the mtime/sha are computed
# ONCE at the top of the run and stamped on every plate from this invocation.
set -euo pipefail

BIN="${BIN:-./target/release/voxelforge.exe}"
QUALITY="${QUALITY:-high}"
OUT="${OUT:-docs/assets/gate3}"

mkdir -p "$OUT"

if [ ! -f "$BIN" ]; then
  echo "EXE NOT FOUND: $BIN" >&2
  exit 2
fi

EXE_MTIME="$(date -r "$BIN" '+%Y-%m-%dT%H:%M:%S%z' 2>/dev/null || echo unknown)"
EXE_SHA256="$(sha256sum "$BIN" | awk '{print $1}')"
COMMIT="$(git rev-parse --short=9 HEAD 2>/dev/null || echo unknown)"

echo "=== GATE3-LOOK — binary provenance ==="
echo "  bin:    $BIN"
echo "  mtime:  $EXE_MTIME"
echo "  sha256: $EXE_SHA256"
echo "  commit: $COMMIT"
echo "  quality (after side): $QUALITY"

FAILS=0

shoot() {
  # $1=png-stem $2=label $3=look(on|off) shift; $4..=extra env KEY=VALUE
  local stem="$1"; local label="$2"; local look="$3"; shift 3
  local png="$OUT/$stem"
  local runlog="$OUT/$stem.runlog"
  local log="$OUT/$stem.stdout.log"

  local -a env_kv=("$@")
  env_kv+=("VOXELFORGE_SHOT=$png")
  if [ "$look" = "off" ]; then
    env_kv+=("VOXELFORGE_LOOK_DISABLE=1")
  else
    env_kv+=("VOXELFORGE_LOOK_QUALITY=$QUALITY")
  fi

  echo ""
  echo "--- $label (look=$look) -> $png"
  env "${env_kv[@]}" "$BIN" >"$log" 2>&1
  local rc=$?

  {
    echo "exe: $BIN"
    echo "exe_mtime: $EXE_MTIME"
    echo "exe_sha256: $EXE_SHA256"
    echo "commit: $COMMIT"
    echo "flags: ${env_kv[*]}"
    echo "exit_code: $rc"
    echo "---"
    cat "$log"
  } > "$runlog"

  local bytes=0
  [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  echo "  exit=$rc bytes=$bytes"

  local bad=""
  [ "$rc" -ne 0 ]                    && bad="$bad exit=$rc"
  [ "$bytes" -lt 2048 ]              && bad="$bad no-png"
  grep -qE 'panicked|B0001' "$log"   && bad="$bad PANIC"
  grep -q 'SHOT saved'   "$log"      || bad="$bad no-SHOT"

  if [ -n "$bad" ]; then
    echo "  X FAIL:$bad"
    grep -nE 'panicked|B0001|error\[' "$log" | head -5 || true
    return 1
  fi
  grep -E 'LOOK_APPLIED|SHOT saved' "$log" || true
  echo "  OK PASS"
  return 0
}

# BOOT — spawn into the world, no input. Cleanest, most static comparison.
shoot "boot-before.png" "boot"  off "VOXELFORGE_PLAY=1" || FAILS=$((FAILS+1))
shoot "boot-after.png"  "boot"  on  "VOXELFORGE_PLAY=1" || FAILS=$((FAILS+1))

# WALK — --play-demo drives real input through the controller.
shoot "walk-before.png" "walk"  off "VOXELFORGE_PLAY_DEMO=1" || FAILS=$((FAILS+1))
shoot "walk-after.png"  "walk"  on  "VOXELFORGE_PLAY_DEMO=1" || FAILS=$((FAILS+1))

# COMBAT — husk + HUD on screen.
shoot "combat-before.png" "combat" off "VOXELFORGE_COMBAT_DEMO=1" || FAILS=$((FAILS+1))
shoot "combat-after.png"  "combat" on  "VOXELFORGE_COMBAT_DEMO=1" || FAILS=$((FAILS+1))

echo ""
echo "=== GATE3-LOOK — done: $((6-FAILS))/6 plates clean ==="
if [ "$FAILS" -eq 0 ]; then
  echo "GATE3_LOOK: PASS"
  exit 0
else
  echo "GATE3_LOOK: FAIL ($FAILS/6)"
  exit 1
fi
