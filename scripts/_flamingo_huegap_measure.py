#!/usr/bin/env python3
"""Flamingo - the four numbers the "kill the全-frame orange" order asks for.

READS PIXELS ONLY. Writes nothing unless --masks=DIR is given. Never touches
.rs / assets / maps.

THE FOUR AXES (Director's order, 2026-08-20)
--------------------------------------------
  a) sky_hue      mean hue over the SKY MASK must sit in the cool zone
                  (blue / cool grey), not at ~20 deg orange.
  b) grass_green  the GRASS MASK must actually be green.
  c) range        frame dynamic range must climb from 37-70 to 150+
                  (the CEO rain reference measures 188).
  d) zones        one frame must contain THREE colour zones that genuinely
                  differ.

WHY EACH METRIC IS SHAPED THE WAY IT IS
---------------------------------------
* SKY MASK is `_sun_skyground_measure.sky_mask_invariant` verbatim - the
  contrast-normalised Sobel flood-fill. NOT the grader's raw-Sobel mask, which
  is exposure-dependent and would move the mask when the fix moves the exposure,
  making a before/after unfalsifiable. Importing Sun's fixed one means the mask
  is the same region on both plates for the same geometry.

* SKY HUE is a CHROMA-WEIGHTED CIRCULAR MEAN. Hue is undefined at zero chroma,
  so an unweighted mean over a near-grey sky returns whatever the dither happens
  to be. Weighting by chroma makes a grey sky report "no strong hue" (low
  `sky_chroma`) instead of reporting a confident lie. `sky_warm_RmB` is quoted
  beside it as the falsifiable half: a warm sky has R > B in the median, a cool
  or neutral one does not, and that reading survives zero chroma.

* GRASS MASK is picked from ALBEDO GEOMETRY, not from a hue window. Selecting
  "the green pixels" and then measuring how green they are is circular - it
  passes by construction on any frame that has one green pixel. Instead the mask
  is the region the world builder put grass in: the top-face-lit band in the
  lower 60% of the frame whose GREEN CHANNEL IS THE MAX of the three (a strictly
  weaker condition than "is green": G >= R and G >= B allows olive, yellow-green
  and grey-green, all of which the current frame has). The score is then
  `green_excess = G - max(R, B)` over that mask, which an olive frame FAILS.
  If the mask is under MIN_GRASS_PX the axis returns None, never a number.

* RANGE IS GATED ON p95 - p05 of Rec.709 luma over the whole frame, and the
  row ALSO prints the wider p99 - p01 for reference. Percentiles, not min/max,
  so one blown lamp or one black pixel cannot manufacture the number. The two
  readings are far apart (riverbend: 36.9 vs 77.5) so the units matter: p95-p05
  is the one the order's "37-70" baseline reproduces in, so it is the one the
  target is written against and the one `c_range` tests. Read the p99-p01
  column as context, never against RANGE_MIN.

* ZONES is k-means (k=3, Lab, fixed init, deterministic) over a subsample, then
  the MINIMUM pairwise dE00 between the three cluster centres, plus each
  cluster's share. Three zones "genuinely differ" when the closest pair is far
  apart AND no cluster is a rounding error, so the gate is
  `min_pairwise_dE00 >= 12 and min_share >= 8%`. k-means always returns three
  centres; the separation is what is being measured, not the count.

Usage:
    python scripts/_flamingo_huegap_measure.py <frame.png> [more.png ...]
        [--masks=DIR] [--json=OUT.json] [--label=NAME]
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _pixel_artgap_grade import load, luma  # noqa: E402  (pure helpers only)
from _sun_skyground_measure import sky_mask_invariant  # noqa: E402

# --- gates -----------------------------------------------------------------
# (a) cool zone for a sky: blue-through-cyan hue, OR near-neutral. Expressed as
#     two clauses because a rain-day sky is cool GREY (no hue to speak of) while
#     a clear one is blue - the CEO reference `1787225660697_1.jpg` is the first.
SKY_COOL_HUE = (150.0, 290.0)   # degrees, inclusive - cyan .. blue .. violet
SKY_NEUTRAL_CHROMA = 0.06       # below this the sky is grey; judge on R-B alone
SKY_WARM_RMB_MAX = 2.0          # median R-B a non-warm sky may still carry
MIN_GRASS_PX = 3000
GREEN_EXCESS_MIN = 4.0          # display levels of G over max(R,B)
RANGE_MIN = 150.0
ZONE_DE_MIN = 12.0
ZONE_SHARE_MIN = 8.0


# --- colour helpers --------------------------------------------------------
def _srgb_to_lab(rgb: np.ndarray) -> np.ndarray:
    """rgb uint8-scale float (N,3) -> CIE Lab (D65)."""
    c = rgb / 255.0
    lin = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    m = np.array([[0.4124, 0.3576, 0.1805],
                  [0.2126, 0.7152, 0.0722],
                  [0.0193, 0.1192, 0.9505]])
    xyz = lin @ m.T
    xyz /= np.array([0.95047, 1.00000, 1.08883])
    e, k = 216 / 24389, 24389 / 27
    f = np.where(xyz > e, np.cbrt(xyz), (k * xyz + 16) / 116)
    return np.stack([116 * f[:, 1] - 16,
                     500 * (f[:, 0] - f[:, 1]),
                     200 * (f[:, 1] - f[:, 2])], axis=1)


def _de00(a: np.ndarray, b: np.ndarray) -> float:
    """CIEDE2000 between two Lab triples."""
    L1, a1, b1 = a
    L2, a2, b2 = b
    C1, C2 = np.hypot(a1, b1), np.hypot(a2, b2)
    Cb = (C1 + C2) / 2
    G = 0.5 * (1 - np.sqrt(Cb ** 7 / (Cb ** 7 + 25.0 ** 7 + 1e-12)))
    a1p, a2p = (1 + G) * a1, (1 + G) * a2
    C1p, C2p = np.hypot(a1p, b1), np.hypot(a2p, b2)
    h1p = np.degrees(np.arctan2(b1, a1p)) % 360
    h2p = np.degrees(np.arctan2(b2, a2p)) % 360
    dLp, dCp = L2 - L1, C2p - C1p
    dhp = h2p - h1p
    if C1p * C2p == 0:
        dhp = 0.0
    elif dhp > 180:
        dhp -= 360
    elif dhp < -180:
        dhp += 360
    dHp = 2 * np.sqrt(C1p * C2p) * np.sin(np.radians(dhp / 2))
    Lbp, Cbp = (L1 + L2) / 2, (C1p + C2p) / 2
    if C1p * C2p == 0:
        hbp = h1p + h2p
    elif abs(h1p - h2p) <= 180:
        hbp = (h1p + h2p) / 2
    elif h1p + h2p < 360:
        hbp = (h1p + h2p + 360) / 2
    else:
        hbp = (h1p + h2p - 360) / 2
    T = (1 - 0.17 * np.cos(np.radians(hbp - 30))
         + 0.24 * np.cos(np.radians(2 * hbp))
         + 0.32 * np.cos(np.radians(3 * hbp + 6))
         - 0.20 * np.cos(np.radians(4 * hbp - 63)))
    dTh = 30 * np.exp(-(((hbp - 275) / 25) ** 2))
    Rc = 2 * np.sqrt(Cbp ** 7 / (Cbp ** 7 + 25.0 ** 7 + 1e-12))
    Sl = 1 + (0.015 * (Lbp - 50) ** 2) / np.sqrt(20 + (Lbp - 50) ** 2)
    Sc = 1 + 0.045 * Cbp
    Sh = 1 + 0.015 * Cbp * T
    Rt = -np.sin(np.radians(2 * dTh)) * Rc
    return float(np.sqrt((dLp / Sl) ** 2 + (dCp / Sc) ** 2 + (dHp / Sh) ** 2
                         + Rt * (dCp / Sc) * (dHp / Sh)))


def _hue_chroma(rgb: np.ndarray):
    """(N,3) 0-255 float -> (hue_deg, chroma 0..1, value 0..1)."""
    c = rgb / 255.0
    mx = c.max(axis=1)
    mn = c.min(axis=1)
    d = mx - mn
    h = np.zeros_like(mx)
    r, g, b = c[:, 0], c[:, 1], c[:, 2]
    nz = d > 1e-9
    im = np.argmax(c, axis=1)
    with np.errstate(invalid="ignore", divide="ignore"):
        h = np.where(nz & (im == 0), ((g - b) / np.where(d == 0, 1, d)) % 6, h)
        h = np.where(nz & (im == 1), (b - r) / np.where(d == 0, 1, d) + 2, h)
        h = np.where(nz & (im == 2), (r - g) / np.where(d == 0, 1, d) + 4, h)
    return (h * 60.0) % 360.0, d, mx


def _circ_mean(hue_deg: np.ndarray, w: np.ndarray) -> float:
    if w.sum() <= 1e-9:
        return float("nan")
    a = np.radians(hue_deg)
    return float(np.degrees(np.arctan2((w * np.sin(a)).sum(),
                                       (w * np.cos(a)).sum())) % 360.0)


def _kmeans_lab(lab: np.ndarray, k=3, iters=60):
    """Deterministic k-means: init = k evenly-spaced quantiles of L."""
    qs = np.quantile(lab[:, 0], np.linspace(0.15, 0.85, k))
    cen = np.stack([lab[np.argmin(np.abs(lab[:, 0] - q))] for q in qs])
    lab_ = lab.astype(np.float64)
    for _ in range(iters):
        d = ((lab_[:, None, :] - cen[None, :, :]) ** 2).sum(axis=2)
        lbl = d.argmin(axis=1)
        new = np.stack([lab_[lbl == i].mean(axis=0) if (lbl == i).any() else cen[i]
                        for i in range(k)])
        if np.allclose(new, cen, atol=1e-4):
            cen = new
            break
        cen = new
    d = ((lab_[:, None, :] - cen[None, :, :]) ** 2).sum(axis=2)
    lbl = d.argmin(axis=1)
    return cen, lbl


# --- masks -----------------------------------------------------------------
def grass_mask(rgb: np.ndarray, sky: np.ndarray) -> np.ndarray:
    """Where the world builder put grass: ground pixels in the lower 60% of the
    frame whose GREEN channel is the max of the three.

    Deliberately weaker than "is it green" - G >= R and G >= B admits olive,
    khaki and grey-green, i.e. exactly the failure the score has to be able to
    report. Small blobs are opened away so texture speckle cannot make a mask.
    """
    H, W, _ = rgb.shape
    R, G, B = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    band = np.zeros((H, W), dtype=bool)
    band[int(H * 0.40):, :] = True
    m = band & ~sky & (G >= R) & (G >= B) & (luma(rgb) > 12)
    m = ndimage.binary_opening(m, np.ones((3, 3)))
    return m


def measure(path: str, masks_dir: str | None = None, label: str | None = None) -> dict:
    rgb = load(path).astype(np.float64)
    H, W, _ = rgb.shape
    L = luma(rgb)
    sky = sky_mask_invariant(L)
    flat = rgb.reshape(-1, 3)

    # `px` IS THE NORMALISED SIZE, NOT THE FILE'S. `load()` (shared with the
    # guarded grader) LANCZOS-resizes every frame to a constant AREA, so a
    # 1280x720 capture reports 1333x750 here. That is deliberate - it is what
    # lets a px-denominated threshold survive a plate class changing resolution
    # - but it means this field must never be quoted as the capture's size.
    out: dict = {"label": label or Path(path).stem, "file": path, "px": f"{W}x{H}"}

    # ---- (a) sky hue ------------------------------------------------------
    out["sky_frac_pct"] = round(float(sky.mean() * 100), 2)
    if sky.sum() >= 500:
        s = rgb[sky]
        hue, chroma, _ = _hue_chroma(s)
        h_mean = _circ_mean(hue, chroma)
        c_mean = float(chroma.mean())
        rmb = float(np.median(s[:, 0] - s[:, 2]))
        cool_hue = SKY_COOL_HUE[0] <= h_mean <= SKY_COOL_HUE[1]
        neutral = c_mean < SKY_NEUTRAL_CHROMA
        out["sky_hue_deg"] = round(h_mean, 1)
        out["sky_chroma"] = round(c_mean, 4)
        out["sky_median_RmB"] = round(rmb, 1)
        out["sky_median_L"] = round(float(np.median(L[sky])), 1)
        out["a_sky_cool"] = bool((cool_hue or neutral) and rmb <= SKY_WARM_RMB_MAX)
    else:
        out["sky_hue_deg"] = out["sky_chroma"] = out["sky_median_RmB"] = None
        out["sky_median_L"] = None
        out["a_sky_cool"] = None       # UNMEASURABLE, not FAIL

    # ---- (b) grass green --------------------------------------------------
    gm = grass_mask(rgb, sky)
    out["grass_px"] = int(gm.sum())
    out["grass_frac_pct"] = round(float(gm.mean() * 100), 2)
    if gm.sum() >= MIN_GRASS_PX:
        g = rgb[gm]
        ge = float(np.median(g[:, 1] - np.maximum(g[:, 0], g[:, 2])))
        hue, chroma, _ = _hue_chroma(g)
        out["grass_green_excess"] = round(ge, 2)
        out["grass_hue_deg"] = round(_circ_mean(hue, chroma), 1)
        out["grass_chroma"] = round(float(chroma.mean()), 4)
        out["b_grass_green"] = bool(ge >= GREEN_EXCESS_MIN)
    else:
        out["grass_green_excess"] = out["grass_hue_deg"] = out["grass_chroma"] = None
        out["b_grass_green"] = None    # UNMEASURABLE

    # ---- (c) dynamic range ------------------------------------------------
    p01, p05, p95, p99 = np.percentile(L, [1, 5, 95, 99])
    out["L_p01"] = round(float(p01), 1)
    out["L_p99"] = round(float(p99), 1)
    out["range_p99_p01"] = round(float(p99 - p01), 1)
    # The tighter percentile pair is quoted too because the order's own "37-70"
    # baseline is in those units; both are reported so neither reading can be
    # cherry-picked after the fact.
    out["range_p95_p05"] = round(float(p95 - p05), 1)
    # GATED ON p95-p05, and that is not a free choice: the order's baseline
    # ("37-70") reproduces on these two plates as 36.9 / 70.4 in exactly these
    # units, so this is the reduction the target 150 was written against.
    # `range_p99_p01` stays in the row as the wider reading.
    out["c_range"] = bool(p95 - p05 >= RANGE_MIN)

    # ---- (d) three colour zones ------------------------------------------
    step = max(1, flat.shape[0] // 60000)
    lab = _srgb_to_lab(flat[::step])
    cen, lbl = _kmeans_lab(lab, 3)
    shares = [round(float((lbl == i).mean() * 100), 1) for i in range(3)]
    pairs = [(0, 1), (0, 2), (1, 2)]
    des = [round(_de00(cen[i], cen[j]), 1) for i, j in pairs]
    out["zone_shares_pct"] = shares
    out["zone_dE00_pairs"] = des
    out["zone_min_dE00"] = min(des)
    out["zone_min_share_pct"] = min(shares)
    out["zone_centres_Lab"] = [[round(float(v), 1) for v in c] for c in cen]
    out["d_three_zones"] = bool(min(des) >= ZONE_DE_MIN and min(shares) >= ZONE_SHARE_MIN)

    # ---- whole-frame context (not gated, but it is what "one hue" means) ---
    hue_f, chroma_f, _ = _hue_chroma(flat[::step])
    keep = chroma_f > 0.05
    if keep.sum() > 100:
        hm = _circ_mean(hue_f[keep], chroma_f[keep])
        a = np.radians(hue_f[keep])
        w = chroma_f[keep]
        rbar = np.hypot((w * np.sin(a)).sum(), (w * np.cos(a)).sum()) / w.sum()
        out["frame_hue_deg"] = round(hm, 1)
        # circular STD (deg): 0 = every chromatic pixel is the same hue.
        out["frame_hue_std_deg"] = round(float(np.degrees(np.sqrt(-2 * np.log(max(rbar, 1e-9))))), 1)
    else:
        out["frame_hue_deg"] = out["frame_hue_std_deg"] = None

    if masks_dir:
        from PIL import Image
        d = Path(masks_dir)
        d.mkdir(parents=True, exist_ok=True)
        v = rgb.copy()
        v[sky] = v[sky] * 0.35 + np.array([0, 120, 255]) * 0.65
        v[gm] = v[gm] * 0.35 + np.array([0, 255, 60]) * 0.65
        Image.fromarray(v.clip(0, 255).astype(np.uint8)).save(
            d / (Path(path).stem + "_maskAB.png"))
    return out


def main() -> int:
    files, masks_dir, jout, label = [], None, None, None
    for a in sys.argv[1:]:
        if a.startswith("--masks="):
            masks_dir = a.split("=", 1)[1]
        elif a.startswith("--json="):
            jout = a.split("=", 1)[1]
        elif a.startswith("--label="):
            label = a.split("=", 1)[1]
        else:
            files.append(a)
    if not files:
        print(__doc__)
        return 2
    rows = []
    for f in files:
        r = measure(f, masks_dir, label if len(files) == 1 else None)
        rows.append(r)
        print(json.dumps(r))
    if jout:
        Path(jout).parent.mkdir(parents=True, exist_ok=True)
        Path(jout).write_text(json.dumps(rows, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
