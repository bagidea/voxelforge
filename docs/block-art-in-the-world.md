# The block art reaches the world

*Poppy, 2026-08-17 — branch `poppy/native-only`*

Two changes, one theme: the art an artist can edit is now what the **gameplay**
world is painted with, and one of those tiles is a pane you can see through.

Before this pass the file-backed art set (`assets/textures/blocks/`, read by
`client/src/block_atlas.rs`) reached exactly one thing: the hero/beauty scene.
The chunks a player actually walks on were painted by `voxel.rs::tile_shade`, a
`match` compiled into the binary. Two art sets, one of them unreachable from the
game, and no file an artist could replace to change what the world looks like.

---

## 1. One source of tiles, two ways to address them

`block_atlas.rs` grew a lower-level entry point:

```rust
pub fn load_tiles(dir: Option<&Path>) -> Result<Option<TileSet>, String>
```

`TileSet` is the manifest decoded but **not yet packed**: the raw RGBA tiles plus
the kind→(top, side, bottom) mapping. `block_atlas::load` is now a thin wrapper
over it — it packs into the gutter-cell atlas exactly as before, so the hero
scene renders byte-identically.

The chunk mesher takes the other fork. It cannot address a window inside a
packed atlas, because its UVs are measured **in blocks**: a greedy-merged 12×3
quad runs its UV from 0 to 12, so the texture has to repeat twelve times, and no
address mode repeats a sub-rectangle. So the near path bakes one standalone
`Repeat`-sampled texture per (block, face) from the same tiles.

That is the whole split: **one source of art, two ways to address it.**

## 2. How a block gets file art

`voxel.rs::atlas_kind` is one line — `id.name()`. A gameplay block gets the
artist's tiles the moment `atlas.json` grows a `kinds` entry **named after the
sim block**, and falls back to its procedural tile when one does not exist.
There is deliberately no Rust-side alias table: that would be a second contract
to keep in sync with the manifest, and the two would drift.

Wired in this pass (added to `atlas.json`): `grass`, `sand`, `stone`, `wood`,
`leaves`, `limestone`, `glass`. Still procedural: `dirt`, `snow`, `red_sand`,
`clay`, `gravel`, `cobblestone`, `obsidian`, `brick`, `moss`, `lamp`.

Two consequences worth stating out loud:

* **`stone` currently borrows `stone_bricks`.** It reads as masonry, not natural
  rock. Drop a `stone_rough.png` in the folder and repoint the kind — no Rust
  change, no rebuild.
* **The relief follows the art.** `face_height` reads the *file tile's own
  luminance* when a face is file-backed, so the normal and roughness maps are
  derived from the texture you can see rather than from a procedural pattern you
  can't. A normal map that disagrees with its own albedo is what reads as
  plastic.

## 3. Per-face UVs through the greedy mesher, at no merge cost

A grass block needs three tiles. One mesh carries one material carries one
texture, so a block with distinct faces is three meshes.

The greedy mesher was **already** emitting a quad per (block, face): a mask cell
holds a signed block id, the sweep axis is fixed for the whole pass, and the
merge compares mask cells for equality. A merged quad therefore can never
straddle two block types or two face directions. Splitting the bucket that way
costs nothing in merge quality — no `w`/`h` run got shorter.

It would have cost **draw calls**, though: six block types in a chunk would have
gone from six children to as many as eighteen. So `face_slot` folds all three
faces back onto `Side` for any block whose manifest entry gives its faces the
same tile — which is every procedural block and most file-backed ones. Grass and
logs pay for their tops. Nothing else pays anything.

The far-LOD atlas (`build_atlas`) got the same treatment: one column per
(block, face), filled from the same `face_texels`. The LOD ring sits directly
behind the near ring in every frame, and two art sets meeting at that boundary
is a visible line across the world.

## 4. Glass

`BlockId::GLASS` (id 17) is the first block that is **solid but not opaque**, and
that sentence is the whole feature. `is_opaque` used to mean "not air"; it now
means "hides what is behind it", and a second predicate `is_solid` carries the
old meaning to the callers that actually wanted it — collision, map saving,
spawn clearance, the LOD downsample.

In the mesher, the face rule became:

```rust
let show_a = a.is_solid() && !hides(b) && a != b;
let show_b = b.is_solid() && !hides(a) && a != b;
```

which draws the stone face *behind* a pane (a naive `is_opaque` neighbour test
culls it, and the see-through becomes a see-through onto a hole), and draws no
faces inside a run of glass (`a != b`).

The transparency itself is in the **texture's alpha channel**, not in a
material-wide fade. `scripts/glass_alpha.py` derives the alpha from the tile's
own luminance bands — the dark came/leading stays at 245, the specular glint at
190, the pane between them drops to 46 — so the leading and the highlight stay
solid while the glass between them does not. A uniform `base_color` alpha would
dim the leading along with the pane and read as a ghost block.

`AlphaMode::Blend`, not `Mask`: the pane is genuinely semi-transparent, and
`Mask` can only give a binary hole. `double_sided: false` stays, or a pane
double-blends with its own back face.

### The A/B lever

`VOXELFORGE_GLASS_OPAQUE=1` renders glass exactly the way it rendered before it
had an alpha channel. It has to reach further than the other look levers:
transparency is not only a material setting, it changes which faces the *mesher*
emits. So `voxel.rs::hides` folds the flag in, and both the mesher and the
material read the same `OnceLock` — a mesher and a material that disagree about
whether glass is opaque produce a chunk with holes in it.

---

## Evidence

Both pairs come out of **one binary** (`client/target-poppy/debug/voxelforge.exe`),
same seed, same camera; the only difference between the two runs of a pair is a
single environment variable. Shot by `scripts/_poppy_blockart_shoot.ps1`,
measured by `scripts/_poppy_pair_metrics.py`.

<!-- MEASUREMENTS -->

## Files

| file | what changed |
| --- | --- |
| `client/src/block_atlas.rs` | `load_tiles` / `TileSet` / `Face`; `load` refactored onto them |
| `client/src/voxel.rs` | file-backed `face_texels`, per-face buckets + UVs, glass surface, `hides` |
| `client/src/streaming.rs` | LOD downsample no longer drops block ids ≥ 16 |
| `client/src/main.rs` | material table is per (block, face) |
| `client/src/mapfile.rs` | `"glass"` parses |
| `sim/src/block.rs` | `GLASS`, `is_solid` vs `is_opaque` |
| `sim/src/chunk.rs` | `solid_count` asks `is_solid` |
| `assets/textures/blocks/atlas.json` | world kinds named after sim blocks |
| `assets/textures/blocks/glass.png` | real alpha channel |
| `maps/glass_demo.json` | the split stone\|glass wall the pane is shot on |
