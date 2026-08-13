#!/usr/bin/env python3
"""Beauty axes — measure the three defects the CEO named on 2026-08-14, against a
reference of the SAME SCENE CLASS.

  1. the sky is blown to flat white
  2. the whole frame is one orange-sand hue
  3. near and far read at the same depth (no aerial perspective)

WHY NOT `golden-beauty-shot-ref.png`: it is an INTERIOR kitchen. Measured here, its
sky mask is 0 px — there is no sky in it at all, and its near/far span is one room
deep. Grading a sky axis against it would compare a number to nothing, which is how
a gate ends up passing on an empty region (office scar: "prove the measured region
isn't degenerate"). The outdoor half of `docs/assets/moodboard.png` (the approved
look reference, right panel: village path at golden hour) is the reference that
actually contains the three things being graded, so that is what this measures
against. The kitchen ref still owns the interior axes — it is not being retired.

Geometry differs between reference and render, so every axis here is measured
DIRECTIONALLY per zone (sky band / far band / near band), never pixel-for-pixel.

Zones are derived from the frame, not hand-typed per plate:
  * sky   = bright low-saturation pixels connected to the top edge (the mask px
            count is printed so a 12-px "sky" can't quietly carry an axis)
  * far   = the band between the sky's lowest row and the frame's vertical middle
  * near  = the bottom 22% of the frame (excluding a HUD strip if --hud-bottom)

Usage:
    python scripts/_flamingo_beauty_axes.py REF CUR [--ref-crop L,T,R,B]
        [--cur-crop L,T,R,B] [--cur-hud-bottom N] [--ref-label X] [--cur-label Y]
        [--dump-zones OUT.png] [--json OUT.json]
"""
import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

SKY_L_MIN = 200.0      # sky is the bright end; blocks lit by the key top out lower
SKY_SAT_MAX = 0.42     # a warm haze is still低-sat next to a lit sandstone face
CLIP_LEVEL = 250       # any channel at/over this is on the tonemapper's shoulder
HUE_BIN = 15.0         # +-15 deg = "the same colour" to the eye at these sats
SAT_FLOOR = 0.12       # under this a pixel has no hue worth counting


def load(path, crop=None):
    im = Image.open(path).convert("RGB")
    if crop:
        im = im.crop(crop)
    a = np.asarray(im, dtype=np.float32)
    L = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
    mx, mn = a.max(2), a.min(2)
    sat = np.where(mx > 0, (mx - mn) / np.maximum(mx, 1e-6), 0.0)
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    d = np.maximum(mx - mn, 1e-6)
    h = np.where(mx == r, ((g - b) / d) % 6,
                 np.where(mx == g, (b - r) / d + 2, (r - g) / d + 4)) * 60.0
    return a, L, sat, h % 360.0


def sky_mask(L, sat):
    """Sky = bright, low-sat, and reachable from the top edge.

    The top-edge rule is what keeps a sunlit sandstone roof out of the mask: it is
    just as bright, but it is not connected to the frame's top border through other
    sky pixels.
    """
    cand = (L >= SKY_L_MIN) & (sat <= SKY_SAT_MAX)
    lab, n = ndimage.label(cand)
    if not n:
        return np.zeros_like(cand)
    touching = set(np.unique(lab[0, :])) - {0}
    if not touching:
        return np.zeros_like(cand)
    return np.isin(lab, list(touching))


def zone_stats(a, L, sat, m):
    if m.sum() < 32:
        return None
    px = a[m]
    return dict(px=int(m.sum()),
                L=round(float(L[m].mean()), 1),
                sat=round(float(sat[m].mean()), 3),
                rgb=[round(float(v), 1) for v in px.mean(0)],
                clip_pct=round(100.0 * float((px >= CLIP_LEVEL).any(1).mean()), 2))


def micro_contrast(L, m, block=None):
    """Mean per-block std of luminance — texture/detail energy inside a zone.

    Aerial perspective flattens the far field: distant blocks should carry LESS of
    this than near ones. One number per zone, blocks that are not fully inside the
    zone are skipped so the zone edge cannot fake contrast.

    The block is a FRACTION of frame height (8 px at 1024 tall), not a fixed pixel
    count: a 2560-wide plate and a 1024 reference would otherwise be measured at
    different world scales and the comparison would be meaningless.
    """
    h, w = L.shape
    block = block or max(4, int(round(8 * h / 1024.0)))
    vals = []
    for y in range(0, h - block + 1, block):
        for x in range(0, w - block + 1, block):
            sub = m[y:y + block, x:x + block]
            if sub.all():
                vals.append(float(L[y:y + block, x:x + block].std()))
    return round(float(np.mean(vals)), 2) if vals else None


def structure(L, sat, m):
    """Value + chroma STRUCTURE, which is what "one flat tone" actually means.

    Both the approved reference and the current render are ~100% warm-hue frames —
    measured, not assumed — so "the whole frame is one orange" is not a hue-count
    defect and cannot be fixed by adding hues. What separates a flat frame from a
    deep one at a single hue is how widely it spends VALUE and CHROMA. These are
    the numbers that carry that.
    """
    v = L[m]
    s = sat[m]
    p = [5, 25, 50, 75, 95]
    lp = np.percentile(v, p)
    sp = np.percentile(s, p)
    return dict(L_p=[round(float(x), 1) for x in lp],
                L_std=round(float(v.std()), 1),
                L_p95_minus_p5=round(float(lp[4] - lp[0]), 1),
                sat_p=[round(float(x), 3) for x in sp],
                sat_std=round(float(s.std()), 3),
                sat_p95_minus_p5=round(float(sp[4] - sp[0]), 3))


def horizon_blend(a, L, sky, live):
    """Aerial perspective at the one place every outdoor frame has it: where the
    land meets the sky.

    Distance haze makes far geometry approach the sky's own colour, so the step
    across the horizon is SMALL in a deep frame and LARGE in a flat one. Measured
    per column (first live non-sky pixel under the sky mask) so it does not depend
    on guessing which rows are "far".

    The land side is sampled over a band `k` rows deep, `k` scaled to frame height,
    so a 2560-wide plate and a 1024 reference are compared over the same fraction of
    the image rather than over one physical pixel each.
    """
    h, w = L.shape
    k = max(2, int(round(h * 0.004)))
    dL, dC = [], []
    for x in range(w):
        col = np.nonzero(sky[:, x])[0]
        if not len(col):
            continue
        y = int(col.max())
        below = np.nonzero(live[y + 1:, x] & ~sky[y + 1:, x])[0]
        if not len(below) or int(below[0]) > 2:
            continue                      # a gap means we walked past an occluder
        y2 = y + 1 + int(below[0])
        band = slice(y2, min(h, y2 + k))
        keep = live[band, x] & ~sky[band, x]
        if keep.sum() < k:
            continue
        dL.append(float(L[y, x] - L[band, x][keep].mean()))
        dC.append(float(np.abs(a[y, x] - a[band, x][keep].mean(0)).mean()))
    if len(dL) < 32:
        return None
    return dict(columns=len(dL), band_rows=k, dL=round(float(np.mean(dL)), 1),
                drgb=round(float(np.mean(dC)), 1))


def hue_spread(L, sat, hue, m):
    """How many distinct hues the frame actually spends its pixels on."""
    use = m & (sat >= SAT_FLOOR) & (L >= 20) & (L <= 248)
    if use.sum() < 256:
        return None
    h = hue[use]
    rad = np.deg2rad(h)
    mean_dir = np.rad2deg(np.arctan2(np.sin(rad).mean(), np.cos(rad).mean())) % 360.0
    d = np.abs(h - mean_dir) % 360.0
    d = np.minimum(d, 360.0 - d)
    R = float(np.hypot(np.sin(rad).mean(), np.cos(rad).mean()))
    circ_std = float(np.rad2deg(np.sqrt(-2.0 * np.log(max(R, 1e-9)))))
    fam = dict(warm=float(((h >= 15) & (h < 65)).mean()),
               green=float(((h >= 65) & (h < 165)).mean()),
               cool=float(((h >= 165) & (h < 275)).mean()),
               magenta=float(((h >= 275) | (h < 15)).mean()))
    return dict(px=int(use.sum()),
                dominant_hue=round(mean_dir, 1),
                within_15deg_pct=round(100.0 * float((d <= HUE_BIN).mean()), 1),
                circ_std_deg=round(circ_std, 1),
                neutral_pct=round(100.0 * float(((sat < SAT_FLOOR) & m).sum()
                                                / max(1, m.sum())), 1),
                families_pct={k: round(100.0 * v, 1) for k, v in fam.items()})


def grade(path, crop=None, hud_bottom=0, label=None, dump=None):
    a, L, sat, hue = load(path, crop)
    h, w = L.shape
    live = np.ones_like(L, dtype=bool)
    if hud_bottom:
        live[h - hud_bottom:, :] = False

    sky = sky_mask(L, sat) & live
    ys = np.nonzero(sky.any(1))[0]
    sky_bottom = int(ys.max()) if len(ys) else 0

    far = np.zeros_like(live)
    far[sky_bottom:max(sky_bottom + 1, h // 2), :] = True
    far &= live & ~sky
    near = np.zeros_like(live)
    near_top = int(h * 0.78)
    near[near_top:, :] = True
    near &= live

    out = dict(frame=str(path), label=label or Path(path).name, size=[w, h],
               sky=zone_stats(a, L, sat, sky), far=zone_stats(a, L, sat, far),
               near=zone_stats(a, L, sat, near),
               sky_px_pct=round(100.0 * float(sky.mean()), 2),
               micro_far=micro_contrast(L, far), micro_near=micro_contrast(L, near),
               palette=hue_spread(L, sat, hue, live),
               structure=structure(L, sat, live),
               horizon=horizon_blend(a, L, sky, live))
    if out["sky"] and len(ys) > 8:
        top_rows = sky & (np.arange(h)[:, None] <= ys.min() + max(4, len(ys) // 5))
        bot_rows = sky & (np.arange(h)[:, None] >= ys.max() - max(4, len(ys) // 5))
        if top_rows.sum() > 32 and bot_rows.sum() > 32:
            out["sky_gradient"] = dict(
                rows=int(ys.max() - ys.min() + 1),
                top_rgb=[round(float(v), 1) for v in a[top_rows].mean(0)],
                bottom_rgb=[round(float(v), 1) for v in a[bot_rows].mean(0)],
                dL=round(float(L[bot_rows].mean() - L[top_rows].mean()), 1),
                dsat=round(float(sat[bot_rows].mean() - sat[top_rows].mean()), 3))
    if out["far"] and out["near"]:
        out["depth"] = dict(
            dL_far_minus_near=round(out["far"]["L"] - out["near"]["L"], 1),
            dsat_near_minus_far=round(out["near"]["sat"] - out["far"]["sat"], 3),
            dmicro_near_minus_far=(round(out["micro_near"] - out["micro_far"], 2)
                                   if out["micro_near"] and out["micro_far"] else None))
    if dump:
        vis = a.copy()
        vis[sky] = vis[sky] * 0.35 + np.array([0, 180, 255]) * 0.65
        vis[far] = vis[far] * 0.55 + np.array([255, 0, 200]) * 0.45
        vis[near] = vis[near] * 0.55 + np.array([255, 220, 0]) * 0.45
        Image.fromarray(vis.astype(np.uint8)).save(dump)
    return out


def show(r):
    print(f"\n=== {r['label']}  ({r['size'][0]}x{r['size'][1]}) ===")
    s = r["sky"]
    if s:
        print(f"  SKY   {s['px']:>9,} px ({r['sky_px_pct']:.2f}% of frame)  rgb {s['rgb']}  "
              f"L {s['L']:.1f}  sat {s['sat']:.3f}  clipped {s['clip_pct']:.2f}%")
        g = r.get("sky_gradient")
        if g:
            print(f"        gradient over {g['rows']} rows: top {g['top_rgb']} -> "
                  f"horizon {g['bottom_rgb']}   dL {g['dL']:+.1f}  dsat {g['dsat']:+.3f}")
    else:
        print("  SKY   none found (0 px) — this frame cannot carry a sky axis")
    for k in ("far", "near"):
        z = r[k]
        if z:
            print(f"  {k.upper():5s} {z['px']:>9,} px  rgb {z['rgb']}  L {z['L']:.1f}  "
                  f"sat {z['sat']:.3f}  micro-contrast {r['micro_' + k]}")
    d = r.get("depth")
    if d:
        print(f"  DEPTH far-near  dL {d['dL_far_minus_near']:+.1f}   "
              f"dsat(near-far) {d['dsat_near_minus_far']:+.3f}   "
              f"dmicro(near-far) {d['dmicro_near_minus_far']}")
    st = r["structure"]
    print(f"  VALUE L p5/25/50/75/95 {st['L_p']}  std {st['L_std']}  "
          f"span(p95-p5) {st['L_p95_minus_p5']}")
    print(f"  CHROMA sat p5/25/50/75/95 {st['sat_p']}  std {st['sat_std']}  "
          f"span {st['sat_p95_minus_p5']}")
    hz = r.get("horizon")
    if hz:
        print(f"  HORIZON step over {hz['columns']} columns: dL {hz['dL']:+.1f}  "
              f"mean per-channel step {hz['drgb']:.1f}")
    p = r["palette"]
    if p:
        f = p["families_pct"]
        print(f"  HUE   dominant {p['dominant_hue']:.1f} deg   "
              f"{p['within_15deg_pct']:.1f}% of coloured px within +-15 deg   "
              f"circular std {p['circ_std_deg']:.1f} deg")
        print(f"        families: warm {f['warm']:.1f}%  green {f['green']:.1f}%  "
              f"cool {f['cool']:.1f}%  magenta {f['magenta']:.1f}%  "
              f"neutral(low-sat) {p['neutral_pct']:.1f}%")


def parse_box(s):
    return tuple(int(v) for v in s.split(",")) if s else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ref")
    ap.add_argument("cur")
    ap.add_argument("--ref-crop")
    ap.add_argument("--cur-crop")
    ap.add_argument("--ref-hud-bottom", type=int, default=0)
    ap.add_argument("--cur-hud-bottom", type=int, default=0)
    ap.add_argument("--ref-label")
    ap.add_argument("--cur-label")
    ap.add_argument("--dump-zones")
    ap.add_argument("--json")
    args = ap.parse_args()

    dz = Path(args.dump_zones) if args.dump_zones else None
    R = grade(args.ref, parse_box(args.ref_crop), args.ref_hud_bottom, args.ref_label,
              str(dz.with_name(dz.stem + "-ref" + dz.suffix)) if dz else None)
    C = grade(args.cur, parse_box(args.cur_crop), args.cur_hud_bottom, args.cur_label,
              str(dz.with_name(dz.stem + "-cur" + dz.suffix)) if dz else None)
    show(R)
    show(C)

    print("\n" + "-" * 100)
    print(f"{'axis':<34}{'REFERENCE':>16}{'CURRENT':>16}   gap")
    print("-" * 100)

    def row(name, rv, cv, fmt="{:.1f}", worse=None):
        rs = fmt.format(rv) if rv is not None else "-"
        cs = fmt.format(cv) if cv is not None else "-"
        gap = ""
        if rv is not None and cv is not None:
            gap = f"{cv - rv:+.2f}" + (f"   <-- {worse}" if worse else "")
        print(f"{name:<34}{rs:>16}{cs:>16}   {gap}")

    if R["sky"] and C["sky"]:
        row("sky clip % (any ch >= 250)", R["sky"]["clip_pct"], C["sky"]["clip_pct"],
            "{:.2f}", "blown" if C["sky"]["clip_pct"] > R["sky"]["clip_pct"] + 2 else None)
        row("sky saturation", R["sky"]["sat"], C["sky"]["sat"], "{:.3f}",
            "washed out" if C["sky"]["sat"] < R["sky"]["sat"] - 0.02 else None)
        row("sky luminance", R["sky"]["L"], C["sky"]["L"])
        if R.get("sky_gradient") and C.get("sky_gradient"):
            row("sky top->horizon dL", R["sky_gradient"]["dL"], C["sky_gradient"]["dL"],
                "{:.1f}", "gradient crushed"
                if abs(C["sky_gradient"]["dL"]) < abs(R["sky_gradient"]["dL"]) - 3 else None)
    if R["palette"] and C["palette"]:
        row("hue within +-15 deg of dominant %", R["palette"]["within_15deg_pct"],
            C["palette"]["within_15deg_pct"], "{:.1f}",
            "one-hue frame" if C["palette"]["within_15deg_pct"]
            > R["palette"]["within_15deg_pct"] + 5 else None)
        row("hue circular std (deg)", R["palette"]["circ_std_deg"],
            C["palette"]["circ_std_deg"])
        for fam in ("warm", "green", "cool"):
            row(f"  family {fam} %", R["palette"]["families_pct"][fam],
                C["palette"]["families_pct"][fam])
    row("value span L(p95-p5)", R["structure"]["L_p95_minus_p5"],
        C["structure"]["L_p95_minus_p5"], "{:.1f}",
        "flatter" if C["structure"]["L_p95_minus_p5"]
        < R["structure"]["L_p95_minus_p5"] - 8 else None)
    row("value std L", R["structure"]["L_std"], C["structure"]["L_std"])
    row("chroma span sat(p95-p5)", R["structure"]["sat_p95_minus_p5"],
        C["structure"]["sat_p95_minus_p5"], "{:.3f}",
        "no chroma range" if C["structure"]["sat_p95_minus_p5"]
        < R["structure"]["sat_p95_minus_p5"] - 0.05 else None)
    if R.get("horizon") and C.get("horizon"):
        row("horizon step (mean per-ch)", R["horizon"]["drgb"], C["horizon"]["drgb"],
            "{:.1f}", "no distance haze"
            if C["horizon"]["drgb"] > R["horizon"]["drgb"] + 15 else None)
    if R.get("depth") and C.get("depth"):
        row("aerial dL (far - near)", R["depth"]["dL_far_minus_near"],
            C["depth"]["dL_far_minus_near"], "{:.1f}",
            "far not lifted" if C["depth"]["dL_far_minus_near"]
            < R["depth"]["dL_far_minus_near"] - 5 else None)
        row("aerial dsat (near - far)", R["depth"]["dsat_near_minus_far"],
            C["depth"]["dsat_near_minus_far"], "{:.3f}",
            "far not desaturated" if C["depth"]["dsat_near_minus_far"]
            < R["depth"]["dsat_near_minus_far"] - 0.02 else None)
        row("detail drop (near - far micro)", R["depth"]["dmicro_near_minus_far"],
            C["depth"]["dmicro_near_minus_far"], "{:.2f}")

    if args.json:
        Path(args.json).write_text(json.dumps(dict(ref=R, cur=C), indent=1, default=float))
        print(f"\nwrote {args.json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
