#!/usr/bin/env python3
"""PER-PLATE evidence sheet — 1.02 (shipped) vs 1.05 (rejected), all four plates.

The previous ladder sheet showed six rungs of ONE plate, which is exactly the
mistake the board caught: a single plate presented as the set. This sheet is the
other axis. Four rows = four plates; two columns = the two rungs actually in
dispute; every caption is measured by scripts/grade_axes.py at render time, and
each cell names its own plate.

Read it left-to-right: the cell that changes from PASS to FAIL is `hero`'s clip,
and once that co-gate fires its honest-sat cell goes N-A -- the plate the board
judges on stops being readable. That is the whole argument against 1.05.

FALSE-GREEN AUDIT 2026-08-14 — two fixes:

  * `warmth` and `sat` were stamped PASS/FAIL. Both are `grade_axes.ADVISORY`:
    measured and printed for tuning, NEVER gated
    (docs/VERDICT-rose-chroma-rederive-2026-08-09.md §5). They now read `[ADV]`,
    and `clip` -- the one hard axis -- carries the cell colour. The ADVISORY set
    is imported, not re-listed, so this sheet cannot drift from the gate.
  * `main()` returned None, i.e. exit 0, including when a plate could not be
    measured. It now returns an explicit code:

        0 = all 8 cells measured and the sheet was written
        1 = a plate could not be read (nothing is written)

    A FAIL *cell* is not an error here: the sheet exists to show that 1.05 fails
    on `hero`, so a red exit on that would make the artefact impossible to
    produce. This file emits a picture, not a gate verdict.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import grade_axes as G  # noqa: E402
from PIL import Image, ImageDraw, ImageFont  # noqa: E402

PLATES = ["grade-vista", "hero", "s1-vista", "s4-raking"]
COLS = [("1.02", "_yama_sat_sweep_102", "SHIPPED  (look.rs:578)"),
        ("1.05", "_yama_sat_sweep_105", "PROPOSED  -  NOT APPROVED")]

TW, PAD, CAPH, HEAD = 560, 16, 118, 122
INK, DIM = (232, 234, 240), (150, 156, 170)
OKC, BADC, NAC = (120, 210, 140), (240, 110, 110), (240, 190, 110)
BG = (17, 18, 22)


def font(sz, bold=False):
    for name in (("arialbd.ttf", "arial.ttf") if bold else ("arial.ttf",)):
        try:
            return ImageFont.truetype(name, sz)
        except OSError:
            pass
    return ImageFont.load_default()


def main():
    assert {"warmth", "blue", "sat"} == G.ADVISORY, \
        f"grade_axes.ADVISORY moved to {G.ADVISORY} — re-label this sheet"
    cells = {}
    for p in PLATES:
        for rung, d, _sub in COLS:
            path = f"{d}/after/{p}-nohud2.png"
            if not os.path.isfile(path):
                print(f"MISSING {path} — refusing to write a sheet that is "
                      f"missing a plate", file=sys.stderr)
                return 1
            m = G.measure(path)
            im = Image.open(path).convert("RGB")
            im = im.resize((TW, round(TW * im.height / im.width)), Image.LANCZOS)
            cells[(p, rung)] = (im, m)

    th = max(im.height for im, _ in cells.values())
    W = len(COLS) * TW + (len(COLS) + 1) * PAD + 150
    H = HEAD + len(PLATES) * (th + CAPH) + (len(PLATES) + 1) * PAD
    sheet = Image.new("RGB", (W, H), BG)
    dr = ImageDraw.Draw(sheet)

    f_title, f_col, f_lbl, f_num, f_sm = font(30, True), font(21, True), font(20, True), font(19), font(15)

    dr.text((PAD, 16), "POST_SATURATION 1.02 vs 1.05 - PER PLATE (no plate speaks for another)",
            font=f_title, fill=INK)
    dr.text((PAD, 54), "measured live by scripts/grade_axes.py, profile gameplay - "
                       "source _yama_sat_sweep_{102,105}/after/*-nohud2.png",
            font=f_sm, fill=DIM)

    for ci, (rung, _d, sub) in enumerate(COLS):
        x = PAD + 150 + ci * (TW + PAD)
        dr.text((x, 84), f"sat {rung}", font=f_col, fill=INK)
        dr.text((x + 104, 88), sub, font=f_sm, fill=OKC if ci == 0 else BADC)

    for ri, p in enumerate(PLATES):
        y = HEAD + ri * (th + CAPH + PAD)
        dr.text((PAD, y + th // 2 - 24), p.replace("-", "\n"), font=f_lbl, fill=INK)
        for ci, (rung, _d, _s) in enumerate(COLS):
            x = PAD + 150 + ci * (TW + PAD)
            im, m = cells[(p, rung)]
            sheet.paste(im, (x, y))
            sok, stxt = G.sat_status(m)
            # (label, value, ok, hard?) -- only `clip` is a gate; see the docstring.
            parts = [("warmth", f"{m['warmth']:.1f}", m["warmth"] >= 110.0, False),
                     ("clip", f"{m['clip']:.1f}%", m["clip"] <= G.CLIP_THRESH, True),
                     ("sat", stxt, sok, False)]
            ty = y + im.height + 8
            for label, val, ok, hard in parts:
                col = NAC if ok is None else (OKC if ok else BADC)
                # N-A is not a third verdict -- it means the co-gate fired and this
                # axis can no longer be read on this plate. Say that, don't print "N-A N-A".
                if ok is None:
                    tag = "UNREADABLE (clip>35)"
                elif hard:
                    tag = "PASS" if ok else "FAIL"
                else:
                    # advisory: report the direction, never the word PASS/FAIL
                    tag = f"{'on' if ok else 'off'}-target [ADV]"
                    col = DIM if ok else NAC
                dr.text((x, ty), f"{label:<7}{val:>8}  {tag}", font=f_num, fill=col)
                ty += 25
            dr.text((x + 300, y + im.height + 8),
                    f"blue {m['blue']:.1f}\nmicro {m['micro']:.2f}\np95 {m['p95']:.1f}",
                    font=f_sm, fill=DIM)

    out = "docs/assets/flamingo-perplate-102-vs-105-2026-08-09.png"
    sheet.save(out)
    print(f"[written] {out}  ({sheet.width}x{sheet.height})")
    print(f"drew {len(cells)}/{len(PLATES) * len(COLS)} cells -> exit 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
