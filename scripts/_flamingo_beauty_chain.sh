#!/usr/bin/env bash
# Chain build -> pose probe, detached.
#
# Office scar (wait-for-fresh-exe-then-run): a build kicked off in one tool call
# and consumed in the next can be orphaned by session teardown, and polling it by
# hand wastes the whole link window. So the waiter that detects the link ALSO
# launches the next step, in the same detached process.
#
# Build is judged by `grep -c '^error'` only -- the director pinned that; a tail
# of a cargo log shows warnings and reads like a failure when nothing failed.
set -u
cd "E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge" || exit 1
WT=_flamingo_beauty_wt
EXE="$WT/target-pixel/perf/voxelforge.exe"

while [ ! -f "$WT/_build.done" ]; do sleep 15; done

ERRS=$(grep -c '^error' "$WT/_build.log")
echo "BUILD DONE  errors=$ERRS"
grep '^EXIT=' "$WT/_build.log"

if [ "$ERRS" -ne 0 ]; then echo "BUILD HAS ERRORS - stopping"; exit 1; fi
if [ ! -f "$EXE" ]; then echo "NO EXE AT $EXE - stopping"; exit 1; fi

ls -l "$EXE"
echo "--- launching pose probe ---"
cmd //c "scripts\\_flamingo_beauty_probe.cmd"
echo "PROBE plates: $(ls docs/assets/look/beauty/probe/*.png 2>/dev/null | wc -l)"
