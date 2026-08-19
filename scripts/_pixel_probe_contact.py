#!/usr/bin/env python3
"""Contact sheet of the 8 pose-probe plates — `probe/_contact.png`, remade.

The original `_contact.png` was destroyed with the plates and, unlike `_grid.md`,
it has NO surviving source: no `Write` tool call carries it and no script in
`scripts/` names it, so how it was composed is unknown.  This is therefore a
REMAKE, not a recovery, and it says so on the sheet itself.

What it must still be able to show is the one thing the ladder doc cites it for
(`docs/flamingo-beauty-ladder-2026-08-16.md` §1): that above the horizon these
frames are black — a sun dot and no dome — which is why the picker admitted 0 of
8 poses on a >= 3%-of-frame sky rule.  So every panel is captioned with its OWN
measured sky mask, read off that panel's file at composite time by the same
grader the picker used, and the admitted/rejected verdict is printed per plate
with the reason from `_poses.txt`.

Usage:
  python scripts/_pixel_probe_contact.py [--dir docs/assets/look/beauty/probe]
                                         [--out .../probe/_contact.png]
"""
import argparse
import importlib.util
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "beauty_axes", HERE / "_flamingo_beauty_axes.py")
axes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(axes)

SKY_MIN_PCT = 3.0          # picker's admission rule (_flamingo_beauty_pick.py:29)
HORIZON_MIN_COLS = 100

# The written refusals, quoted from docs/assets/look/beauty/_poses.txt so the
# sheet cannot read as "8 shot, 4 worked".
REFUSED = {
    "p4-zaxis-level": "eye (32,7,58) clips inside a block",
    "p5-zaxis-up": "right half is one out-of-focus face",
    "p6-west-level": "eye (8,6,10) is INSIDE a building; no vista",
    "p7-west-up": "eye (8,6,10) is INSIDE a building; no vista",
}

TILE_W, TILE_H = 620, 349
CAP_H = 62
PAD = 12
COLS = 4
HEADER_H = 84
FOOTER_H = 46
BG = (16, 14, 12)
INK = (232, 216, 184)
DIM = (150, 140, 126)
BAD = (214, 122, 96)
RULE = (58, 54, 48)


def font(size, bold=False):
    for p in (["C:/Windows/Fonts/consolab.ttf"] if bold else ["C:/Windows/Fonts/consola.ttf"]):
        if Path(p).exists():
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", default="docs/assets/look/beauty/probe")
    ap.add_argument("--out", default="docs/assets/look/beauty/probe/_contact.png")
    ap.add_argument("--note", default="")
    a = ap.parse_args()

    plates = sorted(p for p in Path(a.dir).glob("*.png") if not p.stem.startswith("_"))
    if not plates:
        print(f"REFUSED: no probe plates in {a.dir}")
        return 2

    rows = (len(plates) + COLS - 1) // COLS
    W = PAD + COLS * (TILE_W + PAD)
    H = HEADER_H + rows * (TILE_H + CAP_H + PAD) + FOOTER_H
    im = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(im)
    f_h1, f_lbl, f_sm = font(24, True), font(16, True), font(13)

    d.text((PAD, 14), "POSE PROBE — 8 plates, edhari village   "
                      "admission rule: sky mask >= 3.00% of frame", INK, f_h1)
    d.text((PAD, 46), "Sky % below each panel is measured off THAT panel's own file at composite time "
                      "(_flamingo_beauty_axes.grade), not copied from a log.", DIM, f_sm)
    d.text((PAD, 62), a.note or "REMADE 2026-08-17: the original _contact.png had no surviving "
                                "generator; this sheet is a rebuild, not a byte-recovery.", BAD, f_sm)

    admitted = 0
    for i, p in enumerate(plates):
        r = axes.grade(str(p), None, 0, p.stem)
        sky = r["sky_px_pct"]
        span = r["structure"]["L_p95_minus_p5"]
        hz = (r.get("horizon") or {}).get("columns")
        why = []
        if sky < SKY_MIN_PCT:
            why.append(f"sky {sky:.2f}% < {SKY_MIN_PCT}%")
        if (hz or 0) < HORIZON_MIN_COLS:
            why.append(f"horizon {hz} cols")
        if p.stem in REFUSED:
            why.append(REFUSED[p.stem])
        if not why:
            admitted += 1

        col, row = i % COLS, i // COLS
        x = PAD + col * (TILE_W + PAD)
        y = HEADER_H + row * (TILE_H + CAP_H + PAD)
        im.paste(Image.open(p).convert("RGB").resize((TILE_W, TILE_H), Image.LANCZOS), (x, y))
        d.rectangle([x, y, x + TILE_W - 1, y + TILE_H - 1], outline=RULE, width=2)
        cy = y + TILE_H + 5
        d.text((x, cy), p.stem, INK, f_lbl)
        d.text((x, cy + 20), f"sky {sky:.2f}%   value span {span:.1f}   horizon cols {hz}",
               DIM, f_sm)
        d.text((x, cy + 38), ("ADMITTED" if not why else "REJECT: " + "; ".join(why))[:78],
               INK if not why else BAD, f_sm)

    d.line([PAD, H - FOOTER_H - 6, W - PAD, H - FOOTER_H - 6], RULE, 1)
    d.text((PAD, H - FOOTER_H + 4),
           f"{admitted} of {len(plates)} poses admitted by the 3% sky rule.  "
           f"A 0.00% sky mask is the dome not rendering, not the camera aiming low "
           f"(docs/flamingo-beauty-ladder-2026-08-16.md section 1).", DIM, f_sm)
    d.text((PAD, H - FOOTER_H + 22),
           f"panels: {', '.join(p.name for p in plates)}", DIM, f_sm)

    Path(a.out).parent.mkdir(parents=True, exist_ok=True)
    im.save(a.out)
    print(f"wrote {a.out}  ({W}x{H})  admitted {admitted}/{len(plates)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
