#!/usr/bin/env python3
"""Generate the Edhari village voxel map for Voxelforge."""
import json
import math
from pathlib import Path

WIDTH = 64
DEPTH = 64
CHUNKS_X = 2
CHUNKS_Z = 2
CX = 31
CZ = 31

blocks = {}


def add(x, y, z, block):
    if not (0 <= x < WIDTH and 0 <= z < DEPTH and 0 <= y < 32):
        raise ValueError(f"out of bounds: ({x},{y},{z})")
    blocks[(x, y, z)] = block


# Grass floor for the whole village
for z in range(DEPTH):
    for x in range(WIDTH):
        add(x, 0, z, "grass")

# Central X-shaped paths (stone on one diagonal, sand on the other)
for i in range(WIDTH):
    add(i, 0, i, "stone")
    add(i, 0, DEPTH - 1 - i, "sand")

# Campfire: small stone ring in the middle of the square
for dx in range(-2, 3):
    for dz in range(-2, 3):
        dist = math.sqrt(dx * dx + dz * dz)
        if 1.5 < dist <= 2.5:
            add(CX + dx, 1, CZ + dz, "stone")


def house(x0, z0, w, d, door_x, door_z, door_face, height=5):
    """Add a simple hollow house with walls of stone + dirt."""
    x1 = x0 + w - 1
    z1 = z0 + d - 1
    for y in range(1, height + 1):
        block = "dirt" if y == height else "stone"
        for x in range(x0, x1 + 1):
            for z in range(z0, z1 + 1):
                # only perimeter walls
                if not (x == x0 or x == x1 or z == z0 or z == z1):
                    continue
                # door opening (2 voxels high)
                if y <= 2 and x == door_x and z == door_z:
                    continue
                add(x, y, z, block)


# 5 houses around the central square, leaving a 5x5 open spawn area at the centre
house(27, 14, 7, 7, 30, 20, "south")      # north
house(27, 38, 7, 7, 30, 38, "north")      # south
house(38, 27, 7, 7, 38, 30, "west")       # east
house(14, 27, 7, 7, 20, 30, "east")       # west
house(38, 14, 7, 7, 41, 20, "south")      # north-east

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
