# Voxelforge map format (`.json`)

A map is a **plain-text JSON file** that fully describes a voxel world: how big it
is, and every solid block in it. It is designed so a **human or an AI agent can read,
edit, or generate one from scratch** with no tooling — just write JSON.

The engine reads these files (load) and writes them (save); nothing else is needed to
build a level.

---

## Schema

```json
{
  "version": 1,
  "name": "my-first-map",
  "size": { "chunks_x": 1, "chunks_z": 1 },
  "blocks": [
    { "x": 0, "y": 0, "z": 0, "block": "grass" },
    { "x": 1, "y": 0, "z": 0, "block": "grass" },
    { "x": 5, "y": 1, "z": 5, "block": "stone" }
  ]
}
```

| Field            | Type   | Meaning                                                              |
| ---------------- | ------ | ------------------------------------------------------------------- |
| `version`        | int    | Schema version. Always `1` for now.                                 |
| `name`           | string | Free label shown in the HUD. Optional (defaults to empty).          |
| `size.chunks_x`  | int    | World width in **chunks** along X. Each chunk is 32 voxels wide.    |
| `size.chunks_z`  | int    | World depth in **chunks** along Z.                                  |
| `blocks`         | array  | Every **solid** block. Order does not matter.                       |
| `blocks[].x/y/z` | int    | **World-voxel** coordinates (not chunk-local).                      |
| `blocks[].block` | string | Block type name (see below).                                        |

### Coordinate system

- `x`, `z` are the horizontal ground plane; `y` is **up**.
- One voxel = one unit. Voxel `(x,y,z)` occupies the cube from `(x,y,z)` to
  `(x+1,y+1,z+1)`.
- A world of `chunks_x × chunks_z` chunks spans voxels
  `x ∈ [0, chunks_x*32)`, `z ∈ [0, chunks_z*32)`.
- `y` must be in `[0, 32)` — the world is a single 32-tall layer. Blocks outside
  the size or the y-range are **skipped on load** (reported in the console, never
  silently applied).

### Block names

| Name    | Id | Notes                        |
| ------- | -- | ---------------------------- |
| `air`   | 0  | Empty. **Never written** — see below. |
| `grass` | 1  | Green top surface.           |
| `dirt`  | 2  | Brown.                       |
| `stone` | 3  | Grey.                        |
| `sand`  | 4  | Pale.                        |

Names are case-insensitive on load.

### Air is implicit

The `blocks` list holds **only solid blocks**. Any voxel you do not list is air.
That is what keeps hand-written maps tiny: to make a floating stone block you write
exactly one entry, not a chunk full of `air`.

---

## Writing a map by hand (or from an agent)

Minimum viable map — a 2×1 grass strip:

```json
{
  "version": 1,
  "name": "strip",
  "size": { "chunks_x": 1, "chunks_z": 1 },
  "blocks": [
    { "x": 10, "y": 0, "z": 10, "block": "grass" },
    { "x": 11, "y": 0, "z": 10, "block": "grass" }
  ]
}
```

Tips for generating maps programmatically:

- Emit a floor with a nested loop over `x` and `z` at `y = 0`.
- Stack blocks by increasing `y` (a tower is the same `x,z` at `y = 1,2,3,…`).
- Keep everything inside `x,z ∈ [0, chunks_x*32) × [0, chunks_z*32)` and `y ∈ [0,32)`.

---

## Loading & saving in the game

- **Save** (`F5` in-game, or `VOXELFORGE_MAP_SAVE=<path>` headless) writes the live
  world to JSON, creating the `maps/` folder if needed. Only solid blocks are saved.
- **Load** (`F9` in-game, or `VOXELFORGE_MAP_LOAD=<path>` headless) rebuilds the world
  from the file: it spawns empty chunks for the declared size, then stamps every
  listed block. **No procedural terrain is generated** — the file is the whole world.

A loaded map is fully playable: you can walk on it, and the physics/collision use the
saved geometry directly.
