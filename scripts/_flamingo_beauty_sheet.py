#!/usr/bin/env python3
"""Before/after sheet for the beauty ladder — same frame, one binary, one knob set.

Every panel is captioned with the numbers measured off THAT panel's own file, by
the same code path that produced the grid.  Office scar, verbatim: "pixel-diff
every A/B panel against the file its caption names".  This script therefore
re-reads each file at composite time instead of accepting numbers passed in, so a
caption cannot drift from the pixels above it.

Usage:
    python scripts/_flamingo_beauty_sheet.py OUT.png --pair BEFORE.png AFTER.png
        [--pair ...] [--title "..."] [--after-label "..."]
"""
import argparse
import importlib.util
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "beauty_axes", HERE / "_flamingo_beauty_axes.py")
axes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(axes)

PANEL_W = 900
PAD = 18
CAP_H = 108
TITLE_H = 82


def font(sz, bold=False):
    for name in (("seguisb.ttf", "segoeuib.ttf") if bold else ("segoeui.ttf",)):
        try:
            return ImageFont.truetype(name, sz)
        except OSError:
            pass
    try:
        return ImageFont.truetype("arialbd.ttf" if bold else "arial.ttf", sz)
    except OSError:
        return ImageFont.load_default()


def numbers(path):
    r = axes.grade(path)
    st = r["structure"]
    sky = r["sky"] or {}
    return (f"value span {st['L_p95_minus_p5']:.1f}   "
            f"shadow p5 {st['L_p'][0]:.1f} ({100 * st['L_p'][0] / 255:.1f}% of white)   "
            f"chroma span {st['sat_p95_minus_p5']:.3f}",
            f"sky {r['sky_px_pct']:.2f}% of frame   "
            f"sky clipped {sky.get('clip_pct', float('nan')):.1f}%   "
            f"horizon step {(r.get('horizon') or {}).get('drgb', float('nan')):.1f}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--pair", nargs=2, action="append", required=True,
                    metavar=("BEFORE", "AFTER"))
    ap.add_argument("--title", default="Beauty axes — before / after")
    ap.add_argument("--before-label", default="BEFORE  (shipped look, no overrides)")
    ap.add_argument("--after-label", default="AFTER")
    a = ap.parse_args()

    probe = Image.open(a.pair[0][0])
    ph = int(PANEL_W * probe.height / probe.width)
    row_h = ph + CAP_H
    W = PAD + 2 * (PANEL_W + PAD)
    H = TITLE_H + len(a.pair) * (row_h + PAD)
    sheet = Image.new("RGB", (W, H), (16, 16, 19))
    d = ImageDraw.Draw(sheet)
    f_title, f_lab, f_num = font(30, True), font(21, True), font(17)

    d.text((PAD, 24), a.title, font=f_title, fill=(240, 238, 234))

    for i, (b, af) in enumerate(a.pair):
        y = TITLE_H + i * (row_h + PAD)
        for j, (path, lab) in enumerate(((b, a.before_label), (af, a.after_label))):
            x = PAD + j * (PANEL_W + PAD)
            im = Image.open(path).convert("RGB").resize((PANEL_W, ph), Image.LANCZOS)
            sheet.paste(im, (x, y))
            d.rectangle([x, y, x + PANEL_W - 1, y + ph - 1], outline=(70, 70, 78))
            d.text((x, y + ph + 8), f"{lab}", font=f_lab,
                   fill=(255, 190, 120) if j else (150, 190, 255))
            d.text((x, y + ph + 34), Path(path).name, font=f_num, fill=(150, 150, 158))
            n1, n2 = numbers(path)
            d.text((x, y + ph + 56), n1, font=f_num, fill=(225, 225, 230))
            d.text((x, y + ph + 78), n2, font=f_num, fill=(190, 190, 198))

    Path(a.out).parent.mkdir(parents=True, exist_ok=True)
    sheet.save(a.out)
    print(f"sheet -> {a.out}  ({W}x{H}, {len(a.pair)} pairs)")


if __name__ == "__main__":
    main()
