#!/usr/bin/env python3
"""Flamingo halo probe - checklist rev.2 item 2, as a number instead of an opinion.

The question: does the bright sky actually BLEED onto the terrain just under the
horizon silhouette (a real bloom halo), or is the sky just a brighter flat plate?
"It looks glowy" is exactly the judgement that has been wrong twice in this
project, so this measures it column by column.

WHAT IS MEASURED
  A column that starts in sky and hits terrain gives one silhouette edge. Under
  that edge:

      L_near = mean luminance of rows edge+2 .. edge+6     (the halo band)
      L_far  = mean luminance of rows edge+25 .. edge+45   (un-bloomed terrain)

  A real bloom lifts L_near over L_far and the lift FADES with distance. A
  brighter exposure lifts both equally and cancels. Each column is its own
  control, so a frame shot at a different exposure stays comparable - and the
  MEDIAN over 6-8 columns spread across the frame is what grades, so one lucky
  column (or one dark tree) cannot carry or sink the verdict.

THE THREE CHECKS
  (a) halo exists    median(L_near - L_far) >= 2.0     [luminance, 0..255 scale]
  (b) halo is BLUE   median((B-R)_near - (B-R)_far) > 0
      A blue-led lift is the signature of THIS mechanism: the authored sky is
      blue-dominant, so a sky that leaks past the bloom prefilter must leak blue
      first. A warm/neutral lift of the same size is some other light - a lamp,
      a rim, a tonemap shoulder - and must not be allowed to pass as sky bloom.
  (c) sky sanity     B > G > R  |  no sky px with min(R,G,B) >= 250  |  L-std > 0.07
      Guards the degenerate ways (a) and (b) can be "won": a sky blown to white
      clips every channel (kills the blue lead and any gradient), and a flat
      ClearColor plate has ~zero luminance spread. If the sky is not a real sky,
      the halo numbers above are not evidence of anything.

  L-std is measured on luminance normalised to 0..1, so 0.07 ~= 18/255 of
  spread: an authored gradient/haze clears it easily, a flat plate reads ~0.00.
  The value is printed on all three scales so a borderline call stays readable.

CALIBRATION (measured on frames already on disk, 2026-08-05 -- NOT a verdict on
any current build; both frames predate the exe this probe was written for)

  frame                                    L-std(0..1)   (a) dL   (b) d(B-R)
  _poppy_look/before-eff6c7b-stale-exe/
    look-on-vista-nohud2.png                  0.0180      -2.08      +0.40
  _flamingo_g7b/g7full-nohud2.png             0.0895      +5.80      +1.05

  That is the whole reason the 0.07 threshold is trusted at this unit: the flat
  stale-exe sky reads 0.018 and the graded g7b haze sky reads 0.0895, so 0.07
  lands between them and separates "flat plate" from "real authored sky" on
  actual renderer output. It is not a threshold nothing can clear -- g7full
  passes all three checks -- and not one everything clears either.

  Watch check (b): g7full only clears it by +1.05 with 4/7 columns blue-led.
  A blue-lead that thin is the check most likely to flip on a re-shoot, so if
  (a) passes big and (b) sits near zero, read the per-column table before
  calling it: a couple of warm-lit columns can drag the median across.

Exit: 0 = all three PASS, 1 = at least one FAIL, 2 = refused (guard/usage),
      3 = inconclusive (too few usable silhouette columns to speak).

Usage: _flamingo_halo_probe.py <frame>-nohud2.png
"""
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, must be at the head)

NEAR = (2, 6)          # rows under the silhouette = the halo band
FAR = (25, 45)         # rows under the silhouette = un-bloomed far field
MIN_SKY_RUN = 24       # px of contiguous sky above the edge before a column counts
SOLID_FRAC = 0.85      # of rows edge+1..edge+FAR_hi must be non-sky (no sky through a gap)
EDGE_MARGIN = 0.02     # of frame width dropped at each side (vignette, not bloom)
N_COLS = 8             # columns sampled, spread evenly across the frame
N_COLS_MIN = 6         # fewer usable than this -> the probe refuses to speak

HALO_DL_MIN = 2.0      # check (a)
SKY_LSTD_MIN = 0.07    # check (c), luminance on 0..1
CLIP_LEVEL = 250       # check (c), min(R,G,B) >= this = white-clipped sky px


def lum(a):
    """Rec.709 luminance on the 0..255 scale (same formula as grade_gate.py)."""
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def sky_mask(a):
    """Sky = blue-dominant and not dark. Hue-based on purpose: a brightness rule
    would also swallow the sunlit ground that the colour axes grade."""
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    return (B > R + 15) & (B > 70)


def silhouette(a):
    """First non-sky row per column, for columns that start in real sky."""
    m = sky_mask(a)
    H, W = m.shape
    ys = np.full(W, -1, dtype=int)
    for x in range(W):
        col = m[:, x]
        if not col[0]:
            continue
        nz = np.flatnonzero(~col)
        if nz.size == 0:
            continue                      # sky all the way down - no edge here
        y = int(nz[0])
        if y >= MIN_SKY_RUN:
            ys[x] = y
    return ys, m


def usable(a, m, ys):
    """Columns with an edge, room for the far band, and solid terrain below it.

    Columns inside EDGE_MARGIN of the left/right frame border are dropped: the
    vignette darkens them, which is a luminance lift/drop that has nothing to do
    with bloom, and a border column would otherwise get picked whenever the
    horizon happens to be occluded across that whole side of the frame."""
    H, W = m.shape
    margin = int(EDGE_MARGIN * W)
    out = []
    for x in range(margin, W - margin):
        y = ys[x]
        if y < 0 or y + FAR[1] >= H:
            continue
        below = m[y + 1: y + FAR[1] + 1, x]
        if below.size and (~below).mean() >= SOLID_FRAC:
            out.append(x)
    return out


def pick_columns(cols, W, n=N_COLS):
    """Snap n evenly-spaced targets across the frame onto the usable columns, so
    the sample spans the whole horizon instead of clustering on one hillside."""
    if not cols:
        return []
    arr = np.asarray(cols)
    picked = []
    for i in range(n):
        target = W * (i + 0.5) / n
        x = int(arr[np.argmin(np.abs(arr - target))])
        if x not in picked:
            picked.append(x)
    return sorted(picked)


def column_stats(a, x, y):
    near = a[y + NEAR[0]: y + NEAR[1] + 1, x, :]
    far = a[y + FAR[0]: y + FAR[1] + 1, x, :]
    nm, fm = near.mean(axis=0), far.mean(axis=0)
    return {
        "x": x, "y": y,
        "L_near": float(lum(nm)), "L_far": float(lum(fm)),
        "dL": float(lum(nm) - lum(fm)),
        "BR_near": float(nm[2] - nm[0]), "BR_far": float(fm[2] - fm[0]),
        "dBR": float((nm[2] - nm[0]) - (fm[2] - fm[0])),
    }


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        print("REFUSED  need exactly one argument: <frame>-nohud2.png", file=sys.stderr)
        sys.exit(2)

    path = sys.argv[1]
    require_nohud2([path], tool="_flamingo_halo_probe.py")

    a = np.asarray(Image.open(path).convert("RGB")).astype(np.float64)
    H, W, _ = a.shape
    print(f"# halo probe  {path}  {W}x{H}")
    print(f"#   L_near = rows edge+{NEAR[0]}..+{NEAR[1]}   "
          f"L_far = rows edge+{FAR[0]}..+{FAR[1]}   (luminance on 0..255)\n")

    ys, m = silhouette(a)
    cols = usable(a, m, ys)
    picked = pick_columns(cols, W)
    print(f"silhouette columns: {int((ys >= 0).sum())} found, {len(cols)} usable "
          f"(edge + {FAR[1]}px of solid terrain below), {len(picked)} sampled")

    if len(picked) < N_COLS_MIN:
        print(f"\nINCONCLUSIVE  only {len(picked)} usable column(s), need >= {N_COLS_MIN}.")
        print("  The frame has no clean sky/terrain horizon to measure across "
              "(camera pitched down, sky occluded, or the sky is not blue-dominant).")
        print("  Re-shoot the vista framing - do NOT read the checks below as a FAIL.")
        sys.exit(3)

    stats = [column_stats(a, x, ys[x]) for x in picked]

    print()
    print("   col     edge_y    L_near    L_far       dL    (B-R)near   (B-R)far     d(B-R)")
    print("  " + "-" * 82)
    for s in stats:
        print(f"  {s['x']:>5}  {s['y']:>9}  {s['L_near']:8.2f} {s['L_far']:8.2f} "
              f"{s['dL']:+8.2f}   {s['BR_near']:9.2f}  {s['BR_far']:9.2f}  {s['dBR']:+9.2f}")

    dLs = np.array([s["dL"] for s in stats])
    dBRs = np.array([s["dBR"] for s in stats])
    med_dL, med_dBR = float(np.median(dLs)), float(np.median(dBRs))

    # ---- (a) halo exists ------------------------------------------------
    a_ok = med_dL >= HALO_DL_MIN
    print(f"\n(a) HALO EXISTS   median(L_near - L_far) = {med_dL:+.2f}   "
          f"(need >= {HALO_DL_MIN:+.2f})")
    print(f"    per-column dL: min {dLs.min():+.2f}  max {dLs.max():+.2f}  "
          f"mean {dLs.mean():+.2f}  columns lifted: {int((dLs > 0).sum())}/{len(dLs)}")
    print(f"    -> {'PASS' if a_ok else 'FAIL'}")

    # ---- (b) halo is blue -----------------------------------------------
    b_ok = med_dBR > 0
    print(f"\n(b) HALO IS BLUE  median((B-R)_near - (B-R)_far) = {med_dBR:+.2f}   (need > 0)")
    print(f"    per-column d(B-R): min {dBRs.min():+.2f}  max {dBRs.max():+.2f}  "
          f"mean {dBRs.mean():+.2f}  columns blue-led: {int((dBRs > 0).sum())}/{len(dBRs)}")
    print(f"    -> {'PASS' if b_ok else 'FAIL'}")

    # ---- (c) sky sanity --------------------------------------------------
    sky = a[m]
    n_sky = int(sky.shape[0])
    if n_sky == 0:
        print("\n(c) SKY SANITY    no sky pixels at all -> FAIL")
        c_ok = False
        mR = mG = mB = 0.0
        clipped = 0
        lstd01 = 0.0
    else:
        mR, mG, mB = (float(sky[:, i].mean()) for i in range(3))
        clipped = int((sky.min(axis=1) >= CLIP_LEVEL).sum())
        lstd255 = float(lum(sky).std())
        lstd01 = lstd255 / 255.0
        order_ok = mB > mG > mR
        clip_ok = clipped == 0
        std_ok = lstd01 > SKY_LSTD_MIN
        c_ok = order_ok and clip_ok and std_ok
        print(f"\n(c) SKY SANITY    {n_sky} sky px ({100.0 * n_sky / (H * W):.1f}% of frame)")
        print(f"    channel order   B={mB:.1f} > G={mG:.1f} > R={mR:.1f}"
              f"                     -> {'PASS' if order_ok else 'FAIL'}")
        print(f"    not blown out   sky px with min(R,G,B) >= {CLIP_LEVEL}: {clipped} "
              f"({100.0 * clipped / n_sky:.3f}% of sky)   -> {'PASS' if clip_ok else 'FAIL'}")
        print(f"    not a flat plate  L-std = {lstd01:.4f} (0..1) = {lstd255:.2f} (0..255) "
              f"= {lstd255 / 2.55:.2f} (0..100)")
        print(f"                      need > {SKY_LSTD_MIN} on the 0..1 scale"
              f"          -> {'PASS' if std_ok else 'FAIL'}")
        print(f"    -> {'PASS' if c_ok else 'FAIL'}")

    print("\n" + "=" * 84)
    print(f"HALO PROBE:  (a) halo {'PASS' if a_ok else 'FAIL'}   "
          f"(b) blue {'PASS' if b_ok else 'FAIL'}   "
          f"(c) sky {'PASS' if c_ok else 'FAIL'}")
    allp = a_ok and b_ok and c_ok
    print("=> " + ("HALO PRESENT AND BLUE, on a real sky"
                  if allp else "NOT PROVEN - see the FAILing check(s) above"))
    sys.exit(0 if allp else 1)


if __name__ == "__main__":
    main()
