#!/usr/bin/env python3
"""_pixel_artgap_controls.py — calibration harness for _pixel_artgap_grade.py.

The reference scores 1.00 on every axis BY CONSTRUCTION (the targets are derived
from it). That is worth nothing on its own. What this harness proves is the other
half: that each axis moves when — and only when — the property it names is
damaged.

Method: take the CEO reference and break ONE property at a time, then require
  (a) the axis that names that property to COLLAPSE (measure a real number below
      the 0.60 gate — "it refused to measure" does NOT count as collapsing), and
  (b) at least one named bystander axis to SURVIVE (ratio >= 0.60), and
  (c) where the tool is supposed to give up, that it REFUSES outright,
so a lesion that tanks every number equally is caught as a non-specific metric,
and a guard that quietly scores a degenerate frame is caught as a lie.

C0  identity       untouched REF                -> every axis 1.00, nothing fails
C1  desaturate     S -> 0                       -> saturation/palette die, detail lives
C2  blur r=4       gaussian                     -> local contrast/detail die, colour lives
C3  black sky      sky region -> RGB 0          -> sky axes die, SILHOUETTE REFUSES,
                                                   the terrain-band axes SURVIVE
C4  flat fill      whole frame -> one grey      -> everything dies (sanity: axes CAN fail)
C5  crush+clip     hard S-curve                 -> dynamic range dies, hue lives
C6  hue collapse   rotate all hue to one value  -> palette breadth dies, luminance lives
C7  far-band blur  blur ONLY below the skyline  -> distance detail dies, near detail lives
C8  near desat     desaturate ONLY the near band-> aerial perspective (sat) dies
C9  blown sky      sky region -> RGB 255        -> SILHOUETTE REFUSES (white void is as
                                                   degenerate as black), terrain SURVIVES
C10 near blur      blur ONLY the near band      -> aerial perspective (detail) dies

C3 / C7 / C8 / C9 / C10 are what license the 2026-08-19 guard split. Before it,
ONE `sky_ok` flag gated all four far-field axes, so any dark-sky frame lost three
honest terrain measurements along with the one that genuinely could not be made.
C3 and C9 prove the silhouette still refuses on a degenerate sky in BOTH
directions; C7 / C8 / C10 prove the three axes that were freed actually bite.

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


def _bands(a):
    """The grader's own sky / far / near geometry, at the lesion's resolution.

    Reuses G.sky_mask + G.horizon_from_sky + G.silhouette rather than
    re-deriving anything, so a lesion is guaranteed to land on the SAME pixels
    the axis under test will later read. A control that lesions a region the
    metric never samples proves nothing.
    """
    L = G.luma(a)
    sky = G.sky_mask(L)
    hz = G.horizon_from_sky(sky)
    _, far, near, _ = G.silhouette(L, hz, sky)
    return sky, far, near


def _soft(mask, sigma=2.0):
    """Feathered alpha, so a lesion does not paint a hard seam of its own.

    A hard-edged composite would ADD high-frequency energy at the band border —
    inside the very band whose micro-contrast is being measured — which would
    fight the collapse the control is asking for. Feathering keeps the control
    conservative: it can only make a COLLAPSE verdict harder to reach.
    """
    return ndimage.gaussian_filter(mask.astype(np.float64), sigma)[..., None]


def c7_far_blur(src, dst):
    """Blur ONLY the band just below the skyline. Distant detail dies; near lives."""
    a = _rgb(src)
    _, far, _ = _bands(a)
    blur = np.dstack([ndimage.gaussian_filter(a[..., c], 4.0) for c in range(3)])
    al = _soft(far)
    _save(a * (1 - al) + blur * al, dst)


def c8_near_desaturate(src, dst):
    """Desaturate ONLY the near band -> aerial perspective (sat) inverts.

    Lesioning the NEAR side, not the far side. Both directions describe the same
    defect - foreground no more saturated than distance - but only this one can
    actually reach the gate: depth_sat_ratio is near/far, and pushing FAR up far
    enough to halve the ratio would need far_sat ~94%, which saturation being
    bounded at 100 makes unreachable. A control that cannot reach its own
    threshold tests the lesion's strength, not the metric.

    `L + (a - L) * k` leaves luma algebraically untouched (luma(a - L) == 0), so
    this lesion is chroma-only by construction: depth_sat_ratio must collapse
    while far_micro / near_micro, which read a luma high-pass, must survive.
    """
    a = _rgb(src)
    _, _, near = _bands(a)
    L = G.luma(a)[..., None]
    grey = L + (a - L) * 0.05
    al = _soft(near)
    _save(a * (1 - al) + grey * al, dst)


def c10_near_blur(src, dst):
    """Blur ONLY the near band -> aerial perspective (detail) inverts.

    Same reasoning as C8: depth_micro_ratio is near/far, so the lesion that
    COLLAPSES it is the one that kills near detail. Blurring the FAR band (C7)
    RAISES this ratio - correctly, because blurring the distance IS aerial
    perspective. The first draft of C7 demanded depth_micro_ratio collapse and
    got 2.69x; the control was wrong, not the axis.
    """
    a = _rgb(src)
    _, _, near = _bands(a)
    blur = np.dstack([ndimage.gaussian_filter(a[..., c], 4.0) for c in range(3)])
    al = _soft(near)
    _save(a * (1 - al) + blur * al, dst)


def c9_blown_sky(src, dst):
    """Sky -> pure white. The mirror of C3: a white void is still a void."""
    a = _rgb(src)
    sky = G.sky_mask(G.luma(a))
    a[sky] = 255.0
    _save(a, dst)


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


# control name -> (lesion fn, MUST collapse, MUST survive, MUST refuse to measure)
#
# COLLAPSE is strict: the axis has to come back with a real number below the
# gate. An axis that returns "unmeasurable" is NOT collapsing - it is declining,
# which is a different claim and belongs in the refuse column.
CONTROLS = [
    ("C0 identity",     c0_identity,   [],
     ["sat_mean", "edge_density", "hue_bins", "sky_frac_pct", "range_p5_p95",
      "far_edge_contrast", "far_micro", "depth_sat_ratio", "depth_micro_ratio"], []),
    ("C1 desaturate",   c1_desaturate, ["sat_mean", "hue_bins", "hue_entropy"],
     ["edge_density", "micro_r3", "sky_frac_pct"], []),
    ("C2 blur r4",      c2_blur,       ["edge_density", "micro_r3", "micro_r1"],
     ["sat_mean", "hue_bins", "range_p5_p95"], []),
    # The control that licenses the guard SPLIT. Blacking the sky touches zero
    # terrain pixels, so the three band axes must come back essentially
    # unchanged. Before 2026-08-19 they all returned "unmeasurable" here - three
    # honest numbers thrown away because a fourth one could not be made.
    ("C3 black sky",    c3_black_sky,  ["sky_L_range", "sky_hue_span_deg",
                                        "sky_ground_ratio", "sky_void_pct"],
     ["edge_density", "sat_mean",
      "far_micro", "depth_sat_ratio", "depth_micro_ratio"],
     ["far_edge_contrast"]),
    ("C4 flat grey",    c4_flat,       ["edge_density", "micro_r3", "sat_mean",
                                        "hue_bins", "range_p5_p95", "tonal_bins"], [], []),
    # C5 first named tonal_bins and FAILED (32 -> 32). The control was wrong, not
    # the metric: an S-curve crushes the ENDS but re-spreads what is left across
    # the full range, so histogram occupancy is untouched. The axes that actually
    # name this damage are crush_pct / clip_pct. tonal_bins keeps its own negative
    # control in C4 (32 -> 1), so it is not left unproven.
    ("C5 crush+clip",   c5_crush_clip, ["crush_pct", "clip_pct"],
     ["hue_bins", "edge_density", "tonal_bins"], []),
    ("C6 hue collapse", c6_hue_collapse, ["hue_bins", "hue_entropy"],
     ["range_p5_p95", "edge_density"], []),
    # C7/C8/C10 are the other half of the split: having FREED these three axes,
    # prove they are worth having. Each lesion damages ONE property inside ONE
    # band, and the other band must not move.
    ("C7 far-band blur", c7_far_blur,  ["far_micro"],
     ["near_micro", "sat_mean", "sky_L_range"], []),
    ("C8 near desat",   c8_near_desaturate, ["depth_sat_ratio"],
     ["far_micro", "near_micro", "sky_L_range"], []),
    ("C10 near blur",   c10_near_blur, ["depth_micro_ratio", "near_micro"],
     ["far_micro", "sat_mean", "sky_L_range"], []),
    # The guard the 2026-08-18 version did not have. Over-correcting the black
    # sky into a white one is the single most likely next state of this frame,
    # and it is exactly as degenerate to measure a silhouette against.
    ("C9 blown sky",    c9_blown_sky,  ["sky_blown_pct"],
     ["edge_density", "far_micro", "depth_sat_ratio", "depth_micro_ratio"],
     ["far_edge_contrast"]),
]

DIRS = {k: d for k, d, _, _, _ in G.AXES}


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

    for name, fn, must_die, must_live, must_refuse in CONTROLS:
        out = tmp / (name.split()[0] + ".png")
        fn(str(REF), str(out))
        m = G.measure(str(out))
        for axis, expect in [(a, "COLLAPSE") for a in must_die] + \
                            [(a, "survive") for a in must_live] + \
                            [(a, "REFUSE") for a in must_refuse]:
            d = DIRS.get(axis, +1)
            r = G.ratio(m[axis], base[axis], d)
            if r is None:
                # "unmeasurable" satisfies REFUSE only. It is NOT a collapse: an
                # axis that declines to answer has not demonstrated it can see
                # the damage, and letting it count as a pass is how a guard ends
                # up standing in for the metric it was supposed to protect.
                ok = expect == "REFUSE"
                shown = "refused"
            else:
                ok = {"COLLAPSE": r < G.GATE,
                      "survive": r >= G.GATE,
                      "REFUSE": False}[expect]
                shown = f"{r:.2f}x"
            if not ok:
                bad += 1
            print(f"{name:<18}{axis:<22}{G.fmt(base[axis]):>10}{G.fmt(m[axis]):>12}"
                  f"{shown:>12}  {expect:<8} {'PASS' if ok else '** FAIL **'}")
        print()

    # C0 must not fail ANY axis, or the harness itself is unsound
    c0 = G.measure(str(tmp / "C0.png"))
    c0_fails = [k for k, d, _, _, _ in G.AXES
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
