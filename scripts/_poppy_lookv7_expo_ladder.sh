#!/usr/bin/env bash
# Poppy -- v7: the ev100 ladder, on ONE binary, graded rung by rung.
#
# THE QUESTION: Rose tuned ev100 8.6. Does it survive on all three plates, or
# only on the one it was tuned against? An exposure value accepted off a single
# plate is the exact shape of the "it passed, on the frame I picked" failure, so
# every rung here is shot on all three scenes and graded by the same gate.
#
# ONE BINARY, EVERY RUNG. `VOXELFORGE_LOOK_EXPOSURE` is read in look.rs AFTER
# the v3 fork, so driving it reproduces exactly what a v3-only `EV100_V3`
# constant does to the pixels. A ladder that relinked per rung would be
# measuring the link.
#
# THE BEFORE LEGS ARE SHOT ONCE AND COPIED. They are v2 at the scene's own
# baseline exposure and this change cannot touch them; re-shooting them per rung
# would spend 9 boots proving they are identical. Copied, and the gate's
# before-column then doubles as the instrument control on every rung.
#
# Usage: bash scripts/_poppy_lookv7_expo_ladder.sh [exe]
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
EXE="${1:-$ROOT/target-poppy/perf/voxelforge.exe}"
SHOOT="$ROOT/scripts/_poppy_lookv7_expo_shoot.cmd"
LAD="$ROOT/_poppy_lookv7/ladder"
SCENES=(outdoor-noon evening-raking night-firelit)

[[ -f "$EXE" ]] || { echo "no exe at $EXE"; exit 1; }
echo "EXE   $EXE"
echo "SHA   $(sha256sum "$EXE" | awk '{print $1}')"
echo "MTIME $(date -r "$EXE" '+%F %T')"

run() {  # run <name> <ev_noon> <ev_eve> <ev_night> <mode>
  local name=$1 evn=$2 eve=$3 evg=$4 mode=$5
  local dir="$LAD/$name"
  echo
  echo "=== rung $name : noon=$evn evening=$eve night=$evg (mode=$mode) ==="
  mkdir -p "$dir"
  local t0=$SECONDS
  cmd //c "$(cygpath -w "$SHOOT")" "$(cygpath -w "$EXE")" "$(cygpath -w "$dir")" \
      "$evn" "$eve" "$evg" "$mode" >/dev/null 2>&1
  [[ -f "$dir/_shoot.done" ]] || { echo "  rung $name: no _shoot.done -- capture died"; return 1; }
  if [[ "$mode" == "after" ]]; then
    for s in "${SCENES[@]}"; do cp "$LAD/base/${s}_before.png" "$dir/${s}_before.png"; done
  fi
  local n
  n=$(ls "$dir"/*_before.png "$dir"/*_after.png 2>/dev/null | wc -l)
  echo "  ${n} frames, $((SECONDS - t0))s"
  [[ "$n" -eq 6 ]] || { echo "  rung $name: expected 6 frames, not grading it"; return 1; }
  # `$?` AFTER AN `echo` IS THE ECHO'S. Written as `python ...; echo "EXIT=$?"`
  # this function always returned 0, so `run base ... || exit 1` could not catch
  # a FAILING CONTROL RUNG and the ladder would keep shooting 8.6/9.2/9.8 on a
  # broken base. Captured first, returned last. (The 8.6/9.2/9.8 rungs are
  # EXPECTED to fail -- that is the measurement -- so only `base` is fatal.)
  python scripts/_poppy_lookv7_gate.py "$dir" "ev100 $name"
  local rc=$?
  echo "  RUNG $name GATE EXIT=$rc"
  return $rc
}

# base = every scene at its own shipped exposure (noon 10.6 by scene pin,
# evening 10.3 = Hour::GOLDEN, night 7.5 = Hour::NIGHT). This is the control:
# the after leg here differs from the before leg ONLY by the v2/v3 rig.
run base base base base both || exit 1

# THE ARTEFACT PROOF, AT RUNTIME AND NOT BY `strings`. This round adds `ev100=`
# to the LOOK_FILL provenance line, and the OTHER line that carries `ev100=`
# (the sky-dome spawn) is already in the old binary -- so a .rdata scan for that
# token cannot tell the two exes apart. The frame's own log can: only a binary
# built from this source prints `ev100=` on a LOOK_FILL line. If it is missing,
# the ladder is being shot by the exe that shot the v6 plates and every number
# below would be captioned with the wrong source.
echo
echo "=== artefact proof: LOOK_FILL carries ev100 (this source, not the v6 exe) ==="
if grep -q 'LOOK_FILL .*ev100=' "$LAD/base/_shoot.log"; then
  grep -ao 'LOOK_FILL gen=[A-Za-z0-9]* ambient=[0-9]* .*ev100=[0-9.]*' "$LAD/base/_shoot.log" \
    | sed 's/.*\(gen=[A-Za-z0-9]*\) \(ambient=[0-9]*\).*\(ev100=[0-9.]*\)/  \1 \2 \3/' \
    | sort | uniq -c
else
  echo "  FAIL: no LOOK_FILL line carries ev100 -- this is the OLD binary. Stopping."
  exit 1
fi


# --- ladder A: ONE ABSOLUTE ev100 for every hour ---------------------------
# This is what "set ev100 to 8.6" literally means, and 8.6 is the rung under
# test. 9.2 and 9.8 walk it back toward the day baselines so the failure, if
# there is one, has a slope attached instead of being a single verdict.
for ev in 8.6 9.2 9.8; do
  run "abs-$ev" "$ev" "$ev" "$ev" after
done

# --- ladder B: ONE DELTA, applied to each hour's OWN ev100 -----------------
# `Hour::GOLDEN` exposes at 10.3 and `Hour::NIGHT` at 7.5 -- 2.8 EV apart, and
# deliberately so (the NIGHT note in look.rs is explicit that a night frame's
# only warm sources are the fire and the lanterns). So a SINGLE absolute number
# cannot be right for both by construction, and ladder A is expected to prove
# that rather than to find a winner. Ladder B is the shape that can: keep the
# hours' relationship and move all of them together.
#
# d-1.70 IS ROSE'S NUMBER, READ AS AN INTENT INSTEAD OF A CONSTANT. 8.6 is
# exactly `Hour::GOLDEN`'s 10.3 minus 1.70, i.e. "the evening plate, 1.7 stops
# brighter". This rung asks whether that intent survives when it is applied to
# every hour instead of pinned to the one plate it was tuned on.
#                 noon(10.6)  evening(10.3)  night(7.5)
run "d-0.50"        10.1          9.8           7.0   after
run "d-1.00"         9.6          9.3           6.5   after
run "d-1.70"         8.9          8.6           5.8   after
