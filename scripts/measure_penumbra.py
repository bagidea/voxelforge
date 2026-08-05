#!/usr/bin/env python3
"""Measure shadow-edge PENUMBRA WIDTH on the floor, to prove PCSS soft shadows are
real (scale with the light's angular size) and not a uniform ~1px TAA smear.

For each scan row in a floor band we find the strongest luminance step (a shadow
edge) and measure the horizontal distance over which luminance climbs from 20% to
80% of that local dark->light range. A hard PCF/TAA edge is ~1-2px regardless; a
real PCSS penumbra WIDENS as soft_shadow_size grows and with occluder distance.

Usage: measure_penumbra.py <frame>-nohud2.png [rows_lo_frac rows_hi_frac]
Prints per-row widths (top rows = far/deep floor, bottom rows = near floor) + mean.
"""
import os
import sys
import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, must run first)

# HARD GUARD — this measures the STRONGEST luminance step per row in the floor
# band, and the `[E]` prompt glyphs are ~250 sitting on exactly that floor. A
# glyph edge is a far stronger step than a shadow edge, so on a raw capture this
# script does not measure a soft penumbra at all: it measures text.
require_nohud2(sys.argv[1:2], tool="measure_penumbra.py")

path = sys.argv[1]
lo = float(sys.argv[2]) if len(sys.argv) > 2 else 0.55
hi = float(sys.argv[3]) if len(sys.argv) > 3 else 0.95
im = Image.open(path).convert("L")
a = np.asarray(im).astype(np.float32)
H, W = a.shape

widths = []
per_row = []
for yf in np.linspace(lo, hi, 14):
    y = int(yf * H)
    row = a[y]
    # smooth a touch so single-pixel noise doesn't masquerade as an edge
    k = np.ones(3) / 3.0
    s = np.convolve(row, k, mode="same")
    d = np.diff(s)
    # strongest RISING edge (dark shadow -> lit floor) in the central 80%
    x0, x1 = int(W * 0.10), int(W * 0.90)
    seg = d[x0:x1]
    if seg.max() < 3:  # no real edge on this row
        continue
    ex = x0 + int(np.argmax(seg))
    # local dark/light levels: min/max of the smoothed row within +/-60px of edge
    w = 60
    lft = s[max(0, ex - w):ex + 1]
    rgt = s[ex:min(W, ex + w + 1)]
    dark = float(lft.min())
    light = float(rgt.max())
    rng = light - dark
    if rng < 10:
        continue
    lo_t, hi_t = dark + 0.2 * rng, dark + 0.8 * rng
    # walk outward from the edge to find where it crosses 20% and 80%
    xr = ex
    while xr < min(W - 1, ex + w) and s[xr] < hi_t:
        xr += 1
    xl = ex
    while xl > max(0, ex - w) and s[xl] > lo_t:
        xl -= 1
    width = xr - xl
    widths.append(width)
    per_row.append((y, ex, round(width, 1), round(rng, 1)))

print(f"# {path}  ({W}x{H})  floor band y {lo:.2f}-{hi:.2f}")
for y, ex, wd, rng in per_row:
    print(f"  row y={y:4d}  edge x={ex:4d}  penumbra={wd:5.1f}px  step={rng}")
if widths:
    print(f"  => edges={len(widths)}  mean penumbra = {np.mean(widths):.2f}px  "
          f"(near/bottom rows should be sharper than far/top)")
else:
    print("  => no clear shadow edge found in band")
