#!/usr/bin/env bash
# Poppy — the look-v2 before/after plates.
#
# ONE BINARY, ONE CAMERA. The pair is shot off the same
# `target-poppy/perf/voxelforge.exe`, so "before" is not a second build carrying
# a second set of unrelated drift. Every other VOXELFORGE_LOOK_* knob is
# explicitly unset below, so what lands on disk is the SHIPPED look of each
# generation rather than a sweep row that happened to flatter one side.
#
# WHY THE v1 SIDE SETS THREE ENV VARS AND NOT ONE. `VOXELFORGE_LOOK_GEN=v1`
# reverts exactly what it documents: the fill rig (flat ambient, no sky-fill, no
# bounce) and the PCSS width/tier. It does NOT revert the two other things this
# change added — the Bevy atmosphere (`_LOOK_ATMOS`, default `lut`) and the
# gradient sky dome (`_LOOK_SKYGRAD`) — because those hang off their own hooks.
# Left at their defaults the "before" plate would render the NEW sky behind the
# OLD fill: a frame that never shipped, flattering the after shot by hiding half
# of what changed. So the before side is pinned to the pre-change sky as well:
#   GEN=v1  ATMOS=off  SKYGRAD=off   ==  flat ClearColor sky + flat ambient fill.
# The env diff printed at the end names all three, so this is on the record with
# the plates instead of buried here.
#
# Three scenes, because the change has three claims and each needs its own frame:
#   noon   — high sun, open ground: the sky-fill + bounce split against a flat term
#   shade  — castle interior, low boom: shaded faces split by orientation (and the
#            AO/contact layers finally having a directional fill to read against)
#   dusk   — the shipped 22-deg golden hour: sky, haze, sun glow, PCSS softening
set -uo pipefail
cd "$(dirname "$0")/.."

ROOT="$PWD"
# Overridable ONLY so the harness itself can be rehearsed against an older exe
# while the real one links — a dry run proves the env/shot/map plumbing without
# spending a build slot. The plates the CEO grades always come from the default.
EXE="${VOXELFORGE_AB_EXE:-$ROOT/target-poppy/perf/voxelforge.exe}"
OUT="${VOXELFORGE_AB_OUT:-$ROOT/_poppy_lookv2_ab}"
SCENES=(${VOXELFORGE_AB_SCENES:-noon shade dusk})
GENS=(${VOXELFORGE_AB_GENS:-v1 v2})
mkdir -p "$OUT"

[[ -f "$EXE" ]] || { echo "NO EXE at $EXE"; exit 2; }
echo "exe: $(ls -la --time-style='+%Y-%m-%d %H:%M:%S' "$EXE")"

# Every tuning knob that would make a frame something other than the shipped
# look of its generation. `_LOOK_GEN` is deliberately NOT in this list.
UNSET=(-u VOXELFORGE_LOOK_EXPOSURE -u VOXELFORGE_LOOK_SKY -u VOXELFORGE_LOOK_SKYGAIN
       -u VOXELFORGE_LOOK_AMBIENT -u VOXELFORGE_LOOK_FILL -u VOXELFORGE_LOOK_GRADE
       -u VOXELFORGE_LOOK_LIGHT -u VOXELFORGE_LOOK_FOG -u VOXELFORGE_LOOK_NIGHT
       -u VOXELFORGE_LOOK_FORCE -u VOXELFORGE_LOOK_DISABLE -u VOXELFORGE_LOOK_SSAO
       -u VOXELFORGE_LOOK_CONTACT -u VOXELFORGE_LOOK_PCSS -u VOXELFORGE_LOOK_HAZE
       -u VOXELFORGE_LOOK_ATMOS -u VOXELFORGE_LOOK_VFOG -u VOXELFORGE_LOOK_SKYGRAD
       -u VOXELFORGE_GRADE -u VOXELFORGE_EXPOSURE -u VOXELFORGE_AMBIENT
       -u VOXELFORGE_SUN -u VOXELFORGE_FOG -u VOXELFORGE_BOUNCE)

# shot <scene> <gen> <extra env pairs...>
shot() {
  local scene=$1 gen=$2; shift 2
  local png="$OUT/$scene-$gen.png"
  rm -f "$png"
  printf '%-14s %-3s ' "$scene" "$gen"

  # The generation, whole — fill rig AND sky. See the header note.
  local genenv=()
  [[ $gen == v1 ]] && genenv=(VOXELFORGE_LOOK_GEN=v1
                              VOXELFORGE_LOOK_ATMOS=off
                              VOXELFORGE_LOOK_SKYGRAD=off)

  # The env is dumped by the very process that execs the binary, so "only GEN
  # moved" is evidence on disk rather than a claim in this comment.
  env "${UNSET[@]}" VOXELFORGE_PLAY=1 VOXELFORGE_NOHUD=1 VOXELFORGE_SEED=42 \
      VOXELFORGE_SHOT="$png" "${genenv[@]}" "$@" \
      bash -c 'env | grep "^VOXELFORGE" | sort > "$1"; exec "$2"' _ \
        "$OUT/$scene-$gen.env.txt" "$EXE" \
        > "$OUT/$scene-$gen.out.log" 2> "$OUT/$scene-$gen.err.log"
  local code=$?

  if [[ -f "$png" ]]; then
    echo "OK  $(stat -c%s "$png")b exit=$code"
  else
    echo "MISS exit=$code"; tail -4 "$OUT/$scene-$gen.err.log" | sed 's/^/      /'
  fi
}

# --- the three framings -----------------------------------------------------
# VOXELFORGE_LOOK_CAM = yaw_deg, pitch_deg, boom_dist.
# VOXELFORGE_LOOK_SUN = elev_deg, azim_deg, illuminance.
#
# ALL THREE LOAD castle.json, AND THAT IS A MEASURED CHOICE, NOT A PREFERENCE.
# The default seed-42 world spawns the avatar at the floor of a canyon: dry runs
# of `35,-18,26` and `48,-12,30` against the 13:29 exe both came back with the
# boom buried in terrain, one wall filling 70 % of frame and no horizon at all
# (`_poppy_lookv2_dryrun/noon-v2.png`, `dusk-v2.png`). A light rig cannot be
# judged in a frame with no sky and no lit ground in it. The castle courtyard has
# what these plates have to show: open sunlit ground, tall walls throwing real
# shade, a horizon line, and characters for scale.
#
# noon holds GOLDEN's azimuth and illuminance and moves ONLY the elevation, so
# the scene reads as midday without also becoming a brightness A/B.
MAP=maps/castle.json
NOON_CAM=35,-30,48 ;  NOON_SUN=62,205,22000   # up and back: courtyard + horizon
SHADE_CAM=20,-6,11                            # low + close: the boom sits under the walls
DUSK_CAM=48,-20,40                            # sun-side yaw so the haze glow is in frame

has() { [[ " ${SCENES[*]} " == *" $1 "* ]]; }

for gen in "${GENS[@]}"; do
  has noon  && shot noon  "$gen" VOXELFORGE_MAP_LOAD=$MAP VOXELFORGE_LOOK_CAM=$NOON_CAM \
                                 VOXELFORGE_LOOK_SUN=$NOON_SUN
  has shade && shot shade "$gen" VOXELFORGE_MAP_LOAD=$MAP VOXELFORGE_LOOK_CAM=$SHADE_CAM
  has dusk  && shot dusk  "$gen" VOXELFORGE_MAP_LOAD=$MAP VOXELFORGE_LOOK_CAM=$DUSK_CAM
done

echo
echo "=== what actually differs per pair (should be GEN + ATMOS + SKYGRAD only) ==="
for s in "${SCENES[@]}"; do
  if [[ -f "$OUT/$s-v1.env.txt" && -f "$OUT/$s-v2.env.txt" ]]; then
    printf '%-6s %s\n' "$s" "$(diff "$OUT/$s-v1.env.txt" "$OUT/$s-v2.env.txt" | grep '^[<>]' | tr '\n' ' ')"
  fi
done

echo
echo "=== sheets ==="
python scripts/_poppy_lookv2_sheet.py "$OUT" "${SCENES[@]}"
