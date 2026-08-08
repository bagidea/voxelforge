#!/usr/bin/env python3
"""Grass sun/shade split — does the frame's vegetation break into TWO humps?

WHY THIS EXISTS. `grade_look.py` grades G2 with a top-5%-vs-median delta and G4b
with a "groove strength" indicator its own docstring calls confounded by the
checkerboard floor. Neither answers the question a raking key light is FOR: is
any given patch of ground either IN the sun or IN a shadow? A frame where every
blade of grass sits at the same luminance has no key light in it, however bright
the brightest 5% happens to be — and that is exactly the failure this file was
written to catch on the 2026-08-08 shotset, where all 8 plates read one hump.

WHAT IT MEASURES. Grass pixels only (hue-gated, so the terracotta ruin and the
sky cannot vote), luminance histogram, then three numbers about its SHAPE:

  separation  Otsu-split the grass luminance; distance between the two group
              means, in L units (0-100). "How far apart are sun and shade."
  dip         1 - valley/min(peak), where the two peaks are the tallest bin on
              EACH SIDE of the Otsu split and the valley is the lowest bin
              between them. "Is there actually a gap, or one fat hump."
              Anchoring on the split is the whole point: a plate lit by a smooth
              distance gradient (s1-vista, 2026-08-07) has a wide separation and
              no valley at all, and this is the number that tells them apart.
  balance     the smaller mode's share of grass pixels. "Is the second hump a
              real population or a tail." Deliberately a LOW bar - a shadow that
              covers a twentieth of a lawn is still a shadow; the calibration
              anchor reads 7.7% here and is unarguably two-humped.

THE THRESHOLDS ARE CALIBRATED, NOT INVENTED — see the table in
`docs/lighting-g2-g4b-2026-08-08.md`. Two anchors, both measured with this
script: a frame with a real shadow-casting key (`VOXELFORGE_LOOK_DISABLE=1`,
main.rs's own 57-degree sun over a 380-lux fill) and the shipped 2026-08-07
`--play` plates. The bar sits between them.

Usage: grade_sunsplit.py <frame.png> [more.png ...]
Exit code 0 = every gradeable plate shows two humps.
"""
import sys

import numpy as np
from PIL import Image

# --- calibrated bars (see module docstring) --------------------------------
MIN_COVERAGE = 0.010  # grass must be >=1% of the frame, else the plate is N/A
MIN_SEPARATION = 8.0  # L units between the sun mode and the shade mode
MIN_DIP = 0.20  # depth of the valley between the two modes, 0..1
MIN_BALANCE = 0.05  # smaller mode's share of grass pixels

# Histogram grid. Fixed 0..100 at 1 L per bin so `dip` means the same thing on
# every plate regardless of how much range that plate happens to occupy.
BINS = 100
RANGE = (0.0, 100.0)

# Vegetation hue window, degrees. Deliberately WIDE on the yellow side (55) so a
# heavily warm-graded lawn still counts, and cut at 140 before the sky's ~225.
HUE_LO, HUE_HI = 55.0, 140.0
MIN_SAT = 0.15


def hsv_hue_sat(a):
    """Hue in degrees and HSV saturation, from a float32 HxWx3 RGB array."""
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    mx = a.max(2)
    mn = a.min(2)
    d = mx - mn
    safe = np.maximum(d, 1e-6)
    hue = np.zeros_like(mx)
    m = (mx == r) & (d > 0)
    hue[m] = (((g - b)[m] / safe[m]) % 6.0)
    m = (mx == g) & (d > 0)
    hue[m] = ((b - r)[m] / safe[m]) + 2.0
    m = (mx == b) & (d > 0)
    hue[m] = ((r - g)[m] / safe[m]) + 4.0
    return hue * 60.0, d / np.maximum(mx, 1e-6)


def otsu(vals, bins=BINS):
    """Threshold that maximises between-class variance. Returns (t, hist, edges)."""
    hist, edges = np.histogram(vals, bins=bins, range=RANGE)
    centres = 0.5 * (edges[:-1] + edges[1:])
    w = hist.astype(np.float64)
    total = w.sum()
    if total == 0:
        return float(np.median(vals)), hist, edges
    w0 = np.cumsum(w)
    w1 = total - w0
    s0 = np.cumsum(w * centres)
    s_all = s0[-1]
    s1 = s_all - s0
    ok = (w0 > 0) & (w1 > 0)
    between = np.zeros_like(w0)
    between[ok] = w0[ok] * w1[ok] * ((s0[ok] / w0[ok]) - (s1[ok] / w1[ok])) ** 2
    return float(centres[int(np.argmax(between))]), hist, edges


def smooth(h, sigma=1.6):
    """Small gaussian blur so single-bin dither doesn't invent peaks/valleys."""
    rad = max(1, int(3 * sigma))
    x = np.arange(-rad, rad + 1)
    k = np.exp(-0.5 * (x / sigma) ** 2)
    k /= k.sum()
    return np.convolve(h.astype(np.float64), k, mode="same")


def dip_across_split(hist, edges, t):
    """1 - valley/min(peak), peaks taken on either side of the Otsu split `t`.

    Anchoring the two peaks to the split (rather than "the two tallest modes")
    is what stops this reading a single fat hump as bimodal: the tallest two
    local maxima of one broad hump sit inside it and the valley between them is
    barely below the peaks, so the naive version scored a genuinely two-humped
    calibration frame at dip=0.006. Here the question asked is exactly the one
    the gate cares about - "between the shade population and the sun population,
    does the histogram come back DOWN?"
    """
    s = smooth(hist)
    ti = int(np.searchsorted(edges, t)) - 1
    ti = max(1, min(len(s) - 1, ti))
    if s[:ti].max() <= 0 or s[ti:].max() <= 0:
        return 0.0, None
    lo = int(np.argmax(s[:ti]))
    hi = ti + int(np.argmax(s[ti:]))
    if hi <= lo:
        return 0.0, None
    valley = float(s[lo:hi + 1].min())
    floor = float(min(s[lo], s[hi]))
    if floor <= 0:
        return 0.0, None
    return 1.0 - valley / floor, (lo, hi)


def grade(path):
    a = np.asarray(Image.open(path).convert("RGB")).astype(np.float32)
    H, W, _ = a.shape
    hue, sat = hsv_hue_sat(a)
    lum = (0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]) / 255.0 * 100.0
    mask = (hue >= HUE_LO) & (hue < HUE_HI) & (sat >= MIN_SAT)
    cov = mask.sum() / float(H * W)
    print(f"# {path}  {W}x{H}")
    print(f"  grass mask: hue[{HUE_LO:.0f},{HUE_HI:.0f}) sat>={MIN_SAT}  "
          f"coverage={cov*100:.2f}% of frame (need >= {MIN_COVERAGE*100:.1f}%)")
    if cov < MIN_COVERAGE:
        print("  -> N/A  (not enough vegetation in this framing to grade)\n")
        return None

    vals = lum[mask]
    t, hist, edges = otsu(vals)
    loi = vals[vals < t]
    hii = vals[vals >= t]
    if loi.size == 0 or hii.size == 0:
        print("  -> FAIL  (degenerate split)\n")
        return False
    sep = float(hii.mean() - loi.mean())
    bal = float(min(loi.size, hii.size) / vals.size)
    dip, pk = dip_across_split(hist, edges, t)
    # classic bimodality coefficient, reported for cross-reference only
    z = (vals - vals.mean()) / (vals.std() + 1e-9)
    skew = float((z ** 3).mean())
    kurt = float((z ** 4).mean())
    bc = (skew ** 2 + 1.0) / max(kurt, 1e-9)

    print(f"  shade mode  L={loi.mean():6.2f}  ({100*loi.size/vals.size:5.1f}% of grass)")
    print(f"  sun   mode  L={hii.mean():6.2f}  ({100*hii.size/vals.size:5.1f}% of grass)")
    print(f"  separation = {sep:6.2f} L   (need >= {MIN_SEPARATION})")
    print(f"  dip        = {dip:6.3f}     (need >= {MIN_DIP})"
          + (f"   modes at L={0.5*(edges[pk[0]]+edges[pk[0]+1]):.1f} / "
             f"{0.5*(edges[pk[1]]+edges[pk[1]+1]):.1f}" if pk else "   (single mode)"))
    print(f"  balance    = {bal:6.3f}     (need >= {MIN_BALANCE})")
    print(f"  [xref] bimodality coeff = {bc:.3f} (>0.555 suggests bimodal)")
    ok = sep >= MIN_SEPARATION and dip >= MIN_DIP and bal >= MIN_BALANCE
    print(f"  -> {'TWO HUMPS (PASS)' if ok else 'ONE HUMP (FAIL)'}\n")
    return ok


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    results = [(p, grade(p)) for p in argv]
    graded = [(p, r) for p, r in results if r is not None]
    print("=" * 58)
    for p, r in results:
        print(f"  {'N/A ' if r is None else ('PASS' if r else 'FAIL')}  {p}")
    if not graded:
        print("  no gradeable plate")
        return 2
    npass = sum(1 for _, r in graded if r)
    print(f"  => {npass}/{len(graded)} gradeable plates show a sun/shade split")
    return 0 if npass == len(graded) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
