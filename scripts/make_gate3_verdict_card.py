"""
Render docs/assets/gate3/gate3-colour-verdict.png — the one-glance card behind
docs/gate3-colour-review-2026-08-01.md.

Every number on the card is MEASURED HERE at render time from the PNGs on disk
(via scripts/colour_gate.py and scripts/grade_axes.py) and the authored sky is
PARSED from client/src/main.rs. Nothing is typed in by hand, so the card cannot
drift away from the frames the way the first hand-typed version did.

The VERDICT LINE and every pass/fail colour are derived the same way — from the
gate functions in colour_gate.py and the TARGETS table in grade_axes.py. An
earlier version hard-coded "FAIL (global magenta grade)" in the title, so after
Poppy's fix the card kept announcing FAIL over frames that measure PASS. Nothing
that states a verdict may be a string literal here.

The footer stamps each measured PNG's mtime so a stale card is visible on the
card itself.

usage:  python scripts/make_gate3_verdict_card.py [-o out.png]
"""
import datetime
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

W, H = 1280, 580
BG = (26, 27, 33)
FG = (215, 217, 224)
DIM = (150, 154, 165)
RED = (232, 93, 108)
GRN = (120, 200, 140)
AMB = (231, 180, 92)
RULE = (58, 60, 70)

FONTS = ["C:/Windows/Fonts/consola.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"]
FONTS_B = ["C:/Windows/Fonts/consolab.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"]

FRAMES = ("boot", "walk", "combat")


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


def tone(rgb):
    """Name the ordering the way the gate reads it — not the way the doc hopes."""
    o = order(rgb)
    if o == "R>B>G":
        return "MAGENTA", RED
    if o == "B>G>R":
        return "blue", GRN
    if o == "R>G>B":
        return "amber", GRN
    return o.lower(), AMB


def colour_gates_pass(m):
    """Gates A / B / C on one measured frame, using colour_gate's own predicates."""
    ok = m["mag_frac"] <= colour_gate.MAGENTA_MAX_FRACTION
    if m["sky"] is not None:
        ok = ok and m["sky"][2] > m["sky"][1] > m["sky"][0]
        ok = ok and not colour_gate.sky_gain_is_magenta(m["sky_gain"])
    if m["sun"] is not None:
        ok = ok and not colour_gate.sunlit_is_magenta(m["sun"])
    return bool(ok)


def axes_pass(ax):
    """The gameplay profile of grade_axes.TARGETS — its table, not a copy of it."""
    active = grade_axes.PROFILES["gameplay"]
    return all(grade_axes.verdict(cmp_, bound, ax[key])
               for key, _, cmp_, bound, _ in grade_axes.TARGETS if key in active)


def stamp(path):
    t = datetime.datetime.fromtimestamp(os.path.getmtime(path))
    return f"{os.path.basename(path)} {t:%H:%M}"


def main():
    out = sys.argv[sys.argv.index("-o") + 1] if "-o" in sys.argv else OUT

    auth = colour_gate.authored_clearcolor()
    auth_rgb = tuple(c * 255.0 for c in auth)

    jul = colour_gate.measure(A("thirdperson-walk.png"))
    ref_sun = sunlit(A("golden-beauty-shot-ref.png"))

    frames = {k: colour_gate.measure(A("gate3", f"gate3-after-{k}.png")) for k in FRAMES}
    axes = {k: grade_axes.measure(A("gate3", f"gate3-after-{k}.png")) for k in FRAMES}
    ref_m = colour_gate.measure(A("golden-beauty-shot-ref.png"))
    base_m = colour_gate.measure(A("wide-hero-final.png"))
    now = frames["boot"]

    colour_ok = all(colour_gates_pass(m) for m in frames.values())
    axes_ok = all(axes_pass(a) for a in axes.values())

    if colour_ok and axes_ok:
        headline, hcol = "PASS — colour clean, P0 axes at target", GRN
    elif colour_ok:
        headline, hcol = "PASS colour / HOLD store page (P0 axes below target)", AMB
    else:
        headline, hcol = "FAIL (global magenta grade)", RED

    im = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(im)
    f_h1, f_lbl, f_txt, f_sm = font(23, True), font(15, True), font(14), font(13)

    d.text((24, 18), f"VOXELFORGE — GATE 3 COLOUR VERDICT: {headline}", hcol, f_h1)
    d.text((24, 54), f"Same authored ClearColor({auth[0]:g},{auth[1]:g},{auth[2]:g}) rendered by two builds. "
                     "Sky cannot be R>B>G under any lighting.", DIM, f_sm)

    cards = [
        ("AUTHORED\nClearColor\nmain.rs (parsed)", auth_rgb),
        ("JUL-31 BUILD\nthirdperson-walk\n(ungraded)", jul["sky"]),
        ("CURRENT BUILD\ngate3-after-boot\n(graded, on disk)", now["sky"]),
        ("GOLDEN REF\nsunlit patch\n(target look)", ref_sun),
    ]
    x, cw, gap = 34, 250, 62
    for i, (label, rgb) in enumerate(cards):
        cx = x + i * (cw + gap)
        d.rectangle([cx, 110, cx + cw, 258], fill=tuple(int(round(v)) for v in rgb))
        d.multiline_text((cx, 270), label, FG, f_lbl, spacing=4)
        d.text((cx, 348), f"RGB {rgb[0]:.0f},{rgb[1]:.0f},{rgb[2]:.0f}", DIM, f_sm)
        name, col = tone(rgb)
        d.text((cx, 367), f"{order(rgb)}  {name}", col, f_sm)
        if i in (1, 2):
            dv = max(abs(a_ - b_) for a_, b_ in zip(rgb, auth_rgb))
            d.text((cx + 124, 367), f"= {dv:.1f}/255", GRN if dv <= 25 else AMB, f_sm)

    d.line([24, 405, W - 24, 405], RULE, 1)

    gr, gg, gb = now["sky_gain"]
    fpct = lambda m, k: m[k] * 100.0
    red_push = colour_gate.sky_gain_is_magenta(now["sky_gain"])
    gcol = RED if red_push else GRN
    d.text((24, 419), "PER-CHANNEL LINEAR GAIN, authored sky -> current render:"
                      f"   R x{gr:.2f}   G x{gg:.2f}   B x{gb:.2f}", gcol, f_lbl)
    note = ("G and B pulled down by the SAME factor while R lifts = red-only push, not amber white-balance."
            if red_push else
            f"No red lift: R x{gr:.2f} stays under the sky's own falloff, so the clear survives as {order(now['sky'])}.")
    d.text((24, 442), note, DIM, f_sm)

    def row(y, head, cells, col):
        d.text((24, y), head, col, f_sm)
        d.text((24 + 300, y), cells, col, f_sm)

    b, w_, c = frames["boot"], frames["walk"], frames["combat"]
    ab, aw, ac = axes["boot"], axes["walk"], axes["combat"]
    row(471, "P0 axes (chromatic only)",
        f"blue B: {ab['blue']:.1f} / {aw['blue']:.1f} / {ac['blue']:.1f}  vs target <=10"
        f"      saturation: {ab['sat']:.0f} / {aw['sat']:.0f} / {ac['sat']:.0f}  vs target >=90",
        GRN if axes_ok else RED)
    warm_ok = min(fpct(m, "warm_frac_lit") for m in frames.values()) >= 80.0
    row(492, "Warm-ordered px (R>G>B, of lit)",
        f"boot {fpct(b, 'warm_frac_lit'):.1f}%  walk {fpct(w_, 'warm_frac_lit'):.1f}%"
        f"  combat {fpct(c, 'warm_frac_lit'):.1f}%"
        f"      golden ref {fpct(ref_m, 'warm_frac_lit'):.1f}%"
        f"   CEO baseline {fpct(base_m, 'warm_frac_lit'):.1f}%",
        GRN if warm_ok else RED)
    mag_ok = max(m["mag_frac"] for m in frames.values()) <= colour_gate.MAGENTA_MAX_FRACTION
    row(513, "Magenta px (G below BOTH R and B)",
        f"boot {fpct(b, 'mag_frac'):.2f}%  walk {fpct(w_, 'mag_frac'):.2f}%"
        f"  combat {fpct(c, 'mag_frac'):.2f}%"
        f"      gate A limit: {colour_gate.MAGENTA_MAX_FRACTION * 100:g}%"
        f"   golden ref {fpct(ref_m, 'mag_frac'):.2f}%",
        GRN if mag_ok else RED)

    d.text((24, 538), "Measured by scripts/colour_gate.py + scripts/grade_axes.py at card-render time"
                      " — verdict line, colours and numbers all derived, none typed by hand.", DIM, f_sm)
    d.text((24, 556), "frames measured: "
                      + "  ".join(stamp(A("gate3", f"gate3-after-{k}.png")) for k in FRAMES)
                      + f"   |  card rendered {datetime.datetime.now():%Y-%m-%d %H:%M}", DIM, f_sm)

    im.save(out)
    print(f"wrote {out}  — verdict: {headline}")


if __name__ == "__main__":
    main()
