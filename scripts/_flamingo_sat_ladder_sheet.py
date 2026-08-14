#!/usr/bin/env python3
"""Flamingo — the saturation ladder as ONE picture (2026-08-05).

The P0 chromatic axes and the eye disagree on this scene, and a table of numbers
cannot show that. This lays the sweep rungs side by side with their own measured
axes printed under them, so "the row that passes is the row that looks wrong" is
something a reviewer sees rather than something they take on trust.

Numbers are measured at render time by scripts/grade_axes.py (the single source
of truth for the axes) — none is typed in.

FALSE-GREEN AUDIT 2026-08-14 — three things this file did wrong:

  1. One rung's caption was the typed-in string "ALL AXES PASS". A verdict typed
     into a caption is exactly the laundering this sheet exists to prevent, and
     it was stale on top of that (see 2). Captions now describe the LEVER only;
     every verdict on the sheet is measured.
  2. It stamped PASS/FAIL on warmth, blue and sat. Those three are
     `grade_axes.ADVISORY` — measured and printed for tuning, NEVER gated
     (docs/VERDICT-rose-chroma-rederive-2026-08-09.md §5). Advisory axes are now
     labelled `[ADV]`, the tile colour follows the one HARD axis (clip), and the
     ADVISORY set is imported rather than re-listed so it cannot drift.
  3. A missing rung PNG printed `missing ...`, was skipped, and the sheet was
     still written and still exited 0 — a 4-tile sheet posing as the 6-rung
     ladder. Missing input is now exit 1 (`--partial` to allow it explicitly).

The sheet still emits a picture, not a gate verdict (`nohud2_guard.py` records
that intent): exit 0 means the ladder was drawn in full, not that the look
passed.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import grade_axes  # noqa: E402
from PIL import Image, ImageDraw, ImageFont  # noqa: E402

SW = "_fl_grade_sweep"
# stem, caption -- the LEVER only. No caption may name a verdict: the tags under
# each tile are measured, and a typed-in one cannot notice the gate moved.
RUNGS = [
    ("boot-r00-ctrl", "sat 1.05  shipped lights\nSHIPPED DEFAULT"),
    ("boot-r32-s135G", "sat 1.35  G-lift lights\nSHIPPED NOW (this change)"),
    ("boot-r20-s145G", "sat 1.45  G-lift lights"),
    ("boot-r21-s175G", "sat 1.75  G-lift lights"),
    ("boot-r05-s190L", "sat 1.90  B-drained lights\n(2026-08-01 prescription)"),
    ("boot-r06-s230L", "sat 2.30  B-drained lights"),
]
COLS, TW = 3, 620
PAD, CAPH = 14, 96


def font(sz, bold=False):
    for name in (("arialbd.ttf", "arial.ttf") if bold else ("arial.ttf",)):
        try:
            return ImageFont.truetype(name, sz)
        except OSError:
            pass
    return ImageFont.load_default()


def main():
    partial_ok = "--partial" in sys.argv
    tiles, missing = [], []
    for stem, cap in RUNGS:
        p = f"{SW}/{stem}-nohud2.png"
        if not os.path.exists(p):
            print(f"missing {p}", file=sys.stderr)
            missing.append(p)
            continue
        m = grade_axes.measure(p)
        im = Image.open(p).convert("RGB")
        th = round(TW * im.height / im.width)
        tiles.append((im.resize((TW, th), Image.LANCZOS), cap, m))
    if not tiles:
        print("no tiles — nothing to draw", file=sys.stderr)
        return 1
    if missing and not partial_ok:
        print(f"REFUSED: {len(missing)}/{len(RUNGS)} rung(s) have no PNG, so this "
              f"would be a {len(tiles)}-tile sheet captioned as the {len(RUNGS)}-rung "
              f"ladder. Re-run the sweep, or pass --partial to say so on purpose.",
              file=sys.stderr)
        return 1

    th = tiles[0][0].height
    rows = (len(tiles) + COLS - 1) // COLS
    W = COLS * TW + (COLS + 1) * PAD
    H = rows * (th + CAPH) + (rows + 1) * PAD + 52
    sheet = Image.new("RGB", (W, H), (22, 22, 26))
    d = ImageDraw.Draw(sheet)
    d.text((PAD, 14), "Voxelforge — POST_SATURATION ladder, --play boot frame, one binary "
                      "(2026-08-05, Flamingo).  HARD gate: clip <=35%.  [ADV] advisory, "
                      "never gated: warmth >=110 · blue <=10 · sat >=90 (honest; N-A if clip > 35%)",
           font=font(19, True), fill=(235, 235, 240))

    for i, (im, cap, m) in enumerate(tiles):
        r, c = divmod(i, COLS)
        x = PAD + c * (TW + PAD)
        y = 52 + PAD + r * (th + CAPH + PAD)
        sheet.paste(im, (x, y))
        d.rectangle([x, y, x + TW - 1, y + th - 1], outline=(70, 70, 78))
        ty = y + th + 6
        for line in cap.split("\n"):
            d.text((x + 4, ty), line, font=font(17, True), fill=(240, 240, 245))
            ty += 21
        sat_ok, sat_txt = grade_axes.sat_status(m)   # honest sat, or N-A if clip co-gate fired
        clip_ok = m["clip"] <= grade_axes.CLIP_THRESH
        # warmth/blue/sat are grade_axes.ADVISORY -- printed for tuning, never a
        # verdict. `clip` is the one hard axis on this sheet, so it alone colours
        # the row; imported from the module so the set cannot drift from the gate.
        assert {"warmth", "blue", "sat"} == grade_axes.ADVISORY, \
            f"grade_axes.ADVISORY moved to {grade_axes.ADVISORY} — re-label this sheet"
        sat_tag = "N-A" if sat_ok is None else f"{'ok' if sat_ok else 'off-target'} [ADV]"
        d.text((x + 4, ty + 2),
               f"warmth {m['warmth']:6.1f} [ADV]   "
               f"blue {m['blue']:5.1f} [ADV]   "
               f"clip {m['clip']:5.1f}% {'PASS' if clip_ok else 'FAIL'}   "
               f"sat {sat_txt:>5} {sat_tag}",
               font=font(16), fill=(120, 230, 140) if clip_ok else (235, 175, 110))

    out = "docs/assets/look-2026-08-05/sat-ladder.png"
    os.makedirs(os.path.dirname(out), exist_ok=True)
    sheet.save(out)
    print(out, sheet.size)
    if missing:
        print(f"PARTIAL: drew {len(tiles)}/{len(RUNGS)} rungs "
              f"(--partial was given) -> exit 1")
        return 1
    print(f"drew {len(tiles)}/{len(RUNGS)} rungs -> exit 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
