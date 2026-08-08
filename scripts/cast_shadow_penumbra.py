#!/usr/bin/env python3
"""Is there a CAST SHADOW in this frame, and is its edge SOFT? (G4a)

`scripts/measure_penumbra.py` answers the second question without asking the
first: it takes the strongest luminance step in each floor row and measures its
20->80 ramp. On the 2026-08-07 plates that reported "penumbra 6-21 px" for
frames whose strongest floor steps were the seam between two block FACES and
the seam where grass meets terracotta. Neither is a shadow; both are steps in
the picture. So this file is built around the three ways that measurement lies:

  1. NOT EVERY DARK REGION IS A SHADOW. The grass block's own albedo is a
     light/dark checker, and once the key light raised contrast the checker
     resolves into two luminance humps all by itself. `--albedo-check` settles
     it against the pre-light plate of the same camera: if the pixels that are
     "in shade" now are the same pixels that were already the dark half then,
     the split is painted into the texture, not cast by a light.
  2. NOT EVERY EDGE IS A SHADOW EDGE. A material boundary and a block-face
     seam both produce a clean luminance step. Neither is excluded by any
     numeric guard that survives contact with this renderer -- a hue-continuity
     test looked obvious and is WRONG here, measured: across a real cast-shadow
     edge on the terracotta wall of `gate3-boot` the hue swings 14.8 -> 36.4 deg,
     because the key is orange and the fill is sky-blue. So edges are not
     auto-classified. `--at X,Y` measures an edge a human has pointed at, and
     the frame-wide sweep is reported as what it is: every strong edge, shadow
     or not.
  3. A WIDTH MEANS NOTHING WITHOUT THE CONTROL. This renderer's edges are
     already 1-3 px from AA/TAA whatever the light does, and a DIAGONAL edge is
     wider than an axis-aligned one at identical softness, purely from the
     raster staircase. So every run also measures the sky silhouette -- pure
     geometry, no penumbra by definition -- and compares against the control
     edges in the SAME orientation bucket. Comparing a diagonal shadow edge to a
     vertical silhouette is how a hard shadow gets reported as soft.

Usage:
  cast_shadow_penumbra.py <frame>-nohud2.png [more...] [--at X,Y[;X,Y...]]
                          [--albedo-check <before>-nohud2.png]
"""
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from grass_bimodality import grass_select, hsv_planes, lum, shape  # noqa: E402
from nohud2_guard import require_nohud2  # noqa: E402

R = 14.0             # px — half-length of a sampled profile
DT = 0.25            # px — profile sample pitch
PLATEAU = 4.0        # px at each end that must be flat
PLATEAU_SD = 3.0     # L — above this the "plateau" is texture, not a level
MIN_STEP = 10.0      # L — dark->light range needed before a ramp is gradeable
MAX_EDGES = 600      # profiles per class; more is only slower
SKY_HUE = (195.0, 265.0)
ORIENT_BINS = [(0, 15), (15, 30), (30, 46)]  # deg of the normal off the nearest axis


def box3(a):
    p = np.pad(a, 1, mode="edge")
    return sum(p[y:y + a.shape[0], x:x + a.shape[1]]
               for y in range(3) for x in range(3)) / 9.0


def label(mask):
    """Two-pass 4-connected CCL; no scipy on this box. The python loop walks the
    mask's own pixels only, which is thousands, not the whole frame."""
    H, W = mask.shape
    lab = np.zeros((H, W), np.int32)
    parent = [0]

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    nxt = 1
    for y in range(H):
        for x in np.flatnonzero(mask[y]):
            up = lab[y - 1, x] if y else 0
            lf = lab[y, x - 1] if x else 0
            if up and lf:
                ru, rl = find(up), find(lf)
                if ru != rl:
                    parent[max(ru, rl)] = min(ru, rl)
                lab[y, x] = min(up, lf)
            elif up or lf:
                lab[y, x] = up or lf
            else:
                parent.append(nxt)
                lab[y, x] = nxt
                nxt += 1
    flat = np.array([find(i) for i in range(nxt)], np.int32)
    _, lab = np.unique(flat[lab], return_inverse=True)
    return lab.reshape(H, W)


def sample(a, xs, ys):
    H, W = a.shape
    xs, ys = np.clip(xs, 0, W - 1.001), np.clip(ys, 0, H - 1.001)
    x0, y0 = xs.astype(int), ys.astype(int)
    fx, fy = xs - x0, ys - y0
    return (a[y0, x0] * (1 - fx) * (1 - fy) + a[y0, x0 + 1] * fx * (1 - fy)
            + a[y0 + 1, x0] * (1 - fx) * fy + a[y0 + 1, x0 + 1] * fx * fy)


class Frame:
    def __init__(self, path):
        self.path = path
        self.arr = np.asarray(Image.open(path).convert("RGB"))
        self.H, self.W = self.arr.shape[:2]
        self.L = lum(self.arr)
        self.Ls = box3(self.L)                       # measured on
        big = box3(box3(box3(self.L)))               # normal direction only
        self.gy, self.gx = np.gradient(big)
        hue, sat, val = hsv_planes(self.arr)
        self.sky = ((hue >= SKY_HUE[0]) & (hue <= SKY_HUE[1]) & (sat > 20) & (val > 40))

    def profile(self, x, y, allow_sky=False):
        """(width20-80, orientation deg) for the edge at (x,y), or None."""
        g = np.hypot(self.gx[y, x], self.gy[y, x])
        if g < 1e-6:
            return None
        nx, ny = self.gx[y, x] / g, self.gy[y, x] / g       # points dark -> light
        t = np.arange(-R, R + DT, DT)
        xs, ys = x + nx * t, y + ny * t
        if not allow_sky:
            xi = np.clip(np.rint(xs), 0, self.W - 1).astype(int)
            yi = np.clip(np.rint(ys), 0, self.H - 1).astype(int)
            if self.sky[yi, xi].any():
                return None
        v = sample(self.Ls, xs, ys)
        n = int(PLATEAU / DT)
        dark, light = v[:n], v[-n:]
        if dark.std() > PLATEAU_SD or light.std() > PLATEAU_SD:
            return None
        d, l = dark.mean(), light.mean()
        if l - d < MIN_STEP:
            return None
        lo, hi = d + 0.2 * (l - d), d + 0.8 * (l - d)

        def cross(level):
            i = int(np.argmax(v >= level))
            if i == 0 or v[i] <= v[i - 1]:
                return None
            return t[i - 1] + (level - v[i - 1]) / (v[i] - v[i - 1]) * DT

        t20, t80 = cross(lo), cross(hi)
        if t20 is None or t80 is None or t80 < t20:
            return None
        ang = np.degrees(np.arctan2(abs(ny), abs(nx)))
        return t80 - t20, min(ang, 90.0 - ang)

    def ridge(self, mask=None):
        """Pixels that are a local maximum of |grad L| along the gradient."""
        g = np.hypot(self.gx, self.gy)
        keep = np.zeros_like(g, bool)
        i = g > 1.0
        ys, xs = np.nonzero(i)
        nx, ny = self.gx[i] / g[i], self.gy[i] / g[i]
        a = sample(g, xs + nx, ys + ny)
        b = sample(g, xs - nx, ys - ny)
        keep[ys, xs] = (g[i] >= a) & (g[i] >= b)
        return keep & mask if mask is not None else keep


def thin(mask, n):
    ys, xs = np.nonzero(mask)
    if ys.size <= n:
        return list(zip(ys, xs))
    step = max(1, ys.size // n)
    return list(zip(ys[::step][:n], xs[::step][:n]))


def collect(fr, pts, allow_sky):
    w, o = [], []
    for y, x in pts:
        r = fr.profile(int(x), int(y), allow_sky)
        if r:
            w.append(r[0])
            o.append(r[1])
    return np.array(w), np.array(o)


def by_orient(w, o):
    return {b: w[(o >= b[0]) & (o < b[1])] for b in ORIENT_BINS}


def fmt(a):
    if not a.size:
        return "     n=0"
    return (f"n={a.size:4d} median={np.median(a):5.2f}px "
            f"p25={np.percentile(a, 25):5.2f} p75={np.percentile(a, 75):5.2f}")


def albedo_check(after_path, before_path):
    """Is the frame's shade population cast by a light, or painted in the
    albedo? Answered against the same camera before the light changed."""
    a = np.asarray(Image.open(after_path).convert("RGB"))
    b = np.asarray(Image.open(before_path).convert("RGB"))
    if a.shape != b.shape:
        print("   albedo-check: frames differ in size -- not the same camera, skipped")
        return
    m, _, _ = grass_select(a)
    La, Lb = lum(a), lum(b)
    s = shape(La[m], 0.0, 100.0, 100, 2.5) if m.any() else {"peaks": None}
    if not s["peaks"]:
        print("   albedo-check: grass reads ONE hump -- no shade population to test")
        return
    shade = m & (La < s["valley"])
    lab = label(shade)
    areas = np.bincount(lab.ravel())[1:]
    big = np.isin(lab, np.flatnonzero(areas >= 500) + 1) & shade
    mb, _, _ = grass_select(b)
    dark_then = Lb < np.median(Lb[mb if mb.any() else m])
    ov, base = dark_then[big].mean(), dark_then[m].mean()
    print(f"   albedo-check: shade cut L={s['valley']:.1f}, {int(big.sum())}px in blobs>=500px; "
          f"{100 * ov:.1f}% of them were ALREADY the dark half before the light changed "
          f"(base rate {100 * base:.1f}%)")
    print("                 -> " + ("PAINTED IN THE ALBEDO, not a new cast shadow"
                                    if ov > 0.80 else
                                    "not albedo-locked -- consistent with a real cast shadow"))


def run(path, ats, before):
    print(f"== {os.path.splitext(os.path.basename(path))[0]}")
    fr = Frame(path)
    if before:
        albedo_check(path, before)

    ctl_w, ctl_o = collect(fr, thin(fr.ridge(fr.sky), MAX_EDGES), allow_sky=True)
    ctl = by_orient(ctl_w, ctl_o)
    print(f"   CONTROL sky silhouette (geometry, zero penumbra) : {fmt(ctl_w)}")
    for b in ORIENT_BINS:
        print(f"      normal {b[0]:2d}-{b[1]:2d}deg off-axis : {fmt(ctl[b])}")

    all_w, all_o = collect(fr, thin(fr.ridge(~fr.sky), MAX_EDGES), allow_sky=False)
    print(f"   ALL in-scene edges (shadow AND geometry mixed)   : {fmt(all_w)}")
    for b in ORIENT_BINS:
        print(f"      normal {b[0]:2d}-{b[1]:2d}deg off-axis : {fmt(by_orient(all_w, all_o)[b])}")

    named = []
    for (x, y) in ats:
        r = fr.profile(x, y)
        if not r:
            print(f"   --at ({x},{y}) : no gradeable ramp here "
                  "(flat, textured, under MIN_STEP, or touching sky)")
            continue
        w, ang = r
        named.append(w)
        pool = next((ctl[b] for b in ORIENT_BINS if b[0] <= ang < b[1]), np.array([]))
        if pool.size:
            ref = np.median(pool)
            verdict = "SOFT" if w >= 1.5 * ref else "HARD (at the AA/TAA floor)"
            print(f"   --at ({x},{y}) : width {w:5.2f}px  normal {ang:.0f}deg off-axis  "
                  f"vs control {ref:5.2f}px = {w / ref:.2f}x -> {verdict}")
        else:
            print(f"   --at ({x},{y}) : width {w:5.2f}px  normal {ang:.0f}deg off-axis  "
                  "-> no control edge at this orientation, UNCALIBRATED")
    if named:
        n = np.array(named)
        print(f"   --at pooled  : {fmt(n)}  vs CONTROL {fmt(ctl_w)}"
              + (f"  = {np.median(n) / np.median(ctl_w):.2f}x" if ctl_w.size else ""))
    print()


def main(argv=None):
    argv = list(argv if argv is not None else sys.argv[1:])
    ats, before, plates = [], None, []
    while argv:
        a = argv.pop(0)
        if a == "--at":
            ats = [tuple(int(v) for v in p.split(",")) for p in argv.pop(0).split(";")]
        elif a == "--albedo-check":
            before = argv.pop(0)
        else:
            plates.append(a)
    # guarded here, not at import: the `[E]` prompt glyphs sit on the ground and
    # are a far stronger step than any shadow edge, so a raw capture would be
    # graded on text.
    require_nohud2(plates + ([before] if before else []), tool="cast_shadow_penumbra.py")
    for p in plates:
        run(p, ats, before)


if __name__ == "__main__":
    main()
