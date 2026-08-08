#!/usr/bin/env python3
"""Which edges in this frame are CAST BY THE SUN, and how wide are they? (G4a)

`cast_shadow_penumbra.py --albedo-check` asks "is that dark region cast by a
light" against the pre-light plate of the same camera, and on this shotset it
answered "89-94 % of the grass shade was already dark before" -> PAINTED IN THE
ALBEDO. That control is invalid here, measured: the pre-light plate was shot at
the SAME azimuth (205 deg) and only 5 deg lower (17 -> 22), so the same walls
throw the same shadows onto the same grass in both frames. A cast shadow that
barely moved scores as albedo.

The control that works is the one the albedo cannot follow: MOVE THE SUN'S
AZIMUTH and re-shoot off the same binary (`VOXELFORGE_LOOK_SUN=<elev>,<azim>,
<lux>`). Painted texture is bolted to the blocks and cannot move; a cast shadow
must. So:

  * a pixel whose luminance swings by more than `MOVE` L between two azimuths is
    lit differently by the two suns -- on a FLAT receiver that swing cannot come
    from N.L, which depends on elevation only, and elevation is held fixed.
  * an edge whose ridge sits on such a pixel is a SUN-LOCKED edge. Its 20->80
    width is a penumbra, graded against the same frame's sky silhouette in the
    same orientation bucket (`cast_shadow_penumbra.Frame` does both).

Usage:
  sun_locked_edges.py <plate-A>.png <plate-B>.png [--mask grass|all] [--move 10]
  sun_locked_edges.py <plate-A>.png <plate-B>.png --corr

  A is the frame that gets measured (shoot it at the shipped azimuth); B is the
  same camera and elevation with the azimuth moved.

`--corr` is the albedo test on its own: it high-passes both frames (a box blur
subtracted, so the smooth volumetric in-scatter term that DOES follow the sun's
azimuth is gone) and correlates the sharp structure over the shared grass mask.
Painted texture cannot move, so albedo scores r ~ 1 no matter where the sun is.
Measured on `s1-vista`, one binary (sha B024BBC2), only `VOXELFORGE_LOOK_SUN`
moving:  repeat shot r=0.99  |  before-plate (17 deg, SAME azimuth) r=0.76-0.87
|  azimuth +180 deg r=-0.03. The dark grass is sun-locked, not painted.
"""
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cast_shadow_penumbra import ORIENT_BINS, Frame, by_orient, collect, fmt, thin  # noqa: E402
from grass_bimodality import grass_select, lum  # noqa: E402
from nohud2_guard import require_nohud2  # noqa: E402

MOVE = 10.0       # L — swing between the two azimuths that makes a pixel sun-locked
MAX_SITES = 600


def boxblur(a, r):
    """Separable moving average — the low-frequency term, no scipy on this box."""
    k = 2 * r + 1
    p = np.pad(a, r, mode="edge")
    c = np.vstack([np.zeros((1, p.shape[1])), np.cumsum(p, 0)])
    a1 = (c[k:, :] - c[:-k, :]) / k
    c = np.hstack([np.zeros((a1.shape[0], 1)), np.cumsum(a1, 1)])
    return (c[:, k:] - c[:, :-k]) / k


def corr(pa, pb, radius=24):
    a = np.asarray(Image.open(pa).convert("RGB"))
    b = np.asarray(Image.open(pb).convert("RGB"))
    if a.shape != b.shape:
        sys.exit("the two plates are different sizes -- not the same camera")
    m = grass_select(a)[0] & grass_select(b)[0]
    la, lb = lum(a), lum(b)
    ha, hb = (la - boxblur(la, radius))[m], (lb - boxblur(lb, radius))[m]
    print(f"== {os.path.basename(pa)}  vs  {os.path.basename(pb)}")
    print(f"   shared grass mask {int(m.sum())} px, high-pass box radius {radius}px")
    print(f"   mean L {lb[m].mean():.2f} -> {la[m].mean():.2f}   "
          f"sharp-structure sd {hb.std():.2f} -> {ha.std():.2f}")
    print(f"   r = {np.corrcoef(ha, hb)[0, 1]:.3f}   "
          "(albedo cannot move: painted texture stays r ~ 1 under ANY sun)")


def main(argv=None):
    argv = list(argv if argv is not None else sys.argv[1:])
    mask_kind, move, only_corr, plates = "grass", MOVE, False, []
    while argv:
        a = argv.pop(0)
        if a == "--mask":
            mask_kind = argv.pop(0)
        elif a == "--move":
            move = float(argv.pop(0))
        elif a == "--corr":
            only_corr = True
        else:
            plates.append(a)
    if len(plates) != 2:
        sys.exit(__doc__)
    require_nohud2(plates, tool="sun_locked_edges.py")
    pa, pb = plates
    if only_corr:
        corr(pa, pb)
        return
    fr = Frame(pa)
    b = np.asarray(Image.open(pb).convert("RGB"))
    if b.shape != fr.arr.shape:
        sys.exit("the two plates are different sizes -- not the same camera")

    swing = np.abs(fr.L - lum(b))
    keep = swing > move
    if mask_kind == "grass":
        keep &= grass_select(fr.arr)[0]
    keep &= ~fr.sky

    print(f"== {os.path.basename(pa)}  vs  {os.path.basename(pb)}   mask={mask_kind}")
    print(f"   sun-locked pixels (|dL| > {move:.0f} L between the two azimuths): "
          f"{int(keep.sum())} of {int(keep.size)} ({100 * keep.mean():.2f}%)")

    ctl_w, ctl_o = collect(fr, thin(fr.ridge(fr.sky), MAX_SITES), allow_sky=True)
    ctl = by_orient(ctl_w, ctl_o)
    print(f"   CONTROL sky silhouette (geometry, zero penumbra) : {fmt(ctl_w)}")

    w, o = collect(fr, thin(fr.ridge(keep), MAX_SITES), allow_sky=False)
    print(f"   SUN-LOCKED edges (cast shadow, proven by the move): {fmt(w)}")
    for bnd in ORIENT_BINS:
        s, c = by_orient(w, o)[bnd], ctl[bnd]
        ratio = (f"  = {np.median(s) / np.median(c):.2f}x vs control"
                 if s.size and c.size else "")
        print(f"      normal {bnd[0]:2d}-{bnd[1]:2d}deg off-axis : {fmt(s)}{ratio}")
    if w.size and ctl_w.size:
        print(f"   POOLED : median {np.median(w):.2f}px vs control {np.median(ctl_w):.2f}px "
              f"= {np.median(w) / np.median(ctl_w):.2f}x")


if __name__ == "__main__":
    main()
