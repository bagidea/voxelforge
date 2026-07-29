#!/usr/bin/env bash
# Native CONTROL frame for the web-parity v3 pair (docs/web-parity-checklist.md §2.6).
#
# §2.6 originally specified "VOXELFORGE_HERO=1 and no other VOXELFORGE_* at all",
# because at the time the wasm `read_cfg()` hard-coded every knob to None and the web
# was *forced* onto baked defaults. That debt (W0-C) is paid: `read_cfg()` on wasm now
# parses the same knobs off the query string. So the matched pair is
# "same recipe on both sides" — env here, query string there — which is what §2.6 was
# after ("ต้องเป็น...ชุดเดียวกับที่ web ถูกบังคับให้ใช้ ไม่งั้นเทียบคนละสูตร").
#
# THE KNOBS ARE NOT WRITTEN HERE ON PURPOSE. They come from
# `scripts/hero_recipe.py --env`, which parses scripts/render_wide_hero.sh — the one
# file the CEO-approved TILT-DOWN recipe actually lives in, and the file that
# produced the baseline docs/assets/wide-hero-final.png. scripts/verify_web_v3.sh
# renders the SAME dict as a query string from the SAME call.
#
# This used to be a hand-copied list taken from the older "Hero tilt-B" note. Same
# camera, but the pre-atmosphere-pin light: it silently dropped SHOULDER / DUST /
# BOUNCE / BOUNCE2 and carried the old SUN 20000 / AMBIENT 4400 / EXPOSURE 8.82 /
# GRADE 1.15. Measured against the baseline that control came out warmth +53.8,
# p95 196.6 (outside the rubric's own 150..185 band), micro −1.30, penumbra 2.92px
# (below G4's 3px bar) — i.e. the "control" was the least gradeable frame in the
# pair. W0-D in grade_web_parity.py now rejects exactly that; deriving the values
# instead of retyping them is what stops it happening again.
#
# BINARY: the MAIN binary (`voxelforge`, the one index.html builds to wasm) with
# VOXELFORGE_HERO=1, so the control runs the same code path as the web frame.
# render_wide_hero.sh uses `voxelforge_shot` instead (it predates main.rs building
# again). The swap is MEASURED equivalent, not assumed — same recipe through both
# exes, 2026-07-29: warmth 123.54 vs 123.61, blue/sat/p95 identical to 2dp, micro
# 4.96 vs 4.98. Re-check with VF_EXE=target/release/voxelforge_shot.exe.
set -euo pipefail
cd "$(dirname "$0")/.."

OUT="${1:-docs/assets/native-control-v3.png}"
EXE="${VF_EXE:-target/release/voxelforge.exe}"
[ -f "$EXE" ] || { echo "render_native_control_v3: no $EXE — cargo build --release" >&2; exit 2; }

# TAA + dust + volumetrics accumulate for 3.2s of WALL-CLOCK time before the grab
# (hero.rs / main.rs screenshot_once), so how many frames land inside that window
# depends on how loaded the box is. A second renderer or a rust build contending
# for CPU/GPU under-converges the capture, which reads exactly like a look
# regression and is not one — so refuse to shoot rather than hand in a frame that
# then fails W0-D. wasm-opt and a second voxelforge*.exe count as blockers too.
# (chrome is deliberately NOT listed — it is always running on this box; the W0-D
# check at the end of this script is the backstop for that case.)
#
# WHAT LOAD ACTUALLY COSTS, split from the CRLF bug it was first confused with
# (all four measured 2026-07-29, same exe, same recipe, warmth/micro vs baseline):
#   under wasm build + CRLF env   warmth −5.34   micro −0.95
#   idle             + CRLF env   warmth −5.13   micro −0.01
#   idle             + LF env     warmth −0.05   micro −0.02
# So the ~−5.1 warmth was NOT contention: it was `hero_recipe.py` emitting CRLF on
# Windows, which made Rust's bare `v.parse()` reject every scalar knob and fall back
# to the baked default (see the note at the top of hero_recipe.py::main). Contention
# costs ~−0.2 warmth but a full −0.94 of MICRO-CONTRAST — dust motes and TAA are
# what stop accumulating, and micro is the axis that measures them. That alone
# exceeds BASELINE_TOL's 0.4, hence this guard.
if tasklist 2>/dev/null | grep -qiE '^(rustc|cargo|trunk|rust-lld|wasm-opt|voxelforge[_a-z]*)\.exe'; then
  echo "render_native_control_v3: a build or a second renderer is running — the 3.2s" >&2
  echo "  TAA settle is wall-clock, so this capture would under-converge. Wait." >&2
  tasklist 2>/dev/null | grep -iE '^(rustc|cargo|trunk|rust-lld|wasm-opt|voxelforge[_a-z]*)\.exe' >&2
  exit 3
fi

# Unquoted on purpose: one VOXELFORGE_X=Y per line, no spaces in any value, so word
# splitting is exactly the arg list `env` wants.
# shellcheck disable=SC2046
env $(python scripts/hero_recipe.py --env) \
  VOXELFORGE_SHOT="$OUT" \
  "$EXE"

# The window is 1280x720 (main.rs) and screenshot_once grabs at t>3.2s, then exits at
# t>4.4s — same TAA settle the web capture is held to.
test -f "$OUT" || { echo "render_native_control_v3: no $OUT produced" >&2; exit 1; }

# Run W0-D here too, so a control that would stop the grader never leaves this
# script. Tolerances and metrics are IMPORTED from the grader — not restated.
python - "$OUT" <<'PY'
import sys
from pathlib import Path

from PIL import Image

sys.path.insert(0, "scripts")
from grade_axes import measure
from grade_web_parity import BASELINE_TOL
from hero_recipe import BASELINE_PNG

out = sys.argv[1]
im = Image.open(out)
print(f"native control: {im.size[0]}x{im.size[1]}  ->  {out}")
assert im.size == (1280, 720), f"expected 1280x720, got {im.size}"

got, base = measure(out), measure(str(BASELINE_PNG))
print(f"W0-D vs {Path(BASELINE_PNG).name}:  "
      + "  ".join(f"{k} {got[k] - base[k]:+.2f}" for k in BASELINE_TOL))
off = {k: got[k] - base[k] for k in BASELINE_TOL if abs(got[k] - base[k]) > BASELINE_TOL[k]}
if off:
    print("  FAIL — off baseline on " + ", ".join(
        f"{k} {v:+.2f} (tol +/-{BASELINE_TOL[k]:g})" for k, v in off.items()), file=sys.stderr)
    sys.exit(1)
print("  OK — inside BASELINE_TOL, this control clears W0-D")
PY
