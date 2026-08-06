#!/usr/bin/env bash
# Runtime proof that gate3_shoot.sh's newest_source_mtime guard function is
# defined, callable, and returns a real (>0) mtime — so MIN_MTIME is never empty
# (an empty MIN_MTIME was the silent-pass hole). Extracts the function straight
# out of the file under test, no copy.
set -u
cd "E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge" || exit 11

if ! grep -q '^newest_source_mtime() {' scripts/gate3_shoot.sh; then
  echo "FAIL: newest_source_mtime not defined"; exit 1
fi
# Eval only the function block (awk on the brace-delimited range).
eval "$(awk '/^newest_source_mtime\(\) \{/,/^}$/' scripts/gate3_shoot.sh)" || { echo "FAIL: eval"; exit 2; }

v=$(newest_source_mtime)
echo "newest_source_mtime() => $v"
if [ "${v:-0}" -gt 0 ] 2>/dev/null; then
  echo "OK: returns >0 — guard will hold"
  exit 0
else
  echo "BAD: returns empty or <=0 — guard still broken"
  exit 3
fi
