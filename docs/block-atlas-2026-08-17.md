# Block texture atlas — how to reshoot it (2026-08-17, Poppy)

The atlas turns Voxelforge's blocks from flat `base_color` cubes into real
16×16 pixel-art materials. This is the whole operating manual: where the
contract lives, how to swap the art, and how to re-render the proof pair.

## 1. The contract

`assets/textures/blocks/atlas.json` — tiles (name → file) and kinds (block kind
→ which tile each face wears). Shipping a new art set is a folder drop plus a
JSON edit. **No Rust change, no rebuild.** Faces: `all` sets top+side+bottom,
`top`/`side`/`bottom` override it. Tiles must be seamless (they are packed with
a wrapped gutter) and square at `tile_px`.

Current set: 12 tiles / 10 kinds, art by Monanisa — see `PALETTE.md` beside it.

## 2. Modes — the A/B lever

`VOXELFORGE_ATLAS_MODE`, read once, applies to the whole atlas:

| value | what it does |
|---|---|
| `detail` (default) | tile normalised to unit mean luminance, multiplies the material's authored `base_color`. The signed-off grade survives; the block gains grain. |
| `albedo` / `raw` | the tile IS the colour; callers drop `base_color` to white. |
| `off` / `0` / `none` | no atlas at all — the flat materials that shipped before. |

`VOXELFORGE_ATLAS_DIR` repoints the folder (absolute path recommended; the
default `assets/textures/blocks` is relative to the *working* directory, because
the tiles are read with `std::fs` before Bevy's asset server exists).

## 3. Reshooting the outdoor pair

`voxelforge_atlasshot` is an isolated bin: it links `block_atlas.rs` and nothing
else, so it renders while any other lane is mid-edit. Built on `--profile perf`
(no fat LTO — a release link of this workspace is ~20 min, the perf link is ~2).

```sh
cargo build -p voxelforge --profile perf --bin voxelforge_atlasshot \
  --target-dir target-poppy

# from the repo root; ATLAS_DIR absolute so CWD cannot matter
VOXELFORGE_ATLAS_DIR=$PWD/assets/textures/blocks \
VOXELFORGE_ATLAS_MODE=off    VOXELFORGE_SHOT=$PWD/docs/assets/blocks/outdoor-blocks_before.png \
  target-poppy/perf/voxelforge_atlasshot

VOXELFORGE_ATLAS_DIR=$PWD/assets/textures/blocks \
VOXELFORGE_ATLAS_MODE=albedo VOXELFORGE_SHOT=$PWD/docs/assets/blocks/outdoor-blocks_after.png \
  target-poppy/perf/voxelforge_atlasshot
```

Each run prints `ATLAS_STAGE mode=… textured=…` and `ATLAS_STAGE blocks=…`.
`textured=false` on the after plate means the tiles were not found — the plate is
then a second before plate, not an after plate.

## 4. Why the pair is honest

Every flat colour in the stage (`atlas_shot_main.rs::flat_color`) is the
**measured mean sRGB of that kind's own tile**. Both plates therefore carry the
same palette and the only difference in the frame is texture detail. A before
plate painted in some other colour would be proving a repaint.

Measured on the committed pair (1280×720, 681 blocks, same exe, counted frame 90):

```
mean|d| = 6.17     37.5 % of pixels changed
mean RGB  before 125.4/160.9/158.1   after 125.3/159.5/158.0
```

Brightness held to ~1 LSB, as designed.

> Correction: the commit that landed the pair (`art(blocks): file-backed block
> texture atlas…`) quotes `mean|d| 6.09 / 37.2 %`. Those are the numbers of an
> earlier take, shot before the window moved to the camera-facing wall and the
> interior floor became a porch deck. The committed PNGs are the later take and
> the figures above are theirs.

## 5. What the stage proves, and what it does not

Visible in the after plate: grass (top **and** side — the per-face proof),
sand, stone bricks, plaster, plank, floorboard, oak log, roof tile, leaves,
glass. All 10 kinds.

Not proven here:

* **Glass is opaque.** `glass.png` has alpha = 1.00 across the tile, and the
  material is not in a transparent alpha mode. The pane reads as a pale blue
  block, not as something you can see through.
* **Gameplay chunks are untouched.** The atlas is wired into `hero.rs` (and this
  showcase). The greedy-meshed world in `voxel.rs` / `scene.rs` still builds its
  own procedural atlas — wiring per-face UVs through the mesher is the voxel
  material lane's call, not this one.
