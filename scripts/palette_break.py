#!/usr/bin/env python3
"""palette_break.py — measure the two palette defects the AAA scorecard never samples.

The scorecard (docs/aaa-gap-scorecard-2026-08-06.md) grades *global* colour axes
(warmth R-B, blue leg, saturation, micro-contrast, p95). All of those are frame
averages, so a small screaming-green patch and a flat blue plate both average out.
This script grades the two things a frame average cannot see:

  1. OFF-PALETTE  — how much of the coloured frame sits outside the look-bible
     palette anchors (docs/look-bible.md #4), and specifically how far the
     foliage greens are from the spec'd foliage accent #8A8A3C.
  2. FLAT SKY     — whether the sky is a plate (one value, hard horizon) or an
     atmosphere (vertical gradient + haze softening into the scene).

Authority is docs/look-bible.md #4, NOT the golden beauty shot: the golden ref is
an interior with no sky, so a ref-delta criterion cannot grade an outdoor frame.

usage:  python scripts/palette_break.py [image ...]
        (no args -> the 3 gate3 frames + the golden ref + the wide hero)
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]

# docs/look-bible.md #4 — Palette, Golden Hour. (role, hex)
LOOK_BIBLE_PALETTE = [
    ("sun-key",      "#F4B860"),
    ("sun-key-hi",   "#FFD98A"),
    ("bounce-honey", "#C88A4A"),
    ("wood-mid",     "#6B4A2E"),
    ("wood-dark",    "#3A2716"),
    ("stone-beige",  "#B9A98C"),
    ("sky-day",      "#BCD3E0"),
    ("fog-cream",    "#E8D8B8"),
    ("foliage",      "#8A8A3C"),
    ("teal-accent",  "#4FC9D6"),
    ("shadow",       "#2A2030"),
]

FOLIAGE_SPEC_HEX = "#8A8A3C"

SAT_FLOOR = 0.20      # below this a pixel is neutral - it carries no hue opinion
OFF_PALETTE_DEG = 25.0  # hue distance to the nearest anchor that still counts as "in palette"
GREEN_LO, GREEN_HI = 75.0, 165.0   # hue window we call "foliage/green"
SKY_LO, SKY_HI = 185.0, 255.0      # hue window we call "sky blue"


def hex_to_hsl(hx: str) -> tuple[float, float, float]:
    r, g, b = (int(hx[i:i + 2], 16) / 255.0 for i in (1, 3, 5))
    return rgb_to_hsl(np.array([[[r, g, b]]]))[0][0, 0], \
        rgb_to_hsl(np.array([[[r, g, b]]]))[1][0, 0], \
        rgb_to_hsl(np.array([[[r, g, b]]]))[2][0, 0]


def rgb_to_hsl(rgb: np.ndarray):
    """rgb float 0..1 HxWx3 -> (hue deg 0..360, sat 0..1, lum 0..1)."""
    mx = rgb.max(axis=2)
    mn = rgb.min(axis=2)
    d = mx - mn
    lum = (mx + mn) / 2.0

    sat = np.zeros_like(lum)
    nz = d > 1e-6
    denom = 1.0 - np.abs(2.0 * lum - 1.0)
    denom = np.where(np.abs(denom) < 1e-6, 1e-6, denom)
    sat[nz] = (d / denom)[nz]
    sat = np.clip(sat, 0.0, 1.0)

    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    hue = np.zeros_like(lum)
    dd = np.where(nz, d, 1.0)
    m_r = nz & (mx == r)
    m_g = nz & (mx == g) & ~m_r
    m_b = nz & (mx == b) & ~m_r & ~m_g
    hue[m_r] = (((g - b) / dd) % 6.0)[m_r]
    hue[m_g] = (((b - r) / dd) + 2.0)[m_g]
    hue[m_b] = (((r - g) / dd) + 4.0)[m_b]
    return hue * 60.0, sat, lum


def hue_dist(a: np.ndarray, b: float) -> np.ndarray:
    d = np.abs(a - b) % 360.0
    return np.minimum(d, 360.0 - d)


def load(path: Path):
    im = Image.open(path).convert("RGB")
    rgb = np.asarray(im, dtype=np.float64) / 255.0
    return rgb, rgb_to_hsl(rgb)


def anchors():
    out = []
    for role, hx in LOOK_BIBLE_PALETTE:
        rgb = np.array([[[int(hx[i:i + 2], 16) / 255.0 for i in (1, 3, 5)]]])
        h, s, l = rgb_to_hsl(rgb)
        out.append((role, hx, float(h[0, 0]), float(s[0, 0]), float(l[0, 0])))
    return out


def off_palette(hue, sat):
    """% of *coloured* pixels whose hue is further than OFF_PALETTE_DEG from every anchor."""
    coloured = sat >= SAT_FLOOR
    n = int(coloured.sum())
    if n == 0:
        return 0.0, 0.0, None
    h = hue[coloured]
    best = np.full(h.shape, 1e9)
    who = np.zeros(h.shape, dtype=np.int16)
    for i, (_, _, ah, _, _) in enumerate(anchors()):
        d = hue_dist(h, ah)
        upd = d < best
        best = np.where(upd, d, best)
        who = np.where(upd, i, who)
    off = best > OFF_PALETTE_DEG
    frac_frame = 100.0 * n / hue.size
    return 100.0 * off.mean(), frac_frame, (best, who)


def warm_overshoot(hue, sat, rgb):
    """% of coloured pixels REDDER than the warmest look-bible anchor (wood-mid, 27.5deg).

    The palette's warm leg runs 27.5deg (wood-dark/mid) -> 40.5deg (sun-key-hi).
    Anything below 27.5 has overshot the warm end into red/fire-orange: it is not
    a colour the look bible contains. Frame-average warmth (R-B) rewards exactly
    this drift, which is why the scorecard never flags it.
    """
    coloured = sat >= SAT_FLOOR
    if coloured.sum() == 0:
        return None
    warmest = min(a[2] for a in anchors() if a[2] < 90.0)  # 27.5, wood-mid
    over = coloured & ((hue < warmest) | (hue > 330.0))
    n = int(over.sum())
    return {
        "warmest_anchor_deg": warmest,
        "pct_of_coloured": 100.0 * n / int(coloured.sum()),
        "pct_of_frame": 100.0 * n / hue.size,
        "mean_rgb": (rgb[over].mean(axis=0) * 255).round(1).tolist() if n else None,
        "mean_hue": float(hue[over].mean()) if n else float("nan"),
    }


def green_stats(hue, sat, lum):
    m = (hue >= GREEN_LO) & (hue <= GREEN_HI) & (sat >= SAT_FLOOR)
    if m.sum() == 0:
        return None
    sh, ss, sl = hex_to_hsl(FOLIAGE_SPEC_HEX)
    return {
        "frac_frame_pct": 100.0 * m.sum() / hue.size,
        "hue": float(hue[m].mean()),
        "sat": float(sat[m].mean()),
        "lum": float(lum[m].mean()),
        "d_hue": float(hue[m].mean() - sh),
        "d_sat_pp": float((sat[m].mean() - ss) * 100.0),
        "spec_hue": sh, "spec_sat": ss,
    }


def sky_stats(hue, sat, lum):
    """sky = blue-hue pixels in the upper 70% of the frame."""
    H, W = hue.shape
    band = np.zeros_like(hue, dtype=bool)
    band[: int(H * 0.70), :] = True
    m = band & (hue >= SKY_LO) & (hue <= SKY_HI) & (sat >= SAT_FLOOR) & (lum > 0.25)
    n = int(m.sum())
    if n < 0.002 * hue.size:
        return None

    rows = np.where(m.any(axis=1))[0]
    top, bot = rows[0], rows[-1]
    # mean L of the top 15% of sky rows vs the bottom 15% -> vertical gradient
    span = max(1, int((bot - top + 1) * 0.15))
    t_rows, b_rows = slice(top, top + span), slice(bot - span + 1, bot + 1)

    def band_mean(sl):
        mm = m[sl, :]
        return float(lum[sl, :][mm].mean() * 100.0) if mm.any() else float("nan")

    l_top, l_bot = band_mean(t_rows), band_mean(b_rows)

    # horizon hardness: mean |dL| across the last sky row -> first non-sky row below it
    grad = np.abs(np.diff(lum, axis=0)) * 100.0
    edge = m[:-1, :] & ~m[1:, :]
    horizon = float(grad[edge].mean()) if edge.any() else float("nan")

    return {
        "frac_frame_pct": 100.0 * n / hue.size,
        "L_mean": float(lum[m].mean() * 100.0),
        "L_std": float(lum[m].std() * 100.0),
        "grad_top_to_horizon_pp": l_top - l_bot,
        "horizon_step_pp": horizon,
        "sat": float(sat[m].mean()),
    }


def report(path: Path):
    rgb, (hue, sat, lum) = load(path)
    print(f"\n=== {path.name}  ({rgb.shape[1]}x{rgb.shape[0]}) ===")

    offp, coloured, _ = off_palette(hue, sat)
    print(f"  coloured pixels (sat>={SAT_FLOOR:.2f}) : {coloured:5.1f}% of frame")
    print(f"  OFF-PALETTE (>{OFF_PALETTE_DEG:.0f}deg from every look-bible anchor)"
          f" : {offp:5.1f}% of coloured")

    w = warm_overshoot(hue, sat, rgb)
    if w:
        print(f"  WARM OVERSHOOT (hue < {w['warmest_anchor_deg']:.1f}deg = redder than any anchor)"
              f" : {w['pct_of_coloured']:5.1f}% of coloured "
              f"({w['pct_of_frame']:4.1f}% of frame) · mean RGB {w['mean_rgb']} "
              f"· mean hue {w['mean_hue']:.1f}")

    g = green_stats(hue, sat, lum)
    if g is None:
        print("  green/foliage        : none")
    else:
        print(f"  green/foliage        : {g['frac_frame_pct']:5.2f}% of frame · "
              f"hue {g['hue']:6.1f} (spec {g['spec_hue']:.0f}, d {g['d_hue']:+.1f}) · "
              f"sat {g['sat']:.2f} (spec {g['spec_sat']:.2f}, d {g['d_sat_pp']:+.1f}pp) · "
              f"L {g['lum'] * 100:.0f}")

    s = sky_stats(hue, sat, lum)
    if s is None:
        print("  sky                  : none in frame (interior)")
    else:
        print(f"  sky                  : {s['frac_frame_pct']:5.2f}% of frame · "
              f"L {s['L_mean']:.1f} +-{s['L_std']:.2f} · "
              f"gradient zenith->horizon {s['grad_top_to_horizon_pp']:+.2f}pp · "
              f"horizon step {s['horizon_step_pp']:.1f}pp · sat {s['sat']:.2f}")
    return {"file": path.name, "off_palette_pct": offp, "green": g, "sky": s}


POST_SATURATION = 1.90   # client/src/look.rs :: grade::POST_SATURATION (shipped)


def clip_forecast(sat_knob: float = POST_SATURATION):
    """Forecast which colours `post_saturation` drives out of gamut.

    Bevy's ColorGradingGlobal::post_saturation is a mix about luminance:
        c' = L + (c - L) * sat            L = 0.2126R + 0.7152G + 0.0722B
    With sat > 1 the MINORITY channels move away from L and go negative, and
    negative is clamped to 0. That clamp is a HUE CHANGE, not a saturation
    change: the colour stops being "a more saturated version of itself".

    Run against the look-bible anchors plus the two colours the gates publish
    (`golden-beauty-shot.md`: darkest shade 45,22,7 / sunlit wood 232,211,181)
    and the shipped foliage albedo (`sim/src/block.rs`: grass 91,140,70).
    """
    probes = [(role, hx, None) for role, hx, _, _, _ in anchors()]
    probes += [
        ("gate darkest shade", None, (45, 22, 7)),
        ("gate sunlit wood", None, (232, 211, 181)),
        ("block.rs grass", None, (91, 140, 70)),
        ("block.rs leaves", None, (58, 116, 54)),
        ("block.rs moss", None, (75, 110, 55)),
    ]
    print(f"\n=== clip forecast @ post_saturation = {sat_knob} "
          f"(c' = L + (c-L)*sat, then clamp 0..255) ===")
    print(f"  {'probe':22s} {'in RGB':>16s} -> {'out RGB':>18s}  clipped")
    for role, hx, rgbv in probes:
        r, g, b = rgbv if rgbv else tuple(int(hx[i:i + 2], 16) for i in (1, 3, 5))
        lum = 0.2126 * r + 0.7152 * g + 0.0722 * b
        out = [lum + (c - lum) * sat_knob for c in (r, g, b)]
        clipped = [i for i, v in enumerate(out) if v < 0.0 or v > 255.0]
        cl = "".join("RGB"[i] for i in clipped) or "-"
        print(f"  {role:22s} {str((r, g, b)):>16s} -> "
              f"{str(tuple(round(max(0.0, min(255.0, v))) for v in out)):>18s}  {cl}")


DEFAULTS = [
    "docs/assets/gate3/gate3-after-boot-nohud2.png",
    "docs/assets/gate3/gate3-after-walk-nohud2.png",
    "docs/assets/gate3/gate3-after-combat-nohud2.png",
    "docs/assets/golden-beauty-shot-ref.png",
    "docs/assets/wide-hero-final.png",
]


def main(argv):
    paths = [Path(a) for a in argv[1:]] or [ROOT / p for p in DEFAULTS]
    print("look-bible #4 anchors (role, hex, hue, sat, L):")
    for role, hx, h, s, l in anchors():
        print(f"  {role:14s} {hx}  h={h:6.1f}  s={s:.2f}  L={l:.2f}")
    for p in paths:
        if not p.exists():
            print(f"\n!! missing: {p}")
            continue
        report(p)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
