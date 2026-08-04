#!/usr/bin/env python3
"""A2 root cause: MEASURE the depth and MEASURE the alpha, don't assume either.

Both come out of frames the shipped binary already produced (`_flamingo_g7probe`),
so nothing here rests on my reading of the shader.

DEPTH.  `VOXELFORGE_LOOK_FOG=S,S+0.5` makes fog alpha a STEP at distance S: a
pixel nearer than S is byte-identical to the zero-fog frame, one further is 100%
fog colour.  Rising S across a ladder brackets every pixel's distance.

ALPHA.  `VOXELFORGE_LOOK_FOG=-30000A,30000-30000A` makes alpha a CONSTANT A over
the whole depth range (it varies 1% absolute across 0..300 blocks).  That ladder
is a per-pixel response curve -- display value as a function of a KNOWN alpha --
so the ship frame's own value reads back as the alpha the shader actually
applied, tonemap/bloom/grade included.  Compare to 1-exp(-(d*rho)^2) at the
measured d and the curve is either honoured or it isn't.

--selftest FIRST.  The ladder is sampled at 0.5/1/2/3/5/8/12/16/25/60%; each
point is held out in turn and read back off the remaining points.  If the
instrument cannot recover an alpha it was literally given, its verdict on the
near field is worth nothing -- and the coarse first-pass ladder (0.05 upward)
failed exactly this way, over-reading the 0-5% bucket because a straight chord
under a concave tonemap response is a biased estimator precisely there.
"""
import argparse
import math
import sys

from PIL import Image

G7 = "_flamingo_g7"
PR = "_flamingo_g7probe"
RHO = 0.0072
SLICES = [8, 12, 16, 20, 25, 30, 40, 55, 75, 110]
# (alpha, file stem) -- the fine ladder from probe2, plus the coarse tail.
UNIS = [(0.005, "u5"), (0.01, "u10"), (0.02, "u20"), (0.03, "u30"), (0.05, "u50"),
        (0.08, "u80"), (0.12, "u120"), (0.16, "u160"), (0.25, "u250"), (0.40, "uni40"),
        (0.60, "u600"), (1.00, "uni100")]
STEP = 4
CHANGED = 6
MIN_SPAN = 25     # a=0 -> a=1 travel a pixel needs before its alpha is readable


def load(path):
    return Image.open(path).convert("RGB")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frame", default=f"{G7}/ship-nohud2.png")
    a = ap.parse_args()

    off_img = load(f"{G7}/hazeoff-nohud2.png")
    W, H = off_img.size
    off = off_img.load()
    sl = {s: load(f"{PR}/slice{s}-nohud2.png").load() for s in SLICES}
    un = [(av, load(f"{PR}/{n}-nohud2.png").load()) for av, n in UNIS]

    def diff(p, q):
        return max(abs(p[i] - q[i]) for i in range(3))

    def is_sky(p):
        return p[2] > 150 and p[2] > p[0] + 30 and p[1] > p[0]

    def depth_of(x, y):
        """Bracketed distance in blocks; None if beyond the last slice.

        A pixel is fogged by `FOG=S,S+0.5` iff it is FURTHER than S, so the
        bracket is the first S at which the pixel STOPS moving: d <= S.
        """
        for s in SLICES:
            if diff(off[x, y], sl[s][x, y]) < CHANGED:
                return s
        return None

    def alpha_of(obs, x, y, skip=None):
        """Read alpha back off this pixel's own response curve."""
        pts = [(0.0, off[x, y])] + [(av, m[x, y]) for av, m in un if av != skip]
        ch = max(range(3), key=lambda i: abs(pts[-1][1][i] - pts[0][1][i]))
        if abs(pts[-1][1][ch] - pts[0][1][ch]) < MIN_SPAN:
            return None
        v = obs[x, y][ch]
        pa, pv = pts[0][0], pts[0][1][ch]
        for av, p in pts[1:]:
            cur = p[ch]
            if min(pv, cur) - 1 <= v <= max(pv, cur) + 1:
                t = 0.0 if cur == pv else (v - pv) / (cur - pv)
                return pa + max(0.0, min(1.0, t)) * (av - pa)
            pa, pv = av, cur
        return None

    def scan(obs, skip=None):
        """(depth bucket -> [measured alpha]) over the non-sky frame."""
        out = {s: [] for s in SLICES}
        for y in range(0, H, STEP):
            for x in range(0, W, STEP):
                if is_sky(off[x, y]):
                    continue
                d = depth_of(x, y)
                if d is None:
                    continue
                m = alpha_of(obs, x, y, skip)
                if m is not None:
                    out[d].append(m)
        return out

    if a.selftest:
        # Hold-out is DELIBERATELY pessimistic: dropping a point doubles the gap
        # the estimator has to span, so the error printed here is ~2x the error
        # the full ladder carries. The verdict is scoped to the band the A2
        # argument lives in (alpha <= 16%, i.e. every pixel inside ~85 blocks);
        # the 25/40/60% rows are reported but not gated, their ladder spacing is
        # coarse on purpose and no near-field claim rests on them.
        print("# SELFTEST -- hold each ladder point out, read it back off the rest")
        print(f"{'given':>8s} {'read':>8s} {'err':>8s}  {'n':>7s}  gated")
        bad = 0
        for av, n in UNIS:
            if av >= 1.0:
                continue
            obs = load(f"{PR}/{n}-nohud2.png").load()
            vals = [v for b in scan(obs, skip=av).values() for v in b]
            got = sum(vals) / len(vals)
            err = got - av
            gated = av <= 0.16
            flag = "  <-- BIASED" if gated and abs(err) > 0.010 else ""
            if flag:
                bad += 1
            print(f"{av * 100:7.2f}% {got * 100:7.2f}% {err * 100:+7.2f}% {len(vals):7d}"
                  f"  {'yes' if gated else 'no '}{flag}")
        print(f"\n-> instrument {'FAILS' if bad else 'OK'} over alpha <= 16% "
              f"({bad} biased points); worst gated |err| is the near-field error bar\n")
        if bad:
            return 1

    obs = load(a.frame).load()
    print(f"# {a.frame}   {W}x{H}   rho={RHO}\n")
    print("## measured alpha vs the shipped curve, bucketed by MEASURED depth")
    print(f"{'d<=':>5s} {'n':>7s} {'a_pred%':>8s} {'a_meas%':>8s} {'excess':>7s}")
    rows = []
    for s, v in scan(obs).items():
        if len(v) < 40:
            continue
        pred = 100 * (1.0 - math.exp(-((s * RHO) ** 2)))
        meas = 100 * sum(v) / len(v)
        rows.append((s, len(v), pred, meas))
        print(f"{s:5d} {len(v):7d} {pred:8.2f} {meas:8.2f} {meas / pred:6.2f}x")

    if len(rows) >= 2:
        (d1, _, p1, m1), (d2, _, p2, m2) = rows[0], rows[-1]
        k = (m2 - m1) / (p2 - p1)
        c = m1 - k * p1
        print(f"\n   fit  a_meas = {k:.3f} * a_curve + {c:+.2f}%"
              f"   (a constant floor of {c:.2f}% is the A2 defect if c >> 0)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
