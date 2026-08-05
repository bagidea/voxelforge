"""Flamingo — one sheet the reviewer can judge the grade change from.

Three frames stacked with their OWN measured axes printed under each, so the
picture and the numbers are never separated. Everything on the sheet is derived
at render time from `scripts/grade_axes.py`'s TARGETS and from the frames' real
measurements — the scar from `gate3-colour-verdict.png` is that a card which
hard-codes its verdict keeps shouting the old one after the bug is fixed.

Usage: python scripts/grade_compare_sheet.py OUT.png LABEL=frame.png [LABEL=... ]
"""

import os
import subprocess
import sys
import re

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from grade_axes import TARGETS  # noqa: E402  (the ONLY authority on targets)

W = 900          # per-frame width on the sheet
PAD = 18
TEXT_H = 128


def measure(frame: str) -> dict:
    out = subprocess.run(
        [sys.executable, os.path.join(HERE, "grade_axes.py"), frame],
        capture_output=True, text=True,
    ).stdout
    vals = {}
    for key, label, *_ in TARGETS:
        m = re.search(re.escape(label) + r"\s+([\d.]+)", out)
        if m:
            vals[key] = float(m.group(1))
    return vals


def verdict(key, value) -> bool:
    """PASS/FAIL derived from grade_axes.TARGETS, never written down here."""
    for k, _label, cmp_, bound, _ref in TARGETS:
        if k != key:
            continue
        if cmp_ == "ge":
            return value >= bound
        if cmp_ == "le":
            return value <= bound
        if cmp_ == "band":
            return bound[0] <= value <= bound[1]
    return False


def font(sz):
    for p in ("C:/Windows/Fonts/consola.ttf", "C:/Windows/Fonts/arial.ttf"):
        if os.path.exists(p):
            return ImageFont.truetype(p, sz)
    return ImageFont.load_default()


def main() -> None:
    out_path = sys.argv[1]
    items = [a.split("=", 1) for a in sys.argv[2:]]
    rows = [(lbl, f, measure(f)) for lbl, f in items]

    thumbs = []
    for _lbl, f, _v in rows:
        im = Image.open(f).convert("RGB")
        thumbs.append(im.resize((W, round(W * im.height / im.width)), Image.LANCZOS))

    sheet_h = sum(t.height + TEXT_H + PAD for t in thumbs) + PAD
    sheet = Image.new("RGB", (W + 2 * PAD, sheet_h), (16, 16, 20))
    d = ImageDraw.Draw(sheet)
    f_big, f_small = font(26), font(20)

    y = PAD
    for (lbl, path, vals), th in zip(rows, thumbs):
        sheet.paste(th, (PAD, y))
        y += th.height + 6
        d.text((PAD, y), lbl, font=f_big, fill=(235, 235, 240))
        y += 32
        d.text((PAD, y), os.path.basename(path), font=f_small, fill=(120, 120, 130))
        y += 26
        x = PAD
        for key, label, cmp_, bound, _ref in TARGETS:
            if key not in vals:
                continue
            ok = verdict(key, vals[key])
            txt = f"{label.split('(')[0].strip()} {vals[key]:.1f}"
            d.text((x, y), txt, font=f_small,
                   fill=(120, 220, 140) if ok else (240, 120, 110))
            x += 12 + d.textlength(txt, font=f_small)
        y += 34 + PAD

    sheet.save(out_path)
    print(f"-> {out_path}  ({sheet.width}x{sheet.height})")


if __name__ == "__main__":
    main()
