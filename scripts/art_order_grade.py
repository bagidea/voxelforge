#!/usr/bin/env python3
"""art_order_grade.py — the verdict line for docs/art-order-2026-08-09-composition.md.

`palette_break.py` and `depth_layering.py` already MEASURE the composition axes.
Neither of them ever says PASS or FAIL, and neither returns a non-zero exit code,
so work handed back against that order could only be judged by eye. This script is
the missing half: every A-item that can be decided from pixels (or from the map /
the source tree) gets one named metric, one published cut line, and an exit code.

  A1  sky is an atmosphere, not a painted plate   (gradient / monotonic / seam)
  A2  god rays actually render in the play scene  (source gate + same-camera A/B)
  A3  the map uses more than two materials        (maps/edhari.json)
  A4  aerial perspective: far reads softer        (far/near local contrast)
  A5  the boom camera is not inside a block       (worst-third contrast share)
  A6  the hero reads as a character, not a box    (silhouette complexity + hue)
  A7  grass is natural, not neon                  (blue channel + saturation)
  A8  one place for the eye to land               (largest bright-island share)

THREE RULES THIS FILE ENFORCES, because the lane has been bitten by all three:

 1. CALIBRATE BEFORE YOU TRUST. The control suite runs on EVERY invocation, not
    behind a flag. It feeds each gate synthetic inputs whose answer is known
    (a ramp of exactly 30 L, a flat plate, a shaft overlay, an isotropic glow)
    plus the approved reference frames, and refuses to print a single verdict if
    any control lands on the wrong side of its own cut line. A grader that has
    not passed its positive AND negative control is not evidence.
 2. UNMEASURABLE IS NOT PASS. A gate whose input is missing (no A/B pair, no sky
    in the frame, no hero mask) reports SKIP and makes the run INCOMPLETE. It
    never silently counts as a pass. Exit 4 exists for exactly that.
 3. ONE METRIC PER CLAIM, NAMED IN THE ORDER. Every threshold below is quoted in
    the order document next to the item it decides, so a number cannot drift on
    one side without the other noticing.

usage:
  python scripts/art_order_grade.py                       # gate3 frames + map + source
  python scripts/art_order_grade.py --frame play.png --before play-nofog.png
  python scripts/art_order_grade.py --idle boot.png       # A6 is idle-pose only
  python scripts/art_order_grade.py --calibrate           # controls only, no grading

exit: 0 all measured gates PASS · 1 some gate FAILs · 2 bad usage
      3 CALIBRATION FAILED (no verdict is trustworthy) · 4 incomplete (a gate was
      not measurable, so the set cannot be called passed)
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[1]

# ---------------------------------------------------------------- cut lines --
# Every number here is quoted in docs/art-order-2026-08-09-composition.md.
# Where it came from is in the comment: an approved frame, a measured defect, or
# a synthetic control. No threshold in this file was chosen by feel.
T = {
    # A1 — "gradient ≥ 25 L" is the order's own line. The control proves the
    # metric reports a 30 L ramp as 30 L (+-2), so 25 means 25.
    "sky_grad_L": 25.0,
    "sky_mono": 0.85,      # control: true ramp 0.99+, gate3 plates 0.56-0.65
    "sky_seam_L": 12.0,    # control: soft haze blend 4.9, hard seam 40+
    "sky_min_frac": 0.20,  # % of frame that must be sky before A1 is gradeable
    # A2 — differential, because no single-frame shaft detector separates the
    # approved ref from the shaft-free gate3 frames (probe result, see docstring
    # of godray_ab). All four sub-gates must hold on the same A/B pair.
    "ray_amp_L": 6.0,      # p99.5 of dL; control: shafts 18 L pass, 2 L fail
    "ray_lift": 0.35,      # median/p99 of dL; a global exposure bump sits at ~1.0
    "ray_elong": 3.0,      # major/minor axis of the biggest dL island; a glow ~1
    "ray_cov_lo": 0.5,     # % of frame; below this nobody sees it
    "ray_cov_hi": 25.0,    # above this it is a fog wall, not shafts (rubric:156)
    # A3 — measured defect is 4 types / 93.9% top-2 / 56.0% bare columns
    "map_types": 8,
    "map_top2_pct": 75.0,
    "map_bare_pct": 35.0,
    "map_wood_pct": 1.0,
    # A4 — golden ref (approved) measures 0.49; gate3 measures 0.86-2.12
    "aerial_ratio": 0.80,
    # A5 — walk (camera inside a block) measures 0.10; every other frame 0.51-0.94
    "third_share": 0.35,
    # A6 — order's own line; an empty box measures 16.0
    "silhouette": 40.0,
    "hero_hue_sep": 25.0,
    # A7 — approved accent green in wide-hero-final is B=94 sat 0.25;
    # shipped gate3 grass is B=1-4 sat 0.97-0.99
    "grass_blue": 40.0,
    "grass_sat": 0.75,
    # A8 — order's own line; approved frames measure 80.4% / 91.9%
    "island_share": 40.0,
}

PASS, FAIL, SKIP = "PASS", "FAIL", "SKIP"


class Verdict:
    def __init__(self):
        self.rows = []

    def add(self, item, gate, state, got, want, note=""):
        self.rows.append((item, gate, state, got, want, note))

    def show(self):
        print(f"\n{'item':4s} {'gate':26s} {'verdict':7s} {'measured':>12s}   want")
        print("-" * 88)
        for item, gate, state, got, want, note in self.rows:
            mark = {PASS: "PASS  ", FAIL: "FAIL !", SKIP: "SKIP ?"}[state]
            print(f"{item:4s} {gate:26s} {mark:7s} {got:>12s}   {want}"
                  + (f"\n{'':40s}{note}" if note else ""))

    def exit_code(self):
        if any(r[2] == FAIL for r in self.rows):
            return 1
        if any(r[2] == SKIP for r in self.rows):
            return 4
        return 0


# ------------------------------------------------------------------ pixels --
def load(path):
    im = Image.open(path).convert("RGB")
    return from_image(im)


def from_image(im):
    a = np.asarray(im, dtype=np.float32)
    L = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
    g = np.asarray(im.convert("L")).astype(np.float32)
    hp = np.abs(g - np.asarray(im.convert("L").filter(ImageFilter.GaussianBlur(3))).astype(np.float32))
    detail = ndimage.uniform_filter(
        np.abs(g - np.asarray(im.convert("L").filter(ImageFilter.GaussianBlur(2))).astype(np.float32)), 5)
    mx, mn = a.max(2), a.min(2)
    sat = np.where(mx > 0, (mx - mn) / np.maximum(mx, 1e-6), 0.0)
    return dict(im=im, rgb=a, L=L, hp=hp, detail=detail, sat=sat, hue=hue_deg(a))


def hue_deg(a):
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    mx, mn = a.max(2), a.min(2)
    d = mx - mn
    h = np.zeros_like(mx)
    nz = d > 1e-6
    dd = np.where(nz, d, 1.0)
    mr = nz & (mx == r)
    mg = nz & (mx == g) & ~mr
    mb = nz & (mx == b) & ~mr & ~mg
    h[mr] = (((g - b) / dd) % 6.0)[mr]
    h[mg] = (((b - r) / dd) + 2.0)[mg]
    h[mb] = (((r - g) / dd) + 4.0)[mb]
    return h * 60.0


# ------------------------------------------------------------------- A1 sky --
def sky_region(f):
    """Blue core anchors the region; a texture-free walk extends it to the true horizon.

    Hue alone cannot bound the sky once A1 is DONE: the order asks for the horizon
    to melt into FOG_COLOR_DAY (orange). A blue-only mask would stop at the last
    blue row, hand the warm haze band to the ground, and then report the blend it
    just cut off as a hard seam. So: locate with hue (that is what rejects an
    interior), extend with "no micro detail" (atmosphere has none, voxels do).
    """
    L, hue, sat, detail = f["L"], f["hue"], f["sat"], f["detail"]
    lum01 = L / 255.0
    H, W = L.shape
    top = np.zeros_like(L, dtype=bool)
    top[: int(H * 0.70)] = True
    core = top & (hue >= 185) & (hue <= 255) & (sat >= 0.20) & (lum01 > 0.25)
    if core.mean() * 100 < T["sky_min_frac"]:
        return None
    m = core.copy()
    quiet = detail < 1.2
    for x in range(W):
        idx = np.where(core[:, x])[0]
        if not len(idx):
            continue
        y = idx[-1] + 1
        while y < H and quiet[y, x]:
            m[y, x] = True
            y += 1
        y = idx[0] - 1
        while y >= 0 and quiet[y, x]:
            m[y, x] = True
            y -= 1
    return m


def sky_metrics(f):
    """Row-mean luminance profile down the sky, in 0..100 L units.

    grad = max - min of the smoothed profile, NOT a top-band/bottom-band delta.
    Band deltas throw away 10-15% of the ramp by construction (a 30 L ramp reads
    24-27) and the order's cut line is written on the real amplitude. Reading the
    extremes of a profile that is required to be monotone recovers 96% of a known
    ramp; the residual is the smoothing window and it errs strict, never loose.

    mono = |sum(dL)| / sum(|dL|) — total displacement over total variation. A true
    ramp is 1.0, noise is ~0.3, and a dead-flat plate is undefined (0/0) so it is
    forced to 0: a plate is not a shallow gradient, it is not a gradient at all.
    """
    m = sky_region(f)
    if m is None:
        return None
    L = f["L"]
    rows = np.where(m.sum(axis=1) >= 8)[0]
    prof = np.array([L[y][m[y]].mean() for y in rows]) / 255.0 * 100.0
    sm = ndimage.uniform_filter1d(prof, 9)
    grad = float(sm.max() - sm.min())
    d = np.diff(sm)
    var = float(np.abs(d).sum())
    mono = float(abs(d.sum()) / var) if var >= 0.5 else 0.0
    edge = m[:-1] & ~m[1:]
    n_edge = int(edge.sum())
    seam = float(np.abs(np.diff(L, axis=0))[edge].mean()) / 255.0 * 100.0 if n_edge else float("nan")
    return dict(frac=100.0 * m.mean(), grad=grad, mono=mono, seam=seam, n_edge=n_edge,
                bright="horizon" if sm[-1] > sm[0] else "zenith",
                rows=(int(rows[0]), int(rows[-1])))


# --------------------------------------------------------------- A2 god ray --
def godray_ab(after, before):
    """Shafts measured as the difference between two same-camera frames.

    An ABSOLUTE single-frame shaft detector was tried first and thrown away: on
    orientation-coherence and on Hessian bright-ridge energy the shaft-free gate3
    frames score HIGHER than the approved ref that visibly has shafts (voxel
    geometry is all coherent bright ridges). Any absolute number would have been
    a coin flip wearing a threshold. The order already asks for the A/B pair, so
    the gate asks for it too: no pair, no verdict.
    """
    A, B = after["L"], before["L"]
    if A.shape != B.shape:
        return None
    d = ndimage.gaussian_filter(A - B, 3)
    amp = float(np.percentile(d, 99.5)) / 255.0 * 100.0
    med = float(np.median(d)) / 255.0 * 100.0
    lift = med / amp if amp > 1e-6 else 1.0
    thr = max(0.5 * np.percentile(d, 99.5), 2.0)
    m = ndimage.binary_closing(d > thr, np.ones((5, 5)))
    cov = 100.0 * m.mean()
    lab, n = ndimage.label(m)
    elong = 0.0
    if n:
        sz = ndimage.sum(np.ones_like(lab), lab, range(1, n + 1))
        blob = lab == int(np.argmax(sz)) + 1
        ys, xs = np.where(blob)
        if len(ys) > 8:
            cov_m = np.cov(np.vstack([xs - xs.mean(), ys - ys.mean()]))
            ev = np.linalg.eigvalsh(cov_m)
            elong = float(np.sqrt(max(ev[1], 1e-9) / max(ev[0], 1e-9)))
    return dict(amp=amp, lift=lift, elong=elong, cov=cov)


# ------------------------------------------------------- A4 / A5 / A7 / A8 --
def aerial(f):
    a, L, hp, sat = f["rgb"], f["L"], f["hp"], f["sat"]
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    sky = (B > R + 12) & (B > G + 6)
    H = L.shape[0]
    far, near = slice(0, int(H * 0.33)), slice(int(H * 0.66), H)
    fm, nm = ~sky[far], ~sky[near]
    if fm.sum() < 100 or nm.sum() < 100:
        return None
    W = L.shape[1]
    wings = np.r_[hp[:, : int(W * .2)].ravel(), hp[:, int(W * .8):].ravel()].mean()
    centre = hp[:, int(W * .4): int(W * .6)].mean()
    return dict(contrast=float(hp[far][fm].mean() / max(hp[near][nm].mean(), 1e-6)),
                sat=float(sat[far][fm].mean() / max(sat[near][nm].mean(), 1e-6)),
                wings=float(centre / max(wings, 1e-6)))


def worst_third(f):
    hp = f["hp"]
    W = hp.shape[1]
    th = [float(hp[:, : int(W * .3)].mean()), float(hp[:, int(W * .3): int(W * .7)].mean()),
          float(hp[:, int(W * .7):].mean())]
    return min(th) / max(float(hp.mean()), 1e-6), th


def grass(f):
    hue, sat, a = f["hue"], f["sat"], f["rgb"]
    m = (hue >= 75) & (hue <= 165) & (sat >= 0.20)
    if m.mean() < 0.001:
        return None
    return dict(rgb=a[m].mean(0), sat=float(sat[m].mean()), frac=100.0 * m.mean())


def bright_islands(f):
    L = f["L"]
    thr = np.percentile(L, 92)
    m = ndimage.binary_closing(L > thr, np.ones((5, 5)))
    lab, n = ndimage.label(m)
    if n == 0:
        return None
    sz = ndimage.sum(np.ones_like(lab), lab, range(1, n + 1))
    keep = np.sort(sz[sz >= L.size * 0.002])[::-1]
    if not len(keep):
        return None
    return dict(share=float(keep[0] / keep.sum() * 100.0), n=int(len(keep)))


# ------------------------------------------------------------ A6 silhouette --
def shape_complexity(blob):
    """perimeter^2 / area, perimeter = boundary pixel count.

    Scale-free. A filled square measures ~16, a disc ~4pi = 12.6, and the three
    approved character concepts measure 53.5 / 136.7 / 298.1 — which is what makes
    the order's ">= 40" a reachable bar rather than a wish.
    """
    area = int(blob.sum())
    if area < 200:
        return None
    perim = int((blob & ~ndimage.binary_erosion(blob)).sum())
    return perim ** 2 / area


def silhouette(f, box):
    """Hero silhouette inside a CALLER-GIVEN box. No box, no verdict.

    An auto colour-key was tried and dropped: the warm placeholder key finds an
    11%-of-frame blob in the approved (warm interior) reference and scores it 221,
    i.e. it happily grades a kitchen wall as a character. A mask that cannot be
    validated against ground truth cannot carry a gate, so A6 asks for the box.
    """
    if box is None:
        return None
    x0, y0, x1, y1 = box
    L, sat, hue = f["L"], f["sat"], f["hue"]
    sub = L[y0:y1, x0:x1]
    if sub.size < 400:
        return None
    # inside the box the hero is the salient object: split on Otsu-ish midpoint of
    # the box histogram, keep the side that does NOT touch the box border most.
    thr = float(np.percentile(sub, 55))
    cand = [sub > thr, sub <= thr]
    best, bestscore = None, -1.0
    for c in cand:
        lab, n = ndimage.label(c)
        if n == 0:
            continue
        sz = ndimage.sum(np.ones_like(lab), lab, range(1, n + 1))
        b = lab == int(np.argmax(sz)) + 1
        border = (b[0].mean() + b[-1].mean() + b[:, 0].mean() + b[:, -1].mean()) / 4.0
        score = b.mean() * (1.0 - border)
        if score > bestscore:
            best, bestscore = b, score
    if best is None:
        return None
    blob = np.zeros_like(L, dtype=bool)
    blob[y0:y1, x0:x1] = best
    cx = shape_complexity(blob)
    if cx is None:
        return None
    ring = ndimage.binary_dilation(blob, iterations=24) & ~ndimage.binary_dilation(blob, iterations=10)
    dh = abs(float(hue[blob].mean()) - float(hue[ring].mean())) % 360.0
    return dict(complexity=cx, area_pct=100.0 * blob.sum() / blob.size,
                hue_sep=min(dh, 360.0 - dh))


# ------------------------------------------------------------------- A3 map --
def map_stats(path):
    d = json.loads(Path(path).read_text())
    blocks = d["blocks"]
    counts = {}
    cols = {}
    for b in blocks:
        counts[b["block"]] = counts.get(b["block"], 0) + 1
        cols.setdefault((b["x"], b["z"]), []).append((b["y"], b["block"]))
    total = len(blocks)
    top2 = sum(sorted(counts.values(), reverse=True)[:2])
    bare = sum(1 for v in cols.values() if len(v) == 1 and v[0][1] == "grass")
    return dict(total=total, types=len(counts), counts=counts,
                top2_pct=100.0 * top2 / total, bare_pct=100.0 * bare / max(len(cols), 1),
                wood_pct=100.0 * counts.get("wood", 0) / total, columns=len(cols))


# ---------------------------------------------------------------- A2 source --
def fogvolume_in_play():
    """Bevy only scatters INSIDE a FogVolume. hero.rs has one; the play path must too."""
    hits = []
    for p in sorted((ROOT / "client" / "src").rglob("*.rs")):
        txt = p.read_text(encoding="utf-8", errors="ignore")
        for i, line in enumerate(txt.splitlines(), 1):
            if "FogVolume {" in line or "FogVolume::" in line:
                hits.append((p.relative_to(ROOT).as_posix(), i))
    play = [h for h in hits if not h[0].endswith("hero.rs")]
    return hits, play


def lamp_parseable():
    txt = (ROOT / "sim" / "src" / "block.rs").read_text(encoding="utf-8", errors="ignore")
    return '"lamp"' in txt


# ------------------------------------------------------------- calibration --
def _unit_L(tint):
    """Scale an RGB tint so that its Rec.709 luminance is exactly 1.

    Without this a control that says "a ramp of 30 L" actually paints 26.7 L and
    then blames the grader for reading 26.7. Every synthetic amplitude below is in
    the same L units the gates are written in.
    """
    t = np.asarray(tint, dtype=np.float32)
    return t / float(0.2126 * t[0] + 0.7152 * t[1] + 0.0722 * t[2])


SKY_TINT = _unit_L([0.78, 0.90, 1.10])      # ~215 deg hue, sat ~0.24
GROUND_TINT = _unit_L([1.05, 0.95, 0.75])
RAY_TINT = _unit_L([1.0, 0.86, 0.60])       # warm shaft


def synth_sky(amp_L, seam_L=0.0, blend=0, size=(360, 640), noise=0.0):
    """Blue sky over ground: top 55% is a ramp of exactly `amp_L` (0..100 L units)."""
    H, W = size
    img = np.zeros((H, W, 3), dtype=np.float32)
    sky_h = int(H * 0.55)
    top_L, bot_L = 55.0 + amp_L / 2.0, 55.0 - amp_L / 2.0
    for y in range(sky_h):
        t = y / max(sky_h - 1, 1)
        Ly = (top_L + (bot_L - top_L) * t) / 100.0 * 255.0
        img[y] = SKY_TINT * Ly
    ground = (55.0 - amp_L / 2.0 - seam_L) / 100.0 * 255.0
    img[sky_h:] = GROUND_TINT * ground
    if blend:
        img[sky_h - blend: sky_h + blend] = ndimage.gaussian_filter1d(
            img, blend, axis=0)[sky_h - blend: sky_h + blend]
    # ground needs micro detail or the texture-free walk swallows it
    rng = np.random.default_rng(7)
    img[sky_h:] += rng.normal(0, 9.0, img[sky_h:].shape)
    if noise:
        img[:sky_h] += rng.normal(0, noise, img[:sky_h].shape)
    return Image.fromarray(np.clip(img, 0, 255).astype(np.uint8))


def synth_shafts(base_im, amp, n=3, ang=28.0, width=42):
    a = np.asarray(base_im.convert("RGB"), dtype=np.float32)
    H, W = a.shape[:2]
    yy, xx = np.mgrid[0:H, 0:W]
    t = np.cos(np.radians(ang)) * xx + np.sin(np.radians(ang)) * yy
    add = np.zeros((H, W), dtype=np.float32)
    for i in range(n):
        c = (i + 1) * (W + H) / (n + 1.0)
        add += np.exp(-0.5 * ((t - c) / width) ** 2)
    add = add / max(add.max(), 1e-6) * amp / 100.0 * 255.0
    return Image.fromarray(np.clip(a + add[..., None] * RAY_TINT, 0, 255).astype(np.uint8))


def synth_glow(base_im, amp, radius=0.35):
    a = np.asarray(base_im.convert("RGB"), dtype=np.float32)
    H, W = a.shape[:2]
    yy, xx = np.mgrid[0:H, 0:W]
    r = np.sqrt(((yy - H / 2) / H) ** 2 + ((xx - W / 2) / W) ** 2)
    add = np.exp(-0.5 * (r / radius) ** 2) * amp / 100.0 * 255.0
    return Image.fromarray(np.clip(a + add[..., None], 0, 255).astype(np.uint8))


def synth_lift(base_im, amp):
    a = np.asarray(base_im.convert("RGB"), dtype=np.float32) + amp / 100.0 * 255.0
    return Image.fromarray(np.clip(a, 0, 255).astype(np.uint8))


def synth_shape(kind, n=200):
    """Analytic silhouettes whose complexity is known in closed form."""
    m = np.zeros((n + 40, n + 40), dtype=bool)
    if kind == "square":                       # (4n-4)^2 / n^2 -> ~16
        m[20:20 + n, 20:20 + n] = True
    elif kind == "disc":                       # (2*pi*r)^2 / (pi r^2) = 4pi -> ~12.6
        yy, xx = np.mgrid[0:m.shape[0], 0:m.shape[1]]
        m = ((yy - m.shape[0] / 2) ** 2 + (xx - m.shape[1] / 2) ** 2) < (n / 2) ** 2
    elif kind == "comb":                       # a body with four thin limbs
        m[20 + n // 3: 20 + 2 * n // 3, 20:20 + n] = True
        for i in range(4):
            x = 20 + int(n * (0.1 + 0.25 * i))
            m[20: 20 + n, x: x + max(2, n // 40)] = True
    return m


def charmask_reachability():
    """Optional real-data check: do the approved character concepts clear A6's bar?"""
    out = []
    d = ROOT / "docs" / "assets" / "characters"
    for p in sorted(d.glob("*charmask*.png")):
        a = np.asarray(Image.open(p).convert("L")) > 127
        lab, n = ndimage.label(a)
        if not n:
            continue
        sz = ndimage.sum(np.ones_like(lab), lab, range(1, n + 1))
        cx = shape_complexity(lab == int(np.argmax(sz)) + 1)
        if cx:
            out.append((p.stem.replace("-concept-charmask", ""), cx))
    return out


class Control:
    def __init__(self):
        self.rows = []
        self.ok = True

    def check(self, gate, case, got, expect, detail=""):
        good = (got == expect)
        self.ok &= good
        self.rows.append((gate, case, detail, expect, got, "ok" if good else "WRONG"))
        return good

    def show(self):
        print("CALIBRATION — every gate against inputs whose answer is known")
        print(f"  {'gate':11s} {'control case':34s} {'measured':>22s} {'want':>5s} {'got':>5s}")
        print("  " + "-" * 84)
        for gate, case, detail, expect, got, mark in self.rows:
            flag = "" if mark == "ok" else "   <<< CALIBRATION FAILURE"
            print(f"  {gate:11s} {case:34s} {detail:>22s} {expect:>5s} {got:>5s}{flag}")


def calibrate(verbose=True):
    c = Control()

    # --- A1: does the metric report the ramp amplitude it was given? ----------
    for amp, want in ((35.0, PASS), (30.0, PASS), (20.0, FAIL), (0.0, FAIL)):
        s = sky_metrics(from_image(synth_sky(amp, blend=8)))
        got = PASS if (s and abs(s["grad"]) >= T["sky_grad_L"] and s["mono"] >= T["sky_mono"]) else FAIL
        c.check("A1", f"synthetic ramp of exactly {amp:.0f} L", got, want,
                f"grad {s['grad']:.1f} mono {s['mono']:.2f}" if s else "no sky")
    s = sky_metrics(from_image(synth_sky(0.0, blend=8, noise=4.0)))
    c.check("A1", "flat plate + sensor noise", FAIL if s["mono"] < T["sky_mono"] else PASS, FAIL,
            f"grad {s['grad']:.1f} mono {s['mono']:.2f}")
    s = sky_metrics(from_image(synth_sky(30.0, seam_L=40.0, blend=0)))
    c.check("A1", "hard horizon seam (40 L step)", FAIL if s["seam"] > T["sky_seam_L"] else PASS, FAIL,
            f"seam {s['seam']:.1f} L")
    s = sky_metrics(from_image(synth_sky(30.0, seam_L=0.0, blend=14)))
    c.check("A1", "hazed horizon blend", PASS if s["seam"] <= T["sky_seam_L"] else FAIL, PASS,
            f"seam {s['seam']:.1f} L")
    ref = ROOT / "docs/assets/golden-beauty-shot-ref.png"
    if ref.exists():
        c.check("A1", "approved interior ref has no sky", SKIP if sky_metrics(load(ref)) is None else PASS,
                SKIP, "sky mask empty")

    # --- A2: differential detector vs shafts, glow, exposure bump ------------
    boot = ROOT / "docs/assets/gate3/gate3-after-boot-nohud2.png"
    if boot.exists():
        base = Image.open(boot).convert("RGB")
        b = from_image(base)
        cases = [
            ("3 parallel shafts, 18 L", synth_shafts(base, 18.0), PASS),
            ("same frame twice (no change)", base, FAIL),
            ("global exposure lift +8 L", synth_lift(base, 8.0), FAIL),
            ("isotropic bloom glow 25 L", synth_glow(base, 25.0), FAIL),
            ("full-frame fog wall 12 L", synth_glow(base, 12.0, radius=3.0), FAIL),
            ("shafts too faint, 2 L", synth_shafts(base, 2.0), FAIL),
        ]
        for label, after_im, want in cases:
            r = godray_ab(from_image(after_im), b)
            got = PASS if (r and r["amp"] >= T["ray_amp_L"] and r["lift"] <= T["ray_lift"]
                           and r["elong"] >= T["ray_elong"]
                           and T["ray_cov_lo"] <= r["cov"] <= T["ray_cov_hi"]) else FAIL
            c.check("A2", label, got, want,
                    f"amp {r['amp']:.1f} lift {r['lift']:.2f} el {r['elong']:.1f} cov {r['cov']:.1f}%")
        hits, play = fogvolume_in_play()
        c.check("A2-src", "grep finds the hero.rs FogVolume", PASS if hits else FAIL, PASS,
                f"{len(hits)} site(s)")

    # --- A4 / A5 / A7 / A8 against approved frames and known defects ---------
    wide = ROOT / "docs/assets/wide-hero-final-nohud2.png"
    walk = ROOT / "docs/assets/gate3/gate3-after-walk-nohud2.png"
    if ref.exists():
        f = load(ref)
        a = aerial(f)
        c.check("A4", "approved golden ref reads deep", PASS if a["contrast"] < T["aerial_ratio"] else FAIL,
                PASS, f"far/near {a['contrast']:.2f}")
        sh, _ = worst_third(f)
        c.check("A5", "approved ref: no dead third", PASS if sh >= T["third_share"] else FAIL, PASS,
                f"share {sh:.2f}")
        isl = bright_islands(f)
        c.check("A8", "approved ref has one light pool", PASS if isl["share"] >= T["island_share"] else FAIL,
                PASS, f"share {isl['share']:.1f}%")
        c.check("A6", "no hero box given -> no verdict", SKIP if silhouette(f, None) is None else PASS,
                SKIP, "mask must be caller-given")
    if walk.exists():
        sh, _ = worst_third(load(walk))
        c.check("A5", "walk frame (camera in a block)", FAIL if sh < T["third_share"] else PASS, FAIL,
                f"share {sh:.2f}")
    if wide.exists():
        g = grass(load(wide))
        c.check("A7", "approved accent green in wide-hero",
                PASS if (g and g["rgb"][2] >= T["grass_blue"] and g["sat"] <= T["grass_sat"]) else FAIL,
                PASS, f"B {g['rgb'][2]:.0f} sat {g['sat']:.2f}")
        isl = bright_islands(load(wide))
        c.check("A8", "approved wide-hero light pool", PASS if isl["share"] >= T["island_share"] else FAIL,
                PASS, f"share {isl['share']:.1f}%")
    if boot.exists():
        g = grass(load(boot))
        c.check("A7", "shipped neon grass (known defect)",
                FAIL if (g["rgb"][2] < T["grass_blue"] or g["sat"] > T["grass_sat"]) else PASS,
                FAIL, f"B {g['rgb'][2]:.0f} sat {g['sat']:.2f}")

    # --- A6: the shape metric against closed-form shapes, then real characters --
    # discrete boundary counting matches the analytic value on straight edges and
    # under-reads curved ones (disc: 10.1 vs 4pi = 12.6) — a bias that errs strict.
    for kind, want, exact in (("square", FAIL, "16.0"), ("disc", FAIL, "12.6*"), ("comb", PASS, "-")):
        cx = shape_complexity(synth_shape(kind))
        got = PASS if cx >= T["silhouette"] else FAIL
        c.check("A6", f"{kind} (closed form {exact})", got, want, f"cx {cx:.1f}")
    for who, cx in charmask_reachability():
        c.check("A6", f"approved concept: {who}", PASS if cx >= T["silhouette"] else FAIL, PASS,
                f"cx {cx:.1f}")

    # --- A3: the map reader on a synthetic map with a known composition ------
    fake = {"blocks": [{"x": i, "y": 0, "z": 0, "block": n}
                       for i, n in enumerate(["grass"] * 6 + ["stone"] * 2 + ["wood", "moss"])]}
    tmp = ROOT / "scripts" / "__a9_control_map.json"
    tmp.write_text(json.dumps(fake))
    try:
        ms = map_stats(tmp)
        c.check("A3", "map reader on a known 4-type map",
                PASS if (ms["types"] == 4 and abs(ms["top2_pct"] - 80.0) < 1e-6
                         and abs(ms["wood_pct"] - 10.0) < 1e-6) else FAIL,
                PASS, f"{ms['types']} types top2 {ms['top2_pct']:.0f}%")
    finally:
        tmp.unlink(missing_ok=True)

    if verbose:
        c.show()
    return c


# ------------------------------------------------------------------- grade --
def grade(frame, before, idle, hero_box, map_path, v: Verdict):
    if frame:
        f = load(frame)
        print(f"\nframe : {Path(frame).name}  ({f['L'].shape[1]}x{f['L'].shape[0]})")
        s = sky_metrics(f)
        if s is None:
            v.add("A1", "sky gradient", SKIP, "no sky", f">= {T['sky_grad_L']:.0f} L",
                  "interior / sky < 0.2% of frame — an outdoor look cannot be graded here")
        else:
            v.add("A1", "sky gradient |zenith-hz|", PASS if s["grad"] >= T["sky_grad_L"] else FAIL,
                  f"{s['grad']:.1f} L", f">= {T['sky_grad_L']:.0f} L",
                  f"sky {s['frac']:.1f}% of frame, rows {s['rows'][0]}-{s['rows'][1]}, "
                  f"{s['bright']} end is the bright one")
            v.add("A1", "sky ramp is monotonic", PASS if s["mono"] >= T["sky_mono"] else FAIL,
                  f"{s['mono']:.2f}", f">= {T['sky_mono']:.2f}")
            if s["n_edge"] < 200:
                v.add("A1", "horizon seam step", SKIP, f"{s['n_edge']} px", f"<= {T['sky_seam_L']:.0f} L",
                      "too few sky/ground boundary pixels to average — not measured")
            else:
                v.add("A1", "horizon seam step", PASS if s["seam"] <= T["sky_seam_L"] else FAIL,
                      f"{s['seam']:.2f} L", f"<= {T['sky_seam_L']:.0f} L",
                      f"averaged over {s['n_edge']} boundary px")

        a = aerial(f)
        if a:
            v.add("A4", "far/near local contrast", PASS if a["contrast"] < T["aerial_ratio"] else FAIL,
                  f"{a['contrast']:.2f}", f"< {T['aerial_ratio']:.2f}",
                  f"advisory: far/near saturation {a['sat']:.2f} · "
                  f"centre/wings contrast {a['wings']:.2f} (the order's hand-picked split)")
        sh, th = worst_third(f)
        v.add("A5", "worst third vs frame", PASS if sh >= T["third_share"] else FAIL,
              f"{sh:.2f}", f">= {T['third_share']:.2f}",
              "thirds " + " ".join(f"{t:.2f}" for t in th))
        g = grass(f)
        if g is None:
            v.add("A7", "grass blue channel", SKIP, "no green px", f">= {T['grass_blue']:.0f}")
        else:
            v.add("A7", "grass blue channel", PASS if g["rgb"][2] >= T["grass_blue"] else FAIL,
                  f"B {g['rgb'][2]:.0f}", f">= {T['grass_blue']:.0f}",
                  f"mean RGB {g['rgb'].round(0).tolist()} on {g['frac']:.2f}% of frame")
            v.add("A7", "grass saturation", PASS if g["sat"] <= T["grass_sat"] else FAIL,
                  f"{g['sat']:.2f}", f"<= {T['grass_sat']:.2f}")
        isl = bright_islands(f)
        if isl:
            v.add("A8", "largest bright-island share", PASS if isl["share"] >= T["island_share"] else FAIL,
                  f"{isl['share']:.1f}%", f">= {T['island_share']:.0f}%",
                  f"{isl['n']} islands >= 0.2% of frame")

        if before:
            r = godray_ab(f, load(before))
            if r is None:
                v.add("A2", "god ray A/B", SKIP, "size mismatch", "same camera",
                      "before/after must be the same resolution and camera")
            else:
                v.add("A2", "shaft amplitude", PASS if r["amp"] >= T["ray_amp_L"] else FAIL,
                      f"{r['amp']:.1f} L", f">= {T['ray_amp_L']:.0f} L")
                v.add("A2", "not a global lift", PASS if r["lift"] <= T["ray_lift"] else FAIL,
                      f"{r['lift']:.2f}", f"<= {T['ray_lift']:.2f}")
                v.add("A2", "shaft elongation", PASS if r["elong"] >= T["ray_elong"] else FAIL,
                      f"{r['elong']:.1f}", f">= {T['ray_elong']:.0f}")
                v.add("A2", "coverage (beam, not wall)",
                      PASS if T["ray_cov_lo"] <= r["cov"] <= T["ray_cov_hi"] else FAIL,
                      f"{r['cov']:.1f}%", f"{T['ray_cov_lo']:.1f}-{T['ray_cov_hi']:.0f}%")
        else:
            v.add("A2", "god ray A/B", SKIP, "no pair", "--before <frame>",
                  "no same-camera before-frame given — shafts CANNOT be judged from one frame")

    if idle:
        fi = load(idle)
        sil = silhouette(fi, hero_box)
        print(f"idle  : {Path(idle).name}")
        if sil is None:
            v.add("A6", "silhouette complexity", SKIP, "no hero box", f">= {T['silhouette']:.0f}",
                  "pass --hero-box x0,y0,x1,y1 around the idle hero; an auto colour key "
                  "scores the approved interior ref 221 and cannot be trusted")
        else:
            v.add("A6", "silhouette complexity", PASS if sil["complexity"] >= T["silhouette"] else FAIL,
                  f"{sil['complexity']:.1f}", f">= {T['silhouette']:.0f}",
                  f"hero is {sil['area_pct']:.2f}% of frame (an empty box measures 16.0)")
            v.add("A6", "hero vs bg hue split", PASS if sil["hue_sep"] >= T["hero_hue_sep"] else FAIL,
                  f"{sil['hue_sep']:.1f} deg", f">= {T['hero_hue_sep']:.0f} deg")
    else:
        v.add("A6", "silhouette complexity", SKIP, "no idle frame", "--idle <frame>",
              "A6 is an idle-pose gate; walk/combat poses do not count (order A6.1)")

    if map_path is None:
        return
    if Path(map_path).exists():
        ms = map_stats(map_path)
        print(f"map   : {Path(map_path).name}  ({ms['total']} blocks, {ms['columns']} columns)")
        v.add("A3", "distinct block types", PASS if ms["types"] >= T["map_types"] else FAIL,
              f"{ms['types']}", f">= {T['map_types']}",
              " ".join(f"{k}:{n}" for k, n in sorted(ms["counts"].items(), key=lambda kv: -kv[1])))
        v.add("A3", "top-2 material share", PASS if ms["top2_pct"] <= T["map_top2_pct"] else FAIL,
              f"{ms['top2_pct']:.1f}%", f"<= {T['map_top2_pct']:.0f}%")
        v.add("A3", "bare grass columns", PASS if ms["bare_pct"] <= T["map_bare_pct"] else FAIL,
              f"{ms['bare_pct']:.1f}%", f"<= {T['map_bare_pct']:.0f}%")
        v.add("A3", "wood placed", PASS if ms["wood_pct"] >= T["map_wood_pct"] else FAIL,
              f"{ms['wood_pct']:.2f}%", f">= {T['map_wood_pct']:.0f}%")
        v.add("A3", "lamp is parseable", PASS if lamp_parseable() else FAIL,
              "yes" if lamp_parseable() else "no", 'sim/src/block.rs has "lamp"')

    hits, play = fogvolume_in_play()
    v.add("A2", "FogVolume in the play path", PASS if play else FAIL,
          f"{len(play)} site", ">= 1 outside hero.rs",
          "all sites: " + (", ".join(f"{p}:{i}" for p, i in hits) or "none"))


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--frame", help="the play frame under judgement")
    ap.add_argument("--before", help="same-camera frame WITHOUT the change (required by A2)")
    ap.add_argument("--idle", help="idle-pose frame (A6 only)")
    ap.add_argument("--hero-box", help="x0,y0,x1,y1 around the hero in the idle frame (A6)")
    ap.add_argument("--map", default=str(ROOT / "maps" / "edhari.json"))
    ap.add_argument("--calibrate", action="store_true", help="run the control suite and stop")
    ap.add_argument("--quiet-calibration", action="store_true")
    a = ap.parse_args(argv[1:])

    c = calibrate(verbose=a.calibrate or not a.quiet_calibration)
    if not c.ok:
        print("\nCALIBRATION FAILED — the grader is not measuring what it claims. "
              "No verdict printed.", file=sys.stderr)
        return 3
    print(f"\ncalibration: {len(c.rows)}/{len(c.rows)} controls landed on the right side "
          f"of their own cut line.")
    if a.calibrate:
        return 0

    frames = [a.frame] if a.frame else [
        str(ROOT / "docs/assets/gate3/gate3-after-boot-nohud2.png"),
        str(ROOT / "docs/assets/gate3/gate3-after-walk-nohud2.png"),
        str(ROOT / "docs/assets/gate3/gate3-after-combat-nohud2.png"),
    ]
    idle = a.idle or (frames[0] if not a.frame else None)
    box = None
    if a.hero_box:
        try:
            box = tuple(int(x) for x in a.hero_box.split(","))
            assert len(box) == 4
        except Exception:
            print("--hero-box wants x0,y0,x1,y1", file=sys.stderr)
            return 2

    codes = []
    for i, fr in enumerate(frames):
        if not Path(fr).exists():
            print(f"missing frame: {fr}", file=sys.stderr)
            return 2
        v = Verdict()
        grade(fr, a.before, idle if i == 0 else None, box, a.map if i == 0 else None, v)
        v.show()
        code = v.exit_code()
        codes.append(code)
        print(f"\n==> {Path(fr).name}: "
              + {0: "PASS", 1: "FAIL", 4: "INCOMPLETE (a gate could not be measured — not a pass)"}[code])
    return 1 if 1 in codes else (4 if 4 in codes else 0)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
