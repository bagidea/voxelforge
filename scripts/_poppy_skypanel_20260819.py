#!/usr/bin/env python3
"""Poppy - the sky before/after contact panel for 2026-08-19.

Builds ONE image from the two plates the numbers were measured on, and prints
each source's md5 next to the caption it is drawn under, so the caption can be
checked against the pixels instead of trusted (rubric: caption must match
pixels). Nothing here measures anything - every number in the strip is read
back out of `_poppy_sky/artgap_ba.json`, the grader's own output.
"""
from __future__ import annotations

import hashlib
import json
import os

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SKY = os.path.join(ROOT, "_poppy_sky")
OUT = os.path.join(SKY, "sky_ba_panel.png")

PAIR = [("before.png", "BEFORE  (ATMOS=lut, no dome)"),
        ("after.png", "AFTER  (painted dome + clouds + sun)")]

# key -> (label, ref, direction) - the five renderer-owned sky axes.
AXES = [("sky_void_pct", "sky at black %", 0.00, -1),
        ("sky_ground_ratio", "sky/ground L", 1.80, +1),
        ("sky_L_range", "sky gradient", 172.58, +1),
        ("sky_hue_span_deg", "sky hue span", 60.00, +1),
        ("sky_blown_pct", "sky blown % [guard]", 5.09, -1)]


def md5(p: str) -> str:
    with open(p, "rb") as fh:
        return hashlib.md5(fh.read()).hexdigest()


def frame(data: dict, stem: str) -> dict:
    for f in data["frames"]:
        if f["file"].replace("\\", "/").endswith(stem):
            return f
    raise SystemExit(f"{stem} not in artgap json")


def font(sz: int):
    for name in ("arialbd.ttf", "arial.ttf", "DejaVuSans-Bold.ttf"):
        try:
            return ImageFont.truetype(name, sz)
        except OSError:
            continue
    return ImageFont.load_default()


def main() -> int:
    data = json.load(open(os.path.join(SKY, "artgap_ba.json"), encoding="utf-8"))
    imgs = [Image.open(os.path.join(SKY, n)).convert("RGB") for n, _ in PAIR]
    w, h = imgs[0].size
    if imgs[1].size != (w, h):
        raise SystemExit("plates differ in size - not a comparable pair")

    pad, head, strip = 12, 46, 150
    canvas = Image.new("RGB", (w * 2 + pad * 3, head + h + strip + pad * 2), (18, 18, 22))
    d = ImageDraw.Draw(canvas)
    f_head, f_row = font(24), font(19)

    for i, (name, cap) in enumerate(PAIR):
        x = pad + i * (w + pad)
        canvas.paste(imgs[i], (x, head))
        d.text((x + 6, 12), cap, font=f_head, fill=(240, 240, 245))
        d.text((x + 6, head + h + 6), f"{name}  md5 {md5(os.path.join(SKY, name))[:16]}",
               font=f_row, fill=(150, 150, 158))

    fb, fa = frame(data, "before.png"), frame(data, "after.png")
    y = head + h + 34
    d.text((pad + 6, y), f"{'axis':<22}{'REF':>9}{'before':>11}{'after':>11}   verdict",
           font=f_row, fill=(200, 200, 210))
    y += 24
    for key, label, ref, direction in AXES:
        b, a = fb.get(key), fa.get(key)
        rt = ((ref + 0.5) / (a + 0.5)) if direction < 0 else (a / ref if ref else 0)
        ok = rt >= 0.60
        col = (120, 220, 140) if ok else (235, 130, 120)
        d.text((pad + 6, y),
               f"{label:<22}{ref:9.2f}{b:11.2f}{a:11.2f}   "
               f"{'PASS' if ok else 'GAP '}  {rt:.2f}x",
               font=f_row, fill=col)
        y += 22

    canvas.save(OUT)
    print("wrote", OUT, canvas.size)
    for name, _ in PAIR:
        print(f"  src {name:<12} md5 {md5(os.path.join(SKY, name))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
