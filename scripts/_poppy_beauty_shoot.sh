#!/usr/bin/env bash
# Poppy — beauty-shot capture off the green flamingo build (no cargo; Rose holds
# the build lane). Every frame comes out of the REAL gameplay camera through
# `--play` + VOXELFORGE_SHOT, same lane docs/look-acceptance-rubric.md grades.
set -uo pipefail
cd "$(dirname "$0")/.."
EXE=target-flamingo/release/voxelforge.exe
OUT=_poppy_beauty
mkdir -p "$OUT"
[[ -f "$EXE" ]] || { echo "NO EXE $EXE"; exit 2; }

shoot() { # name  extra-env...
  local name=$1; shift
  env "$@" VOXELFORGE_PLAY=1 VOXELFORGE_SHOT="$OUT/$name.png" \
    "$EXE" >"$OUT/$name.log" 2>&1
  local code=$?
  if [[ -f "$OUT/$name.png" ]]; then
    echo "OK   $name ($(stat -c%s "$OUT/$name.png")b) exit=$code"
  else
    echo "MISS $name exit=$code"; tail -3 "$OUT/$name.log"
  fi
}

for spec in "$@"; do
  name=${spec%%:*}
  rest=${spec#*:}
  # shellcheck disable=SC2086
  shoot "$name" $rest
done
