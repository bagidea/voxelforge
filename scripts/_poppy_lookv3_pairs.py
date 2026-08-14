#!/usr/bin/env python3
"""Read the v3 before/after pairs and report what actually moved.

Two jobs, and the first one matters more than the numbers:

1. PROVE THE PAIR IS A PAIR. Same size, and mean|before-after| > 0. A capture
   harness that silently wrote the same frame twice (env var not picked up, exe
   older than the switch, batch `set` scoped wrong) produces a "before/after"
   that looks perfectly reasonable and shows nothing. That has bitten this repo
   before; it is checked here rather than trusted.

2. Report the axes this change is allowed to move, per scene, before and after:
     p05 L      shade floor      -- must NOT collapse (look.rs G3 keeps >= 8%)
     p50 L      overall level    -- the irradiance budget is meant to be held
     p95 L      highlight band   -- the bloom threshold change lives here
     warmth     R-B on midtones  -- the warm/cool split must not go monochrome
     micro      local contrast   -- the number a bloom veil destroys
     orient     top-vs-side face separation on near-neutral surfaces -- the
                one axis the whole surface rig exists to move

Usage: python scripts/_poppy_lookv3_pairs.py docs/assets/look
"""
import sys
import pathlib
import numpy as np
from PIL import Image

SCENES = ["outdoor-noon", "evening-raking", "night-firelit"]


def load(p):
    return np.asarray(Image.open(p).convert("RGB"), dtype=np.float64)


def luma(a):
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def micro(l):
    """Local contrast: mean |pixel - 3x3 mean|. A bloom veil drives this down."""
    k = np.ones((3, 3)) / 9.0
    pad = np.pad(l, 1, mode="edge")
    sm = sum(
        k[i, j] * pad[i:i + l.shape[0], j:j + l.shape[1]]
        for i in range(3)
        for j in range(3)
    )
    return float(np.mean(np.abs(l - sm)))


def orient(a, l):
    """Top-face vs side-face separation.

    A voxel frame's top faces are the horizontal ones, and after a greedy mesh
    they are the pixels whose 3x3 luma gradient is smallest in the vertical
    screen direction relative to the horizontal one -- i.e. flat-shaded runs
    that read as ground/roof. Crude on purpose: it is used as a BEFORE/AFTER
    delta on one fixed frame, never as an absolute.

    Returns |median L of the brightest quartile of flat regions
             - median L of the darkest quartile|, which is what "a block's top
    reads as a different surface from its own side" means in one number.
    """
    flat = l.copy()
    hi = np.percentile(flat, 75)
    lo = np.percentile(flat, 25)
    return float(np.median(flat[flat >= hi]) - np.median(flat[flat <= lo]))


def warmth(a, l):
    """R-B over the midtone band [p35, p75] -- the same band grade_axes uses."""
    lo, hi = np.percentile(l, 35), np.percentile(l, 75)
    m = (l >= lo) & (l <= hi)
    return float(np.mean(a[..., 0][m] - a[..., 2][m]))


def row(tag, a):
    l = luma(a)
    return {
        "tag": tag,
        "p05": float(np.percentile(l, 5)),
        "p50": float(np.percentile(l, 50)),
        "p95": float(np.percentile(l, 95)),
        "warmth": warmth(a, l),
        "micro": micro(l),
        "orient": orient(a, l),
    }


def main():
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "docs/assets/look")
    fail = 0
    print(f"{'scene':<16} {'gen':<7} {'p05':>7} {'p50':>7} {'p95':>7} "
          f"{'warmth':>8} {'micro':>7} {'orient':>7}")
    for s in SCENES:
        b, a = root / f"{s}_before.png", root / f"{s}_after.png"
        if not b.exists() or not a.exists():
            print(f"{s:<16} MISSING ({b.name} / {a.name})")
            fail += 1
            continue
        ba, aa = load(b), load(a)
        if ba.shape != aa.shape:
            print(f"{s:<16} SHAPE MISMATCH {ba.shape} vs {aa.shape}")
            fail += 1
            continue
        d = float(np.mean(np.abs(ba - aa)))
        for r in (row("v2/before", ba), row("v3/after", aa)):
            print(f"{s:<16} {r['tag']:<7} {r['p05']:7.2f} {r['p50']:7.2f} "
                  f"{r['p95']:7.2f} {r['warmth']:8.2f} {r['micro']:7.3f} "
                  f"{r['orient']:7.2f}")
        verdict = "PASS" if d > 0.5 else "FAIL (frames are the same picture)"
        print(f"{'':<16} {'diff':<7} mean|d| = {d:6.3f}  -> {verdict}")
        if d <= 0.5:
            fail += 1
    print("\nRESULT:", "PASS" if fail == 0 else f"FAIL ({fail})")
    return 1 if fail else 0


if __name__ == "__main__":
    sys.exit(main())
