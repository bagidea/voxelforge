#!/usr/bin/env python3
"""_pixel_world_project.py - which map columns does the graded camera actually see?

Lane: Pixel / WORLD. Reads maps + a plate, writes an overlay PNG. Touches no .rs.

WHY
---
The graded plate is a CLOSE-UP. Placing water or lanterns "in the scene" means
nothing if they land outside the frustum - the axis will not move and the next
round reads as "we added water and it did nothing", which is a false negative
that costs a whole build cycle to unpick.

So: project every surface column through the same camera the plate was shot
with, and only ever place content in the set this prints.

CALIBRATION
-----------
The projection is not trusted on assertion. `--overlay` draws the projected
footprint of a KNOWN landmark (the brick building, x23-35 z15-25, y5-8) onto
the real plate. If the drawn box does not sit on the bricks in the image, the
projection is wrong and every number below it is wrong too.

Camera comes from scripts/_poppy_matmaps_ab_final.ps1:
  VOXELFORGE_CINE = "41,15,37, 41,15,37, 29,3,20, 1"   eye -> aim
  main.rs:1085 spawns Camera3d::default() with no Projection override, so Bevy's
  PerspectiveProjection default applies: fov = PI/4 = 45 deg, VERTICAL.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

import numpy as np

EYE = np.array([41.0, 15.0, 37.0])
AIM = np.array([29.0, 3.0, 20.0])
VFOV_DEG = 45.0
W_PX, H_PX = 1280, 720


def view_matrix(eye, aim, up=np.array([0.0, 1.0, 0.0])):
    """Right-handed look-at, matching Bevy's -Z forward convention."""
    f = aim - eye
    f = f / np.linalg.norm(f)
    r = np.cross(f, up)
    r = r / np.linalg.norm(r)
    u = np.cross(r, f)
    return np.stack([r, u, -f])  # rows: right, up, backward


def project(pts, eye=EYE, aim=AIM, vfov_deg=VFOV_DEG, w=W_PX, h=H_PX):
    """World points -> (px, py, depth). depth>0 means in front of the camera."""
    M = view_matrix(eye, aim)
    cam = (pts - eye) @ M.T          # x right, y up, z backward
    depth = -cam[:, 2]                # positive in front
    tan_v = math.tan(math.radians(vfov_deg) * 0.5)
    aspect = w / h
    with np.errstate(divide="ignore", invalid="ignore"):
        ndc_x = cam[:, 0] / (depth * tan_v * aspect)
        ndc_y = cam[:, 1] / (depth * tan_v)
    px = (ndc_x * 0.5 + 0.5) * w
    py = (1.0 - (ndc_y * 0.5 + 0.5)) * h
    return px, py, depth


def surface_columns(map_path):
    """(x,z) -> (top_y, block) for every column that has any block."""
    m = json.loads(Path(map_path).read_text())
    top = {}
    for b in m["blocks"]:
        k = (b["x"], b["z"])
        if k not in top or b["y"] > top[k][0]:
            top[k] = (b["y"], b["block"])
    return m, top


def visible_columns(top, margin=0.0):
    """Columns whose surface centre falls inside the frame."""
    keys = list(top.keys())
    pts = np.array([[x + 0.5, top[(x, z)][0] + 1.0, z + 0.5] for x, z in keys])
    px, py, d = project(pts)
    ok = (d > 0.1) & (px >= -margin) & (px < W_PX + margin) \
        & (py >= -margin) & (py < H_PX + margin)
    return {keys[i]: (px[i], py[i], d[i]) for i in range(len(keys)) if ok[i]}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--map", default="maps/beach_dusk.json")
    ap.add_argument("--plate", default="_pixel_world_ctl.png")
    ap.add_argument("--overlay", default=None, help="write a calibration overlay PNG")
    a = ap.parse_args()

    m, top = surface_columns(a.map)
    vis = visible_columns(top)
    xs = [k[0] for k in vis]
    zs = [k[1] for k in vis]
    print(f"map            : {a.map}  blocks={len(m['blocks'])}  columns={len(top)}")
    print(f"visible columns: {len(vis)} of {len(top)}")
    if vis:
        print(f"  x range {min(xs)}..{max(xs)}   z range {min(zs)}..{max(zs)}")
        ds = [v[2] for v in vis.values()]
        print(f"  depth  {min(ds):.1f}..{max(ds):.1f}")

    # how big is one block on screen, at a few depths - decides how many
    # lanterns can even be resolved as separate emissive blobs
    print("\napparent size of a 1x1 block face:")
    for dd in (5, 10, 20, 30, 40):
        s = (1.0 / dd) / (2 * math.tan(math.radians(VFOV_DEG) / 2)) * H_PX
        print(f"  depth {dd:2d}  ->  {s:5.1f} px  ({s*s:7.0f} px^2)")

    if a.overlay:
        from PIL import Image, ImageDraw
        im = Image.open(a.plate).convert("RGB")
        dr = ImageDraw.Draw(im)
        # landmark: the brick building block, drawn as its 8 projected corners
        lo = np.array([23.0, 5.0, 15.0])
        hi = np.array([36.0, 9.0, 26.0])
        corners = np.array([[x, y, z] for x in (lo[0], hi[0])
                            for y in (lo[1], hi[1]) for z in (lo[2], hi[2])])
        px, py, d = project(corners)
        for i in range(len(corners)):
            if d[i] > 0:
                dr.ellipse([px[i] - 7, py[i] - 7, px[i] + 7, py[i] + 7],
                           outline=(0, 255, 255), width=4)
        # existing lamps
        lamps = np.array([[b["x"] + .5, b["y"] + .5, b["z"] + .5]
                          for b in m["blocks"] if b["block"] == "lamp"])
        if len(lamps):
            px, py, d = project(lamps)
            for i in range(len(lamps)):
                if d[i] > 0:
                    dr.ellipse([px[i] - 12, py[i] - 12, px[i] + 12, py[i] + 12],
                               outline=(255, 0, 255), width=5)
        im.save(a.overlay)
        print(f"\nwrote {a.overlay}  (cyan = brick-building corners, magenta = lamps)")


if __name__ == "__main__":
    main()
