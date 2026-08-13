#!/usr/bin/env bash
# The sky before/after, isolated — ONE binary, one map, one camera.
#
# WHY NOT THE TWO EXES. The obvious pair (release exe from 05:35 = pre-fix,
# release exe from 06:15 = fixed) does not straddle the fix alone: 72408ad
# "one source of truth per block colour" and 3262c6e landed in between, so every
# voxel's colour moved too. `_poppy_sky_drift_probe.py` measures the damage —
# ~11-13 levels of mean |diff| in the bottom fifth of the frame, which the dome
# cannot reach. That pair can never answer "what did the dome do".
#
# So reproduce the pre-fix dome from the SHIPPED binary instead. `exp_comp`
# multiplied both dome stops by 1/exposure() = 1513.4; `sky_gain` multiplies
# both stops too (zenith `h.sky * h.sky_gain`, look.rs:1905; horizon
# `haze_color()`, whose scale is `HAZE_GAIN * h.sky_gain`, look.rs:1264). So
#
#     VOXELFORGE_LOOK_SKYGAIN = 2.4 * 1513.4 = 3632.16
#
# puts the dome back at exactly the radiance it shipped at, out of the binary
# that contains the fix. Nothing else about the build differs — same blocks,
# same map, same controller.
#
# ONE side effect has to be neutralised: `haze_color()` is also the DistanceFog
# colour for GEOMETRY (look.rs:1278), so a 1513x sky_gain would blow the terrain
# too and the two frames would stop being comparable outside the sky. Both shots
# therefore run with the haze pushed past the world (`VOXELFORGE_LOOK_FOG` wins
# outright in `haze_falloff`, look.rs:1190) and the volumetric medium off — in
# BOTH sides, so it cancels. The angle guard in `_poppy_sky_after_plate.py` is
# what checks that this actually held.
#
#   BIN=./target/release/voxelforge.exe bash scripts/_poppy_sky_gain_ab.sh
set -uo pipefail

BIN="${BIN:-./target/release/voxelforge.exe}"
OUT="${OUT:-docs/assets}"
LOGS="${LOGS:-_poppy_proof}"
PREFIX_GAIN="${PREFIX_GAIN:-3632.16}"   # 2.4 * 1513.4

mkdir -p "$OUT" "$LOGS"
[ -f "$BIN" ] || { echo "✗ binary not found: $BIN" >&2; exit 1; }

shoot() { # <png> <label> <env...>
  local png="$1" label="$2"; shift 2
  local log="$LOGS/${label}.log"
  echo "=== $label -> $png ==="
  env "$@" \
      VOXELFORGE_PLAY=1 \
      VOXELFORGE_SHOT="$png" \
      VOXELFORGE_LOOK_FOG=100000,200000 \
      VOXELFORGE_LOOK_VFOG=off \
      "$BIN" >"$log" 2>&1
  local rc=$?
  local bytes=0; [ -f "$png" ] && bytes=$(wc -c <"$png" | tr -d ' ')
  grep -aE 'SCENE_READY|SHOT saved' "$log" || true
  echo "  exit=$rc bytes=$bytes"
  [ "$rc" -eq 0 ] && [ "$bytes" -gt 1024 ] || { echo "  ✗ FAIL"; return 1; }
  grep -aqE 'panicked|B0001' "$log" && { echo "  ✗ PANIC"; return 1; }
  return 0
}

fails=0
# A — the dome at the radiance it shipped at before 5eba1e8.
shoot "$OUT/_poppy_sky_ab_prefix.png" ab-prefix \
      VOXELFORGE_LOOK_SKYGAIN="$PREFIX_GAIN" || fails=$((fails + 1))
# B — the dome as it ships now (sky_gain default 2.4, exp_comp 1.0).
shoot "$OUT/_poppy_sky_ab_fixed.png" ab-fixed || fails=$((fails + 1))
# C — B again with the dome forced to `base_color: BLACK` (look.rs:1988). Not a
# beauty shot: it is the only way to find out WHICH pixels in B are the dome.
# `sky_mask` is a colour range (`R>=250 & B>=170`) and this camera stands inside
# a house, so it also selects sunlit sandstone — 88.4% of it, measured. The
# plate refuses to write without this frame, because without it the "% still
# pinned at R>=250" is a statement about masonry.
shoot "$OUT/_poppy_sky_ab_probe.png" ab-probe \
      VOXELFORGE_LOOK_SKYPROBE=1 || fails=$((fails + 1))

echo
[ "$fails" -eq 0 ] && echo "SKY_AB: both frames captured" || echo "SKY_AB: FAIL — $fails shot(s) bad"
exit "$fails"
