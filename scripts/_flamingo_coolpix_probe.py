#!/usr/bin/env python3
"""Where did the blue go? — cool-pixel census for the P0-ENV frame vs the CEO ref.

Answers exactly one question, in numbers a second person can re-run:
  * what fraction of each image has B >= R (the "any cool pixel at all" test the
    CEO quoted as 0.0% vs 9.5%), and
  * once the frame is split into sky band / ground band, WHICH band lost it,
  * plus the mean B/R of the sky plate the dome is painted with, per row, so the
    dome's own contribution is a measured number rather than an argument.

Read-only. Prints a table; writes nothing.
"""
from __future__ import annotations

import sys
import numpy as np
from PIL import Image


def load(p: str) -> np.ndarray:
    im = Image.open(p).convert("RGB")
    return np.asarray(im).astype(np.float32)


def census(a: np.ndarray, label: str) -> None:
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    cool = (b >= r)
    # guard against the degenerate case: near-black pixels satisfy B>=R for free
    lit = (r + g + b) > 30.0
    cool_lit = cool & lit
    print(
        f"{label:<34} B>=R {100.0 * cool.mean():5.2f}%   "
        f"B>=R & not-black {100.0 * cool_lit.mean():5.2f}%   "
        f"mean B/R {(b.mean() / max(r.mean(), 1e-6)):5.3f}   "
        f"mean R-B {(r.mean() - b.mean()):6.2f}"
    )


def bands(a: np.ndarray, label: str) -> None:
    h = a.shape[0]
    for name, sl in (("top third (sky)", slice(0, h // 3)),
                     ("mid third", slice(h // 3, 2 * h // 3)),
                     ("bottom third (ground)", slice(2 * h // 3, h))):
        census(a[sl], f"  {label} / {name}")


def plate_rows(p: str) -> None:
    a = load(p)
    h = a.shape[0]
    print(f"\n{p}  ({a.shape[1]}x{h}) per-row B/R, top -> bottom:")
    for y in range(0, h, max(1, h // 8)):
        row = a[y]
        r, b = row[..., 0].mean(), row[..., 2].mean()
        print(f"   row {y:>3}  R {r:6.1f}  G {row[..., 1].mean():6.1f}  B {b:6.1f}"
              f"   B/R {b / max(r, 1e-6):5.3f}")
    r, b = a[..., 0].mean(), a[..., 2].mean()
    print(f"   WHOLE PLATE  B/R {b / max(r, 1e-6):5.3f}   "
          f"B>=R {100.0 * (a[..., 2] >= a[..., 0]).mean():5.2f}%")


def main(argv: list[str]) -> int:
    for p in argv[1:]:
        if p.endswith(("sky_gradient_sunset.png",)):
            plate_rows(p)
            continue
        a = load(p)
        print()
        census(a, p)
        bands(a, p.rsplit("/", 1)[-1])
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
