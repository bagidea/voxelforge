#!/usr/bin/env python
"""Poppy — pair each look-v2 scene's v1/v2 plates into one BEFORE | AFTER sheet.

The CEO judges these by eye against a Minecraft shader mod, so the sheet exists
to make that judgement possible in one glance: same scene, same camera, side by
side, labelled, with a hairline between so nobody has to guess which half is
which. The printed numbers are not the verdict — they are there so a sheet that
LOOKS different can be checked against a frame that actually IS different.
"""
import sys
import os
from PIL import Image, ImageDraw

BAR = 44          # label strip height, px
GAP = 8           # hairline between the halves
BG = (16, 17, 20)
FG = (232, 234, 238)


def stats(im):
    """Mean / p05 / p95 luminance of a plate, on the 0-255 scale."""
    g = im.convert("L")
    px = sorted(g.getdata())
    n = len(px)
    return (sum(px) / n, px[int(n * 0.05)], px[int(n * 0.95)])


def sheet(out_dir, scene):
    a = os.path.join(out_dir, f"{scene}-v1.png")
    b = os.path.join(out_dir, f"{scene}-v2.png")
    if not (os.path.isfile(a) and os.path.isfile(b)):
        print(f"  {scene}: MISSING ({os.path.basename(a)} / {os.path.basename(b)})")
        return None

    ia, ib = Image.open(a).convert("RGB"), Image.open(b).convert("RGB")
    if ia.size != ib.size:
        # Never rescale one side to match: a resample would be a difference the
        # change did not make. Report and bail instead.
        print(f"  {scene}: SIZE MISMATCH {ia.size} vs {ib.size} - not sheeted")
        return None

    w, h = ia.size
    out = Image.new("RGB", (w * 2 + GAP, h + BAR), BG)
    out.paste(ia, (0, BAR))
    out.paste(ib, (w + GAP, BAR))

    d = ImageDraw.Draw(out)
    sa, sb = stats(ia), stats(ib)
    d.text((12, 14), f"BEFORE  v1 flat fill      mean {sa[0]:.1f}  p05 {sa[1]}  p95 {sa[2]}", fill=FG)
    d.text((w + GAP + 12, 14), f"AFTER  v2 sky+bounce      mean {sb[0]:.1f}  p05 {sb[1]}  p95 {sb[2]}", fill=FG)
    d.text((out.width - 90, 14), scene.upper(), fill=(120, 170, 255))

    path = os.path.join(out_dir, f"AB-{scene}.png")
    out.save(path)
    print(f"  {scene}: mean {sa[0]:.1f} -> {sb[0]:.1f} | p05 {sa[1]} -> {sb[1]} | p95 {sa[2]} -> {sb[2]}")
    print(f"    {path}")
    return path


def stack(out_dir, sheets):
    """All the pair-sheets in one file, one scene per row.

    The per-scene sheets stay the deliverable — this is for the single glance
    that decides whether the change reads at all. Rows are left-aligned rather
    than centred so the BEFORE halves line up in one column down the image and
    the eye can run the seam.
    """
    if len(sheets) < 2:
        return None
    ims = [Image.open(p).convert("RGB") for p in sheets]
    w = max(i.width for i in ims)
    out = Image.new("RGB", (w, sum(i.height for i in ims)), BG)
    y = 0
    for i in ims:
        out.paste(i, (0, y))
        y += i.height
    path = os.path.join(out_dir, "AB-all.png")
    out.save(path)
    return path


if __name__ == "__main__":
    out_dir = sys.argv[1]
    made = [p for p in (sheet(out_dir, s) for s in sys.argv[2:]) if p]
    combined = stack(out_dir, made)
    print()
    for p in made:
        print(p)
    if combined:
        print(combined)
