"""DOF decision sheet — current(stale) vs fixed(f4.5) vs golden, full frame + bowl-crop.
Flamingo, 2026-07-26."""
import numpy as np
from numpy.lib.stride_tricks import sliding_window_view
from PIL import Image, ImageDraw, ImageFont

ROOT = "E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge/"
def lv(im):
    g=np.asarray(im.convert('L'),dtype=np.float64)
    k=np.array([[0,1,0],[1,-4,1],[0,1,0]],dtype=np.float64)
    w=sliding_window_view(g,(3,3)); return float((w*k).sum(axis=(-1,-2)).var())

# (file, title, subtitle, bowlbox, wallbox)
COLS = [
    ("hero-look-final.png", "CURRENT (shipped)", "env DOF=11,3.2  focus BEHIND bowl",
     (410,235,890,480),(1055,90,1280,435)),
    ("dof-f45.png", "FIXED (recommend)", "baked focus 10 + f/4.5",
     (410,235,890,480),(1055,90,1280,435)),
    ("docs/assets/golden-beauty-shot-ref.png", "GOLDEN (target)", "painted-texture reference",
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

for i,(f,title,sub,bb,wb) in enumerate(COLS):
    im=Image.open(ROOT+f).convert('RGB')
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
