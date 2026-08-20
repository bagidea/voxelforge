#!/usr/bin/env python3
"""Flamingo - build the hue-gap BEFORE/AFTER contact panel, and PROVE each tile
is the file its caption names.

The scar this guards: a panel whose caption says "after" over a tile that is
actually the before frame is worse than no panel, because it is evidence-shaped.
So every tile is pasted at NATIVE size (no resample, nothing that could make a
mismatch look like a resize artefact) and then read back out of the composed
canvas and compared to the source array. Any tile that is not byte-identical
aborts the whole panel - it will not write a half-true picture.

Usage:
    python scripts/_flamingo_huegap_panel.py OUT.png LABEL=path.png [LABEL=path.png ...]
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

BAR = 34          # caption bar height, px
PAD = 10          # gutter between tiles
BG = (18, 18, 20)
FG = (236, 236, 240)


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    out = Path(sys.argv[1])
    items = []
    for a in sys.argv[2:]:
        if "=" not in a:
            print(f"REFUSED  '{a}' is not LABEL=path")
            return 2
        lab, p = a.split("=", 1)
        p = Path(p)
        if not p.is_file():
            print(f"REFUSED  {p} does not exist - refusing to caption a missing tile")
            return 2
        items.append((lab, p))

    imgs = [(lab, p, np.asarray(Image.open(p).convert("RGB"))) for lab, p in items]
    w = max(a.shape[1] for _, _, a in imgs)
    h_total = sum(a.shape[0] + BAR + PAD for _, _, a in imgs) + PAD
    canvas = Image.new("RGB", (w + 2 * PAD, h_total), BG)
    dr = ImageDraw.Draw(canvas)

    placed = []
    y = PAD
    for lab, p, a in imgs:
        dr.text((PAD + 4, y + 9), f"{lab}   [{p.name}]", fill=FG)
        y += BAR
        canvas.paste(Image.fromarray(a), (PAD, y))
        placed.append((lab, p, a, PAD, y))
        y += a.shape[0] + PAD

    # ---- the check: read the tiles back OUT of the finished canvas ----------
    back = np.asarray(canvas.convert("RGB"))
    for lab, p, a, x0, y0 in placed:
        got = back[y0:y0 + a.shape[0], x0:x0 + a.shape[1]]
        if got.shape != a.shape or not np.array_equal(got, a):
            d = (float(np.abs(got.astype(int) - a.astype(int)).mean())
                 if got.shape == a.shape else float("nan"))
            print(f"REFUSED  tile '{lab}' does not match {p} (mean|d| = {d}) - panel not written")
            return 1
        print(f"ok   {lab:<28} == {p}   (mean|d| = 0.0)")

    out.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(out)
    print(f"\npanel {out}  {canvas.size[0]}x{canvas.size[1]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
