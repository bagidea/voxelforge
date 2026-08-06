#!/usr/bin/env python3
"""Rose — reconstruct the haze-intensity metric (net_mean + %>=T) and find which
definition reproduces the ~1.5 / ~10% plateau the cut sweep reported.

Two families are computed so the DATA picks the metric, not a guess:

  A/B  (needs a haze-off baseline): for a (off, on) pair, per-pixel |dL| on Rec709
       luminance 0..100.  net_mean = mean |dL| over frame; %>=T = fraction of
       pixels with |dL| >= T.  This is the natural haze-intensity reading and
       matches the _delta machinery in grade_g7.py / the "B SSAO not biting" row.

  SINGLE-IMAGE (so the golden reference -- one PNG, no pair -- can be graded the
       same way every other rubric target is): proxies that read haze from one
       frame -- depth-stratified desaturation, haze-colour proximity, top/bottom
       luminance gradient.  These let us put a number on the reference at all.

Usage:
  _rose_haze_delta.py --ab <off.png> <on.png> [<on.png> ...]
  _rose_haze_delta.py --single <img.png> [<img.png> ...]
"""
import argparse
import sys
from PIL import Image


def lum(r, g, b):
    return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255 * 100


def load(path):
    return Image.open(path).convert("RGB")


def ab(off_path, on_paths):
    off = load(off_path)
    a = off.load()
    W, H = off.size
    print(f"# A/B haze delta   baseline(off)={off_path}   {W}x{H}\n")
    print(f"{'on':28s} {'net_mean':>8s} {'%>=2':>6s} {'%>=5':>6s} {'%>=10':>6s} "
          f"{'meanmaxCh':>8s} {'p95':>7s}   | per-band mean|dL| / %>=5 (far/mid/near by row thirds)")
    for on_path in on_paths:
        on = load(on_path)
        if on.size != off.size:
            print(f"{on_path:28s}  SIZE MISMATCH {on.size} -- skipped")
            continue
        b = on.load()
        dl = []  # |dL|
        mc = []  # max |channel|
        # depth bands by row thirds: top=far, mid, bottom=near
        bands = {"far": [], "mid": [], "near": []}
        for y in range(0, H, 2):
            for x in range(0, W, 2):
                p, q = a[x, y], b[x, y]
                v = abs(lum(*p) - lum(*q))
                dl.append(v)
                mc.append(max(abs(p[i] - q[i]) for i in range(3)))
                if y < H / 3:
                    bands["far"].append(v)
                elif y < 2 * H / 3:
                    bands["mid"].append(v)
                else:
                    bands["near"].append(v)
        n = len(dl)
        net = sum(dl) / n
        ge2 = 100 * sum(v >= 2 for v in dl) / n
        ge5 = 100 * sum(v >= 5 for v in dl) / n
        ge10 = 100 * sum(v >= 10 for v in dl) / n
        mmc = sum(mc) / n
        dl.sort()
        p95 = dl[int(len(dl) * 0.95)]
        bn = {k: (sum(v) / len(v) if v else 0,
                  100 * sum(x >= 5 for x in v) / len(v) if v else 0)
              for k, v in bands.items()}
        print(f"{on_path:28s} {net:8.3f} {ge2:6.1f} {ge5:6.1f} {ge10:6.1f} {mmc:8.2f} {p95:7.2f}"
              f"   | far {bn['far'][0]:5.2f}/{bn['far'][1]:4.1f}"
              f"  mid {bn['mid'][0]:5.2f}/{bn['mid'][1]:4.1f}"
              f"  near {bn['near'][0]:5.2f}/{bn['near'][1]:4.1f}")


def single(paths, haze_rgb=None):
    """Haze proxies readable from ONE frame.

    HAZE_COLOUR proximity needs the shipped haze colour; pass it as --haze R,G,B
    (0..255). If absent, that column is skipped (the other proxies don't need it).
    """
    haze = None
    if haze_rgb:
        haze = tuple(float(x) for x in haze_rgb.split(","))
        hl = lum(*haze)
    print(f"# SINGLE-IMAGE haze proxies   haze_colour={haze}\n")
    head = (f"{'img':28s} {'meanL':>6s} {'p05L':>6s} {'p50L':>6s} {'p95L':>6s} "
            f"{'topL':>6s} {'botL':>6s} {'top-bot':>7s} {'farSat':>6s} {'nearSat':>6s}")
    if haze:
        head += f" {'mean|L-hL|':>10s} {'%|d|>=5':>8s}"
    print(head)
    for path in paths:
        img = load(path)
        px = img.load()
        W, H = img.size
        L = []
        for y in range(0, H, 2):
            for x in range(0, W, 2):
                L.append(lum(*px[x, y]))
        L.sort()
        n = len(L)
        meanL = sum(L) / n
        p05, p50, p95 = L[int(n * .05)], L[int(n * .50)], L[int(n * .95)]

        # depth-stratified luminance (top = far/sky band, bottom = near band),
        # band = top/bottom 20% of rows
        bh = max(1, H // 5)
        topL = botL = 0.0
        tn = bn = 0
        far_sat = near_sat = 0.0
        for y in range(0, H, 2):
            for x in range(0, W, 2):
                r, g, b = px[x, y]
                l = lum(r, g, b)
                mx, mn = max(r, g, b), min(r, g, b)
                sat = 0.0 if mx == 0 else 100 * (mx - mn) / mx
                if y < bh:
                    topL += l; tn += 1; far_sat += sat
                elif y >= H - bh:
                    botL += l; bn += 1; near_sat += sat
        topL /= max(1, tn); botL /= max(1, bn)
        far_sat /= max(1, tn); near_sat /= max(1, bn)

        row = (f"{path:28s} {meanL:6.2f} {p05:6.2f} {p50:6.2f} {p95:6.2f} "
               f"{topL:6.2f} {botL:6.2f} {topL - botL:7.2f} {far_sat:6.1f} {near_sat:6.1f}")
        if haze:
            dl = []
            for y in range(0, H, 2):
                for x in range(0, W, 2):
                    dl.append(abs(lum(*px[x, y]) - hl))
            mn = sum(dl) / len(dl)
            ge5 = 100 * sum(v >= 5 for v in dl) / len(dl)
            row += f" {mn:10.3f} {ge5:8.1f}"
        print(row)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ab", nargs="+", metavar=("OFF", "ON"))
    ap.add_argument("--single", nargs="+", metavar="IMG")
    ap.add_argument("--haze", default=None, help="haze colour R,G,B 0..255 for proximity proxy")
    a = ap.parse_args()
    if a.ab:
        ab(a.ab[0], a.ab[1:])
    if a.single:
        single(a.single, a.haze)
    return 0


if __name__ == "__main__":
    sys.exit(main())
