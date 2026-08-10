"""
Render docs/assets/overview-<date>.png — a one-glance contact sheet of the
game's current state, for the CEO. Six real, freshly-shot frames (no mockups):
wide establishing / hero framing / in-game walk / combat / HUD / boot (the
game has no main-menu screen by design — see docs/first-playable-loop.md
Act 0, "No menu. No loading screen text. Player wakes up directly in the
world").

Every panel is a crop/resize of a PNG that actually exists on disk from a
same-day capture; this script does not draw or synthesize any game content.

usage: python scripts/make_overview_contact_sheet.py <out.png> \
         wide=<path> hero=<path> walk=<path> combat=<path> hud=<path> boot=<path>
"""
import os
import sys
import datetime

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")

BG = (18, 19, 23)
FG = (230, 232, 238)
DIM = (150, 154, 165)
RULE = (58, 60, 70)

FONTS_B = ["C:/Windows/Fonts/consolab.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"]
FONTS = ["C:/Windows/Fonts/consola.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"]


def font(size, bold=False):
    for p in (FONTS_B if bold else FONTS):
        if os.path.exists(p):
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


PANELS = [
    ("wide", "WIDE ESTABLISHING", "Edhari village, pulled-back overview"),
    ("hero", "HERO FRAMING", "hero.rs kitchen bowl — Charm-Layer Pass"),
    ("walk", "IN-GAME WALK", "gate3 walk demo, HUD off"),
    ("combat", "COMBAT", "guard-husk encounter, gameplay HUD on"),
    ("hud", "HUD", "combat frame, UI corner crop"),
    ("boot", "BOOT (NO MAIN MENU)", "player spawns straight into the world — by design"),
]

TILE_W, TILE_H = 640, 360
CAP_H = 64
PAD = 14
COLS = 3
HEADER_H = 64
FOOTER_H = 30


def fit_cover(im, w, h):
    sw, sh = im.size
    scale = max(w / sw, h / sh)
    nw, nh = round(sw * scale), round(sh * scale)
    im = im.resize((nw, nh), Image.LANCZOS)
    x0 = (nw - w) // 2
    y0 = (nh - h) // 2
    return im.crop((x0, y0, x0 + w, y0 + h))


def main():
    argv = sys.argv[1:]
    if not argv:
        print("usage: make_overview_contact_sheet.py <out.png> key=path ...", file=sys.stderr)
        sys.exit(2)
    out = argv[0]
    paths = {}
    for kv in argv[1:]:
        k, _, v = kv.partition("=")
        paths[k] = v

    missing = [k for k, _, _ in PANELS if k not in paths]
    if missing:
        print(f"REFUSED  missing panel source(s): {missing}", file=sys.stderr)
        sys.exit(2)
    for k, p in paths.items():
        if not os.path.isfile(p):
            print(f"REFUSED  not a real file on disk: {k}={p}", file=sys.stderr)
            sys.exit(2)

    rows = (len(PANELS) + COLS - 1) // COLS
    W = PAD + COLS * (TILE_W + PAD)
    H = HEADER_H + rows * (TILE_H + CAP_H + PAD) + FOOTER_H

    im = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(im)
    f_h1 = font(26, True)
    f_lbl = font(17, True)
    f_sm = font(13)

    d.text((PAD, 16), "VOXELFORGE — CURRENT STATE OVERVIEW", FG, f_h1)

    for i, (key, label, sub) in enumerate(PANELS):
        col, row = i % COLS, i // COLS
        x = PAD + col * (TILE_W + PAD)
        y = HEADER_H + row * (TILE_H + CAP_H + PAD)

        src = Image.open(paths[key]).convert("RGB")
        tile = fit_cover(src, TILE_W, TILE_H)
        im.paste(tile, (x, y))
        d.rectangle([x, y, x + TILE_W - 1, y + TILE_H - 1], outline=RULE, width=2)

        cy = y + TILE_H + 6
        d.text((x, cy), label, FG, f_lbl)
        d.text((x, cy + 22), sub, DIM, f_sm)
        d.text((x, cy + 40), os.path.relpath(paths[key], ROOT).replace("\\", "/"), DIM, f_sm)

    d.line([PAD, H - FOOTER_H - 8, W - PAD, H - FOOTER_H - 8], RULE, 1)
    d.text((PAD, H - FOOTER_H), f"generated {datetime.datetime.now():%Y-%m-%d %H:%M}"
                                 "  — every panel a real same-day capture, no mockups", DIM, f_sm)

    os.makedirs(os.path.dirname(out), exist_ok=True)
    im.save(out)
    print(f"wrote {out}  ({W}x{H})")


if __name__ == "__main__":
    main()
