#!/usr/bin/env python3
"""A2 near-field diagnosis: does the near-band delta scale like the density curve?

ExponentialSquared alpha = 1 - exp(-(d*rho)^2) ~= (d*rho)^2 for small d*rho.
So at a FIXED pixel (fixed depth d), the haze delta vs the zero-fog frame must
scale as rho^2.  If it does, the curve is being evaluated honestly and the near
band is simply further away than assumed.  If the delta is flat (or scales far
weaker than rho^2), there is a density-INDEPENDENT term on top of the curve.

No rebuild: reads the 16-row sweep PNGs already on disk.
"""
import sys
from PIL import Image

D = "_flamingo_g7"
ROWS = [("h0068", 0.0068), ("ship", 0.0072), ("h0090", 0.0090), ("h0120", 0.0120)]


def load(n):
    return Image.open(f"{D}/{n}-nohud2.png").convert("RGB")


def band_delta(off, on, y0, y1, x0=0, x1=None):
    a, b, (W, H) = off.load(), on.load(), off.size
    x1 = W if x1 is None else x1
    s = n = 0
    for y in range(y0, y1, 2):
        for x in range(x0, x1, 2):
            p, q = a[x, y], b[x, y]
            s += max(abs(p[i] - q[i]) for i in range(3))
            n += 1
    return s / n if n else 0.0


def main():
    off = load("hazeoff")
    W, H = off.size
    bands = {
        "far  (top 18%)": (0, int(H * 0.18), 0, W),
        "near (bot 18%)": (H - int(H * 0.18), H, 0, W),
        "nearest (bot 8%)": (H - int(H * 0.08), H, 0, W),
        "nearest-centre": (H - int(H * 0.08), H, int(W * 0.35), int(W * 0.65)),
    }
    print(f"# {W}x{H}   baseline = hazeoff (linear FOG_START 112 == zero fog in a ~55-block set)\n")
    ref = {}
    for label, dens in ROWS:
        on = load(label)
        out = []
        for bn, (y0, y1, x0, x1) in bands.items():
            d = band_delta(off, on, y0, y1, x0, x1)
            ref.setdefault(bn, {})[dens] = d
            out.append(f"{bn}={d:6.2f}")
        print(f"rho={dens:.4f}  " + "  ".join(out))

    print("\n## scaling vs rho=0.0072 (ship).  curve predicts delta ~ rho^2")
    for bn, m in ref.items():
        base = m[0.0072]
        print(f"\n  {bn}")
        for dens, d in sorted(m.items()):
            pred = (dens / 0.0072) ** 2
            got = d / base if base else 0
            print(f"    rho={dens:.4f}  predicted x{pred:5.2f}   measured x{got:5.2f}"
                  f"   ({'curve' if abs(got - pred) < 0.25 else 'OFF-CURVE'})")

    print("\n## implied constant term (fit delta = k*rho^2 + c on the two extremes)")
    for bn, m in ref.items():
        r1, r2 = 0.0068, 0.0120
        d1, d2 = m[r1], m[r2]
        k = (d2 - d1) / (r2**2 - r1**2)
        c = d1 - k * r1**2
        print(f"  {bn:18s} k={k:10.1f}  c={c:6.2f}   "
              f"(curve share at rho=0.0072: {100 * (k * 0.0072**2) / m[0.0072]:5.1f}%, "
              f"constant share {100 * c / m[0.0072]:5.1f}%)")


if __name__ == "__main__":
    sys.exit(main())
