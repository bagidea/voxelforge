# _kevin_vfx_sheet.py — before/after contact sheet for kevin's VFX pass, with the
# difference MEASURED against a control (same methodology as make_vfx_pair_sheet.py,
# but the row set includes the DISSOLVE beat that carry the blood burst + pool).
#
# Each row: muted plate (left) vs the same beat live (right). A control (the before
# plate shot twice) measures this renderer's per-frame noise, and a row only earns
# PASS when its before/after signal is well clear of that floor AND clears an
# absolute bar — so "the two images differ" is never mistaken for "the effect drew".
#
# Usage:  python scripts/_kevin_vfx_sheet.py
import os
import sys

from PIL import Image, ImageChops, ImageDraw, ImageFont

CELL_W, CELL_H, BAR, PAD = 620, 349, 30, 8
HERE = os.path.dirname(os.path.abspath(__file__))
PAIRS_DIR = os.path.join(HERE, "..", "docs", "assets", "pairs")

ROWS = [
    ("impact", "impact", "blood droplets + impact dust + trail + sparks"),
    ("dissolve", "dissolve", "death blood burst + lingering pool + dissolve"),
    ("parry", "parry", "parry ring flash"),
    ("stagger", "stagger", "stagger reel + ground reaction"),
]

CHANGED_THRESHOLD = 12
MIN_RATIO = 3.0
MIN_SIGNAL_FRACTION = 0.0025  # 0.25 % of the frame
MIN_SIGNAL_PEAK = 40  # out of 255


def changed_fraction(a_path, b_path):
    a = Image.open(a_path).convert("RGB")
    b = Image.open(b_path).convert("RGB")
    if a.size != b.size:
        raise SystemExit(f"FAIL size mismatch: {a_path} {a.size} vs {b_path} {b.size}")
    diff = ImageChops.difference(a, b).convert("L")
    hist = diff.histogram()
    total = a.size[0] * a.size[1]
    changed = sum(hist[CHANGED_THRESHOLD + 1 :])
    peak = max((i for i, n in enumerate(hist) if n), default=0)
    return changed / total, peak


def load_fit(path):
    im = Image.open(path).convert("RGB")
    im.thumbnail((CELL_W, CELL_H), Image.LANCZOS)
    canvas = Image.new("RGB", (CELL_W, CELL_H), (16, 14, 15))
    canvas.paste(im, ((CELL_W - im.width) // 2, (CELL_H - im.height) // 2))
    return canvas


def main():
    try:
        font = ImageFont.truetype("arial.ttf", 16)
        small = ImageFont.truetype("arial.ttf", 14)
    except OSError:
        font = small = ImageFont.load_default()

    def p(name):
        return os.path.join(PAIRS_DIR, name)

    control = p("impact-a-before-control.png")
    need = [control]
    for slug, _, _ in ROWS:
        need += [p(f"{slug}-a-before.png"), p(f"{slug}-b-after.png")]
    missing = [os.path.basename(f) for f in need if not os.path.isfile(f)]
    if missing:
        print("FAIL missing renders: " + ", ".join(missing))
        return 1

    floor, floor_peak = changed_fraction(p("impact-a-before.png"), control)
    print(f"FLOOR  (identical command, twice): {floor*100:.2f}% of pixels changed, peak {floor_peak}/255")
    if floor == 0.0 and floor_peak == 0:
        print("FLOOR  control is byte-identical — render is deterministic; rows graded on the absolute bar")

    cols = 2
    sheet = Image.new(
        "RGB",
        (CELL_W * cols + PAD * 3, (CELL_H + BAR) * len(ROWS) + PAD * (len(ROWS) + 1) + 34),
        (12, 11, 12),
    )
    draw = ImageDraw.Draw(sheet)
    floor_txt = (
        "control byte-identical — render is deterministic"
        if floor == 0
        else f"renderer noise floor this run: {floor*100:.2f}% of pixels"
    )
    draw.text((PAD + 2, 9), f"Voxelforge VFX — before / after pairs   ({floor_txt})", font=font, fill=(236, 226, 214))

    ok = True
    for r, (slug, _, desc) in enumerate(ROWS):
        before, after = p(f"{slug}-a-before.png"), p(f"{slug}-b-after.png")
        signal, peak = changed_fraction(before, after)
        ratio = signal / floor if floor > 0 else float("inf")
        big_enough = signal >= MIN_SIGNAL_FRACTION and peak >= MIN_SIGNAL_PEAK
        clears_noise = ratio >= MIN_RATIO
        verdict = "PASS" if (big_enough and clears_noise) else "WEAK"
        if verdict != "PASS":
            ok = False
        ratio_txt = "deterministic" if floor == 0 else f"{ratio:4.1f}x floor"
        why = ""
        if not big_enough:
            why = f"  <- under the bar ({MIN_SIGNAL_FRACTION*100:.2f}% / peak {MIN_SIGNAL_PEAK})"
        elif not clears_noise:
            why = f"  <- under {MIN_RATIO}x the noise floor"
        print(f"{verdict}   {slug:<8} signal {signal*100:5.2f}%  peak {peak:3d}/255  = {ratio_txt}  ({desc}){why}")

        y = 34 + PAD + r * (CELL_H + BAR + PAD)
        for c, (path, tag) in enumerate(((before, "BEFORE — emitters muted"), (after, "AFTER — emitters live"))):
            x = PAD + c * (CELL_W + PAD)
            draw.rectangle([x, y, x + CELL_W, y + BAR], fill=(26, 24, 26))
            draw.text((x + 9, y + 7), f"{slug.upper()}  {tag}", font=small, fill=(236, 226, 214))
            sheet.paste(load_fit(path), (x, y + BAR))
        draw.text(
            (PAD + CELL_W + PAD + 9, y + BAR + CELL_H - 21),
            f"{desc} — {signal*100:.2f}% of pixels, peak {peak}/255 ({ratio_txt})",
            font=small,
            fill=(255, 214, 138),
        )

    out = os.path.join(PAIRS_DIR, "kevin-vfx-pairs-sheet.png")
    sheet.save(out)
    print("SHEET " + os.path.normpath(out))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
