#!/usr/bin/env python3
"""Hero-vs-background hue delta, same method as
docs/VERDICT-a6-teal-accent-2026-08-10.md's repro: grade_character.py's own
analytic_bbox + mask_from_bbox for the character mask, a ring just outside
that bbox for background, colorsys.rgb_to_hsv on each mean RGB.

Usage: python scripts/_flamingo_hue_delta.py <frame.png> --cam Y,P,D [--actor player]
"""
import argparse
import colorsys
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, "scripts")
import grade_character as gc  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("image")
    ap.add_argument("--cam", required=True)
    ap.add_argument("--actor", default="player")
    ap.add_argument("--hud-crop", type=int, default=80)
    ap.add_argument("--key-tol", type=float, default=26.0)
    args = ap.parse_args()

    im = Image.open(args.image).convert("RGB")
    rgb = np.asarray(im).astype(np.float32)
    h, w, _ = rgb.shape

    yaw, pitch, dist = (float(x) for x in args.cam.split(","))
    bbox, foot_y = gc.analytic_bbox(w, h, yaw, pitch, dist, args.actor, args.hud_crop)
    mask = gc.mask_from_bbox(rgb, bbox, foot_y, args.key_tol, shadow_guard=True)

    x0, y0, x1, y1 = bbox
    bw, bh = x1 - x0, y1 - y0
    # ring: a band around the padded bbox, outside the character mask
    pad = 0.25
    X0 = int(max(0, x0 - bw * pad)); X1 = int(min(w, x1 + bw * pad))
    Y0 = int(max(0, y0 - bh * pad)); Y1 = int(min(h, y1 + bh * pad))
    X0r = int(max(0, x0 - bw * (pad * 2))); X1r = int(min(w, x1 + bw * (pad * 2)))
    Y0r = int(max(0, y0 - bh * (pad * 2))); Y1r = int(min(h, y1 + bh * (pad * 2)))

    ring = np.zeros((h, w), bool)
    ring[Y0r:Y1r, X0r:X1r] = True
    inner = np.zeros((h, w), bool)
    inner[Y0:Y1, X0:X1] = True
    ring &= ~inner
    ring &= ~mask

    hero_rgb = rgb[mask].mean(axis=0)
    bg_rgb = rgb[ring].mean(axis=0)

    hh, hs, hv = colorsys.rgb_to_hsv(*(hero_rgb / 255.0))
    bh_, bs, bv = colorsys.rgb_to_hsv(*(bg_rgb / 255.0))
    hue_hero = hh * 360
    hue_bg = bh_ * 360
    delta = abs(hue_hero - hue_bg)
    delta = min(delta, 360 - delta)

    print(f"{args.image}")
    print(f"  mask px={mask.sum()}  ring px={ring.sum()}")
    print(f"  hero  RGB ({hero_rgb[0]:.1f}, {hero_rgb[1]:.1f}, {hero_rgb[2]:.1f})   hue {hue_hero:.1f} deg")
    print(f"  bg    RGB ({bg_rgb[0]:.1f}, {bg_rgb[1]:.1f}, {bg_rgb[2]:.1f})   hue {hue_bg:.1f} deg")
    print(f"  hue delta: {delta:.1f} deg")


if __name__ == "__main__":
    main()
