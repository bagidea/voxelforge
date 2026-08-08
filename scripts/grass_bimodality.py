#!/usr/bin/env python3
"""Does the grass in a plate break into TWO luminance humps, or sit as one tone?

The question is about the grass alone: pull the vegetation pixels out of a
plate, build their luminance histogram, and say — with numbers, not by eye —
whether it is bimodal (a lit hump and a shaded hump separated by a real notch)
or unimodal (one flat tone, no light/shade separation).

    PASS = two resolvable humps      FAIL = one hump
    (flip with --expect unimodal if the lane wants the opposite)

Four metrics, each independently readable, no scipy needed:

  * peak separation   distance in L% between the two tallest smoothed peaks.
                      Two humps 2 L% apart are one hump with a wobble.
  * dip depth         1 - valley / min(peak) — how far the notch between them
                      actually descends. 0.0 = no notch, 1.0 = they never touch.
  * valley mass       share of grass pixels inside the notch (+/-3 L%). A real
                      gap is empty; a shoulder is not.
  * EM fit            a 2-component gaussian mixture: how far apart the fitted
                      components are (Ashman D) and whether the fitted DENSITY
                      itself has one hump or two.

Hartigan's dip test itself is deliberately NOT used: it needs scipy/diptest and
neither is installed on this box, and a p-value nobody here can recompute is
weaker evidence than four numbers anyone can re-derive from the saved histogram.

Only peak separation and dip depth gate the verdict; the EM numbers are printed
as corroboration, so a plate that passes on a technicality shows up as such.

EVERY NUMBER PRINTED HERE PASSES THE POSITIVE CONTROL, and two did not survive
that rule (2026-08-08, reviewer-found — both were shipping in a CEO-facing
table while contradicting the verdict beside them):

  * Ashman D was READ OFF A LOCAL OPTIMUM. The EM was seeded once, at the 25/75
    percentiles, and on the injected two-hump control (sep 25.0 L, dip 0.84) it
    settled on 49.1+/-14.9 / 59.1+/-3.5 => D 0.92, "not resolvable", and stayed
    there at iters=3000 tol=1e-12 — converged, just converged wrong. So the one
    plate designed to be bimodal scored BELOW three plates the tool itself
    FAILed (2.38, 2.46, 2.80). Fixed by multi-start: see gmm2().
  * The "D >= 2 = resolvable" bar was quoted unconditionally. It is the
    bimodality threshold only for two components of equal weight and equal
    width; at 87/13 or 62/38 (measured, this set) a mixture with D > 2 is still
    one hump. So D is no longer labelled a verdict — the fitted mixture's own
    hump count is printed next to it, which is the same question asked exactly.
  * Sarle's bimodality coefficient is REMOVED. It scored 0.485 on that same
    control — below its own 0.555 flag — because BC is built from skew and
    kurtosis and goes blind on an unbalanced mixture (the control's fit is
    19/81). A corroborator that cannot see the positive control corroborates
    nothing; leaving the number in the table only lends it authority.

THE MASK IS GUARDED, because it was wrong first time round.  A hue-band mask
alone (scripts/grade_g7.py's is_veg) also swallows the warm sunlit stone and the
campfire's bloom-lit block: measured on the before shotset, three plates came
back "BIMODAL / PASS" purely because the mask had picked up a second material —
the bright hump sat at hue 43.4 +/- 0.3 deg (one flat stone colour) while the
actual grass sat at hue 52-77.  On `grade-vista` those bright pixels are the lit
block beside the campfire.  So before any luminance verdict the mask's own HUE
distribution is checked for a split: grass under one sun varies in LUMINANCE,
not by 18 degrees of hue, so two hue modes mean two materials.

When it does split, the mask is cut at the hue valley and the GREENER mode is
kept — not the larger one.  That is the only tie-break that holds across the
set: on s3-clash the contaminant is 64% of the mask, so "keep the majority"
would have thrown the grass away and graded the stone.  Greener-wins is also the
only rule here that is not frame-tuned; the veg band runs 40..150deg precisely
because green sits at its top.  A fixed hue cut was rejected on purpose —
s1-vista's real grass sits at hue 45.7, right on top of the other plates'
contaminant at 43.3, so any global cut that cleaned one plate would gut another.

A plate whose mask is too small AFTER cleaning reports UNRELIABLE rather than a
verdict — same rule as grade_hero.py: a gate that cannot see its subject says so
instead of guessing.

Usage:
  python scripts/grass_bimodality.py <plate>.png [more.png ...]
         [--out DIR] [--expect bimodal|unimodal] [--no-plot] [--no-mask]
"""
import argparse
import colorsys
import math
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

# ---- grass mask -------------------------------------------------------------
# HUE BAND, never `g > r`. Lifted from scripts/grade_g7.py:is_veg, which
# documents why: sunlit grass in this look is warm yellow-olive, i.e. R > G, so
# a green-dominant mask finds ZERO vegetation on exactly the plates that are
# correctly lit. Measured here: on s1-vista-nohud2.png the `g >= r` rule sees
# 0.00% of a frame whose bottom half is entirely grass.
VEG_HUE_LO, VEG_HUE_HI = 40.0, 150.0
VEG_SAT_MIN, VEG_VAL_MIN = 15.0, 10.0

# ---- measurement constants --------------------------------------------------
STEP = 2             # pixel stride; 2 keeps a 1600x820 plate at ~330k samples
L_BINS = 100         # 1 L% per bin over 0..100
L_SIGMA = 2.5        # L%, gaussian kernel — wide enough to swallow voxel dither
PEAK_FLOOR = 0.05    # a "peak" under 5% of the tallest is histogram noise
VALLEY_HALF = 3.0    # L% either side of the notch counted as valley mass
SEP_MIN = 6.0        # L% — gate
DIP_MIN = 0.20       # gate
MIN_VEG_PIX = 2000   # below this the histogram is shot noise
MIN_VEG_SHARE = 0.5  # % of sampled pixels

# Mask purity: same shape machinery, run on HUE instead of luminance.
H_BINS, H_SIGMA = 180, 2.5   # 1 deg per bin over 0..180
HUE_SPLIT_MIN = 12.0         # deg between hue modes that means "two materials"
HUE_DIP_MIN = 0.20
W_DEGENERATE = 0.02          # EM component below this weight = collapsed fit


def hsv(r, g, b):
    h, s, v = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
    return h * 360, s * 100, v * 100


def is_veg(r, g, b):
    h, s, v = hsv(r, g, b)
    return VEG_HUE_LO <= h <= VEG_HUE_HI and s > VEG_SAT_MIN and v > VEG_VAL_MIN


def hsv_planes(arr):
    """Vectorised HSV (deg, %, %) for an (H,W,3) uint8 array.

    Verified against the scalar hsv()/is_veg() above on 70,880 pixels across
    three plates: 7 disagreements, all of them the single colour (132,88,0)
    whose hue lands exactly on the 40deg band edge (colorsys says
    39.99999999999999, this says 40.0). One boundary colour at 0.01% is not
    worth reproducing colorsys' exact float arithmetic for.
    """
    a = arr.astype(np.float64) / 255.0
    mx, mn = a.max(axis=2), a.min(axis=2)
    d = mx - mn
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    h = np.zeros_like(mx)
    nz = d > 0
    ir = nz & (mx == r)
    ig = nz & (mx == g) & ~ir
    ib = nz & (mx == b) & ~ir & ~ig
    with np.errstate(invalid="ignore", divide="ignore"):
        h[ir] = ((g - b)[ir] / d[ir]) % 6.0
        h[ig] = ((b - r)[ig] / d[ig]) + 2.0
        h[ib] = ((r - g)[ib] / d[ib]) + 4.0
    h *= 60.0
    s = np.where(mx > 0, d / np.where(mx > 0, mx, 1.0), 0.0) * 100.0
    return h, s, mx * 100.0


def veg_mask(arr):
    h, s, v = hsv_planes(arr)
    return (h >= VEG_HUE_LO) & (h <= VEG_HUE_HI) & (s > VEG_SAT_MIN) & (v > VEG_VAL_MIN)


def grass_select(arr):
    """Vegetation mask with a second-material hue split resolved.

    Returns (mask, note, hue_shape). `note` is None when the mask is already one
    material; otherwise it records where the cut fell and how much was dropped,
    so the number that follows is always traceable to a stated mask.
    """
    hue, _, _ = hsv_planes(arr)
    mask = veg_mask(arr)
    hues = hue[mask]
    if hues.size == 0:
        return mask, None, None
    hs = shape(hues, 0.0, 180.0, H_BINS, H_SIGMA)
    if not (hs["peaks"] and hs["sep"] >= HUE_SPLIT_MIN and hs["dip"] >= HUE_DIP_MIN):
        return mask, None, hs
    cut = hs["valley"]
    keep = mask & (hue >= cut)
    dropped = 100.0 * (int(mask.sum()) - int(keep.sum())) / int(mask.sum())
    note = (f"mask split at hue {cut:.0f}deg (modes {hs['peaks'][0]:.0f}/{hs['peaks'][1]:.0f}deg,"
            f" gap {hs['sep']:.0f}deg, dip {hs['dip']:.2f}) -> kept greener mode,"
            f" dropped {dropped:.1f}%")
    return keep, note, hs


def lum(arr):
    a = arr.astype(np.float64)
    return (0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]) / 255.0 * 100.0


# ---- distribution shape -----------------------------------------------------
def smooth(counts, sigma):
    rad = int(math.ceil(3 * sigma))
    k = np.exp(-0.5 * (np.arange(-rad, rad + 1) / sigma) ** 2)
    k /= k.sum()
    return np.convolve(counts, k, mode="same")


def shape(vals, lo, hi, bins, sigma):
    """Histogram + smoothed two-peak/valley description of a 1-D sample."""
    counts, edges = np.histogram(vals, bins=bins, range=(lo, hi))
    centers = (edges[:-1] + edges[1:]) / 2.0
    dens = smooth(counts.astype(np.float64), sigma / ((hi - lo) / bins))
    s = {"counts": counts, "dens": dens, "centers": centers,
         "peaks": None, "sep": 0.0, "dip": 0.0, "valley": float("nan"),
         "valley_mass": float("nan"), "idx": None}
    top = dens.max()
    if top <= 0:
        return s
    pk = [i for i in range(1, len(dens) - 1)
          if dens[i] >= dens[i - 1] and dens[i] > dens[i + 1]
          and dens[i] >= PEAK_FLOOR * top]
    if len(pk) < 2:
        return s
    pk.sort(key=lambda i: dens[i], reverse=True)
    a, b = sorted(pk[:2])
    v = a + int(np.argmin(dens[a:b + 1]))
    s["idx"] = (a, b, v)
    s["peaks"] = (float(centers[a]), float(centers[b]))
    s["valley"] = float(centers[v])
    s["sep"] = float(centers[b] - centers[a])
    s["dip"] = float(1.0 - dens[v] / min(dens[a], dens[b]))
    s["valley_mass"] = float(np.mean(np.abs(vals - centers[v]) <= VALLEY_HALF))
    return s


def em2(x, m0, iters=300, tol=1e-7):
    """One EM run from a fixed start. (m1,s1,m2,s2,w1,loglik), mean-ordered."""
    m = np.array(m0, dtype=np.float64)
    s = np.array([x.std(), x.std()], dtype=np.float64) + 1e-6
    w = np.array([0.5, 0.5])
    prev = ll = None
    for _ in range(iters):
        p = w * np.exp(-0.5 * ((x[:, None] - m) / s) ** 2) / (s * math.sqrt(2 * math.pi))
        tot = p.sum(axis=1, keepdims=True)
        tot[tot == 0] = 1e-300
        r = p / tot
        nk = r.sum(axis=0) + 1e-300
        w = nk / len(x)
        m = (r * x[:, None]).sum(axis=0) / nk
        s = np.sqrt((r * (x[:, None] - m) ** 2).sum(axis=0) / nk) + 1e-6
        ll = float(np.log(tot).sum())
        if prev is not None and abs(ll - prev) < tol:
            break
        prev = ll
    o = np.argsort(m)
    return m[o[0]], s[o[0]], m[o[1]], s[o[1]], w[o[0]], ll


def gmm2(x, valley=None):
    """1-D two-component EM, MULTI-START; the best-likelihood fit wins.

    Single-start from the 25/75 percentiles lands in a local optimum on exactly
    the shape this tool exists to detect. Measured on the positive control (a
    plate given two humps 25 L apart, dip 0.84): 25/75 converges to
    49.1+/-14.9 / 59.1+/-3.5, D 0.92, loglik -275593 — and does not move at
    iters=3000, tol=1e-12. Seeding the means either side of the histogram's own
    valley reaches 32.8+/-2.6 / 59.0+/-6.8, D 5.13, loglik -268269: a strictly
    BETTER fit by the model's own criterion, so the old answer was a bug and not
    a judgement call. Every seed is run and the highest loglik kept — no
    randomness, so the number is reproducible run to run.

    The valley seed is the one that matters, but it only exists when the
    histogram found two peaks; the percentile seeds cover the rest and cost
    nothing at this sample size.
    """
    seeds = [tuple(np.percentile(x, [25, 75])), tuple(np.percentile(x, [5, 95])),
             tuple(np.percentile(x, [40, 60])), (float(x.min()), float(x.max()))]
    if valley is not None and valley == valley:
        lo, hi = x[x < valley], x[x >= valley]
        if lo.size and hi.size:
            seeds.append((float(lo.mean()), float(hi.mean())))
    return max((em2(x, m0) for m0 in seeds), key=lambda f: f[5]), len(seeds)


def mix_shape(m1, s1, m2, s2, w1, grid=4001):
    """Hump count and dip depth of the FITTED mixture density. -> (humps, dip)

    Ashman D measures how far apart the components are; it answers "is the
    mixture bimodal" only when the two are of equal weight and equal width. This
    set is nowhere near that (measured weights run 87/13, 75/25, 62/38), so the
    density is evaluated and its own humps counted instead of trusting the
    D >= 2 rule of thumb. On the before shotset the two disagree: gate3-combat
    has D 2.45 with ONE hump, which is the whole point.

    The dip is computed exactly as the gate computes it on the histogram, so the
    two are directly comparable — and where they part company the FIT is the one
    that is wrong. s1-vista: fit dip 0.36 against a measured dip of 0.05, i.e.
    two gaussians carve a valley into a shoulder the pixels do not have. That
    comparison is the reason a fitted number never gates anything here.
    """
    g = np.linspace(0.0, 100.0, grid)
    d = (w1 * np.exp(-0.5 * ((g - m1) / s1) ** 2) / s1
         + (1 - w1) * np.exp(-0.5 * ((g - m2) / s2) ** 2) / s2)
    pk = [i for i in range(1, grid - 1) if d[i] > d[i - 1] and d[i] >= d[i + 1]]
    if len(pk) < 2:
        return len(pk) or 1, 0.0
    pk.sort(key=lambda i: d[i], reverse=True)
    a, b = sorted(pk[:2])
    v = a + int(np.argmin(d[a:b + 1]))
    return len(pk), float(1.0 - d[v] / min(d[a], d[b]))


def measure(path):
    img = Image.open(path).convert("RGB")
    arr = np.asarray(img)[::STEP, ::STEP, :]
    # Mask purity first: a mixed mask makes every luminance number a lie.
    mask, note, _ = grass_select(arr)
    hue, _, _ = hsv_planes(arr)
    vals = lum(arr)[mask]
    hues = hue[mask]
    sampled = arr.shape[0] * arr.shape[1]

    m = {"path": path, "size": img.size, "n": int(vals.size),
         "share": 100.0 * vals.size / sampled, "flags": [], "L": None,
         "note": note, "hue_mean": float(hues.mean()) if hues.size else float("nan")}
    if vals.size < MIN_VEG_PIX or m["share"] < MIN_VEG_SHARE:
        m["flags"].append(f"grass-mask-too-small({vals.size}px,{m['share']:.2f}%)")
        return m

    m["L"] = ls = shape(vals, 0.0, 100.0, L_BINS, L_SIGMA)
    m["mean"], m["sd"] = float(vals.mean()), float(vals.std())
    (m1, s1, m2, s2, w1, ll), nseed = gmm2(vals, ls["valley"])
    m["gmm"], m["loglik"], m["nseed"] = (m1, s1, m2, s2, w1), ll, nseed
    # A fit that put ~everything in one component has no second mean to compare
    # against: report no number rather than the distance to a ghost.
    degenerate = min(w1, 1 - w1) < W_DEGENERATE
    m["ashman"] = (float("nan") if degenerate
                   else math.sqrt(2) * abs(m1 - m2) / math.sqrt(s1 ** 2 + s2 ** 2))
    m["humps"], m["fit_dip"] = (1, 0.0) if degenerate else mix_shape(m1, s1, m2, s2, w1)
    m["bimodal"] = ls["sep"] >= SEP_MIN and ls["dip"] >= DIP_MIN
    return m


# ---- output -----------------------------------------------------------------
def _font(sz):
    for name in ("arial.ttf", "consola.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, sz)
        except OSError:
            continue
    return ImageFont.load_default()


def plot(m, out_png, verdict):
    W, H = 960, 560
    L, R, T, B = 70, 30, 78, 60
    pw, ph = W - L - R, H - T - B
    s = m["L"]
    im = Image.new("RGB", (W, H), (18, 18, 22))
    d = ImageDraw.Draw(im)
    f, fs = _font(17), _font(13)

    d.text((L, 14), os.path.basename(m["path"]), fill=(235, 235, 235), font=f)
    col = (245, 205, 110) if verdict.startswith("UNRELIABLE") else (
        (120, 235, 140) if verdict == "PASS" else (245, 120, 110))
    d.text((L, 38), f"grass luminance histogram   n={m['n']} px ({m['share']:.2f}% of frame)",
           fill=(180, 180, 190), font=fs)
    d.text((L, 55), f"-> {verdict}", fill=col, font=fs)

    d.rectangle((L, T, L + pw, T + ph), outline=(70, 70, 80))
    for t in range(0, 101, 10):
        x = L + pw * t / 100.0
        d.line((x, T + ph, x, T + ph + 5), fill=(110, 110, 120))
        d.text((x - 8, T + ph + 9), f"{t}", fill=(150, 150, 160), font=fs)
    d.text((L + pw / 2 - 55, T + ph + 30), "luminance L (%)", fill=(180, 180, 190), font=fs)

    top = max(s["counts"].max(), s["dens"].max())
    bw = pw / L_BINS
    for i, c in enumerate(s["counts"]):
        if c <= 0:
            continue
        h = ph * c / top
        x0 = L + i * bw
        d.rectangle((x0, T + ph - h, x0 + max(1.0, bw - 0.6), T + ph), fill=(88, 92, 105))
    d.line([(L + (i + 0.5) * bw, T + ph - ph * v / top) for i, v in enumerate(s["dens"])],
           fill=(255, 214, 120), width=2)

    if s["idx"]:
        a, b, v = s["idx"]
        for i, c in ((a, (120, 235, 220)), (b, (120, 235, 220)), (v, (235, 120, 220))):
            d.line((L + (i + 0.5) * bw, T, L + (i + 0.5) * bw, T + ph), fill=c, width=1)
        # Labels flip to the left of their line once they would run in under the
        # metrics block on the right.
        for i, (txt, c, dy) in ((a, (f"peak {s['peaks'][0]:.1f}", (120, 235, 220), 4)),
                                (b, (f"peak {s['peaks'][1]:.1f}", (120, 235, 220), 20)),
                                (v, (f"valley {s['valley']:.1f}", (235, 120, 220), 36))):
            x = L + (i + 0.5) * bw
            w = d.textlength(txt, font=fs)
            d.text((x - w - 5 if x + w + 6 > L + pw - 336 else x + 4, T + dy),
                   txt, fill=c, font=fs)

    ash = "  n/a (degenerate)" if m["ashman"] != m["ashman"] else f"{m['ashman']:5.2f}"
    lines = [f"peak separation {s['sep']:5.1f} L   (need >= {SEP_MIN})",
             f"dip depth       {s['dip']:5.2f}     (need >= {DIP_MIN})",
             f"valley mass     {s['valley_mass'] * 100:5.1f}%   (+/-{VALLEY_HALF:.0f} L of notch)"
             if s["valley_mass"] == s["valley_mass"] else "valley mass       n/a",
             f"EM fit humps    {m['humps']:5d}     (density's own shape)",
             f"Ashman D      {ash}     (component gap, not a bar)",
             f"grass mean L    {m['mean']:5.1f}   sd {m['sd']:.1f}",
             f"mask hue mean   {m['hue_mean']:5.1f} deg"]
    for i, t in enumerate(lines):
        d.text((L + pw - 330, T + 8 + 19 * i), t, fill=(205, 205, 215), font=fs)
    notes = ([m["note"]] if m["note"] else []) + m["flags"]
    for i, n in enumerate(notes):
        d.text((L, T + ph + 46 + 15 * i), "! " + n, fill=(245, 205, 110), font=fs)
    im.save(out_png)


def mask_png(m, out_png):
    img = Image.open(m["path"]).convert("RGB")
    arr = np.asarray(img)
    mk, _, _ = grass_select(arr)
    grey = (lum(arr) * 2.2).clip(0, 255)
    out = np.dstack([grey, grey, grey])
    out[mk] = np.array([255, 0, 220])
    Image.fromarray(out.astype(np.uint8)).resize(
        (img.size[0] // 2, img.size[1] // 2), Image.NEAREST).save(out_png)


def report(m, expect):
    key = os.path.splitext(os.path.basename(m["path"]))[0]
    print(f"== {key}  ({m['size'][0]}x{m['size'][1]})")
    if m["note"]:
        print(f"   mask         : {m['note']}")
    print(f"   grass pixels : {m['n']} ({m['share']:.2f}% of frame)", end="")
    if m["L"] is None:
        print(f"\n   -> UNRELIABLE ({','.join(m['flags'])})\n")
        return "UNRELIABLE (" + ",".join(m["flags"]) + ")"
    s = m["L"]
    print(f"  mean L={m['mean']:.1f} sd={m['sd']:.1f}  mask hue mean={m['hue_mean']:.1f}deg")
    if s["peaks"]:
        print(f"   peaks        : {s['peaks'][0]:.1f} L  and  {s['peaks'][1]:.1f} L"
              f"   valley {s['valley']:.1f} L")
        print(f"   separation   : {s['sep']:5.1f} L     need >= {SEP_MIN}    "
              f"{'ok' if s['sep'] >= SEP_MIN else 'x'}")
        print(f"   dip depth    : {s['dip']:5.2f}       need >= {DIP_MIN}   "
              f"{'ok' if s['dip'] >= DIP_MIN else 'x'}")
        print(f"   valley mass  : {s['valley_mass'] * 100:5.1f}%     "
              f"(within +/-{VALLEY_HALF:.0f} L of the notch)")
    else:
        print("   peaks        : only ONE local maximum survives smoothing -- single hump")
        print(f"   separation   :   0.0 L     need >= {SEP_MIN}    x")
        print(f"   dip depth    :  0.00       need >= {DIP_MIN}   x")
    m1, s1, m2, s2, w1 = m["gmm"]
    if m["ashman"] != m["ashman"]:
        print(f"   EM fit       :   n/a       (degenerate: one component "
              f"{min(w1, 1 - w1) * 100:.1f}% weight)")
    else:
        print(f"   EM fit       : {m['humps']} hump{'s' if m['humps'] > 1 else ' '}"
              f"  fit dip {m['fit_dip']:.2f} (vs measured {s['dip']:.2f})   Ashman D "
              f"{m['ashman']:.2f}")
        print(f"                  components {m1:.1f}+/-{s1:.1f} ({w1 * 100:.0f}%) | "
              f"{m2:.1f}+/-{s2:.1f} ({(1 - w1) * 100:.0f}%)  loglik {m['loglik']:.1f} "
              f"(best of {m['nseed']} seeds)")
        print("                  corroboration only: D is the component gap, and the >=2 bar "
              "assumes equal weight+width")
    if m["flags"]:
        print(f"   -> UNRELIABLE ({','.join(m['flags'])})\n")
        return "UNRELIABLE (" + ",".join(m["flags"]) + ")"
    shape_txt = "BIMODAL (two humps)" if m["bimodal"] else "UNIMODAL (one hump)"
    ok = m["bimodal"] if expect == "bimodal" else not m["bimodal"]
    v = "PASS" if ok else "FAIL"
    print(f"   -> {shape_txt}  expect={expect}  {v}\n")
    return v


def main(argv=None):
    ap = argparse.ArgumentParser(description="grass luminance bimodality meter")
    ap.add_argument("plates", nargs="+")
    ap.add_argument("--out", default="_poppy_grass", help="where histogram/mask PNGs land")
    ap.add_argument("--expect", choices=("bimodal", "unimodal"), default="bimodal",
                    help="which shape counts as PASS (default: bimodal)")
    ap.add_argument("--no-plot", action="store_true")
    ap.add_argument("--no-mask", action="store_true")
    a = ap.parse_args(argv)

    os.makedirs(a.out, exist_ok=True)
    print(f"# grass bimodality  expect={a.expect}  "
          f"gates: sep>={SEP_MIN}L dip>={DIP_MIN}  "
          f"mask: hue {VEG_HUE_LO:.0f}-{VEG_HUE_HI:.0f}deg, split-guard {HUE_SPLIT_MIN:.0f}deg")
    print("# corroboration = multi-start EM (humps + Ashman D). Sarle BC dropped "
          "2026-08-08: 0.485 on the positive control, i.e. blind to it.\n")
    rows = []
    for p in a.plates:
        m = measure(p)
        v = report(m, a.expect)
        key = os.path.splitext(os.path.basename(p))[0]
        if m["L"] is not None:
            if not a.no_plot:
                plot(m, os.path.join(a.out, f"{key}-hist.png"), v)
        if not a.no_mask:
            mask_png(m, os.path.join(a.out, f"{key}-mask.png"))
        rows.append((key, m, v))

    print("== SUMMARY ==")
    print(f"  {'plate':28s} {'n grass':>9s} {'sep':>6s} {'dip':>6s} {'humps':>6s} {'AshD':>6s}"
          f"  verdict        (sep+dip gate; humps/AshD corroborate)")
    for key, m, v in rows:
        if m["L"] is None:
            print(f"  {key:28s} {m['n']:9d} {'-':>6s} {'-':>6s} {'-':>6s} {'-':>6s}  {v}")
        else:
            ash = " n/a" if m["ashman"] != m["ashman"] else f"{m['ashman']:6.2f}"
            print(f"  {key:28s} {m['n']:9d} {m['L']['sep']:6.1f} {m['L']['dip']:6.2f} "
                  f"{m['humps']:6d} {ash:>6s}  {v}")
    # Name every plate where the EM fit and the gate point different ways, rather
    # than leaving a reader of the table to notice "2 humps ... FAIL" alone. The
    # gate reads the pixels; the fit reads two gaussians it imposed on them.
    split = [(k, m) for k, m, v in rows
             if m["L"] is not None and (m["humps"] > 1) != (v == "PASS")]
    if split:
        print("\n  fit-vs-gate disagreement (gate wins -- it reads the histogram, "
              "the fit reads two gaussians it imposed):")
        for k, m in split:
            print(f"    {k:26s} EM {m['humps']} humps / fit dip {m['fit_dip']:.2f}, but the "
                  f"pixels' own dip is {m['L']['dip']:.2f} (need {DIP_MIN})")
    npass = sum(1 for _, _, v in rows if v == "PASS")
    nfail = sum(1 for _, _, v in rows if v == "FAIL")
    nunk = len(rows) - npass - nfail
    print(f"\n  {npass} PASS / {nfail} FAIL / {nunk} UNRELIABLE   artifacts -> {a.out}/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
