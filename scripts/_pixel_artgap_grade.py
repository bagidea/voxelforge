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
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

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
        sky_void_pct = sky_L_range = sky_L_mean = sky_hue_span = sky_sat = None
        sky_ground_ratio = None
    elif sky.sum() >= 200:
        Ls = L[sky]
        sky_void_pct = float((Ls < 10).mean() * 100)
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
        sky_void_pct = sky_L_range = sky_L_mean = sky_hue_span = sky_sat = 0.0
        sky_ground_ratio = None

    # --- G. distant silhouette + atmospheric perspective ------------------
    # DEGENERACY GUARD. Both axes compare the far field against the sky. If the
    # sky is a hole, |L_sky - L_terrain| is maximal for the worst possible
    # reason: v1 handed _matmaps_after 49.04 vs REF 22.33 = "2.20x BETTER than
    # the reference" for having no sky at all. Contrast against nothing is not a
    # silhouette. Refuse to measure instead of paying it a compliment.
    far_edge_contrast, far_band, near_band = silhouette(L, hz)
    sky_ok = (scene == "outdoor" and sky_frac is not None and sky_frac >= 3.0
              and sky_void_pct is not None and sky_void_pct <= 20.0)
    depth_reason = ("ok" if sky_ok else
                    "no sky to measure against" if (sky_frac or 0) < 3.0 else
                    "sky unlit (%.0f%% below L=10) - contrast against it is degenerate"
                    % (sky_void_pct or 0))
    if not sky_ok:
        far_edge_contrast = None
    hp3 = np.abs(L - ndimage.gaussian_filter(L, 3.0))

    def band_stats(band):
        if band.sum() < 500:
            return None, None  # refused to measure, not "measured zero"
        s = float(sat[band][val[band] >= 25].mean() * 100) if (val[band] >= 25).any() else 0.0
        return s, float(hp3[band].std())

    far_sat, far_micro = band_stats(far_band)
    near_sat, near_micro = band_stats(near_band)
    if not sky_ok:
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
        "px": f"{W}x{H}",
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
        "sky_L_mean": rnd(sky_L_mean),
        "sky_L_range": rnd(sky_L_range),
        "sky_hue_span_deg": rnd(sky_hue_span, 1),
        "sky_sat": rnd(sky_sat),
        "sky_ground_ratio": sky_ground_ratio,
        "depth_measurable": depth_reason,
        "far_edge_contrast": rnd(far_edge_contrast),
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


def silhouette(L: np.ndarray, hz: np.ndarray):
    """Contrast right across the skyline + the far/near bands for depth cues.

    Measured PER COLUMN, so a blazing sun on one side cannot average itself
    against a dark hillside on the other and invent a silhouette that is not
    there. Columns with no skyline (hz==0) are skipped, not scored 0.
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

    depth = max(8, H // 12)
    hzb = hz[None, :]
    far_band = (yy >= hzb) & (yy < hzb + depth) & valid[None, :]
    near_band = np.zeros((H, W), dtype=bool)
    near_band[int(H * 0.80):, :] = True
    near_band &= ~far_band
    return contrast, far_band, near_band


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
AXES = [
    ("range_p5_p95", +1, "dynamic range", "renderer"),
    ("crush_pct", -1, "crushed blacks", "renderer"),
    ("clip_pct", -1, "clipped highlights", "renderer"),
    ("tonal_bins", +1, "tonal spread", "renderer"),
    ("sat_mean", +1, "saturation", "renderer"),
    ("hue_bins", +1, "palette breadth", "art"),
    ("hue_entropy", +1, "hue diversity", "art"),
    ("cool_chroma_pct", +1, "cool/water chroma", "world"),
    ("micro_r3", +1, "local contrast r3", "art"),
    ("edge_density", +1, "detail per area", "art"),
    ("flat_pct", -1, "flat/featureless area", "art"),
    ("sky_frac_pct", +1, "sky presence", "world"),
    ("sky_ground_ratio", +1, "sky brighter than ground", "renderer"),
    ("sky_void_pct", -1, "sky sitting at black (L<10)", "renderer"),
    ("sky_L_range", +1, "sky tonal gradient", "renderer"),
    ("sky_hue_span_deg", +1, "sky hue range", "renderer"),
    ("far_edge_contrast", +1, "distant silhouette", "renderer"),
    ("far_micro", +1, "detail surviving at distance", "art"),
    ("depth_sat_ratio", +1, "atmospheric perspective (sat)", "renderer"),
    ("depth_micro_ratio", +1, "atmospheric perspective (detail)", "renderer"),
    ("emissive_blobs", +1, "emissive light points", "world"),
]

GATE = 0.60  # a frame passes an axis at >=60% of the reference (or <=1/0.60 for -1)


def ratio(cur, ref, direction) -> float | None:
    if cur is None or ref is None:
        return None
    if direction > 0:
        return cur / ref if ref > 1e-9 else None
    # less-is-better, with a floor so 0-vs-0 is a pass not a divide-by-zero
    return (ref + 0.5) / (cur + 0.5)


def main():
    ap = argparse.ArgumentParser(
        epilog="frame syntax: PATH or PATH=interior (interior frames SKIP the sky axes)")
    ap.add_argument("frames", nargs="+")
    ap.add_argument("--ref", default="docs/refs/ceo_ref_sunset_valley.jpg")
    ap.add_argument("--json", default=None)
    ap.add_argument("--mask-dir", default=None)
    ap.add_argument("--gate", action="store_true",
                    help="exit 1 if any frame fails an axis, 2 if an axis could not be measured")
    a = ap.parse_args()

    def maskpath(p):
        if not a.mask_dir:
            return None
        return str(Path(a.mask_dir) / (Path(p).stem + "_mask.png"))

    ref = measure(a.ref, maskpath(a.ref), "outdoor")
    rows = []
    for spec in a.frames:
        path, _, scene = spec.partition("=")
        rows.append(measure(path, maskpath(path), scene or "outdoor"))

    print(f"REF = {a.ref}  ({ref['px']} normalised)\n")
    hdr = f"{'axis':<30}{'owner':<10}{'REF':>9}" + "".join(
        f"{Path(r['file']).stem[:17]:>19}" for r in rows)
    print(hdr)
    print("-" * len(hdr))
    failed = skipped = 0
    for key, direction, label, owner in AXES:
        line = f"{label:<30}{owner:<10}{fmt(ref[key]):>9}"
        for r in rows:
            rt = ratio(r[key], ref[key], direction)
            if rt is None:
                cell = f"{fmt(r[key])}  SKIP"
                skipped += 1
            elif rt < GATE:
                cell = f"{fmt(r[key])} {rt:.2f}x GAP"
                failed += 1
            else:
                cell = f"{fmt(r[key])} {rt:.2f}x ok"
            line += f"{cell:>19}"
        print(line)
    print(f"\n{failed} axis-GAPs, {skipped} unmeasurable, across {len(rows)} frame(s) "
          f"(gate = {GATE:.0%} of REF)")

    if a.json:
        Path(a.json).parent.mkdir(parents=True, exist_ok=True)
        Path(a.json).write_text(json.dumps({"ref": ref, "frames": rows}, indent=2), encoding="utf-8")
        print(f"wrote {a.json}")

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
