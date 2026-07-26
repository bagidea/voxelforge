# mapgen — Voxelforge map generator

Turn a **spec** (or a line of plain words) into a schema-valid `maps/<name>.json` that
the game loads with `skipped=0`. The map format itself is documented in
[`maps/FORMAT.md`](../../maps/FORMAT.md); this tool just stamps structures into voxels
so nobody has to place blocks by hand.

Pure Python 3, no dependencies.

## Three ways to call it

```bash
# 1. A preset + a size (the demo the CEO asked for: a 20×20 castle)
python tools/mapgen/mapgen.py --preset castle --size 20 --out maps/castle.json

# 2. A line of natural language (a few keywords + a size)
python tools/mapgen/mapgen.py --nl "castle 20x20" --out maps/castle.json
python tools/mapgen/mapgen.py --nl "ปราสาท 20"    --out maps/castle.json

# 3. A spec JSON — the real interface; an AI writes this directly
python tools/mapgen/mapgen.py --spec my_spec.json --out maps/my_level.json
```

Every run prints one line and exits `0` on success:

```
OK wrote maps/castle.json  name='castle-20x20'  size=1x1 chunks  blocks=928
```

## The spec

A spec is a list of **structures** stamped in order (later blocks overwrite earlier
ones — that is how a gate opening is carved through a wall already built):

```json
{
  "name": "my-level",
  "size_chunks": [1, 1],
  "structures": [
    { "type": "floor",  "size": [20, 20], "at": [6, 6], "block": "grass" },
    { "type": "castle", "size": [20, 20], "at": [6, 6], "gate": "south" }
  ]
}
```

- `size_chunks` is optional — omit it and the tool grows the world to cover the geometry.
- `at` is the `[x, z]` origin of a structure (defaults to `[0, 0]`).
- All coordinates are **world-voxel** units; one chunk is 32 voxels, `y ∈ [0, 32)`.

## Primitives

| type              | key fields                                                       | what it stamps |
| ----------------- | --------------------------------------------------------------- | -------------- |
| `floor` / `arena` | `size [w,d]`, `at`, `y=0`, `block=grass`                         | a flat rectangle |
| `walls`           | `size [w,d]`, `at`, `height=4`, `block=stone`                    | a hollow enclosure (no roof/floor) |
| `tower`           | `size [w,d]=[3,3]`, `at`, `height=8`, `block=stone`              | a solid square column |
| `pyramid`         | `base`, `at`, `block=sand`                                       | a stepped square pyramid |
| `castle`          | `size [w,d]`, `at`, `wall_height=5`, `tower_height=wall+3`, `tower_size=3`, `gate=south`, `gate_width=3`, `gate_height=4` | floor + 4 curtain walls (crenellated) + 4 corner towers + a carved gate |

Block names: `grass`, `dirt`, `stone`, `sand` (air is implicit — never listed).

`gate` accepts `north` / `south` / `east` / `west`.

## Guarantees

`build()` refuses to emit anything the loader would skip:

- unknown block name → error
- `y` outside `[0, 32)` → error
- a block past the declared `size_chunks` → error (bump the size)
- duplicate voxels are collapsed (last write wins)

So a file that comes out of this tool is guaranteed to load `skipped=0`.

## For agents

The intended flow for "make me a map from words": an agent reads the request, writes a
**spec JSON**, and runs `mapgen.py --spec`. The `--preset` / `--nl` doors are shortcuts
for the common cases; the spec is where the real expressiveness lives (multiple
structures, exact placement, mixed block types). You can also skip this tool entirely
and hand-write the map JSON per `maps/FORMAT.md` — this generator just saves the tedium.
