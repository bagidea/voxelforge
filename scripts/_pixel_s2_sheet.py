#!/usr/bin/env python
"""Flamingo/pixel - before/after contact sheet for the INTERIOR s2 bake.

Every caption is derived from the plate's OWN run log (the `PILEA ...` line the
shot binary prints), never typed by hand, so a caption cannot drift away from
the frame it sits under. Scar: captions that say something the image doesn't.
"""
import sys
from PIL import Image, ImageDraw, ImageFont

ROOT = "E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge"
OUT = ROOT + "/_pixel_hero"

# (tag, headline) - headline says WHAT the frame is; the numbers come from the log
PANELS = [
    ("after_baked", "BEFORE - PILE A baked default"),
    ("r1", "round 1 winner - r1 (env only)"),
    ("s2", "AFTER - s2, now the baked default"),
]


def recipe_line(tag):
    """Pull the binary's own `PILEA ...` echo out of the (UTF-16) run log."""
    raw = open(f"{OUT}/{tag}.log", "rb").read()
    txt = raw.decode("utf-16", errors="replace")
    for ln in txt.splitlines():
        if "PILEA" in ln:
            return ln.strip()
    return "(no PILEA line in log)"


def short(line):
    """Two compact caption rows out of the PILEA echo."""
    kv = {}
    for part in line.replace("PILEA ", "").split():
        if "=" in part:
            k, v = part.split("=", 1)
            kv.setdefault(k, v)
    amb = line.split("amb=")[1].split("bounce2=")[0].strip() if "amb=" in line else "?"
    rim = line.split("rim=")[1].split("panehi=")[0].strip() if "rim=" in line else "?"
    r1 = f"ev100 {kv.get('ev100','?')}   shoulder {kv.get('shoulder','?')}   grade {kv.get('grade','?')}"
    r2 = f"amb {amb}   rim {rim}"
    return r1, r2


def font(sz, bold=False):
    for p in ("C:/Windows/Fonts/segoeuib.ttf" if bold else "C:/Windows/Fonts/segoeui.ttf",
              "C:/Windows/Fonts/arialbd.ttf" if bold else "C:/Windows/Fonts/arial.ttf"):
        try:
            return ImageFont.truetype(p, sz)
        except OSError:
            continue
    return ImageFont.load_default()


W = 760                      # per-panel width
PAD, CAPH, TOPH = 18, 96, 84
imgs = []
for tag, head in PANELS:
    im = Image.open(f"{OUT}/{tag}.png").convert("RGB")
    im = im.resize((W, round(W * im.height / im.width)), Image.LANCZOS)
    imgs.append((tag, head, im))

ih = imgs[0][2].height
sheet_w = PAD + (W + PAD) * len(imgs)
sheet_h = TOPH + PAD + ih + CAPH + PAD
sheet = Image.new("RGB", (sheet_w, sheet_h), (17, 18, 22))
d = ImageDraw.Draw(sheet)

d.text((PAD, 20), "Voxelforge interior look - PILE A -> s2, baked into hero.rs mod recipe",
       font=font(28, True), fill=(238, 240, 245))
d.text((PAD, 54), "all three frames: same binary, same camera 7.6,5.9,-5.2 -> 7.6,3.2,6.0 fov 52, 1280x720."
                  "  captions read from each frame's own run log.",
       font=font(17), fill=(150, 156, 170))

x = PAD
for tag, head, im in imgs:
    sheet.paste(im, (x, TOPH + PAD))
    cy = TOPH + PAD + ih + 10
    d.text((x, cy), head, font=font(21, True), fill=(240, 214, 150))
    a, b = short(recipe_line(tag))
    d.text((x, cy + 30), a, font=font(16), fill=(190, 196, 208))
    d.text((x, cy + 52), b, font=font(16), fill=(190, 196, 208))
    d.text((x, cy + 74), f"_pixel_hero/{tag}.png", font=font(14), fill=(120, 126, 140))
    x += W + PAD

path = sys.argv[1] if len(sys.argv) > 1 else f"{OUT}/s2_before_after_sheet.png"
sheet.save(path)
print("wrote", path, sheet.size)
for tag, _, _ in imgs:
    print(f"  {tag}: {recipe_line(tag)[:170]}")
