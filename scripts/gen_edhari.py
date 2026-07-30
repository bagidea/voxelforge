#!/usr/bin/env python3
"""Generate the redesigned Edhari village voxel map for Voxelforge.

This pass turns the flat 64x64 campsite into a deliberately authored ruin:
- Long sight-lines to three landmarks (watchtower, petrified tree, ruined keep)
  that pull the eye north toward the dungeon gate.
- A readable spawn -> campfire -> first-encounter path on the main street.
- Multiple elevations: sunken fire plaza, raised village terraces, a stone
  bridge, and a stepped ascent to the sealed gate.
- Stone arches that frame the journey (shelter opening, village gate, bridge,
  dungeon gate).
- Environmental story told with blocks alone: burned-out houses, scattered
  belongings, ash piles, and Unravelling pits.

Block palette is restricted to what client/src/mapfile.rs actually loads:
air, grass, dirt, stone, sand.  Wood/leaves are NOT emitted (see maps/FORMAT.md
and the first-five-minutes punch list).

Coordinate convention matches client/src/scene.rs:
- -Z is "north"; the player wakes at (32,32) facing -Z.
- FIRE_AHEAD=3.0 places the procedural campfire at (32,29).
- The engine spawns a Guard Husk 7 blocks north of the player at (32,25).
"""
import json
import math
import random
from pathlib import Path

WIDTH = 64
DEPTH = 64
CHUNKS_X = 2
CHUNKS_Z = 2
SPAWN_X, SPAWN_Z = 32, 32

# New seed for the redesigned layout; keep it fixed so regeneration is stable.
rng = random.Random(20260731)

blocks = {}


def add(x, y, z, block):
    if not (0 <= x < WIDTH and 0 <= z < DEPTH and 0 <= y < 32):
        raise ValueError(f"out of bounds: ({x},{y},{z})")
    blocks[(x, y, z)] = block


def in_bounds(x, z):
    return 0 <= x < WIDTH and 0 <= z < DEPTH


# ---------------------------------------------------------------------------
# Basic volume helpers
# ---------------------------------------------------------------------------
def box(x0, x1, y0, y1, z0, z1, block):
    """Solid axis-aligned box, inclusive bounds."""
    for y in range(y0, y1 + 1):
        for z in range(z0, z1 + 1):
            for x in range(x0, x1 + 1):
                add(x, y, z, block)


def hollow_box(x0, x1, y0, y1, z0, z1, block):
    """Walls, floor and ceiling only; interior left empty."""
    for y in range(y0, y1 + 1):
        for z in range(z0, z1 + 1):
            for x in range(x0, x1 + 1):
                if x0 < x < x1 and z0 < z < z1 and y0 < y < y1:
                    continue
                add(x, y, z, block)


def disc(cx, cz, r, y, block):
    for dz in range(-r, r + 1):
        for dx in range(-r, r + 1):
            if dx * dx + dz * dz <= r * r:
                x, z = cx + dx, cz + dz
                if in_bounds(x, z):
                    add(x, y, z, block)


def floor_all(block="grass"):
    for z in range(DEPTH):
        for x in range(WIDTH):
            add(x, 0, z, block)


# ---------------------------------------------------------------------------
# Terrain / paving
# ---------------------------------------------------------------------------
def paved_rect(x0, x1, z0, z1, block, y=0):
    for z in range(z0, z1 + 1):
        for x in range(x0, x1 + 1):
            add(x, y, z, block)


def paved_disc(cx, cz, r, block, y=0):
    disc(cx, cz, r, y, block)


def raise_terrain(patch, y, block="grass"):
    """Raise a list of (x,z) columns to y by capping them at that level.
    Any existing lower blocks remain (the loader's top-solid scan picks the
    highest anyway); we just add the cap block."""
    for x, z in patch:
        if in_bounds(x, z):
            add(x, y, z, block)


# ---------------------------------------------------------------------------
# Architecture primitives
# ---------------------------------------------------------------------------
def stone_arch(cx, cz, y_base, height, span, width=1, axis="x"):
    """A simple stone arch over a path.  span = half-width of the opening."""
    y_top = y_base + height
    if axis == "x":
        # Pillars at cx - span and cx + span, z in [cz - width//2, cz + width//2]
        z0, z1 = cz - width // 2, cz + width // 2
        for z in range(z0, z1 + 1):
            for y in range(y_base + 1, y_top + 1):
                add(cx - span, y, z, "stone")
                add(cx + span, y, z, "stone")
            # lintel
            for x in range(cx - span, cx + span + 1):
                add(x, y_top + 1, z, "stone")
    else:
        # arch along z
        x0, x1 = cx - width // 2, cx + width // 2
        for x in range(x0, x1 + 1):
            for y in range(y_base + 1, y_top + 1):
                add(x, y, cz - span, "stone")
                add(x, y, cz + span, "stone")
            for z in range(cz - span, cz + span + 1):
                add(x, y_top + 1, z, "stone")


def stairs(x0, x1, z_start, z_end, y_start, rise=1):
    """A straight flight of stairs climbing along z.  rise blocks per step."""
    steps = abs(z_end - z_start) + 1
    dz = 1 if z_end >= z_start else -1
    y = y_start
    for i in range(steps):
        z = z_start + i * dz
        for x in range(x0, x1 + 1):
            add(x, y, z, "stone")
        if (i + 1) % 2 == 0:
            y += rise
    return y


def stone_bridge(x0, x1, z0, z1, y_deck, pillar_depth=4):
    """A stone bridge with railings and pillars down to the lower ground."""
    # deck
    for z in range(z0, z1 + 1):
        for x in range(x0, x1 + 1):
            add(x, y_deck, z, "stone")
    # railings
    for z in range(z0, z1 + 1):
        add(x0, y_deck + 1, z, "stone")
        add(x1, y_deck + 1, z, "stone")
    # pillars at ends
    for x in (x0, x1):
        for z in (z0, z1):
            for y in range(y_deck - 1, max(0, y_deck - pillar_depth) - 1, -1):
                add(x, y, z, "stone")


# ---------------------------------------------------------------------------
# Buildings
# ---------------------------------------------------------------------------
def intact_house(x0, z0, w, d, door_x, door_z, height=4, y_base=0):
    """Hollow perimeter house with one door gap."""
    x1, z1 = x0 + w - 1, z0 + d - 1
    for y in range(y_base + 1, y_base + height + 1):
        block = "dirt" if y == y_base + height else "stone"
        for x in range(x0, x1 + 1):
            for z in range(z0, z1 + 1):
                if not (x == x0 or x == x1 or z == z0 or z == z1):
                    continue
                if y <= y_base + 2 and x == door_x and z == door_z:
                    continue
                add(x, y, z, block)
    # compact floor at y_base inside
    for x in range(x0 + 1, x1):
        for z in range(z0 + 1, z1):
            add(x, y_base, z, "stone")


def ruin_house(x0, z0, w, d, y_base=0, scattered=False, burned=False):
    """Collapsed house with random broken skyline, rubble, and optional story props."""
    x1, z1 = x0 + w - 1, z0 + d - 1
    for x in range(x0, x1 + 1):
        for z in range(z0, z1 + 1):
            if not (x == x0 or x == x1 or z == z0 or z == z1):
                continue
            stub = rng.choices([0, 1, 2, 3], weights=[35, 30, 25, 10])[0]
            for y in range(y_base + 1, y_base + stub + 1):
                wall_block = "dirt" if y == y_base + stub else "stone"
                add(x, y, z, wall_block)
    # rubble inside / around
    for _ in range((w * d) // 4):
        rx = rng.randint(x0 + 1, x1 - 1)
        rz = rng.randint(z0 + 1, z1 - 1)
        rubble_y = y_base + 1
        if burned and rng.random() < 0.5:
            add(rx, rubble_y, rz, "dirt")  # ash / char
        else:
            add(rx, rubble_y, rz, rng.choice(["stone", "dirt"]))
    # scattered belongings: use stone, not sand — sand is reserved for the sigil.
    if scattered:
        for _ in range(rng.randint(3, 6)):
            rx = rng.randint(x0 - 1, x1 + 1)
            rz = rng.randint(z0 - 1, z1 + 1)
            if in_bounds(rx, rz):
                add(rx, y_base + 1, rz, "stone")


def spawn_shelter(cx, cz, r=3):
    """Crumbling stone shelter around spawn; north wall collapsed open."""
    x0, x1, z0, z1 = cx - r, cx + r, cz - r, cz + r
    for x in range(x0, x1 + 1):
        for z in range(z0, z1 + 1):
            if not (x == x0 or x == x1 or z == z0 or z == z1):
                continue
            if z == z0:
                continue  # open to the north (-Z)
            stub = rng.choices([1, 2, 3], weights=[30, 45, 25])[0]
            for y in range(1, stub + 1):
                add(x, y, z, "stone")


# ---------------------------------------------------------------------------
# Landmarks
# ---------------------------------------------------------------------------
def watchtower(cx, cz, base_w, height):
    """Square stone watchtower with a crenellated top — visible from anywhere."""
    half = base_w // 2
    x0, x1 = cx - half, cx + half
    z0, z1 = cz - half, cz + half
    hollow_box(x0, x1, 1, height, z0, z1, "stone")
    # internal column so it reads solid from a distance
    box(cx, cx, 1, height - 2, cz, cz, "stone")
    # crenellations
    for x in range(x0, x1 + 1):
        for z in range(z0, z1 + 1):
            if (x + z) % 2 == 0:
                add(x, height + 1, z, "stone")


def petrified_tree(cx, cz, height):
    """A tall, dead stone tree; the canopy is rough stone/dirt clusters."""
    # trunk
    for y in range(1, height + 1):
        add(cx, y, cz, "stone")
        if y % 4 == 0 and y < height - 2:
            # branches
            for dx, dz in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
                add(cx + dx, y, cz + dz, "stone")
                add(cx + 2 * dx, y + 1, cz + 2 * dz, "stone")
    # canopy / dead foliage
    for dy in range(-2, 3):
        r = 2 if abs(dy) < 2 else 1
        disc(cx, cz, r, height + dy + 1, "dirt")
    disc(cx, cz, 1, height + 4, "dirt")


def ruined_keep_wall(x0, z0, w, d, height):
    """A fragment of an old fortification with a large breach."""
    x1, z1 = x0 + w - 1, z0 + d - 1
    breach_x = x0 + w // 2
    for x in range(x0, x1 + 1):
        for z in range(z0, z1 + 1):
            if x == breach_x and z0 + 1 <= z <= z1 - 1:
                continue  # breach
            stub = rng.choices([2, 3, 4, height], weights=[20, 30, 30, 20])[0]
            for y in range(1, stub + 1):
                add(x, y, z, "stone")
    # rubble at the foot of the breach
    for z in range(z0 + 1, z1):
        add(breach_x, 1, z, "stone")
        if rng.random() < 0.5:
            add(breach_x, 2, z, "stone")


def village_well(cx, cz, r=2):
    """Raised stone curb + dry dirt bottom."""
    for dz in range(-r, r + 1):
        for dx in range(-r, r + 1):
            d = math.hypot(dx, dz)
            x, z = cx + dx, cz + dz
            if not in_bounds(x, z):
                continue
            if r - 1 < d <= r:
                add(x, 1, z, "stone")
            elif d <= r - 1:
                add(x, 0, z, "dirt")


def dungeon_gate(cx, z_near, z_far, y_base=0, half_span=5, height=8):
    """Massive sealed gate with a visible sand sigil."""
    x_out_l, x_in_l = cx - half_span, cx - half_span + 1
    x_in_r, x_out_r = cx + half_span - 1, cx + half_span
    # pillars
    for z in range(z_near, z_far + 1):
        for x in (x_out_l, x_in_l, x_in_r, x_out_r):
            for y in range(y_base + 1, y_base + height + 1):
                add(x, y, z, "stone")
    # sealed door fill
    for x in range(x_in_l + 1, x_in_r):
        for z in range(z_near, z_far + 1):
            for y in range(y_base + 1, y_base + height - 2):
                add(x, y, z, "stone")
    # lintel
    for x in range(x_out_l, x_out_r + 1):
        for z in range(z_near, z_far + 1):
            add(x, y_base + height + 1, z, "stone")
    # sigil on the +Z face of the lintel, facing the village
    add(cx, y_base + height + 1, z_far, "sand")


# ---------------------------------------------------------------------------
# Story details
# ---------------------------------------------------------------------------
def ash_pile(cx, cz, r=1):
    disc(cx, cz, r, 1, "dirt")


def unravelling_pit(x0, z0, w, d):
    """A sunken, cracked patch where the ground gave way."""
    x1, z1 = x0 + w - 1, z0 + d - 1
    for z in range(z0, z1 + 1):
        for x in range(x0, x1 + 1):
            # remove anything at y=1 that might have been raised by terraces
            blocks.pop((x, 1, z), None)
            # cracked rim
            if x == x0 or x == x1 or z == z0 or z == z1:
                add(x, 1, z, "stone")


def scattered_belongings(cx, cz, count=4):
    """Small stone debris: broken pottery, bone shards, dropped tools.
    Sand is reserved for the dungeon-gate sigil, so debris uses stone."""
    for _ in range(count):
        dx = rng.randint(-2, 2)
        dz = rng.randint(-2, 2)
        x, z = cx + dx, cz + dz
        if in_bounds(x, z):
            add(x, 1, z, "stone")


# ---------------------------------------------------------------------------
# Build the world
# ---------------------------------------------------------------------------
floor_all("grass")

# ---- 1. Spawn shelter and fire plaza (southern anchor) ---------------------
spawn_shelter(SPAWN_X, SPAWN_Z, r=3)
paved_disc(SPAWN_X, SPAWN_Z - 3, 3, "stone")  # campfire at (32,29)

# ---- 2. Main street: the readable spine -----------------------------------
# Keep x=30..34, z=6..29 at y=0 so spawn(32,32), fire(32,29) and husk(32,25)
# all stand on real ground at the expected height.
MAIN_X0, MAIN_X1 = SPAWN_X - 2, SPAWN_X + 2
paved_rect(MAIN_X0, MAIN_X1, 6, 29, "stone", y=0)

# ---- 3. Entry arch into the village proper ---------------------------------
stone_arch(SPAWN_X, 26, y_base=0, height=5, span=4, width=2, axis="x")

# ---- 4. Village square around the well -------------------------------------
village_well(SPAWN_X, 17, r=3)
# Two intact houses on raised terraces flanking the well
intact_house(12, 19, 7, 7, door_x=15, door_z=22, height=4, y_base=1)
intact_house(46, 19, 7, 7, door_x=46, door_z=22, height=4, y_base=1)

# Raise the terraces under those houses
terrace_patch = []
for x in range(10, 55):
    for z in range(16, 26):
        if not (MAIN_X0 <= x <= MAIN_X1):  # keep the main street clear
            terrace_patch.append((x, z))
raise_terrain(terrace_patch, 1, "grass")

# ---- 5. Burned / collapsed houses with story props -------------------------
ruin_house(8, 10, 7, 7, y_base=0, scattered=True, burned=True)
ruin_house(48, 10, 7, 7, y_base=0, scattered=True, burned=True)
ruin_house(10, 36, 7, 7, y_base=0, scattered=True, burned=True)
ruin_house(46, 36, 7, 7, y_base=0, scattered=True, burned=True)

# Scattered belongings near the path
scattered_belongings(28, 21, count=5)
scattered_belongings(36, 21, count=4)
scattered_belongings(30, 15, count=4)

# Ash piles (Unravelling aftermath)
ash_pile(24, 24, r=2)
ash_pile(40, 13, r=1)
ash_pile(14, 38, r=2)

# Unravelling pits
unravelling_pit(20, 32, 4, 5)
unravelling_pit(42, 30, 3, 4)

# ---- 6. Elevation change: bridge and stepped ascent to the gate ------------
# Bridge at z=12..14 carries the main street over a drop to the gate plateau.
stone_bridge(MAIN_X0, MAIN_X1, 12, 14, y_deck=1, pillar_depth=3)
# Steps from the bridge deck up to the gate plateau at y=2
stairs(MAIN_X0, MAIN_X1, 11, 8, y_start=1, rise=1)
# Gate plateau surface
paved_rect(24, 40, 3, 11, "stone", y=2)

# ---- 7. Sealed dungeon gate (northern anchor) -----------------------------
dungeon_gate(SPAWN_X, z_near=3, z_far=5, y_base=2, half_span=6, height=9)

# ---- 8. Landmarks visible from spawn ---------------------------------------
# Watchtower on a low grassy knoll in the south-east, tall enough to read far away.
watchtower(52, 50, base_w=5, height=18)
# Petrified tree in the north-west corner, silhouetted against the sky.
petrified_tree(10, 10, height=14)
# Ruined keep wall along the western edge, framing the village from the side.
ruined_keep_wall(4, 20, 8, 20, height=8)

# Small framed arch near the bridge on the west side, a side-path teaser.
stone_arch(18, 14, y_base=1, height=4, span=2, width=1, axis="z")

# ---- 9. Export -------------------------------------------------------------
out = {
    "version": 1,
    "name": "edhari",
    "size": {"chunks_x": CHUNKS_X, "chunks_z": CHUNKS_Z},
    "blocks": [{"x": x, "y": y, "z": z, "block": b} for (x, y, z), b in blocks.items()],
}

project_root = Path(__file__).resolve().parent.parent
out_path = project_root / "maps" / "edhari.json"
out_path.parent.mkdir(parents=True, exist_ok=True)
with open(out_path, "w", encoding="utf-8") as f:
    json.dump(out, f)

print(f"wrote {len(out['blocks'])} blocks to {out_path}")
