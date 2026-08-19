# -*- coding: utf-8 -*-
"""Generate maps/glass_demo.json — the map the glass before/after is shot on.

The scene is built around ONE frame, taken through the real gameplay camera:

    VOXELFORGE_MAP_LOAD=maps/glass_demo.json \
    VOXELFORGE_LOOK_CAM=0,-8,5 VOXELFORGE_SHOT=out.png

A loaded map spawns the avatar at the world centre (16, ~10.7, 16) and
`VOXELFORGE_LOOK_CAM=yaw,pitch,dist` poses the orbit boom. Yaw 0 puts the camera
at +Z looking toward -Z, so the wall at z = FRONT_Z is what fills the frame.

That wall is deliberately HALF STONE, HALF GLASS, split down the middle of the
view. One frame therefore contains the control and the change side by side: the
same light, the same camera, the same wall — one half you cannot see through and
one half you can. Behind it stand lantern pillars and bright snow/red-sand
blocks, which are visible through the right half and invisible through the left.

Run from the repo root:  python scripts/gen_glass_demo_map.py
"""
import json
import os

OUT = "maps/glass_demo.json"

# One 32x32x32 chunk; the loader hovers the avatar at y ~ 10.7 over its centre.
CHUNKS_X, CHUNKS_Z = 1, 1

FLOOR_Y = 6
WALL_Y0, WALL_Y1 = 7, 13  # inclusive

# The room the camera stands in.
X0, X1 = 8, 25
FRONT_Z, BACK_Z = 10, 23
# Off-centre on purpose: the avatar stands dead centre of the frame and is much
# closer to the lens than the wall, so a split at x=16 would be the one part of
# the wall the player's own body hides.
SPLIT_X = 13  # stone left of it, glass right of it


def main():
    blocks = []

    def put(x, y, z, name):
        blocks.append(dict(x=x, y=y, z=z, block=name))

    # ---- floor ------------------------------------------------------------
    for z in range(4, 28):
        for x in range(4, 28):
            # A two-tone floor so the ground is legible through the pane too.
            put(x, FLOOR_Y, z, "stone" if (x // 2 + z // 2) % 2 else "limestone")

    # ---- the room: three plain glass walls, one split wall in front --------
    for y in range(WALL_Y0, WALL_Y1 + 1):
        for x in range(X0, X1 + 1):
            # The frame's subject: same wall, two materials.
            put(x, y, FRONT_Z, "stone" if x < SPLIT_X else "glass")
            put(x, y, BACK_Z, "glass")
        for z in range(FRONT_Z, BACK_Z + 1):
            put(X0, y, z, "glass")
            put(X1, y, z, "glass")

    # ---- what stands behind the wall, and must only show through the glass -
    # Lantern pillars: emissive, so "visible / not visible" is unmistakable
    # even in a thumbnail.
    for x in range(X0 + 1, X1, 3):
        for y in range(WALL_Y0, WALL_Y1):
            put(x, y, FRONT_Z - 3, "lamp")
    # A bright band between and behind the pillars.
    for x in range(X0, X1 + 1):
        for y in range(WALL_Y0, WALL_Y0 + 4):
            put(x, y, FRONT_Z - 5, "snow" if (x + y) % 2 else "red_sand")
    # Foliage above the band, so the far plane is not one flat colour.
    for x in range(X0, X1 + 1):
        put(x, WALL_Y0 + 4, FRONT_Z - 5, "leaves")
        put(x, WALL_Y0 + 5, FRONT_Z - 5, "leaves")

    # ---- a lone glass pillar inside the room -------------------------------
    # Free-standing, so the frame also shows glass with nothing behind it —
    # the case where a wrong alpha mode reads as "the block vanished".
    for y in range(WALL_Y0, WALL_Y0 + 4):
        put(X0 + 3, y, FRONT_Z + 4, "glass")
        put(X1 - 3, y, FRONT_Z + 4, "wood")

    doc = dict(
        version=1,
        name="glass demo - stone|glass split wall, lanterns behind",
        size=dict(chunks_x=CHUNKS_X, chunks_z=CHUNKS_Z),
        blocks=blocks,
    )
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(doc, f, indent=2)
        f.write("\n")

    kinds = {}
    for b in blocks:
        kinds[b["block"]] = kinds.get(b["block"], 0) + 1
    print("wrote %s: %d blocks" % (OUT, len(blocks)))
    print("  " + ", ".join("%s=%d" % kv for kv in sorted(kinds.items())))


if __name__ == "__main__":
    main()
