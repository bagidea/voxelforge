# -*- coding: utf-8 -*-
"""Generate maps/beach_dusk.json — the outdoor "beach dusk" hero scene.

Pure data generation, no engine, no cargo. Deterministic (one fixed seed), so a
re-run is byte-identical and the JSON is never hand-edited:

    python scripts/gen_beach_dusk_map.py
    python scripts/_shiba_beach_compose.py        # audit + blockout preview

SCHEMA (read from client/src/mapfile.rs:19-46 and maps/FORMAT.md, not guessed):
a map is `version` + `name` + `size.chunks_x/z` + a flat `blocks[]` of
`{x,y,z,block}`. Air is implicit. Block names must be in
`mapfile::block_id_from_name` — this script validates that itself (BLOCK_NAMES
below is transcribed from that match arm) and refuses to write an unknown name,
because `main.rs::apply_map_blocks` would silently drop it into a `skipped`
counter that never names the block.

WHAT THE MAP OWNS AND WHAT IT DOES NOT
--------------------------------------
Map data owns geometry and material only. Sun angle, grade, sky and the
hand-built props (water plane, campfire flame, boat, pots) live in Rust —
`client/src/beach_shot.rs` (`VOXELFORGE_BEACHSHOT=1`) and `client/src/look.rs`.
Two hard constraints come back the other way and are honoured here:

  * `beach_shot.rs::spawn_water` puts a flat water quad at **y = 0.4** covering
    **x 0-64, z 40-64**. So land may never STOP before z = 40 — a column of air
    short of the plane's edge is a hole in the world, not a bay. Every land
    column therefore runs unbroken from z = 0 to `shore_z(x) >= 40`, and the bay
    is shaped by pushing the shoreline SEAWARD (40..47), never landward.
  * The Rust props stand at y = 1.0 (`PROP_CAMPFIRE (23,1,14)`,
    pots `(27,1,25)`/`(30.5,1,25)`), which means the ground surface under them
    must be exactly y = 1 — i.e. one ground block at y = 0. So the walkable
    ground is a single layer everywhere; every raised feature (dune, bluff,
    rock, building) is built ON TOP of it, and never under a prop.

COMPOSITION (see docs/hero-scene-beach-dusk.md §3 for the camera it is built for)
--------------------------------------------------------------------------------
Shot from the yard, over the cabin's landward shoulder, out to sea. With the
recommended camera the frame reads:

    left third   : the cabin (3/4 view, chimney + gable breaking the skyline),
                   the palm line, the dune that closes the left edge
    upper third  : the sea/sky horizon; the dock and its beacon lead into it
    lower right  : the fire pit — the single brightest point in frame
    bottom edge  : dark foreground mass (rock cluster, driftwood, shrubs) that
                   the eye enters through

Depth is built as three separated bands, not one wall: FOREGROUND z 0-12
(rocks/driftwood/shrubs/dune), MIDGROUND z 12-40 (cabin, fire, yard, beach,
dock), BACKGROUND z 40-64 (sea, sandbar, sea stacks, island).

ONE BRIGHTEST POINT. Emissive `lamp` blocks are rationed: a two-block ember bed
in the fire pit (near, large, the hotspot), one lantern at the far dock end
(distant, tiny), and two behind the cabin's glass panes (attenuated by the pane
so they read as a warm interior, not as a second hotspot). Nothing else glows.
`_shiba_beach_compose.py` ranks them by apparent brightness and prints the
dominance ratio.

EDGE DETAIL is the one art-gap axis a map file can actually move — see
docs/art-gap-vs-reference-2026-08-17.md pile B, which names `maps/` as the lane
that owns it. Every large surface here is broken up: clustered speckle in the
sand and grass, a ragged (never straight) grass/sand and sand/water boundary,
half-timbered cabin walls, a raised boardwalk that casts a striped shadow, and
irregular tree canopies instead of solid slabs. The blocks that carry FILE ART
(`assets/textures/blocks/atlas.json`: sand, grass, leaves, wood, stone,
limestone, glass) are preferred where a surface is large and near.
"""
from __future__ import annotations

import json
import math
import os
import random
from collections import Counter

# Transcribed from client/src/mapfile.rs::block_id_from_name. A name outside
# this set is a silent hole in the world, so the writer refuses it.
BLOCK_NAMES = {
    "air", "grass", "dirt", "stone", "sand", "wood", "leaves", "snow",
    "red_sand", "clay", "gravel", "cobblestone", "obsidian", "brick", "moss",
    "limestone", "lamp", "glass",
}

SIZE = 64          # 2x2 chunks
CHUNKS = 2
MAX_Y = 32         # maps/FORMAT.md: y must be in [0, 32)

RNG = random.Random(20260817)

# Sparse column store: (x, z) -> {y: block}. Later writes win, which is how the
# props overwrite the ground layer they stand on without leaving a duplicate.
COLS: dict[tuple[int, int], dict[int, str]] = {}


def put(x, y, z, block):
    if not (0 <= x < SIZE and 0 <= z < SIZE and 0 <= y < MAX_Y):
        raise ValueError(f"out of bounds: ({x},{y},{z})")
    if block not in BLOCK_NAMES:
        raise ValueError(f"unknown block name {block!r} — see mapfile.rs")
    COLS.setdefault((x, z), {})[y] = block


def get(x, y, z):
    return COLS.get((x, z), {}).get(y)


def surface(x, z):
    """Highest solid y in a column, or -1 for open water. Everything planted
    after the terrain pass sits at `surface + 1` so a palm on the dune is not
    buried to its canopy and a rock in the surf is not floating."""
    col = COLS.get((x, z))
    return max(col) if col else -1


# ---------------------------------------------------------------------------
# 1. shoreline + zone curves — every boundary in this scene is ragged on
#    purpose. A straight line across the frame is the single most obvious
#    "generated" tell, and a straight edge contributes one long edge where a
#    ragged one contributes dozens (art-gap pile B).
# ---------------------------------------------------------------------------
def shore_z(x: int) -> int:
    """Where the sand ends and the sea begins, per column. Always >= 40."""
    bay = 43.0 + 2.6 * math.sin((x + 5) / 9.5) + 1.7 * math.sin(x / 3.7)
    # a spit reaching out beside the dock, and a scoured notch further along
    bay += 2.2 * math.exp(-((x - 31) ** 2) / 70.0)
    bay -= 1.6 * math.exp(-((x - 12) ** 2) / 40.0)
    return max(40, min(47, int(round(bay))))


def sand_z(x: int) -> int:
    """Where the grass yard gives way to sand."""
    v = 19.0 + 2.4 * math.sin(x / 6.1) + 1.5 * math.sin((x + 3) / 2.6)
    return int(round(v))


def wet_band(x: int) -> int:
    """Depth (in z) of the wet/cool sand band inside the shoreline."""
    return 3 + (1 if math.sin(x / 5.0) > 0.3 else 0)


# ---------------------------------------------------------------------------
# 2. ground plane — ONE layer at y=0 everywhere on land (see module docstring),
#    with the material chosen per column and broken up by clustered speckle.
# ---------------------------------------------------------------------------
def speckle_patches(seed_pts, radius, chance):
    """Grow small blobs around seed points -> a set of (x,z) cells.

    Clustered, not per-cell random: scattered single pixels read as noise, small
    patches read as shells, pebbles, moss.
    """
    out = set()
    for (sx, sz) in seed_pts:
        for dx in range(-radius, radius + 1):
            for dz in range(-radius, radius + 1):
                if dx * dx + dz * dz > radius * radius:
                    continue
                if RNG.random() < chance:
                    out.add((sx + dx, sz + dz))
    return out


def build_ground():
    # Blob radius and fill are tuned UP from the first pass: at radius 1-2 with
    # p=0.45 the patches came out as isolated single cells and the yard read as
    # a chessboard, which is noise, not texture. Bigger, denser blobs read as
    # shell drifts, pebble beds and moss.
    shell = speckle_patches(
        [(RNG.randrange(0, SIZE), RNG.randrange(22, 46)) for _ in range(11)], 2, 0.62)
    pebble = speckle_patches(
        [(RNG.randrange(0, SIZE), RNG.randrange(22, 46)) for _ in range(14)], 3, 0.66)
    dune_streak = speckle_patches(
        [(RNG.randrange(0, SIZE), RNG.randrange(20, 34)) for _ in range(11)], 4, 0.50)
    mosspatch = speckle_patches(
        [(RNG.randrange(0, SIZE), RNG.randrange(0, 22)) for _ in range(22)], 3, 0.70)
    scuff = speckle_patches(
        [(RNG.randrange(0, SIZE), RNG.randrange(0, 18)) for _ in range(5)], 2, 0.55)

    for x in range(SIZE):
        sz = sand_z(x)
        wz = shore_z(x)
        wet_from = wz - wet_band(x)
        for z in range(0, wz + 1):
            if z < sz:
                b = "grass"
                if (x, z) in mosspatch:
                    b = "moss"
                elif (x, z) in scuff:
                    b = "dirt"
            elif z <= sz + 1 and RNG.random() < 0.5:
                # the ragged grass/sand seam: two materials interleaved for a
                # couple of rows instead of one hard line
                b = "grass" if RNG.random() < 0.5 else "sand"
            elif z >= wet_from:
                # wet sand — `clay` is the palette's one genuinely COOL block
                # (#7e96a0), so the tide band is also where the frame's cool
                # channel comes from at ground level (art-gap target 2).
                b = "clay" if RNG.random() < 0.62 else "gravel"
            else:
                b = "sand"
                if (x, z) in shell:
                    b = "limestone"
                elif (x, z) in pebble:
                    b = "gravel"
                elif (x, z) in dune_streak:
                    b = "red_sand"
            put(x, 0, z, b)


def build_foam():
    """A broken white line where the water meets the sand.

    `snow` (#f0ece0) is the brightest block in the palette and its relief grade
    is "almost no shape, wide finish spread" (voxel.rs) — which is exactly what
    surf reads as. Broken, never continuous: a solid white line is a fence.
    """
    for x in range(SIZE):
        wz = shore_z(x)
        for z in (wz, wz - 1):
            if RNG.random() < (0.55 if z == wz else 0.22):
                put(x, 0, z, "snow")


# ---------------------------------------------------------------------------
# 3. raised terrain — dune that closes the left edge, bluff that closes the
#    right. Both are built ON the ground layer and both are kept clear of the
#    Rust prop footprints.
# ---------------------------------------------------------------------------
def mound(cx, cz, rx, rz, height, top, side, jitter=0.65):
    """A soft elliptical rise. `top` covers the crown, `side` the flanks."""
    for x in range(cx - rx, cx + rx + 1):
        for z in range(cz - rz, cz + rz + 1):
            if not (0 <= x < SIZE and 0 <= z < SIZE):
                continue
            d = ((x - cx) / rx) ** 2 + ((z - cz) / rz) ** 2
            if d > 1.0:
                continue
            h = height * (1.0 - d) + RNG.uniform(-jitter, jitter)
            h = int(round(h))
            for y in range(1, h + 1):
                put(x, y, z, top if y == h else side)


def build_terrain_frames():
    # LEFT-EDGE DUNE (high x, near the camera) — the mass that closes the frame
    # and stops the eye walking out of the left side.
    mound(52, 11, 9, 7, 4, "grass", "dirt")
    mound(59, 20, 6, 6, 3, "grass", "dirt")
    mound(45, 5, 5, 4, 3, "grass", "dirt")
    # RIGHT-SIDE BLUFF (low x, mid distance) — a rocky point the beach runs out
    # to, so the right third has a silhouette instead of empty sand.
    mound(4, 30, 7, 6, 5, "cobblestone", "stone")
    mound(9, 38, 5, 4, 3, "gravel", "stone")
    # a couple of low sand rises so the open beach is not a billiard table
    mound(20, 30, 6, 4, 2, "sand", "sand", jitter=0.4)
    mound(38, 34, 5, 3, 2, "sand", "sand", jitter=0.4)


# ---------------------------------------------------------------------------
# 4. the cabin — a 3/4-readable building, not a box. Half-timbered walls, a
#    pitched gable, a chimney breaking the skyline, a veranda on the yard side
#    (the side the hero camera actually sees) and glass with a lamp behind it.
# ---------------------------------------------------------------------------
CX0, CX1 = 24, 34     # wall planes in x
CZ0, CZ1 = 16, 24     # CZ0 = yard/camera side, CZ1 = sea side (the pots' door)
FLOOR_Y = 1
WALL_Y0, WALL_Y1 = 2, 5
RIDGE_Y = 9


def roof_y(x):
    """Gable height above the ridge line at x = 29."""
    d = abs(x - 29)
    return RIDGE_Y - int(round(d * 0.72))


def build_cabin():
    # plinth: a pale limestone course the low sun rakes across
    for x in range(CX0, CX1 + 1):
        for z in range(CZ0, CZ1 + 1):
            edge = x in (CX0, CX1) or z in (CZ0, CZ1)
            put(x, FLOOR_Y, z, "limestone" if edge else "wood")

    posts = {CX0, CX0 + 1, 29, CX1 - 1, CX1}

    def wall_block(u, y):
        """Corner posts and a mid-rail in wood, panels in pale clay.

        Two materials on one wall is the cheapest big win on edge density there
        is: it turns a 10x4 flat rectangle into a grid of material boundaries.
        """
        if u in posts or y == WALL_Y1:
            return "wood"
        # `clay` (#7e96a0) only. The first pass alternated clay with limestone
        # (#decca8) and the wall came out as a pale chequerboard that pulled
        # more attention than the fire did. One cool panel colour against warm
        # timber is the read; two is a pattern.
        return "clay"

    # ---- yard wall (z = CZ0): the face the hero camera sees ----------------
    door_x = (28, 29)
    win_yard = {(26, 3), (26, 4), (27, 3), (27, 4), (31, 3), (31, 4), (32, 3), (32, 4)}
    for x in range(CX0, CX1 + 1):
        for y in range(WALL_Y0, WALL_Y1 + 1):
            if x in door_x and y <= WALL_Y0 + 2:
                continue                      # yard door, onto the fire pit
            if (x, y) in win_yard:
                put(x, y, CZ0, "glass")
                continue
            put(x, y, CZ0, wall_block(x, y))
    # ---- sea wall (z = CZ1): keeps the door the Rust pots flank ------------
    win_sea = {(26, 3), (26, 4), (32, 3), (32, 4)}
    for x in range(CX0, CX1 + 1):
        for y in range(WALL_Y0, WALL_Y1 + 1):
            if x in door_x and y <= WALL_Y0 + 2:
                continue
            if (x, y) in win_sea:
                put(x, y, CZ1, "glass")
                continue
            put(x, y, CZ1, wall_block(x, y))
    # ---- side walls --------------------------------------------------------
    win_side = {(17, 3), (17, 4), (18, 3), (18, 4)}
    for z in range(CZ0, CZ1 + 1):
        for y in range(WALL_Y0, WALL_Y1 + 1):
            put(CX0, y, z, wall_block(z, y))
            if (z, y) in win_side:
                put(CX1, y, z, "glass")
            else:
                put(CX1, y, z, wall_block(z, y))

    # ---- interior lanterns: seen only THROUGH the panes, so they read as a
    # ---- warm room rather than as a second hotspot in frame.
    put(31, 4, CZ0 + 1, "lamp")
    put(26, 4, CZ0 + 4, "lamp")

    # ---- pitched roof, with an overhang on all four sides ------------------
    #
    # DARK GREY, not brick and not pale stone. The first pass roofed this in
    # `brick` (150,90,60) over `wood` walls (156,107,58) — six points apart in
    # every channel, so roof and walls merged into one orange mass. `stone`
    # (143,135,118) separated them but is nearly as bright as the sky at the
    # horizon, so the roofline vanished instead. `gravel` (110,100,94) is the
    # darkest non-black grey in the palette and carries the strongest relief
    # grade there is (voxel.rs: 1.00, "a pile of separate loose objects"), so
    # the roof both silhouettes against the dusk sky and reads as shingle.
    for x in range(CX0 - 1, CX1 + 2):
        ry = roof_y(x)
        cap = "brick" if abs(x - 29) <= 1 else "gravel"   # warm ridge cap
        for z in range(CZ0 - 1, CZ1 + 2):
            put(x, ry, z, cap)
            # closed gable ends, so the roof is not a floating plate
            if z in (CZ0 - 1, CZ1 + 1):
                for y in range(WALL_Y1 + 1, ry):
                    put(x, y, z, "wood" if x in posts else "clay")
        # the step down the pitch leaves a one-block riser; fill it so the roof
        # reads as a surface instead of a flight of stairs with holes
        if x > CX0 - 1 and roof_y(x - 1) > ry:
            for z in range(CZ0 - 1, CZ1 + 2):
                for y in range(ry, roof_y(x - 1)):
                    put(x, y, z, "gravel")

    # ---- chimney: 2x2 and punched THROUGH the roof, on the camera side ----
    # (1x1 read as a flagpole from 35 voxels out — a chimney has to have width
    # to be a chimney)
    for cx in (31, 32):
        for cz in (CZ0 + 3, CZ0 + 4):
            for y in range(WALL_Y0, RIDGE_Y + 2):
                put(cx, y, cz, "cobblestone")
            put(cx, RIDGE_Y + 2, cz, "brick")   # a warm cap to catch the low sun

    # ---- veranda on the yard side: deck, posts, awning --------------------
    for x in range(CX0 + 1, CX1):
        for z in range(CZ0 - 3, CZ0):
            put(x, FLOOR_Y, z, "wood")
    for x in (CX0 + 1, 27, 31, CX1 - 1):
        for y in range(WALL_Y0, WALL_Y0 + 3):
            put(x, y, CZ0 - 3, "wood")
        put(x, WALL_Y0 + 3, CZ0 - 3, "wood")
    for x in range(CX0 + 1, CX1):
        for z in range(CZ0 - 2, CZ0):
            put(x, WALL_Y0 + 3, z, "wood")

    # ---- dressing on the veranda: potted plants (the map's own version of
    # ---- the Rust pots, on the side the camera can actually see), a bench,
    # ---- a stacked woodpile against the wall.
    for (px, pz) in ((26, CZ0 - 3), (30, CZ0 - 3)):
        put(px, WALL_Y0, pz, "brick")
        put(px, WALL_Y0 + 1, pz, "leaves")
    for x in range(CX0 + 2, CX0 + 6):
        put(x, WALL_Y0, CZ0 - 1, "wood")
    for x in range(31, 34):
        for y in (WALL_Y0, WALL_Y0 + 1):
            put(x, y, CZ0 - 1, "wood")


# ---------------------------------------------------------------------------
# 5. the dock — a RAISED boardwalk (deck at y=2, not flush with the sand), so
#    it throws a striped shadow across the beach and reads as a structure.
# ---------------------------------------------------------------------------
DOCK_X = (29, 30, 31)
# Starts at z=26, not at the cabin wall: the Rust flower pot at (30.5, 1, 25)
# would otherwise sit UNDER the raised deck. Row z=25 stays open sand.
DOCK_Z0, DOCK_Z1 = 26, 46
DECK_Y = 2


def build_dock():
    for z in range(DOCK_Z0, DOCK_Z1 + 1):
        for x in DOCK_X:
            put(x, DECK_Y, z, "wood")
        if (z - DOCK_Z0) % 3 == 0:
            for x in (DOCK_X[0], DOCK_X[-1]):
                put(x, DECK_Y - 1, z, "wood")     # pile head under the deck
                put(x, 0, z, "wood")              # pile foot
        if (z - DOCK_Z0) % 4 == 1:
            for x in (DOCK_X[0], DOCK_X[-1]):
                put(x, DECK_Y + 1, z, "wood")     # railing stub
    # landing platform + the far beacon
    for x in range(28, 33):
        for z in range(DOCK_Z1 - 2, DOCK_Z1 + 1):
            put(x, DECK_Y, z, "wood")
    for y in (DECK_Y + 1, DECK_Y + 2):
        put(32, y, DOCK_Z1, "wood")
    put(32, DECK_Y + 3, DOCK_Z1, "lamp")          # ONE distant lantern
    # crates and a fish-drying rack at the shore end
    for (bx, bz, h) in ((27, 26, 2), (27, 27, 1), (33, 27, 2), (33, 26, 1)):
        for y in range(1, h + 1):
            put(bx, y, bz, "wood")
    for zx in (34, 37):
        for y in (1, 2, 3):
            put(zx, y, 29, "wood")
    for x in range(34, 38):
        put(x, 3, 29, "wood")


# ---------------------------------------------------------------------------
# 6. the fire pit — the frame's single brightest point.
#
#    Yamamoto's Rust campfire (beach_shot.rs::PROP_CAMPFIRE) is anchored at the
#    world POINT (23, 1, 14), which is the shared corner of cells (22,13),
#    (22,14), (23,13), (23,14) — its stone ring is r=0.85 and its logs are 1.05
#    long, so they sit inside those four cells. The block hearth is therefore
#    built as a ring at radius 2 with an ember bed on the (22,13)/(23,14)
#    diagonal: the two blocks that glow, with the Rust logs and point light
#    landing in the two cells left open between them.
# ---------------------------------------------------------------------------
FIRE = (23, 14)


def build_firepit():
    fx, fz = FIRE
    for x in range(fx - 2, fx + 2):
        for z in range(fz - 2, fz + 2):
            ring = x in (fx - 2, fx + 1) or z in (fz - 2, fz + 1)
            if ring:
                put(x, 1, z, "cobblestone" if (x + z) % 3 else "stone")
    put(fx - 1, 1, fz - 1, "lamp")
    put(fx, 1, fz, "lamp")
    # log seats and a small stack of firewood, so the hearth reads as used
    for (lx, lz, n, along_x) in ((fx - 5, fz + 1, 3, False), (fx + 3, fz - 3, 3, True),
                                 (fx + 1, fz + 4, 2, True)):
        for i in range(n):
            put(lx + (i if along_x else 0), 1, lz + (0 if along_x else i), "wood")
    for i in range(3):
        put(fx - 4, 1 + i // 2, fz - 4 + (i % 2), "wood")


# ---------------------------------------------------------------------------
# 7. planting + scatter — the detail layer. This is where the art-gap edge
#    number is actually won or lost.
# ---------------------------------------------------------------------------
def palm(x, z, height, lean=0):
    """A tall trunk with an IRREGULAR canopy.

    A solid 3x3x2 canopy slab has 4 silhouette edges; a ragged one has dozens,
    and it is the silhouette against a bright dusk sky that the eye reads.
    """
    base = surface(x, z) + 1
    top = None
    for i in range(height):
        tx = x + (lean if i >= height - 2 else 0)
        put(tx, base + i, z, "wood")
        top = (tx, base + i)
    cx, cy = top
    for dx in range(-2, 3):
        for dz in range(-2, 3):
            for dy in (0, 1):
                d = abs(dx) + abs(dz) + dy * 2
                if d > 3:
                    continue
                if d == 3 and RNG.random() < 0.55:
                    continue
                if 0 <= cx + dx < SIZE and 0 <= z + dz < SIZE:
                    put(cx + dx, cy + dy, z + dz, "leaves")
    put(cx, cy + 2, z, "leaves")
    if RNG.random() < 0.6:
        put(cx + RNG.choice((-1, 1)), cy + 1, z + RNG.choice((-1, 1)), "leaves")


def rock_cluster(cx, cz, n, tall=2, materials=("cobblestone", "stone", "gravel")):
    cells = [(cx, cz)]
    for _ in range(n - 1):
        bx, bz = RNG.choice(cells)
        cells.append((bx + RNG.choice((-1, 0, 1)), bz + RNG.choice((-1, 0, 1))))
    for (x, z) in cells:
        if not (0 <= x < SIZE and 0 <= z < SIZE):
            continue
        base = surface(x, z) + 1   # 0 out in the water, ground top + 1 on land
        h = RNG.randint(1, tall)
        for y in range(base, base + h):
            put(x, y, z, RNG.choice(materials))


def driftwood(x, z, length, along_x=True):
    for i in range(length):
        dx = x + (i if along_x else i // 3)
        dz = z + (i // 3 if along_x else i)
        if 0 <= dx < SIZE and 0 <= dz < SIZE:
            put(dx, surface(dx, dz) + 1, dz, "wood")


def build_planting():
    # --- the palm line: dense on the left (high x) where it frames the edge,
    # --- thinning toward the right so the eye is not walled in.
    for (px, pz, h, lean) in (
        (49, 9, 7, 1), (54, 14, 8, -1), (46, 16, 6, 0), (58, 8, 7, 1),
        (43, 21, 7, -1), (51, 24, 6, 0), (60, 27, 7, 1), (44, 3, 6, 0),
        (38, 12, 6, -1), (17, 8, 7, 1), (11, 15, 6, 0), (6, 6, 7, -1),
    ):
        palm(px, pz, h, lean)
    # --- shrubs: single leaf blocks read as beach scrub and cost one block each
    for _ in range(150):
        x = RNG.randrange(0, SIZE)
        z = RNG.randrange(0, sand_z(x) + 3)
        if get(x, 1, z) is None and not (CX0 - 4 <= x <= CX1 + 2 and CZ0 - 5 <= z <= CZ1 + 2):
            put(x, 1, z, "leaves" if RNG.random() < 0.7 else "moss")
    # --- FOREGROUND mass (z 0-12): the dark shapes the eye enters the frame
    # --- through. Big, near, and deliberately unlit-side-on to the sun.
    for (cx, cz, n, tall) in ((33, 3, 9, 3), (30, 7, 6, 2), (39, 5, 7, 3),
                              (26, 2, 5, 2), (20, 4, 6, 2), (13, 2, 7, 3)):
        rock_cluster(cx, cz, n, tall)
    driftwood(28, 9, 6, along_x=True)
    driftwood(35, 8, 5, along_x=False)
    driftwood(18, 11, 5, along_x=True)
    # --- beach scatter: rocks and driftwood down the sand, sizes falling off
    # --- with distance so the recession reads.
    for (cx, cz, n) in ((14, 26, 5), (22, 33, 4), (44, 30, 5), (52, 36, 4),
                        (33, 38, 3), (8, 24, 4), (58, 33, 4)):
        rock_cluster(cx, cz, n, 2)
    driftwood(24, 28, 5, along_x=True)
    driftwood(41, 27, 4, along_x=False)
    driftwood(48, 33, 5, along_x=True)
    # --- surf rocks: obsidian is the palette's one smooth, dark, half-
    # --- transparent block (voxel.rs: roughness 0.06, reflectance 0.92) — wet
    # --- rock at the waterline is exactly what it is for.
    for x in (7, 19, 26, 37, 45, 56):
        wz = shore_z(x)
        rock_cluster(x, wz + 1, RNG.randint(2, 4), 2,
                     materials=("obsidian", "obsidian", "cobblestone"))


# ---------------------------------------------------------------------------
# 8. background — everything past the waterline. Read as silhouette only, so
#    it is built for shape, not for material detail.
# ---------------------------------------------------------------------------
def build_background():
    # sea stacks: the vertical accents that stop the horizon being one ruled line
    for (sx, sz, h) in ((16, 52, 5), (18, 55, 3), (46, 51, 4), (49, 53, 3),
                        (10, 48, 3), (57, 57, 4)):
        for y in range(0, h):
            put(sx, y, sz, "stone" if y else "cobblestone")
        if h >= 4:
            put(sx + 1, 0, sz, "cobblestone")
            put(sx, h, sz, "gravel")
    # the island: sand crown, a little grass, two palms — the far anchor
    for x in range(24, 36):
        for z in range(53, 62):
            d = ((x - 30) / 6.0) ** 2 + ((z - 57) / 4.5) ** 2
            if d > 1.0:
                continue
            put(x, 0, z, "sand" if d > 0.35 else "grass")
            if d < 0.3 and RNG.random() < 0.5:
                put(x, 1, z, "grass")
    palm(29, 57, 6, 0)
    palm(32, 56, 5, 1)
    rock_cluster(26, 59, 4, 2)
    # a low breakwater running out from the right-hand bluff
    for i in range(9):
        x = 6 + i
        z = shore_z(x) + 2 + i // 3
        put(x, 0, z, "cobblestone")
        if i % 2 == 0:
            put(x, 1, z, "gravel")


# ---------------------------------------------------------------------------
# 9. the path — dirt, but a WORN one: it wanders, and its edges are frayed.
# ---------------------------------------------------------------------------
def build_path():
    z = 0
    while z <= CZ0 - 4:
        cx = 28 + int(round(1.6 * math.sin(z / 5.0)))
        for x in (cx, cx + 1):
            put(x, 0, z, "dirt")
        if RNG.random() < 0.45:
            put(cx + RNG.choice((-1, 2)), 0, z, "dirt")
        z += 1
    # a short spur from the path to the fire pit
    for i in range(6):
        put(27 - i, 0, 13 + i // 3, "dirt")


# ---------------------------------------------------------------------------
# write + validate
# ---------------------------------------------------------------------------
def main(bones=False, out_path=None):
    """`bones=True` writes the set WITHOUT the planting/scatter layer.

    Used to lock the camera against the fixed skeleton (ground, shoreline,
    cabin, dock, hearth) before dressing — dressing a frame whose camera is
    still moving is how the first pass ended up with a palm through the lens.
    """
    build_ground()
    build_terrain_frames()
    build_foam()
    build_path()
    build_cabin()
    build_dock()
    build_firepit()
    if not bones:
        build_planting()
        build_background()

    blocks = []
    for (x, z), col in COLS.items():
        for y, name in col.items():
            blocks.append({"x": x, "y": y, "z": z, "block": name})
    blocks.sort(key=lambda b: (b["z"], b["y"], b["x"]))

    # ---- self-validation: every check main.rs::apply_map_blocks would fail on
    for b in blocks:
        assert b["block"] in BLOCK_NAMES, b
        assert 0 <= b["x"] < CHUNKS * 32 and 0 <= b["z"] < CHUNKS * 32, b
        assert 0 <= b["y"] < MAX_Y, b
    seen = {(b["x"], b["y"], b["z"]) for b in blocks}
    assert len(seen) == len(blocks), "duplicate voxel"
    # the water plane's edge must never be exposed: land has to reach z=40
    for x in range(SIZE):
        for z in range(0, 41):
            assert (x, 0, z) in seen, f"hole in the ground at ({x},0,{z})"

    # `scene.rs::map_spawn` spirals from the world centre for a standable column
    assert (32, 0, 32) in seen, "map centre must be standable for --play spawn"
    tops = {}
    for b in blocks:
        k = (b["x"], b["z"])
        tops[k] = max(tops.get(k, -1), b["y"])
    assert tops[(32, 32)] + 3 < 32, "spawn column has no head room"
    # The Rust props stand on a y=1 surface, so their columns must carry exactly
    # one ground block and nothing else. The FIRE PIT is the deliberate
    # exception: its two ember blocks occupy (22,1,13) and (23,1,14), the cells
    # the Rust campfire's stone ring and logs sit inside. That intersection is
    # wanted — the small grey stones end up hidden inside a glowing block and
    # the (shadow-less) point light still lights the yard, so the hearth reads
    # as one hot core whether or not `VOXELFORGE_BEACHSHOT` is set.
    # (the pots stand under the roof's eave overhang, which is fine — what must
    # be clear is the block cell the pot mesh itself occupies, y = 1)
    for (px, pz, who) in ((27, 25, "pot A"), (30, 25, "pot B")):
        assert (px, 0, pz) in seen, f"{who} at ({px},{pz}) has no ground"
        assert (px, 1, pz) not in seen, f"{who} at ({px},{pz}) is inside a block"
    for (px, pz) in ((22, 13), (23, 14)):
        assert COLS[(px, pz)].get(1) == "lamp", "fire pit ember bed moved"

    data = {
        "version": 1,
        "name": "beach-dusk",
        "size": {"chunks_x": CHUNKS, "chunks_z": CHUNKS},
        "blocks": blocks,
    }
    out = out_path or os.path.join(os.path.dirname(__file__), "..", "maps", "beach_dusk.json")
    with open(out, "w", encoding="utf-8") as fh:
        json.dump(data, fh, indent=2)
    c = Counter(b["block"] for b in blocks)
    print(f"wrote {len(blocks)} blocks -> {os.path.abspath(out)}")
    print("palette:", ", ".join(f"{k}={v}" for k, v in c.most_common()))
    print("lamps:", [(b["x"], b["y"], b["z"]) for b in blocks if b["block"] == "lamp"])


if __name__ == "__main__":
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--bones", action="store_true",
                    help="skip the planting/scatter layer (camera-locking pass)")
    ap.add_argument("--out", default=None)
    a = ap.parse_args()
    main(bones=a.bones, out_path=a.out)
