#!/usr/bin/env python3
"""Generate the AAA redesigned Edhari village voxel map for Voxelforge.

This pass turns the flat 64x64 campsite into a deliberately authored ruin that
reads as a world people once lived in:
- Long sight-lines to three landmarks (watchtower, petrified tree, ruined keep)
  that pull the eye north toward the dungeon gate.
- A readable spawn -> campfire -> first-encounter path on the main street.
- Multiple elevations: sunken fire plaza, raised village terraces, a stone
  bridge, and a stepped ascent to the sealed gate.
- Stone arches that frame the journey (shelter opening, village gate, bridge,
  dungeon gate).
- Environmental story told with blocks alone: burned-out houses, scattered
  belongings, ash piles, and Unravelling pits.
- Combat cover placed around the first husk encounter.
- Rest fire spots (campfire + extinguished rings) so the space feels lived in.
- The previously-empty southern half is filled with fields, a pond, fences and
  a travellers' rest stop so the village no longer ends at z=44.

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


def ring(cx, cz, r_inner, r_outer, y, block):
    """Annulus of blocks on one y layer."""
    for dz in range(-r_outer, r_outer + 1):
        for dx in range(-r_outer, r_outer + 1):
            d2 = dx * dx + dz * dz
            if r_inner * r_inner <= d2 <= r_outer * r_outer:
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


def lantern_post(x, z, y_base=0, height=3):
    """Stone marker along a path; doubles as a subtle sight-line guide."""
    for y in range(y_base + 1, y_base + height + 1):
        add(x, y, z, "stone")
    add(x, y_base + height + 1, z, "stone")


def low_wall(x0, x1, z0, z1, y_base=0, height=2, gaps=()):
    """Combat cover: a wall the player can circle, with optional door gaps."""
    for z in range(z0, z1 + 1):
        for x in range(x0, x1 + 1):
            if (x, z) in gaps:
                continue
            if x == x0 or x == x1 or z == z0 or z == z1:
                for y in range(y_base + 1, y_base + height + 1):
                    add(x, y, z, "stone")


def broken_pillar(cx, cz, y_base=0, height=4):
    """A single free-standing broken column for cover or framing."""
    for y in range(y_base + 1, y_base + height + 1):
        add(cx, y, cz, "stone")
    # capstone tilted off-centre
    if in_bounds(cx + 1, cz):
        add(cx + 1, y_base + height, cz, "stone")


# ---------------------------------------------------------------------------
# Buildings
# ---------------------------------------------------------------------------
def intact_house(x0, z0, w, d, door_x, door_z, height=4, y_base=0, furnished=False):
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

    if furnished:
        # stone hearth + chimney
        hx, hz = x0 + 2, z0 + 2
        for y in range(y_base + 1, y_base + height + 2):
            add(hx, y, hz, "stone")
        add(hx, y_base + 1, hz + 1, "dirt")  # cold fire-bed
        # stone table + two dirt sleeping mats
        add(x1 - 2, y_base + 1, z1 - 2, "stone")
        add(x1 - 1, y_base + 1, z1 - 2, "stone")
        add(x1 - 2, y_base + 1, z1 - 1, "dirt")
        add(x1 - 3, y_base + 1, z1 - 1, "dirt")


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
    z0, z1 = cz - half, cz - half
    z0, z1 = cz - half, cz + half
    hollow_box(x0, x1, 1, height, z0, z1, "stone")
    # internal column so it reads solid from a distance
    box(cx, cx, 1, height - 2, cz, cz, "stone")
    # crenellations
    for x in range(x0, x1 + 1):
        for z in range(z0, z1 + 1):
            if (x + z) % 2 == 0:
                add(x, height + 1, z, "stone")
    # beacon pole on top for extra silhouette
    for y in range(height + 2, height + 5):
        add(cx, y, cz, "stone")


def petrified_tree(cx, cz, height):
    """A tall, dead stone tree; the canopy is rough stone/dirt clusters."""
    # trunk
    for y in range(1, height + 1):
        add(cx, y, cz, "stone")
        if y % 3 == 0 and y < height - 2:
            # branches
            for dx, dz in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
                add(cx + dx, y, cz + dz, "stone")
                if in_bounds(cx + 2 * dx, cz + 2 * dz):
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
    # corner tower for silhouette
    tower_x = x1 - 1
    for z in range(z0, z0 + 3):
        for y in range(1, height + 3):
            add(tower_x, y, z, "stone")
            add(tower_x + 1, y, z, "stone")


def fallen_monolith(cx, cz, height):
    """A broken southern marker visible from the spawn shelter."""
    # tilted shaft: rising on one side, snapped off on the other
    for y in range(1, height + 1):
        add(cx, y, cz, "stone")
        if y > height // 2:
            add(cx + 1, y, cz, "stone")
    # fallen chunk beside it
    for dx in range(2, 5):
        add(cx + dx, 1, cz, "stone")
        add(cx + dx, 1, cz + 1, "stone")


def sentinel_spire(cx, cz, height, y_base=1):
    """Tall north-eastern landmark: the first thing the player sees on waking.

    Rises behind the dungeon gate so it reads as a far destination from spawn
    (32,32, facing -Z).  A spiral stair is carved into the cliff-like shaft so
    the vista is reachable on foot.
    """
    top = y_base + height
    # Main shaft: a 3x3 core that tapers to a 1x1 needle.
    for y in range(y_base + 1, top + 1):
        rel = y - y_base
        if rel <= height - 6:
            # broad base (3x3) with corners clipped for a rounded silhouette
            for dx in (-1, 0, 1):
                for dz in (-1, 0, 1):
                    if abs(dx) == 1 and abs(dz) == 1 and rel > 4:
                        continue  # clip corners above the foundation
                    add(cx + dx, y, cz + dz, "stone")
        elif rel <= height - 2:
            # narrower collar
            for dx in (-1, 0, 1):
                for dz in (-1, 0, 1):
                    if abs(dx) == 1 and abs(dz) == 1:
                        continue
                    add(cx + dx, y, cz + dz, "stone")
        else:
            # needle tip
            add(cx, y, cz, "stone")

    # Crenellated crown just below the needle.
    crown_y = top - 4
    for dx in (-2, -1, 0, 1, 2):
        for dz in (-2, -1, 0, 1, 2):
            if abs(dx) == 2 and abs(dz) == 2:
                continue
            add(cx + dx, crown_y, cz + dz, "stone")
    # gaps in the crown so it reads as a ruined lookout, not a solid cap
    for dx in (-1, 0, 1):
        for dz in (-1, 0, 1):
            if dx == 0 and dz == 0:
                continue
            blocks.pop((cx + dx, crown_y, cz + dz), None)

    # Spiral stair on the south-west face (the side facing the village).
    # It climbs from the pedestal up to a vista ledge near the top.
    stair_y = y_base + 2
    for step in range(15):
        sx = cx - 2 + (step % 3)
        sz = cz + 2 - (step // 3)
        if not in_bounds(sx, sz):
            continue
        add(sx, stair_y, sz, "stone")
        if step % 2 == 0:
            stair_y += 1

    # Vista ledge facing south-west toward spawn.
    for dx in range(-2, 1):
        for dz in range(0, 3):
            add(cx + dx, top - 8, cz + dz, "stone")
    # Safety parapet so the ledge reads as intentional.
    for dx in range(-2, 1):
        add(cx + dx, top - 7, cz + 2, "stone")


# ---------------------------------------------------------------------------
# Village fixtures
# ---------------------------------------------------------------------------
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


def village_well(cx, cz, r=2):
    """Raised stone curb + dry dirt bottom, now 3 blocks deep."""
    for dz in range(-r, r + 1):
        for dx in range(-r, r + 1):
            d = math.hypot(dx, dz)
            x, z = cx + dx, cz + dz
            if not in_bounds(x, z):
                continue
            if r - 1 < d <= r:
                # curb rises above ground
                add(x, 1, z, "stone")
                add(x, 2, z, "stone")
            elif d <= r - 1:
                # dry bottom below the surrounding ground
                add(x, 0, z, "dirt")
                add(x, 1, z, "dirt")


def skeleton(cx, cz, y_base=0):
    """A small arrangement of stone blocks suggesting remains near the well."""
    add(cx, y_base + 1, cz, "stone")
    add(cx + 1, y_base + 1, cz, "stone")
    add(cx, y_base + 1, cz + 1, "stone")
    add(cx - 1, y_base + 1, cz, "stone")
    add(cx, y_base + 2, cz, "stone")  # skull-ish lump


def fire_pit(cx, cz, r=2, y_base=0, lit=False):
    """A rest-fire ring: stone curb with dirt/ash centre.
    lit=False means extinguished; the engine still gets a readable fire ring."""
    ring(cx, cz, r - 1, r, y_base + 1, "stone")
    disc(cx, cz, r - 1, y_base + 1, "dirt")
    if lit:
        add(cx, y_base + 1, cz, "stone")  # central fuel marker


def market_stall(cx, cz, y_base=0):
    """A stone-framed stall silhouette in the village square."""
    for y in range(y_base + 1, y_base + 3):
        add(cx - 1, y, cz, "stone")
        add(cx + 1, y, cz, "stone")
    for x in range(cx - 2, cx + 3):
        add(x, y_base + 3, cz, "stone")


def garden_plot(x0, z0, w, d, y_base=0):
    """Stone-bordered dirt garden (fields in the south)."""
    x1, z1 = x0 + w - 1, z0 + d - 1
    for z in range(z0, z1 + 1):
        for x in range(x0, x1 + 1):
            if x == x0 or x == x1 or z == z0 or z == z1:
                add(x, y_base + 1, z, "stone")
            else:
                add(x, y_base + 1, z, "dirt")


def pond(cx, cz, r):
    """Shallow dirt-bottom pond with a stone rim."""
    for dz in range(-r, r + 1):
        for dx in range(-r, r + 1):
            d = math.hypot(dx, dz)
            x, z = cx + dx, cz + dz
            if not in_bounds(x, z):
                continue
            if d <= r:
                add(x, 0, z, "dirt")
            if r - 1 < d <= r:
                add(x, 1, z, "stone")


def fence(x0, z0, x1, z1, y_base=0):
    """Low stone-post fence along a line."""
    dx = 1 if x1 >= x0 else -1
    dz = 1 if z1 >= z0 else -1
    if x0 == x1:
        for z in range(z0, z1 + dz, dz):
            if (z - z0) % 3 == 0:
                add(x0, y_base + 1, z, "stone")
    else:
        for x in range(x0, x1 + dx, dx):
            if (x - x0) % 3 == 0:
                add(x, y_base + 1, z0, "stone")


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
# Combat arena
# ---------------------------------------------------------------------------
def husk_arena_cover():
    """Low walls and broken pillars around the first Guard Husk spawn at (32,25).
    The player can circle these for cover while learning the combat timing."""
    # west flank low wall
    low_wall(24, 28, 23, 27, y_base=0, height=2)
    # east flank low wall
    low_wall(36, 40, 23, 27, y_base=0, height=2)
    # north side partial barrier with a gap in the middle
    low_wall(28, 36, 22, 22, y_base=0, height=2, gaps={(32, 22), (33, 22)})
    # broken pillars the player can kite around
    broken_pillar(27, 25, y_base=0, height=3)
    broken_pillar(37, 25, y_base=0, height=3)
    broken_pillar(30, 28, y_base=0, height=2)


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
skeleton(SPAWN_X + 5, 18)  # remains of someone who didn't reach the well
market_stall(SPAWN_X - 6, 17)
market_stall(SPAWN_X + 8, 19)

# Two intact houses on raised terraces flanking the well, now furnished
intact_house(12, 19, 7, 7, door_x=15, door_z=22, height=4, y_base=1, furnished=True)
intact_house(46, 19, 7, 7, door_x=46, door_z=22, height=4, y_base=1, furnished=True)

# Raise the terraces under those houses
terrace_patch = []
for x in range(10, 55):
    for z in range(16, 26):
        if not (MAIN_X0 <= x <= MAIN_X1):  # keep the main street clear
            terrace_patch.append((x, z))
raise_terrain(terrace_patch, 1, "grass")

# Secondary path from spawn to the well and east house
paved_rect(35, 44, 17, 19, "stone", y=1)
paved_rect(32, 34, 19, 22, "stone", y=1)
# Path from well toward the west ruin area
paved_rect(20, 29, 17, 17, "stone", y=1)

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
ash_pile(22, 28, r=1)

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

# ---- 6b. Vista terrace: the open moment before the sealed gate -------------
# A raised stone balcony east of the gate.  The player walks out, the dungeon
# gate frames the near foreground, and the Sentinel Spire rises behind it.
VISTA_X0, VISTA_X1 = 41, 53
VISTA_Z0, VISTA_Z1 = 3, 11
paved_rect(VISTA_X0, VISTA_X1, VISTA_Z0, VISTA_Z1, "stone", y=2)
# Railing on the east and north sides so it reads as a deliberate overlook.
for x in range(VISTA_X0, VISTA_X1 + 1):
    add(x, 3, VISTA_Z0, "stone")
    add(x, 3, VISTA_Z1, "stone")
for z in range(VISTA_Z0, VISTA_Z1 + 1):
    add(VISTA_X0, 3, z, "stone")
    add(VISTA_X1, 3, z, "stone")
# Gaps in the railing for the approach path and the spire path.
for z in range(6, 9):
    blocks.pop((VISTA_X0, 3, z), None)  # entrance from the gate plateau
for x in range(46, 50):
    blocks.pop((x, 3, VISTA_Z0), None)  # exit toward the spire

# Approach path from the main street / bridge onto the vista terrace.
paved_rect(35, 40, 6, 11, "stone", y=2)
# A second, lower switchback from the southern field for explorers returning.
paved_rect(45, 50, 12, 14, "stone", y=1)
for x in range(45, 51):
    add(x, 2, 12, "stone")

# ---- 7. Sealed dungeon gate (northern anchor) -----------------------------
dungeon_gate(SPAWN_X, z_near=3, z_far=5, y_base=2, half_span=6, height=9)

# ---- 8. Combat cover around first husk encounter --------------------------
husk_arena_cover()

# ---- 9. Rest fire spots ---------------------------------------------------
# Main campfire is the procedural one at (32,29).  Add smaller fire rings
# that read as places villagers paused — reinforcing that this was a home.
fire_pit(22, 24, r=2, y_base=0, lit=False)
fire_pit(42, 24, r=2, y_base=0, lit=False)
fire_pit(32, 48, r=2, y_base=0, lit=True)  # travellers' rest in the south field

# ---- 10. Sight-line lanterns along the main street ------------------------
for z in (25, 20, 15, 10):
    lantern_post(28, z, y_base=0, height=3)
    lantern_post(36, z, y_base=0, height=3)

# ---- 11. Landmarks visible from spawn --------------------------------------
# Watchtower on a low grassy knoll in the south-east, tall enough to read far away.
watchtower(52, 50, base_w=5, height=22)
# Petrified tree in the north-west corner, silhouetted against the sky.
petrified_tree(10, 10, height=18)
# Ruined keep wall along the western edge, framing the village from the side.
ruined_keep_wall(4, 16, 8, 28, height=12)
# Fallen monolith in the south, giving the empty half a focal point.
fallen_monolith(32, 55, height=10)

# Sentinel Spire: the dominant vista in the north-east.  Rises behind the
# dungeon gate, directly in the player's forward view on waking (yaw 0 -> -Z).
# A paved terrace and spiral stair make it reachable on foot.
# Pedestal is two blocks tall so the courtyard around the shaft is walkable.
for sx in range(46, 51):
    for sz in range(2, 7):
        add(sx, 3, sz, "stone")
        add(sx, 4, sz, "stone")
sentinel_spire(48, 4, height=24, y_base=4)
# Torch-like markers leading from the terrace entrance to the spire base.
lantern_post(43, 7, y_base=2, height=2)
lantern_post(45, 5, y_base=2, height=2)

# Small framed arch near the bridge on the west side, a side-path teaser.
stone_arch(18, 14, y_base=1, height=4, span=2, width=1, axis="z")

# ---- 12. Southern fields / lived-in back half ------------------------------
# The old map had z>=44 empty; fill it with farmland, a pond, fences and ruins
# so the village feels like it continues past the playable path.
garden_plot(10, 46, 8, 10, y_base=0)
garden_plot(42, 44, 10, 8, y_base=0)
garden_plot(22, 56, 8, 6, y_base=0)

pond(18, 54, r=3)

fence(9, 45, 9, 55, y_base=0)
fence(10, 56, 21, 56, y_base=0)
fence(42, 44, 42, 51, y_base=0)
fence(43, 52, 51, 52, y_base=0)

ruin_house(52, 54, 6, 6, y_base=0, scattered=True, burned=False)
ruin_house(8, 56, 6, 6, y_base=0, scattered=True, burned=True)

# A low southern boundary wall/cliff to stop the world feeling infinite.
low_wall(0, 63, 60, 63, y_base=0, height=3)
low_wall(0, 0, 0, 63, y_base=0, height=2)

# ---- 13. Export ------------------------------------------------------------
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
