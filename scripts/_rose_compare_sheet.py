#!/usr/bin/env python3
"""Rose — side-by-side BEFORE(ctrl)/AFTER(new) sheet for the AAA scorecard patch.

For each graded stem (gate3-boot/walk/combat, grade-vista) it pastes the
control arm (old look constants restored via env: 1100 lux / ambient B 0.60 /
PCSS 3.0) next to the new baked default (2200 lux / B 0.48 / PCSS 4.0), off the
SAME binary, so the delta is the patch's contribution alone — not the 5-day
stale-binary confound the original scorecard carried.

Writes one per-stem pair and one 4-row contact sheet. Run AFTER the recap
capture has produced the *-nohud2.png frames.
"""
import sys, os
from PIL import Image, ImageDraw, ImageFont

CTRL_LABEL = "BEFORE  ctrl  ·  1100 lux / ambient B 0.60 / PCSS 3.0"
NEW_LABEL = "AFTER  baked  ·  2200 lux / ambient B 0.45 / PCSS 4.0 / ev100 10.9"

def font(sz):
    for c in ("arial.ttf", "C:/Windows/Fonts/arialbd.ttf", "C:/Windows/Fonts/arial.ttf"):
        try: return ImageFont.truetype(c, sz)
        except Exception: pass
    return ImageFont.load_default()

def load(path):
    im = Image.open(path).convert("RGB")
    return im

def fit(im, w, h):
    im.thumbnail((w, h), Image.LANCZOS)
    cv = Image.new("RGB", (w, h), (16, 12, 10))
    cv.paste(im, ((w - im.width) // 2, (h - im.height) // 2))
    return cv

def pair(ctrl_path, new_path, out_path, bar=34, gap=6, cell_w=620, cell_h=348):
    c = load(ctrl_path); n = load(new_path)
    W = 2 * cell_w + gap
    H = bar + cell_h
    sheet = Image.new("RGB", (W, H), (10, 7, 6))
    d = ImageDraw.Draw(sheet)
    f = font(17)
    # ctrl cell
    d.rectangle([0, 0, cell_w, bar], fill=(70, 30, 24))
    d.text((10, 7), CTRL_LABEL, fill=(250, 225, 210), font=f)
    sheet.paste(fit(c, cell_w, cell_h), (0, bar))
    # divider
    d.rectangle([cell_w, 0, cell_w + gap, H], fill=(220, 180, 90))
    # new cell
    nx = cell_w + gap
    d.rectangle([nx, 0, W, bar], fill=(30, 70, 40))
    d.text((nx + 10, 7), NEW_LABEL, fill=(220, 250, 225), font=f)
    sheet.paste(fit(n, cell_w, cell_h), (nx, bar))
    sheet.save(out_path)
    return sheet.size

def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "_rose_recap_20260806"
    stems = sys.argv[2:] if len(sys.argv) > 2 else ["gate3-boot", "gate3-walk", "gate3-combat", "grade-vista"]
    pairs = []
    for stem in stems:
        ctrl = os.path.join(out, f"{stem}-ctrl-nohud2.png")
        new = os.path.join(out, f"{stem}-new-nohud2.png")
        if not (os.path.exists(ctrl) and os.path.exists(new)):
            print(f"SKIP {stem}: missing ({ctrl=}, {new=})")
            continue
        op = os.path.join(out, f"_compare_{stem}.png")
        sz = pair(ctrl, new, op)
        print(f"PAIR {stem}: {op}  {sz}")
        pairs.append((op, stem))

    if not pairs:
        print("no pairs produced"); return
    # contact sheet: stack all pairs vertically, scaled to a common width.
    target_w = 1240
    ims = []
    for op, stem in pairs:
        im = Image.open(op).convert("RGB")
        sc = target_w / im.width
        im = im.resize((target_w, int(im.height * sc)), Image.LANCZOS)
        ims.append((im, stem))
    cap = font(20)
    band = 26
    H = sum(im.height + band for im, _ in ims)
    sheet = Image.new("RGB", (target_w, H), (8, 6, 6))
    d = ImageDraw.Draw(sheet)
    y = 0
    for im, stem in ims:
        d.text((10, y + 4), stem, fill=(235, 210, 150), font=cap)
        sheet.paste(im, (0, y + band))
        y += im.height + band
    cs = os.path.join(out, "_compare_sheet_all.png")
    sheet.save(cs)
    print(f"SHEET {cs}  {sheet.size}")

if __name__ == "__main__":
    main()
