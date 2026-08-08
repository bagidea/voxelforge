#!/usr/bin/env python3
"""Poppy — build the before/after comparison sheets for the magenta-wash fix.

For every angle shot by `scripts/_poppy_ba_shoot.sh` this writes a 2-up sheet
(before | after, labelled) and prints the numbers the sheet is claiming:

  * full-frame mean RGB and its channel ordering
  * sky-band median RGB and its ordering, for the angles that have open sky

Ordering is the whole story. The authored `ClearColor` is srgb(0.53,0.72,0.92)
-> B > G > R. The bug ran that through a chromatic-adaptation matrix whose
off-diagonal terms bleed G and B into R, which flips it to R > B > G — green
pushed under both neighbours, which is what the eye reads as magenta.
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
SHOTS = ROOT / "_poppy_ba" / "shots"

# Angles, in the order the sheet contact-print is laid out. `sky` is the band
# (y0, y1, x0, x1) that is open sky in that framing — None where the framing has
# no usable patch of it, in which case only the full-frame mean is reported.
ANGLES = [
    ("far", "STANDING / FAR - open arena, horizon + ~45% sky", (90, 300, 100, 1180)),
    ("boot", "STANDING / SPAWN - Edhari village", None),
    ("walk", "MID-STRIDE - camera orbited", None),
    ("combat", "COMBAT - closing on a husk, HUD up", None),
]

LABEL_H = 34
GAP = 8


def order(rgb: np.ndarray) -> str:
    """Channel ordering as a string, brightest first — 'B > G > R' etc."""
    names = "RGB"
    idx = np.argsort(-rgb)
    return " > ".join(names[i] for i in idx)


def stats(img: Image.Image, sky: tuple[int, int, int, int] | None):
    a = np.asarray(img.convert("RGB"), dtype=np.float64)
    full = a.reshape(-1, 3).mean(axis=0)
    out = {"full": full}
    if sky is not None:
        y0, y1, x0, x1 = sky
        band = a[y0:y1, x0:x1].reshape(-1, 3)
        # Median, not mean: the band can clip a silhouette (the husk stands in
        # it on the arena framing) and a median ignores that; a mean would drag
        # the numbers toward the figure and weaken the claim either way.
        out["sky"] = np.median(band, axis=0)
    return out


def label(draw: ImageDraw.ImageDraw, x: int, y: int, w: int, text: str, bg: tuple[int, int, int]):
    draw.rectangle([x, y, x + w, y + LABEL_H], fill=bg)
    draw.text((x + 10, y + 10), text, fill=(255, 255, 255))


def sheet(before: Image.Image, after: Image.Image, title: str, out: Path):
    w, h = before.size
    canvas = Image.new("RGB", (w * 2 + GAP, h + LABEL_H * 2 + GAP), (18, 18, 22))
    d = ImageDraw.Draw(canvas)
    label(d, 0, 0, w * 2 + GAP, title, (32, 32, 40))
    y = LABEL_H
    label(d, 0, y, w, "BEFORE - temp 0.10, white lights (pre-26b2ae6)", (120, 30, 80))
    label(d, w + GAP, y, w, "AFTER - temp 0.02, warmth on KEY/AMBIENT (shipped)", (28, 80, 60))
    canvas.paste(before, (0, y + LABEL_H + GAP))
    canvas.paste(after, (w + GAP, y + LABEL_H + GAP))
    canvas.save(out)
    return canvas.size


def main() -> int:
    outdir = ROOT / "docs" / "assets" / "magenta-fix"
    outdir.mkdir(parents=True, exist_ok=True)
    missing = 0
    for name, title, sky in ANGLES:
        b_path = SHOTS / "before" / f"{name}.png"
        a_path = SHOTS / "after" / f"{name}.png"
        if not (b_path.exists() and a_path.exists()):
            print(f"  ✗ {name}: missing shot(s)")
            missing += 1
            continue
        b, a = Image.open(b_path), Image.open(a_path)
        bs, as_ = stats(b, sky), stats(a, sky)
        # Anything holding pre-fix pixels carries the `REJECTED` keyword, which is
        # the exemption signal docs/assets/.colour-gate-allow already documents
        # ("the keyword in the filename is the signal"). These frames display the
        # magenta bug on purpose; without the keyword they would fail the colour
        # gate and read as a regression. It is a gate exemption, NOT a verdict on
        # the fix — the `-after` frames carry no keyword and are graded normally.
        out = outdir / f"magenta-{name}-REJECTED-before-vs-after.png"
        size = sheet(b, a, title, out)
        print(f"\n=== {name} — {title}")
        print(f"  sheet {out.relative_to(ROOT)}  {size[0]}x{size[1]}")
        for key in ("sky", "full"):
            if key not in bs:
                continue
            bv, av = bs[key], as_[key]
            print(
                f"  {key:5s} before {bv[0]:6.1f},{bv[1]:6.1f},{bv[2]:6.1f}  {order(bv):11s}"
                f"   ->  after {av[0]:6.1f},{av[1]:6.1f},{av[2]:6.1f}  {order(av):11s}"
            )
        # Also copy the raw frames next to the sheet so a reviewer can grade the
        # untouched PNGs rather than a composite.
        b.save(outdir / f"magenta-{name}-REJECTED-before.png")
        a.save(outdir / f"magenta-{name}-after.png")
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())
