#!/usr/bin/env python3
"""Measure the in-game wind A/B pair: % pixels moved + md5 of both frames.

Same "moved" definition as `_kevin_sway_measure.py`: a pixel counts as moved
once any channel shifts by more than THRESH 8-bit levels (above dither/tonemap
noise, below a real blade sliding off a pixel).

Usage:
    python scripts/_kevin_ingame_measure.py _kevin_foliage/ingame/before.png \
        _kevin_foliage/ingame/after.png
"""
import hashlib
import sys
from pathlib import Path

import numpy as np
from PIL import Image

THRESH = 8
EDGE_LEVEL = 20


def load(p: Path) -> np.ndarray:
    return np.asarray(Image.open(p).convert("RGB"), dtype=np.int16)


def md5(p: Path) -> str:
    return hashlib.md5(p.read_bytes()).hexdigest()


def edge_pct(img: np.ndarray) -> float:
    """Share of pixels on a strong luminance edge — a plant-blade sanity check."""
    lum = img.mean(axis=2)
    gx = np.abs(np.diff(lum, axis=1))
    gy = np.abs(np.diff(lum, axis=0))
    g = np.zeros_like(lum)
    g[:, :-1] = np.maximum(g[:, :-1], gx)
    g[:-1, :] = np.maximum(g[:-1, :], gy)
    return 100.0 * (g > EDGE_LEVEL).mean()


def main() -> int:
    a, b = Path(sys.argv[1]), Path(sys.argv[2])
    ia, ib = load(a), load(b)
    if ia.shape != ib.shape:
        print(f"shape mismatch: {a.name} {ia.shape} vs {b.name} {ib.shape}")
        return 2
    d = np.abs(ia - ib).max(axis=2)
    moved = d > THRESH
    print(f"before md5   {md5(a)}")
    print(f"after  md5   {md5(b)}")
    print(f"resolution   {ia.shape[1]}x{ia.shape[0]}  ({ia.shape[0]*ia.shape[1]} px)")
    print(f"pixels moved {100.0 * moved.mean():.3f}%   (max|d|={int(d.max())})")
    print(f"edge density before {edge_pct(ia):.2f}%  after {edge_pct(ib):.2f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
