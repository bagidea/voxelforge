#!/usr/bin/env bash
# Poppy — the "this exe really is HEAD" receipt.
#
# The scar this exists for: `_poppy_look/before-eff6c7b-stale-exe` — a set of look
# frames shot through a binary that predated the commit they were credited to. The
# check that would have caught it is cheap, so it is now mandatory and automatic.
#
# Three facts, printed together, because any one alone can lie:
#   1. WHICH SOURCE — the worktree the exe was built from, its HEAD, and its
#      `git status` (mtime says nothing about which tree the compiler read).
#   2. WHEN        — exe mtime vs the HEAD commit time. exe older than the commit
#      => stale, hard FAIL.
#   3. WHAT        — sha256 of the exe, so the frames can be tied to this exact
#      binary later.
set -uo pipefail
REPO="E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge"
WT="$REPO/_poppy_head_wt"
EXE="$REPO/target-poppy/release/voxelforge.exe"

echo "=== SHOWCASE EXE PROVENANCE — $(date -Is) ==="
echo
echo "-- 1. source tree the exe was built from"
echo "worktree      : $WT"
HEAD_SHA=$(git -C "$WT" rev-parse HEAD)
HEAD_EPOCH=$(git -C "$WT" log -1 --format=%ct)
HEAD_ISO=$(git -C "$WT" log -1 --format=%cI)
echo "HEAD          : $HEAD_SHA"
echo "HEAD subject  : $(git -C "$WT" log -1 --format=%s)"
echo "HEAD committed: $HEAD_ISO"
echo "worktree diff vs HEAD (must be EMPTY — the exe has to come from committed"
echo "source, not from a local hunk nobody else can reproduce):"
git -C "$WT" status --porcelain | sed 's/^/                /'
git -C "$WT" diff --stat | sed 's/^/                /'
echo

echo "-- 2. exe vs HEAD in time"
EXE_EPOCH=$(stat -c %Y "$EXE")
echo "exe           : $EXE"
echo "exe mtime     : $(stat -c %y "$EXE")"
DELTA=$(( EXE_EPOCH - HEAD_EPOCH ))
if [[ $DELTA -ge 0 ]]; then
  echo "VERDICT       : PASS — exe is ${DELTA}s NEWER than the HEAD commit"
else
  echo "VERDICT       : FAIL — exe is $(( -DELTA ))s OLDER than the HEAD commit (STALE)"
fi
echo

echo "-- 3. exe identity"
echo "size          : $(stat -c %s "$EXE") bytes"
echo "sha256        : $(sha256sum "$EXE" | cut -d' ' -f1)"
echo
echo "-- 4. build log verdict (grep '^error', never a pipe exit code)"
echo "errors        : $(grep -c '^error' "$REPO/_poppy_showcase_build.log")"
grep -E '^(BUILD_START|BUILD_EXIT|    Finished)' "$REPO/_poppy_showcase_build.log" | sed 's/^/                /'
