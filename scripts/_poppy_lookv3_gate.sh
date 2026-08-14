#!/usr/bin/env bash
# Poppy — the gate that stands between a fresh exe and the v3 before/after plates.
#
# WHY THIS EXISTS. `_poppy_lookv3_shoot.cmd` shoots both sides of every pair off
# ONE binary and moves only VOXELFORGE_LOOK_GEN. That is the right design, and it
# has exactly one failure mode: if the exe pre-dates the v3 rig, then GEN=v3 and
# GEN=v2 run the SAME code, the harness writes two frames that differ by nothing
# but sampling noise, and the pair reads as "the change did nothing" when what it
# actually shows is "the change was never in the binary".
#
# mtime DOES NOT PROVE IT. This lane has already been burnt by a build log whose
# error was read off a source file rustc had loaded three minutes earlier — a
# timestamp is evidence about a file, not about what a compiler put in a binary.
# So the check is on the ARTEFACT: `LookFill::Rim` spawns a light literally named
# "look rim fill" (look.rs:2382), and `rim_lux()` reads an env var literally named
# VOXELFORGE_LOOK_RIM. Both are string literals, so both survive into .rdata and
# both are greppable. An exe carrying them ran a compiler over the v3 rig; an exe
# without them did not, whatever its mtime says.
#
# Usage:  bash scripts/_poppy_lookv3_gate.sh [exe]
# Exit:   0 = safe to shoot   1 = exe is not v3   2 = no exe
set -uo pipefail
cd "$(dirname "$0")/.."

EXE="${1:-$PWD/target-poppy/perf/voxelforge.exe}"

if [[ ! -f "$EXE" ]]; then
  echo "GATE: FAIL — no exe at $EXE"
  exit 2
fi

echo "GATE: exe   $EXE"
echo "GATE: mtime $(date -r "$EXE" -Is)   size $(stat -c%s "$EXE")"
echo "GATE: sha256 $(sha256sum "$EXE" | awk '{print $1}')"

# -a: read the binary as text. Both markers, because one of them could plausibly
# arrive from somewhere else later; two that only the v3 rig writes cannot.
rim_name=$(grep -a -c 'look rim fill' "$EXE" || true)
rim_env=$(grep -a -c 'VOXELFORGE_LOOK_RIM' "$EXE" || true)
echo "GATE: marker 'look rim fill'      x$rim_name"
echo "GATE: marker 'VOXELFORGE_LOOK_RIM' x$rim_env"

if [[ "$rim_name" -gt 0 && "$rim_env" -gt 0 ]]; then
  echo "GATE: PASS — this binary carries the v3 rig; the pair will be a real pair."
  exit 0
fi

echo "GATE: FAIL — v3 markers absent. Shooting now would write two identical"
echo "GATE:        frames and label them before/after. Rebuild first."
exit 1
