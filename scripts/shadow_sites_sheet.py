#!/usr/bin/env python3
"""Draw the site sets from shadow_edge_sites.py onto their plates, all locks at once.

The 2026-08-08 sheet showed ONE site set (locked on N5) on three plates, and the
reader had no way to see that the before plate's own edge is elsewhere in frame
-- the dots simply sat on lit wall and the caption said "no edge here". Every
lock's sites are drawn on every plate here, in its own colour, so a moved edge
is visible as three parallel dotted lines instead of one line and two absences.

Usage: shadow_sites_sheet.py <sites.json> <out.png> [scale]
"""
import json
import os
import sys

from PIL import Image, ImageDraw

COLOURS = {0: (60, 230, 200), 1: (255, 140, 60), 2: (170, 130, 255)}
BAR = 22


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    d = json.load(open(argv[0]))
    scale = int(argv[2]) if len(argv) > 2 else 2
    x0, y0, x1, y1 = d["roi"]
    keys = list(d["plates"])
    cw, ch = (x1 - x0) * scale, (y1 - y0) * scale
    sheet = Image.new("RGB", (cw, (ch + BAR) * len(keys) + BAR), (16, 16, 20))
    dr = ImageDraw.Draw(sheet)

    for i, k in enumerate(keys):
        crop = Image.open(d["plates"][k]).convert("RGB").crop((x0, y0, x1, y1))
        top = i * (ch + BAR)
        sheet.paste(crop.resize((cw, ch), Image.NEAREST), (0, top + BAR))
        pop = d["population"][k]
        dr.text((6, top + 6), f"{k}  --  ROI population n={pop['n']} "
                f"median={pop['median']:.2f}px = {pop['ratio']:.2f}x control",
                fill=(235, 235, 235))
        for j, lk in enumerate(keys):
            c = COLOURS[j % 3]
            for (sx, sy) in d["locks"][lk]["sites"]:
                if not (x0 <= sx < x1 and y0 <= sy < y1):
                    continue
                px, py = (sx - x0) * scale, top + BAR + (sy - y0) * scale
                r = 4
                dr.ellipse([px - r, py - r, px + r, py + r], outline=c, width=2)
    # legend in the strip under the last panel
    for j, lk in enumerate(keys):
        dr.rectangle([6 + j * 150, sheet.height - 16, 18 + j * 150, sheet.height - 4],
                     outline=COLOURS[j % 3], width=2)
        dr.text((24 + j * 150, sheet.height - 17), f"locked on {lk}",
                fill=COLOURS[j % 3])
    sheet.save(argv[1])
    print(f"written: {argv[1]}  {sheet.size[0]}x{sheet.size[1]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
