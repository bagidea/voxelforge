#!/usr/bin/env python3
"""Flamingo — the Pile A decision sheet: four frames, one image, one table.

`label=path` pairs, laid out 2-up per row, with every axis printed against REF
underneath. Same rule as every other tool in this set: the instrument has to pass
`control` before a single pixel is drawn.

USAGE  _fl_v4_quad_20260818.py --out S.png "A=a.png" "B=b.png" ...
"""
import argparse
import os
import sys

from PIL import Image, ImageDraw

os.environ.setdefault("LOKY_MAX_CPU_COUNT", "4")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
os.chdir(ROOT)

import importlib  # noqa: E402
_g = importlib.import_module("_fl_v4_grade_20260818")

AXES = [
    ("lum_mean", "lum mean", 1.0, "{:7.1f}"),
    ("shadow", "shadow %", 100.0, "{:7.1f}"),
    ("highlight", "highlight %", 100.0, "{:7.1f}"),
    ("edge_mean", "edge", 1.0, "{:7.2f}"),
    ("hue90", "hue spread", 1.0, "{:7.0f}"),
    ("occupied", "hue bins/36", 1.0, "{:7.0f}"),
    ("sat_mean", "sat mean", 1.0, "{:7.3f}"),
    ("sat_std", "sat std", 1.0, "{:7.3f}"),
    ("warm_pct", "warm %", 1.0, "{:7.1f}"),
    ("cool_pct", "cool %", 1.0, "{:7.1f}"),
]
PAD, GAP, COLS = 16, 10, 2


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--out", required=True)
    p.add_argument("--title", default="")
    p.add_argument("pairs", nargs="+")
    a = p.parse_args()

    if not _g.control():
        sys.exit(2)
    print()

    items = []
    for s in a.pairs:
        label, _, path = s.partition("=")
        if not os.path.isfile(path):
            print(f"SKIP missing {path}")
            continue
        items.append((label, path, _g.art_measure(label, path)))
    ref = _g.art_measure("REF", _g.CONTROL_ART["REF_mid"]["path"])

    w, h = Image.open(items[0][1]).size
    scale = 0.62
    tw, th = int(w * scale), int(h * scale)
    rows = (len(items) + COLS - 1) // COLS
    tab_h = 24 + 16 * (len(AXES) + 2)
    W = PAD * 2 + tw * COLS + GAP * (COLS - 1)
    H = PAD * 2 + 22 + rows * (th + 20) + tab_h
    im = Image.new("RGB", (W, H), (18, 18, 22))
    d = ImageDraw.Draw(im)
    d.text((PAD, PAD), a.title, fill=(255, 200, 140))

    y0 = PAD + 22
    for i, (label, path, _m) in enumerate(items):
        r, c = divmod(i, COLS)
        x = PAD + c * (tw + GAP)
        y = y0 + r * (th + 20)
        im.paste(Image.open(path).convert("RGB").resize((tw, th), Image.LANCZOS), (x, y))
        d.text((x + 2, y + th + 3), f"[{i+1}] {label}", fill=(220, 220, 230))

    y = y0 + rows * (th + 20) + 8
    head = f"{'axis':<14}" + "".join(f"{'['+str(i+1)+']':>9}" for i in range(len(items))) + f"{'REF':>9}"
    d.text((PAD, y), head, fill=(240, 240, 245)); y += 17
    for k, lab, sc, fmt in AXES:
        vals = [float(m[k]) * sc for _l, _p, m in items]
        vr = float(ref[k]) * sc
        line = f"{lab:<14}" + "".join(fmt.format(v) + "  " for v in vals) + fmt.format(vr)
        # colour the row by whether the LAST frame is closer to REF than the FIRST
        col = (200, 200, 210)
        if len(vals) >= 2:
            col = (140, 220, 150) if abs(vr - vals[-1]) < abs(vr - vals[0]) else (240, 150, 140)
        d.text((PAD, y), line, fill=col); y += 16
    im.save(a.out)
    print("wrote", a.out)


if __name__ == "__main__":
    main()
