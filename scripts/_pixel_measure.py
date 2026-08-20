#!/usr/bin/env python3
"""Flamingo/pixel - the two numbers this round is judged on, and nothing else.

Two modes, because the two frames fail differently:

  --mode interior   HUE/VALUE SEPARATION.  Not "cool pixel %": that axis was
                    proven unreachable on an interior graded against an outdoor
                    valley reference (docs/VERDICT-b-clamp-root-cause-2026-08-20.md),
                    and chasing it is what flipped the room from sepia to blue
                    wash. What a viewer actually reads is whether the material
                    classes in frame land on DIFFERENT hues at DIFFERENT values.
                    So the frame is k-means'd into 5 colour clusters and the
                    spread BETWEEN cluster centres is reported. A sepia frame has
                    5 centres on one hue; a separated frame does not.

  --mode sky        SKY vs GROUND.  Fixed horizontal bands, printed with the
                    result so the reduction is never in doubt, and identical on
                    both sides of every A/B here. Band choice is plate-bound (see
                    px-thresholds-are-plate-bound): valid for the pinned
                    beach_dusk vista framing at 1280x720 ONLY.

Read-only. Prints a table; exits 0 unless a file is missing (2).
"""
import sys, os, math
import numpy as np
from PIL import Image

SKY_ROWS = (0, 55)        # beach_dusk pinned vista, 1280x720: pure dome
GND_ROWS = (420, 720)     # ... and pure lit ground


def rgb_hsl(a):
    """a: HxWx3 float 0..1 -> (hue_deg, sat, lightness)."""
    mx, mn = a.max(-1), a.min(-1)
    d = mx - mn
    L = (mx + mn) * 0.5
    denom = np.where(L < 0.5, mx + mn, 2.0 - mx - mn)
    S = np.where(d == 0, 0.0, d / np.where(denom == 0, 1.0, denom))
    dd = np.where(d == 0, 1.0, d)
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    h = np.where(mx == r, ((g - b) / dd) % 6,
        np.where(mx == g, (b - r) / dd + 2, (r - g) / dd + 4)) * 60.0
    return h, S, L


def hue_dist(h1, h2):
    d = abs(h1 - h2) % 360.0
    return min(d, 360.0 - d)


def kmeans(x, k, iters=40, seed=0):
    """Tiny deterministic k-means. seed fixed so a re-run reproduces the table."""
    rng = np.random.default_rng(seed)
    c = x[rng.choice(len(x), k, replace=False)]
    for _ in range(iters):
        d = ((x[:, None, :] - c[None, :, :]) ** 2).sum(-1)
        lab = d.argmin(1)
        for j in range(k):
            m = lab == j
            if m.any():
                c[j] = x[m].mean(0)
    return c, lab


def interior(path, k=5):
    im = Image.open(path).convert("RGB")
    a = np.asarray(im).astype(np.float64) / 255.0
    h, s, L = rgb_hsl(a)

    # Occupied hue bins - chromatic pixels only, and a bin has to hold >=0.5%
    # of them to count (so one stray texel is not "a colour in the frame").
    sel = (s > 0.06) & (L > 0.03) & (L < 0.97)
    hh = h[sel]
    bins = np.histogram(hh, bins=24, range=(0, 360))[0]
    occ = int((bins > max(1, hh.size * 0.005)).sum())
    span = float(np.percentile(hh, 95) - np.percentile(hh, 5)) if hh.size else 0.0

    # Cluster the frame and measure how far apart the clusters actually are.
    flat = a.reshape(-1, 3)
    step = max(1, len(flat) // 60000)
    c, _ = kmeans(flat[::step].copy(), k)
    ch, cs, cL = rgb_hsl(c[None, :, :])
    ch, cs, cL = ch[0], cs[0], cL[0]
    order = np.argsort(cL)
    ch, cs, cL, c = ch[order], cs[order], cL[order], c[order]
    hue_spread = max(hue_dist(ch[i], ch[j]) for i in range(k) for j in range(k))
    val_spread = float(cL.max() - cL.min())

    print(f"  {os.path.basename(path)}")
    print(f"    hue bins occupied  {occ}/24        hue p5-p95 span  {span:6.1f} deg")
    print(f"    cluster hue spread {hue_spread:6.1f} deg   cluster value spread {val_spread:6.3f}")
    print(f"    L  mean {L.mean():.3f}  p5 {np.percentile(L,5):.3f}  p95 {np.percentile(L,95):.3f}"
          f"   warm(R>B) {100*(a[...,0]>a[...,2]).mean():5.1f}%  cool(B>=R) {100*(a[...,2]>=a[...,0]).mean():5.1f}%")
    for i in range(k):
        rgb = tuple((c[i] * 255).round().astype(int).tolist())
        print(f"      c{i}  rgb{str(rgb):>18}  hue {ch[i]:6.1f}  sat {cs[i]:.2f}  L {cL[i]:.3f}")
    return dict(occ=occ, span=span, hue_spread=hue_spread, val_spread=val_spread)


def sky(path):
    im = Image.open(path).convert("RGB")
    a = np.asarray(im).astype(np.float64)
    if a.shape[0] != 720 or a.shape[1] != 1280:
        print(f"  {os.path.basename(path)}  REFUSED - bands are pinned to 1280x720, got {a.shape[1]}x{a.shape[0]}")
        return None
    lum = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
    sk, gd = lum[SKY_ROWS[0]:SKY_ROWS[1]], lum[GND_ROWS[0]:GND_ROWS[1]]
    skr = a[SKY_ROWS[0]:SKY_ROWS[1]].reshape(-1, 3)
    sm, gm = float(np.median(sk)), float(np.median(gd))
    blown = 100.0 * float((skr.min(1) >= 250).mean())
    h, s, L = rgb_hsl(a[SKY_ROWS[0]:SKY_ROWS[1]] / 255.0)
    chroma = s > 0.04
    hspan = float(np.percentile(h[chroma], 95) - np.percentile(h[chroma], 5)) if chroma.any() else 0.0
    print(f"  {os.path.basename(path)}")
    print(f"    sky_median_L {sm:7.2f}   ground_median_L {gm:7.2f}   ratio {sm/gm:6.3f}   (target 1.80)")
    print(f"    sky rgb mean {str(tuple(skr.mean(0).round(1).tolist()))}  gradient p95-p5 {np.percentile(sk,95)-np.percentile(sk,5):6.2f}"
          f"  hue span {hspan:5.1f}  blown {blown:5.2f}%")
    return dict(ratio=sm / gm, sky=sm, gnd=gm)


def main():
    mode = "interior"
    args = []
    for a in sys.argv[1:]:
        if a == "--mode":
            mode = None
        elif mode is None:
            mode = a
        else:
            args.append(a)
    missing = [p for p in args if not os.path.exists(p)]
    if missing:
        print("REFUSED - missing: " + ", ".join(missing))
        return 2
    print(f"== mode {mode} ==")
    if mode == "sky":
        print(f"   bands: sky rows {SKY_ROWS}, ground rows {GND_ROWS} (1280x720 beach_dusk vista only)")
    for p in args:
        (sky if mode == "sky" else interior)(p)
    return 0


if __name__ == "__main__":
    sys.exit(main())
