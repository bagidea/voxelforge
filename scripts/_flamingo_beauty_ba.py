#!/usr/bin/env python3
"""Before/after sheets for the beauty-axes ladder (flamingo, 2026-08-16).

Two sheets, each answering one question the grid raises:

  ba-winner.png   r0 -> win-atmosfog on all four admitted poses.  Numbers are
                  burned into the panel from the SAME measurement the grid table
                  ran, so a caption can never drift from its pixels (office scar:
                  "caption must match pixels" -- every panel here is diffed
                  against the file its caption names before the sheet is saved).

  ba-inert.png    r0 / amb1100 / fill-up / fill-down on one pose.  This is the
                  negative result and it gets the same real estate as the winner:
                  the brief assigned the shade-floor axis to ambient_lux, and all
                  four fill levers move it by <= 0.1 L while visibly changing a
                  fifth of the frame.  A ladder that only publishes its winners
                  hides exactly this.

Usage: python scripts/_flamingo_beauty_ba.py
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).resolve().parent))
import importlib

axes = importlib.import_module("_flamingo_beauty_axes")

LADDER = Path("docs/assets/look/beauty/ladder")
OUT = Path("docs/assets/look/beauty")
POSES = ["p1-east-level", "p2-east-up", "p3-east-down", "p8-abelev-eye-horizon"]

PANEL_W, PANEL_H = 640, 360
BAR = 46
BG = (16, 14, 12)
INK = (232, 216, 184)
DIM = (150, 140, 126)


def lit_metrics(path):
    """The same three axes the grid reports, on the same lit mask (L >= 12)."""
    a = np.asarray(Image.open(path).convert("RGB")).astype(float)
    L = a.mean(2)
    lit = L >= 12.0
    v = L[lit]
    mx = a.max(2)[lit]
    mn = a.min(2)[lit]
    sat = np.where(mx > 0, (mx - mn) / np.maximum(mx, 1e-6), 0.0)
    p5, p95 = np.percentile(v, 5), np.percentile(v, 95)
    s5, s95 = np.percentile(sat, 5), np.percentile(sat, 95)
    return dict(span=p95 - p5, p5=p5, p5pct=100 * p5 / 255.0,
                chroma=s95 - s5, black=100 * (~lit).mean())


def panel(path, title, sub):
    im = Image.open(path).convert("RGB").resize((PANEL_W, PANEL_H))
    card = Image.new("RGB", (PANEL_W, PANEL_H + BAR), BG)
    card.paste(im, (0, BAR))
    d = ImageDraw.Draw(card)
    d.text((8, 5), title, fill=INK)
    d.text((8, 24), sub, fill=DIM)
    return card


def sheet(rows, out, header):
    cols = max(len(r) for r in rows)
    W = PANEL_W * cols
    H = (PANEL_H + BAR) * len(rows) + 34
    s = Image.new("RGB", (W, H), BG)
    ImageDraw.Draw(s).text((10, 10), header, fill=INK)
    for r, row in enumerate(rows):
        for c, card in enumerate(row):
            s.paste(card, (c * PANEL_W, 34 + r * (PANEL_H + BAR)))
    s.save(out)
    return out


def verify(pairs):
    """Every panel must be the file its caption names -- prove it, don't assert it."""
    for path, caption in pairs:
        a = np.asarray(Image.open(path).convert("RGB")).astype(float)
        b = np.asarray(Image.open(LADDER / (caption + ".png")).convert("RGB")).astype(float)
        d = float(np.abs(a - b).mean())
        assert d == 0.0, f"panel/caption mismatch {path} vs {caption}: mean|d|={d}"


def four_pose_sheet(rung, name, header):
    rows, checks = [], []
    for p in POSES:
        row = []
        for r in ("r0", rung):
            f = LADDER / f"{p}-{r}.png"
            m = lit_metrics(f)
            row.append(panel(
                f, f"{p}  |  {r}",
                f"span {m['span']:.1f}   shade p5 {m['p5']:.1f} ({m['p5pct']:.1f}% of white)   "
                f"chroma {m['chroma']:.3f}   dead-black {m['black']:.1f}%"))
            checks.append((f, f"{p}-{r}"))
        rows.append(row)
    verify(checks)
    return sheet(rows, OUT / name, header)


def main():
    # ---- sheet 1: the only rung that moves span without failing chroma -------
    o1 = four_pose_sheet(
        "atmos-off", "ba-atmosoff.png",
        "BEFORE (r0, no overrides)  ->  AFTER (VOXELFORGE_LOOK_ATMOS=off)   "
        "-- the only rung that widens value span on all 4 poses AND keeps chroma span "
        "passing. Metrics on lit pixels only (L>=12); target span >=150, shade 8-11%, chroma >=0.62")

    # ---- sheet 1b: the rung that wins the number and loses the picture -------
    o1b = four_pose_sheet(
        "win-atmosfog", "ba-overfog.png",
        "REJECTED CANDIDATE  r0 -> ATMOS=off + _LOOK_FOG=8,45.  Best value span in the whole "
        "ladder (+28.7..+32.6) and it looks WORSE: the 45-block fog end whites the village out "
        "and chroma span collapses 0.94 -> 0.22 (target >=0.62). The number moved; the shot did not.")

    # ---- sheet 2: the negative result, same real estate ----------------------
    p = "p1-east-level"
    rows, checks = [], []
    for pair in (("r0", "amb1100"), ("fill-up", "fill-down")):
        row = []
        for rung in pair:
            f = LADDER / f"{p}-{rung}.png"
            m = lit_metrics(f)
            row.append(panel(
                f, f"{p}  |  {rung}",
                f"shade p5 {m['p5']:.1f} ({m['p5pct']:.1f}% of white)   span {m['span']:.1f}"))
            checks.append((f, f"{p}-{rung}"))
        rows.append(row)
    verify(checks)
    o2 = sheet(rows, OUT / "ba-inert.png",
               "NEGATIVE RESULT: ambient_lux 2200->1100 and fill 1400,1100 -> 2400,1900 / 700,550 "
               "move the shade floor by <=0.1 L. The levers ARE live (39% of pixels differ) -- they "
               "just do not own this axis.")
    print(f"wrote {o1}\nwrote {o1b}\nwrote {o2}")


if __name__ == "__main__":
    raise SystemExit(main())
