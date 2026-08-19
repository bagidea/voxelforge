#!/usr/bin/env python3
"""Rose — the river_sunset map pair (2026-08-18).

Builds the map the water pass is proven on: a sunset valley after the CEO
reference (docs/refs/ceo_ref_sunset_valley.jpg — valley reads as: warm sky
glow from the back, hills framing left+back with a snow cap, a meandering
river widening toward the viewer, sandy shores where the water meets land).

Emits TWO maps from ONE terrain grid so the evidence pair is a single
variable:

  maps/river_sunset.json      — the valley, with the river in it
  maps/river_sunset_dry.json  — byte-for-byte the same valley, water removed

Same generator, same seed, same camera. The only difference a frame can show
is the water.

Geometry contract with the engine (client/src/voxel.rs, sim/src/block.rs):
  * water top block sits at y = WATER_TOP (6), the mesher draws its surface
    at 6 + 1 - 2/16 = 6.875;
  * the channel bed is carved to column height 4 (top face at 4) with sand,
    so ~2.9 blocks of see-through depth over a bright bed;
  * banks blend up to column height 8 (top face 8 → 1.125 above the surface),
    so the shoreline reads without being a cliff.

Deterministic: value-noise from a fixed-seed integer hash, no `random`.
"""

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

W = 64            # world width  (2 chunks)
D = 64            # world depth  (2 chunks)
WATER_TOP = 6     # top water block y; surface renders at WATER_TOP+14/16
BED_H = 4         # channel bed column height (top face at 4)

# ---- deterministic value noise -------------------------------------------


def h2(x, z, salt):
    """Integer hash -> [0, 1). Fixed seed, no RNG state."""
    h = (x * 374761393 + z * 668265263 + salt * 2654435761) & 0xFFFFFFFF
    h ^= h >> 13
    h = (h * 1274126177) & 0xFFFFFFFF
    h ^= h >> 16
    return h / 4294967296.0


def vnoise(x, z, salt, cell=8):
    """Smooth value noise on a cell grid, bilinear + smoothstep."""
    gx, gz = x / cell, z / cell
    x0, z0 = int(gx), int(gz)
    fx, fz = gx - x0, gz - z0
    fx = fx * fx * (3 - 2 * fx)
    fz = fz * fz * (3 - 2 * fz)
    a = h2(x0, z0, salt)
    b = h2(x0 + 1, z0, salt)
    c = h2(x0, z0 + 1, salt)
    d = h2(x0 + 1, z0 + 1, salt)
    return a + (b - a) * fx + (c - a) * fz + (a - b - c + d) * fx * fz


def smooth(t):
    return t * t * (3 - 2 * t)


# ---- the river ------------------------------------------------------------


def river_cx(z):
    """Meander: back entry right of centre, mid-course swing east, front
    swing west — an S the camera at (36,16,60) looking north can follow."""
    return 33.0 + 7.0 * math.sin(z * 0.095) + 2.5 * math.sin(z * 0.33 + 2.0)


def river_halfwidth(z):
    """Half-width in blocks: ~2.6 at the back, ~6 at the viewer, plus a
    shallow flare where the front opens out (the reference's widening)."""
    base = 2.6 + 3.4 * (z / (D - 1))
    if z > 48:
        base += 1.6 * smooth((z - 48) / (D - 1 - 48))
    return base


# ---- terrain --------------------------------------------------------------


def terrain_height(x, z):
    """Column height (top block sits at h-1, its face at h)."""
    # valley floor, sloping gently toward the front-right opening
    h = 8.0
    h += 1.6 * (vnoise(x, z, 11, 12) - 0.5) * 2.0
    # back wall (north): rises over the first ~14 rows
    if z < 14:
        h += 10.0 * smooth((14 - z) / 14.0) * (0.75 + 0.5 * vnoise(x, z, 21, 6))
    # left ridge (west)
    if x < 13:
        h += 6.5 * smooth((13 - x) / 13.0) * (0.7 + 0.6 * vnoise(x, z, 31, 6))
    # a shoulder on the right, low, so the river has an east bank
    if x > 50:
        h += 2.2 * smooth((x - 50) / 13.0)
    # rolling detail
    h += 1.4 * vnoise(x, z, 41, 7)
    return h


def column_height(x, z):
    """terrainHeight with the channel carved in and banks blended."""
    h = terrain_height(x, z)
    cx = river_cx(z)
    hw = river_halfwidth(z) + 0.9 * (vnoise(x, z, 51, 5) - 0.5) * 2.0
    dist = abs(x - cx)
    if dist < hw:
        target = BED_H + 0.8 * vnoise(x, z, 61, 4)   # rippled bed, 4..4.8
    elif dist < hw + 3.0:
        t = (dist - hw) / 3.0
        target = BED_H + 4.0 * smooth(t)             # 4 -> 8 bank blend
    else:
        return round(h)
    return round(min(h, target))


# ---- materials ------------------------------------------------------------


def surface_block(x, z, h, dist):
    """Top block of a dry column."""
    if h >= 17 and z < 14:
        return "snow"                      # the reference's snow-capped back wall
    if h >= 13 and vnoise(x, z, 71, 5) > 0.62:
        return "stone"                     # outcrops on the hills
    if dist < river_halfwidth(z) + 4.5:
        if vnoise(x, z, 81, 5) > 0.55:
            return "gravel"                # shingle near the shore
        if vnoise(x, z, 91, 4) > 0.72:
            return "moss"                  # damp ground by the water
    return "grass"


def bed_block(y, h):
    """Blocks of an underwater column."""
    if y >= h - 1:
        return "sand"                      # the bright bed you see through
    if y >= h - 3:
        return "dirt"
    return "stone"


# ---- props ----------------------------------------------------------------

TREES = []  # filled after the grid exists, from deterministic candidates


def add_tree(blocks, x, z, h, tall):
    """Trunk + leaf canopy, staying inside the world bounds."""
    top = min(h + (5 if tall else 4), 28)
    for y in range(h, top):
        blocks.setdefault((x, y, z), "wood")
    r = 2
    for dy in (-1, 0, 1):
        for dx in range(-r, r + 1):
            for dz in range(-r, r + 1):
                if dx * dx + dz * dz + dy * dy * 2 > r * r + 1:
                    continue
                p = (x + dx, top - 1 + dy, z + dz)
                if blocks.get(p) is None and 0 <= p[0] < W and 0 <= p[2] < D:
                    blocks[p] = "leaves"


def add_dock(blocks, water_top):
    """A small wooden dock running from the east bank OUT over the water at
    the front bend, with two lanterns — the reference's human element and a
    scale marker that proves the surface sits below the block grid."""
    z0 = 50
    deck_y = water_top + 2                 # deck at y=8, face 9, ~2.1 over water
    # first column past the channel's east edge on this row
    cx = river_cx(z0)
    x_bank = int(cx + river_halfwidth(z0)) + 1
    # deck runs WEST (x descending) from the bank out over the river
    xs = list(range(x_bank - 5, x_bank + 1))
    for x in xs:
        for z in (z0, z0 + 1):
            blocks[(x, deck_y, z)] = "wood"
    for (px, pz) in [(xs[0], z0), (xs[0], z0 + 1), (xs[-1], z0), (xs[-1], z0 + 1)]:
        for y in range(3, deck_y):
            blocks[(px, y, pz)] = "wood"   # posts stand in the river bed
    blocks[(xs[0], deck_y + 1, z0)] = "lamp"          # lantern over the water end
    blocks[(xs[-1], deck_y + 1, z0 + 1)] = "lamp"     # lantern on the bank end


# ---- build ----------------------------------------------------------------


def build(with_water: bool):
    blocks = {}
    heights = {}
    for z in range(D):
        for x in range(W):
            h = column_height(x, z)
            heights[(x, z)] = h
            cx = river_cx(z)
            dist = abs(x - cx)
            if h <= WATER_TOP:
                # channel column: bed down to 0, water above it
                for y in range(h):
                    blocks[(x, y, z)] = bed_block(y, h)
                if with_water:
                    for y in range(h, WATER_TOP + 1):
                        blocks[(x, y, z)] = "water"
            else:
                for y in range(h):
                    if y == h - 1:
                        blocks[(x, y, z)] = surface_block(x, z, h, dist)
                    elif y >= h - 3:
                        blocks[(x, y, z)] = "dirt"
                    else:
                        blocks[(x, y, z)] = "stone"

    # trees: deterministic candidates on dry, gentle ground
    for z in range(4, D - 2, 5):
        for x in range(4, W - 2, 7):
            if h2(x, z, 7717) < 0.45:
                continue
            h = heights[(x, z)]
            cx = river_cx(z)
            if 8 <= h <= 14 and abs(x - cx) > river_halfwidth(z) + 3:
                add_tree(blocks, x, z, h, tall=(z < 16 or x < 16))

    # scattered rocks on the hills
    for z in range(2, D, 6):
        for x in range(2, W, 9):
            h = heights[(x, z)]
            if h >= 11 and h2(x, z, 991) > 0.72 and (x, h - 1, z) in blocks:
                blocks[(x, h, z)] = "cobblestone"
                if h2(x, z, 993) > 0.8:
                    blocks[(x, h + 1, z)] = "cobblestone"

    add_dock(blocks, WATER_TOP)

    # shell filter: drop blocks whose 6 neighbours are all opaque — the mesher
    # would cull every face of them anyway. Water/glass count as transparent,
    # so beds under rivers and ground under panes survive by construction.
    opaque = lambda b: b not in ("air", "water", "glass")

    def all_opaque(x, y, z):
        for dx, dy, dz in ((1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)):
            if not opaque(blocks.get((x + dx, y + dy, z + dz), "air")):
                return False
        return True

    slim = {p: b for p, b in blocks.items() if not all_opaque(*p)}

    out = {
        "version": 1,
        "name": "river-sunset" if with_water else "river-sunset-dry",
        "size": {"chunks_x": 2, "chunks_z": 2},
        "blocks": [
            {"x": p[0], "y": p[1], "z": p[2], "block": b} for p, b in sorted(slim.items())
        ],
    }
    return out, slim


def validate(m, slim):
    errs = []
    n_water = sum(1 for b in m["blocks"] if b["block"] == "water")
    for b in m["blocks"]:
        if not (0 <= b["x"] < W and 0 <= b["z"] < D and 0 <= b["y"] < 32):
            errs.append(f"out of bounds: {b}")
    # every z row must carry water (channel continuity, no dammed reaches)
    rows_with_water = {b["z"] for b in m["blocks"] if b["block"] == "water"}
    missing = [z for z in range(D) if z not in rows_with_water]
    if m["name"] == "river-sunset":
        if missing:
            errs.append(f"river is dammed at rows {missing}")
        if n_water < 400:
            errs.append(f"only {n_water} water blocks — that is a puddle, not a river")
    return errs, n_water


def main():
    for with_water, fname in ((True, "river_sunset.json"), (False, "river_sunset_dry.json")):
        m, slim = build(with_water)
        errs, n_water = validate(m, slim)
        path = ROOT / "maps" / fname
        path.write_text(json.dumps(m), encoding="utf-8")
        kinds = {}
        for b in m["blocks"]:
            kinds[b["block"]] = kinds.get(b["block"], 0) + 1
        print(f"{fname}: {len(m['blocks'])} blocks ({n_water} water) -> {path}")
        print("   ", dict(sorted(kinds.items(), key=lambda kv: -kv[1])))
        for e in errs:
            print("   ERROR:", e)
        if errs:
            sys.exit(1)


if __name__ == "__main__":
    main()
