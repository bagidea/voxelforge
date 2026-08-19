# -*- coding: utf-8 -*-
"""Composition audit + blockout preview for a Voxelforge map, from an exact camera.

Pure data lane: NO cargo, NO engine. This reads `maps/*.json` (the real schema,
`client/src/mapfile.rs:19-46`) and the real block albedo table
(`sim/src/block.rs::base_color`), projects the world through the SAME camera math
the engine uses, and answers the questions a set-dresser has to answer before a
build lane is free to shoot:

  * where does each landmark land in frame (rule of thirds)?
  * is there a real foreground / midground / background split, or one flat wall?
  * is there exactly ONE brightest point, or a row of equal lamps?
  * what hue range does the *palette in frame* span (Kevin's art-gap axes)?

Camera model, matched to the engine:
  `Transform::from_xyz(ex,ey,ez).looking_at((tx,ty,tz), Vec3::Y)` with Bevy's
  default `PerspectiveProjection` (vertical fov = PI/4 = 45 deg), rendered into
  the 1280x720 window `client/src/main.rs:509` opens. Bevy is right-handed with
  +Y up and the camera looking down its local -Z, so:
      forward = normalize(target - eye)
      right   = normalize(cross(forward, +Y))
      up      = cross(right, forward)
  and a point projects to NDC (sx, sy) = (dot(v,right)/d/tan(hfov/2),
  dot(v,up)/d/tan(vfov/2)) with d = dot(v, forward).

The preview PNG is a **blockout**, not a render: flat per-face lambert off the
unshaded `base_color` table, a dusk sky ramp, and distance haze. It is honest
about geometry, framing, silhouette and occlusion — and says nothing about the
engine's grade, bloom, normal maps, SSAO or textures. Never quote a look-gate
number off it; use it to place things.

Usage:
    python scripts/_shiba_beach_compose.py maps/beach_dusk.json \
        --cam 18,10,6,29,4,30,45 --out docs/assets/look/beauty/beach-blockout.png
"""
from __future__ import annotations

import argparse
import json
import math
import os
import sys
from collections import Counter, defaultdict

from PIL import Image, ImageDraw

# --- the real albedo table, transcribed from sim/src/block.rs::base_color ----
# Kept as a literal on purpose: this script must fail loudly if a block name it
# does not know shows up, rather than paint it a guess.
BASE_COLOR = {
    "grass": (91, 140, 70),
    "dirt": (107, 85, 64),
    "stone": (143, 135, 118),
    "sand": (214, 202, 148),
    "wood": (156, 107, 58),
    "leaves": (58, 116, 54),
    "snow": (240, 236, 224),
    "red_sand": (200, 130, 70),
    "clay": (126, 150, 160),
    "gravel": (110, 100, 94),
    "cobblestone": (140, 138, 120),
    "obsidian": (26, 22, 32),
    "brick": (150, 90, 60),
    "moss": (75, 110, 55),
    "limestone": (222, 204, 168),
    "lamp": (255, 196, 118),
    "glass": (198, 222, 226),
}
# Blocks that draw in the transparent pass (client/src/voxel.rs::block_surface).
TRANSPARENT = {"glass", "obsidian"}
# Emissive blocks — `lamp` only (voxel.rs: LinearRgba::rgb(9.0, 4.6, 1.5)).
EMISSIVE = {"lamp"}

FACES = {
    # name: (normal, 4 corner offsets ccw)
    "py": ((0, 1, 0), [(0, 1, 0), (1, 1, 0), (1, 1, 1), (0, 1, 1)]),
    "ny": ((0, -1, 0), [(0, 0, 0), (0, 0, 1), (1, 0, 1), (1, 0, 0)]),
    "px": ((1, 0, 0), [(1, 0, 0), (1, 0, 1), (1, 1, 1), (1, 1, 0)]),
    "nx": ((-1, 0, 0), [(0, 0, 0), (0, 1, 0), (0, 1, 1), (0, 0, 1)]),
    "pz": ((0, 0, 1), [(0, 0, 1), (0, 1, 1), (1, 1, 1), (1, 0, 1)]),
    "nz": ((0, 0, -1), [(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)]),
}


# ---------------------------------------------------------------------------
# camera
# ---------------------------------------------------------------------------
class Cam:
    def __init__(self, eye, target, fov_deg=45.0, w=1280, h=720):
        self.eye = eye
        self.w, self.h = w, h
        f = _sub(target, eye)
        self.fwd = _norm(f)
        self.right = _norm(_cross(self.fwd, (0.0, 1.0, 0.0)))
        self.up = _cross(self.right, self.fwd)
        self.tan_v = math.tan(math.radians(fov_deg) / 2.0)
        self.tan_h = self.tan_v * (w / h)

    def project(self, p):
        """World point -> (px, py, depth). depth <= 0 means behind the lens."""
        v = _sub(p, self.eye)
        d = _dot(v, self.fwd)
        if d <= 1e-4:
            return None, None, d
        sx = _dot(v, self.right) / d / self.tan_h
        sy = _dot(v, self.up) / d / self.tan_v
        return (sx + 1.0) * 0.5 * self.w, (1.0 - sy) * 0.5 * self.h, d

    def ndc(self, p):
        """World point -> (u, v) in [0,1] screen space, or None if behind."""
        px, py, d = self.project(p)
        if px is None:
            return None
        return (px / self.w, py / self.h, d)

    def clip_project(self, poly):
        """Project a 3-D polygon, clipping it against the near plane first.

        Without this a quad with even ONE vertex behind the lens is dropped
        whole — which silently deleted the far ocean slab (its far-side corners
        sit behind a camera that is looking along the water) and made every
        sweep report `water = 1 %`. Sutherland-Hodgman against d >= NEAR.
        """
        NEAR = 0.05
        n = len(poly)
        ds = [_dot(_sub(p, self.eye), self.fwd) for p in poly]
        kept = []
        for i in range(n):
            j = (i + 1) % n
            pi, pj, di, dj = poly[i], poly[j], ds[i], ds[j]
            if di >= NEAR:
                kept.append(pi)
            if (di >= NEAR) != (dj >= NEAR):
                t = (NEAR - di) / (dj - di)
                kept.append(tuple(pi[k] + t * (pj[k] - pi[k]) for k in range(3)))
        if len(kept) < 3:
            return None
        return [self.project(p)[:2] for p in kept]


def _sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def _dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def _cross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def _norm(a):
    m = math.sqrt(_dot(a, a)) or 1.0
    return (a[0] / m, a[1] / m, a[2] / m)


# ---------------------------------------------------------------------------
# blockout preview
# ---------------------------------------------------------------------------
def sun_dir(elev_deg, azim_deg):
    """Unit vector pointing FROM the scene TOWARD the sun."""
    e = math.radians(elev_deg)
    a = math.radians(azim_deg)
    return (math.cos(e) * math.sin(a), math.sin(e), math.cos(e) * math.cos(a))


SKY_TOP = (58, 82, 132)      # dusk zenith, cool
SKY_HORIZON = (247, 178, 120)  # golden-hour horizon


def sky_pixel(v):
    """v in [0,1] down the frame -> sky ramp colour."""
    t = min(max(v * 1.55, 0.0), 1.0)
    return tuple(int(SKY_TOP[i] + (SKY_HORIZON[i] - SKY_TOP[i]) * t) for i in range(3))


def render(map_path, cam, sun_elev, sun_azim, props=True, w=1280, h=720):
    with open(map_path, "r", encoding="utf-8") as fh:
        data = json.load(fh)
    occ = {}
    for b in data["blocks"]:
        occ[(b["x"], b["y"], b["z"])] = b["block"]
    unknown = sorted({n for n in occ.values() if n not in BASE_COLOR})
    if unknown:
        sys.exit(f"unknown block names in {map_path}: {unknown}")

    img = Image.new("RGB", (w, h))
    d = ImageDraw.Draw(img, "RGBA")
    for y in range(h):
        d.line([(0, y), (w, y)], fill=sky_pixel(y / h))

    sd = sun_dir(sun_elev, sun_azim)
    quads = []  # (depth, poly, rgba)

    def face_visible(pos, n):
        q = (pos[0] + n[0], pos[1] + n[1], pos[2] + n[2])
        nb = occ.get(q)
        if nb is None:
            return True
        # transparent neighbours do not hide a face (voxel.rs: `!hides(b)`),
        # but a run of the SAME transparent block draws no internal faces.
        return nb in TRANSPARENT and nb != occ.get(pos)

    for pos, name in occ.items():
        base = BASE_COLOR[name]
        for fname, (n, corners) in FACES.items():
            if not face_visible(pos, n):
                continue
            cx = pos[0] + 0.5 + n[0] * 0.5
            cy = pos[1] + 0.5 + n[1] * 0.5
            cz = pos[2] + 0.5 + n[2] * 0.5
            v = _sub((cx, cy, cz), cam.eye)
            if _dot(v, n) > 0:  # back-facing
                continue
            pts = cam.clip_project(
                [(pos[0] + c[0], pos[1] + c[1], pos[2] + c[2]) for c in corners])
            if pts is None:
                continue
            if max(p[0] for p in pts) < -80 or min(p[0] for p in pts) > w + 80:
                continue
            if max(p[1] for p in pts) < -80 or min(p[1] for p in pts) > h + 80:
                continue
            dist = math.sqrt(_dot(v, v))
            lam = max(_dot(n, sd), 0.0)
            if name in EMISSIVE:
                shade = 1.0
                col = (255, 214, 150)
            else:
                # sun key + sky fill; the fill is cooler on up-facing surfaces.
                amb = 0.34 + 0.16 * max(n[1], 0.0)
                shade = amb + 0.82 * lam
                col = base
            r, g, b = (min(255, int(c * shade)) for c in col)
            # distance haze toward the horizon colour — the depth cue a blockout
            # otherwise has no way to state.
            hz = min(dist / 110.0, 0.55)
            r = int(r * (1 - hz) + SKY_HORIZON[0] * hz)
            g = int(g * (1 - hz) + SKY_HORIZON[1] * hz)
            b = int(b * (1 - hz) + SKY_HORIZON[2] * hz)
            a = 165 if name in TRANSPARENT else 255
            quads.append((dist, pts, (r, g, b, a), name, dist))

    if props:
        quads.extend(prop_quads(cam, sd))

    quads.sort(key=lambda q: -q[0])
    depth_hits = []
    for _, pts, rgba, name, dist in quads:
        d.polygon(pts, fill=rgba)
        depth_hits.append((pts, dist, name))
    return img, occ, depth_hits, data


# --- the hand-built Rust props (client/src/beach_shot.rs), drawn so the audit
# --- sees what the rig will actually put in frame. Positions are read off that
# --- file, not invented here.
WATER_Y = 0.4
# `beach_shot.rs::spawn_water` as it stands today: Plane3d half-extents
# (32, 12) at (32, 0.4, 52) -> x 0..64, z 40..64. That is a 24-voxel-deep pond
# whose far edge is IN FRAME, which is why `--water shipped` shows sky under the
# horizon line. `--water ocean` models the one-constant fix (see the doc §6).
WATER_PRESETS = {
    "shipped": ((0.0, 64.0), (40.0, 64.0)),
    # Centred on the world and run out to a real horizon, so the sea surrounds
    # the island instead of ending inside the frame. The plane sits BELOW the
    # land's top face (0.4 vs 1.0), so the beach simply pokes through it — no
    # seam to line up, and no gap anywhere the shoreline wanders.
    "ocean": ((32.0 - 400.0, 32.0 + 400.0), (32.0 - 400.0, 32.0 + 400.0)),
}
WATER_X, WATER_Z = WATER_PRESETS["ocean"]
PROP_CAMPFIRE = (23.0, 1.0, 14.0)
PROP_BOAT = (32.5, 1.2, 50.0)
PROP_POTS = [(27.0, 1.0, 25.0), (30.5, 1.0, 25.0)]


def _water_tiles():
    """Near water tiled small (it has to sort against the sandbars), far water
    as one slab (it is behind everything and 160k tiles is not a preview)."""
    near_z0, near_z1 = max(WATER_Z[0], 30.0), min(WATER_Z[1], 80.0)
    step = 2.0
    z = near_z0
    while z < near_z1:
        x = max(WATER_X[0], -26.0)
        while x < min(WATER_X[1], 92.0):
            yield x, z, step
            x += step
        z += step
    # the rest of the sea, as slabs that cannot sort wrongly against the island
    for band in ((WATER_Z[0], near_z0), (near_z1, WATER_Z[1])):
        if band[1] - band[0] > 0.5:
            yield band[0], band[1], None


def prop_quads(cam, sd):
    out = []
    for x, z, step in _water_tiles():
        if step is None:
            z0b, z1b = x, z
            corners = [
                (WATER_X[0], WATER_Y, z0b),
                (WATER_X[1], WATER_Y, z0b),
                (WATER_X[1], WATER_Y, z1b),
                (WATER_X[0], WATER_Y, z1b),
            ]
            pts = cam.clip_project(corners)
            if pts is not None:
                v = _sub(((WATER_X[0] + WATER_X[1]) / 2, WATER_Y, (z0b + z1b) / 2), cam.eye)
                dist = math.sqrt(_dot(v, v))
                out.append((dist, pts, (34, 62, 78, 240), "water", dist))
            continue
        if True:
            corners = [
                (x, WATER_Y, z),
                (x + step, WATER_Y, z),
                (x + step, WATER_Y, z + step),
                (x, WATER_Y, z + step),
            ]
            pts = cam.clip_project(corners)
            if pts is not None:
                cxz = (x + step / 2, WATER_Y, z + step / 2)
                v = _sub(cxz, cam.eye)
                dist = math.sqrt(_dot(v, v))
                # base_color srgba(0.05,0.17,0.22), lifted toward the horizon
                # haze with distance. NO fake sun glint: the real one comes from
                # roughness 0.06 under the IBL, which a blockout cannot model,
                # and painting a guess where the money shot's highlight goes is
                # exactly the kind of picture that lies about itself.
                hz = min(dist / 130.0, 0.5)
                r = int(0.05 * 255 * 2.2 * (1 - hz) + SKY_HORIZON[0] * hz)
                g = int(0.17 * 255 * 2.2 * (1 - hz) + SKY_HORIZON[1] * hz)
                b = int(0.22 * 255 * 2.2 * (1 - hz) + SKY_HORIZON[2] * hz)
                out.append((dist, pts, (r, g, b, 235), "water", dist))
    # campfire, boat, pots as boxes
    for at, size, col, tag in (
        (PROP_CAMPFIRE, (1.8, 0.6, 1.8), (120, 110, 105), "campfire_ring"),
        ((PROP_CAMPFIRE[0], PROP_CAMPFIRE[1] + 0.45, PROP_CAMPFIRE[2]), (0.36, 0.5, 0.36), (255, 190, 110), "campfire_ember"),
        (PROP_BOAT, (1.3, 0.5, 3.2), (97, 61, 33), "boat"),
        (PROP_POTS[0], (0.6, 0.75, 0.6), (158, 82, 51), "pot"),
        (PROP_POTS[1], (0.6, 0.75, 0.6), (158, 82, 51), "pot"),
    ):
        out.extend(box_quads(cam, at, size, col, tag))
    return out


def box_quads(cam, centre, size, col, tag):
    hx, hy, hz = size[0] / 2, size[1] / 2, size[2] / 2
    x0, y0, z0 = centre[0] - hx, centre[1] - hy, centre[2] - hz
    out = []
    for fname, (n, corners) in FACES.items():
        cx = x0 + hx + n[0] * hx
        cy = y0 + hy + n[1] * hy
        cz = z0 + hz + n[2] * hz
        v = _sub((cx, cy, cz), cam.eye)
        if _dot(v, n) > 0:
            continue
        pts = cam.clip_project(
            [(x0 + c[0] * size[0], y0 + c[1] * size[1], z0 + c[2] * size[2])
             for c in corners])
        if pts is None:
            continue
        dist = math.sqrt(_dot(v, v))
        sh = 0.62 + 0.38 * max(n[1], 0.0)
        c2 = tuple(min(255, int(c * sh)) for c in col)
        out.append((dist, pts, c2 + (255,), tag, dist))
    return out


# ---------------------------------------------------------------------------
# composition metrics
# ---------------------------------------------------------------------------
def rgb_to_hsv_deg(r, g, b):
    mx, mn = max(r, g, b), min(r, g, b)
    v = mx / 255.0
    s = 0.0 if mx == 0 else (mx - mn) / mx
    if mx == mn:
        h = 0.0
    elif mx == r:
        h = 60 * (((g - b) / (mx - mn)) % 6)
    elif mx == g:
        h = 60 * (((b - r) / (mx - mn)) + 2)
    else:
        h = 60 * (((r - g) / (mx - mn)) + 4)
    return h % 360, s, v


def hue_spread_90(hues, weights):
    """Minimal circular arc holding 90% of hue mass — Kevin's definition."""
    if not hues:
        return 0.0, 0
    bins = [0.0] * 360
    for hdeg, w in zip(hues, weights):
        bins[int(hdeg) % 360] += w
    total = sum(bins)
    if total <= 0:
        return 0.0, 0
    target = total * 0.90
    best = 360
    acc = 0.0
    lo = 0
    for hi in range(720):
        acc += bins[hi % 360]
        while acc >= target:
            best = min(best, hi - lo + 1)
            acc -= bins[lo % 360]
            lo += 1
        if lo > 360:
            break
    bins10 = [0.0] * 36
    for i, v in enumerate(bins):
        bins10[i // 10] += v
    occupied = sum(1 for v in bins10 if v >= total * 0.005)
    return float(best), occupied


def audit(img, occ, cam, landmarks, out_json=None):
    w, h = img.size
    px = img.load()
    hues, weights, cool, warm, neutral, sat_px = [], [], 0, 0, 0, 0
    lum_hist = Counter()
    for y in range(0, h, 2):
        for x in range(0, w, 2):
            r, g, b = px[x, y]
            hdeg, s, v = rgb_to_hsv_deg(r, g, b)
            lum = 0.299 * r + 0.587 * g + 0.114 * b
            lum_hist["shadow" if lum < 85 else ("mid" if lum <= 170 else "high")] += 1
            if s >= 0.08:
                sat_px += 1
                hues.append(hdeg)
                weights.append(1.0)
                if hdeg < 70 or hdeg >= 340:
                    warm += 1
                elif 170 <= hdeg < 270:
                    cool += 1
                else:
                    neutral += 1
    spread, occupied = hue_spread_90(hues, weights)
    tot = sum(lum_hist.values())
    rep = {
        "hue90_deg": round(spread, 1),
        "occupied_bins_of_36": occupied,
        "cool_share_pct": round(100.0 * cool / max(sat_px, 1), 1),
        "warm_share_pct": round(100.0 * warm / max(sat_px, 1), 1),
        "neutral_share_pct": round(100.0 * neutral / max(sat_px, 1), 1),
        "shadow_pct": round(100.0 * lum_hist["shadow"] / tot, 1),
        "midtone_pct": round(100.0 * lum_hist["mid"] / tot, 1),
        "highlight_pct": round(100.0 * lum_hist["high"] / tot, 1),
        "landmarks": landmarks,
    }
    if out_json:
        with open(out_json, "w", encoding="utf-8") as fh:
            json.dump(rep, fh, indent=2)
    return rep


def thirds_note(u, v):
    """How close a normalised point is to a rule-of-thirds line/intersection."""
    dx = min(abs(u - 1 / 3), abs(u - 2 / 3))
    dy = min(abs(v - 1 / 3), abs(v - 2 / 3))
    tags = []
    if dx < 0.045:
        tags.append("on V-third")
    if dy < 0.045:
        tags.append("on H-third")
    if dx < 0.06 and dy < 0.06:
        tags = ["ON THIRDS INTERSECTION"]
    if not (0.0 <= u <= 1.0 and 0.0 <= v <= 1.0):
        tags.append("OFF-FRAME")
    return ", ".join(tags) or "-"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("map", nargs="?", default="maps/beach_dusk.json")
    ap.add_argument("--cam", default="18,10,6,29,4,30,45",
                    help="ex,ey,ez,tx,ty,tz,fov_deg (the VOXELFORGE_CAM layout)")
    ap.add_argument("--sun", default="9,206", help="elevation_deg,azimuth_deg")
    ap.add_argument("--out", default="")
    ap.add_argument("--json", default="")
    ap.add_argument("--no-props", action="store_true")
    ap.add_argument("--scale", type=float, default=1.0)
    ap.add_argument("--water", choices=sorted(WATER_PRESETS), default="ocean",
                    help="'shipped' = beach_shot.rs's 64x24 pond as it stands; "
                         "'ocean' = the extents this scene is composed for")
    args = ap.parse_args()
    global WATER_X, WATER_Z
    WATER_X, WATER_Z = WATER_PRESETS[args.water]

    c = [float(t) for t in args.cam.split(",")]
    if len(c) != 7:
        sys.exit("--cam needs 7 numbers: ex,ey,ez,tx,ty,tz,fov")
    se, sa = (float(t) for t in args.sun.split(","))
    w, h = int(1280 * args.scale), int(720 * args.scale)
    cam = Cam((c[0], c[1], c[2]), (c[3], c[4], c[5]), c[6], w, h)

    img, occ, hits, data = render(args.map, cam, se, sa,
                                  props=not args.no_props, w=w, h=h)

    # ---- landmark projection (rule of thirds) ------------------------------
    lm_defs = {
        "campfire (Rust prop)": PROP_CAMPFIRE,
        "cabin door": (28.5, 3.0, 24.0),
        "cabin roof ridge": (29.0, 8.0, 20.0),
        "dock far end": (30.0, 2.0, 47.0),
        "boat (Rust prop)": PROP_BOAT,
        "island": (54.0, 3.0, 57.0),
        "horizon @ x=32": (32.0, WATER_Y, 400.0),
    }
    landmarks = {}
    for k, p in lm_defs.items():
        r = cam.ndc(p)
        if r is None:
            landmarks[k] = {"u": None, "v": None, "note": "BEHIND CAMERA"}
        else:
            u, v, dist = r
            landmarks[k] = {"u": round(u, 3), "v": round(v, 3),
                            "dist": round(dist, 1), "note": thirds_note(u, v)}

    # ---- depth bands (foreground / midground / background) -----------------
    # measured off the drawn quads: for each sampled pixel, the nearest quad that
    # covers it. Cheap version: rasterise a depth buffer at 1/4 res.
    dw, dh = w // 4, h // 4
    zbuf = [[None] * dw for _ in range(dh)]
    for pts, dist, name in hits:
        xs = [p[0] / 4 for p in pts]
        ys = [p[1] / 4 for p in pts]
        x0, x1 = max(0, int(min(xs))), min(dw - 1, int(max(xs)) + 1)
        y0, y1 = max(0, int(min(ys))), min(dh - 1, int(max(ys)) + 1)
        for yy in range(y0, y1 + 1):
            for xx in range(x0, x1 + 1):
                if _pt_in_poly(xx + 0.5, yy + 0.5, list(zip(xs, ys))):
                    cur = zbuf[yy][xx]
                    if cur is None or dist < cur[0]:
                        zbuf[yy][xx] = (dist, name)
    bands = Counter()
    subject = Counter()
    for row in zbuf:
        for cell in row:
            if cell is None:
                bands["sky"] += 1
                subject["sky"] += 1
                continue
            subject["water" if cell[1] == "water" else "land"] += 1
            if cell[0] < 16:
                bands["foreground(<16)"] += 1
            elif cell[0] < 45:
                bands["midground(16-45)"] += 1
            else:
                bands["background(>45)"] += 1
    tot = dw * dh
    band_pct = {k: round(100.0 * v / tot, 1) for k, v in bands.items()}
    subj_pct = {k: round(100.0 * v / tot, 1) for k, v in subject.items()}

    # ---- brightest-point ranking (analytic, not off the preview) ------------
    # apparent brightness ~ emissive_radiance * projected_area / dist^2.
    lights = []
    for (x, y, z), name in occ.items():
        if name != "lamp":
            continue
        p = (x + 0.5, y + 0.5, z + 0.5)
        v = _sub(p, cam.eye)
        dist = math.sqrt(_dot(v, v))
        # 9.0/4.6/1.5 linear emissive over one 1x1 block face
        lights.append(("lamp block @ %d,%d,%d" % (x, y, z), 9.0 * 1.0 / dist ** 2, dist))
    v = _sub(PROP_CAMPFIRE, cam.eye)
    fdist = math.sqrt(_dot(v, v))
    # ember cuboid 0.36 x 0.5 face, emissive 6.0, PLUS a 260k point light
    lights.append(("campfire ember (Rust)", 6.0 * 0.18 / fdist ** 2 + 260000.0 / (4 * math.pi * 1000.0) / fdist ** 2, fdist))
    lights.sort(key=lambda t: -t[1])

    print(f"MAP {args.map}  blocks={len(data['blocks'])}  size={data['size']}")
    print(f"CAM eye=({c[0]},{c[1]},{c[2]}) -> aim=({c[3]},{c[4]},{c[5]}) fov={c[6]} @ {w}x{h}")
    print(f"SUN elev={se} azim={sa}")
    print("\n-- landmarks (u,v normalised; thirds at .333/.667) --")
    for k, m in landmarks.items():
        if m["u"] is None:
            print(f"  {k:26s} {m['note']}")
        else:
            print(f"  {k:26s} u={m['u']:.3f} v={m['v']:.3f} d={m['dist']:6.1f}  {m['note']}")
    print("\n-- depth bands (share of frame) --")
    for k in ("foreground(<16)", "midground(16-45)", "background(>45)", "sky"):
        print(f"  {k:20s} {band_pct.get(k, 0.0):5.1f}%")
    print("-- subject cover --   " + "  ".join(
        f"{k}={subj_pct.get(k, 0.0):.1f}%" for k in ("land", "water", "sky")))
    print("\n-- brightest points (apparent, analytic) --")
    for name, b, dist in lights[:6]:
        print(f"  {name:34s} rel={b:9.4f}  d={dist:5.1f}")
    if len(lights) >= 2 and lights[1][1] > 0:
        print(f"  dominance (1st / 2nd) = {lights[0][1] / lights[1][1]:.2f}x")

    rep = audit(img, occ, cam, landmarks, args.json or None)
    print("\n-- palette proxy (blockout pixels; NOT an engine look measurement) --")
    for k in ("hue90_deg", "occupied_bins_of_36", "cool_share_pct", "warm_share_pct",
              "neutral_share_pct", "shadow_pct", "midtone_pct", "highlight_pct"):
        print(f"  {k:22s} {rep[k]}")

    if args.out:
        os.makedirs(os.path.dirname(args.out) or ".", exist_ok=True)
        # thirds guides, drawn on a copy so the clean plate stays clean
        guide = img.copy()
        gd = ImageDraw.Draw(guide, "RGBA")
        for f in (1 / 3, 2 / 3):
            gd.line([(int(w * f), 0), (int(w * f), h)], fill=(255, 255, 255, 70), width=1)
            gd.line([(0, int(h * f)), (w, int(h * f))], fill=(255, 255, 255, 70), width=1)
        img.save(args.out)
        gpath = args.out.replace(".png", "-thirds.png")
        guide.save(gpath)
        print(f"\nwrote {args.out}\nwrote {gpath}")


def _pt_in_poly(x, y, poly):
    inside = False
    n = len(poly)
    j = n - 1
    for i in range(n):
        xi, yi = poly[i]
        xj, yj = poly[j]
        if (yi > y) != (yj > y):
            xint = (xj - xi) * (y - yi) / ((yj - yi) or 1e-9) + xi
            if x < xint:
                inside = not inside
        j = i
    return inside


if __name__ == "__main__":
    main()
