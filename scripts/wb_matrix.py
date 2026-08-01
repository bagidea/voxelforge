#!/usr/bin/env python3
"""Reproduce Bevy 0.19's ColorGrading white-balance matrix on the CPU.

Port of `From<ColorGrading> for ColorGradingUniform` in
bevy_render-0.19.0/src/view/mod.rs (lines 71-95, 722-765). Same constants, same
order. Used to answer one question without a GPU: what per-channel gain does a
given `ColorGradingGlobal::temperature` actually apply?

Usage:  python scripts/wb_matrix.py [temperature ...]
"""
import sys

import numpy as np

# glam's mat3(a, b, c) takes COLUMNS, so each literal below is a column.
RGB_TO_LMS = np.array(
    [
        [0.311692, 0.652085, 0.0362225],
        [0.0905138, 0.901341, 0.00814478],
        [0.00764433, 0.0486554, 0.943700],
    ]
)
LMS_TO_RGB = np.array(
    [
        [4.06305, -2.93241, -0.130646],
        [-0.40791, 1.40437, 0.00353630],
        [-0.0118812, -0.0486532, 1.0605344],
    ]
)
D65_XY = np.array([0.31272, 0.32903])
D65_LMS = np.array([0.975538, 1.01648, 1.08475])


def balance(temperature: float, tint: float = 0.0) -> np.ndarray:
    """The `balance` Mat3 Bevy uploads to the tonemapping shader."""
    # NOTE the sign: temperature is SUBTRACTED from the white point's x.
    wp_xy = D65_XY + np.array([-temperature, tint])
    wp_lms = np.array([0.701634, 1.15856, -0.904175] ) + (
        np.array([-0.051461, 0.045854, 0.953127])
        + np.array([0.452749, -0.296122, -0.955206]) * wp_xy[0]
    ) / wp_xy[1]
    return LMS_TO_RGB @ np.diag(D65_LMS / wp_lms) @ RGB_TO_LMS


def srgb_to_linear(c):
    c = np.asarray(c, dtype=float)
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(c):
    c = np.asarray(c, dtype=float)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * np.clip(c, 0, None) ** (1 / 2.4) - 0.055)


def main():
    temps = [float(a) for a in sys.argv[1:]] or [0.0, 0.02, 0.04, 0.06, 0.08, 0.10]
    # main.rs:358 — the authored playable sky.
    sky_srgb = np.array([0.53, 0.72, 0.92])
    sky_lin = srgb_to_linear(sky_srgb)
    print(f"authored sky srgb {sky_srgb} -> linear {np.round(sky_lin, 4)}")
    print()
    hdr = "temp    diag-gain R/G/B              sky_out(linear)         sky_out(srgb 0-255)  order"
    print(hdr)
    print("-" * len(hdr))
    for t in temps:
        m = balance(t)
        out = m @ sky_lin
        gains = np.diag(m)
        s255 = np.round(linear_to_srgb(np.clip(out, 0, 1)) * 255, 1)
        order = "".join(
            c for _, c in sorted(zip(out, "RGB"), reverse=True)
        )
        order = " > ".join(order)
        print(
            f"{t:<7.3f} {gains[0]:.3f}/{gains[1]:.3f}/{gains[2]:.3f}      "
            f"{np.round(out, 4)}   {s255}   {order}"
        )


if __name__ == "__main__":
    main()
