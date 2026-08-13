#!/usr/bin/env python3
"""Instrument control for the A6.2 accent scanner — prove a 0-px reading means
the ART is missing, not that the SCANNER is blind.

The negative control (`_flamingo_a62_accent_presence.py` on every pre-fix frame)
reads 0 px, 3/3. On its own that is ambiguous: a scanner that returned 0 on
everything would print exactly the same table. So before that 0 is allowed to
carry a verdict, plant a KNOWN accent into the very same frames, at the very same
screen coordinate the clasp is predicted to land on, and require the scanner to
recover it.

The coordinate is not chosen for convenience: **(1337, 842)** on the 2560x1360
de-HUDded boot frame is where `docs/VERDICT-a6-teal-accent-2026-08-11.md` projected
the clasp's torso-local point through `grade_character.py`'s own camera math, and
then cropped at 5x to find flat, featureless cloak colour. Planting there is what
makes this a control on the 08-11 finding rather than a generic self-test.

Two rungs, because a lit gem is not a flat swatch:
  * full value  #4FC9D6  — the nominal accent
  * 45 % value           — a gem sitting in the cloak's shade. This is the rung
                           that matters: the L>=20 degeneracy floor must not eat a
                           legitimately shaded gem. #4FC9D6 only falls under L 20
                           below ~11.4 % of full value, so 45 % keeps >4x headroom
                           and MUST survive.

PASS for this control = both rungs recover the planted pixel count exactly, with a
bbox centred on the plant site and a minor axis that clears the read floor.

Usage:
    python scripts/_flamingo_a62_synth_control.py FRAME [--x X --y Y] [--size N]
"""
import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _flamingo_a62_accent_presence import GEM_SRGB, L_MIN, MIN_AXIS_PX, measure

# See module docstring — the 08-11 pixel-projection site, not an arbitrary spot.
PRED_X, PRED_Y = 1337, 842


def plant(src, dst, x, y, size, value):
    """Paint a `size`x`size` #4FC9D6 patch at `value` scale centred on (x, y)."""
    im = Image.open(src).convert("RGB")
    a = np.asarray(im, dtype=np.uint8).copy()
    h, w = a.shape[:2]
    rgb = np.array([round(c * 255.0 * value) for c in GEM_SRGB], dtype=np.uint8)
    half = size // 2
    x0, x1 = max(0, x - half), min(w, x - half + size)
    y0, y1 = max(0, y - half), min(h, y - half + size)
    a[y0:y1, x0:x1] = rgb
    Image.fromarray(a).save(dst)
    L = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
    return (x1 - x0) * (y1 - y0), tuple(int(v) for v in rgb), float(L)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("frame")
    ap.add_argument("--x", type=int, default=PRED_X)
    ap.add_argument("--y", type=int, default=PRED_Y)
    ap.add_argument("--size", type=int, default=16)
    ap.add_argument("--outdir", default="_fl_a62_control")
    args = ap.parse_args()

    out = Path(args.outdir)
    out.mkdir(parents=True, exist_ok=True)

    print(f"instrument control — plant #4FC9D6 at ({args.x},{args.y}) "
          f"[the 08-11 pixel-projection site] size {args.size}x{args.size}")
    print(f"source frame: {args.frame}")
    print("-" * 96)

    # rung 0: the untouched frame. Must read 0 — otherwise the plant proves nothing.
    base = measure(args.frame, "-charmask.png")
    print(f"{'rung':>10}  {'planted':>8}  {'recovered':>9}  {'bbox':>9}  "
          f"{'centre':>13}  {'L':>6}  verdict")
    print(f"{'unplanted':>10}  {'-':>8}  {base['px']:9d}  {'-':>9}  {'-':>13}  "
          f"{'-':>6}  " + ("clean baseline" if base["px"] == 0
                           else "!! frame already has accent — control void"))

    ok = base["px"] == 0
    for label, value in (("full", 1.00), ("45% value", 0.45)):
        # NOTE: plant into a *-nohud2.png name so the graders' de-HUD guard and the
        # scanner's charmask lookup behave exactly as they do on a real plate.
        dst = out / f"synth-{label.split('%')[0].strip().replace(' ', '')}-nohud2.png"
        n, rgb, L = plant(args.frame, dst, args.x, args.y, args.size, value)
        r = measure(str(dst), "-charmask.png")
        top = r["top"][0] if r["top"] else None
        exact = top is not None and r["px"] == n
        centred = top is not None and abs(top["cx"] - args.x) <= 1 and abs(top["cy"] - args.y) <= 1
        reads = top is not None and top["minor"] >= MIN_AXIS_PX
        good = exact and centred and reads and L >= L_MIN
        ok = ok and good
        bbox = f"{top['w']}x{top['h']}" if top else "-"
        ctr = f"({top['cx']},{top['cy']})" if top else "-"
        print(f"{label:>10}  {n:8d}  {r['px']:9d}  {bbox:>9}  {ctr:>13}  {L:6.1f}  "
              + ("RECOVERED" if good else "LOST — scanner is blind here")
              + (f"  rgb{rgb}" if top else ""))

    print("-" * 96)
    print("INSTRUMENT CONTROL: " + ("PASS — a 0-px reading means the art is missing, "
                                    "not the scanner blind"
                                    if ok else "FAIL — do not trust any 0-px reading"))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
