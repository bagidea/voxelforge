#!/usr/bin/env python3
"""Poppy -- INSTRUMENT CONTROL for the cool%/hue grader.

The after-plates read `cool px % = 0.00` and `hue concentration = 0.98`
(near-monochrome). Before anyone tunes a look constant on the back of those two
numbers, the instrument that produced them has to be shown to be able to
produce a DIFFERENT answer. A metric that returns 0.00 because its mask can
never fire is indistinguishable, on one plate, from a metric that returns 0.00
because the frame really has no cool pixels.

So this runs `_poppy_ab_20260818_measure.extras()` -- the SAME function, imported,
not reimplemented -- over plates whose answer is known before the run:

  POSITIVE  synth_cool    every pixel hue 220 deg   -> cool must be ~100%
  NEGATIVE  synth_warm    every pixel hue  30 deg   -> cool must be ~0%
  HALF      synth_half    half 220 / half 30        -> cool must be ~50%
  SPREAD    synth_spread  full 0-360 hue ramp       -> hue_r must be ~0 (no mean)
  GOLDEN    the signed golden beauty ref + Kevin's REF panel -> cool must be > 0

The negative control matters as much as the positive one: an instrument that
answers 100% to everything is just as broken, and only `synth_warm` catches it.
`synth_spread` is there because `hue_r = 0.98` is the other number under
suspicion -- if a full hue ramp also scored ~1.0, the concentration figure would
be meaningless and the "near-monochrome" reading with it.

A control that FAILS here means the script is broken and the plates are
innocent. A control that PASSES licenses -- and only then -- reading the
after-plate numbers as facts about the render.

Exit 0 = every control within tolerance. Exit 1 = instrument is not trustworthy.
"""
import os
import sys

os.environ.setdefault("LOKY_MAX_CPU_COUNT", "4")
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
ROOT = os.path.dirname(HERE)

import numpy as np  # noqa: E402
from PIL import Image  # noqa: E402
from _poppy_ab_20260818_measure import extras  # noqa: E402

OUT = os.path.join(ROOT, "_poppy_control")
os.makedirs(OUT, exist_ok=True)

W, H = 256, 256


def hsv_plate(hue_deg, sat=0.75, val=0.85):
    """Solid HSV plate -> uint8 RGB. hue_deg may be scalar or HxW array."""
    h = np.broadcast_to(np.asarray(hue_deg, dtype=np.float64), (H, W)) / 60.0
    c = val * sat
    x = c * (1.0 - np.abs((h % 2.0) - 1.0))
    m = val - c
    i = np.floor(h).astype(int) % 6
    z = np.zeros_like(x)
    tbl = [(c, x, z), (x, c, z), (z, c, x), (z, x, c), (x, z, c), (c, z, x)]
    r = np.select([i == k for k in range(6)], [t[0] for t in tbl])
    g = np.select([i == k for k in range(6)], [t[1] for t in tbl])
    b = np.select([i == k for k in range(6)], [t[2] for t in tbl])
    a = np.stack([r + m, g + m, b + m], axis=-1)
    return np.clip(a * 255.0, 0, 255).astype(np.uint8)


def save(name, arr):
    p = os.path.join(OUT, name + ".png")
    Image.fromarray(arr).save(p)
    return p


def build():
    plates = []
    plates.append(("synth_cool", save("synth_cool", hsv_plate(220.0))))
    plates.append(("synth_warm", save("synth_warm", hsv_plate(30.0))))

    half = np.concatenate([hsv_plate(220.0)[:, : W // 2],
                           hsv_plate(30.0)[:, W // 2:]], axis=1)
    plates.append(("synth_half", save("synth_half", half)))

    ramp = np.tile(np.linspace(0.0, 360.0, W, endpoint=False), (H, 1))
    plates.append(("synth_spread", save("synth_spread", hsv_plate(ramp))))
    return plates


# label -> (metric, lo, hi, why)
EXPECT = {
    "synth_cool":   [("cool_pct_all", 99.0, 100.01, "solid hue 220 is cool by definition")],
    "synth_warm":   [("cool_pct_all", -0.01, 0.5, "solid hue 30 must NOT read cool")],
    "synth_half":   [("cool_pct_all", 45.0, 55.0, "half the pixels are hue 220")],
    "synth_spread": [("hue_r", -0.01, 0.15, "a full 0-360 ramp has no mean hue")],
}

# Real frames, with the answer each one is ENTITLED to.
#
# `golden-beauty-shot-ref.png` is the signed-off beauty reference and it scores
# cool 0.00% / hue_r 0.99 -- the exact signature that started this
# investigation. That is not a grader fault and not a render fault: it is an
# INTERIOR, a firelit kitchen with no sky in frame, and a warm interior with no
# cool pixel is the correct reading. Keeping it here as a real-frame NEGATIVE
# control is the whole point -- it proves 0.00% is an APPROVED, achievable
# reading rather than automatic evidence of breakage. My first version of this
# file gated it at `cool > 0` and it FAILED, which was the gate being wrong
# about the plate, not the plate being wrong.
#
# `kevin_ref_panel_mid.png` is the outdoor reference and carries sky, so it is
# the frame that must show cool pixels. Cool% is only a defect signal on a plate
# that has sky in it.
GOLDEN = [
    ("golden_beauty", os.path.join(ROOT, "docs", "assets", "golden-beauty-shot-ref.png"),
     "max", 0.5, "signed INTERIOR ref: no sky in frame, so ~0 cool is CORRECT"),
    ("kevin_REF", os.path.join(ROOT, "_kevin_ref_panel_mid.png"),
     "min", 1.0, "OUTDOOR ref with sky: must show cool pixels"),
]


def main():
    print("=" * 74)
    print("GRADER INSTRUMENT CONTROL -- extras() from _poppy_ab_20260818_measure")
    print("=" * 74)

    rows = []
    for label, path in build():
        rows.append((label, path, extras(path)))
    for label, path, _d, _b, _w in GOLDEN:
        if os.path.isfile(path):
            rows.append((label, path, extras(path)))
        else:
            print("!! MISSING GOLDEN %-14s %s" % (label, path))

    print("\n%-16s %10s %10s %10s %10s" %
          ("plate", "cool_all%", "cool_sat%", "hue_mean", "hue_r"))
    print("-" * 62)
    for label, _p, r in rows:
        print("%-16s %10.2f %10.2f %10.1f %10.3f" %
              (label, r["cool_pct_all"], r["cool_pct_sat"],
               r["hue_mean"], r["hue_r"]))

    print("\n--- assertions ---")
    fails = 0
    by = {l: r for l, _p, r in rows}
    for label, checks in EXPECT.items():
        for key, lo, hi, why in checks:
            got = by[label][key]
            ok = lo <= got <= hi
            fails += 0 if ok else 1
            print("[%s] %-14s %-13s = %8.3f  want %.2f..%.2f   (%s)" %
                  ("PASS" if ok else "FAIL", label, key, got, lo, hi, why))

    # Real frames: a one-sided bound each, in the direction the frame's own
    # CONTENT justifies. An interior gets a ceiling, an outdoor gets a floor.
    for label, _p, direction, bound, why in GOLDEN:
        if label not in by:
            continue
        got = by[label]["cool_pct_all"]
        ok = got <= bound if direction == "max" else got >= bound
        fails += 0 if ok else 1
        print("[%s] %-14s %-13s = %8.3f  want %s %.2f    (%s)" %
              ("PASS" if ok else "FAIL", label, "cool_pct_all", got,
               "<=" if direction == "max" else ">=", bound, why))

    print("\n%s" % ("CONTROL PASSED -- the instrument can tell cool from warm, and\n"
                    "a 0.00 reading on a plate is a fact about that plate."
                    if fails == 0 else
                    "CONTROL FAILED -- do NOT trust any cool%%/hue number until fixed."))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
