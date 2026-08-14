"""Confirmation pass over commit 6376138: prove each shipped capsule against Valve's current spec
AND against the master it claims to come from. Read-only on the art; writes one contact sheet.

Two questions the size table alone can't answer:
  a) is the file really the crop the generator says it is (i.e. nothing composited in since)?
  b) does it carry any real pixels the master doesn't have (a logo, a fresh composition)?
Both are answered by re-deriving the crop from the master and diffing.
"""
import os
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont

D = r"E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\docs\assets\steam"
OUT = os.path.join(D, "review")
os.makedirs(OUT, exist_ok=True)

land = Image.open(os.path.join(D, "key-art-landscape-master.png")).convert("RGB")
port = Image.open(os.path.join(D, "key-art-portrait-master.png")).convert("RGB")

# (file, spec_w, spec_h, source, kind, band) — bands copied from scripts/make_steam_capsules.py
BANDS = [
    ("header-capsule-920x430.png",   920,  430, land, "band", (140, 619)),
    ("small-capsule-462x174.png",    462,  174, land, "band", (187, 573)),
    ("main-capsule-1232x706.png",   1232,  706, land, "band", (130, 717)),
    ("vertical-capsule-748x896.png", 748,  896, port, "vert", None),
    ("page-background-1438x810.png",1438,  810, land, "band", (135, 712)),
    ("library-hero-3840x1240.png",  3840, 1240, land, "band", (330, 660)),
    ("library-capsule-600x900.png",  600,  900, port, "vert", None),
]
MISSING = [("library-header-920x430.png", 920, 430), ("library-logo-1280x720.png", 1280, 720)]


def rederive(src, kind, band, tw, th):
    if kind == "band":
        return src.crop((0, band[0], src.width, band[1])).resize((tw, th), Image.LANCZOS)
    bw = int(src.height * (tw / th))
    x0 = (src.width - bw) // 2
    return src.crop((x0, 0, x0 + bw, src.height)).resize((tw, th), Image.LANCZOS)


print("=" * 92)
print("DERIVATION PROOF — shipped file vs the crop re-derived from the master in this repo")
print("=" * 92)
rows = []
all_ok = True
for f, sw, sh, src, kind, band in BANDS:
    p = os.path.join(D, f)
    shipped = Image.open(p).convert("RGB")
    w, h = shipped.size
    ok = (w, h) == (sw, sh)
    ref = rederive(src, kind, band, sw, sh)
    a = np.asarray(shipped).astype(np.int16)
    b = np.asarray(ref).astype(np.int16)
    diff = np.abs(a - b)
    maxd, meand = int(diff.max()), float(diff.mean())
    changed = 100 * (diff.max(axis=-1) > 8).mean()
    verdict = "pure crop of master (nothing composited)" if changed < 0.05 else \
              f"{changed:.2f}% of px differ -> extra content present"
    print(f"  {f:32} {w}x{h} spec {sw}x{sh} {'PASS' if ok else 'FAIL':4} | "
          f"maxdiff={maxd:3} mean={meand:5.3f} -> {verdict}")
    rows.append((f, w, h, sw, sh, ok, changed))
    if not ok or changed >= 0.05:
        all_ok = False

print()
print("=" * 92)
print("CONTACT SHEET")
print("=" * 92)
BG = (27, 40, 56)
try:
    font = ImageFont.truetype("arial.ttf", 15)
    fontb = ImageFont.truetype("arialbd.ttf", 17)
except Exception:
    font = fontb = ImageFont.load_default()

TH = 150  # thumb height
pad, lab = 26, 46
thumbs = []
for f, w, h, sw, sh, ok, changed in rows:
    im = Image.open(os.path.join(D, f)).convert("RGB")
    tw = max(1, int(im.width * TH / im.height))
    thumbs.append((f, im.resize((min(tw, 420), TH), Image.LANCZOS), f"{w}x{sh and h}", ok))

W = sum(t[1].width for t in thumbs) + pad * (len(thumbs) + 1) + 2 * (260 + pad)
canvas = Image.new("RGB", (W, TH + pad * 2 + lab + 40), BG)
d = ImageDraw.Draw(canvas)
d.text((pad, 10), "Voxelforge — Steam capsule set @ commit 6376138 · measured from disk 2026-08-01",
       fill=(210, 220, 232), font=fontb)
x = pad
for (f, im, _, ok), (_, w, h, sw, sh, _, _) in zip(thumbs, rows):
    canvas.paste(im, (x, 48 + lab - 24))
    col = (120, 230, 150) if ok else (255, 110, 110)
    d.rectangle([x - 2, 48 + lab - 26, x + im.width + 1, 48 + lab - 24 + im.height + 1],
                outline=col, width=2)
    d.text((x, 46), f.replace(".png", "").replace("-capsule", "").replace("-", " "),
           fill=(200, 210, 220), font=font)
    d.text((x, 48 + lab + im.height - 20), f"{w}x{h}  {'PASS' if ok else 'FAIL'}", fill=col, font=fontb)
    x += im.width + pad
for f, sw, sh in MISSING:
    ph = Image.new("RGB", (int(260), TH), (58, 30, 34))
    dd = ImageDraw.Draw(ph)
    dd.line([0, 0, 260, TH], fill=(120, 60, 66), width=2)
    dd.line([0, TH, 260, 0], fill=(120, 60, 66), width=2)
    canvas.paste(ph, (x, 48 + lab - 24))
    d.rectangle([x - 2, 48 + lab - 26, x + 261, 48 + lab - 24 + TH + 1], outline=(255, 110, 110), width=2)
    d.text((x, 46), f.replace(".png", "").replace("-", " "), fill=(255, 180, 180), font=font)
    d.text((x + 70, 48 + lab + TH // 2 - 30), "MISSING", fill=(255, 110, 110), font=fontb)
    d.text((x, 48 + lab + TH - 20), f"need {sw}x{sh}", fill=(255, 110, 110), font=fontb)
    x += 260 + pad
p = os.path.join(OUT, "capsule-set-confirmation.png")
canvas.save(p)
print("  wrote", p)

sys.exit(0 if all_ok else 1)
