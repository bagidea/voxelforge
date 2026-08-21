#!/usr/bin/env python3
"""BEFORE/AFTER close-up of the avatar itself, at 1:1 pixels.

The film answers "does it blend"; this answers the blunter question the review
actually asked -- "is the AFTER body a human being with proportions, or is it
still a shape?" A 1280x720 gameplay frame renders a 1.8 m avatar about a fifth
of the frame tall, which is enough to see a stride and not enough to see a
forearm, so a reviewer squinting at the full frame is entitled to say the pixels
do not carry the claim.

    python scripts/_poppy_herofilm_bodycrop.py _poppy_hero 60 90 120

The crop box is NOT hunted for per-frame: the camera boom keeps the avatar's
pivot centred every frame, so a fixed centre box is the honest choice -- it
cannot be nudged toward whichever crop flatters the rig, and it lands in the
same place on the BEFORE plate as on the AFTER one.
"""
import sys
import pathlib
from PIL import Image, ImageDraw, ImageFont

# Fraction of the frame kept, around the centre. The boom holds the body at the
# middle of the frame; this is wide enough to keep an arm swing and a jump apex
# inside the box at LOOK_CAM dist 5.5.
CROP_W = 0.34
CROP_H = 0.66
ZOOM = 3
PAD = 10
CAP_H = 26


def font(size):
    for name in ("DejaVuSans.ttf", "arial.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def crop(path):
    im = Image.open(path).convert("RGB")
    w, h = im.size
    cw, ch = int(w * CROP_W), int(h * CROP_H)
    x, y = (w - cw) // 2, (h - ch) // 2
    return im.crop((x, y, x + cw, y + ch)).resize((cw * ZOOM, ch * ZOOM), Image.NEAREST)


def main():
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "_poppy_hero")
    idxs = [int(a) for a in sys.argv[2:]] or [60]
    cols = []
    for i in idxs:
        for row, d in (("BEFORE (capsule)", "film_before"), ("AFTER (rig)", "film_after")):
            p = out / d / f"f{i:05d}.png"
            if not p.exists():
                print(f"CROP FAIL: missing {p}")
                return 2
        cols.append(i)

    tiles = {r: [crop(out / d / f"f{i:05d}.png") for i in cols]
             for r, d in (("BEFORE (capsule)", "film_before"), ("AFTER (rig)", "film_after"))}
    tw, th = tiles["AFTER (rig)"][0].size

    W = PAD + len(cols) * (tw + PAD)
    H = CAP_H + 2 * (th + CAP_H + PAD)
    sheet = Image.new("RGB", (W, H), (16, 16, 18))
    dr = ImageDraw.Draw(sheet)
    f_cap = font(16)

    for c, i in enumerate(cols):
        dr.text((PAD + c * (tw + PAD), 5), f"frame f{i:05d}  (t={0.30 + i / 30.0:.2f}s)",
                font=f_cap, fill=(235, 235, 225))

    y = CAP_H
    for row in ("BEFORE (capsule)", "AFTER (rig)"):
        for c, im in enumerate(tiles[row]):
            sheet.paste(im, (PAD + c * (tw + PAD), y))
        y += th
        dr.text((PAD, y + 5), f"{row}   VOXELFORGE_ANIM_RIG_OFF="
                              f"{'1' if row.startswith('BEFORE') else '(unset)'}"
                              f"   crop {CROP_W:.0%}x{CROP_H:.0%} centre, {ZOOM}x nearest",
                font=f_cap, fill=(255, 210, 130))
        y += CAP_H + PAD

    dst = out / "hero-rig-bodycrop.png"
    sheet.save(dst)
    print(f"CROP OK {dst} {sheet.size[0]}x{sheet.size[1]} frames={cols}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
