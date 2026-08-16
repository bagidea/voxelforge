#!/usr/bin/env bash
# _pixel_g2g4_ab.sh -- BEFORE/AFTER plates for gates G2 + G4 (pixel lane, 2026-08-16).
#
# Closing the two gates docs/VERDICT-flamingo-beauty-2026-08-16.md left open:
#   G2  "ไม่มีแถบ mullion ทาบพื้น/ผนังเลยสักเส้น" -- the emissive window pane was an
#       opaque cube wall bolted over the opening from outside, so it sat in the shadow
#       map and stopped the key light one block before the mullions. Fixed by spawning
#       the pane NotShadowCaster (hero.rs `VoxelGrid::fill_nocast`).
#   G4  penumbra median 4px (line >=5, ref 8px) + "บล็อกมิ้นต์/ชามไม่มีเงาสัมผัส นั่งลอย".
#       Fixed by halving the directional shadow map 4096 -> 2048 (the Temporal filter's
#       kernel is denominated in shadow-map TEXELS, so penumbra scales with texel
#       world-size; PCSS `soft_shadow_size` measured FLAT here) and by adding
#       `ContactShadows` to the camera for the grounding seam SSAO structurally cannot
#       draw.
#
# ONE BINARY, SIX PLATES. Every plate below comes out of the SAME exe; the only thing
# that differs is env. That is deliberate and it is the whole evidentiary point: a
# before/after pair shot from two different builds cannot distinguish "the fix worked"
# from "something else changed in between", and this repo has been bitten by exactly
# that (a commit clock is not a build clock -- two binaries 30 bytes apart in 80.7 MB
# were link stamps only). The BEFORE levers restore the old behaviour bit-for-bit:
#   VOXELFORGE_G2_PANE=block    pane occludes the sun again
#   VOXELFORGE_SHADOWMAP=4096   the shot bin's old shadow-map size
#   VOXELFORGE_G4_CONTACT=off   inserts no ContactShadows component at all
#
# Plates:
#   before      G2 block + 4096 + contact off  -- reproduces the graded FAIL plate
#   before-r2   identical repeat               -- CAPTURE NOISE FLOOR for the before side
#   after       NO env at all                  -- the baked recipe; the deliverable
#   after-r2    identical repeat               -- noise floor for the after side
#   g2-only     pane pass, 4096, contact off   -- attribution: bars are G2's doing alone
#   g4-only     pane block, 2048, contact on   -- attribution: penumbra is G4's doing alone
#
# The two -r2 plates exist so a diff of N pixels can be read. Without a measured floor,
# "the after differs from the before by 400k px" is not evidence of anything -- it is a
# number with no zero.
set -u
cd "$(dirname "$0")/.."

EXE="${EXE:-target-pixel/release/voxelforge_shot.exe}"
OUT="${OUT:-_fl_g2g4_20260816}"
[[ -f "$EXE" ]] || {
  echo "NO EXE $EXE -- run: .\\scripts\\lane-build.ps1 -Lane pixel -Profile release -Bin voxelforge_shot"
  exit 2
}
mkdir -p "$OUT"

echo "EXE   $EXE"
echo "size  $(stat -c%s "$EXE")b"
echo "mtime $(stat -c%y "$EXE")"
echo "md5   $(md5sum "$EXE" | cut -d' ' -f1)"
echo "rev   $(git rev-parse --short HEAD)"
echo

shoot() {
  local name="$1"
  shift
  local out="$OUT/$name-nohud2.png"
  rm -f "$out"
  env "$@" VOXELFORGE_SHOT="$out" "$EXE" >"$OUT/$name.runlog" 2>&1
  local code=$?
  if [[ -f "$out" ]]; then
    # Echo the three lines hero.rs prints for THIS plate's levers. A plate whose
    # runlog says G2_PANE=block while its caption says "after" is caught here, not
    # by a reviewer squinting at the floor.
    local levers
    levers=$(grep -hE '^(G2_PANE|SHADOW_MAP|G4_CONTACT|VOXEL_NOCAST)=' "$OUT/$name.runlog" | tr '\n' ' ')
    echo "OK   $out ($(stat -c%s "$out")b) exit=$code  [$levers]"
  else
    echo "MISS $out exit=$code"
    return 1
  fi
}

BEFORE=(VOXELFORGE_G2_PANE=block VOXELFORGE_SHADOWMAP=4096 VOXELFORGE_G4_CONTACT=off)

shoot before "${BEFORE[@]}"
shoot before-r2 "${BEFORE[@]}"

# NO env. Not one look knob. This is the plate the bake has to stand on.
shoot after
shoot after-r2

shoot g2-only VOXELFORGE_SHADOWMAP=4096 VOXELFORGE_G4_CONTACT=off
shoot g4-only VOXELFORGE_G2_PANE=block

echo
echo "== grade_look.py per plate =="
for p in before after g2-only g4-only; do
  f="$OUT/$p-nohud2.png"
  [[ -f "$f" ]] || continue
  python scripts/grade_look.py "$f" >"$OUT/$p.grade.log" 2>&1
  echo "-- $p (exit $?)"
  grep -E 'G2 |G4a|penumbra|edges found|-> (PASS|FAIL)' "$OUT/$p.grade.log" | sed 's/^/   /'
done

echo
echo "== noise floor + before/after deltas =="
# `_pixel_bake_diff.py` takes LABEL=a.png:b.png (it partitions on '=' first) -- an
# unlabelled a:b silently degrades to two empty paths, so the labels are load-bearing.
python scripts/_pixel_bake_diff.py \
  "FLOOR-before=$OUT/before-nohud2.png:$OUT/before-r2-nohud2.png" \
  "FLOOR-after=$OUT/after-nohud2.png:$OUT/after-r2-nohud2.png" \
  "before-vs-after=$OUT/before-nohud2.png:$OUT/after-nohud2.png" \
  "before-vs-g2only=$OUT/before-nohud2.png:$OUT/g2-only-nohud2.png" \
  "before-vs-g4only=$OUT/before-nohud2.png:$OUT/g4-only-nohud2.png"

echo DONE
