# make_vfx_pair_sheet.py — before/after contact sheet for the VFX pairs, with the
# difference actually MEASURED (lane: pixel / Flamingo).
#
# Each row is one beat: the muted plate on the left, the same beat with its
# emitters live on the right. Both plates come out of the same code path at the
# same beat timestamps with the same pose (see render_vfx_pairs.sh), so the only
# thing that can differ between the two cells is the effect layer.
#
# The sheet is not the whole argument, though. This renderer is noisy — SSAO and
# the shadow filter jitter per frame and the showcase stage runs without TAA — so
# "the two images look different" is not by itself evidence. The script therefore
# renders a CONTROL (the before plate shot twice) and reports:
#
#   floor  = how much two identical commands differ  → the renderer idling
#   signal = how much before differs from after      → the effect layer
#
# A row only earns the word PASS when signal is well clear of floor. If a beat
# ever comes back with signal ~= floor, the honest read is that the effect is not
# visible in a still, not that the plate is fine.
#
# Usage:  python scripts/make_vfx_pair_sheet.py
import os
import sys

from PIL import Image, ImageChops, ImageDraw, ImageFont

CELL_W, CELL_H, BAR, PAD = 620, 349, 30, 8
HERE = os.path.dirname(os.path.abspath(__file__))
PAIRS_DIR = os.path.join(HERE, "..", "docs", "assets", "pairs")

ROWS = [
    ("impact", "impact", "weapon trail + contact sparks / voxel debris"),
    ("parry", "parry", "parry ring flash"),
    ("stagger", "stagger", "stagger reel + ground reaction"),
]

# A pixel counts as changed when any channel moves by more than this. Below it the
# difference is dithering and 8-bit rounding, not something an eye reads.
CHANGED_THRESHOLD = 12
# How many times over the measured floor the signal must land to count as proven.
# 3x is deliberately blunt: it is meant to catch "the effect did not render at
# all", not to grade the art.
MIN_RATIO = 3.0

# The ratio test ALONE is not a gate. Once shot_main.rs went to a real fixed step
# (ManualDuration + a counted grab frame) the control started coming back
# byte-identical to the plate it copies — floor = 0.00 %. Divide by that and every
# row scores an infinite ratio and "passes" without rendering anything, which is
# precisely the gate-that-cannot-fail this file's header warns about.
#
# So a row must ALSO clear an absolute bar, which is what actually answers "did
# the effect draw?": it has to move a real share of the frame, and it has to move
# it far enough to see. Both, because a wash of +13 over half the image and three
# blown-out pixels are each a way of scoring well while showing nothing.
MIN_SIGNAL_FRACTION = 0.0025  # 0.25 % of the frame
MIN_SIGNAL_PEAK = 40  # out of 255


def changed_fraction(a_path, b_path):
    """Fraction of pixels that moved by more than CHANGED_THRESHOLD, and the peak."""
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
        # Fail loudly rather than emit a sheet with holes — a half-filled sheet
        # reads as "this is all there is".
        print("FAIL missing renders: " + ", ".join(missing))
        return 1

    # The floor, measured on this run, on this machine.
    floor, floor_peak = changed_fraction(p("impact-a-before.png"), control)
    print(f"FLOOR  (identical command, twice): {floor*100:.2f}% of pixels changed, peak {floor_peak}/255")
    if floor == 0.0 and floor_peak == 0:
        print("FLOOR  control is byte-identical — this render is deterministic; rows are graded on the absolute bar")

    cols = 2
    sheet = Image.new("RGB", (CELL_W * cols + PAD * 3, (CELL_H + BAR) * len(ROWS) + PAD * (len(ROWS) + 1) + 34), (12, 11, 12))
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
        # Absolute bar first — it is the one that still bites when floor is 0.
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

    out = os.path.join(PAIRS_DIR, "vfx-pairs-sheet.png")
    sheet.save(out)
    print("SHEET " + os.path.normpath(out))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
