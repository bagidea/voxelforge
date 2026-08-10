#!/usr/bin/env python3
"""Halo probe — does the SKY actually reach the bloom, or is it still a flat plate?

The claim under test (commit eff6c7b, `Hour::sky_gain`): `ClearColor` bypasses
`Exposure`, so the authored sky topped out at linear 0.79, under the bloom
prefilter threshold of 1.0, and could never bloom. A gain of 2.4 is said to put
BLUE at linear 1.89 while red (0.26) and green (0.76) stay under 1.0 — so if the
mechanism is real the halo must be there AND must be blue-dominant. A brighter
sky alone proves nothing: the tonemapper can lift a plate without any bloom.

What is measured, per frame:

  1. the sky/terrain silhouette, column by column, from the top down;
  2. the per-channel profile of the terrain just under that silhouette, out to
     `FAR` px, expressed RELATIVE to that frame's own far-field (so two frames
     shot at different exposures stay comparable);
  3. `halo dL` = the near-edge lift over far-field, and the same split per
     channel. Bloom = a lift that DECAYS with distance. A mere exposure change
     lifts the whole column flat and cancels in the relative profile.

With two frames it also prints the before/after delta, which is the actual
proof: bloom appears in the after-frame's relative profile and not the before's.

Usage: _poppy_halo_probe.py <after-nohud2.png> [<before-nohud2.png>]
"""
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402

NEAR = (1, 4)      # px under the silhouette = the halo band
FAR = (24, 40)     # px under the silhouette = un-bloomed far field
MIN_SKY_RUN = 24   # px of contiguous sky a column needs before it counts
PROFILE_TO = 40


def srgb_to_linear(c):
    c = c / 255.0
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def load(path):
    a = np.asarray(Image.open(path).convert("RGB")).astype(np.float64)
    return a


def sky_mask(a):
    """Sky = blue-dominant and not dark. Deliberately hue-based, not brightness-
    based: a brightness rule would also catch the sunlit ground the gate grades."""
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    return (B > R + 15) & (B > 70)


def silhouette(a):
    """First non-sky row per column, for columns that start with real sky."""
    m = sky_mask(a)
    H, W = m.shape
    ys = np.full(W, -1, dtype=int)
    for x in range(W):
        col = m[:, x]
        if not col[0]:
            continue
        # length of the contiguous sky run from the top
        nz = np.flatnonzero(~col)
        if nz.size == 0:
            continue          # column is sky all the way down — no edge in it
        y = int(nz[0])
        if y >= MIN_SKY_RUN:
            ys[x] = y
    return ys


def profile(a, ys):
    """Mean R,G,B at each depth d under the silhouette, over all valid columns."""
    H, W, _ = a.shape
    prof = np.full((PROFILE_TO + 1, 3), np.nan)
    for d in range(1, PROFILE_TO + 1):
        vals = []
        for x in range(W):
            y = ys[x]
            if y < 0:
                continue
            if y + d >= H:
                continue
            vals.append(a[y + d, x])
        if vals:
            prof[d] = np.mean(vals, axis=0)
    return prof


def lum(rgb):
    return 0.2126 * rgb[..., 0] + 0.7152 * rgb[..., 1] + 0.0722 * rgb[..., 2]


def band(prof, lo, hi):
    return np.nanmean(prof[lo:hi + 1], axis=0)


def report(path):
    a = load(path)
    ys = silhouette(a)
    n = int((ys >= 0).sum())
    H, W, _ = a.shape
    print(f"# {path}  {W}x{H}")
    if n < 40:
        print(f"  !! only {n} usable silhouette columns — probe cannot speak")
        return None

    m = sky_mask(a)
    sky_px = a[m]
    sky_mean = sky_px.mean(axis=0)
    sky_lin = srgb_to_linear(sky_mean)
    clipped = int((sky_px.max(axis=1) >= 250).sum())
    print(f"  sky: {m.sum()} px ({100*m.mean():.1f}% of frame)  "
          f"mean RGB=({sky_mean[0]:.0f},{sky_mean[1]:.0f},{sky_mean[2]:.0f})  "
          f"display-linear=({sky_lin[0]:.3f},{sky_lin[1]:.3f},{sky_lin[2]:.3f})  "
          f">=250 in any channel: {clipped} px")
    print(f"  silhouette columns used: {n}/{W}")

    prof = profile(a, ys)
    near, far = band(prof, *NEAR), band(prof, *FAR)
    rel = prof - far           # each frame against its OWN far field
    dR, dG, dB = near - far
    dL = lum(near) - lum(far)
    print(f"  halo (mean d={NEAR[0]}..{NEAR[1]} minus d={FAR[0]}..{FAR[1]}):"
          f"  dL={dL:+.2f}  dR={dR:+.2f}  dG={dG:+.2f}  dB={dB:+.2f}"
          f"   blue-lead dB-dR={dB-dR:+.2f}")
    print("  relative profile under the silhouette (frame's own far field = 0):")
    print("    d :  " + "  ".join(f"{d:>5}" for d in (1, 2, 3, 4, 6, 8, 12, 16, 24, 32)))
    for i, ch in enumerate("RGB"):
        print(f"    {ch} :  " + "  ".join(f"{rel[d][i]:+5.1f}" for d in
                                          (1, 2, 3, 4, 6, 8, 12, 16, 24, 32)))
    print("    L :  " + "  ".join(f"{lum(rel[d]):+5.1f}" for d in
                                  (1, 2, 3, 4, 6, 8, 12, 16, 24, 32)))
    # monotone decay over the first 8 px = a real bleed, not a bright band
    head = [lum(rel[d]) for d in range(1, 9)]
    decays = sum(head[i] >= head[i + 1] - 0.15 for i in range(len(head) - 1))
    print(f"  decay: {decays}/7 steps non-increasing over d=1..8   "
          f"(bloom bleeds and fades; a lit band would not)")
    return {"dL": dL, "dB": dB, "dR": dR, "rel": rel, "sky_lin": sky_lin,
            "decays": decays, "n": n}


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    require_nohud2(sys.argv[1:3], tool="_poppy_halo_probe.py")
    after = report(sys.argv[1])
    print()
    before = None
    if len(sys.argv) > 2:
        before = report(sys.argv[2])
        print()

    if after is None:
        sys.exit(1)
    print("=" * 64)
    verdict_bloom = after["dL"] > 1.0 and after["dB"] > after["dR"] and after["decays"] >= 5
    print(f"sky above the bloom threshold?  halo dL={after['dL']:+.2f} (need > +1.0), "
          f"blue-led dB-dR={after['dB']-after['dR']:+.2f} (need > 0), "
          f"decay {after['decays']}/7 (need >= 5)")
    if before is not None:
        print(f"before/after:  dL {before['dL']:+.2f} -> {after['dL']:+.2f}"
              f"   dB {before['dB']:+.2f} -> {after['dB']:+.2f}"
              f"   sky linear B {before['sky_lin'][2]:.3f} -> {after['sky_lin'][2]:.3f}")
        # The delta of the two relative profiles is the cleanest read there is:
        # the terrain's own bright horizon rim is in BOTH frames and cancels,
        # leaving only what the sky started adding.
        d = after["rel"] - before["rel"]
        print("  after-minus-before relative profile (terrain's own rim cancels):")
        cols = (1, 2, 3, 4, 6, 8, 12, 16, 24, 32)
        print("    d :  " + "  ".join(f"{c:>5}" for c in cols))
        for i, ch in enumerate("RGB"):
            print(f"    {ch} :  " + "  ".join(f"{d[c][i]:+5.1f}" for c in cols))
    print(f"=> {'HALO PRESENT (sky blooms)' if verdict_bloom else 'NO HALO - sky is still a flat plate'}")
    sys.exit(0 if verdict_bloom else 1)


if __name__ == "__main__":
    main()
