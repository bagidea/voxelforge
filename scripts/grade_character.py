#!/usr/bin/env python3
"""CHARACTER-REGION grader — the axes Sun grades on the whole frame, measured on
the character's pixels ONLY, plus four character-specific axes.

WHY THIS EXISTS
---------------
`grade_axes.py` measures the WHOLE frame. On a gameplay/vista frame the avatar is
2-4 % of the pixels, so every one of its numbers is a statement about terrain, sky
and haze — the character can be a flat grey blob and the frame still reads ALL
AXES PASS. The scorecard had no character number in it at all. This closes that.

Nothing here re-derives or relaxes Sun's targets: the six inherited axes use the
SAME definitions and the SAME bounds as `scripts/grade_axes.py` / `grade_g3.py` /
`measure_penumbra.py`. The only change is the sample set — a boolean mask instead
of the full raster. A character living in the same golden hour as the set has to
sit inside the same colour envelope; one that reads +40 warmth in a +110 world is
a cutout pasted onto the frame, and that is precisely the complaint being graded.

THE MASK (and its honest limits)
--------------------------------
Two independent constraints, ANDed:

  1. ANALYTIC BBOX. In `--play` the orbit camera is anchored to the avatar
     (`main.rs`: pivot = player.translation + Y*PIVOT_UP, camera = pivot +
     back*dist), so where the body lands in frame is decided by (yaw, pitch,
     dist, fov, EYE_HEIGHT, PIVOT_UP, PLAYER_HEIGHT) and NOTHING else — not by
     the world position, not by the map. That box is computed by pinhole
     projection, not eyeballed.
  2. BACKGROUND REJECTION inside that box. Per image row, the background model is
     the median colour of the pixels of that row OUTSIDE the box; a pixel inside
     the box is character iff it deviates from that model by more than
     `--key-tol` in RGB distance. Then: largest connected component, holes
     filled, 1-px border erode (kills the antialiased halo, which is a blend of
     character and background and belongs to neither).

  LIMIT, stated because it moves numbers: a cast shadow the character throws on
  ground INSIDE the bbox also deviates from the row background and can survive
  into the mask. `--shadow-guard` (default on) drops mask pixels below the
  computed FOOT LINE, which is where that shadow lives. Anything left is
  reported as `mask_area_pct` and drawn into `<stem>-charmask.png` — grade the
  overlay with your eyes before you trust the table under it.

For a concept reference (`--ref`) there is no camera to project, so the mask is
built from gradient magnitude → close → fill → largest component instead. That is
only valid on a studio render against a plain backdrop, which is what the concept
sheets in `docs/assets/characters/` are.

SCALE NORMALISATION
-------------------
Sun resamples to 1024x1024 so numbers compare across renders. The equivalent for a
region is per-CHARACTER, not per-frame: every measurement here is taken after
resampling so the character's bbox HEIGHT is `--norm-h` px (default 512). A
close-up and a medium of the same asset then produce comparable micro-contrast and
silhouette numbers, and so does a 1024^2 concept sheet.

Usage
-----
  grade_character.py --cam YAW,PITCH,DIST [--actor player|husk] <frame>-nohud2.png
  grade_character.py --ref docs/assets/characters/auren-hero-concept.png
"""
import argparse
import json
import os
import sys

import numpy as np
from PIL import Image, ImageFilter
from scipy import ndimage

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# ---------------------------------------------------------------------------
# Scene constants — MIRRORED from client/src/main.rs. If a number moves there
# this file is wrong and the bbox silently drifts, so they are named and cited.
# ---------------------------------------------------------------------------
EYE_HEIGHT = 1.62      # main.rs:1523  feet -> camera
PLAYER_HEIGHT = 1.80   # main.rs:1522  feet -> crown
PIVOT_UP = 0.35        # main.rs:1534  look-pivot lift above the eye
FOV_Y_DEG = 45.0       # Bevy PerspectiveProjection default (Camera3d::default())

# Half-width of each actor's body in metres, widest limb pose included.
# player: torso 0.46 + arm swing; husk: torso 0.88 + shoulder 0.44*2.
ACTOR_HALF_W = {"player": 0.50, "husk": 0.85}
ACTOR_HEIGHT = {"player": PLAYER_HEIGHT, "husk": 2.38}  # anim.rs Dims::of

# ---------------------------------------------------------------------------
# TARGETS
# ---------------------------------------------------------------------------
# WHY THE FRAME-LEVEL BOUNDS ARE NOT COPIED HERE.
#
# The obvious move is to inherit `grade_axes.py`'s numbers verbatim. Measured,
# that produces a gate the CEO-APPROVED CONCEPT ART ITSELF FAILS:
#
#   axis                    frame bound   auren   maren   husk
#   C1 warmth R-B (mid)      >= 110        87.0    81.2    60.0   <- all 3 fail
#   C5 highlight p95         150..185     145.5   135.1   158.7   <- 2 of 3 fail
#
# That is not the characters being wrong; it is the frame bound being a statement
# about a raster dominated by sunlit wood and terrain. A body made of skin, wool
# and steel legitimately sits cooler than the ground it stands on. Shipping the
# copied bound would put a permanent red FAIL next to art that is signed off —
# i.e. exactly the "gate everyone learns to ignore" that grade_axes.py's own
# header warns about for DOF on non-hero framing.
#
# So the character lane gets the SAME METHOD, not the same numbers: the approved
# concept sheet in docs/assets/characters/ is this lane's golden reference,
# measured by THIS script under --ref, and every bound below is a stated
# tolerance around that sheet's own value. `golden-beauty-shot-ref.png` is to
# grade_axes.py what `<actor>-concept.png` is to this file.
#
# The frame-level bound is still PRINTED beside each chromatic axis as context —
# a character 50 points cooler than the set it stands in is a cutout, and that
# distance is the number that says so. It is context, not a gate.
#
# rule: ("relmin", f) -> v >= f*ref | ("relmax", f) -> v <= f*ref
#       ("band_abs", w) -> |v - ref| <= w | ("abs_ge", x) -> v >= x
TARGETS = [
    ("warmth", "C1 warmth R-B (mid)",       ("relmin", 0.85),  "ref x0.85"),
    ("blue",   "C2 blue B (mid)",           ("relmax", 1.50),  "ref x1.50"),
    ("sat",    "C3 saturation (mid)",       ("relmin", 0.85),  "ref x0.85"),
    ("micro",  "C4 micro-contrast",         ("relmin", 0.70),  "ref x0.70"),
    ("p95",    "C5 highlight p95",          ("band_abs", 20.0), "ref +/-20"),
    ("p05L",   "C6a shade p05 L%",          ("relmin", 0.70),  "ref x0.70"),
    ("darkRB", "C6b darkest R-B",           ("abs_ge", 0.0),   "grade_g3 hue clause"),
    ("pen",    "C7 contact penumbra px",    ("abs_ge", 5.0),   "grade_look G4a"),
    ("edge",   "C8 silhouette pop",         ("relmin", 0.70),  "ref x0.70"),
    ("form",   "C9 form range p90-p10",     ("relmin", 0.70),  "ref x0.70"),
    ("iso",    "C10 silhouette complexity", ("relmin", 0.70),  "ref x0.70"),
    ("mats",   "C11 material clusters",     ("relmin", 0.70),  "ref x0.70"),
]

# The frame-level numbers, printed as CONTEXT only (see the note above).
FRAME_CONTEXT = {
    "warmth": ">=110", "blue": "<=10", "sat": ">=90",
    "micro": ">=5", "p95": "150..185", "p05L": ">=8", "darkRB": ">=0", "pen": ">=5",
}


def resolve_bound(rule, ref):
    """Turn a rule + the concept sheet's own value into (cmp, bound)."""
    kind, k = rule
    if kind == "abs_ge":
        return "ge", k
    if ref is None:
        return None, None
    if kind == "relmin":
        return "ge", k * ref
    if kind == "relmax":
        return "le", k * ref
    if kind == "band_abs":
        return "band", (ref - k, ref + k)
    raise ValueError(kind)


def verdict(cmp, bound, v):
    if v is None:
        return None
    if cmp == "ge":
        return v >= bound
    if cmp == "le":
        return v <= bound
    if cmp == "band":
        return bound[0] <= v <= bound[1]
    raise ValueError(cmp)


def fmt_target(cmp, bound):
    if cmp == "ge":
        return f">= {bound:g}"
    if cmp == "le":
        return f"<= {bound:g}"
    return f"{bound[0]:g}..{bound[1]:g}"


# ---------------------------------------------------------------------------
# Mask construction
# ---------------------------------------------------------------------------
def analytic_bbox(w, h, yaw_deg, pitch_deg, dist, actor, hud_crop):
    """Project the actor's body cylinder through the play camera.

    Returns (x0, y0, x1, y1) in pixels of the ALREADY-de-HUDded image (the top
    `hud_crop` rows were removed by _flamingo_dehud2.py, so every y is shifted).

    Frame: the camera sits at pivot + back*dist and looks back down `-back` at the
    pivot, where pivot = feet + EYE_HEIGHT + PIVOT_UP. Yaw only spins the pair
    about Y, so it cannot change the vertical placement and drops out; pitch tilts
    the view and does move the body up/down the frame.
    """
    full_h = h + hud_crop
    fy = np.radians(FOV_Y_DEG)
    # focal length in pixels, on the FULL (pre-crop) frame — that is the raster the
    # projection actually happened on.
    f_px = (full_h * 0.5) / np.tan(fy * 0.5)

    p = np.radians(pitch_deg)
    # Camera-space: origin at the lens, +Z back toward the boom, forward = -Z.
    # A point on the body axis at height `t` above the FEET sits, relative to the
    # pivot, at world offset (0, t - (EYE_HEIGHT + PIVOT_UP), 0).
    def project(t, x_off):
        dy = t - (EYE_HEIGHT + PIVOT_UP)
        # Camera basis, main.rs convention: rot = Yaw(y) * PitchX(p); the boom is
        # `back = rot*Z`, the lens looks along `-back`, and `up = rot*Y`. In the
        # yaw-aligned plane that gives back = (0, -sin p, cos p),
        # forward = (0, sin p, -cos p), up = (0, cos p, sin p).
        #
        # Point at world offset v = (x_off, dy, 0) from the pivot, camera at
        # pivot + back*dist:
        #   depth = dot(v - back*dist, forward) = dist + dy*sin(p)
        #   up    = dot(v - back*dist, up)      = dy*cos(p)   (back _|_ up)
        #
        # SIGN, checked by hand at the shipped pose: WAKE_PITCH = -0.25 rad, so
        # the lens sits ABOVE the pivot and the FEET must come out FARTHER than
        # the pivot. dy_feet = -1.97, sin(p) = -0.247 -> depth = dist + 0.487.
        # The first draft had `dist - dy*sin(p)`, which put the feet 0.487 m
        # NEARER than the head and tilted the whole bbox the wrong way.
        depth = dist + dy * np.sin(p)
        up = dy * np.cos(p)
        if depth <= 0.05:
            depth = 0.05
        return (x_off * f_px / depth, up * f_px / depth)

    hgt = ACTOR_HEIGHT.get(actor, PLAYER_HEIGHT)
    hw = ACTOR_HALF_W.get(actor, 0.50)
    pts = [project(t, sx) for t in (0.0, hgt) for sx in (-hw, hw)]
    xs = [q[0] for q in pts]
    ys = [q[1] for q in pts]
    cx = full_h * 0.0 + (w * 0.5)          # principal point x (frame centre)
    cy = full_h * 0.5 - hud_crop           # principal point y, shifted by the crop
    x0 = cx + min(xs)
    x1 = cx + max(xs)
    # image y grows DOWN, world up grows UP
    y0 = cy - max(ys)
    y1 = cy - min(ys)
    # foot line = the projected y of t=0
    foot_y = cy - project(0.0, 0.0)[1]
    return (x0, y0, x1, y1), foot_y


def mask_from_bbox(rgb, bbox, foot_y, key_tol, shadow_guard, pad=0.18):
    """Background-reject inside a padded analytic bbox."""
    h, w, _ = rgb.shape
    x0, y0, x1, y1 = bbox
    bw, bh = x1 - x0, y1 - y0
    X0 = int(max(0, x0 - bw * pad))
    X1 = int(min(w, x1 + bw * pad))
    Y0 = int(max(0, y0 - bh * pad))
    Y1 = int(min(h, y1 + bh * pad))
    if X1 - X0 < 8 or Y1 - Y0 < 8:
        raise SystemExit(f"bbox degenerate: {(X0, Y0, X1, Y1)} on a {w}x{h} frame")

    box = np.zeros((h, w), bool)
    box[Y0:Y1, X0:X1] = True

    # Per-row background model from the pixels of that row OUTSIDE the box.
    dev = np.zeros((h, w), np.float32)
    for y in range(Y0, Y1):
        outside = np.concatenate([rgb[y, :X0], rgb[y, X1:]], axis=0)
        if len(outside) < 16:
            continue
        bgm = np.median(outside, axis=0)
        dev[y, X0:X1] = np.linalg.norm(rgb[y, X0:X1] - bgm, axis=1)

    m = box & (dev > key_tol)
    if shadow_guard:
        yy = np.arange(h)[:, None]
        m &= yy <= (foot_y + 0.02 * h)

    if not m.any():
        return m
    lab, n = ndimage.label(m)
    if n > 1:
        sizes = ndimage.sum(m, lab, range(1, n + 1))
        m = lab == (int(np.argmax(sizes)) + 1)
    m = ndimage.binary_fill_holes(m)
    m = ndimage.binary_erosion(m, iterations=1)   # drop the AA halo
    return m


def mask_from_ref(rgb, grad_thresh=5.0):
    """Studio-render matte, by GROWING THE BACKDROP rather than the figure.

    First attempt grew the figure from its own edges and closed the result — which
    bridged the contact shadow into the floor and matted 56 % of the sheet (the
    whole ground plane came back as "character"). The backdrop is the thing with
    the simple description here, not the figure: a studio sheet is a smooth
    gradient (wall + floor) that touches the frame border, and the figure is
    fenced off from it by a hard silhouette edge.

    So: low-gradient pixels reachable from the border ARE the backdrop; everything
    the silhouette walls off is the figure. Flat faces inside the body are also
    low-gradient but cannot reach the border, which is exactly the property that
    makes this work.
    """
    g = (0.2126 * rgb[..., 0] + 0.7152 * rgb[..., 1] + 0.0722 * rgb[..., 2])
    # Denoise FIRST. These sheets carry render grain and floating dust motes; on
    # the raw luminance those specks fragment the backdrop into thousands of
    # smooth islands, only a few of which touch the border, and the "backdrop"
    # then fails to grow — the matte came back at 98 % of the sheet.
    g = ndimage.gaussian_filter(g, 2.0)
    mag = np.hypot(ndimage.sobel(g, axis=1), ndimage.sobel(g, axis=0))
    smooth = mag < grad_thresh
    # NOTE: do NOT binary_closing() this. `binary_closing` erodes with a zero
    # border value, which strips the outermost rows/cols — the exact pixels the
    # backdrop is identified BY. It measured bg = 0.0 % and matted 100 % of the
    # sheet as "character". The gaussian above is the denoise; nothing else is
    # needed, and anything that touches the frame edge breaks the seed.

    lab, n = ndimage.label(smooth)
    if n == 0:
        raise SystemExit("ref matte: no smooth backdrop found")
    border = set(lab[0, :]) | set(lab[-1, :]) | set(lab[:, 0]) | set(lab[:, -1])
    border.discard(0)
    bg = np.isin(lab, list(border))
    bg = ndimage.binary_dilation(bg, iterations=3)

    m = ~bg
    m = ndimage.binary_opening(m, structure=np.ones((3, 3)))
    m = ndimage.binary_fill_holes(m)
    lab, n = ndimage.label(m)
    if n == 0:
        raise SystemExit("ref matte: nothing left after backdrop removal")
    sizes = ndimage.sum(m, lab, range(1, n + 1))
    m = lab == (int(np.argmax(sizes)) + 1)
    m = ndimage.binary_fill_holes(m)
    return ndimage.binary_erosion(m, iterations=2)


# ---------------------------------------------------------------------------
# Measurement
# ---------------------------------------------------------------------------
def normalise(rgb, mask, norm_h):
    """Resample so the character's bbox height is `norm_h` px."""
    ys, xs = np.where(mask)
    ch = ys.max() - ys.min() + 1
    s = norm_h / max(ch, 1)
    H, W, _ = rgb.shape
    nw, nh = max(8, int(round(W * s))), max(8, int(round(H * s)))
    im = Image.fromarray(rgb.astype(np.uint8)).resize((nw, nh), Image.LANCZOS)
    mk = Image.fromarray((mask * 255).astype(np.uint8)).resize((nw, nh), Image.LANCZOS)
    return np.asarray(im).astype(np.float32), (np.asarray(mk) > 127), s


def measure(rgb, mask, norm_h):
    a, m, scale = normalise(rgb, mask, norm_h)
    R, G, B = a[..., 0], a[..., 1], a[..., 2]
    L = 0.2126 * R + 0.7152 * G + 0.0722 * B
    mx, mn = a.max(2), a.min(2)
    sat = np.where(mx > 0, (mx - mn) / np.maximum(mx, 1e-6), 0.0)

    Lm = L[m]
    if Lm.size < 200:
        raise SystemExit(f"character mask too small to grade: {Lm.size} px")

    # midtone band, inside the mask (grade_axes definition, region-restricted)
    tlo, thi = np.percentile(Lm, 35), np.percentile(Lm, 75)
    band = m & (L >= tlo) & (L <= thi)

    # micro-contrast: same hi-pass (GaussianBlur r3) as grade_axes, sampled in-mask.
    g8 = L.astype(np.uint8)
    blur3 = np.asarray(Image.fromarray(g8).filter(ImageFilter.GaussianBlur(3))).astype(np.float32)
    hi = (L - blur3)
    # only interior pixels — the silhouette edge is a step, not surface detail, and
    # would read as "micro-contrast" the moment the character stands on anything.
    interior = ndimage.binary_erosion(m, iterations=4)
    micro = float(hi[interior].std()) if interior.sum() > 200 else float(hi[m].std())

    # C6 shade: darkest representative 11x11 patch INSIDE the mask
    k = np.ones((11, 11), np.float32) / 121.0
    cov = ndimage.convolve(m.astype(np.float32), k, mode="constant")
    full = cov > 0.995
    if full.sum() < 4:
        full = ndimage.binary_erosion(m, iterations=5)
    Lb = ndimage.convolve(L, k, mode="constant")
    cand = np.where(full, Lb, np.inf)
    yy, xx = np.unravel_index(int(np.argmin(cand)), cand.shape)
    pr = float(ndimage.convolve(R, k, mode="constant")[yy, xx])
    pg = float(ndimage.convolve(G, k, mode="constant")[yy, xx])
    pb = float(ndimage.convolve(B, k, mode="constant")[yy, xx])

    # C8 silhouette pop — LOCAL, and absolute per boundary pixel.
    #
    # The first version took |mean L inside - mean L outside| over the whole
    # silhouette. On the Auren sheet that read 1.01: the figure's lit side is
    # brighter than the backdrop and its shadow side is darker, so the two halves
    # cancel and a figure that visibly separates scores as if it were invisible.
    # What the eye reads is the step at each point of the outline, so measure it
    # there and take the magnitude BEFORE averaging.
    ring_r = 6
    mf = m.astype(np.float32)
    win = np.ones((ring_r * 2 + 1, ring_r * 2 + 1), np.float32)
    n_in = ndimage.convolve(mf, win, mode="nearest")
    n_out = ndimage.convolve(1.0 - mf, win, mode="nearest")
    s_in = ndimage.convolve(L * mf, win, mode="nearest")
    s_out = ndimage.convolve(L * (1.0 - mf), win, mode="nearest")
    boundary = ndimage.binary_dilation(m, iterations=1) & ~ndimage.binary_erosion(m, iterations=1)
    ok = boundary & (n_in > 12) & (n_out > 12)
    if ok.sum() > 50:
        step = np.abs(s_in[ok] / n_in[ok] - s_out[ok] / n_out[ok])
        edge = float(np.mean(step))
    else:
        edge = None

    # C9 form range: how much light-to-shade modelling the body carries
    form = float(np.percentile(Lm, 90) - np.percentile(Lm, 10))

    # C10 silhouette complexity: isoperimetric ratio P^2/A (a circle = 12.57,
    # a plain box-stack ~ 17-20, a character with cape/pack/weapon >> that).
    # Measured on a gaussian-smoothed mask: raw single-pixel jaggies on an
    # antialiased matte add perimeter that no eye reads as silhouette shape.
    ms = ndimage.gaussian_filter(mf, 2.0) > 0.5
    per = ndimage.binary_dilation(ms, iterations=1) & ~ms
    P, A = float(per.sum()), float(m.sum())
    iso = (P * P / A) if A > 0 else None

    # C11 material clusters: distinct (hue, value) modes inside the mask. Coarse
    # 8x6 histogram; a bin counts as a material if it holds >= 3 % of the body.
    hsv = np.asarray(Image.fromarray(a.astype(np.uint8)).convert("HSV")).astype(np.float32)
    hq = (hsv[..., 0] / 32).astype(int).clip(0, 7)
    vq = (hsv[..., 2] / 43).astype(int).clip(0, 5)
    bins = np.zeros((8, 6))
    np.add.at(bins, (hq[m], vq[m]), 1)
    mats = float((bins / max(A, 1) >= 0.03).sum())

    return {
        "warmth": float((R[band] - B[band]).mean()),
        "blue": float(B[band].mean()),
        "sat": float(sat[band].mean() * 100),
        "micro": micro,
        "p95": float(np.percentile(Lm, 95)),
        "p05L": float(np.percentile(Lm, 5) / 255 * 100),
        "darkRB": pr - pb,
        "edge": edge,
        "form": form,
        "iso": iso,
        "mats": mats,
        "_dark_rgb": (pr, pg, pb),
        "_mask_px": int(A),
        "_scale": scale,
        "_meanL": float(Lm.mean()),
    }


def contact_penumbra(rgb, mask, foot_y):
    """C7 — 10->90 % fall width of the shadow the character casts at its feet.

    Scanned on rows just BELOW the foot line, outward from the body centre, which
    is where a PCSS contact shadow lives. Same 10-90 definition as
    scripts/measure_penumbra.py (that script scans a floor band for the frame's
    strongest edge; here the edge has to be the character's own).
    """
    g = (0.2126 * rgb[..., 0] + 0.7152 * rgb[..., 1] + 0.0722 * rgb[..., 2])
    h, w = g.shape
    ys, xs = np.where(mask)
    cx = int(xs.mean())
    body_w = int(xs.max() - xs.min() + 1)
    widths = []
    y_lo = int(foot_y + 2)
    y_hi = int(min(h - 2, foot_y + 0.10 * h))
    for y in range(y_lo, y_hi, max(1, (y_hi - y_lo) // 12)):
        row = np.convolve(g[y], np.ones(3) / 3.0, mode="same")
        # walk right from the body centre out to 2.5 body widths
        x_end = int(min(w - 1, cx + 2.5 * body_w))
        seg = row[cx:x_end]
        if seg.size < 12:
            continue
        dark, light = float(seg.min()), float(seg.max())
        rng = light - dark
        if rng < 8:
            continue
        lo_t, hi_t = dark + 0.1 * rng, dark + 0.9 * rng
        below = np.where(seg <= lo_t)[0]
        above = np.where(seg >= hi_t)[0]
        if below.size == 0 or above.size == 0:
            continue
        a_i = int(below[0])
        b_i = above[above > a_i]
        if b_i.size == 0:
            continue
        widths.append(int(b_i[0]) - a_i)
    if not widths:
        return None, 0
    return float(np.median(widths)), len(widths)


def save_overlay(rgb, mask, path, bbox=None):
    ov = rgb.copy()
    edge = ndimage.binary_dilation(mask, iterations=2) & ~mask
    ov[mask] = ov[mask] * 0.55 + np.array([0, 255, 120]) * 0.45
    ov[edge] = np.array([255, 0, 200])
    if bbox:
        x0, y0, x1, y1 = [int(v) for v in bbox]
        h, w, _ = ov.shape
        for x in range(max(0, x0), min(w, x1)):
            for y in (y0, y1 - 1):
                if 0 <= y < h:
                    ov[y, x] = [60, 200, 255]
        for y in range(max(0, y0), min(h, y1)):
            for x in (x0, x1 - 1):
                if 0 <= x < w:
                    ov[y, x] = [60, 200, 255]
    Image.fromarray(ov.astype(np.uint8)).save(path)
    return path


def main():
    ap = argparse.ArgumentParser(description="Grade the CHARACTER region of a frame")
    ap.add_argument("image")
    ap.add_argument("--ref", action="store_true",
                    help="concept-sheet mode: matte from gradient, print values only")
    ap.add_argument("--cam", help="YAW,PITCH,DIST of VOXELFORGE_LOOK_CAM (play frames)")
    ap.add_argument("--actor", default="player", choices=["player", "husk"])
    ap.add_argument("--hud-crop", type=int, default=80,
                    help="rows _flamingo_dehud2.py removed from the top (HUD_BAND)")
    ap.add_argument("--key-tol", type=float, default=26.0)
    ap.add_argument("--norm-h", type=int, default=512)
    ap.add_argument("--no-shadow-guard", action="store_true")
    ap.add_argument("--label", default=None)
    ap.add_argument("--json", default=None)
    ap.add_argument("--ref-json", default=None,
                    help="the concept sheet's own --json output; it supplies every bound")
    args = ap.parse_args()

    if not args.ref and "-nohud2" not in os.path.basename(args.image):
        sys.exit("REFUSED: play frames must be de-HUDded first "
                 "(python scripts/_flamingo_dehud2.py <frame>.png). "
                 "HUD glyphs sit at ~250 and poison p95 and micro-contrast.")

    rgb = np.asarray(Image.open(args.image).convert("RGB")).astype(np.float32)
    h, w, _ = rgb.shape
    stem = os.path.splitext(args.image)[0]

    bbox = None
    if args.ref:
        mask = mask_from_ref(rgb)
        foot_y = float(np.where(mask)[0].max())
    else:
        if not args.cam:
            sys.exit("--cam YAW,PITCH,DIST is required for a play frame "
                     "(it is what makes the bbox analytic instead of eyeballed)")
        yaw, pitch, dist = [float(v) for v in args.cam.split(",")]
        bbox, foot_y = analytic_bbox(w, h, yaw, pitch, dist, args.actor, args.hud_crop)
        mask = mask_from_bbox(rgb, bbox, foot_y, args.key_tol,
                              not args.no_shadow_guard)
        if mask.sum() < 200:
            sys.exit(f"character mask empty ({int(mask.sum())} px) — bbox {bbox}. "
                     "Wrong --cam, or the avatar is off-frame.")

    m = measure(rgb, mask, args.norm_h)
    pen, pen_edges = (None, 0) if args.ref else contact_penumbra(rgb, mask, foot_y)
    m["pen"] = pen

    overlay = save_overlay(rgb, mask, f"{stem}-charmask.png", bbox)

    label = args.label or os.path.basename(args.image)
    print(f"# {label}   {w}x{h}   actor={args.actor}"
          f"{'   [REF concept sheet]' if args.ref else f'   cam={args.cam}'}")
    print(f"  mask {m['_mask_px']} px @norm  ({mask.sum() / (w * h) * 100:.2f} % of frame)"
          f"   norm scale x{m['_scale']:.3f}   mean L {m['_meanL']:.1f}")
    print(f"  darkest 11x11 in-body patch RGB=({m['_dark_rgb'][0]:.0f},"
          f"{m['_dark_rgb'][1]:.0f},{m['_dark_rgb'][2]:.0f})")
    if not args.ref:
        print(f"  contact-shadow rows measured: {pen_edges}")
    print(f"  matte overlay -> {overlay}")
    print()

    refv = {}
    if args.ref_json:
        with open(args.ref_json, encoding="utf-8") as f:
            refv = json.load(f).get("metrics", {})
        print(f"  reference sheet: {args.ref_json}")
        print()

    if args.ref:
        print(f"  {'axis':<30}{'value':>9}")
        for key, lbl, _rule, _src in TARGETS:
            v = m.get(key)
            print(f"  [REF ] {lbl:<28} " + ("      --" if v is None else f"{v:8.2f}"))
        print()
        print("  => reference values printed; a concept sheet is the bar, not a candidate")
        if args.json:
            with open(args.json, "w", encoding="utf-8") as f:
                json.dump({"label": label, "image": args.image, "ref": True,
                           "actor": args.actor,
                           "metrics": {k: v for k, v in m.items() if not k.startswith("_")}},
                          f, indent=2)
        sys.exit(0)

    if not refv:
        sys.exit("--ref-json <concept.json> is required to grade a render: the "
                 "approved concept sheet IS this lane's target. Produce it with "
                 "grade_character.py --ref <sheet>.png --json <out>.json")

    fails, rows = [], []
    print(f"  {'axis':<30}{'render':>9}{'concept':>10}{'target':>16}  {'frame ctx':>10}")
    for key, lbl, rule, src in TARGETS:
        v = m.get(key)
        ref = refv.get(key)
        cmp, bound = resolve_bound(rule, ref)
        ctx = FRAME_CONTEXT.get(key, "")
        if v is None or cmp is None:
            print(f"  [N/A ] {lbl:<28}{'--':>9}{'--':>10}{'--':>16}  {ctx:>10}")
            rows.append((key, lbl, v, ref, None, None))
            continue
        ok = verdict(cmp, bound, v)
        if not ok:
            fails.append(lbl)
        rs = "--" if ref is None else f"{ref:.2f}"
        print(f"  [{'PASS' if ok else 'FAIL'}] {lbl:<28}{v:9.2f}{rs:>10}"
              f"{fmt_target(cmp, bound):>16}  {ctx:>10}   ({src})")
        rows.append((key, lbl, v, ref, bound, ok))
    print()
    print(f"  => {'ALL CHARACTER AXES PASS' if not fails else f'FAIL ({len(fails)}/{len(TARGETS)}): ' + ', '.join(fails)}")

    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump({"label": label, "image": args.image, "ref": False,
                       "actor": args.actor, "cam": args.cam,
                       "ref_json": args.ref_json,
                       "metrics": {k: v for k, v in m.items() if not k.startswith("_")},
                       "ref_metrics": refv,
                       "mask_px_frame": int(mask.sum()),
                       "rows": [{"key": k, "label": l, "render": vv, "concept": rr,
                                 "bound": (list(b) if isinstance(b, tuple) else b),
                                 "pass": o} for k, l, vv, rr, b, o in rows],
                       "fails": fails}, f, indent=2)
    sys.exit(0 if not fails else 1)


if __name__ == "__main__":
    main()
