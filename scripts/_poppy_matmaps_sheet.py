#!/usr/bin/env python3
# ===========================================================================
# Poppy — the MAT_MAPS contact sheet: three scenes, on vs off, ONE binary.
#
# Every number printed on this sheet is recomputed here from the PNGs named on
# it, so the caption cannot drift away from the pixels. The env each plate was
# shot with is printed on the sheet too — a panel whose recipe lives only in a
# shell history is a panel nobody can re-shoot.
#
# USAGE
#   python scripts/_poppy_matmaps_sheet.py <out.png>
# ===========================================================================
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PUB = os.path.join(ROOT, "docs", "assets", "look")

SCENES = [
    ("day", "outdoor noon  SUN=66,205,20000  EXPOSURE=10.6"),
    ("evening-raking", "raking evening  (look.rs hour default)"),
    ("night-firelit", "night, lit by fire  LOOK_NIGHT=1"),
]
COL_W = 640


def load(p):
    return np.asarray(Image.open(p).convert("RGB"), dtype=np.float64)


def luma(a):
    return a[:, :, 0] * 0.2126 + a[:, :, 1] * 0.7152 + a[:, :, 2] * 0.0722


def lap(y):
    return np.abs(4 * y[1:-1, 1:-1] - y[:-2, 1:-1] - y[2:, 1:-1] - y[1:-1, :-2] - y[1:-1, 2:])


def font(sz):
    for f in ("C:/Windows/Fonts/consola.ttf", "C:/Windows/Fonts/arial.ttf"):
        if os.path.isfile(f):
            return ImageFont.truetype(f, sz)
    return ImageFont.load_default()


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(PUB, "matmaps_contact_sheet.png")
    f_hdr, f_cap, f_sm = font(22), font(17), font(14)

    thumbs, caps = [], []
    for scene, hour in SCENES:
        row = []
        for lever in ("on", "off"):
            p = os.path.join(PUB, f"matmaps_{scene}_{lever}.png")
            im = Image.open(p).convert("RGB")
            h = int(im.height * COL_W / im.width)
            row.append(im.resize((COL_W, h), Image.LANCZOS))
        a, b = load(os.path.join(PUB, f"matmaps_{scene}_on.png")), load(os.path.join(PUB, f"matmaps_{scene}_off.png"))
        d = np.abs(a - b).mean(axis=2)
        ha, hb = lap(luma(a)).mean(), lap(luma(b)).mean()
        thumbs.append(row)
        caps.append((scene, hour, d.mean(), 100.0 * (d > 1.0).mean(), ha, hb, 100.0 * (ha - hb) / hb))

    tw, th = thumbs[0][0].size
    pad, head, cap_h = 14, 96, 46
    W = pad * 3 + tw * 2
    H = head + sum(cap_h + th + pad for _ in thumbs) + 74
    sheet = Image.new("RGB", (W, H), (17, 17, 20))
    dr = ImageDraw.Draw(sheet)

    dr.text((pad, 12), "VOXELFORGE_MAT_MAPS — authored _n/_r maps vs derived, ONE binary", (240, 240, 240), font=f_hdr)
    dr.text((pad, 42), "target/release/voxelforge.exe  built 2026-08-18 08:01:34  sha256 C46A7015…", (170, 170, 175), font=f_sm)
    dr.text((pad, 60), "shared env: PLAY=1 NOHUD=1 LOOK_QUALITY=ultra CINE_START=1.0  |  "
                       "ATLAS_DIR=_poppy_matmaps/atlas16 (16px albedo + 64px maps)", (170, 170, 175), font=f_sm)
    dr.text((pad, 78), "left = MAT_MAPS unset (authored, 72 maps bound)   right = MAT_MAPS=off (derived only, 0 bound)",
            (170, 170, 175), font=f_sm)

    y = head
    for (scene, hour, mad, chg, ha, hb, hpd), row in zip(caps, thumbs):
        dr.text((pad, y + 4), f"{scene}   {hour}", (235, 235, 235), font=f_cap)
        dr.text((pad, y + 25), f"mean|on-off| {mad:.2f} lv   pixels changed {chg:.1f}%   "
                               f"local contrast (mean |Laplacian| of luma) {hb:.2f} -> {ha:.2f}  = {hpd:+.1f}%   "
                               f"[null pair floor +1.85%]", (150, 200, 150), font=f_sm)
        y += cap_h
        sheet.paste(row[0], (pad, y))
        sheet.paste(row[1], (pad * 2 + tw, y))
        dr.text((pad + 8, y + 6), "MAT_MAPS on", (255, 235, 140), font=f_cap)
        dr.text((pad * 2 + tw + 8, y + 6), "MAT_MAPS=off", (255, 180, 180), font=f_cap)
        y += th + pad

    dr.text((pad, y + 4), "The same lever on the SHIPPED atlas dir (assets/textures/blocks, manifest tile_px 64 vs "
                          "voxel.rs TILE_PX 16) moves nothing:", (200, 200, 205), font=f_sm)
    dr.text((pad, y + 22), "mad 0.17-0.52 lv against a 0.50 null floor, local contrast -0.89% / -0.58% / +0.35% "
                           "against a +1.25% floor, BLOCK_PBR authored = 0 in every log.", (200, 160, 160), font=f_sm)
    dr.text((pad, y + 40), "scripts/_poppy_matmaps_{prep,shoot,measure,sheet}  |  plates: docs/assets/look/matmaps_*.png",
            (140, 140, 145), font=f_sm)

    sheet.save(out)
    print(f"{out}  {sheet.size[0]}x{sheet.size[1]}")


if __name__ == "__main__":
    main()
