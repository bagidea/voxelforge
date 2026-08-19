#!/usr/bin/env python3
"""Poppy -- compose the 2026-08-18 before/after board.

One PNG the CEO can read without opening four files: each scene is a row,
before on the left, after on the right, with that plate's own three numbers
burned under it and the REF value printed in the header so the gap is readable
without a second document.

Numbers are READ FROM `_poppy_ab_20260818_metrics.json`, never retyped -- a
caption typed by hand is a caption that can drift off the pixels it sits under.
The file each panel was built from is printed in its own label, so any panel can
be traced back to the plate on disk.
"""
import os
import sys
import json

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(ROOT, "docs", "look-ab-poppy-2026-08-18")
METRICS = os.path.join(ROOT, "_poppy_ab_20260818_metrics.json")

PANEL_W = 720          # each panel is scaled to this width
PAD = 18
LABEL_H = 86
HEADER_H = 104
BG = (22, 22, 26)
FG = (238, 238, 242)
DIM = (150, 150, 160)
WARN = (255, 140, 90)
GOOD = (140, 220, 150)


def font(size, bold=False):
    names = (["arialbd.ttf", "seguisb.ttf"] if bold else ["arial.ttf", "segoeui.ttf"])
    for n in names + ["DejaVuSans.ttf"]:
        try:
            return ImageFont.truetype(n, size)
        except OSError:
            continue
    return ImageFont.load_default()


F_H1 = font(30, True)
F_H2 = font(19)
F_LBL = font(21, True)
F_NUM = font(17)


def panel(path, title, m, note_color):
    """One labelled panel: the plate, scaled, with its numbers under it."""
    im = Image.open(path).convert("RGB")
    h = int(im.height * (PANEL_W / im.width))
    im = im.resize((PANEL_W, h), Image.LANCZOS)

    card = Image.new("RGB", (PANEL_W, h + LABEL_H), BG)
    card.paste(im, (0, 0))
    d = ImageDraw.Draw(card)
    d.text((6, h + 6), title, font=F_LBL, fill=note_color)
    d.text((6, h + 32),
           "edge %.1f    hue %.0f deg    cool %.2f%%"
           % (m["edge_mean"], m["hue_mean"], m["cool_pct_all"]),
           font=F_NUM, fill=FG)
    d.text((6, h + 56), os.path.basename(m["path"]), font=F_NUM, fill=DIM)
    return card


def main():
    if not os.path.isfile(METRICS):
        print("no metrics json at %s -- run the measure script first" % METRICS)
        return 2
    with open(METRICS) as f:
        M = json.load(f)

    rows = [("outdoor-noon", "before_noon", "after_noon"),
            ("village-raking", "before_vill", "after_vill")]
    for _scene, a, b in rows:
        for k in (a, b):
            if k not in M:
                print("metrics json has no '%s' -- re-run the measure script" % k)
                return 2

    cards = []
    for scene, a, b in rows:
        ca = panel(M[a]["path"], "BEFORE  %s" % scene, M[a], WARN)
        cb = panel(M[b]["path"], "AFTER  %s" % scene, M[b], GOOD)
        cards.append((ca, cb))

    w = PAD + 2 * PANEL_W + PAD + PAD
    h = HEADER_H + sum(c[0].height + PAD for c in cards) + PAD
    board = Image.new("RGB", (w, h), BG)
    d = ImageDraw.Draw(board)

    d.text((PAD, 16), "Voxelforge look A/B -- 2026-08-18 (poppy)", font=F_H1, fill=FG)
    ref = M.get("REF")
    sub = ("BEFORE target-flamingo voxelforge.exe (Aug 8)   vs   "
           "AFTER target voxelforge.exe (Aug 18 00:18)")
    d.text((PAD, 54), sub, font=F_H2, fill=DIM)
    if ref:
        d.text((PAD, 78),
               "REF (art-gap 2026-08-17, middle panel):  edge %.1f    hue %.0f deg    cool %.2f%%"
               % (ref["edge_mean"], ref["hue_mean"], ref["cool_pct_all"]),
               font=F_H2, fill=(210, 200, 160))

    y = HEADER_H
    for ca, cb in cards:
        board.paste(ca, (PAD, y))
        board.paste(cb, (PAD + PANEL_W + PAD, y))
        y += ca.height + PAD

    out = os.path.join(OUT_DIR, "board_before_after.png")
    board.save(out)
    print("WROTE %s  (%dx%d)" % (out, board.width, board.height))
    return 0


if __name__ == "__main__":
    sys.exit(main())
