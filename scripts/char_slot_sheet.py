#!/usr/bin/env python3
"""Build the character-slot contact sheets from the charshot stage renders.

Three panels:
  1. char-before-after.png  — Auren v1 vs the current body (one binary, one camera)
  2. char-outfit-swap.png   — the outfit ladder on one root entity
  3. char-weapon-swap.png   — the weapon ladder on one root entity

Every panel self-checks: after compositing, each tile is read back OUT of the
finished panel and pixel-compared against the file its caption names. A caption
that does not match its pixels is a hard failure here, not a thing to notice in
review later.

Exit codes:  0 = built + verified   1 = built but a check FAILED   2 = refused
(missing inputs — could not measure, which is not the same as measured-and-bad).
"""

import sys
import pathlib

from PIL import Image, ImageDraw, ImageFont

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "_fl_charshot"
OUT = ROOT / "_fl_charshot"

BG = (22, 20, 26)
FG = (238, 234, 228)
DIM = (150, 143, 136)
RULE = (70, 64, 72)

PAD = 28
GAP = 20
CAP_H = 74
HEAD_H = 64


def font(size, bold=False):
    names = (
        ["seguisb.ttf", "segoeuib.ttf", "arialbd.ttf"]
        if bold
        else ["segoeui.ttf", "arial.ttf"]
    )
    for n in names:
        try:
            return ImageFont.truetype(n, size)
        except OSError:
            continue
    return ImageFont.load_default()


F_TITLE = font(30, True)
F_CAP = font(23, True)
F_SUB = font(18)


def load(name):
    p = SRC / name
    if not p.exists():
        return None, p
    return Image.open(p).convert("RGB"), p


def mean_abs_diff(a, b):
    """Mean |difference| per channel-pixel between two same-size RGB images."""
    if a.size != b.size:
        return None
    pa, pb = a.tobytes(), b.tobytes()
    return sum(abs(x - y) for x, y in zip(pa, pb)) / len(pa)


def wrap(draw, text, fnt, maxw):
    """Greedy word-wrap to `maxw` px. Captions now list all six slots, which is
    wider than a tile — unwrapped they ran off the sheet edge and over the
    neighbouring tile, so a caption could end up sitting on the wrong picture."""
    words, lines, cur = text.split(" "), [], ""
    for w in words:
        trial = w if not cur else cur + " " + w
        if draw.textlength(trial, font=fnt) <= maxw or not cur:
            cur = trial
        else:
            lines.append(cur)
            cur = w
    if cur:
        lines.append(cur)
    return lines


SUB_LH = 22


def build(panel_name, title, subtitle, tiles, scale_w):
    """tiles = [(filename, caption, sub)] laid out left→right."""
    imgs = []
    for fn, cap, sub in tiles:
        im, p = load(fn)
        if im is None:
            print(f"REFUSE: missing input {p}")
            return 2
        imgs.append((im, fn, cap, sub))

    tw = scale_w
    th = round(imgs[0][0].height * (tw / imgs[0][0].width))
    n = len(imgs)

    # Measure the wrapped captions first so the sheet is tall enough for them.
    probe = ImageDraw.Draw(Image.new("RGB", (8, 8)))
    wrapped = [wrap(probe, sub, F_SUB, tw) for _, _, _, sub in imgs]
    cap_h = 42 + SUB_LH * max(len(l) for l in wrapped) + 14

    W = PAD * 2 + tw * n + GAP * (n - 1)
    # The header subtitle needs wrapping for the same reason the tile captions do:
    # at 3 tiles it ran past the right edge and the last words were cropped off.
    head_lines = wrap(probe, subtitle, F_SUB, W - PAD * 2)
    head_h = max(HEAD_H, 38 + SUB_LH * len(head_lines) + 8)
    H = PAD + head_h + th + cap_h + PAD

    sheet = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(sheet)
    d.text((PAD, PAD - 4), title, font=F_TITLE, fill=FG)
    for li, line in enumerate(head_lines):
        d.text((PAD, PAD + 34 + li * SUB_LH), line, font=F_SUB, fill=DIM)

    # paste
    boxes = []
    for i, (im, fn, cap, sub) in enumerate(imgs):
        x = PAD + i * (tw + GAP)
        y = PAD + head_h
        rs = im.resize((tw, th), Image.LANCZOS)
        sheet.paste(rs, (x, y))
        # Rule drawn OUTSIDE the tile bounds. Drawn on top of them, it overwrites
        # border pixels and the readback check below can never reach mean|d|==0 —
        # which would quietly turn the check into noise instead of a check.
        d.rectangle([x - 1, y - 1, x + tw, y + th], outline=RULE, width=1)
        d.text((x, y + th + 12), cap, font=F_CAP, fill=FG)
        for li, line in enumerate(wrapped[i]):
            d.text((x, y + th + 42 + li * SUB_LH), line, font=F_SUB, fill=DIM)
        boxes.append((x, y, tw, th, rs, fn, cap))

    OUT.mkdir(parents=True, exist_ok=True)
    dest = OUT / panel_name
    sheet.save(dest)

    # --- caption/pixel check: read each tile back OUT of the saved panel ----
    saved = Image.open(dest).convert("RGB")
    bad = 0
    for x, y, w, h, rs, fn, cap in boxes:
        got = saved.crop((x, y, x + w, y + h))
        mad = mean_abs_diff(got, rs)
        ok = mad is not None and mad == 0.0
        print(f"  CHECK tile '{cap}' <- {fn}: mean|d|={mad} {'PASS' if ok else 'FAIL'}")
        if not ok:
            bad += 1

    # --- and prove the tiles are not the same picture twice ----------------
    for i in range(len(boxes) - 1):
        a, b = boxes[i], boxes[i + 1]
        mad = mean_abs_diff(a[4], b[4])
        distinct = mad is not None and mad > 0.5
        print(
            f"  DISTINCT '{a[6]}' vs '{b[6]}': mean|d|={mad:.3f} "
            f"{'PASS' if distinct else 'FAIL — tiles are the same image'}"
        )
        if not distinct:
            bad += 1

    print(f"WROTE {dest}  ({sheet.width}x{sheet.height})  failures={bad}")
    return 1 if bad else 0


def main():
    if not SRC.exists():
        print(f"REFUSE: {SRC} does not exist — nothing was shot")
        return 2

    rc = 0
    print("== panel 0: full body, front / back ==")
    r = build(
        "char-body-front-back.png",
        "Auren — full body, front and back",
        "One stage (=auren), one binary. The back plate is the SAME camera "
        "mirrored through the look-at target: same distance, same height, same "
        "32 deg fov — the viewpoint is the only difference.",
        [
            ("body-front.png", "FRONT", "default AB_CAM"),
            ("body-back.png", "BACK", "AB_CAM mirrored through the target"),
        ],
        scale_w=620,
    )
    rc = max(rc, r)

    print("== panel 1: before / after ==")
    r = build(
        "char-before-after.png",
        "Auren — body rebuild, before vs after",
        "Two stages of ONE binary at ONE camera: the plates cannot differ by build, "
        "shader cache, driver or framing — only by the geometry.",
        [
            ("before-auren-v1.png", "BEFORE — auren-v1", "the 2026-08-13 body"),
            ("after-auren.png", "AFTER — auren", "current body + slot system"),
        ],
        scale_w=620,
    )
    rc = max(rc, r)

    print("== panel 2: outfit swap ==")
    r = build(
        "char-outfit-swap.png",
        "Outfit swap — all 6 slots, one root entity",
        "equip_loadout() on the same entity between captures. The root is never "
        "despawned and spawn_character() is never called twice. Captions read in "
        "Slot::ALL order: head · torso · legs · hands · weapon · back.",
        [
            # Read straight off equipment.rs villager()/adventurer()/warplate().
            # The previous captions said 'bracers' for adventurer (it is
            # GLOVES_LEATHER) and dropped the legs slot from all three, which is
            # exactly the slot the swap is meant to show changing.
            (
                "swap-1-villager.png",
                "villager",
                "kerchief · linen tunic · work trousers · cloth wraps · field sickle · harvest pack",
            ),
            (
                "swap-2-adventurer.png",
                "adventurer",
                "travel hood · leather jerkin · travel breeches · leather gloves · shortsword · half-cloak",
            ),
            (
                "swap-3-warplate.png",
                "warplate",
                "greathelm · steel cuirass · plate greaves · steel gauntlets · warhammer · battle cape",
            ),
        ],
        scale_w=430,
    )
    rc = max(rc, r)

    print("== panel 3: weapon swap ==")
    r = build(
        "char-weapon-swap.png",
        "Weapon swap — one slot, everything else held",
        "equip(Slot::Weapon, …) only. Same body, same outfit, same camera — the "
        "hand is the only thing that changes.",
        [
            ("weapons-1-sword.png", "sword_short", "Traveller's Shortsword"),
            ("weapons-2-hammer.png", "hammer_war", "Warhammer"),
            ("weapons-3-stave.png", "stave_shaper", "Shaper's Stave"),
        ],
        scale_w=430,
    )
    rc = max(rc, r)

    print(f"\nOVERALL rc={rc}")
    return rc


if __name__ == "__main__":
    sys.exit(main())
