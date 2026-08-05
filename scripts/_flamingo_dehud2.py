#!/usr/bin/env python3
"""Strip the FULL gameplay HUD before grading — stricter successor to _flamingo_dehud.py.

Why a second one: the original only cropped the top band + the crosshair. A `--play`
capture of the current build also burns in the **interaction prompt block** near the
bottom centre ("[E] Read Scorched handprint" / "[E] Rest at campfire"). Those glyphs are
near-white (~250) and sit ON the ground plane, so they:

  * become the brightest pixel of the frame  -> G5 reads "blown out" on every frame
  * push the global luminance p95            -> the P0 highlight axis reads the UI
  * add hard 1px edges on a flat surface     -> inflates the micro-contrast axis

Top band + crosshair are removed the same way as before (crop / patch). The prompt block
cannot be cropped without eating the foreground the DOF box needs, so its glyph pixels
are detected and inpainted with the local row median of the surrounding ground.

Detection is deliberately conservative-in-favour-of-the-gate: anything in the prompt band
that is far brighter than its own row median is treated as UI and removed. Removing a few
genuine highlight pixels can only make a frame score LOWER, never higher — the gate gets
stricter, never looser.

Usage: python scripts/_flamingo_dehud2.py <frame.png> ...   ->  <frame>-nohud2.png
"""
import sys

import numpy as np
from PIL import Image

HUD_BAND = 80      # px: FPS line + HP/ST bars live in the top-left band
CROSS = 14         # px half-size of the crosshair patch at frame centre
PROMPT_TOP = 0.72  # fraction of frame height where the [E] prompt block starts
PROMPT_BOT = 0.92  # ... and ends
BRIGHT_OVER = 28   # L above the row median => treat as UI glyph
DILATE = 2         # px grow of the glyph mask, to catch antialiased edges


def _dilate(mask: np.ndarray, r: int) -> np.ndarray:
    out = mask.copy()
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            out |= np.roll(np.roll(mask, dy, axis=0), dx, axis=1)
    return out


def dehud(path: str) -> str:
    im = Image.open(path).convert("RGB")
    w, h = im.size

    # 1. drop the top HUD band entirely
    im = im.crop((0, HUD_BAND, w, h))
    w, h = im.size

    # 2. paint out the crosshair with the median of a ring around it
    cx, cy = w // 2, h // 2
    ring = im.crop((cx - CROSS * 3, cy - CROSS * 3, cx + CROSS * 3, cy + CROSS * 3))
    px = list(ring.getdata())
    med = tuple(sorted(c[i] for c in px)[len(px) // 2] for i in range(3))
    im.paste(med, (cx - CROSS, cy - CROSS, cx + CROSS, cy + CROSS))

    # 3. inpaint the [E] prompt glyphs in the bottom band
    a = np.asarray(im, dtype=np.float64)
    lum = 0.2126 * a[:, :, 0] + 0.7152 * a[:, :, 1] + 0.0722 * a[:, :, 2]
    y0, y1 = int(h * PROMPT_TOP), int(h * PROMPT_BOT)

    band = lum[y0:y1]
    row_med = np.median(band, axis=1, keepdims=True)
    glyph = band > (row_med + BRIGHT_OVER)
    glyph = _dilate(glyph, DILATE)

    n = int(glyph.sum())
    if n:
        sub = a[y0:y1].copy()
        for c in range(3):
            ch = sub[:, :, c]
            # per-row median of the NON-glyph pixels = the ground under the text
            for r in range(ch.shape[0]):
                keep = ch[r][~glyph[r]]
                if keep.size:
                    ch[r][glyph[r]] = np.median(keep)
        a[y0:y1] = sub

    out_im = Image.fromarray(a.round().clip(0, 255).astype(np.uint8), "RGB")
    out = path.rsplit(".", 1)[0] + "-nohud2.png"
    out_im.save(out)
    print(f"{out}  {out_im.size}  prompt-glyph px removed={n}")
    return out


if __name__ == "__main__":
    for p in sys.argv[1:]:
        dehud(p)
