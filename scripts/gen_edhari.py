#!/usr/bin/env python3
"""Generate the Edhari village voxel map for Voxelforge.

Layout follows the design docs, not just a placeholder box grid:
- docs/GAME-VISION.md: "1 hand-crafted region (village ruins + 1 dungeon hub),
  ~48x48 block footprint".
- docs/first-playable-loop.md (Act 0/Act 1 environment table): a crumbling
  stone shelter at spawn, a lit campfire ~3 blocks ahead, collapsed houses
  (2-3 with intact interiors) at 6-20m, a village well as the centre
  landmark (~15m), a sealed dungeon-gate arch 30m north.

Two things this file leans on that are NOT in the map schema (maps/FORMAT.md
has no spawn/entity fields — a map is only version/name/size/blocks):
1. Spawn position is derived by the engine, not stored here. `boot_scene()`
   in client/src/scene.rs computes `c = side * CHUNK / 2` and calls
   `map_spawn(&world, c, c)`, which spirals outward from that voxel looking
   for the nearest standable column. For a 2x2-chunk (64x64) world that
   fixed point is (32, 32) — everything below is built relative to it.
2. Yaw 0 (the boot orientation) faces -Z ("fire goes straight ahead" per the
   comment next to `FIRE_AHEAD` in scene.rs), so -Z is "north" throughout
   this generator, and the real campfire mesh is spawned by the client
   itself 3 blocks north of spawn (FIRE_AHEAD = 3.0) — matched below so the
   paved plaza lines up with where the engine actually lights the fire.

No entity/skeleton/sigil/story-fragment content is emitted: the schema is
blocks-only, so those beats from first-playable-loop.md are left for
whatever entity system eventually carries them (see report to Poppy/CEO).
"""
import json
import math
import random
from pathlib import Path

WIDTH = 64
DEPTH = 64
CHUNKS_X = 2
CHUNKS_Z = 2

# Must match client/src/scene.rs boot_scene(): c = world_side(&world) * CHUNK / 2
SPAWN_X, SPAWN_Z = 32, 32

# Fixed seed: the "collapsed" wall skylines are randomised but reproducible —
# regenerating this file twice must not silently change the map.
rng = random.Random(20260730)

blocks = {}


def add(x, y, z, block):
    if not (0 <= x < WIDTH and 0 <= z < DEPTH and 0 <= y < 32):
        raise ValueError(f"out of bounds: ({x},{y},{z})")
    blocks[(x, y, z)] = block


def floor_all(block="grass"):
    for z in range(DEPTH):
        for x in range(WIDTH):
            add(x, 0, z, block)


def paved_rect(x0, x1, z0, z1, block):
    for z in range(z0, z1 + 1):
        for x in range(x0, x1 + 1):
            add(x, 0, z, block)


def paved_disc(cx, cz, r, block):
    for dz in range(-r, r + 1):
        for dx in range(-r, r + 1):
            if dx * dx + dz * dz <= r * r:
                x, z = cx + dx, cz + dz
                if 0 <= x < WIDTH and 0 <= z < DEPTH:
                    add(x, 0, z, block)


def intact_house(x0, z0, w, d, door_x, door_z, height=4):
    """Hollow perimeter house, one door gap — the '2-3 intact interiors'."""
    x1, z1 = x0 + w - 1, z0 + d - 1
    for y in range(1, height + 1):
        block = "dirt" if y == height else "stone"
        for x in range(x0, x1 + 1):
            for z in range(z0, z1 + 1):
                if not (x == x0 or x == x1 or z == z0 or z == z1):
                    continue
                if y <= 2 and x == door_x and z == door_z:
                    continue
                add(x, y, z, block)


def ruin_house(x0, z0, w, d):
    """Collapsed house: perimeter wall with a random broken skyline + rubble."""
    x1, z1 = x0 + w - 1, z0 + d - 1
    for x in range(x0, x1 + 1):
        for z in range(z0, z1 + 1):
            if not (x == x0 or x == x1 or z == z0 or z == z1):
                continue
            stub = rng.choices([0, 1, 2, 3], weights=[35, 30, 25, 10])[0]
            for y in range(1, stub + 1):
                add(x, y, z, "dirt" if y == stub else "stone")
    for _ in range((w * d) // 6):
        rx = rng.randint(x0 + 1, x1 - 1)
        rz = rng.randint(z0 + 1, z1 - 1)
        add(rx, 1, rz, rng.choice(["stone", "dirt"]))


def spawn_shelter(cx, cz, r=3):
    """Crumbling stone shelter around spawn — no north wall (collapsed, open
    onto the campfire/courtyard), low and gap-toothed on the other 3 sides."""
    x0, x1, z0, z1 = cx - r, cx + r, cz - r, cz + r
    for x in range(x0, x1 + 1):
        for z in range(z0, z1 + 1):
            if not (x == x0 or x == x1 or z == z0 or z == z1):
                continue
            if z == z0:
                continue  # collapsed wall faces -Z (north) — open to the village
            stub = rng.choices([1, 2, 3], weights=[30, 45, 25])[0]
            for y in range(1, stub + 1):
                add(x, y, z, "stone")


def village_well(cx, cz, r=2):
    """Raised stone curb + a dry dirt shaft floor — the centre landmark."""
    for dz in range(-r, r + 1):
        for dx in range(-r, r + 1):
            d = math.hypot(dx, dz)
            x, z = cx + dx, cz + dz
            if not (0 <= x < WIDTH and 0 <= z < DEPTH):
                continue
            if r - 1 < d <= r:
                add(x, 1, z, "stone")
            elif d <= r - 1:
                add(x, 0, z, "dirt")


def dungeon_gate(cx, z_near, z_far, half_span=5, height=7):
    """Two stone pillars + lintel, sealed with a sand 'sigil' accent block —
    'large stone arch, sealed; glowing sigil above the door'."""
    x_out_l, x_in_l = cx - half_span, cx - half_span + 1
    x_in_r, x_out_r = cx + half_span - 1, cx + half_span
    for z in range(z_near, z_far + 1):
        for x in (x_out_l, x_in_l, x_in_r, x_out_r):
            for y in range(1, height + 1):
                add(x, y, z, "stone")
    # sealed door: solid fill between the pillars, well short of the lintel
    for x in range(x_in_l + 1, x_in_r):
        for z in range(z_near, z_far + 1):
            for y in range(1, height - 2):
                add(x, y, z, "stone")
    # lintel spanning the full arch width
    for x in range(x_out_l, x_out_r + 1):
        for z in range(z_near, z_far + 1):
            add(x, height + 1, z, "stone")
    # the "glowing sigil" — a single pale accent block, above the door and on
    # the z_far (+Z) face of the lintel, i.e. the side facing the village/
    # spawn (-Z is "north"/into the gate per the module docstring). Centring
    # it mid-slab instead (z_near+z_far)//2 buries it — both z-neighbours are
    # then lintel stone too, so it has no face a player standing in the
    # village can ever see.
    add(cx, height + 1, z_far, "sand")


floor_all("grass")

# Main street: spawn shelter -> campfire plaza -> village well -> dungeon gate.
paved_rect(SPAWN_X - 2, SPAWN_X + 2, 6, SPAWN_Z - 1, "stone")

village_well(SPAWN_X, 17)  # ~15 blocks north of spawn, per first-playable-loop.md
dungeon_gate(SPAWN_X, z_near=3, z_far=5)  # ~29 blocks north — "30 m north"

# Campfire plaza — matches FIRE_AHEAD=3.0 in scene.rs exactly, so the paving
# lines up with where the client actually spawns the (procedural) campfire.
paved_disc(SPAWN_X, SPAWN_Z - 3, 3, "stone")

spawn_shelter(SPAWN_X, SPAWN_Z, r=3)

# 2 intact-interior houses (flanking the well, off the main street)
intact_house(14, 22, 6, 6, door_x=19, door_z=24, height=4)
intact_house(44, 22, 6, 6, door_x=44, door_z=24, height=4)

# 4 collapsed ruins (village edge, 6-20m band, clear of the street/gate/well)
ruin_house(20, 14, 6, 6)
ruin_house(40, 14, 6, 6)
ruin_house(18, 38, 6, 6)
ruin_house(40, 38, 6, 6)

out = {
    "version": 1,
    "name": "edhari",
    "size": {"chunks_x": CHUNKS_X, "chunks_z": CHUNKS_Z},
    "blocks": [{"x": x, "y": y, "z": z, "block": b} for (x, y, z), b in blocks.items()],
}

# Write next to the script's project root so it works from any cwd
project_root = Path(__file__).resolve().parent.parent
out_path = project_root / "maps" / "edhari.json"
out_path.parent.mkdir(parents=True, exist_ok=True)
with open(out_path, "w", encoding="utf-8") as f:
    json.dump(out, f)

print(f"wrote {len(out['blocks'])} blocks to {out_path}")
