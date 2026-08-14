"""P2a probe: iso (C10) raw-mask vs gaussian-smoothed-mask, golden vs A6 render.

Read-only. Imports grade_character.py's own mask extractors so the numbers come
from the SHIPPED pipeline, not a re-implementation.

EXIT CODE (added 2026-08-14): this file used to print a block headed `== gates ==`
with the words PASS and FAIL in it and then exit 0 no matter what any of them
said. Anything that ran it and read `$?` -- a chain, a CI step, a person -- was
told "green" by a run whose own output said FAIL. The verdicts are now collected
and carried:

    0 = every gate printed PASS
    1 = at least one gate printed FAIL
    2 = a mask came back empty -- there is no silhouette to measure, so there is
        no iso number, and "unmeasurable" must not be spelled the same as "bad"

Nothing about the measurement changed; the numbers below are the same numbers.

`--golden` / `--render` / `--cam` exist so the exit code can be exercised on
inputs other than the two defaults. Bare (no flags) is the real probe and is
what the VERDICT doc quotes.

WHAT THE CONTROL RUNS ACTUALLY SHOWED (2026-08-14) — read this before quoting a
PASS off this probe. I added `--render` believing a known-bad frame would drive
the exit red. It does not, and that is a finding about the metric, not a bug in
the flag:

    --render docs/assets/grade-vista-2026-08-05-nohud2.png   -> 5/5 PASS, exit 0

grade-vista contains no character at this camera, yet it scores C10 98.18 and
art_order 123.52 -- both far ABOVE the 43.52 / 49.25 the approved hero concept
itself scores. The bar is cleared by scene texture inside the analytic bbox, so
a higher number here does not mean a better silhouette. The only input that
moved the exit off 0 was an all-black frame, and that is exit 2 (empty mask),
not exit 1. So: exit 1 is live code, but NO real frame in this repo has yet
produced it. Treat a PASS from this probe as "not falsified", never as evidence
the silhouette reads -- cross-check against art_order_grade.py's own control set.
"""
import argparse
import importlib.util
import os
import sys

import numpy as np
from PIL import Image
from scipy import ndimage

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

_ap = argparse.ArgumentParser(description="P2a iso probe (raw vs smoothed mask)")
_ap.add_argument("--golden", default=os.path.join(ROOT, "docs/assets/characters/auren-hero-concept.png"))
_ap.add_argument("--render", default=os.path.join(ROOT, "docs/assets/gate3-a6-2026-08-10/gate3-after-boot-nohud2.png"))
_ap.add_argument("--cam", default="0.0,-14.32,5.289", help="render camera x,y,z for analytic_bbox")
_args = _ap.parse_args()          # BEFORE sys.argv is clobbered for the imports below

spec = importlib.util.spec_from_file_location("gc_mod", os.path.join(HERE, "grade_character.py"))
gc = importlib.util.module_from_spec(spec)
sys.argv = ["gc"]
spec.loader.exec_module(gc)


def iso_pair(mask):
    """Return (iso_raw, iso_smoothed, iso_smoothed_shipped) for one boolean mask.

    shipped = perimeter of SMOOTHED mask / area of RAW mask (what line 410-413 does)
    """
    m = mask.astype(bool)
    mf = m.astype(np.float32)
    A_raw = float(m.sum())

    per_raw = ndimage.binary_dilation(m, iterations=1) & ~m
    P_raw = float(per_raw.sum())

    ms = ndimage.gaussian_filter(mf, 2.0) > 0.5
    A_sm = float(ms.sum())
    per_sm = ndimage.binary_dilation(ms, iterations=1) & ~ms
    P_sm = float(per_sm.sum())

    return (P_raw * P_raw / A_raw,          # pure raw
            P_sm * P_sm / A_sm,             # pure smoothed (self-consistent)
            P_sm * P_sm / A_raw)            # what grade_character.py actually returns


def norm(rgb, mask, who):
    """Use grade_character's OWN normalise() so scaling is identical.

    An empty mask is UNMEASURABLE, not a failure. `normalise()` reduces over the
    mask's ys, so an empty one raised `ValueError: zero-size array to reduction`,
    Python exited 1, and a frame the probe could not read at all handed back the
    same number as a frame that read fine and failed the >=40 bar. Exit 2 keeps
    "I could not measure this" distinct from "I measured it and it is bad".
    """
    if not mask.any():
        print(f"!! {who}: mask is empty — no silhouette to measure, so no iso "
              f"number exists for this frame (exit 2)", file=sys.stderr)
        raise SystemExit(2)
    _a, m, scale = gc.normalise(rgb, mask, 512)
    return m, scale


GOLDEN = _args.golden
A6 = _args.render
A6_CAM = tuple(float(v) for v in _args.cam.split(","))

print("== C10 iso: raw vs smoothed, on the shipped masks ==")
print()

# --- golden concept sheet (mask_from_ref: gradient matte) ---
g_rgb = np.asarray(Image.open(GOLDEN).convert("RGB")).astype(np.float32)
g_mask = gc.mask_from_ref(g_rgb)
gm, gs = norm(g_rgb, g_mask, "golden " + GOLDEN)
graw, gsm, gship = iso_pair(gm)
print(f"GOLDEN auren-hero-concept   mask_from_ref   px={int(gm.sum()):>7} scale={gs:.3f}")
print(f"   iso raw      = {graw:8.2f}")
print(f"   iso smoothed = {gsm:8.2f}   (self-consistent P_sm^2/A_sm)")
print(f"   iso SHIPPED  = {gship:8.2f}   (P_sm^2/A_raw  <- grade_character.py)")
print()

# --- A6 boot frame (analytic bbox + key_tol) ---
r_rgb = np.asarray(Image.open(A6).convert("RGB")).astype(np.float32)
rh, rw, _ = r_rgb.shape
bbox, foot_y = gc.analytic_bbox(rw, rh, *A6_CAM, "player", 80)
r_mask = gc.mask_from_bbox(r_rgb, bbox, foot_y, 26.0, True)
rm, rs = norm(r_rgb, r_mask, "render " + A6)
rraw, rsm, rship = iso_pair(rm)
print(f"A6 boot/idle render         mask_from_bbox  px={int(rm.sum()):>7} scale={rs:.3f}")
print(f"   iso raw      = {rraw:8.2f}")
print(f"   iso smoothed = {rsm:8.2f}   (self-consistent P_sm^2/A_sm)")
print(f"   iso SHIPPED  = {rship:8.2f}   (P_sm^2/A_raw  <- grade_character.py)")
print()

GATES = []          # (name, ok) -- every printed PASS/FAIL lands here, see module docstring


def gate_tag(name, ok):
    GATES.append((name, bool(ok)))
    return "PASS" if ok else "FAIL"


print("== gates ==")
print(f"  spec absolute      >= 40      : "
      f"golden {gate_tag('C10 iso >=40 (golden)', gship >= 40)} ({gship:.2f})"
      f" | A6 {gate_tag('C10 iso >=40 (A6)', rship >= 40)} ({rship:.2f})")
print(f"  grader relmin 0.70 -> {0.70*gship:.2f}: "
      f"A6 {gate_tag('C10 iso >= 0.70x golden (A6)', rship >= 0.70 * gship)}")
print()
print(f"  raw-vs-shipped delta  golden {graw - gship:+8.2f}   A6 {rraw - rship:+8.2f}")


# ---------------------------------------------------------------------------
# PART 2 (added after review): the >=40 bar is NOT C10's. It is calibrated in
# scripts/art_order_grade.py, whose shape_complexity() uses NO smoothing and an
# INNER erosion boundary. Its own docstring (line 305-320) says in capitals:
#   "DO NOT CROSS-QUOTE THIS NUMBER WITH grade_character.py C10"
# So run the golden + the render through THAT function too, on the same masks.
# ---------------------------------------------------------------------------
ao_spec = importlib.util.spec_from_file_location("ao_mod", os.path.join(HERE, "art_order_grade.py"))
ao = importlib.util.module_from_spec(ao_spec)
sys.argv = ["ao"]
ao_spec.loader.exec_module(ao)

print()
print("== art_order_grade.shape_complexity (the pipeline the >=40 bar lives in) ==")
ARCH = os.path.join(ROOT, "docs/assets/characters/archive-before-2026-08-06/"
                          "auren-hero-concept-PASS1-flathair-2026-08-06.png")
a_rgb = np.asarray(Image.open(ARCH).convert("RGB")).astype(np.float32)
a_mask = gc.mask_from_ref(a_rgb)
am, _ = norm(a_rgb, a_mask, "archived PASS1 " + ARCH)

for tag, mk in (("GOLDEN concept (current)", gm),
                ("GOLDEN concept (ARCHIVED PASS1)", am),
                ("A6 boot/idle render", rm)):
    ao_v = ao.shape_complexity(mk)
    c10 = iso_pair(mk)[2]
    print(f"  {tag:<34} art_order={ao_v:8.2f}   C10={c10:8.2f}   ratio={ao_v/c10:.2f}x")

g_ao = ao.shape_complexity(gm)
a_ao = ao.shape_complexity(am)
r_ao = ao.shape_complexity(rm)
print()
print(f"  bar >=40 (art_order pipeline): "
      f"golden {gate_tag('art_order shape >=40 (golden)', g_ao >= 40)} ({g_ao:.2f})"
      f" | A6 {gate_tag('art_order shape >=40 (A6)', r_ao >= 40)} ({r_ao:.2f})")
print(f"  docstring says the bar = 0.75x approved hero concept:")
print(f"     0.75 x archived PASS1 ({a_ao:.2f}) = {0.75*a_ao:.2f}")
print(f"     0.75 x current       ({g_ao:.2f}) = {0.75*g_ao:.2f}")
print(f"  docstring quotes the approved concepts as 53.5 / 136.7 / 298.1")

# --- carry the verdict into the exit code -----------------------------------
failed = [n for n, ok in GATES if not ok]
print()
print("== VERDICT ==")
for name, ok in GATES:
    print(f"  {'PASS' if ok else 'FAIL'}  {name}")
if failed:
    print(f"\nP2a ISOPROBE: FAIL ({len(failed)}/{len(GATES)} gate(s) not PASS) -> exit 1")
    raise SystemExit(1)
print(f"\nP2a ISOPROBE: PASS ({len(GATES)}/{len(GATES)}) -> exit 0")
