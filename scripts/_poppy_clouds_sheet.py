#!/usr/bin/env python3
"""Poppy -- A3 cloud deck: the before/after contact sheet.

One row per scene: BEFORE (VOXELFORGE_CLOUDS=off) on the left, AFTER on the
right, captioned with the scene, the lever state, and the pair's own
mean|dRGB| measured HERE from the two files being pasted -- not copied from the
measure run.

That last point is the whole reason this script re-measures instead of taking a
number on the command line: a caption that names a file must be computed from
that file, or the sheet can drift from the plates it claims to show (which has
happened on this project before, on a re-shoot that overwrote the plates under a
finished review).

Usage:
  _poppy_clouds_sheet.py <dir> <out.png> <scene> [<scene> ...]
"""
import os
import sys

import numpy as np
from PIL import Image, ImageDraw

PAD = 8
BAR = 34


def font():
    from PIL import ImageFont
    for p in (r"C:\Windows\Fonts\consola.ttf", r"C:\Windows\Fonts\arial.ttf"):
        if os.path.exists(p):
            return ImageFont.truetype(p, 15)
    return ImageFont.load_default()


def main(argv):
    if len(argv) < 4:
        print(__doc__)
        return 2
    root, out, scenes = argv[1], argv[2], argv[3:]
    f = font()

    rows = []
    for scene in scenes:
        b = os.path.join(root, f"before_{scene}.png")
        a = os.path.join(root, f"after_{scene}.png")
        if not (os.path.exists(b) and os.path.exists(a)):
            print(f"skip {scene}: missing plate")
            continue
        bi, ai = Image.open(b).convert("RGB"), Image.open(a).convert("RGB")
        if bi.size != ai.size:
            print(f"skip {scene}: size mismatch {bi.size} vs {ai.size}")
            continue
        d = np.abs(np.asarray(bi, np.float64) - np.asarray(ai, np.float64)).mean()
        rows.append((scene, bi, ai, d))

    if not rows:
        print("nothing to sheet")
        return 1

    w, h = rows[0][1].size
    sheet_w = w * 2 + PAD * 3
    sheet_h = sum(h + BAR for _ in rows) + PAD * (len(rows) + 1)
    sheet = Image.new("RGB", (sheet_w, sheet_h), (16, 16, 18))
    dr = ImageDraw.Draw(sheet)

    y = PAD
    for scene, bi, ai, d in rows:
        dr.text((PAD, y + 8), f"{scene}   BEFORE  VOXELFORGE_CLOUDS=off",
                (210, 210, 215), font=f)
        dr.text((PAD * 2 + w, y + 8),
                f"{scene}   AFTER  clouds on   mean|dRGB| {d:.3f}",
                (255, 214, 150), font=f)
        y += BAR
        sheet.paste(bi, (PAD, y))
        sheet.paste(ai, (PAD * 2 + w, y))
        y += h + PAD

    sheet.save(out)
    print(f"wrote {out}  {sheet.size[0]}x{sheet.size[1]}  rows={len(rows)}")
    for scene, _, _, d in rows:
        print(f"  {scene:<16} mean|dRGB| {d:.3f}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
