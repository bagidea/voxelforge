#!/usr/bin/env python3
"""Refit HAZE_FULL against the haze density the grade was actually verified at.

WHY THIS EXISTS. `scripts/_flamingo_g7_curvefit.py` picked HAZE_FULL = 250 by
least-squares fitting `Linear{20, end}` to `ExponentialSquared(0.0072)` over the
pinned framing's depth span, i.e. "keep the signed-off look, drop the near-field
wash". That is still the right method. What changed is the target: commit
bd3cde5 verified the vista grade at haze density 0.0100, and it did so through
`VOXELFORGE_LOOK_HAZE=0.0100`, which does not bump a density on the shipped
ramp -- it SWAPS THE FALLOFF back to `ExponentialSquared`. So the value that was
graded is a curve the shipped binary does not run, and baking `HAZE_DENSITY`
alone moves nothing (measured: baked default warmth 99.20 against the same
binary's 124.65 under the env row).

Running the same fit against RHO = 0.0100 asks the honest question instead: what
`Linear{20, end}` carries the air the grade was verified with, while KEEPING the
dead zone f8a1a8f shipped the ramp for? Same method, same depth span, new target.
"""
import math

START = 20.0
VISIBLE = list(range(26, 80, 2))      # the measured depth span of the set
TABLE = [16, 20, 26, 30, 40, 55, 75, 100, 150, 200, 260, 300, 320]


def expsq(d, rho):
    return 1.0 - math.exp(-((d * rho) ** 2))


def lin(d, end):
    return max(0.0, min(1.0, (d - START) / (end - START)))


for rho in (0.0072, 0.0100):
    best, best_err = None, 1e9
    for end in range(60, 340, 1):
        err = math.sqrt(sum((lin(d, end) - expsq(d, rho)) ** 2 for d in VISIBLE) / len(VISIBLE))
        if err < best_err:
            best, best_err = end, err
    print(f"RHO {rho:.4f}: best-fit end over d=26..78 = {best}  (rms {100 * best_err:.2f} pp)")

RHO = 0.0100
ends = [140, 150, 160, 180, 250]
print(f"\ntarget = ExpSq({RHO}), the curve the vista grade was verified on\n")
print(f"{'d':>5s} {'ExpSq%':>8s} " + " ".join(f"Lin{e}%".rjust(9) for e in ends))
for d in TABLE:
    print(f"{d:5d} {100 * expsq(d, RHO):8.2f} "
          + " ".join(f"{100 * lin(d, e):9.2f}" for e in ends))

print(f"\n{'end':>5s} {'rms 26..78':>12s} {'max|dev|':>10s} {'alpha@16':>9s} {'alpha@20':>9s}")
for e in ends:
    rms = math.sqrt(sum((lin(d, e) - expsq(d, RHO)) ** 2 for d in VISIBLE) / len(VISIBLE))
    mx = max(abs(lin(d, e) - expsq(d, RHO)) for d in VISIBLE)
    print(f"{e:5d} {100 * rms:11.2f}pp {100 * mx:9.2f}pp "
          f"{100 * lin(16, e):8.2f}% {100 * lin(20, e):8.2f}%")
print("\nThe dead zone is unconditional in Linear{20, end}: alpha@16 and alpha@20 are\n"
      "0.00% for every end, which is the whole property f8a1a8f shipped the ramp for\n"
      f"and the property ExpSq({RHO}) cannot have (it applies "
      f"{100 * expsq(16, RHO):.2f}% at 16 blocks).")
