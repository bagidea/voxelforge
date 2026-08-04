#!/usr/bin/env python3
"""Pick HAZE_FULL so the new curve keeps the signed-off look and drops the wash.

The G7 default is FogFalloff::ExponentialSquared @ 0.0072. Measured (see
_flamingo_g7_alpha.py) it applies 1.15-1.6% at 16-20 blocks -- the density curve
IS being honoured, there is no extra near-field term. But 1.6% of a haze colour
that is several times the radiance of shadowed near-field ground is still ~19
display units, and no value of the density changes that: across the whole 16-row
sweep the depth ratio sits at 2.2-2.4 whatever the density or the haze colour.

ExponentialSquared has no offset parameter, so the only Bevy falloff that can put
a genuine dead zone in front of the lens is Linear{start, end}. The question this
answers is whether a linear ramp can do that WITHOUT changing the look that was
already signed off: how close does Linear{20, end} track ExpSq(0.0072) across the
depth range the pinned framing actually contains (measured: 16..75 blocks)?
"""
import math

RHO = 0.0072
START = 20.0
VISIBLE = list(range(26, 80, 2))      # the measured depth span of the set
TABLE = [16, 20, 26, 30, 40, 55, 75, 100, 150, 200, 260, 300, 320]


def expsq(d):
    return 1.0 - math.exp(-((d * RHO) ** 2))


def lin(d, end):
    return max(0.0, min(1.0, (d - START) / (end - START)))


best, best_err = None, 1e9
for end in range(180, 340, 2):
    err = math.sqrt(sum((lin(d, end) - expsq(d)) ** 2 for d in VISIBLE) / len(VISIBLE))
    if err < best_err:
        best, best_err = end, err
print(f"best-fit end over d={VISIBLE[0]}..{VISIBLE[-1]}: {best}  (rms {100 * best_err:.2f} pp)\n")

ends = [best, 240, 250, 260]
print(f"{'d':>5s} {'ExpSq%':>8s} " + " ".join(f"Lin{e}%".rjust(9) for e in ends))
for d in TABLE:
    print(f"{d:5d} {100 * expsq(d):8.2f} "
          + " ".join(f"{100 * lin(d, e):9.2f}" for e in ends))

print(f"\n{'end':>5s} {'rms over set':>13s} {'max|dev| 26..75':>16s} "
      f"{'opaque at':>10s} {'alpha@16':>9s} {'alpha@20':>9s}")
for e in ends:
    rms = math.sqrt(sum((lin(d, e) - expsq(d)) ** 2 for d in VISIBLE) / len(VISIBLE))
    mx = max(abs(lin(d, e) - expsq(d)) for d in VISIBLE)
    print(f"{e:5d} {100 * rms:12.2f}pp {100 * mx:15.2f}pp {e:10d} "
          f"{100 * lin(16, e):8.2f}% {100 * lin(20, e):8.2f}%")
print(f"\nExpSq for reference:                                    "
      f"320(99.5%) {100 * expsq(16):8.2f}% {100 * expsq(20):8.2f}%")
