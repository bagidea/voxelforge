#!/usr/bin/env python3
"""_rose_sky_gradient.py — A1 acceptance numbers for the gradient sky dome.

The art order (docs/art-order-2026-08-09-composition.md §A1) accepts the dome on
two numbers:

  * vertical gradient of the WHOLE sky ≥ 25 L (top-to-bottom)
  * no sharp seam where the sky meets the world at the horizon

`palette_break.sky_stats` measures the gradient only inside the BLUE sub-region
(its sky mask is hue-gated), so a blue→gold golden-hour dome under-reports. This
probe measures the full vertical luminance swing across the top (sky) half of the
frame and the sharpness of the sky→terrain edge at the world horizon — the two
quantities the acceptance is actually about.

NOT a gate (no thresholds, no exit code): it prints the numbers so the before/
after pair speaks for itself, same as every other _rose_ lane probe.

Usage:
    python scripts/_rose_sky_gradient.py frame_before.png frame_after.png ...
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image


def _lum(rgb: np.ndarray) -> np.ndarray:
    """sRGB display luminance 0..1 (the L the look-bible / rubric grades in)."""
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0


def _pearson(x: np.ndarray, y: np.ndarray) -> float:
    """Pearson correlation — the ramp-monotonicity score (no scipy dep)."""
    x = np.asarray(x, dtype=np.float64)
    y = np.asarray(y, dtype=np.float64)
    xm, ym = x - x.mean(), y - y.mean()
    denom = np.sqrt(float((xm * xm).sum()) * float((ym * ym).sum()))
    return float(xm.dot(ym) / denom) if denom else 0.0


def report(path: Path) -> dict:
    rgb = np.asarray(Image.open(path).convert("RGB"), dtype=np.float32)
    H, W, _ = rgb.shape
    lum = _lum(rgb)

    # Row-mean luminance profile (collapse the X axis). The boom camera looks
    # slightly down, so the sky occupies roughly the top half of the frame.
    row_l = lum.mean(axis=1) * 100.0  # percent L per row, top = row 0

    # Find the WORLD horizon adaptively: the strongest single-row L drop in the
    # 30-65% band is where the sky melts into terrain/haze. Everything above it is
    # sky; the gradient is measured INSIDE that sky region (zenith vs the band
    # just above the horizon) — exactly the two points the flat ClearColor could
    # never separate.
    lo, hi = int(H * 0.30), int(H * 0.65)
    drop = -np.diff(row_l[lo:hi + 1])  # positive = bright sky -> darker terrain
    hk = int(np.argmax(drop)) if drop.size else 0
    horizon_row = int(lo + hk)
    sky_bot = max(2, horizon_row - 1)
    span = max(1, int(sky_bot * 0.10))
    l_top = float(row_l[:span].mean())
    l_hor = float(row_l[sky_bot - span: sky_bot + 1].mean())
    grad = l_top - l_hor  # signed: <0 means horizon brighter than zenith (golden hour)
    seam = float(drop[hk]) if drop.size else float("nan")

    # Sky share of the frame (the owner's "sky% (row 0..N)" diagnostic): if this
    # is tiny the gradient is noise, not signal — a camera-preset problem, not a
    # dome problem.
    sky_pct = 100.0 * horizon_row / H
    # Ramp monotonicity = Pearson(row, L) over the sky band. A clean zenith->horizon
    # ramp is monotonic (the haze brightens toward the horizon), so |r| is near 1.
    sky_profile = row_l[:sky_bot + 1]
    ramp_mono = _pearson(np.arange(sky_profile.size), sky_profile) if sky_profile.size > 2 else 0.0

    print(
        f"=== {path.name} ({W}x{H}) ===\n"
        f"  sky region rows 0..{sky_bot}  (sky% = {sky_pct:4.1f}, horizon ~{100.0*horizon_row/H:.0f}% down)\n"
        f"  A1 zenith-hz      : |zenith-horizon| = {abs(grad):5.2f} L   "
        f"(>=25 PASS / <25 FAIL)\n"
        f"  A1 ramp-monotonic : r = {abs(ramp_mono):4.2f}   "
        f"(>=0.85 PASS / <0.85 FAIL)\n"
        f"  A1 horizon-seam   : step = {seam:5.2f} L   "
        f"({'seamless melt' if seam < 12 else ('soft' if seam < 25 else 'HARD EDGE')} | <=12 PASS)\n"
        f"  raw  zenith={l_top:6.2f}  horizon={l_hor:6.2f}  signed_grad={grad:+6.2f}"
    )
    return {
        "file": path.name,
        "sky_pct": sky_pct,
        "zenith_hz_L": abs(grad),
        "ramp_monotonic": abs(ramp_mono),
        "horizon_seam_L": seam,
    }


def main(argv: list[str]) -> int:
    paths = [Path(a) for a in argv[1:]]
    if not paths:
        print(__doc__)
        return 2
    for p in paths:
        if not p.exists():
            print(f"!! missing: {p}")
            continue
        report(p)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
