# _pixel_g2g4_sheet.py -- before/after sheet for gates G2 (mullion shadow bars) and
# G4 (penumbra + contact shadows). Lane: pixel / Flamingo, 2026-08-16.
#
# WHAT THIS SHEET IS ALLOWED TO CLAIM
#
# Two plates that look different are not evidence on their own: this renderer jitters
# (SSAO + a stochastic shadow filter + TAA), so some pixels differ between two runs of
# the SAME command. The sheet therefore reports both numbers and refuses to grade
# without them:
#
#   floor  = before vs before-r2  -- identical command twice: the renderer idling
#   signal = before vs after      -- the same binary with the fix levers flipped
#
# A row earns PASS only when signal is well clear of floor AND the gate's own measured
# number moved the right way. If signal ever comes back near floor, the honest read is
# "the change is not visible in this still", not "the plate is fine".
#
# Every caption is derived from the file it names -- the numbers are re-measured here
# from the PNGs and parsed out of the grade logs, never typed in. A caption that has
# drifted from its pixels is the specific failure this repo has been bitten by.
#
# Usage:  python scripts/_pixel_g2g4_sheet.py [plate_dir] [out.png]
import os
import re
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

DIR = sys.argv[1] if len(sys.argv) > 1 else "_fl_g2g4_20260816"
OUT = sys.argv[2] if len(sys.argv) > 2 else "docs/assets/look/g2g4-before-after.png"

# The floor band: the lower 45% of the frame is exactly where grade_look.py hunts for
# penumbra, and it is where the window bars and the block/floor contact seams land.
# Cropping to it is not cherry-picking -- it is showing the region the gate scores.
CROP = (0.10, 0.55, 0.90, 1.00)  # x0, y0, x1, y1 as fractions


def load(name):
    p = os.path.join(DIR, f"{name}-nohud2.png")
    if not os.path.exists(p):
        sys.exit(f"MISSING {p} -- run scripts/_pixel_g2g4_ab.sh first")
    return p, Image.open(p).convert("RGB")


def arr(im):
    return np.asarray(im, dtype=np.int16)


def diff_px(a, b):
    """Share of pixels that differ at all, and the mean absolute channel delta."""
    da, db = arr(a), arr(b)
    if da.shape != db.shape:
        sys.exit(f"SHAPE MISMATCH {da.shape} vs {db.shape}")
    d = np.abs(da - db)
    n = int((d.max(axis=2) > 0).sum())
    return 100.0 * n / d.shape[0] / d.shape[1], float(d.mean())


def grade_num(plate, pattern, cast=float):
    """Pull one measured number out of that plate's grade_look.py log.

    Returns (value, why) so a caption can say WHICH kind of nothing it got:
    a missing log and a log that found no shadow edge at all are different
    facts, and "no log" printed over a plate that was graded is a caption
    lying about its own evidence.
    """
    p = os.path.join(DIR, f"{plate}.grade.log")
    if not os.path.exists(p):
        return None, "no grade log"
    m = re.search(pattern, open(p, encoding="utf-8", errors="replace").read())
    if m:
        return cast(m.group(1)), ""
    return None, "not measurable (grade_look found no qualifying edge)"


def crop(im):
    w, h = im.size
    return im.crop(
        (int(CROP[0] * w), int(CROP[1] * h), int(CROP[2] * w), int(CROP[3] * h))
    )


def font(sz):
    for f in ("C:/Windows/Fonts/segoeui.ttf", "C:/Windows/Fonts/arial.ttf"):
        if os.path.exists(f):
            return ImageFont.truetype(f, sz)
    return ImageFont.load_default()


bpath, before = load("before")
apath, after = load("after")
_, before_r2 = load("before-r2")
_, after_r2 = load("after-r2")

floor_pct, floor_mean = diff_px(before, before_r2)
floor2_pct, floor2_mean = diff_px(after, after_r2)
sig_pct, sig_mean = diff_px(before, after)

pen_b, pen_b_why = grade_num("before", r"penumbra median=(\d+)px", int)
pen_a, pen_a_why = grade_num("after", r"penumbra median=(\d+)px", int)
edges_b, _ = grade_num("before", r"edges found=(\d+)", int)
edges_a, _ = grade_num("after", r"edges found=(\d+)", int)

# The >=5px line is PLATE-BOUND: grade_look.py calibrated it at 1280w with a 0.6px
# pre-blur (golden ref 8px, old hard-PCF frame 3px). A plate of another width would
# make the same number mean something else, so the sheet PRINTS the size it graded
# instead of leaving the reader to assume it.
PLATE_W, PLATE_H = before.size
PLATE_NOTE = f"{PLATE_W}x{PLATE_H}" + ("" if PLATE_W == 1280 else "  !! threshold calibrated at 1280w")

# ATTRIBUTION. `after` flips both fixes at once, so on its own it cannot say which one
# earned the penumbra. The two single-lever plates can:
#   g2-only = pane passes light, shadow map still 4096, no contact shadows
#   g4-only = pane still blocks, shadow map 2048, contact shadows on
pen_g2, _ = grade_num("g2-only", r"penumbra median=(\d+)px", int)
pen_g4, _ = grade_num("g4-only", r"penumbra median=(\d+)px", int)
edges_g2, _ = grade_num("g2-only", r"edges found=(\d+)", int)
edges_g4, _ = grade_num("g4-only", r"edges found=(\d+)", int)

# Which number decides "the change is visible"? NOT the share of differing pixels: this
# renderer's TAA/SSAO dither flips ~10% of pixels by +-1 between two runs of the SAME
# command, so a pixel COUNT is mostly noise on both sides. mean|d| weights each pixel by
# how far it moved, and the floor rows show what idling costs there. Both are printed;
# the verdict colour follows the mean, and 5x the worse floor is the bar.
floor_mean_max = max(floor_mean, floor2_mean)
mean_ratio = sig_mean / floor_mean_max if floor_mean_max > 0 else float("inf")
VISIBLE = mean_ratio >= 5.0

def brightening_box(a_before, a_after, win=(360, 200)):
    """Where did the sun actually land? Derived from the two frames, never typed in.

    Sum the POSITIVE luminance delta (after - before) into a coarse cell grid, take the
    hottest cell, and return a native-pixel window centred on it. Positive-only on
    purpose: G2's claim is "light arrives somewhere it did not before", so a region that
    merely got darker must not win the crop. Hard-coding a box here would be the
    next-person trap this repo keeps stepping in -- a plate reshot at another framing
    would silently zoom on the wrong wall while the caption still said "the light".
    """
    lb = arr(a_before).mean(axis=2)
    la = arr(a_after).mean(axis=2)
    gain = np.clip(la - lb, 0, None)
    h, w = gain.shape
    cw, ch = 40, 40
    ny, nx = h // ch, w // cw
    cells = gain[: ny * ch, : nx * cw].reshape(ny, ch, nx, cw).sum(axis=(1, 3))
    iy, ix = np.unravel_index(int(cells.argmax()), cells.shape)
    cx, cy = ix * cw + cw // 2, iy * ch + ch // 2
    ww, wh = win
    x0 = max(0, min(w - ww, cx - ww // 2))
    y0 = max(0, min(h - wh, cy - wh // 2))
    # Plain ints: numpy scalars stringify as "np.int64(800)" and that would land in a
    # printed caption.
    box = (int(x0), int(y0), int(x0 + ww), int(y0 + wh))
    return box, float(gain.max()), float(cells.max() / (cw * ch))


BOX, gain_max, gain_cell = brightening_box(before, after)

# The sheet is 3 rows: full frame, the graded floor band, then the measured hot-spot.
CELL_W = 760
pad, gutter, cap_h, hdr_h = 18, 16, 108, 92

full_b = before.copy()
full_a = after.copy()
sc = CELL_W / full_b.width
full_b = full_b.resize((CELL_W, int(full_b.height * sc)), Image.LANCZOS)
full_a = full_a.resize((CELL_W, int(full_a.height * sc)), Image.LANCZOS)

cr_b, cr_a = crop(before), crop(after)
sc2 = CELL_W / cr_b.width
cr_b = cr_b.resize((CELL_W, int(cr_b.height * sc2)), Image.LANCZOS)
cr_a = cr_a.resize((CELL_W, int(cr_a.height * sc2)), Image.LANCZOS)

hot_b, hot_a = before.crop(BOX), after.crop(BOX)
sc3 = CELL_W / hot_b.width
hot_b = hot_b.resize((CELL_W, int(hot_b.height * sc3)), Image.NEAREST)
hot_a = hot_a.resize((CELL_W, int(hot_a.height * sc3)), Image.NEAREST)

W = pad * 2 + CELL_W * 2 + gutter
H = (
    hdr_h
    + full_b.height + cap_h
    + gutter + cr_b.height + cap_h
    + gutter + hot_b.height + cap_h
    + pad
)
sheet = Image.new("RGB", (W, H), (18, 18, 20))
d = ImageDraw.Draw(sheet)
f_hdr, f_ttl, f_cap = font(24), font(20), font(15)

d.text(
    (pad, 16),
    "Voxelforge - G2 window shadow bars + G4 penumbra/contact  (pixel lane, 2026-08-16)",
    font=f_hdr,
    fill=(240, 236, 228),
)
d.text(
    (pad, 46),
    f"ONE binary, {os.path.basename(bpath)} vs {os.path.basename(apath)} - only env differs.   plate {PLATE_NOTE}   "
    f"noise floor mean|d| {floor_mean:.4f} / {floor2_mean:.4f} ({floor_pct:.1f}% / {floor2_pct:.1f}% of px)   |   "
    f"signal mean|d| {sig_mean:.4f} ({sig_pct:.1f}% of px) = {mean_ratio:.1f}x floor",
    font=f_cap,
    fill=(150, 200, 170) if VISIBLE else (220, 160, 120),
)
d.text(
    (pad, 66),
    "ATTRIBUTION (single-lever plates): "
    + f"g2-only penumbra {pen_g2}px / {edges_g2} edges   -   g4-only penumbra {pen_g4}px / {edges_g4} edges   -   "
    + f"both {pen_a}px / {edges_a} edges   vs before {pen_b}px / {edges_b} edges",
    font=f_cap,
    fill=(178, 176, 172),
)

y = hdr_h
for i, (im, ttl) in enumerate(((full_b, "BEFORE"), (full_a, "AFTER"))):
    x = pad + i * (CELL_W + gutter)
    sheet.paste(im, (x, y))
    d.text((x, y + im.height + 6), ttl, font=f_ttl, fill=(235, 230, 220))

cap_b = [
    "pane OCCLUDES the sun (VOXELFORGE_G2_PANE=block)",
    "shadow map 4096  |  ContactShadows OFF",
    f"grade_look G4a penumbra median = {pen_b}px  (line >=5, ref 8)" if pen_b is not None else f"grade_look G4a penumbra: {pen_b_why}",
    f"shadow edges found = {edges_b}" if edges_b is not None else "",
]
cap_a = [
    "pane spawned NotShadowCaster - sun reaches the mullions",
    "shadow map 2048  |  ContactShadows on (0.85 blocks)",
    f"grade_look G4a penumbra median = {pen_a}px  (line >=5, ref 8)" if pen_a is not None else f"grade_look G4a penumbra: {pen_a_why}",
    f"shadow edges found = {edges_a}" if edges_a is not None else "",
]
for i, lines in enumerate((cap_b, cap_a)):
    x = pad + i * (CELL_W + gutter)
    for j, ln in enumerate(lines):
        d.text((x, y + full_b.height + 34 + j * 18), ln, font=f_cap, fill=(178, 176, 172))

y2 = y + full_b.height + cap_h + gutter
for i, (im, ttl) in enumerate(((cr_b, "BEFORE - graded floor band"), (cr_a, "AFTER - graded floor band"))):
    x = pad + i * (CELL_W + gutter)
    sheet.paste(im, (x, y2))
    d.text((x, y2 + im.height + 6), ttl, font=f_ttl, fill=(235, 230, 220))
    d.text(
        (x, y2 + im.height + 32),
        f"lower {int((1 - CROP[1]) * 100)}% of frame - the exact band grade_look.py scans for penumbra",
        font=f_cap,
        fill=(178, 176, 172),
    )

y3 = y2 + cr_b.height + cap_h + gutter
for i, (im, ttl) in enumerate(
    ((hot_b, "BEFORE - where the sun now lands"), (hot_a, "AFTER - where the sun now lands"))
):
    x = pad + i * (CELL_W + gutter)
    sheet.paste(im, (x, y3))
    d.text((x, y3 + im.height + 6), ttl, font=f_ttl, fill=(235, 230, 220))
    # Two lines: one long line runs past the cell and prints over its neighbour, which
    # is how a caption ends up sitting on a plate it is not describing.
    d.text(
        (x, y3 + im.height + 32),
        f"box {BOX} FOUND by max (after-before) luminance gain - not chosen by hand",
        font=f_cap,
        fill=(178, 176, 172),
    )
    d.text(
        (x, y3 + im.height + 50),
        f"peak gain +{gain_max:.0f} L   hottest 40x40 cell +{gain_cell:.1f} L/px   (native px, nearest-neighbour zoom)",
        font=f_cap,
        fill=(178, 176, 172),
    )

os.makedirs(os.path.dirname(OUT), exist_ok=True)
sheet.save(OUT)
print(f"floor(before)  {floor_pct:7.3f}% px  mean|d| {floor_mean:.4f}")
print(f"floor(after)   {floor2_pct:7.3f}% px  mean|d| {floor2_mean:.4f}")
print(f"signal(b->a)   {sig_pct:7.3f}% px  mean|d| {sig_mean:.4f}   = {mean_ratio:.1f}x the worse floor -> {'VISIBLE' if VISIBLE else 'NOT CLEAR OF NOISE'}")
print(f"attribution    g2-only={pen_g2}px/{edges_g2}e   g4-only={pen_g4}px/{edges_g4}e   both={pen_a}px/{edges_a}e")
print(f"penumbra       before={pen_b}px  after={pen_a}px   (line >=5, ref 8)  plate {PLATE_NOTE}")
print(f"hotspot        box={BOX}  peak gain +{gain_max:.0f} L  hottest cell +{gain_cell:.1f} L/px")
print(f"WROTE {OUT} ({sheet.width}x{sheet.height})")
