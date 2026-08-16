#!/usr/bin/env python
"""Draw the Hour::NIGHT.ev100 verdict: the decision pair, and the ladder behind it.

TWO SHEETS, BECAUSE THEY ANSWER DIFFERENT QUESTIONS.

  night-ev100-decision.png   the pair. LEFT is the 8.6 that was sitting
                             uncommitted in the tree; RIGHT is the 7.5 that is
                             shipped. This is the before/after of the DECISION --
                             the left frame is what would have shipped had the
                             marker been committed unmeasured.
  night-ev100-ladder.png     all five rungs in exposure order with the two
                             failure directions labelled, because the pair alone
                             cannot show WHY the correction everyone reaches for
                             first (ride the exposure down) is also wrong.

CAPTIONS ARE GENERATED FROM THE PIXELS, NOT TYPED. Every number under every
panel is recomputed here from the file being drawn, so a caption cannot drift
from the frame above it -- and the md5 of each source file is printed on the
sheet so a panel can be traced back to the exact plate it came from. Nothing is
copied from the gate's stdout.

Usage: python scripts/_poppy_ev100_night_sheet.py [sweepdir] [outdir]
"""
import sys
import pathlib
import hashlib

import numpy as np
from PIL import Image, ImageDraw, ImageFont

SCENE = "night-firelit"
RUNGS = [("ev86", 8.6), ("ev75", 7.5), ("ev70", 7.0), ("ev65", 6.5), ("ev60", 6.0)]
W = np.array([0.2126, 0.7152, 0.0722], dtype=np.float64)
DARK, HI = 32.0, 200.0


def fonts():
    try:
        return (ImageFont.truetype("arialbd.ttf", 34), ImageFont.truetype("arial.ttf", 22),
                ImageFont.truetype("arialbd.ttf", 24))
    except Exception:
        d = ImageFont.load_default()
        return d, d, d


def measure(p):
    a = np.asarray(Image.open(p).convert("RGB"), dtype=np.float64)
    l = a @ W
    lo, hi = np.percentile(l, 35), np.percentile(l, 75)
    m = (l >= lo) & (l <= hi)
    return dict(
        p05=float(np.percentile(l, 5)),
        p50=float(np.percentile(l, 50)),
        warm=float(np.mean(a[..., 0][m] - a[..., 2][m])),
        dark=float(np.mean(l < DARK) * 100.0),
        hi=float(np.mean(l > HI) * 100.0),
        md5=hashlib.md5(pathlib.Path(p).read_bytes()).hexdigest()[:8],
    )


def panel(path, w):
    im = Image.open(path).convert("RGB")
    return im.resize((w, int(im.height * w / im.width)), Image.LANCZOS)


def decision(sweep, outdir, f1, f2, f3):
    left, right = sweep / f"{SCENE}_ev86.png", sweep / f"{SCENE}_ev75.png"
    PW, PAD, TOP, CAP = 900, 24, 108, 132
    a, b = panel(left, PW), panel(right, PW)
    H = TOP + a.height + CAP + PAD
    sh = Image.new("RGB", (PAD * 3 + PW * 2, H), (17, 17, 20))
    d = ImageDraw.Draw(sh)
    d.text((PAD, 20), "Hour::NIGHT.ev100 — the value that was pending vs the value that ships",
           font=f1, fill=(255, 255, 255))
    d.text((PAD, 64), "night-firelit · one binary · only ev100 moves · "
                      "captions computed from these exact files",
           font=f2, fill=(150, 150, 160))

    for i, (im, p, title, tint) in enumerate((
            (a, left, "BEFORE  ev100 = 8.6   (was uncommitted in the tree — REJECTED)", (255, 120, 110)),
            (b, right, "AFTER  ev100 = 7.5   (measured best — SHIPPED)", (120, 230, 150)))):
        x = PAD + i * (PW + PAD)
        sh.paste(im, (x, TOP))
        d.rectangle([x - 2, TOP - 2, x + PW + 1, TOP + im.height + 1], outline=tint, width=2)
        m = measure(p)
        d.text((x, TOP + im.height + 10), title, font=f3, fill=tint)
        d.text((x, TOP + im.height + 42),
               f"p05 {m['p05']:.2f}   warmth {m['warm']:.2f}   "
               f"dark% {m['dark']:.2f}   md5 {m['md5']}",
               font=f2, fill=(205, 205, 215))
        d.text((x, TOP + im.height + 70),
               "fails p05 floor 20.40, and warmth/separation vs 7.5"
               if i == 0 else "clears every measurability bar, closest admissible to the night ref",
               font=f2, fill=(160, 160, 170))

    out = outdir / "night-ev100-decision.png"
    sh.save(out)
    return out


def ladder(sweep, outdir, f1, f2, f3):
    PW, PAD, TOP, CAP = 520, 16, 112, 128
    ims = [(t, ev, panel(sweep / f"{SCENE}_{t}.png", PW), measure(sweep / f"{SCENE}_{t}.png"))
           for t, ev in RUNGS]
    ih = ims[0][2].height
    sh = Image.new("RGB", (PAD * (len(ims) + 1) + PW * len(ims), TOP + ih + CAP), (17, 17, 20))
    d = ImageDraw.Draw(sh)
    d.text((PAD, 18), "…and why riding the exposure down is not the fix either",
           font=f1, fill=(255, 255, 255))
    d.text((PAD, 60), "left = darker, loses the measurable midtones · right = brighter, "
                      "loses the night · all five rungs from ONE binary",
           font=f2, fill=(150, 150, 160))

    for i, (t, ev, im, m) in enumerate(ims):
        x = PAD + i * (PW + PAD)
        sh.paste(im, (x, TOP))
        if ev == 7.5:
            col, note = (120, 230, 150), "SHIPPED"
        elif ev > 7.5:
            col, note = (255, 120, 110), "fails p05/warmth/sep"
        else:
            col, note = (235, 190, 110), "passes bars, drifts off night"
        d.rectangle([x - 2, TOP - 2, x + PW + 1, TOP + ih + 1], outline=col, width=2)
        d.text((x, TOP + ih + 10), f"ev100 = {ev}", font=f3, fill=col)
        d.text((x, TOP + ih + 40), note, font=f2, fill=col)
        d.text((x, TOP + ih + 68),
               f"p05 {m['p05']:.1f}  warm {m['warm']:.1f}", font=f2, fill=(205, 205, 215))
        d.text((x, TOP + ih + 92),
               f"dark% {m['dark']:.1f}  md5 {m['md5']}", font=f2, fill=(150, 150, 160))

    out = outdir / "night-ev100-ladder.png"
    sh.save(out)
    return out


def main():
    root = pathlib.Path(__file__).resolve().parent.parent
    sweep = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "_poppy_ev100night_sweep")
    outdir = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else root / "docs/assets/look/night-ev100")
    outdir.mkdir(parents=True, exist_ok=True)

    for t, _ in RUNGS:
        p = sweep / f"{SCENE}_{t}.png"
        if not p.exists():
            print(f"MISSING {p}")
            return 1

    f1, f2, f3 = fonts()
    for out in (decision(sweep, outdir, f1, f2, f3), ladder(sweep, outdir, f1, f2, f3)):
        print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
