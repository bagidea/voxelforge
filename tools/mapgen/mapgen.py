#!/usr/bin/env python3
"""Voxelforge map generator — spec (or a line of natural language) -> maps/<name>.json.

The engine's map format (see maps/FORMAT.md) is just: a world size in chunks plus an
explicit list of solid blocks. That makes it trivial for an AI agent to author a level:
describe the structures you want as a small JSON *spec*, and this tool stamps them into
voxels and emits a schema-valid map the game loads with skipped=0.

Two front doors:
  * a spec file           : mapgen.py --spec spec.json --out maps/foo.json
  * a preset + size       : mapgen.py --preset castle --size 20 --out maps/castle.json
  * a line of plain words : mapgen.py --nl "castle 20x20" --out maps/castle.json

The spec is the real interface (an LLM can write it directly); --preset / --nl are thin
conveniences over it. See tools/mapgen/README.md for the primitive catalogue.
"""

import argparse
import json
import math
import re
import sys

CHUNK = 32                 # one chunk = 32 voxels on a side (matches the sim crate)
Y_MAX = CHUNK              # world is a single 32-tall layer: y in [0, 32)
KNOWN_BLOCKS = {"grass", "dirt", "stone", "sand"}   # air is implicit, never emitted


# ---------------------------------------------------------------------------
# Voxel canvas: a dict keyed (x,y,z) -> block name. Later stamps overwrite earlier
# ones, and setting a voxel to None (air) carves it back out — that is how a gate
# opening is punched through a wall we already built.
# ---------------------------------------------------------------------------
class Canvas:
    def __init__(self):
        self.vox = {}

    def set(self, x, y, z, block):
        if block is None:
            self.vox.pop((x, y, z), None)
        else:
            self.vox[(x, y, z)] = block

    def fill(self, x0, z0, w, d, y0, y1, block):
        """Solid box: [x0, x0+w) x [z0, z0+d) x [y0, y1] inclusive on y."""
        for x in range(x0, x0 + w):
            for z in range(z0, z0 + d):
                for y in range(y0, y1 + 1):
                    self.set(x, y, z, block)

    def perimeter(self, x0, z0, w, d, y0, y1, block):
        """Hollow box wall: the four edge columns of a w x d footprint."""
        for x in range(x0, x0 + w):
            for z in range(z0, z0 + d):
                on_edge = x in (x0, x0 + w - 1) or z in (z0, z0 + d - 1)
                if on_edge:
                    for y in range(y0, y1 + 1):
                        self.set(x, y, z, block)


# ---------------------------------------------------------------------------
# Primitives — each takes the canvas + a structure dict and stamps voxels.
# ---------------------------------------------------------------------------
def prim_floor(cv, s):
    """floor / arena: a flat rectangle of one block at a given y (default 0)."""
    ox, oz = s.get("at", [0, 0])
    w, d = s["size"]
    y = s.get("y", 0)
    cv.fill(ox, oz, w, d, y, y, s.get("block", "grass"))


def prim_walls(cv, s):
    """walls: a hollow rectangular enclosure of the given height (no roof, no floor)."""
    ox, oz = s.get("at", [0, 0])
    w, d = s["size"]
    y0 = s.get("y", 0) + 1                      # sit the wall on top of y0 (the floor)
    h = s.get("height", 4)
    cv.perimeter(ox, oz, w, d, y0, y0 + h - 1, s.get("block", "stone"))


def prim_tower(cv, s):
    """tower: a solid square column w x d, `height` tall — reads as a keep/turret."""
    ox, oz = s.get("at", [0, 0])
    w, d = s.get("size", [3, 3])
    y0 = s.get("y", 0) + 1
    h = s.get("height", 8)
    cv.fill(ox, oz, w, d, y0, y0 + h - 1, s.get("block", "stone"))


def prim_pyramid(cv, s):
    """pyramid: a stepped square pyramid, `base` wide, shrinking one ring per course."""
    ox, oz = s.get("at", [0, 0])
    base = s.get("base", s.get("size", [8])[0])
    y0 = s.get("y", 0)
    block = s.get("block", "sand")
    layers = (base + 1) // 2                    # until the apex is 1 (odd) or 2 (even)
    for i in range(layers):
        side = base - 2 * i
        if side <= 0:
            break
        cv.fill(ox + i, oz + i, side, side, y0 + i, y0 + i, block)


def prim_castle(cv, s):
    """castle: grass floor + 4 stone walls + 4 corner towers + a carved gate.

    A composite of the primitives above, tuned so the walls read from a distance and
    the courtyard is walkable. Crenellations (alternating merlons) crown the walls.
    """
    ox, oz = s.get("at", [0, 0])
    w, d = s["size"]
    wall_h = s.get("wall_height", 5)
    tower_h = s.get("tower_height", wall_h + 3)
    t = s.get("tower_size", 3)                  # corner tower footprint (t x t)
    gate = s.get("gate", "south")               # which wall the doorway breaks
    gate_w = s.get("gate_width", 3)
    gate_h = s.get("gate_height", 4)

    # Floor (grass) across the whole footprint.
    cv.fill(ox, oz, w, d, 0, 0, "grass")
    # Curtain walls (stone) on the perimeter, y = 1..wall_h.
    cv.perimeter(ox, oz, w, d, 1, wall_h, "stone")
    # Crenellations: knock out every other block along the wall top for a merlon row.
    for x in range(ox, ox + w):
        for z in range(oz, oz + d):
            if x in (ox, ox + w - 1) or z in (oz, oz + d - 1):
                if (x + z) % 2 == 1:
                    cv.set(x, wall_h, z, None)
    # Four corner towers (stone), taller than the wall.
    corners = [
        (ox, oz),
        (ox + w - t, oz),
        (ox, oz + d - t),
        (ox + w - t, oz + d - t),
    ]
    for cx, cz in corners:
        cv.fill(cx, cz, t, t, 1, tower_h, "stone")
    # Gate: carve an opening through the middle of the chosen wall (floor stays).
    _carve_gate(cv, ox, oz, w, d, gate, gate_w, gate_h)


def _carve_gate(cv, ox, oz, w, d, side, gw, gh):
    if side in ("south", "north"):
        z = oz + d - 1 if side == "south" else oz
        cx = ox + (w - gw) // 2
        for x in range(cx, cx + gw):
            for y in range(1, gh + 1):
                cv.set(x, y, z, None)
    else:  # east / west
        x = ox + w - 1 if side == "east" else ox
        cz = oz + (d - gw) // 2
        for z in range(cz, cz + gw):
            for y in range(1, gh + 1):
                cv.set(x, y, z, None)


PRIMITIVES = {
    "floor": prim_floor,
    "arena": prim_floor,
    "walls": prim_walls,
    "tower": prim_tower,
    "pyramid": prim_pyramid,
    "castle": prim_castle,
}


# ---------------------------------------------------------------------------
# Spec -> MapFile
# ---------------------------------------------------------------------------
def build(spec):
    """Stamp every structure in a spec onto a canvas and return a MapFile dict."""
    cv = Canvas()
    for s in spec.get("structures", []):
        kind = s.get("type")
        fn = PRIMITIVES.get(kind)
        if fn is None:
            raise ValueError(f"unknown structure type: {kind!r} "
                             f"(known: {', '.join(sorted(PRIMITIVES))})")
        fn(cv, s)

    blocks = []
    max_x = max_z = 0
    for (x, y, z), block in cv.vox.items():
        # Hard schema guarantees so the game loads with skipped=0.
        if block not in KNOWN_BLOCKS:
            raise ValueError(f"block {block!r} at ({x},{y},{z}) is not a known block")
        if not (0 <= y < Y_MAX):
            raise ValueError(f"y={y} at ({x},{z}) out of range [0,{Y_MAX})")
        if x < 0 or z < 0:
            raise ValueError(f"negative coord ({x},{y},{z}) — the world starts at 0")
        max_x, max_z = max(max_x, x), max(max_z, z)
        blocks.append({"x": x, "y": y, "z": z, "block": block})

    # Size in chunks: honour an explicit value, else grow to cover the geometry.
    if "size_chunks" in spec:
        cx, cz = spec["size_chunks"]
    else:
        cx = max(1, math.ceil((max_x + 1) / CHUNK))
        cz = max(1, math.ceil((max_z + 1) / CHUNK))

    # Anything past the declared size would be skipped on load — refuse to emit it.
    for b in blocks:
        if b["x"] >= cx * CHUNK or b["z"] >= cz * CHUNK:
            raise ValueError(f"block ({b['x']},{b['z']}) falls outside size "
                             f"{cx}x{cz} chunks — bump size_chunks")

    blocks.sort(key=lambda b: (b["y"], b["z"], b["x"]))
    return {
        "version": 1,
        "name": spec.get("name", ""),
        "size": {"chunks_x": cx, "chunks_z": cz},
        "blocks": blocks,
    }


# ---------------------------------------------------------------------------
# Convenience front doors: preset + a tiny natural-language parser.
# ---------------------------------------------------------------------------
def preset_spec(preset, size, at=None):
    """Turn a preset name + a size into a full spec, centred in a single chunk."""
    if at is None:
        # Centre the structure in a 1-chunk world when it fits, else at the origin.
        at = [(CHUNK - size) // 2, (CHUNK - size) // 2] if size < CHUNK else [0, 0]
    name = f"{preset}-{size}x{size}"
    if preset in ("castle",):
        st = {"type": "castle", "size": [size, size], "at": at}
    elif preset in ("arena", "floor"):
        st = {"type": "floor", "size": [size, size], "at": at}
    elif preset in ("walls",):
        st = {"type": "walls", "size": [size, size], "at": at, "height": 4}
    elif preset in ("tower",):
        st = {"type": "tower", "size": [max(3, size // 4), max(3, size // 4)],
              "at": at, "height": size}
    elif preset in ("pyramid",):
        st = {"type": "pyramid", "base": size, "at": at}
    else:
        raise ValueError(f"unknown preset: {preset!r}")
    return {"name": name, "structures": [st]}


def parse_nl(text):
    """Map a short line of words to a preset spec. Understands e.g. 'castle 20x20',
    'ปราสาท 20', 'pyramid base 16', 'a 24 wide arena'. This is deliberately small —
    for anything richer, write the spec JSON (an LLM does this well)."""
    t = text.lower()
    keyword_map = {
        "castle": ["castle", "ปราสาท", "fort", "keep"],
        "tower": ["tower", "หอคอย", "หอ", "turret"],
        "pyramid": ["pyramid", "พีระมิด", "ปิรามิด"],
        "arena": ["arena", "floor", "พื้น", "ลาน", "สนาม"],
        "walls": ["wall", "กำแพง", "enclosure"],
    }
    preset = None
    for name, words in keyword_map.items():
        if any(w in t for w in words):
            preset = name
            break
    if preset is None:
        raise ValueError(f"could not find a structure keyword in {text!r} "
                         f"(try: castle, tower, pyramid, arena, walls)")
    # Size: first "NxN" wins, else the first standalone number, else a default.
    m = re.search(r"(\d+)\s*[x×]\s*(\d+)", t)
    if m:
        size = max(int(m.group(1)), int(m.group(2)))
    else:
        nums = re.findall(r"\d+", t)
        size = int(nums[0]) if nums else 20
    size = max(4, min(size, CHUNK))            # keep it inside a single chunk
    return preset_spec(preset, size)


# ---------------------------------------------------------------------------
def main(argv=None):
    ap = argparse.ArgumentParser(description="Voxelforge map generator")
    src = ap.add_mutually_exclusive_group(required=True)
    src.add_argument("--spec", help="path to a spec JSON file")
    src.add_argument("--preset", help="preset name: castle|arena|walls|tower|pyramid")
    src.add_argument("--nl", help="a short line of natural language, e.g. 'castle 20x20'")
    ap.add_argument("--size", type=int, default=20, help="footprint size for --preset")
    ap.add_argument("--at", help="origin 'x,z' override for --preset")
    ap.add_argument("--out", required=True, help="output map path, e.g. maps/castle.json")
    args = ap.parse_args(argv)

    if args.spec:
        with open(args.spec, "r", encoding="utf-8") as f:
            spec = json.load(f)
    elif args.preset:
        at = [int(v) for v in args.at.split(",")] if args.at else None
        spec = preset_spec(args.preset, args.size, at)
    else:
        spec = parse_nl(args.nl)

    m = build(spec)
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(m, f, indent=2)
        f.write("\n")

    print(f"OK wrote {args.out}  name={m['name']!r}  "
          f"size={m['size']['chunks_x']}x{m['size']['chunks_z']} chunks  "
          f"blocks={len(m['blocks'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
