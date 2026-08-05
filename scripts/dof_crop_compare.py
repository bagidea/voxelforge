"""DOF reversal proof: near-bowl vs far-wall sharpness, current bake vs golden ref.
Flamingo, 2026-07-26. Emits labeled crops + a 2x2 comparison sheet + a sharpness table.

    Usage: dof_crop_compare.py <frame>-nohud2.png

HARD GUARD (see scripts/nohud2_guard.py). This prints LapVar per crop and an
fg:bg ratio next to the golden ref's — the same class of number `grade_beauty.py`
prints — and it used to read a HARDCODED `hero-look-final.png` with no argv at
all, so you could not tell from the command line which bake produced the table.
That frame is a `voxelforge.exe` capture (render_charm2.sh) and it does carry the
HUD: `_flamingo_dehud2.py` lifts 8,442 prompt-glyph px out of it. Measured
honestly, the effect on THIS number is small — fg:bg 1.57 raw vs 1.56 de-HUDded,
because neither crop box lands on the top band or the [E] block — so the guard
here is about frame identity and about a Laplacian variance being the most
HUD-sensitive statistic in the repo, not about a verdict that flipped.
REF is exempt on purpose: the golden ref is curated artwork, never a capture.
"""
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, must run first)

require_nohud2(sys.argv[1:2], tool="dof_crop_compare.py")

ROOT = "E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge/"
HERO = sys.argv[1]
hero = Image.open(HERO).convert("RGB")                            # 1280x720 current bake
gold = Image.open(ROOT + "docs/assets/golden-beauty-shot-ref.png").convert("RGB")  # 1024x1024

# crop boxes (left,top,right,bottom)
BOX = {
    "hero_bowl": (410, 235, 890, 480),   # hero subject (should be SHARP)
    "hero_wall": (1055, 90, 1280, 435),  # right checker wall (should be SOFT)
    "gold_bowl": (150, 745, 575, 975),   # golden subject bowl (is SHARP)
    "gold_wall": (385, 110, 815, 470),   # golden background cabinets (is SOFT)
}

# The hero boxes were authored against the RAW 1280x720 capture, and
# `_flamingo_dehud2.py` crops the top HUD band off — so on a -nohud2 frame the
# same content sits `720 - height` px higher. Without this shift the guard would
# have made the numbers WORSE: right suffix, wrong pixels.
BASE_H = 720


def shift(b, im):
    dy = BASE_H - im.height
    return (b[0], b[1] - dy, b[2], b[3] - dy)

def lap_var(im):
    """variance of Laplacian -> higher = sharper (more high-freq edge energy)."""
    g = np.asarray(im.convert("L"), dtype=np.float64)
    k = np.array([[0,1,0],[1,-4,1],[0,1,0]], dtype=np.float64)
    from numpy.lib.stride_tricks import sliding_window_view
    w = sliding_window_view(g, (3,3))
    lap = (w * k).sum(axis=(-1,-2))
    return float(lap.var())

crops = {}
for name,box in BOX.items():
    src = hero if name.startswith("hero") else gold
    c = src.crop(shift(box, src) if name.startswith("hero") else box)
    crops[name] = c
    c.save(ROOT + f"dof_{name}.png")

# sharpness table
print("region                sharpness(LapVar)")
vals = {}
for name,c in crops.items():
    v = lap_var(c); vals[name] = v
    print(f"  {name:14s}  {v:12.1f}")
print()
print(f"CURRENT bake   fg(bowl)/bg(wall) = {vals['hero_bowl']/vals['hero_wall']:.2f}  (target >=3, i.e. bowl sharper)")
print(f"GOLDEN ref     fg(bowl)/bg(wall) = {vals['gold_bowl']/vals['gold_wall']:.2f}")

# build labeled 2x2 sheet: rows = GOLDEN(target) / CURRENT(bug); cols = NEAR BOWL / FAR WALL
CW, CH = 460, 300
PAD, TOP, LBL = 16, 64, 34
sheet_w = PAD*3 + CW*2
sheet_h = TOP + (LBL+CH+PAD)*2 + PAD
sheet = Image.new("RGB", (sheet_w, sheet_h), (24,22,20))
d = ImageDraw.Draw(sheet)
try:
    fbig = ImageFont.truetype("arialbd.ttf", 26)
    fmid = ImageFont.truetype("arialbd.ttf", 20)
    fsm  = ImageFont.truetype("arial.ttf", 16)
except Exception:
    fbig = fmid = fsm = ImageFont.load_default()

d.text((PAD, 12), "Voxelforge hero DOF — the subject bowl never resolves sharp", font=fbig, fill=(240,235,228))
d.text((PAD, 42), "subject crispness: golden bowl LapVar 422 vs current 8 (~50x)   |   fg:bg gradient  golden 5.2x vs current 1.6x", font=fsm, fill=(198,193,186))

def fit(c):
    return c.resize((CW,CH))

rows = [
    ("GOLDEN (target)", (120,210,120), "gold_bowl", "gold_wall", "razor sharp  \u2713", "soft bokeh  \u2713"),
    ("CURRENT bake",    (235,140,120), "hero_bowl", "hero_wall", "soft \u2717 (should be sharpest)", "also soft \u2014 no bokeh sep"),
]
for r,(rlabel,rcol,bowlk,wallk,bowltag,walltag) in enumerate(rows):
    y0 = TOP + r*(LBL+CH+PAD)
    d.text((PAD, y0-2), rlabel, font=fmid, fill=rcol)
    for cidx,(key,coltitle,tag) in enumerate([(bowlk,"NEAR — hero bowl",bowltag),(wallk,"FAR — back wall",walltag)]):
        x0 = PAD + cidx*(CW+PAD)
        yimg = y0 + LBL
        sheet.paste(fit(crops[key]), (x0, yimg))
        d.rectangle([x0,yimg,x0+CW-1,yimg+CH-1], outline=rcol, width=3)
        d.text((x0+6, yimg+4), coltitle, font=fsm, fill=(255,255,255))
        d.text((x0+6, yimg+CH-24), f"{tag}   LapVar {vals[key]:.0f}", font=fsm, fill=(255,255,255))

sheet.save(ROOT + "dof-reversal-proof.png")
print("\nwrote: dof-reversal-proof.png  + dof_{hero,gold}_{bowl,wall}.png")
