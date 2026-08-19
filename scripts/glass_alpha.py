# -*- coding: utf-8 -*-
"""Give assets/textures/blocks/glass.png a real alpha channel.

RGB is Monanisa's and is not touched — only the alpha byte is written.

The alpha is DERIVED from the tile's own luminance, because the tile already
encodes its three parts in exactly those bands:

  * the dark ~1-texel came/leading around the pane -> nearly opaque (245)
  * the bright diagonal specular streak            -> mostly opaque (190)
  * everything between, i.e. the pane itself       -> see-through (46)

Derived rather than hand-painted so the mask stays in register with the art: if
the artist redraws the leading or moves the glint, re-running this moves the
alpha with it. A hand-painted mask is a second copy of the drawing, and the two
drift the first time anyone touches one of them.

Idempotent: re-running on an already-processed file produces the same bytes,
because the classification reads RGB only.

Usage (from the repo root):  python scripts/glass_alpha.py
"""
import sys

from PIL import Image

SRC = "assets/textures/blocks/glass.png"

# Luminance band edges, in the same sRGB-byte space the tile was authored in.
DARK_BELOW = 150.0
BRIGHT_FROM = 210.0

# Alpha per band.
LEAD, GLINT, PANE = 245, 190, 46


def luma(p):
    return 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]


def main():
    im = Image.open(SRC).convert("RGBA")
    px = im.load()
    before = [p[:3] for p in im.getdata()]

    counts = dict(lead=0, glint=0, pane=0)
    for y in range(im.height):
        for x in range(im.width):
            r, g, b, _ = px[x, y]
            L = luma((r, g, b))
            if L < DARK_BELOW:
                a, band = LEAD, "lead"
            elif L >= BRIGHT_FROM:
                a, band = GLINT, "glint"
            else:
                a, band = PANE, "pane"
            counts[band] += 1
            px[x, y] = (r, g, b, a)

    im.save(SRC)

    # Verify what actually landed on disk, not what we think we wrote.
    chk = Image.open(SRC).convert("RGBA")
    after = list(chk.getdata())
    if [p[:3] for p in after] != before:
        print("FAIL: RGB changed", file=sys.stderr)
        return 1
    alphas = [p[3] for p in after]
    print("texels %s" % counts)
    print(
        "alpha min/max/mean = %d/%d/%.1f"
        % (min(alphas), max(alphas), sum(alphas) / float(len(alphas)))
    )
    print("rgb preserved: True")
    return 0


if __name__ == "__main__":
    sys.exit(main())
