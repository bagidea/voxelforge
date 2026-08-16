#!/usr/bin/env python3
"""Build the A1 dome before/after panels and PROVE each tile is the plate its
caption names.

The scar this guards (`caption-must-match-pixels`): a panel whose caption says
`d3_skygrad-off` while the tile holds some other frame is worse than no panel.
So after compositing, every tile is read back out of the finished sheet and
pixel-differenced against the source PNG it claims; a non-zero mean aborts.
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(r"E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge")
S = ROOT / "_poppy_dome" / "shots"

TILE_W = 900          # each tile is downscaled to this width
PAD = 16
CAP_H = 74            # caption strip under each tile
NOTE_H = 30           # footer strip: the env state the panel does NOT ship with
BG = (14, 12, 10)
FG = (232, 216, 184)
DIM = (140, 132, 120)
WARN = (226, 168, 96)


def font(sz, bold=False):
    for name in (("arialbd.ttf", "arial.ttf") if bold else ("arial.ttf",)):
        try:
            return ImageFont.truetype(name, sz)
        except OSError:
            pass
    return ImageFont.load_default()


F_TITLE = font(30, bold=True)
F_CAP = font(19, bold=True)
F_SUB = font(16)


def sheet(out_name, title, tiles, note=None):
    """tiles = [(plate_stem, caption, subcaption), ...] laid out in one row.

    `note` is drawn as a footer. It carries the env state the panel was shot
    under whenever that differs from what ships, so the panel alone -- not the
    provenance file next to it -- tells a reader what they are looking at.
    """
    imgs = [Image.open(S / f"{p}.png").convert("RGB") for p, _, _ in tiles]
    w, h = imgs[0].size
    th = round(TILE_W * h / w)
    imgs = [im.resize((TILE_W, th), Image.LANCZOS) for im in imgs]

    n = len(tiles)
    W = PAD + n * (TILE_W + PAD)
    H = PAD + 46 + th + CAP_H + (NOTE_H if note else 0) + PAD
    sh = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(sh)
    d.text((PAD, PAD), title, font=F_TITLE, fill=FG)

    boxes = []
    for i, (im, (_, cap, sub)) in enumerate(zip(imgs, tiles)):
        x = PAD + i * (TILE_W + PAD)
        y = PAD + 46
        sh.paste(im, (x, y))
        boxes.append((x, y))
        # Border drawn OUTSIDE the tile bounds on purpose: an inset outline
        # repaints the plate's own edge pixels and the readback check below --
        # which demands mean|d| == 0 exactly -- would fail on the frame it just
        # composited correctly.
        d.rectangle([x - 1, y - 1, x + TILE_W, y + th], outline=(58, 39, 22))
        d.text((x, y + th + 10), cap, font=F_CAP, fill=FG)
        d.text((x, y + th + 36), sub, font=F_SUB, fill=DIM)

    if note:
        d.text((PAD, PAD + 46 + th + CAP_H), note, font=F_CAP, fill=WARN)

    out = S / out_name
    sh.save(out)

    # ---- caption/pixel verification -------------------------------------
    back = Image.open(out).convert("RGB")
    for (x, y), im, (plate, cap, _) in zip(boxes, imgs, tiles):
        cut = np.asarray(back.crop((x, y, x + TILE_W, y + th)), dtype=np.float32)
        ref = np.asarray(im, dtype=np.float32)
        md = float(np.abs(cut - ref).mean())
        if md != 0.0:
            sys.exit(f"ABORT: tile captioned '{cap}' != {plate}.png (mean|d|={md})")
        print(f"  tile OK  {cap:<34} == {plate}.png  (mean|d|=0)")
    print(f"WROTE {out}")
    return out


# Captions carry ATMOS=off because `sky_grad_enabled()` is `!atmos_enabled() && ...`
# (look.rs:3072 "Default: atmosphere on, dome off"), so NEITHER plate below is the
# shipped default — with no env set the game renders d0_atmos-on instead. Writing
# "AFTER — dome on (default)" would have told a reader the right-hand frame is what
# the game looks like today, which is false.
sheet(
    "dome_before_after.png",
    "Voxelforge A1 — gradient sky dome, before / after  (one binary, one lever)",
    [
        ("d3_skygrad-off", "BEFORE — SKYGRAD=off  (ATMOS=off)",
         "dome disabled: flat ClearColor sky, straight edge, column-std 0.00005"),
        ("d2_gradient", "AFTER — SKYGRAD on  (ATMOS=off)",
         "dome renders: curved silhouette + vertical gradient, column-std 0.01025 (205x)"),
    ],
    note="BOTH plates shot with VOXELFORGE_LOOK_ATMOS=off. The dome is NOT the shipped "
         "default — unset env = atmosphere on, dome suppressed (plate d0_atmos-on).",
)
sheet(
    "dome_render_proof.png",
    "Voxelforge A1 — proof the dome reaches the fragment shader (not just the log)",
    [
        ("d3_skygrad-off", "control — dome OFF  (ATMOS=off)",
         "sky is the flat ClearColor navy"),
        ("d1_atmos-off", "probe — dome material forced to BLACK  (ATMOS=off)",
         "whole sky takes the dome's own base_color; only lit geometry survives"),
        ("d0_atmos-on", "gate — probe set, atmosphere ON  (ATMOS unset = shipped default)",
         "dome correctly suppressed; probe has no effect, as designed"),
    ],
    note="Left two plates need VOXELFORGE_LOOK_ATMOS=off to see the dome at all; the right "
         "plate is the unset-env default the game ships with today.",
)
