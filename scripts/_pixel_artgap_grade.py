#!/usr/bin/env python3
"""_pixel_artgap_grade.py — measure the art gap between a render and the CEO reference.

Lane: Flamingo (measurement only). Reads pixels. Writes nothing but JSON/PNG under
paths the caller names. Never touches .rs / assets/ / maps/.

WHY THIS EXISTS
---------------
docs/look-acceptance-rubric.md grades an INTERIOR hero frame (both signed-off refs
have 0 sky px). Nothing in it scores sky, water, atmospheric perspective or
distant silhouette — the six things a sunset-valley reference is mostly made of.
This script adds those axes and re-measures the ones the rubric already has, on a
common footing so a portrait reference and a 16:9 render can be compared honestly.

CALIBRATION CONTRACT (rubric rule 7)
------------------------------------
Every axis is derived FROM the reference, so the reference scores 1.00 by
construction. That alone proves nothing. What proves the axes bite is
`_pixel_artgap_controls.py`, which mutates the reference one property at a time
(desaturate / blur / black out the sky / flatten) and requires the matching axis —
and ONLY the matching axis — to collapse. A metric that scores the CEO reference
low is a broken metric, not a broken picture.

NORMALISATION
-------------
Both images are resampled to a common AREA (default 1.0 Mpx) keeping aspect.
That makes "detail per unit of screen" comparable, which is what a viewer sees.
It does NOT make framing comparable — see FRAMING CAVEAT in the report.

Consequence for every table this tool feeds: the `px` field/column is the size AFTER
that LANCZOS resample — the geometry the numbers were measured on, safe to compare
across plates, and NOT the resolution of the file or asset (a 1280x720 plate prints
1333x750). The size on disk is carried separately as `px_file`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nohud2_guard import require_nohud2  # noqa: E402  (hard guard, see main())

NORM_AREA = 1_000_000

# ---------------------------------------------------------------- basics


def load(path: str, norm_area: int = NORM_AREA) -> np.ndarray:
    im = Image.open(path).convert("RGB")
    w, h = im.size
    scale = math.sqrt(norm_area / (w * h))
    nw, nh = max(8, round(w * scale)), max(8, round(h * scale))
    im = im.resize((nw, nh), Image.LANCZOS)
    return np.asarray(im, dtype=np.float64)


def luma(rgb: np.ndarray) -> np.ndarray:
    return rgb @ np.array([0.2126, 0.7152, 0.0722])


def hsv(rgb: np.ndarray):
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    mx = rgb.max(axis=-1)
    mn = rgb.min(axis=-1)
    d = mx - mn
    sat = np.where(mx > 0, d / np.maximum(mx, 1e-9), 0.0)
    hue = np.zeros_like(mx)
    nz = d > 1e-9
    ri = nz & (mx == r)
    gi = nz & (mx == g) & ~ri
    bi = nz & (mx == b) & ~ri & ~gi
    hue[ri] = (60 * ((g - b)[ri] / d[ri])) % 360
    hue[gi] = 60 * ((b - r)[gi] / d[gi]) + 120
    hue[bi] = 60 * ((r - g)[bi] / d[bi]) + 240
    return hue, sat, mx


def highpass_std(L: np.ndarray, sigma: float) -> float:
    return float((L - ndimage.gaussian_filter(L, sigma)).std())


def sobel_mag(L: np.ndarray) -> np.ndarray:
    g = ndimage.gaussian_filter(L, 0.8)
    return np.hypot(ndimage.sobel(g, 0), ndimage.sobel(g, 1))


def local_std(L: np.ndarray, size: int = 5) -> np.ndarray:
    m = ndimage.uniform_filter(L, size)
    m2 = ndimage.uniform_filter(L * L, size)
    return np.sqrt(np.maximum(m2 - m * m, 0.0))


# ------------------------------------------------- sky / horizon geometry

SKY_T = 22.0  # Sobel magnitude that counts as a HARD (block/geometry) edge
SKY_BLEED = 10  # px of clearance the terrain bands keep from the sky; > 3*sigma
                # of the sigma=3 high-pass those bands are measured with

# v1 of this detector used a generic r4 high-pass and a per-column first-structure
# scan. It scored the CEO reference at 7.34% sky when the eye reads ~28% — it
# stopped at CLOUD texture and called the clouds terrain — and it labelled a
# smooth blurred interior wall as sky. A metric that scores the reference low is
# a broken metric. Replaced with a hard-edge flood from the top of the frame:
# voxel geometry always carries step discontinuities, painted cloud does not.


def sky_mask(L: np.ndarray, t: float = SKY_T) -> np.ndarray:
    """Region reachable from the top edge without crossing a hard geometry edge.

    Deliberately brightness-blind: a sky that renders as a black void must come
    back as sky (and then get convicted by sky_void_pct), not get filed under
    terrain where nothing would ever measure it.
    """
    free = sobel_mag(L) <= t
    lab, n = ndimage.label(free, structure=np.array([[0, 1, 0], [1, 1, 1], [0, 1, 0]]))
    if n == 0:
        return np.zeros_like(L, dtype=bool)
    top = np.unique(lab[0, :])
    top = top[top > 0]
    if top.size == 0:
        return np.zeros_like(L, dtype=bool)
    sky = np.isin(lab, top)
    # border_value=1: the default erodes the top rows away, which silently zeroed
    # every horizon and made the silhouette / depth axes refuse to measure the REF
    sky = ndimage.binary_closing(sky, np.ones((5, 5)), border_value=1)
    # a sky is a lid, not a curtain: drop columns where the region reaches the
    # bottom of the frame (that is a wall or a backdrop running the full height)
    H, W = L.shape
    through = sky[H - 1, :]
    if through.any():
        sky[:, through] = False
    return sky


def horizon_from_sky(sky: np.ndarray) -> np.ndarray:
    """Per-column first non-sky row = the skyline."""
    H, W = sky.shape
    hz = np.where(sky.any(axis=0), np.argmin(sky, axis=0), 0)
    hz = np.where(sky[0, :], hz, 0)  # column with no sky at all -> no skyline
    return ndimage.median_filter(hz.astype(np.float64), 15).astype(int)


# ------------------------------------------------------------- the axes


def measure(path: str, dump_mask: str | None = None, scene: str = "outdoor") -> dict:
    file_w, file_h = Image.open(path).size  # header only: the size on disk, pre-normalise
    rgb = load(path)
    L = luma(rgb)
    H, W = L.shape
    hue, sat, val = hsv(rgb)
    npx = L.size

    # --- A. dynamic range -------------------------------------------------
    p = {q: float(np.percentile(L, q)) for q in (1, 5, 50, 95, 99)}
    crush = float((L < 8).mean() * 100)
    clip = float((rgb.max(axis=-1) >= 250).mean() * 100)
    hist, _ = np.histogram(L, bins=32, range=(0, 255))
    tonal_bins = int((hist / npx >= 0.0025).sum())

    # --- B. saturation ----------------------------------------------------
    lit = val >= 25  # rubric's L>=20 degeneracy guard: near-black sat is quantisation noise
    sat_mean = float(sat[lit].mean() * 100) if lit.any() else 0.0

    # --- C. local contrast ------------------------------------------------
    micro_r3 = highpass_std(L, 3.0)
    micro_r1 = highpass_std(L, 1.0)
    micro_r8 = highpass_std(L, 8.0)

    # --- D. detail density per area --------------------------------------
    sm = sobel_mag(L)
    edge_density = float(sm.mean())
    strong_edge = float((sm > 40).mean() * 100)
    flat_pct = float((local_std(L, 5) < 2.0).mean() * 100)

    # --- E. palette breadth ----------------------------------------------
    chroma_w = sat * np.clip(val / 255.0, 0, 1)
    hbins = np.zeros(36)
    idx = (hue // 10).astype(int) % 36
    np.add.at(hbins, idx.ravel(), chroma_w.ravel())
    tot = hbins.sum()
    hshare = hbins / tot if tot > 0 else hbins
    hue_bins = int((hshare >= 0.01).sum())
    nzp = hshare[hshare > 0]
    hue_entropy = float(-(nzp * np.log2(nzp)).sum()) if nzp.size else 0.0
    cool_pct = float(hshare[17:27].sum() * 100)  # 170-269 deg: water / sky blue

    # --- F. sky ------------------------------------------------------------
    # scene=="interior" -> SKIP, not FAIL. An interior frame has no sky to grade;
    # scoring it 0 would be measuring a refusal, not a defect.
    sky = sky_mask(L) if scene == "outdoor" else np.zeros_like(L, dtype=bool)
    hz = horizon_from_sky(sky)
    sky_frac = float(sky.mean() * 100)
    if scene != "outdoor":
        sky_frac = None
        sky_void_pct = sky_blown_pct = None
        sky_L_range = sky_L_mean = sky_hue_span = sky_sat = None
        sky_ground_ratio = None
    elif sky.sum() >= 200:
        Ls = L[sky]
        sky_void_pct = float((Ls < 10).mean() * 100)
        # Symmetric partner to sky_void_pct. A sky blown to white maximises
        # |L_sky - L_terrain| for exactly as bad a reason as a sky crushed to
        # black does; the v1 guard only watched the black end, which would have
        # let the NEXT degenerate frame score "better than the CEO reference"
        # the moment someone over-corrected the thing this guard was built for.
        sky_blown_pct = float((Ls > 245).mean() * 100)
        sky_L_range = float(np.percentile(Ls, 95) - np.percentile(Ls, 5))
        sky_L_mean = float(Ls.mean())
        w = chroma_w[sky]
        hs = hue[sky]
        sb = np.zeros(36)
        np.add.at(sb, (hs // 10).astype(int) % 36, w)
        sky_hue_span = float(circ_span(sb, 0.80))
        sky_sat = float(sat[sky][val[sky] >= 25].mean() * 100) if (val[sky] >= 25).any() else 0.0
        # The single most framing-robust number in this file: at golden hour the
        # sky is the BRIGHTEST thing in frame. Scale-free, so a portrait reference
        # and a 16:9 render can be compared without any resampling argument.
        gnd = float(np.median(L[~sky])) if (~sky).any() else None
        sky_ground_ratio = (round(float(np.median(Ls)) / gnd, 3)
                            if gnd and gnd > 1e-6 else None)
    else:
        sky_void_pct = sky_blown_pct = 0.0
        sky_L_range = sky_L_mean = sky_hue_span = sky_sat = 0.0
        sky_ground_ratio = None

    # Every refusing axis owns its own sentence. Before this existed, the
    # scoreboard printed the DEPTH reason next to any axis that refused for any
    # other cause - so an interior frame's sky axes were explained by a message
    # about far bands, and a reference with a 0 on some axis would have been
    # reported as "the grader refused" when in fact the ratio was undefined.
    if scene != "outdoor":
        sky_reason = "interior frame (scene=interior) - no sky to grade"
    elif sky.sum() < 200:
        sky_reason = "sky region under 200 px - too small to measure"
    else:
        sky_reason = "ok"

    # --- G. distant silhouette + atmospheric perspective ------------------
    # TWO GUARDS, NOT ONE. v1 (2026-08-18) had a single `sky_ok` flag gating all
    # four axes, and that was too coarse in a way that cost real information:
    #
    #   far_edge_contrast  reads |L_sky - L_terrain| ACROSS the skyline.
    #                      It genuinely needs a sky with tone. If the sky is a
    #                      hole, that difference is maximal for the worst
    #                      possible reason - v1 of this axis handed
    #                      _matmaps_after 49.04 vs REF 22.33 = "2.20x BETTER
    #                      than the CEO reference" for having no sky at all.
    #                      Contrast against nothing is not a silhouette.
    #
    #   far_micro / far_sat / depth_*   sample the FAR and NEAR TERRAIN BANDS.
    #                      Not one of their pixels is a sky pixel (both bands
    #                      are masked with ~sky below, so that is enforced, not
    #                      asserted). What they need is a TRUSTWORTHY SKYLINE to
    #                      place the far band on - which the sky mask still
    #                      gives, because it is deliberately brightness-blind.
    #                      Gating them on sky luminance refused four honest
    #                      numbers on every dark-sky frame we own.
    #
    # So: sky_lit gates the silhouette. horizon_ok gates the depth axes. Proved
    # by _pixel_artgap_controls.py C3 (black sky) and C9 (blown sky), both of
    # which require the silhouette to REFUSE while the terrain axes SURVIVE
    # within tolerance of the untouched reference.
    far_edge_contrast, far_band, near_band, horizon_cols_pct = silhouette(L, hz, sky)

    # sky_near_horizon_void_pct: diagnostic only, NEVER gates or scores anything.
    # sky_void_pct above is a whole-sky average; 2026-08-19 investigation of the
    # beach_dusk frame found the strip actually used by far_edge_contrast (right
    # at the skyline) is voider than the sky-wide average (52.7% vs 36.6%), which
    # rules out "the gate is measuring the wrong region" as the refusal's cause.
    sky_near_horizon_void_pct = None
    if scene == "outdoor" and sky.sum() >= 200:
        H_, W_ = L.shape
        valid_hz = (hz > 4) & (hz < H_ - 12)
        nh_vals = []
        for x in range(W_):
            if not valid_hz[x]:
                continue
            y = hz[x]
            col = L[max(0, y - 20):y, x]
            m = sky[max(0, y - 20):y, x]
            nh_vals.append(col[m])
        if nh_vals:
            nh = np.concatenate([v for v in nh_vals if v.size])
            if nh.size >= 200:
                sky_near_horizon_void_pct = float((nh < 10).mean() * 100)

    sky_present = scene == "outdoor" and sky_frac is not None and sky_frac >= 3.0
    horizon_ok = scene == "outdoor" and horizon_cols_pct >= 20.0
    sky_lit = (sky_present
               and sky_void_pct is not None and sky_void_pct <= 20.0
               and sky_blown_pct is not None and sky_blown_pct <= 20.0)

    if scene != "outdoor":
        depth_reason = "interior frame - no skyline to place a far band on"
    elif not sky_present:
        depth_reason = "no sky region (%.1f%% < 3%%) so no skyline" % (sky_frac or 0)
    elif not horizon_ok:
        depth_reason = "skyline on only %.0f%% of columns (< 20%%)" % horizon_cols_pct
    else:
        depth_reason = "ok"

    if depth_reason != "ok":
        sil_reason = depth_reason
    elif sky_void_pct > 20.0:
        sil_reason = ("sky unlit (%.0f%% below L=10) - contrast against a black void "
                      "is degenerate" % sky_void_pct)
    elif sky_blown_pct > 20.0:
        sil_reason = ("sky blown (%.0f%% above L=245) - contrast against a white void "
                      "is degenerate" % sky_blown_pct)
    else:
        sil_reason = "ok"

    # Keep the raw number even when we refuse to score it. Director's audit
    # 2026-08-19: "measure the real range of the input before touching the
    # threshold" - this is that measurement, published for transparency so
    # nobody has to re-derive it from pixels to see WHAT was refused, not
    # just WHY. It is never read by ratio()/AXES, so it cannot leak into a
    # pass/fail - it exists only so the scoreboard can quote a number next
    # to the refusal instead of asserting one blind.
    far_edge_contrast_raw = far_edge_contrast
    if not (sky_lit and horizon_ok):
        far_edge_contrast = None

    hp3 = np.abs(L - ndimage.gaussian_filter(L, 3.0))

    def band_stats(band):
        if band.sum() < 500:
            return None, None  # refused to measure, not "measured zero"
        s = float(sat[band][val[band] >= 25].mean() * 100) if (val[band] >= 25).any() else 0.0
        return s, float(hp3[band].std())

    far_sat, far_micro = band_stats(far_band)
    near_sat, near_micro = band_stats(near_band)
    if not horizon_ok:
        far_sat = far_micro = None
    depth_sat_ratio = (round(near_sat / far_sat, 3)
                       if far_sat and near_sat is not None and far_sat > 0.5 else None)
    depth_micro_ratio = (round(near_micro / far_micro, 3)
                         if far_micro and near_micro is not None and far_micro > 0.05 else None)

    # --- H. emissive point lights ----------------------------------------
    hot = (L > 210) & (sat > 0.10)
    lab, n = ndimage.label(hot)
    if n:
        sizes = ndimage.sum(hot, lab, range(1, n + 1))
        emissive_blobs = int(((sizes >= 4) & (sizes <= 2000)).sum())
    else:
        emissive_blobs = 0

    if dump_mask:
        write_mask(rgb, sky, hz, far_band, near_band, dump_mask)

    def rnd(v, d=2):
        return None if v is None else round(v, d)

    return {
        "file": path,
        # px is the NORMALISED geometry (LANCZOS -> NORM_AREA), i.e. the pixels every
        # axis below was actually measured on. It is NOT the file on disk: a 1280x720
        # plate reports 1333x750 here. px_file carries the real size so a table built
        # from this JSON can print both and nobody reads px as an asset resolution.
        "px": f"{W}x{H}",
        "px_file": f"{file_w}x{file_h}",
        "scene": scene,
        "L_p1": round(p[1], 2), "L_p5": round(p[5], 2), "L_p50": round(p[50], 2),
        "L_p95": round(p[95], 2), "L_p99": round(p[99], 2),
        "range_p5_p95": round(p[95] - p[5], 2),
        "crush_pct": round(crush, 2),
        "clip_pct": round(clip, 2),
        "tonal_bins": tonal_bins,
        "sat_mean": round(sat_mean, 2),
        "micro_r1": round(micro_r1, 3),
        "micro_r3": round(micro_r3, 3),
        "micro_r8": round(micro_r8, 3),
        "edge_density": round(edge_density, 3),
        "strong_edge_pct": round(strong_edge, 2),
        "flat_pct": round(flat_pct, 2),
        "hue_bins": hue_bins,
        "hue_entropy": round(hue_entropy, 3),
        "cool_chroma_pct": round(cool_pct, 2),
        "sky_frac_pct": rnd(sky_frac),
        "sky_void_pct": rnd(sky_void_pct),
        "sky_blown_pct": rnd(sky_blown_pct),
        "sky_L_mean": rnd(sky_L_mean),
        "sky_L_range": rnd(sky_L_range),
        "sky_hue_span_deg": rnd(sky_hue_span, 1),
        "sky_sat": rnd(sky_sat),
        "sky_ground_ratio": sky_ground_ratio,
        "horizon_cols_pct": rnd(horizon_cols_pct),
        "silhouette_measurable": sil_reason,
        "depth_measurable": depth_reason,
        "sky_measurable": sky_reason,
        "far_edge_contrast": rnd(far_edge_contrast),
        "far_edge_contrast_raw": rnd(far_edge_contrast_raw),
        "far_sat": rnd(far_sat),
        "near_sat": rnd(near_sat),
        "far_micro": rnd(far_micro, 3),
        "near_micro": rnd(near_micro, 3),
        "depth_sat_ratio": depth_sat_ratio,
        "depth_micro_ratio": depth_micro_ratio,
        "emissive_blobs": emissive_blobs,
    }


def circ_span(bins: np.ndarray, frac: float) -> float:
    """Minimal circular arc (deg) holding `frac` of the mass, over 10-deg bins."""
    tot = bins.sum()
    if tot <= 0:
        return 0.0
    b = bins / tot
    n = len(b)
    best = n
    for start in range(n):
        acc = 0.0
        for k in range(n):
            acc += b[(start + k) % n]
            if acc >= frac:
                best = min(best, k + 1)
                break
    return best * 10.0


def silhouette(L: np.ndarray, hz: np.ndarray, sky: np.ndarray):
    """Contrast right across the skyline + the far/near bands for depth cues.

    Measured PER COLUMN, so a blazing sun on one side cannot average itself
    against a dark hillside on the other and invent a silhouette that is not
    there. Columns with no skyline (hz==0) are skipped, not scored 0.

    Returns the raw contrast UNGUARDED - the caller decides whether the sky it
    was measured against was real enough to believe. Both bands are masked with
    `~sky` AND held SKY_BLEED px clear of it, so the claim "the depth axes never
    read a sky pixel" is enforced by the code rather than argued from geometry.

    The clearance is not cosmetic. `~sky` alone is not enough: far_micro reads a
    sigma=3 high-pass, whose kernel reaches ACROSS the skyline and drags sky
    luminance into terrain pixels near the border. Control C3 caught it - 21.4%
    of the reference's far band sits within 10 px of sky, and blacking the sky
    (which touches no terrain pixel at all) moved far_micro 15.70 -> 26.85.
    With the clearance it is 15.71 -> 15.71: sky-independent by measurement, not
    by assertion, which is the whole premise of gating these axes on the horizon
    instead of on the sky.
    """
    H, W = L.shape
    yy = np.arange(H)[:, None]
    valid = (hz > 4) & (hz < H - 12)
    diffs = []
    for x in range(W):
        if not valid[x]:
            continue
        y = hz[x]
        a = L[max(0, y - 4):y, x]
        b = L[y:min(H, y + 8), x]
        if a.size and b.size:
            diffs.append(abs(a.mean() - b.mean()))
    contrast = float(np.mean(diffs)) if len(diffs) >= 20 else None

    clear = ~ndimage.binary_dilation(sky, iterations=SKY_BLEED)
    depth = max(8, H // 12)
    hzb = hz[None, :]
    far_band = (yy >= hzb) & (yy < hzb + depth) & valid[None, :] & clear
    near_band = np.zeros((H, W), dtype=bool)
    near_band[int(H * 0.80):, :] = True
    near_band &= ~far_band & clear
    return contrast, far_band, near_band, float(valid.mean() * 100)


def write_mask(rgb, sky, hz, far_band, near_band, out):
    v = rgb.copy()
    v[sky] = v[sky] * 0.35 + np.array([0, 120, 255]) * 0.65
    v[far_band] = v[far_band] * 0.55 + np.array([255, 0, 200]) * 0.45
    v[near_band] = v[near_band] * 0.70 + np.array([80, 255, 80]) * 0.30
    H, W, _ = v.shape
    for x in range(W):
        y = min(H - 1, max(0, hz[x]))
        v[y, x] = [255, 255, 0]
    Path(out).parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(v.clip(0, 255).astype(np.uint8)).save(out)


# ------------------------------------------------------------ gap scoring
# direction: +1 = more is better, -1 = less is better
# mode:  "gate" = counts toward pass/GAP and toward the exit code
#        "adv"  = measured and printed, but NEVER judged. Rubric rule 7: a
#                 threshold the reference itself does not support is a phantom
#                 target, and this document has already paid for three of them.
AXES = [
    ("range_p5_p95", +1, "dynamic range", "renderer", "gate"),
    ("crush_pct", -1, "crushed blacks", "renderer", "gate"),
    ("clip_pct", -1, "clipped highlights", "renderer", "gate"),
    ("tonal_bins", +1, "tonal spread", "renderer", "gate"),
    ("sat_mean", +1, "saturation", "renderer", "gate"),
    ("hue_bins", +1, "palette breadth", "art", "gate"),
    ("hue_entropy", +1, "hue diversity", "art", "gate"),
    ("cool_chroma_pct", +1, "cool/water chroma", "world", "gate"),
    ("micro_r3", +1, "local contrast r3", "art", "gate"),
    ("edge_density", +1, "detail per area", "art", "gate"),
    ("flat_pct", -1, "flat/featureless area", "art", "gate"),
    ("sky_frac_pct", +1, "sky presence", "world", "gate"),
    ("sky_ground_ratio", +1, "sky brighter than ground", "renderer", "gate"),
    ("sky_void_pct", -1, "sky sitting at black (L<10)", "renderer", "gate"),
    ("sky_blown_pct", -1, "sky blown to white (L>245)", "renderer", "gate"),
    ("sky_L_range", +1, "sky tonal gradient", "renderer", "gate"),
    ("sky_hue_span_deg", +1, "sky hue range", "renderer", "gate"),
    ("far_edge_contrast", +1, "distant silhouette", "renderer", "gate"),
    ("far_micro", +1, "detail surviving at distance", "art", "gate"),
    ("depth_sat_ratio", +1, "atmospheric perspective (sat)", "renderer", "gate"),
    # ADVISORY, not a gate. near_micro / far_micro on the CEO reference is 12.67
    # / 15.61 = 0.81 - the reference's DISTANT band carries MORE detail than its
    # near band, so the reference does not exhibit the thing this axis is named
    # after. Grading a frame against 0.81 measures where the scene happens to
    # put its busy geometry, not whether haze is softening the distance, and
    # "beat 60% of 0.81" is a bar almost any frame clears for free. Its sibling
    # depth_sat_ratio IS supported (1.13 > 1, near more saturated than far) and
    # stays a gate. Re-promote this one only against an outdoor reference whose
    # own ratio is > 1, and log that control here.
    ("depth_micro_ratio", +1, "atmospheric perspective (detail)", "renderer", "adv"),
    ("emissive_blobs", +1, "emissive light points", "world", "gate"),
]

GATE = 0.60  # a frame passes an axis at >=60% of the reference (or <=1/0.60 for -1)

# ADVISORY ONLY - never changes an exit code, never turns an ok into a fail.
# It is the gate mirrored through the reference: 1/0.60 = 1.67x is exactly as
# far from the CEO frame as a GAP is, just on the other side. It exists because
# on 2026-08-18 this tool printed "2.20x" next to a frame whose sky was a hole
# and a human read it as "better than the reference". Nothing that far from the
# reference should render as a quiet tick. Only for direction=+1 axes: on a
# less-is-better axis, further from the reference means less of a bad thing.
OVER = 1.0 / GATE


def ratio(cur, ref, direction) -> float | None:
    if cur is None or ref is None:
        return None
    if direction > 0:
        return cur / ref if ref > 1e-9 else None
    # less-is-better, with a floor so 0-vs-0 is a pass not a divide-by-zero
    return (ref + 0.5) / (cur + 0.5)


def status(rt, direction, mode="gate") -> str:
    """One of: SKIP (refused to measure) / GAP / over (advisory) / ok / adv."""
    if rt is None:
        return "SKIP"
    if mode == "adv":
        return "adv"
    if rt < GATE:
        return "GAP"
    if direction > 0 and rt >= OVER:
        return "over"
    return "ok"


SIL_KEYS = {"far_edge_contrast"}
DEPTH_KEYS = {"far_micro", "near_micro", "depth_sat_ratio", "depth_micro_ratio"}
SKY_KEYS = {"sky_frac_pct", "sky_ground_ratio", "sky_void_pct", "sky_blown_pct",
            "sky_L_range", "sky_hue_span_deg", "sky_sat", "sky_L_mean"}


def refusal_reason(key: str, frame: dict, ref: dict) -> str:
    """Why THIS axis produced no ratio, in its own words.

    An axis must never borrow another axis's excuse. The v1 scoreboard printed
    `depth_measurable` beside anything that was not the silhouette, which would
    have explained an interior frame's missing sky with a sentence about far
    bands. It also could not say the one thing that is not the frame's fault at
    all: a reference that measures 0 on a more-is-better axis makes the ratio
    undefined, which is a REFERENCE limitation, not a refusal to measure us.
    """
    if frame.get(key) is None:
        if key in SIL_KEYS:
            return frame.get("silhouette_measurable") or "silhouette not measurable"
        if key in DEPTH_KEYS:
            return frame.get("depth_measurable") or "depth bands not measurable"
        if key in SKY_KEYS:
            return frame.get("sky_measurable") or "no sky region in this frame"
        return "the grader produced no value on this axis"
    if ref.get(key) is None:
        return "the REFERENCE has no value on this axis - nothing to compare against"
    return ("the REFERENCE measures 0 on this axis, so 'percent of reference' is "
            "undefined - this is a limit of the reference, not of the frame")


def raw_pct(cur, ref) -> str:
    """The value as a plain % of the reference value - NOT the gate ratio.

    The gate ratio for a less-is-better axis is (ref+.5)/(cur+.5), which renders
    "we clip 0.16% where the reference clips 10.30%" as **1636%** and reads, to
    anyone skimming, as sixteen times better than the CEO frame. The raw share
    says 2% and a "less is better" marker says the rest.
    """
    if cur is None or ref is None:
        return "–"
    if abs(ref) < 1e-9:
        return "–"
    return f"{cur / ref * 100:.0f}%"


def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


# ------------------------------------------------------- control gate (rule 7)

def run_controls() -> tuple[int, str]:
    """Rubric rule 7: no control = no cut line. Run the harness, return its verdict.

    Deliberately a subprocess and not an import: the harness must exercise the
    grader exactly the way a human would run it, and its exit code is the thing
    being trusted.
    """
    ctl = Path(__file__).with_name("_pixel_artgap_controls.py")
    if not ctl.exists():
        return 127, f"control harness missing: {ctl}"
    p = subprocess.run([sys.executable, str(ctl)], capture_output=True, text=True)
    tail = (p.stdout or "").strip().splitlines()
    return p.returncode, "\n".join(tail[-3:]) if tail else (p.stderr or "").strip()[-400:]


# ------------------------------------------------------------- the scoreboard

STATUS_MD = {"ok": "✅ ok", "GAP": "❌ GAP", "over": "⚠️ over",
             "SKIP": "⏸ ยังวัดไม่ได้", "adv": "📎 advisory"}


def delta_cell(key, cur, prev, direction):
    """Movement since the previous run of THIS frame, in % of REF, or '-'."""
    if prev is None or cur is None or prev.get(key) is None:
        return "–"
    d = cur - prev[key]
    if abs(d) < 1e-9:
        return "0"
    arrow = "▲" if (d > 0) == (direction > 0) else "▼"
    return f"{arrow} {d:+.2f}"


def write_scoreboard(out, ref_path, ref, rows, prev_by_file, ctl_rc, ctl_tail,
                     stamp, label):
    primary = rows[0]
    counts = {"ok": 0, "GAP": 0, "over": 0, "SKIP": 0, "adv": 0}
    lines = []
    for key, direction, lab, owner, mode in AXES:
        rt = ratio(primary[key], ref[key], direction)
        st = status(rt, direction, mode)
        counts[st] += 1
        lines.append((st, rt if rt is not None else 0.0,
                      (lab + (" ↓" if direction < 0 else ""), owner,
                       fmt(ref[key]), fmt(primary[key]),
                       raw_pct(primary[key], ref[key]), STATUS_MD[st],
                       delta_cell(key, primary[key],
                                  prev_by_file.get(primary["file"]), direction))))

    n = sum(1 for *_, mode in AXES if mode == "gate")
    scored = counts["ok"] + counts["over"] + counts["GAP"]
    prev = prev_by_file.get(primary["file"])
    same_bytes = prev is not None and prev.get("sha256") == primary.get("sha256")

    md = []
    A = md.append
    A("# 🏁 AAA Scoreboard — LIVE")
    A("")
    A("> **ไฟล์นี้ถูกเครื่องสร้างขึ้นใหม่ทุกครั้งที่รัน grader — ห้ามแก้ด้วยมือ.**  ")
    A("> สร้างโดย `scripts/_pixel_artgap_grade.py --oneshot` · "
      "เกณฑ์ทั้งหมดอยู่ใน [`docs/look-acceptance-rubric.md` §P0-ENV]"
      "(look-acceptance-rubric.md#-p0-env--ท้องฟ้า--น้ำ--environment-scripts_pixel_artgap_gradepy--เพิ่ม-2026-08-18)")
    A("")
    A(f"**รอบล่าสุด:** {stamp}" + (f" · **{label}**" if label else ""))
    A("")

    # --- the one thing to read first ---------------------------------------
    if ctl_rc != 0:
        A("## 🚨 ตัวเลขข้างล่างนี้ **เชื่อไม่ได้**")
        A("")
        A(f"`_pixel_artgap_controls.py` **exit {ctl_rc}** — เครื่องวัดยังพิสูจน์ตัวเองไม่ผ่าน "
          "กติกาข้อ 7 ของ rubric บอกว่า **ไม่มี control = ไม่มีเส้นตัด** ⇒ อย่าเพิ่งเอาเลขนี้ไปตัดสินใจอะไร")
        A("")
        A("```")
        A(ctl_tail)
        A("```")
    else:
        # ONE denominator in the headline. The v1 heading said "9 / 21" and the
        # line under it said "9/20 = 45%", which is two different scores for the
        # same frame on the same screen - the first thing a CEO reads should not
        # need a reconciliation. The denominator is the axes actually MEASURED
        # this round; the refused ones are counted beside it, never silently
        # folded in as failures (that would score a refusal as a defect).
        passed = counts["ok"] + counts["over"]
        head = (f"## {passed} / {scored} แกนผ่าน = "
                f"**{passed / scored * 100:.0f}% ของภาพอ้างอิง CEO**" if scored
                else "## ยังไม่มีแกนไหนวัดได้ในรอบนี้")
        A(head + f"  ·  {counts['GAP']} GAP  ·  {counts['SKIP']} ยังวัดไม่ได้"
          + (f"  ·  {counts['adv']} advisory" if counts['adv'] else ""))
        A("")
        A(f"เฟรมที่ตัดสิน: **`{primary['file']}`** ({primary['px']} normalised · "
          f"ไฟล์จริง {primary['px_file']}) "
          f"เทียบ **`{ref_path}`** ({ref['px']} normalised · ไฟล์จริง {ref['px_file']})  ")
        A(f"**คอลัมน์/ค่า `px` ทุกที่ในเอกสารนี้ = ขนาด _หลัง_ normalise** — LANCZOS ย่อ/ขยาย "
          f"ทุกเฟรมลงพื้นที่ร่วม {NORM_AREA/1e6:.1f} Mpx ก่อนวัด (`load()`) ⇒ **เทียบข้ามเพลตได้** "
          f"เพราะทุกใบมีจำนวนพิกเซลเท่ากัน แต่ **ห้ามอ่านเป็นความละเอียดของ asset/เฟรมจริง** "
          f"— ขนาดไฟล์จริงคือค่าที่กำกับว่า “ไฟล์จริง” ข้างบน  ")
        A(f"เครื่องวัดผ่าน control แล้ว (`_pixel_artgap_controls.py` **exit 0**) "
          f"⇒ เลขข้างล่างเชื่อได้ตามกติกาข้อ 7")
        A("")
        A(f"> **ตัวหาร {scored} = แกนที่วัดได้จริงรอบนี้** จากด่านทั้งหมด {n} แกน — "
          f"อีก {counts['SKIP']} แกนเครื่อง**ปฏิเสธที่จะวัด** จึงไม่ถูกนับเป็นทั้งผ่านและตก "
          f"(นับเป็นตก = ให้คะแนนการปฏิเสธเป็นความผิดของเฟรม) · "
          f"advisory อีก {counts['adv']} แกนวัดแล้วแต่ไม่ตัดสิน")
        if counts["over"]:
            A("")
            A(f"> ⚠️ **ในจำนวนที่ \"ผ่าน\" นั้น {counts['over']} แกนอยู่ในสถานะ `over`** — "
              f"ผ่านเกตจริงตามคณิตศาสตร์ แต่ **ไกลจากภาพอ้างอิงเกิน {OVER:.0%}** "
              "ซึ่งไกลเท่ากับ GAP แค่คนละฝั่ง. เปิด mask ดูก่อนดีใจ: "
              "เดือน 8/2026 ด่านนี้เคยพิมพ์ `2.20x` ข้างเฟรมที่ท้องฟ้าเป็นรูโหว่ "
              "แล้วมีคนอ่านว่า \"ดีกว่าภาพอ้างอิง\"")
    A("")
    if same_bytes:
        A("> ⚠️ **เฟรมนี้ไบต์เดิมกับรอบก่อน** (sha256 ตรงกัน) — นี่ไม่ใช่ build ใหม่ "
          "คอลัมน์ Δ จึงเป็น 0 ทั้งแถวโดยธรรมชาติ ไม่ใช่ \"แก้แล้วไม่ขยับ\"")
        A("")

    # --- the table ----------------------------------------------------------
    A("## แกนทั้งหมด — เทียบภาพอ้างอิงของ CEO เป็น %")
    A("")
    A("| แกน | เจ้าของเลน | REF | เฟรมเรา | % ของ REF | สถานะ | Δ จากรอบก่อน |")
    A("|---|---|---:|---:|---:|---|---:|")
    # worst first: a CEO reading top-to-bottom meets the gaps before the ticks
    order = {"GAP": 0, "over": 1, "SKIP": 2, "adv": 3, "ok": 4}
    for _, _, row in sorted(lines, key=lambda r: (order[r[0]], r[1])):
        A("| " + " | ".join(row) + " |")
    A("")
    A("**`↓` ต่อท้ายชื่อแกน = แกนที่ \"น้อยกว่าดีกว่า\"** — คอลัมน์ `% ของ REF` เป็น "
      "*ค่าดิบเทียบค่าดิบ* ไม่ใช่คะแนน ⇒ แกน ↓ ที่ได้ 2% คือ **ดี** (เราเหลือ 2% ของสิ่งที่ REF มี)  ")
    A("คอลัมน์ Δ: **▲ = ขยับเข้าหาภาพอ้างอิง · ▼ = ถอยห่าง** (ตัวเลขคือส่วนต่างดิบ "
      "จึงเป็น `▲ -36.44` ได้ ถ้าแกนนั้นน้อยกว่าดีกว่า) · `–` = รอบก่อนไม่มีตัวเลขให้เทียบ")
    A("")
    A(f"`ok` = ≥ {GATE:.0%} ของ REF (ตามทิศของแกน) · `GAP` = ต่ำกว่านั้น · "
      f"`over` = **ไกลจาก REF เกิน {OVER:.0%}** (advisory เฉย ๆ ไม่ตัด FAIL — "
      f"มีไว้กันคนอ่านตัวเลขไกลจาก REF แล้วนึกว่า \"ดีกว่า ref\") · "
      f"`advisory` = **วัดแล้วแต่ไม่ตัดสิน** (ภาพอ้างอิงเองไม่รองรับเส้นตัดของแกนนั้น) · "
      f"`ยังวัดไม่ได้` = เครื่องมือ **ปฏิเสธที่จะวัด** ไม่ใช่วัดแล้วได้ศูนย์")
    A("")
    adv = [(lab, owner) for (key, d, lab, owner, mode) in AXES if mode == "adv"]
    if adv:
        A("<details><summary>ทำไม " + ", ".join(f"**{l}**" for l, _ in adv) +
          " ถึงเป็น advisory ไม่ใช่ด่าน</summary>")
        A("")
        A("`depth_micro_ratio` = `micro(near) / micro(far)`. บนภาพอ้างอิงของ CEO ค่านี้ = "
          f"**{fmt(ref['depth_micro_ratio'])}** (near {fmt(ref['near_micro'])} / far "
          f"{fmt(ref['far_micro'])}) — คือ **แถบไกลของภาพอ้างอิงมีรายละเอียดมากกว่าแถบใกล้**. "
          "แปลว่า *ภาพอ้างอิงเองไม่ได้แสดงสิ่งที่แกนนี้ตั้งชื่อไว้* การเอาเฟรมเราไปเทียบกับ 0.81 "
          "จึงวัดว่า \"ฉากวางของรกไว้ตรงไหน\" ไม่ได้วัดว่าหมอกทำให้ระยะไกลนุ่มลงไหม — และ "
          "\"ทำให้ได้ 60% ของ 0.81\" เป็นบาร์ที่เฟรมไหนก็ข้ามได้ฟรี. กติกาข้อ 7: **เส้นตัดที่ภาพ"
          "อ้างอิงเองไม่รองรับ = phantom target** เอกสารนี้จ่ายค่ามันมาแล้ว 3 ครั้ง.")
        A("")
        A("ฝาแฝดของมัน `depth_sat_ratio` **ยังเป็นด่านอยู่** เพราะ REF ได้ "
          f"{fmt(ref['depth_sat_ratio'])} > 1 (ใกล้อิ่มสีกว่าไกล) = รองรับชื่อแกนตัวเอง. "
          "จะเลื่อน `depth_micro_ratio` กลับมาเป็นด่านได้ ต้องมี ref กลางแจ้งที่ค่านี้ > 1 "
          "แล้วบันทึก control ไว้ก่อน")
        A("")
        A("</details>")
        A("")

    # --- what is refusing, and what unblocks it -----------------------------
    A("## ที่ยังวัดไม่ได้ — ปฏิเสธเพราะอะไร และใครปลดล็อก")
    A("")
    refused = [(key, lab, owner) for (key, direction, lab, owner, _m) in AXES
               if ratio(primary[key], ref[key], direction) is None]
    if not refused:
        A("**ไม่มี** — ทุกแกนวัดได้หมดในรอบนี้")
    else:
        A("| แกน | เหตุผลที่เครื่องปฏิเสธ (คำต่อคำจาก grader) |")
        A("|---|---|")
        for key, lab, owner in refused:
            A(f"| {lab} ({owner}) | `{refusal_reason(key, primary, ref)}` |")
        A("")
        # This paragraph used to be a fixed string about `distant silhouette` and
        # the two sky-luminance numbers. On an INTERIOR frame it rendered
        # "ต้องการฟ้าที่มีโทน (ตอนนี้ sky_void_pct = -%)" - a caption describing a
        # condition the frame cannot have, quoting numbers that do not exist.
        # Say only what is true of the frame in hand.
        if primary.get("scene") != "outdoor":
            A("**ปลดล็อกยังไง:** เฟรมนี้เป็น **interior** — แกนกลุ่มฟ้า/ระยะไกล"
              "ไม่มีอยู่จริงในเฟรมแบบนี้ จึงไม่ใช่ข้อบกพร่องและจะไม่ปลดล็อกด้วยการแก้ภาพ. "
              "ถ้าอยากให้แกนพวกนี้ถูกวัด ต้องส่ง**เฟรมกลางแจ้ง**มาเกรด")
        else:
            A("**ปลดล็อกยังไง:** แกนพวกนี้จะกลับมาวัดได้ **เอง** ทันทีที่เงื่อนไขข้างบนหาย — "
              "ไม่มีใครต้องแก้เครื่องวัด (พิสูจน์แล้ว: เฟรมที่ฟ้ามีโทนวัด "
              "`distant silhouette` ได้ตามปกติ)")
            if (primary.get("sky_void_pct") is not None
                    and primary.get("sky_blown_pct") is not None):
                A("")
                A(f"`distant silhouette` ต้องการฟ้าที่ *มีโทน*: ตอนนี้ `sky_void_pct` = "
                  f"**{fmt(primary['sky_void_pct'])}%** (ต้อง ≤ 20) และ `sky_blown_pct` = "
                  f"**{fmt(primary['sky_blown_pct'])}%** (ต้อง ≤ 20)")
            if primary.get("far_edge_contrast_raw") is not None:
                A("")
                A(f"ค่าดิบที่วัดได้จริง (ไม่ถูกใช้ให้คะแนน เพราะฟ้าเป็นโพรงดำ): "
                  f"`far_edge_contrast_raw` = **{fmt(primary['far_edge_contrast_raw'])}** "
                  f"vs REF **{fmt(ref['far_edge_contrast_raw'])}** — เลขนี้คือ contrast ชนความว่างเปล่า "
                  f"ไม่ใช่ contrast ชนขอบฟ้าจริง จึงยังไม่นับเป็นแกนที่วัดได้")
    A("")

    # --- instrument health --------------------------------------------------
    A("## สุขภาพเครื่องวัด (ต้องดูก่อนเถียงเรื่องเลข)")
    A("")
    A("| | REF | เฟรมเรา |")
    A("|---|---:|---:|")
    A(f"| sky mask กินพื้นที่ | {fmt(ref['sky_frac_pct'])}% | "
      f"{fmt(primary['sky_frac_pct'])}% |")
    A(f"| คอลัมน์ที่มีเส้นขอบฟ้า | {fmt(ref['horizon_cols_pct'])}% | "
      f"{fmt(primary['horizon_cols_pct'])}% |")
    A(f"| ฟ้าดำสนิท (L<10) | {fmt(ref['sky_void_pct'])}% | "
      f"{fmt(primary['sky_void_pct'])}% |")
    A(f"| ฟ้าไหม้ขาว (L>245) | {fmt(ref['sky_blown_pct'])}% | "
      f"{fmt(primary['sky_blown_pct'])}% |")
    A("")
    A("mask overlay เขียนออกมาทุกครั้ง (ฟ้า=น้ำเงิน · far=ชมพู · near=เขียว · "
      "เส้นขอบฟ้า=เหลือง) — **ดู mask ก่อนเถียงเรื่องตัวเลข**")
    A("")

    # --- other frames -------------------------------------------------------
    if len(rows) > 1:
        A("## เฟรมอื่นในรอบเดียวกัน")
        A("")
        A("| เฟรม | ผ่าน | GAP | ยังวัดไม่ได้ |")
        A("|---|---:|---:|---:|")
        for r in rows[1:]:
            c = {"ok": 0, "GAP": 0, "over": 0, "SKIP": 0, "adv": 0}
            for key, direction, _, _, mode in AXES:
                c[status(ratio(r[key], ref[key], direction), direction, mode)] += 1
            A(f"| `{r['file']}` | {c['ok'] + c['over']} | {c['GAP']} | {c['SKIP']} |")
        A("")

    # --- reproduce ----------------------------------------------------------
    A("## รันซ้ำ (คำสั่งเดียว ต่อ build ใหม่ 1 ตัว)")
    A("")
    A("```bash")
    A(f"python scripts/_pixel_artgap_grade.py {rows[0]['file']} \\")
    for r in rows[1:]:
        A(f"       {r['file']} \\")
    A("       --oneshot --label \"<ใครส่ง build / commit>\"")
    A("```")
    A("")
    A("`--oneshot` = รัน control ก่อนเสมอ (control ตก ⇒ **exit 3** และ **ไม่พิมพ์เลขให้เชื่อ**) "
      "→ เกรด → เขียน JSON + mask + history → สร้างหน้านี้ใหม่  ")
    A("exit: `0` ผ่านหมด · `1` มี GAP · `2` วัดครบแต่มีแกนที่ปฏิเสธจะวัด · `3` control ตก")
    A("")
    A("| ไฟล์ | sha256 (12 ตัวแรก) |")
    A("|---|---|")
    A(f"| `{ref_path}` (REF) | `{ref.get('sha256', '')[:12]}` |")
    for r in rows:
        A(f"| `{r['file']}` | `{r.get('sha256', '')[:12]}` |")

    Path(out).parent.mkdir(parents=True, exist_ok=True)
    Path(out).write_text("\n".join(md) + "\n", encoding="utf-8")


def main():
    ap = argparse.ArgumentParser(
        epilog="frame syntax: PATH or PATH=interior (interior frames SKIP the sky axes)")
    ap.add_argument("frames", nargs="+")
    ap.add_argument("--ref", default="docs/refs/ceo_ref_sunset_valley.jpg")
    ap.add_argument("--json", default=None)
    ap.add_argument("--mask-dir", default=None)
    ap.add_argument("--scoreboard", default=None)
    ap.add_argument("--history", default=None)
    ap.add_argument("--label", default="", help="who sent this build / which commit")
    ap.add_argument("--check-controls", action="store_true",
                    help="rubric rule 7: run _pixel_artgap_controls.py first and "
                         "refuse to print numbers (exit 3) if it does not exit 0")
    ap.add_argument("--oneshot", action="store_true",
                    help="the per-build command: --check-controls --gate plus the "
                         "standard json / mask / history / scoreboard paths")
    ap.add_argument("--gate", action="store_true",
                    help="exit 1 if any frame fails an axis, 2 if an axis could not be measured")
    a = ap.parse_args()

    # ---- the de-HUD guard, before ANY output ------------------------------
    # First statement after parsing on purpose: a refusal must leave no number
    # behind for a human to read out of context, and the very next block prints
    # "rule 7: verifying the instrument ...". Strip the `=interior` scene suffix
    # first — the guard is asked about a FILE, and `frame.png=interior` is a
    # filename it would refuse for the wrong reason. `--ref` is checked in the
    # same call because it is graded too (measure() runs on it and its numbers
    # are the REF column); it passes by IDENTITY through
    # nohud2_guard.REFERENCE_ARTWORK, not by suffix — the CEO's artwork does not
    # get renamed to satisfy a capture-provenance rule.
    require_nohud2([spec.partition("=")[0] for spec in a.frames] + [a.ref],
                   tool="_pixel_artgap_grade.py")

    if a.oneshot:
        a.check_controls = a.gate = True
        a.json = a.json or "docs/assets/artgap/artgap.json"
        a.mask_dir = a.mask_dir or "docs/assets/artgap/masks"
        a.history = a.history or "docs/assets/artgap/history.jsonl"
        a.scoreboard = a.scoreboard or "docs/aaa-scoreboard-live.md"

    # ---- rule 7 first, before a single number is printed -------------------
    ctl_rc, ctl_tail = 0, "not run"
    if a.check_controls:
        print("rule 7: verifying the instrument before trusting it "
              "(_pixel_artgap_controls.py) ...")
        ctl_rc, ctl_tail = run_controls()
        if ctl_rc != 0:
            print(f"\n** CONTROL HARNESS EXIT {ctl_rc} — REFUSING TO GRADE **")
            print(ctl_tail)
            print("\nNo control = no cut line (rubric rule 7). Fix the instrument, "
                  "then re-run. No numbers were printed on purpose.")
            sys.exit(3)
        print(f"controls OK (exit 0). {ctl_tail.splitlines()[0] if ctl_tail else ''}\n")

    stamp = datetime.now().astimezone().strftime("%Y-%m-%d %H:%M %z")

    def maskpath(p):
        if not a.mask_dir:
            return None
        return str(Path(a.mask_dir) / (Path(p).stem + "_mask.png"))

    ref = measure(a.ref, maskpath(a.ref), "outdoor")
    ref["sha256"] = sha256(a.ref)
    rows = []
    for spec in a.frames:
        path, _, scene = spec.partition("=")
        m = measure(path, maskpath(path), scene or "outdoor")
        m["sha256"] = sha256(path)
        rows.append(m)

    print(f"REF = {a.ref}  ({ref['px']} after LANCZOS normalise to "
          f"{NORM_AREA/1e6:.1f} Mpx; file on disk is {ref['px_file']})")
    print("px = measured geometry, comparable across plates; NOT the asset resolution\n")
    hdr = f"{'axis':<38}{'owner':<10}{'REF':>9}" + "".join(
        f"{Path(r['file']).stem[:17]:>19}" for r in rows)
    print(hdr)
    print("-" * len(hdr))
    failed = skipped = over = 0
    for key, direction, label, owner, mode in AXES:
        tag = label + (" [adv]" if mode == "adv" else "")
        line = f"{tag:<38}{owner:<10}{fmt(ref[key]):>9}"
        for r in rows:
            rt = ratio(r[key], ref[key], direction)
            st = status(rt, direction, mode)
            failed += st == "GAP"
            skipped += st == "SKIP"
            over += st == "over"
            cell = (f"{fmt(r[key])}  SKIP" if st == "SKIP"
                    else f"{fmt(r[key])} {rt:.2f}x {st}")
            line += f"{cell:>19}"
        print(line)
    print(f"\n{failed} axis-GAPs, {skipped} unmeasurable, {over} over-advisory, "
          f"across {len(rows)} frame(s) (gate = {GATE:.0%} of REF, "
          f"over-advisory = {OVER:.0%}; [adv] axes are measured but never judged)")

    # ---- previous run of each frame, for the delta column ------------------
    prev_by_file = {}
    if a.history and Path(a.history).exists():
        for ln in Path(a.history).read_text(encoding="utf-8").splitlines():
            try:
                rec = json.loads(ln)
            except ValueError:
                continue
            if rec.get("file"):
                prev_by_file[rec["file"]] = rec

    if a.json:
        Path(a.json).parent.mkdir(parents=True, exist_ok=True)
        Path(a.json).write_text(
            json.dumps({"stamp": stamp, "label": a.label, "controls_exit": ctl_rc,
                        "ref": ref, "frames": rows}, indent=2), encoding="utf-8")
        print(f"wrote {a.json}")

    if a.scoreboard:
        write_scoreboard(a.scoreboard, a.ref, ref, rows, prev_by_file,
                         ctl_rc, ctl_tail, stamp, a.label)
        print(f"wrote {a.scoreboard}")

    # appended AFTER the scoreboard reads it, so "previous" means previous run
    if a.history:
        Path(a.history).parent.mkdir(parents=True, exist_ok=True)
        with open(a.history, "a", encoding="utf-8") as f:
            for r in rows:
                f.write(json.dumps({"stamp": stamp, "label": a.label,
                                    "controls_exit": ctl_rc, **r}) + "\n")
        print(f"appended {len(rows)} row(s) to {a.history}")

    if a.gate:
        sys.exit(1 if failed else (2 if skipped else 0))


def fmt(v):
    if v is None:
        return "-"
    if isinstance(v, float):
        return f"{v:.2f}"
    return str(v)


if __name__ == "__main__":
    main()
