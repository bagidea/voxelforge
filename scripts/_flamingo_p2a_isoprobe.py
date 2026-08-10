"""P2a probe: iso (C10) raw-mask vs gaussian-smoothed-mask, golden vs A6 render.

Read-only. Imports grade_character.py's own mask extractors so the numbers come
from the SHIPPED pipeline, not a re-implementation.
"""
import importlib.util
import os
import sys

import numpy as np
from PIL import Image
from scipy import ndimage

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
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


def norm(rgb, mask):
    """Use grade_character's OWN normalise() so scaling is identical."""
    _a, m, scale = gc.normalise(rgb, mask, 512)
    return m, scale


GOLDEN = os.path.join(ROOT, "docs/assets/characters/auren-hero-concept.png")
A6 = os.path.join(ROOT, "docs/assets/gate3-a6-2026-08-10/gate3-after-boot-nohud2.png")
A6_CAM = (0.0, -14.32, 5.289)

print("== C10 iso: raw vs smoothed, on the shipped masks ==")
print()

# --- golden concept sheet (mask_from_ref: gradient matte) ---
g_rgb = np.asarray(Image.open(GOLDEN).convert("RGB")).astype(np.float32)
g_mask = gc.mask_from_ref(g_rgb)
gm, gs = norm(g_rgb, g_mask)
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
rm, rs = norm(r_rgb, r_mask)
rraw, rsm, rship = iso_pair(rm)
print(f"A6 boot/idle render         mask_from_bbox  px={int(rm.sum()):>7} scale={rs:.3f}")
print(f"   iso raw      = {rraw:8.2f}")
print(f"   iso smoothed = {rsm:8.2f}   (self-consistent P_sm^2/A_sm)")
print(f"   iso SHIPPED  = {rship:8.2f}   (P_sm^2/A_raw  <- grade_character.py)")
print()

print("== gates ==")
print(f"  spec absolute      >= 40      : golden {'PASS' if gship >= 40 else 'FAIL'} ({gship:.2f})"
      f" | A6 {'PASS' if rship >= 40 else 'FAIL'} ({rship:.2f})")
print(f"  grader relmin 0.70 -> {0.70*gship:.2f}: A6 {'PASS' if rship >= 0.70*gship else 'FAIL'}")
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
am, _ = norm(a_rgb, a_mask)

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
print(f"  bar >=40 (art_order pipeline): golden {'PASS' if g_ao>=40 else 'FAIL'} ({g_ao:.2f})"
      f" | A6 {'PASS' if r_ao>=40 else 'FAIL'} ({r_ao:.2f})")
print(f"  docstring says the bar = 0.75x approved hero concept:")
print(f"     0.75 x archived PASS1 ({a_ao:.2f}) = {0.75*a_ao:.2f}")
print(f"     0.75 x current       ({g_ao:.2f}) = {0.75*g_ao:.2f}")
print(f"  docstring quotes the approved concepts as 53.5 / 136.7 / 298.1")
