#!/usr/bin/env python3
"""Where does the before/after difference actually live?

`_poppy_sky_after_plate.py`'s angle guard came back at 28.6 / 13.9 levels — far
past the 3.0 it treats as "the capture did not move". Two readings are possible
and they point opposite ways:

  * the capture really moved, and no sky comparison in the plate means anything;
  * or the guard's "non-sky" set is not terrain. The sky mask kills everything
    below 45 % of frame height, so any sky BELOW that line is scored as guard
    territory — and that sky is exactly what the fix was supposed to change.

So: split the diff by row band, and re-measure the guard over a band that can
only be ground. Prints numbers, writes an amplified diff map to look at.

    python scripts/_poppy_sky_drift_probe.py
"""
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
D = ROOT / "docs" / "assets"
OUT = D / "_poppy_sky_driftmap_2026-08-14.png"
HUD_ROWS = 46
FRAMES = ["playable-walk-after.png", "edhari-load.png"]

panels = []
for name in FRAMES:
    b = np.asarray(Image.open(D / f"_poppy_sky_before_{name}").convert("RGB")).astype(np.int16)
    a = np.asarray(Image.open(D / name).convert("RGB")).astype(np.int16)
    h = b.shape[0]
    diff = np.abs(b - a).mean(axis=2)
    diff[:HUD_ROWS, :] = 0.0

    print(f"=== {name}  {b.shape[1]}x{h} ===")
    print("  row band          mean|d|   %px >8   mean before RGB -> after")
    for i in range(10):
        y0, y1 = max(int(h * i / 10), HUD_ROWS), int(h * (i + 1) / 10)
        if y1 <= y0:
            continue
        band = diff[y0:y1]
        bb, aa = b[y0:y1].reshape(-1, 3).mean(axis=0), a[y0:y1].reshape(-1, 3).mean(axis=0)
        print(f"  y{y0:4d}..{y1:4d}  {band.mean():8.2f}  {100.0 * (band > 8).mean():6.1f}%   "
              f"[{bb[0]:5.1f},{bb[1]:5.1f},{bb[2]:5.1f}] -> [{aa[0]:5.1f},{aa[1]:5.1f},{aa[2]:5.1f}]")

    # A band that cannot be sky under any camera: the bottom fifth of the frame.
    floor = diff[int(h * 0.8):]
    print(f"  bottom-20% (ground only): mean |d| = {floor.mean():.2f} levels, "
          f"{100.0 * (floor > 8).mean():.1f}% of px past 8\n")

    amp = np.clip(diff * 6.0, 0, 255).astype(np.uint8)
    panels.append(Image.fromarray(np.stack([amp] * 3, axis=2)).resize((640, 360), Image.NEAREST))

plate = Image.new("RGB", (640 * len(panels) + 30, 370), (18, 18, 22))
for i, p in enumerate(panels):
    plate.paste(p, (10 + i * 650, 5))
plate.save(OUT)
print(f"wrote {OUT}  (|before-after| x6; white = changed)")
