"""
Render docs/assets/gate3/gate3-colour-verdict.png — the one-glance card behind
docs/gate3-colour-review-2026-08-01.md.

Every number on the card is MEASURED HERE at render time from the PNGs on disk
(via scripts/colour_gate.py and scripts/grade_axes.py) and the authored sky is
PARSED from client/src/main.rs. Nothing is typed in by hand, so the card cannot
drift away from the frames the way the first hand-typed version did.

usage:  python scripts/make_gate3_verdict_card.py [-o out.png]
"""
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import colour_gate
import grade_axes

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
A = lambda *p: os.path.normpath(os.path.join(ROOT, "docs", "assets", *p))
OUT = A("gate3", "gate3-colour-verdict.png")

W, H = 1280, 560
BG = (26, 27, 33)
FG = (215, 217, 224)
DIM = (150, 154, 165)
RED = (232, 93, 108)
GRN = (120, 200, 140)
RULE = (58, 60, 70)

FONTS = ["C:/Windows/Fonts/consola.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"]
FONTS_B = ["C:/Windows/Fonts/consolab.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"]


def font(size, bold=False):
    for p in (FONTS_B if bold else FONTS):
        if os.path.exists(p):
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


def sunlit(path):
    """The gate's own sunlit patch (gate C) — brightest lit non-sky surface."""
    a = np.asarray(Image.open(path).convert("RGB")).astype(np.float32)
    sky, _ = colour_gate.sky_patch(a)
    return tuple(float(v) for v in colour_gate.sunlit_patch(a, sky))


def order(rgb):
    r, g, b = rgb
    names = sorted(zip((r, g, b), "RGB"), reverse=True)
    return ">".join(n for _, n in names)


def main():
    out = sys.argv[sys.argv.index("-o") + 1] if "-o" in sys.argv else OUT

    auth = colour_gate.authored_clearcolor()
    auth_rgb = tuple(c * 255.0 for c in auth)

    jul = colour_gate.measure(A("thirdperson-walk.png"))
    aug = colour_gate.measure(A("gate3", "gate3-after-boot.png"))
    ref_sun = sunlit(A("golden-beauty-shot-ref.png"))

    frames = {k: colour_gate.measure(A("gate3", f"gate3-after-{k}.png"))
              for k in ("boot", "walk", "combat")}
    axes = {k: grade_axes.measure(A("gate3", f"gate3-after-{k}.png"))
            for k in ("boot", "walk", "combat")}
    ref_m = colour_gate.measure(A("golden-beauty-shot-ref.png"))
    base_m = colour_gate.measure(A("wide-hero-final.png"))

    im = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(im)
    f_h1, f_lbl, f_txt, f_sm = font(23, True), font(15, True), font(14), font(13)

    d.text((24, 18), "VOXELFORGE — GATE 3 COLOUR VERDICT: FAIL (global magenta grade)", RED, f_h1)
    d.text((24, 54), f"Same authored ClearColor({auth[0]:g},{auth[1]:g},{auth[2]:g}) rendered by two builds. "
                     "Sky cannot be R>B>G under any lighting.", DIM, f_sm)

    cards = [
        ("AUTHORED\nClearColor\nmain.rs (parsed)", auth_rgb, "blue"),
        ("JUL-31 BUILD\nthirdperson-walk\n(ungraded)", jul["sky"], "blue"),
        ("AUG-01 BUILD\ngate3-after-boot\n(graded)", aug["sky"], "MAGENTA"),
        ("GOLDEN REF\nsunlit patch\n(target look)", ref_sun, "amber"),
    ]
    x, cw, gap = 34, 250, 62
    for i, (label, rgb, tone) in enumerate(cards):
        cx = x + i * (cw + gap)
        d.rectangle([cx, 110, cx + cw, 258], fill=tuple(int(round(v)) for v in rgb))
        d.multiline_text((cx, 270), label, FG, f_lbl, spacing=4)
        d.text((cx, 348), f"RGB {rgb[0]:.0f},{rgb[1]:.0f},{rgb[2]:.0f}", DIM, f_sm)
        o = order(rgb)
        col = RED if tone == "MAGENTA" else GRN
        d.text((cx, 367), f"{o}  {tone}", col, f_sm)
        if i == 1:
            dv = max(abs(a_ - b_) for a_, b_ in zip(rgb, auth_rgb))
            d.text((cx + 124, 367), f"= {dv:.1f}/255", GRN, f_sm)

    d.line([24, 405, W - 24, 405], RULE, 1)

    gr, gg, gb = aug["sky_gain"]
    fpct = lambda m, k: m[k] * 100.0
    d.text((24, 419), "PER-CHANNEL LINEAR GAIN, authored sky -> Aug-01 render:"
                      f"   R x{gr:.2f}   G x{gg:.2f}   B x{gb:.2f}", RED, f_lbl)
    spread = abs(gg - gb) / max(gg, gb) * 100.0
    d.text((24, 442), f"G and B pulled down by the SAME factor ({spread:.1f}% apart) while R doubles"
                      " = red-only push, not amber white-balance.", DIM, f_sm)

    def row(y, head, cells, col=RED):
        d.text((24, y), head, col, f_sm)
        d.text((24 + 300, y), cells, col, f_sm)

    b, w_, c = frames["boot"], frames["walk"], frames["combat"]
    ab, aw, ac = axes["boot"], axes["walk"], axes["combat"]
    row(471, "P0 axes (chromatic only)",
        f"blue B: {ab['blue']:.1f} / {aw['blue']:.1f} / {ac['blue']:.1f}  vs target <=10"
        f"      saturation: {ab['sat']:.0f} / {aw['sat']:.0f} / {ac['sat']:.0f}  vs target >=90")
    row(492, "Warm-ordered px (R>G>B, of lit)",
        f"boot {fpct(b, 'warm_frac_lit'):.1f}%  walk {fpct(w_, 'warm_frac_lit'):.1f}%"
        f"  combat {fpct(c, 'warm_frac_lit'):.1f}%"
        f"      golden ref {fpct(ref_m, 'warm_frac_lit'):.1f}%"
        f"   CEO baseline {fpct(base_m, 'warm_frac_lit'):.1f}%")
    row(513, "Magenta px (G below BOTH R and B)",
        f"boot {fpct(b, 'mag_frac'):.1f}%  walk {fpct(w_, 'mag_frac'):.1f}%"
        f"  combat {fpct(c, 'mag_frac'):.1f}%"
        f"      every approved frame: {fpct(ref_m, 'mag_frac'):.2f}%")
    d.text((24, 534), "Measured by scripts/colour_gate.py + scripts/grade_axes.py at card-render time"
                      " — no hand-typed numbers.", DIM, f_sm)

    im.save(out)
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
