#!/usr/bin/env python3
"""Pixel-lane probe for the rubric sub-checks no existing grader prints.

`grade_gate.py` decides G3/G5/G6, `grade_look.py` adds G2/G4a, `grade_axes.py`
owns the P0 axes. What none of them print are the *Layer B* pass sub-checks that
the rubric asks a human to eyedrop by hand:

  Pass 9a  warm-palette share      -- "~85% of the frame is warm"
  Pass 9b  teal accent share       -- "teal accent <= 15% of the frame"
  Pass 2a  open-shade fill tone    -- shade band R>B and L in 12..35%
  Pass 8b  bloom did not wash out  -- shadow floor not lifted
  Pass 6a  window highlight ladder -- 3 samples down the brightest region

Every number here is printed with the golden ref's own value beside it, because a
number without its control is not a verdict -- that is rule 7 of the rubric and
this file is not exempt from it. Nothing in here gates: it prints, a human reads.

Usage: _pixel_rubric_probe.py <frame>-nohud2.png [more...]
"""
import sys

import numpy as np
from PIL import Image, ImageFilter

# Pass 9b's "teal accent" band -- DERIVED FROM THE CONTROL, not from the word.
#
# The first version of this probe used the A6.2' character-brooch band (#4FC9D6 =
# 185.8 deg +/-25). Run against the golden ref it returned 0 px -- on the very frame
# the pass-list says carries the accent. That is the rubric's own rule 7 firing: a
# band the reference cannot score in is measuring nothing, so it was replaced rather
# than believed.
#
# What the ref actually holds (hue histogram, sat>=0.15 L>=20): 85.6% at 0-30 deg,
# 13.3% at 30-60, and a single 1.11% cluster at 60-90 deg. The accent is a GREEN
# moss block, not a cyan one -- which is exactly what hero.rs calls it in the source
# ("moss block"), and it reproduces the calibration log's "teal-ish 0.96%" line.
# The wide baseline's glass accent sits further round at 120 deg.
#
# So the honest band is "cool-side accent" = hue 60..240, and it has a positive
# control on BOTH approved plates: ref 1.11%, wide baseline 2.87%, both non-zero
# and both well under the 15% ceiling. sat/L floors kept from A6.2' for the same
# degeneracy reason (near-black noise inflates sat=(max-min)/max).
ACCENT_HUE_LO, ACCENT_HUE_HI, TEAL_SAT_MIN, TEAL_L_MIN = 60.0, 240.0, 0.25, 20.0


def lum(a):
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def hsv(a):
    mx = a.max(axis=-1)
    mn = a.min(axis=-1)
    d = mx - mn
    sat = np.where(mx > 0, d / np.maximum(mx, 1e-6), 0.0)
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    h = np.zeros_like(mx)
    nz = d > 1e-6
    idx = (mx == r) & nz
    h[idx] = (60 * ((g - b)[idx] / d[idx])) % 360
    idx = (mx == g) & nz
    h[idx] = 60 * ((b - r)[idx] / d[idx]) + 120
    idx = (mx == b) & nz
    h[idx] = 60 * ((r - g)[idx] / d[idx]) + 240
    return h, sat, mx


def probe(path):
    im = Image.open(path).convert("RGB")
    a = np.asarray(im).astype(np.float32)
    L = lum(a)
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    h, s, _ = hsv(a)
    n = L.size

    print(f"\n=== {path}  {im.size[0]}x{im.size[1]} ===")

    # ---- Pass 9a: warm share --------------------------------------------------
    warm = float((R > B).sum()) / n * 100
    strong = float((R > B + 20).sum()) / n * 100
    print(f"  [9a] warm share R>B          {warm:6.2f}%   (strong R>B+20 {strong:5.2f}%)"
          f"   mean R/G/B {R.mean():6.1f}/{G.mean():6.1f}/{B.mean():6.1f}")

    # ---- Pass 9b: teal accent share -------------------------------------------
    teal = (h >= ACCENT_HUE_LO) & (h <= ACCENT_HUE_HI) & (s >= TEAL_SAT_MIN) & (L >= TEAL_L_MIN)
    hh = h[teal]
    where = f"median hue {np.median(hh):5.1f}deg" if hh.size else "no accent pixels"
    print(f"  [9b] cool accent share       {teal.sum() / n * 100:6.2f}%   "
          f"({int(teal.sum())} px, need <=15% AND non-zero; {where})"
          f"   [REF 1.11% / wide-baseline 2.87%]")

    # ---- Pass 2a: open-shade fill --------------------------------------------
    # The rubric samples "shade that is not in direct sun": take the band below
    # the frame's own p35 luminance, which is the same lower cut grade_axes uses
    # to find the midtone band -- so this is the tone just under the lit wood.
    cut = np.percentile(L, 35)
    sh = L < cut
    shR, shB, shL = R[sh].mean(), B[sh].mean(), L[sh].mean() / 255 * 100
    print(f"  [2a] open shade (L<p35)      L={shL:5.2f}%  R-B={shR - shB:+6.1f}  "
          f"(want L 12..35%, R>B)")

    # ---- Pass 8b: bloom did not wash the shadows ------------------------------
    print(f"  [8b] shadow floor p01/p05 L  {np.percentile(L, 1) / 255 * 100:5.2f}% / "
          f"{np.percentile(L, 5) / 255 * 100:5.2f}%   (bloom wash lifts these)")

    # ---- Pass 6a: highlight ladder -------------------------------------------
    # Three samples down the column through the brightest pixel: the rubric's
    # "3 points down the window" check, located rather than hand-picked.
    yy, xx = np.unravel_index(int(np.argmax(L)), L.shape)
    rows = [yy, min(yy + im.size[1] // 12, L.shape[0] - 1), min(yy + im.size[1] // 6, L.shape[0] - 1)]
    lad = [L[r, xx] / 255 * 100 for r in rows]
    print(f"  [6a] window ladder @x={xx}     L = "
          f"{lad[0]:.1f} / {lad[1]:.1f} / {lad[2]:.1f}%   spread={max(lad) - min(lad):.1f}"
          f"   brightest RGB=({int(R[yy, xx])},{int(G[yy, xx])},{int(B[yy, xx])})")

    # ---- grain / micro texture (Gr + pass 7 support) --------------------------
    hp = np.asarray(Image.fromarray(L.astype(np.uint8)).filter(ImageFilter.GaussianBlur(3))).astype(np.float32)
    print(f"  [Gr] hi-pass std (grain)     {float((L - hp).std()):6.2f}"
          f"   pure-black px {float((L < 4).sum()) / n * 100:5.2f}%"
          f"   near-clip px(L>250) {float((L > 250).sum()) / n * 100:5.2f}%")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit("usage: _pixel_rubric_probe.py <frame>-nohud2.png [more...]")
    for p in sys.argv[1:]:
        probe(p)
