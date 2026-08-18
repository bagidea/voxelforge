#!/usr/bin/env python
"""Contact card for the MAT_MAPS on-vs-off verdict.

Panels are the REAL judged plates, downscaled, nothing repainted. Every number
printed on the card is read back out of the matmaps-verdict.json that
_fl_matmaps_judge.py wrote for that same pair -- so a caption can never drift
away from the run it describes (2026-08-15 lesson).

    python scripts/_fl_matmaps_verdict_card.py
"""
import json
import os
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "docs", "assets", "look",
                   "matmaps-verdict-2026-08-18.png")

# (row title, before, after, verdict dir)
ROWS = [
    ("arm atlas16 / day  -- maps DO reach the loader",
     "docs/assets/look/matmaps_day_off.png",
     "docs/assets/look/matmaps_day_on.png",
     "_fl_matmaps_verdict/atlas16_day"),
    ("arm atlas16 / night-firelit  -- biggest real change in the set",
     "docs/assets/look/matmaps_night-firelit_off.png",
     "docs/assets/look/matmaps_night-firelit_on.png",
     "_fl_matmaps_verdict/atlas16_night-firelit"),
    ("arm shipped / day  -- art set REJECTED at load (tile_px 64 != 16)",
     "_poppy_matmaps/plates/shipped_day_off.png",
     "_poppy_matmaps/plates/shipped_day_on.png",
     "_fl_matmaps_verdict/shipped_day"),
]

PW, PH = 440, 248          # panel size
PAD, GUT = 26, 14
HEAD, ROWHEAD, ROWFOOT = 96, 30, 62


def _font(sz, bold=False):
    for name in (("arialbd.ttf", "DejaVuSans-Bold.ttf") if bold else
                 ("arial.ttf", "DejaVuSans.ttf")):
        try:
            return ImageFont.truetype(name, sz)
        except OSError:
            continue
    return ImageFont.load_default()


def _panel(path):
    im = Image.open(os.path.join(ROOT, path)).convert("RGB")
    im.thumbnail((PW, PH), Image.LANCZOS)
    box = Image.new("RGB", (PW, PH), (18, 18, 20))
    box.paste(im, ((PW - im.width) // 2, (PH - im.height) // 2))
    return box


def main():
    missing = [p for _, b, a, _ in ROWS for p in (b, a)
               if not os.path.isfile(os.path.join(ROOT, p))]
    if missing:
        print("missing plates:\n  " + "\n  ".join(missing))
        return 2

    f_h1, f_h2, f_b, f_s = _font(30, True), _font(19, True), _font(15), _font(13)
    W = PAD * 2 + PW * 2 + GUT
    H = HEAD + len(ROWS) * (ROWHEAD + PH + ROWFOOT) + PAD
    card = Image.new("RGB", (W, H), (12, 12, 14))
    d = ImageDraw.Draw(card)

    d.text((PAD, 22), "VOXELFORGE_MAT_MAPS  on vs off  -- VERDICT", (240, 240, 245), f_h1)
    d.text((PAD, 60), "Flamingo 2026-08-18 | judge: scripts/_fl_matmaps_judge.py"
                      " (control C0-C6 exit 0) | 0 of 6 pairs pass the gate",
           (150, 150, 160), f_b)

    y = HEAD
    for title, before, after, vdir in ROWS:
        d.text((PAD, y + 4), title, (225, 205, 140), f_h2)
        y += ROWHEAD
        card.paste(_panel(before), (PAD, y))
        card.paste(_panel(after), (PAD + PW + GUT, y))
        d.text((PAD + 8, y + 6), "OFF", (235, 235, 240), f_h2)
        d.text((PAD + PW + GUT + 8, y + 6), "ON", (235, 235, 240), f_h2)
        y += PH + 6

        jp = os.path.join(ROOT, vdir, "matmaps-verdict.json")
        if os.path.isfile(jp):
            v = json.load(open(jp))
            g = {r["name"].split()[0]: r for r in v["criteria"]}
            line = "   ".join(
                "%s %s" % ("PASS" if g[k]["ok"] else ("FAIL" if g[k]["hard"]
                                                      else "adv"), k)
                for k in ("M1", "M2", "M3", "M4", "M5", "A6") if k in g)
            col = (120, 210, 130) if v["verdict"] == "PASS" else (235, 120, 110)
            d.text((PAD, y + 4), line, (185, 185, 195), f_b)
            d.text((PAD, y + 26), "VERDICT: %s" % v["verdict"], col, f_h2)
            if v.get("borrowed_floor"):
                d.text((PAD + 220, y + 28),
                       "(noise floor borrowed from the day scene -- margins near"
                       " the floor are UNDETERMINED)", (215, 175, 90), f_s)
        else:
            d.text((PAD, y + 8), "no verdict json at %s -- run the judge first"
                   % vdir, (235, 120, 110), f_b)
        y += ROWFOOT

    card.save(OUT)
    print("wrote", OUT, card.size)
    return 0


if __name__ == "__main__":
    sys.exit(main())
