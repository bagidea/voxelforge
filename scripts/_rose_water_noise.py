#!/usr/bin/env python3
"""Rose — noise floor for the water evidence pair.

Compares _rose_water_20260818/after.png vs noise.png — the SAME map, SAME
exe, SAME env shot twice — and prints the SAME metrics
_rose_water_verify.py prints for before/after. The before->after delta is
only evidence of a river if it towers over these numbers.

Exit 0 always; this is a measurement, not a gate.
"""

import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "_rose_water_20260818"


def metrics(a_p: Path, b_p: Path, label: str):
    a = Image.open(a_p).convert("RGB")
    b = Image.open(b_p).convert("RGB")
    if a.size != b.size:
        print(f"FAIL {label}: size mismatch {a.size} vs {b.size}")
        return
    w, h = a.size
    pa, pb = a.load(), b.load()
    n = 0
    changed = 0
    diff_sum = 0.0
    for y in range(0, h, 2):
        for x in range(0, w, 2):
            ra, ga, ba = pa[x, y]
            rb, gb, bb_ = pb[x, y]
            d = abs(ra - rb) + abs(ga - gb) + abs(ba - bb_)
            n += 1
            diff_sum += d / 3
            if d > 24:
                changed += 1
    print(f"{label:14s}: {w}x{h}  mean |diff| {diff_sum / n:.3f}  "
          f"changed(>24) {changed} px = {changed * 100 / n:.3f}%")


def main():
    pairs = [
        (OUT / "after.png", OUT / "noise.png", "noise floor"),
        (OUT / "before.png", OUT / "after.png", "before->after"),
    ]
    for a_p, b_p, label in pairs:
        if not a_p.exists() or not b_p.exists():
            print(f"skip {label}: missing {a_p.name}/{b_p.name}")
            continue
        metrics(a_p, b_p, label)
    return 0


if __name__ == "__main__":
    sys.exit(main())
