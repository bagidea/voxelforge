#!/usr/bin/env python3
"""_pixel_artgap_controls.py — calibration harness for _pixel_artgap_grade.py.

The reference scores 1.00 on every axis BY CONSTRUCTION (the targets are derived
from it). That is worth nothing on its own. What this harness proves is the other
half: that each axis moves when — and only when — the property it names is
damaged.

Method: take the CEO reference and break ONE property at a time, then require
  (a) the axis that names that property to COLLAPSE (ratio < 0.60), and
  (b) at least one named bystander axis to SURVIVE (ratio >= 0.60),
so a lesion that tanks every number equally is caught as a non-specific metric.

C0  identity      untouched REF                -> every axis 1.00, nothing fails
C1  desaturate    S -> 0                       -> saturation/palette die, detail lives
C2  blur r=4      gaussian                     -> local contrast/detail die, colour lives
C3  black sky     sky region -> RGB 0          -> sky axes die, ground detail lives
C4  flat fill     whole frame -> one grey       -> everything dies (sanity: axes CAN fail)
C5  crush+clip    hard S-curve                 -> dynamic range dies, hue lives
C6  hue collapse  rotate all hue to one value  -> palette breadth dies, luminance lives

Run:  python scripts/_pixel_artgap_controls.py
Exit: 0 all controls behaved; 1 a control failed (a metric is broken/non-specific)
"""

from __future__ import annotations

import importlib.util
import shutil
import sys
import tempfile
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

HERE = Path(__file__).resolve().parent
REF = HERE.parent / "docs" / "refs" / "ceo_ref_sunset_valley.jpg"

spec = importlib.util.spec_from_file_location("artgap", HERE / "_pixel_artgap_grade.py")
G = importlib.util.module_from_spec(spec)
spec.loader.exec_module(G)


# ------------------------------------------------------------- lesions

def _rgb(path):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)


def _save(a, path):
    Image.fromarray(a.clip(0, 255).astype(np.uint8)).save(path)


def c0_identity(src, dst):
    shutil.copy(src, dst)


def c1_desaturate(src, dst):
    a = _rgb(src)
    L = G.luma(a)
    _save(np.dstack([L, L, L]), dst)


def c2_blur(src, dst):
    a = _rgb(src)
    _save(np.dstack([ndimage.gaussian_filter(a[..., c], 4.0) for c in range(3)]), dst)


def c3_black_sky(src, dst):
    a = _rgb(src)
    sky = G.sky_mask(G.luma(a))
    a[sky] = 0
    _save(a, dst)


def c4_flat(src, dst):
    a = _rgb(src)
    _save(np.full_like(a, 128.0), dst)


def c5_crush_clip(src, dst):
    a = _rgb(src) / 255.0
    # hard S-curve: everything below 0.35 -> 0, above 0.75 -> 1
    a = np.clip((a - 0.35) / 0.40, 0, 1)
    _save(a * 255.0, dst)


def c6_hue_collapse(src, dst):
    """Keep luminance and saturation, force every hue to 30 deg (orange)."""
    a = _rgb(src)
    L = G.luma(a)
    _, sat, val = G.hsv(a)
    # HSV -> RGB at fixed hue 30
    c = val * sat
    x = c * (1 - abs(((30 / 60.0) % 2) - 1))
    m = val - c
    out = np.dstack([c + m, x + m, m])
    # renormalise to the original luminance so this lesion is hue-only
    scale = np.where(G.luma(out) > 1e-6, L / np.maximum(G.luma(out), 1e-6), 1.0)
    _save(out * scale[..., None], dst)


# control name -> (lesion fn, axes that MUST collapse, axes that MUST survive)
CONTROLS = [
    ("C0 identity",     c0_identity,   [],
     ["sat_mean", "edge_density", "hue_bins", "sky_frac_pct", "range_p5_p95"]),
    ("C1 desaturate",   c1_desaturate, ["sat_mean", "hue_bins", "hue_entropy"],
     ["edge_density", "micro_r3", "sky_frac_pct"]),
    ("C2 blur r4",      c2_blur,       ["edge_density", "micro_r3", "micro_r1"],
     ["sat_mean", "hue_bins", "range_p5_p95"]),
    ("C3 black sky",    c3_black_sky,  ["sky_L_range", "sky_hue_span_deg",
                                        "sky_ground_ratio", "sky_void_pct"],
     ["edge_density", "sat_mean"]),
    ("C4 flat grey",    c4_flat,       ["edge_density", "micro_r3", "sat_mean",
                                        "hue_bins", "range_p5_p95", "tonal_bins"], []),
    # C5 first named tonal_bins and FAILED (32 -> 32). The control was wrong, not
    # the metric: an S-curve crushes the ENDS but re-spreads what is left across
    # the full range, so histogram occupancy is untouched. The axes that actually
    # name this damage are crush_pct / clip_pct. tonal_bins keeps its own negative
    # control in C4 (32 -> 1), so it is not left unproven.
    ("C5 crush+clip",   c5_crush_clip, ["crush_pct", "clip_pct"],
     ["hue_bins", "edge_density", "tonal_bins"]),
    ("C6 hue collapse", c6_hue_collapse, ["hue_bins", "hue_entropy"],
     ["range_p5_p95", "edge_density"]),
]

DIRS = {k: d for k, d, _, _ in G.AXES}


def main():
    if not REF.exists():
        print(f"FATAL: reference missing: {REF}")
        return 2
    base = G.measure(str(REF))
    tmp = Path(tempfile.mkdtemp(prefix="artgap_ctl_"))
    bad = 0

    print(f"reference: {REF.name}  ({base['px']} normalised)")
    print(f"{'control':<18}{'axis':<22}{'REF':>10}{'lesioned':>12}{'ratio':>8}  expect  verdict")
    print("-" * 88)

    for name, fn, must_die, must_live in CONTROLS:
        out = tmp / (name.split()[0] + ".png")
        fn(str(REF), str(out))
        m = G.measure(str(out))
        for axis, expect in [(a, "COLLAPSE") for a in must_die] + \
                            [(a, "survive") for a in must_live]:
            d = DIRS.get(axis, +1)
            r = G.ratio(m[axis], base[axis], d)
            if r is None:
                ok = expect == "COLLAPSE"      # refusing to measure a destroyed axis is fine
                shown = "unmeasurable"
            else:
                ok = (r < G.GATE) if expect == "COLLAPSE" else (r >= G.GATE)
                shown = f"{r:.2f}x"
            if not ok:
                bad += 1
            print(f"{name:<18}{axis:<22}{G.fmt(base[axis]):>10}{G.fmt(m[axis]):>12}"
                  f"{shown:>8}  {expect:<8} {'PASS' if ok else '** FAIL **'}")
        print()

    # C0 must not fail ANY axis, or the harness itself is unsound
    c0 = G.measure(str(tmp / "C0.png"))
    c0_fails = [k for k, d, _, _ in G.AXES
                if (r := G.ratio(c0[k], base[k], d)) is not None and r < G.GATE]
    if c0_fails:
        bad += len(c0_fails)
        print(f"** C0 POSITIVE CONTROL FAILED on {c0_fails} — the reference cannot "
              f"grade itself, so no threshold here means anything **")
    else:
        print(f"C0 positive control: reference grades itself {len(G.AXES)}/{len(G.AXES)} "
              f"axes at gate {G.GATE:.0%} — thresholds are reachable.")

    shutil.rmtree(tmp, ignore_errors=True)
    print(f"\n{bad} control failure(s)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
