#!/usr/bin/env python3
"""Flamingo -- the JUDGE for `VOXELFORGE_MAT_MAPS` on-vs-off. Two frames in,
four real numbers out, and a verdict that is allowed to refuse.

WHAT IT MEASURES (the four the brief named)

  1. lum_spread   luminance histogram spread, L(p95) - L(p5) on the 1024^2
                  resample.  Same L and same resample as `grade_axes.py`, so the
                  p95 half of it is cross-checkable against a published number.
  2. micro_rms    local contrast = RMS of the high-pass:
                  std(L - GaussianBlur(L, r=3)) on the 1024^2 luminance.
                  This is `grade_axes.py`'s `micro-contrast` axis, formula for
                  formula -- the rubric names that file the single source of
                  truth for the number, so this file reproduces it rather than
                  inventing a second local-contrast metric that would disagree
                  with the gate everyone else grades against.
  3. spec_cov     specular highlight coverage %: pixels that are BOTH locally
                  peaked (top-hat L - blur(r=6) >= SPEC_TOPHAT) AND bright
                  (L >= p80 of the non-sky population), as a % of non-sky px.
                  Reported beside `spec_amp` and `clip_hi` so a "more specular"
                  win that is really the frame railing at 255 is visible in the
                  same row (2026-08-09 lesson: a clamped channel fakes a colour
                  gate; the same trick fakes a highlight gate).
  4. shade_resid  normal-driven shading variance ON ONE BLOCK FACE. Per 48px
                  tile that qualifies as a single lit face, fit
                      log(L+1) ~ a + b*r + c*g          (r,g = chromaticity)
                  and take the std of the RESIDUAL. Chromaticity carries the
                  albedo; a Lambert normal map scales all three channels
                  together, so relief lands in the residual and a coloured
                  albedo change does not.

  ⚠ READ THIS BEFORE QUOTING shade_resid AS "NORMAL-DRIVEN".
  A GREYSCALE albedo variation is achromatic too, so in ONE frame it is
  mathematically indistinguishable from shading -- control C5 below measures
  exactly how much it fools the metric instead of hiding behind a caveat.
  What makes the number attributable in the matmaps A/B is the PAIR: both
  plates are shot from one binary with the same albedo tiles, one env lever
  apart, so the DELTA cannot come from albedo. `chroma_std` is printed as the
  guard on that claim -- if it moved, the pair differed in more than the lever
  and this tool reports UNRELIABLE rather than a verdict.

WHY IT CAN REFUSE. `docs/look-acceptance-rubric.md` rule 7: no control, no cut
line -- and a threshold that a signed-off frame cannot clear means the metric is
broken, not the render. So:

  exit 0  controls passed / verdict PASS
  exit 1  measured, and the pair FAILED the criteria
  exit 2  REFUSED TO JUDGE (no null pair, plates disagree on size, a control
          failed, too few qualifying tiles). Never a silent pass.

The null pair is not optional politeness. Every number here moves a little
between two runs of the SAME binary under the SAME env; without that floor a
+0.4% is indistinguishable from the renderer breathing. `_poppy_matmaps/shoot_ab.sh`
shoots `before` and `after`; one more `shoot after2` line under the after env
gives the floor.

USAGE
  python scripts/_fl_matmaps_judge.py control
  python scripts/_fl_matmaps_judge.py judge BEFORE.png AFTER.png \
         [--null N1.png N2.png] [--out DIR] [--label-a off --label-b on]
"""
import argparse
import json
import os
import sys

os.environ.setdefault("LOKY_MAX_CPU_COUNT", "4")

import numpy as np  # noqa: E402
from PIL import Image, ImageFilter  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# ---------------------------------------------------------------------------
# Tunables. Every one of these is exercised by a control below; none of them was
# picked to make a particular frame pass.
# ---------------------------------------------------------------------------
TILE = 48            # px, native resolution -- ~1.5 block faces at this camera
SPEC_TOPHAT = 12.0   # L above local mean (r=6) before a pixel counts as a peak
SPEC_BRIGHT_P = 80   # ...and it must also sit above this percentile of non-sky L
MIN_TILES = 30       # fewer qualifying tiles than this = no population, refuse

# Tile qualification. Computed per plate from that plate ALONE and then ANDed --
# never from the |A-B| difference. A mask derived from the difference makes
# "the change is on target" true by construction (2026-08-17 lesson).
Q_SKY_MAX = 0.05     # tile is out if >5% of it is sky
Q_CLIP_MAX = 0.05    # ...or >5% of it is railed at 0/255
Q_L_MIN = 20.0       # ...or it is essentially unlit (no shading to measure)
Q_CHROMA_MAX = 0.055  # ...or its chromaticity spread says it spans 2 materials

# Published numbers this file must reproduce before it is allowed to judge.
# Source: `python scripts/grade_axes.py <ref>` -- see the control block.
C0_REF = os.path.join(ROOT, "_fl_v4_20260818", "golden-beauty-shot-ref-nohud2.png")
C0_EXPECT = {"micro_rms": 5.24, "p95": 165.83}
C0_TOL = 0.02


# ---------------------------------------------------------------------------
# loaders
# ---------------------------------------------------------------------------
def _rgb(path):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.float32)


def _luma709(a):
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def _sky(a):
    """grade_axes' sky rule, verbatim: bluer than red AND brighter than median.

    Reused rather than re-derived so 'non-sky' means the same population here as
    it does in the file that owns the P0 axes.
    """
    L = _luma709(a)
    return (a[..., 2] > a[..., 0]) & (L > np.median(L))


def global_axes(path):
    """lum_spread + micro_rms + p95 on the 1024^2 resample (grade_axes' frame)."""
    im = Image.open(path).convert("RGB").resize((1024, 1024), Image.LANCZOS)
    a = np.asarray(im).astype(np.float32)
    L = _luma709(a)
    # micro-contrast: grade_axes:196-198 -- PIL's own L (601 luma), blur on uint8
    g = np.asarray(im.convert("L")).astype(np.float32)
    blur3 = np.asarray(Image.fromarray(g.astype(np.uint8))
                       .filter(ImageFilter.GaussianBlur(3))).astype(np.float32)
    return {
        "lum_spread": float(np.percentile(L, 95) - np.percentile(L, 5)),
        "micro_rms": float((g - blur3).std()),
        "p95": float(np.percentile(L, 95)),
        "p5": float(np.percentile(L, 5)),
    }


def spec_axes(path):
    """Specular coverage on NATIVE pixels.

    Native, not the 1024^2 square: a LANCZOS resample of a 1280x720 plate
    rewrites exactly the 1-3px structure a highlight is made of, and the two
    matmaps plates are pixel-locked by a pinned camera so native is comparable.
    """
    a = _rgb(path)
    L = _luma709(a)
    sky = _sky(a)
    ter = ~sky
    n_ter = int(ter.sum())
    if n_ter < 1000:
        return {"spec_cov": float("nan"), "spec_amp": float("nan"),
                "clip_hi": float("nan"), "n_terrain": n_ter}
    blur = np.asarray(Image.fromarray(L.astype(np.uint8))
                      .filter(ImageFilter.GaussianBlur(6))).astype(np.float32)
    tophat = L - blur
    bright = L >= np.percentile(L[ter], SPEC_BRIGHT_P)
    spec = ter & bright & (tophat >= SPEC_TOPHAT)
    n_spec = int(spec.sum())
    return {
        "spec_cov": 100.0 * n_spec / n_ter,
        "spec_amp": float(tophat[spec].mean()) if n_spec else 0.0,
        # a coverage rise that is really the frame railing shows up here
        "clip_hi": 100.0 * float((L[ter] >= 250).sum()) / n_ter,
        "n_terrain": n_ter,
    }


# ---------------------------------------------------------------------------
# per-face (tile) shading variance
# ---------------------------------------------------------------------------
def tile_fields(path):
    """Per-tile qualification + shading stats, on native pixels.

    Returns (qual[H,W bool grid], shade[], chroma[], resid[]) as 2-D tile grids
    with NaN where the tile did not qualify.
    """
    a = _rgb(path)
    L = _luma709(a)
    sky = _sky(a)
    s = a.sum(2) + 1e-6
    r, g = a[..., 0] / s, a[..., 1] / s
    railed = (a <= 2).any(2) | (a >= 253).any(2)
    logL = np.log(np.maximum(L, 0.0) + 1.0)

    H, W = L.shape
    ny, nx = H // TILE, W // TILE
    qual = np.zeros((ny, nx), bool)
    shade = np.full((ny, nx), np.nan)
    chroma = np.full((ny, nx), np.nan)
    resid = np.full((ny, nx), np.nan)

    for iy in range(ny):
        for ix in range(nx):
            sl = (slice(iy * TILE, (iy + 1) * TILE), slice(ix * TILE, (ix + 1) * TILE))
            if sky[sl].mean() > Q_SKY_MAX:
                continue
            if railed[sl].mean() > Q_CLIP_MAX:
                continue
            if L[sl].mean() < Q_L_MIN:
                continue
            rt, gt = r[sl].ravel(), g[sl].ravel()
            cs = float(np.hypot(rt.std(), gt.std()))
            if cs > Q_CHROMA_MAX:
                continue
            lt = logL[sl].ravel()
            # least squares logL ~ a + b*r + c*g. The chromaticity plane carries
            # the albedo; what it cannot explain is achromatic modulation.
            X = np.stack([np.ones_like(rt), rt, gt], 1)
            try:
                beta, *_ = np.linalg.lstsq(X, lt, rcond=None)
            except np.linalg.LinAlgError:
                continue
            qual[iy, ix] = True
            shade[iy, ix] = float(lt.std())
            chroma[iy, ix] = cs
            resid[iy, ix] = float((lt - X @ beta).std())
    return qual, shade, chroma, resid


def face_axes(path):
    q, sh, ch, rs = tile_fields(path)
    n = int(q.sum())
    if n == 0:
        return {"n_tiles": 0, "shade_std": float("nan"), "chroma_std": float("nan"),
                "shade_resid": float("nan"), "shade_resid_p90": float("nan")}, (q, rs, ch)
    return {
        "n_tiles": n,
        "shade_std": float(np.nanmedian(sh)),
        "chroma_std": float(np.nanmedian(ch)),
        "shade_resid": float(np.nanmedian(rs)),
        "shade_resid_p90": float(np.nanpercentile(rs[q], 90)),
    }, (q, rs, ch)


def paired_resid(A, B):
    """Per-tile shade_resid delta over tiles qualifying in BOTH plates.

    This is the number `judge()` actually gates on, so the controls below run
    through it too. A control that exercises a different code path from the
    verdict is not a control of the verdict. It also makes the controls immune
    to a plate-to-plate difference in tile COUNT: the two plates keep their own
    qualification (never derived from the difference) and only their
    intersection is compared.
    """
    qa, ra, _ = A["_fields"]
    qb, rb, _ = B["_fields"]
    both = qa & qb
    n = int(both.sum())
    if n == 0:
        return {"n": 0, "median": float("nan"), "share_up": float("nan")}
    d = (rb - ra)[both]
    return {"n": n, "median": float(np.median(d)),
            "share_up": 100.0 * float((d > 0).sum()) / d.size}


def measure(path):
    m = {}
    m.update(global_axes(path))
    m.update(spec_axes(path))
    fa, fields = face_axes(path)
    m.update(fa)
    m["_fields"] = fields
    m["path"] = path
    im = Image.open(path)
    m["w"], m["h"] = im.size
    return m


PRINT_KEYS = [
    ("lum_spread", "lum spread L(p95-p5)", "%10.2f"),
    ("micro_rms", "local contrast (hi-pass RMS)", "%10.2f"),
    ("spec_cov", "specular coverage %", "%10.3f"),
    ("shade_resid", "shading resid (median tile)", "%10.4f"),
    ("shade_resid_p90", "  shading resid p90", "%10.4f"),
    ("shade_std", "  raw logL std", "%10.4f"),
    ("chroma_std", "  chroma std (albedo guard)", "%10.4f"),
    ("spec_amp", "  specular amplitude", "%10.2f"),
    ("clip_hi", "  clipped-bright %", "%10.2f"),
    ("p95", "  L p95", "%10.2f"),
    ("n_tiles", "  qualifying tiles", "%10d"),
]


def table(rows, order):
    out = ["%-30s %s" % ("metric", "  ".join("%10s" % n for n in order)),
           "-" * (30 + 13 * len(order))]
    for key, title, fmt in PRINT_KEYS:
        out.append("%-30s %s" % (title, "  ".join(fmt % rows[n][key] for n in order)))
    out.append("%-30s %s" % ("  w x h", "  ".join(
        "%10s" % ("%dx%d" % (rows[n]["w"], rows[n]["h"])) for n in order)))
    return "\n".join(out)


# ---------------------------------------------------------------------------
# synthetic plates for the controls
# ---------------------------------------------------------------------------
def _synth(kind, w=960, h=720, seed=7):
    """Lambert-lit block faces. `kind` picks what changes vs 'flat'.

    flat      coloured albedo, N = +Z everywhere        -> no relief
    relief    SAME albedo, N from a ripple normal map   -> relief only
    albedo    N = +Z, extra CHROMATIC albedo noise      -> albedo only
    grey      N = +Z, extra GREYSCALE albedo noise      -> the known confound
    """
    rng = np.random.default_rng(seed)
    y, x = np.mgrid[0:h, 0:w].astype(np.float32)
    # blocky coloured albedo: 32px faces, each a flat-ish material patch
    fy, fx = (y // 32).astype(int), (x // 32).astype(int)
    # Warm-biased palette (R > G > B) on purpose. `_sky()` calls a pixel sky when
    # it is bluer than red AND above the median -- a uniform-random palette makes
    # a third of the synthetic patches read as sky, which would let the control
    # pass or fail on an artefact of my own test plate rather than on the metric.
    # Every frame this tool is pointed at is a warm voxel scene.
    pal = np.stack([rng.uniform(0.38, 0.78, (fy.max() + 2, fx.max() + 2)),
                    rng.uniform(0.28, 0.60, (fy.max() + 2, fx.max() + 2)),
                    rng.uniform(0.16, 0.40, (fy.max() + 2, fx.max() + 2))],
                   -1).astype(np.float32)
    alb = pal[fy, fx]
    alb = alb * (1.0 + 0.03 * rng.standard_normal((h, w, 1)).astype(np.float32))

    nx = np.zeros((h, w), np.float32)
    ny = np.zeros((h, w), np.float32)
    if kind == "relief":
        nx = 0.55 * np.sin(x / 6.0)
        ny = 0.55 * np.sin(y / 6.0)
    if kind == "albedo":
        tint = rng.standard_normal((h, w, 3)).astype(np.float32) * 0.06
        alb = np.clip(alb + tint, 0.02, 1.0)
    if kind == "grey":
        gnz = rng.standard_normal((h, w, 1)).astype(np.float32) * 0.06
        alb = np.clip(alb + gnz, 0.02, 1.0)

    nz = np.sqrt(np.maximum(1e-6, 1.0 - nx ** 2 - ny ** 2))
    ldir = np.array([0.45, 0.35, 0.82], np.float32)
    ldir /= np.linalg.norm(ldir)
    ndl = np.clip(nx * ldir[0] + ny * ldir[1] + nz * ldir[2], 0.0, 1.0)
    lit = alb * (0.18 + 0.82 * ndl)[..., None]
    return np.clip(lit ** (1 / 2.2) * 255.0, 0, 255).astype(np.uint8)


def _write_synth(outdir):
    os.makedirs(outdir, exist_ok=True)
    paths = {}
    for kind in ("flat", "relief", "albedo", "grey"):
        p = os.path.join(outdir, "synth_%s.png" % kind)
        Image.fromarray(_synth(kind)).save(p)
        paths[kind] = p
    return paths


# ---------------------------------------------------------------------------
# control mode
# ---------------------------------------------------------------------------
GOLDEN = [
    ("golden-beauty-ref", os.path.join(ROOT, "_fl_v4_20260818",
                                       "golden-beauty-shot-ref-nohud2.png")),
    ("wide-hero-final", os.path.join(ROOT, "docs", "assets", "wide-hero-final.png")),
    ("outdoor-noon_after", os.path.join(ROOT, "docs", "assets", "look",
                                        "outdoor-noon_after.png")),
    ("evening-raking_after", os.path.join(ROOT, "docs", "assets", "look",
                                          "evening-raking_after.png")),
    ("night-firelit_after", os.path.join(ROOT, "docs", "assets", "look",
                                         "night-firelit_after.png")),
    ("beauty-board", os.path.join(ROOT, "docs", "assets", "beauty-board.png")),
]


def control():
    out = os.path.join(ROOT, "_fl_matmaps_control")
    os.makedirs(out, exist_ok=True)
    fails = []
    print("=" * 78)
    print("MATMAPS JUDGE -- INSTRUMENT CONTROL")
    print("=" * 78)

    # -- C0: reproduce two numbers grade_axes.py already published --------------
    print("\nC0  reproduce grade_axes' own published values on the golden ref")
    if not os.path.isfile(C0_REF):
        print("  !! missing %s -- cannot calibrate" % C0_REF)
        fails.append("C0 missing ref")
    else:
        g = global_axes(C0_REF)
        for k, want in C0_EXPECT.items():
            got = g[k]
            ok = abs(got - want) <= C0_TOL
            print("  [%s] %-12s got %8.3f  grade_axes says %8.3f  (tol %.2f)"
                  % ("PASS" if ok else "FAIL", k, got, want, C0_TOL))
            if not ok:
                fails.append("C0 %s" % k)

    # -- C1: approved frames must score non-degenerately -----------------------
    print("\nC1  approved / signed-off frames -- the metric must return a real"
          "\n    number on work that was already accepted")
    rows, order = {}, []
    for label, path in GOLDEN:
        if not os.path.isfile(path):
            print("  !! MISSING %-22s %s" % (label, path))
            fails.append("C1 missing %s" % label)
            continue
        rows[label] = measure(path)
        order.append(label)
    print()
    print(table(rows, order))
    print("\n  assertions (a signed-off frame that scores 'broken' means the"
          "\n  METRIC is broken -- rubric rule 7):")
    for label in order:
        r = rows[label]
        checks = [
            ("finite", all(np.isfinite(r[k]) for k in
                           ("lum_spread", "micro_rms", "spec_cov", "shade_resid"))),
            ("tiles>=%d" % MIN_TILES, r["n_tiles"] >= MIN_TILES),
            ("spec 0<cov<25", 0.0 < r["spec_cov"] < 25.0),
            ("micro>0.5", r["micro_rms"] > 0.5),
        ]
        bad = [n for n, ok in checks if not ok]
        print("  [%s] %-22s %s" % ("PASS" if not bad else "FAIL", label,
                                   "ok" if not bad else "broken: " + ", ".join(bad)))
        if bad:
            fails.append("C1 %s (%s)" % (label, ",".join(bad)))

    # -- synthetic pairs -------------------------------------------------------
    sp = _write_synth(out)
    sy = {k: measure(p) for k, p in sp.items()}
    print("\n  synthetic plates written to %s" % out)
    print(table(sy, ["flat", "relief", "albedo", "grey"]))

    # measured the way the verdict measures: paired, on the tile intersection
    p_relief = paired_resid(sy["flat"], sy["relief"])
    p_albedo = paired_resid(sy["flat"], sy["albedo"])
    p_grey = paired_resid(sy["flat"], sy["grey"])
    d_relief, d_albedo, d_grey = (p_relief["median"], p_albedo["median"],
                                  p_grey["median"])
    dc_albedo = sy["albedo"]["chroma_std"] - sy["flat"]["chroma_std"]
    print("\n  paired (tile-intersection) shade_resid deltas vs flat -- the same"
          "\n  reduction judge() gates on:")
    for nm, p in (("relief", p_relief), ("albedo", p_albedo), ("grey", p_grey)):
        print("    %-8s n=%-4d median %+.4f   rose on %.1f%% of tiles"
              % (nm, p["n"], p["median"], p["share_up"]))

    # -- C2: null pair -- identical input must produce identical numbers -------
    print("\nC2  NULL (negative): the same plate measured twice must not move")
    a = measure(sp["flat"])
    b = measure(sp["flat"])
    dz = max(abs(a[k] - b[k]) for k in ("lum_spread", "micro_rms", "spec_cov",
                                        "shade_resid"))
    ok = dz == 0.0
    print("  [%s] max |delta| over the four metrics = %.6g (want exactly 0)"
          % ("PASS" if ok else "FAIL", dz))
    if not ok:
        fails.append("C2 not deterministic")

    # -- C3: planted relief must move it, hard --------------------------------
    # C3 asks ONE question -- does planted relief register? It deliberately does
    # NOT compare against the albedo control: that ratio depends on the noise
    # amplitude I chose for the synthetic, so folding it in here would make the
    # positive control grade my own synthetic instead of the metric. Specificity
    # is C4's job, with its own bar.
    print("\nC3  POSITIVE (planted): same albedo, ripple normal map added")
    ok = (d_relief > 0.005 and p_relief["share_up"] >= 90.0
          and sy["relief"]["shade_resid"] >= 3 * sy["flat"]["shade_resid"])
    print("  [%s] paired median %+.4f on %d tiles, rose on %.1f%% of them;"
          " per-plate %.4f -> %.4f (%.1fx)"
          % ("PASS" if ok else "FAIL", d_relief, p_relief["n"],
             p_relief["share_up"], sy["flat"]["shade_resid"],
             sy["relief"]["shade_resid"],
             sy["relief"]["shade_resid"] / max(sy["flat"]["shade_resid"], 1e-9)))
    if not ok:
        fails.append("C3 relief did not register")

    # -- C4: coloured albedo change must NOT be read as relief ----------------
    print("\nC4  SPECIFICITY (negative): chromatic albedo noise, N unchanged")
    ok = abs(d_albedo) < 0.5 * d_relief and dc_albedo > 0
    print("  [%s] shade_resid delta %+.4f  (vs relief %+.4f)   chroma_std delta %+.5f"
          % ("PASS" if ok else "FAIL", d_albedo, d_relief, dc_albedo))
    print("       chroma_std is the guard: albedo moved it, so a pair whose"
          "\n       chroma_std moves is not a clean one-lever pair.")
    if not ok:
        fails.append("C4 albedo read as relief")

    # -- C5: the confound, measured instead of caveated -----------------------
    print("\nC5  KNOWN CONFOUND (disclosure, not a gate): greyscale albedo noise")
    print("       shade_resid delta %+.4f  -- %.0f%% of the planted relief."
          % (d_grey, 100.0 * d_grey / d_relief if d_relief else 0.0))
    print("       A greyscale albedo change IS achromatic, so one frame cannot")
    print("       separate it from shading. The matmaps A/B is safe from this")
    print("       only because both plates use the SAME albedo tiles -- which is")
    print("       what chroma_std + one-lever provenance are there to prove.")

    print("\n" + "=" * 78)
    if fails:
        print("CONTROL FAILED -- fix the metric before judging anything:")
        for f in fails:
            print("   -", f)
        return 1
    print("CONTROL PASSED -- the instrument reproduces a published number, scores")
    print("approved frames as sane, is deterministic, moves on planted relief and")
    print("does not move on a coloured albedo change.")
    json.dump({k: {kk: vv for kk, vv in v.items() if not kk.startswith("_")}
               for k, v in list(rows.items()) + list(sy.items())},
              open(os.path.join(out, "control.json"), "w"), indent=2)
    print("wrote", os.path.join(out, "control.json"))
    return 0


# ---------------------------------------------------------------------------
# judge mode
# ---------------------------------------------------------------------------
GATED = ["lum_spread", "micro_rms", "spec_cov", "shade_resid"]


def judge(before, after, null=None, out=None, labels=("off", "on")):
    for p in (before, after):
        if not os.path.isfile(p):
            print("REFUSED: missing plate %s" % p)
            return 2
    A, B = measure(before), measure(after)
    if (A["w"], A["h"]) != (B["w"], B["h"]):
        print("REFUSED: %dx%d vs %dx%d -- a paired metric on two different"
              " geometries is not a comparison" % (A["w"], A["h"], B["w"], B["h"]))
        return 2

    la, lb = labels
    rows = {la: A, lb: B}
    print(table(rows, [la, lb]))
    print()
    print("%-30s %10s" % ("delta (%s - %s)" % (lb, la), ""))
    for key, title, fmt in PRINT_KEYS:
        if key == "n_tiles":
            continue
        print("%-30s %10.4f" % ("  " + title.strip(), B[key] - A[key]))

    # per-tile paired delta: same tile index in both plates (the camera is pinned,
    # so tile (iy,ix) is the same block face in both). A population share, not a
    # hand-picked face -- a site chosen on one plate scores N/N there by
    # construction (2026-08-12 lesson). The count that has to clear MIN_TILES is
    # the INTERSECTION, not either plate's own total: a lever that disqualifies
    # half the faces leaves a big per-plate count and nothing to pair.
    qa, ra, _ = A["_fields"]
    qb, rb, _ = B["_fields"]
    both = qa & qb
    if int(both.sum()) < MIN_TILES:
        print("\nREFUSED: %d tiles qualify in BOTH plates (need %d; per-plate"
              " %d / %d) -- no paired population to judge on"
              % (int(both.sum()), MIN_TILES, A["n_tiles"], B["n_tiles"]))
        return 2
    d_tile = (rb - ra)[both]
    share_up = 100.0 * float((d_tile > 0).sum()) / max(d_tile.size, 1)
    print("\npaired tiles qualifying in BOTH plates: %d" % int(both.sum()))
    print("  median per-tile shade_resid delta : %+.4f" % float(np.median(d_tile)))
    print("  tiles that rose                    : %.1f%%" % share_up)

    if not null:
        print("\n" + "=" * 78)
        print("REFUSED TO GATE -- no null pair.")
        print("Everything above is measured and real; none of it is a verdict.")
        print("Two runs of ONE binary under ONE env still differ, and until that")
        print("floor is measured a +0.4% and the renderer breathing are the same")
        print("number. Add one line to _poppy_matmaps/shoot_ab.sh:")
        print("    shoot after2 VOXELFORGE_MAT_MAPS=on")
        print("then re-run with  --null _poppy_matmaps/after.png"
              " _poppy_matmaps/after2.png")
        print("=" * 78)
        return 2

    N1, N2 = measure(null[0]), measure(null[1])
    floor = {k: abs(N2[k] - N1[k]) for k in GATED}
    qn1, rn1, _ = N1["_fields"]
    qn2, rn2, _ = N2["_fields"]
    nb = qn1 & qn2
    floor_tile = float(np.median(np.abs((rn2 - rn1)[nb]))) if int(nb.sum()) else 0.0
    print("\nnoise floor from the null pair (%s vs %s):"
          % (os.path.basename(null[0]), os.path.basename(null[1])))
    for k in GATED:
        print("  |d %-12s| = %.5f" % (k, floor[k]))
    print("  |d per-tile shade_resid| median = %.5f" % floor_tile)

    if floor["shade_resid"] == 0.0 and floor["micro_rms"] == 0.0:
        print("\nREFUSED: the null pair is byte-identical -- that is not a capture"
              "\nnoise floor, it is the same file twice.")
        return 2

    # ---- the criteria ------------------------------------------------------
    d = {k: B[k] - A[k] for k in GATED}
    verdicts = []

    def row(name, ok, detail, hard=True):
        verdicts.append((name, ok, hard))
        print("  [%s] %-34s %s" % ("PASS" if ok else ("FAIL" if hard else "ADV"),
                                   name, detail))

    print("\n=== CRITERIA (differential -- this framing has no absolute target) ===")
    m1 = d["shade_resid"] > 3 * floor["shade_resid"] and share_up >= 60.0
    row("M1 relief registers", m1,
        "d %+.4f vs 3x floor %.4f, %.1f%% of tiles rose (need >=60%%)"
        % (d["shade_resid"], 3 * floor["shade_resid"], share_up))

    # M2 needs a floor that cannot collapse to zero. A null pair whose chroma
    # happens to land on the same value would otherwise make this clause
    # unsatisfiable -- a gate no frame can pass is not a gate (2026-08-10).
    # So the bar is the LOOSER of the measured null move and 2% of the before
    # plate's own chroma scale. Calibrated, not guessed: control C4's chromatic
    # albedo change moves chroma_std by +35% of base, and a null pair moves it
    # 0.1-0.7% -- 2% sits an order of magnitude under the smallest albedo change
    # the instrument can see and well over the noise.
    d_chroma = B["chroma_std"] - A["chroma_std"]
    chroma_floor = max(3 * abs(N2["chroma_std"] - N1["chroma_std"]),
                       0.02 * A["chroma_std"])
    m2 = abs(d_chroma) <= chroma_floor
    row("M2 one-lever (albedo held)", m2,
        "chroma_std d %+.5f, bar %.5f (=max(3x null %.5f, 2%% of %.4f))"
        % (d_chroma, chroma_floor, abs(N2["chroma_std"] - N1["chroma_std"]),
           A["chroma_std"]))

    m3 = d["micro_rms"] >= -floor["micro_rms"]
    row("M3 no blur regression", m3,
        "micro_rms d %+.3f, floor %.3f (relief must not smooth the frame)"
        % (d["micro_rms"], floor["micro_rms"]))

    m4 = d["lum_spread"] >= -floor["lum_spread"]
    row("M4 no value-span loss", m4,
        "lum_spread d %+.3f, floor %.3f" % (d["lum_spread"], floor["lum_spread"]))

    m5 = (B["clip_hi"] - A["clip_hi"]) <= 1.0
    row("M5 highlights not railed", m5,
        "clip_hi d %+.3f pt (a coverage win made of 255s is not a win)"
        % (B["clip_hi"] - A["clip_hi"]))

    a6 = d["spec_cov"] > 3 * floor["spec_cov"]
    row("A6 specular coverage rose", a6,
        "d %+.4f pt vs 3x floor %.4f -- ADVISORY: authored roughness may"
        " legitimately reduce highlights" % (d["spec_cov"], 3 * floor["spec_cov"]),
        hard=False)

    hard_fail = [n for n, ok, hard in verdicts if hard and not ok]
    print()
    if hard_fail:
        print("VERDICT: FAIL -- %s" % ", ".join(hard_fail))
    else:
        print("VERDICT: PASS -- authored maps change the shading on the block"
              " faces, and nothing else regressed.")

    if out:
        os.makedirs(out, exist_ok=True)
        blob = {"before": {k: v for k, v in A.items() if not k.startswith("_")},
                "after": {k: v for k, v in B.items() if not k.startswith("_")},
                "delta": d, "floor": floor, "floor_tile": floor_tile,
                "tiles_both": int(both.sum()), "share_up": share_up,
                "verdict": "FAIL" if hard_fail else "PASS",
                "failed": hard_fail}
        p = os.path.join(out, "matmaps-verdict.json")
        json.dump(blob, open(p, "w"), indent=2)
        print("wrote", p)
    return 1 if hard_fail else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("mode", choices=["control", "judge"])
    ap.add_argument("plates", nargs="*")
    ap.add_argument("--null", nargs=2, metavar=("N1", "N2"))
    ap.add_argument("--out")
    ap.add_argument("--label-a", default="off")
    ap.add_argument("--label-b", default="on")
    a = ap.parse_args()
    if a.mode == "control":
        return control()
    if len(a.plates) != 2:
        print("judge needs exactly two plates: BEFORE.png AFTER.png")
        return 2
    return judge(a.plates[0], a.plates[1], a.null, a.out, (a.label_a, a.label_b))


if __name__ == "__main__":
    sys.exit(main())
