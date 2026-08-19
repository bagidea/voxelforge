#!/usr/bin/env python3
"""Flamingo — one before/after PNG for any pair, with the numbers ON the image.

Same rules as `_fl_v4_sheet_20260818.py` (which does the three play scenes by
name); this one takes explicit paths so it also covers the hero/Pile A pair.

  * refuses to draw anything until the instrument passes `control`
  * every axis is printed with the REF (golden target) value beside it, and the
    verdict is "toward REF" / "AWAY from REF" / "OVERSHOT past REF" — the third
    one exists because an axis can move the right direction and still end up
    further from the target than it started, and a two-way verdict would call
    that a win.
  * a null pair (same env, same gen, shot twice) sets the noise floor; a move
    inside it reads "within noise".

USAGE
  _fl_v4_pairsheet_20260818.py --before A.png --after B.png --label L --out S.png
                               [--null N.png]
"""
import argparse
import os
import sys

import numpy as np
from PIL import Image, ImageDraw

os.environ.setdefault("LOKY_MAX_CPU_COUNT", "4")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
os.chdir(ROOT)

import importlib  # noqa: E402
_g = importlib.import_module("_fl_v4_grade_20260818")

AXES = [
    ("lum_mean", "lum mean", 1.0, "{:.1f}"),
    ("shadow", "shadow %", 100.0, "{:.1f}"),
    ("midtone", "midtone %", 100.0, "{:.1f}"),
    ("highlight", "highlight %", 100.0, "{:.1f}"),
    ("edge_mean", "edge density", 1.0, "{:.2f}"),
    ("strong", "strong-edge %", 100.0, "{:.1f}"),
    ("hue90", "hue spread deg", 1.0, "{:.0f}"),
    ("occupied", "hue bins /36", 1.0, "{:.0f}"),
    ("sat_mean", "sat mean", 1.0, "{:.3f}"),
    ("sat_std", "sat std", 1.0, "{:.3f}"),
    ("warm_pct", "warm %", 1.0, "{:.1f}"),
    ("cool_pct", "cool %", 1.0, "{:.1f}"),
]
PAD, GAP = 16, 10


def rms(a_path, b_path):
    a = np.asarray(Image.open(a_path).convert("RGB"), dtype=np.float64)
    b = np.asarray(Image.open(b_path).convert("RGB"), dtype=np.float64)
    if a.shape != b.shape:
        return float("nan")
    return float(np.sqrt(((a - b) ** 2).mean()))


def verdict(vb, va, vr, floor):
    """before, after, ref, noise floor -> (text, colour)."""
    if abs(va - vb) <= floor:
        return f"within noise (null {floor:.3g})", (150, 150, 155)
    gb, ga = abs(vr - vb), abs(vr - va)
    if ga < gb:
        # closer to REF than it was. Overshoot only matters if it crossed.
        crossed = (vb - vr) * (va - vr) < 0
        return ("toward REF (crossed it)" if crossed else "toward REF"), (140, 220, 150)
    if (vb - vr) * (va - vr) < 0:
        return "OVERSHOT past REF", (250, 175, 90)
    return "AWAY from REF", (240, 130, 120)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--before", required=True)
    p.add_argument("--after", required=True)
    p.add_argument("--null", default=None)
    p.add_argument("--label", required=True)
    p.add_argument("--out", required=True)
    a = p.parse_args()

    if not _g.control():
        print("refusing to draw a sheet on an uncalibrated instrument.")
        sys.exit(2)
    print()

    mb = _g.art_measure(a.label + " before", a.before)
    ma = _g.art_measure(a.label + " after", a.after)
    ref = _g.art_measure("REF_mid", _g.CONTROL_ART["REF_mid"]["path"])

    floors, null_rms = {}, None
    if a.null and os.path.isfile(a.null):
        mn = _g.art_measure("null", a.null)
        for k, _l, _s, _f in AXES:
            floors[k] = abs(float(ma[k]) - float(mn[k]))
        null_rms = rms(a.after, a.null)
        print(f"NULL FLOOR  pixel RMS {null_rms:.3f}   " +
              ", ".join(f"{k}={floors[k]:.4g}" for k, _l, _s, _f in AXES))

    ib, ia = Image.open(a.before).convert("RGB"), Image.open(a.after).convert("RGB")
    w, h = ib.size
    strip = 22 + 16 * (len(AXES) + 1) + 30
    im = Image.new("RGB", (PAD * 2 + w * 2 + GAP, PAD * 2 + 24 + h + strip), (18, 18, 22))
    d = ImageDraw.Draw(im)
    im.paste(ib, (PAD, PAD + 24))
    im.paste(ia, (PAD + w + GAP, PAD + 24))
    d.text((PAD, PAD + 6), f"{a.label}  —  BEFORE", fill=(210, 210, 220))
    d.text((PAD + w + GAP, PAD + 6), f"{a.label}  —  AFTER", fill=(255, 190, 120))

    y = PAD + 24 + h + 12
    d.text((PAD, y), f"{'axis':<17}{'before':>10}{'after':>10}{'delta':>10}{'REF':>10}   verdict",
           fill=(235, 235, 240))
    y += 17
    for k, label, scale, fmt in AXES:
        vb, va, vr = float(mb[k]) * scale, float(ma[k]) * scale, float(ref[k]) * scale
        txt, col = verdict(vb, va, vr, floors.get(k, 0.0) * scale)
        d.text((PAD, y),
               f"{label:<17}{fmt.format(vb):>10}{fmt.format(va):>10}{va-vb:>+10.2f}{fmt.format(vr):>10}   {txt}",
               fill=col)
        y += 16
    y += 8
    tail = f"pixel delta before->after: RMS {rms(a.before, a.after):.2f}"
    if null_rms is not None:
        tail += f"   |   capture noise floor (null pair): RMS {null_rms:.2f}"
    d.text((PAD, y), tail, fill=(200, 200, 210))
    im.save(a.out)
    print(f"wrote {a.out}")


if __name__ == "__main__":
    main()
