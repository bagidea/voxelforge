#!/usr/bin/env python3
"""Strip the gameplay HUD band before grading.

`grade_gate.py`'s G5 looks for the brightest pixel in the frame. On a `--play`
capture that pixel is the white HUD text at (15,12), not a window — so G5 reads
"blown out" on every gameplay frame regardless of the look. Same for the white
crosshair at frame centre and the red/green stat bars.

This crops the top HUD band and the centre crosshair out, writing `<name>-nohud.png`,
so the numeric gates measure the RENDER and not the UI. It does not touch colour.
"""
import sys
from PIL import Image

HUD_BAND = 80   # px: FPS line + HP/ST bars live in the top-left band
CROSS = 14      # px half-size of the crosshair patch at frame centre

for p in sys.argv[1:]:
    im = Image.open(p).convert("RGB")
    w, h = im.size
    # drop the HUD band off the top entirely
    im = im.crop((0, HUD_BAND, w, h))
    w, h = im.size
    # paint the crosshair out with the median of a ring around it
    cx, cy = w // 2, h // 2
    ring = im.crop((cx - CROSS * 3, cy - CROSS * 3, cx + CROSS * 3, cy + CROSS * 3))
    px = list(ring.getdata())
    med = tuple(sorted(c[i] for c in px)[len(px) // 2] for i in range(3))
    im.paste(med, (cx - CROSS, cy - CROSS, cx + CROSS, cy + CROSS))
    out = p.rsplit(".", 1)[0] + "-nohud.png"
    im.save(out)
    print(out, im.size)
