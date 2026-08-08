#!/usr/bin/env bash
# Poppy — shoot showcase frames through the REAL gameplay camera.
#
# Runs the showcase exe (built from the pristine HEAD worktree by
# _poppy_showcase_build.sh) once per plan line, from inside the worktree so the
# map it boots is HEAD's `maps/edhari.json` and not the dirty checkout's.
#
# Every VOXELFORGE_LOOK_* tuning knob is explicitly unset unless a plan line asks
# for it, so what lands on disk is the SHIPPED look — not a sweep row that happens
# to flatter the frame. The env the binary actually saw is dumped by the process
# that execs it, so "no overrides" is evidence, not a promise.
#
# Plan file: one line per shot — "label|yaw,pitch,dist|KEY=value;KEY=value"
#            (third field optional; '#' comments and blank lines skipped)
# Usage: bash scripts/_poppy_showcase_shoot.sh <plan> <outdir>
set -uo pipefail
REPO="E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge"
PLAN="$1"
# Absolute, always: the run subshell `cd`s into the worktree so the binary boots
# HEAD's maps/, and every relative out-path handed to it from the repo root dies
# there ("No such file or directory", zero frames, no obvious cause).
mkdir -p "$2"
OUT="$(cd "$2" && pwd)"
WT="$REPO/_poppy_head_wt"
EXE="$REPO/target-poppy/release/voxelforge.exe"

[[ -f "$EXE" ]] || { echo "NO EXE at $EXE"; exit 2; }


# Knobs that would make the frame something other than the shipped look.
UNSET=(-u VOXELFORGE_LOOK_EXPOSURE -u VOXELFORGE_LOOK_SKY -u VOXELFORGE_LOOK_AMBIENT
       -u VOXELFORGE_LOOK_SUN -u VOXELFORGE_LOOK_GRADE -u VOXELFORGE_LOOK_LIGHT
       -u VOXELFORGE_LOOK_FOG -u VOXELFORGE_LOOK_NIGHT -u VOXELFORGE_LOOK_FORCE
       -u VOXELFORGE_LOOK_DISABLE -u VOXELFORGE_LOOK_QUALITY)

while IFS= read -r line; do
  [[ -z "${line// }" || "${line:0:1}" == "#" ]] && continue
  label=$(cut -d'|' -f1 <<<"$line" | tr -d ' ')
  cam=$(cut -d'|' -f2 <<<"$line" | tr -d ' ')
  extra=$(cut -d'|' -f3 -s <<<"$line")

  png="$OUT/$label.png"
  rm -f "$png"

  # Per-line extras arrive as KEY=v;KEY=v -> a real argv array for `env`.
  extra_env=()
  if [[ -n "${extra// }" ]]; then
    IFS=';' read -ra kvs <<<"$extra"
    for kv in "${kvs[@]}"; do
      [[ -n "${kv// }" ]] && extra_env+=("$(tr -d ' ' <<<"$kv")")
    done
  fi

  ( cd "$WT" && env "${UNSET[@]}" \
      VOXELFORGE_PLAY=1 VOXELFORGE_LOOK_CAM="$cam" VOXELFORGE_SHOT="$png" \
      "${extra_env[@]}" \
      bash -c 'env | grep "^VOXELFORGE" | sort > "$1"; exec "$2"' _ \
        "$OUT/$label.env.txt" "$EXE" \
        > "$OUT/$label.out.log" 2> "$OUT/$label.err.log" )
  code=$?

  if [[ -f "$png" ]]; then
    echo "OK   $label  cam=$cam  extra=${extra:-none}  ($(stat -c%s "$png")b) exit=$code"
  else
    echo "MISS $label  cam=$cam  exit=$code"
    tail -4 "$OUT/$label.err.log"
  fi
done < "$PLAN"

echo "SHOOT_DONE $(date -Is)"
