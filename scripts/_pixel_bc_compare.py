#!/usr/bin/env python3
"""Flamingo/pixel - the B ~= C gate for the s2 bake, judged against this
capture's OWN noise floor instead of a threshold I made up.

  B  = hero cam, BAKED DEFAULT, no look env at all
  B2 = the same shot fired a second time -> the capture noise floor
  C  = hero cam + the s2 look env passed EXPLICITLY

If the s2 recipe really is baked in as the default, B and C are the same
picture. If the default is still an env crutch, C moves and B does not.

The floor matters: a renderer with any temporal/dither jitter will never give
a byte-identical re-shoot, and calling that jitter "a difference" would fail a
bake that is actually fine. So B-vs-B2 is measured first and B-vs-C only counts
as a real difference when it is clearly bigger than the frame's own re-shoot
noise.

exit 0 = B ~= C (pass)   1 = measured, they differ (fail)   2 = refused to measure
"""
import sys, os
import numpy as np
from PIL import Image


def load(p):
    return np.asarray(Image.open(p).convert("RGB")).astype(np.float64)


def lum(a):
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def stats(x, y, label):
    dl = np.abs(lum(x) - lum(y))
    d = np.abs(x - y)
    s = dict(
        mean=float(dl.mean()),
        p99=float(np.percentile(dl, 99)),
        mx=float(dl.max()),
        share=100.0 * float((dl > 1.0).mean()),
        chan=[float((x[..., i] - y[..., i]).mean()) for i in range(3)],
        chan_max=float(d.max()),
    )
    print(f"  {label}")
    print(f"    mean|dL| {s['mean']:8.4f}   p99|dL| {s['p99']:8.4f}   max|dL| {s['mx']:8.3f}"
          f"   pixels|dL|>1 {s['share']:6.3f}%")
    print(f"    signed mean dRGB  R {s['chan'][0]:+.4f}  G {s['chan'][1]:+.4f}  B {s['chan'][2]:+.4f}"
          f"   max|dCh| {s['chan_max']:.1f}")
    return s


def main():
    if len(sys.argv) != 4:
        print("usage: _pixel_bc_compare.py <B.png> <B2.png> <C.png>")
        return 2
    pb, pb2, pc = sys.argv[1:4]
    missing = [p for p in (pb, pb2, pc) if not os.path.exists(p)]
    if missing:
        print("REFUSED - missing: " + ", ".join(missing))
        return 2
    B, B2, C = load(pb), load(pb2), load(pc)
    if not (B.shape == B2.shape == C.shape):
        print(f"REFUSED - shape mismatch: B{B.shape} B2{B2.shape} C{C.shape}")
        return 2
    if B.size == 0 or float(lum(B).std()) < 0.5:
        print("REFUSED - B is degenerate (flat/empty frame); nothing to compare")
        return 2

    print(f"== B ~= C gate ==  {B.shape[1]}x{B.shape[0]}")
    print(f"   B  {os.path.basename(pb)}")
    print(f"   B2 {os.path.basename(pb2)}   (re-shoot of B -> noise floor)")
    print(f"   C  {os.path.basename(pc)}")
    floor = stats(B, B2, "noise floor   B vs B2 (same build, same env, shot twice)")
    test = stats(B, C, "the gate      B vs C  (baked default vs explicit s2 env)")

    tol_mean = max(3.0 * floor["mean"], 0.25)
    tol_max = max(2.0 * floor["mx"], 4.0)
    print(f"\n  tolerance derived from the floor: mean|dL| <= {tol_mean:.4f}   max|dL| <= {tol_max:.3f}")

    ok = test["mean"] <= tol_mean and test["mx"] <= tol_max
    if ok:
        print(f"  PASS - B ~= C. The s2 recipe IS the baked default; the env adds nothing.")
        return 0
    why = []
    if test["mean"] > tol_mean:
        why.append(f"mean|dL| {test['mean']:.4f} > {tol_mean:.4f}")
    if test["mx"] > tol_max:
        why.append(f"max|dL| {test['mx']:.3f} > {tol_max:.3f}")
    print(f"  FAIL - B != C ({'; '.join(why)}). The explicit env still changes the frame,")
    print(f"         so the baked default is NOT the whole s2 recipe.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
