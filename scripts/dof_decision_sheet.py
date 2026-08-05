"""DOF decision sheet — current(stale) vs fixed(f4.5) vs golden, full frame + bowl-crop.
Flamingo, 2026-07-26.

    Usage: dof_decision_sheet.py <current>-nohud2.png <fixed>-nohud2.png

HARD GUARD (see scripts/nohud2_guard.py) — same reason as dof_crop_compare.py:
it stamps an fg:bg ratio and a bowl LapVar onto a sheet a human uses to CHOOSE a
bake, and both capture filenames used to be hardcoded in COLS. The golden ref
(3rd column) is curated artwork, never a capture, so it stays fixed and unguarded.
"""
import os
import sys

import numpy as np
from numpy.lib.stride_tricks import sliding_window_view
from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, must run first)

require_nohud2(sys.argv[1:3], tool="dof_decision_sheet.py")
if len(sys.argv) < 3:
    print("REFUSED  dof_decision_sheet.py: need TWO frames — "
          "<current>-nohud2.png <fixed>-nohud2.png", file=sys.stderr)
    sys.exit(2)
CUR, FIXED = sys.argv[1], sys.argv[2]

ROOT = "E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge/"
def lv(im):
    g=np.asarray(im.convert('L'),dtype=np.float64)
    k=np.array([[0,1,0],[1,-4,1],[0,1,0]],dtype=np.float64)
    w=sliding_window_view(g,(3,3)); return float((w*k).sum(axis=(-1,-2)).var())

# (file, title, subtitle, bowlbox, wallbox)
COLS = [
    (CUR, "CURRENT (shipped)", "env DOF=11,3.2  focus BEHIND bowl",
     (410,235,890,480),(1055,90,1280,435)),
    (FIXED, "FIXED (recommend)", "baked focus 10 + f/4.5",
     (410,235,890,480),(1055,90,1280,435)),
    (ROOT + "docs/assets/golden-beauty-shot-ref.png", "GOLDEN (target)", "painted-texture reference",
     (150,745,575,975),(385,110,815,470)),
]
FW, FH = 300, 169     # full-frame thumb
BW, BH = 300, 150     # bowl crop
PAD, TOP, CAP = 14, 78, 22
sw = PAD*4 + FW*3
sh = TOP + CAP + FH + 26 + BH + CAP + PAD
sheet = Image.new("RGB",(sw,sh),(24,22,20)); d=ImageDraw.Draw(sheet)
try:
    fb=ImageFont.truetype("arialbd.ttf",24); fm=ImageFont.truetype("arialbd.ttf",17); fs=ImageFont.truetype("arial.ttf",14)
except Exception: fb=fm=fs=ImageFont.load_default()

d.text((PAD,12),"Voxelforge hero — DOF fix (one env render, no recompile)",font=fb,fill=(240,235,228))
d.text((PAD,44),"CURRENT is baked with a STALE env override (focus 11 = behind the bowl). Source default is already focus 10; f/2.8->f/4.5 crisps the subject with bokeh intact.",font=fs,fill=(198,193,186))

# The capture boxes were authored against the RAW 1280x720 frame; _flamingo_dehud2.py
# crops the top HUD band, so on a -nohud2 frame the same content sits `720 - h` px
# higher. The golden ref (1024x1024, never a capture) is left alone.
BASE_H = 720
def shift(b,im,is_capture):
    if not is_capture: return b
    dy = BASE_H - im.height
    return (b[0], b[1]-dy, b[2], b[3]-dy)

for i,(f,title,sub,bb,wb) in enumerate(COLS):
    im=Image.open(f).convert('RGB')
    cap = 'GOLDEN' not in title
    bb, wb = shift(bb,im,cap), shift(wb,im,cap)
    b=lv(im.crop(bb)); w=lv(im.crop(wb)); ratio=b/w
    x0=PAD+i*(FW+PAD)
    col=(120,210,120) if 'GOLDEN' in title else ((150,215,255) if 'FIXED' in title else (235,140,120))
    d.text((x0,TOP-24),title,font=fm,fill=col)
    d.text((x0,TOP-4),sub,font=fs,fill=(180,176,170))
    y=TOP+CAP
    sheet.paste(im.resize((FW,FH)),(x0,y)); d.rectangle([x0,y,x0+FW-1,y+FH-1],outline=col,width=2)
    d.text((x0+4,y+FH+4),f"fg:bg ratio  {ratio:4.2f}   bowl LapVar {b:.0f}",font=fs,fill=(255,255,255))
    yb=y+FH+26
    sheet.paste(im.crop(bb).resize((BW,BH)),(x0,yb)); d.rectangle([x0,yb,x0+BW-1,yb+BH-1],outline=col,width=2)
    d.text((x0+4,yb+BH-20),"bowl crop (edge read)",font=fs,fill=(255,255,255))

note="Note: fg:bg>=3 is content-capped here — golden's bowl is PAINTED texture (LapVar 422); voxel faces are flat matte (~13 in focus). Optics max out ~2.6; reaching >=3 needs a foreground-texture pass (P1 framing), not an env tweak."
d.text((PAD,sh-CAP-2),note,font=fs,fill=(210,190,150))
sheet.save(ROOT+"dof-decision-sheet.png")
print("wrote dof-decision-sheet.png",(sw,sh))
