# Foliage lane wiring — patch summary

**Author:** Kevin (engineer) · **2026-08-20** · branch `poppy/native-only`

**The hole:** `client/src/main.rs` declares 34 `mod`s (lines 12–45) but has no
`mod foliage;`, and nothing calls `.add_plugins(foliage::FoliagePlugin)`. The
wind vertex shader therefore only runs inside `voxelforge_foliage_proof` — no
real game frame ever shows a plant swaying. The seven vegetation sprites also
have no `kinds` entry (and there is no cross-quad render mode), so nothing in
the world renders through `FoliageMaterial`.

**The patch:** `docs/_kevin_foliage_wire.patch` (6 files, ~120 added lines).
Apply after the Director lifts the build freeze:

```sh
git apply --recount --check docs/_kevin_foliage_wire.patch   # dry run
git apply --recount docs/_kevin_foliage_wire.patch            # apply
```

(`--recount` makes the apply robust to any hunk-count drift — the hunks are
correct, but recount removes the only fragile part of a hand-authored diff.)

---

## Per-file changes (exact lines)

### 1. `client/src/main.rs` — the two missing wires
- **line 27** (after `mod equipment;`): add `mod foliage;` (keeps the block
  alphabetical — `equipment` < `foliage` < `gizmo`).
- **line 649** (after `.add_plugins(water::WaterPlugin)`): add
  `.add_plugins(foliage::FoliagePlugin)` with a 4-line comment mirroring the
  water block above it. The plugin registers `MaterialPlugin<FoliageMaterial>`
  + the `advance_wind` clock; it is added unconditionally (cheap), exactly like
  `WaterPlugin`.

### 2. `assets/textures/blocks/atlas.json` — kinds + render mode
- **`_comment`** (end of array): one new entry recording that the 7 sprites now
  carry a `kinds` entry with `"mode": "cross"`.
- **`kinds`** (after `"metal"`): seven entries, one per sprite, each
  `{ "all": "<name>", "mode": "cross" }`. `"cross"` is the render mode; `all`
  is reused as the single billboard texture (no top/side/bottom needed for a
  cross-quad). This is the "kinds entry / render mode" the manifest's own
  header has been promising since Monanisa's 2026-08-18 pass.

### 3. `client/src/block_atlas.rs` — parse the mode, expose cross kinds
- **`KindEntry`** (~line 133): add `#[serde(default)] mode: Option<String>`.
- **`TileSet`** (~line 218): add `pub cross: BTreeMap<String, String>` (kind →
  albedo file).
- **`TileSet::cross_kinds()`** (~line 243): iterator over the cross pairs.
- **`load_tiles`** (~line 381): after resolving cube kinds, build `cross` from
  entries whose `mode == "cross"`; a cross kind with no `all` tile is a loud
  `Err` (matches the file's existing error discipline).
- **`load`** destructure (~line 409): add `cross: _` (the packed-atlas consumer
  ignores it).

### 4. `client/src/voxel.rs` — public accessor
- **after `tile_set()` (~line 157)**: add `pub fn cross_vegetation() ->
  Vec<(String, String)>` — a thin wrapper over the cached `TileSet` so the
  scatter reads the manifest without re-decoding PNGs. `tile_set()` stays
  private; this is the only new public surface.

### 5. `client/src/scene.rs` — the scatter (what makes plants actually sway)
- **imports (~line 32–36)**: add `use crate::foliage;`, `use crate::voxel;`,
  and `get_world_voxel` to the `crate::{…}` list.
- **`ScenePlugin` (~line 185)**: register
  `.add_systems(PostStartup, scatter_foliage.run_if(playing).after(boot_scene))`.
- **new fns (~line 535)**: `scatter_foliage`, `foliage_disabled`,
  `foliage_jitter`, `foliage_here`.

`scatter_foliage`:
- gated by `VOXELFORGE_FOLIAGE=off` (A/B lever, same discipline as
  `VOXELFORGE_WATER=off`).
- reads `voxel::cross_vegetation()` for the 7 `(kind, file)` pairs.
- builds ONE shared cross-quad mesh + ONE `FoliageMaterial` per kind (≤7
  materials total; per-plant phase/amp comes from the shader's world-cell hash,
  so no per-plant material is needed).
- for every loaded chunk column that is `BlockId::GRASS` on top (~1 in 6,
  deterministic via a wrapping hash), spawns a cross-quad at `(wx+.5, top+1,
  wz+.5)` wearing its kind's material.

---

## Why this is the right shape (not a shortcut)

- **Mirrors the water precedent exactly**: `water.rs` = marker + swap + plugin
  registered unconditionally + `VOXELFORGE_WATER=off` lever. Foliage follows the
  same split: `foliage.rs` stays **self-contained** (the proof bin
  `#[path]`-includes it and links nothing else), and only `scene.rs` — which
  already knows `World` + props — does the atlas lookup + placement.
- **`foliage.rs` is untouched**: its module doc explicitly promises
  "it links nothing else", and the proof bin depends on that. All new atlas/world
  coupling lives in `scene.rs`, which is allowed to see both.
- **Art stays editable in `atlas.json`**: adding a plant is a `kinds` entry, not
  a Rust change — the manifest remains the single contract.

## Verification plan (run after the freeze lifts — no `cargo build` now)

1. `git apply --recount --check docs/_kevin_foliage_wire.patch` → clean.
2. `git apply --recount docs/_kevin_foliage_wire.patch`.
3. `cargo build` (grep `^error` for the verdict — do NOT trust `tail`; see the
   0xc0000142 / pipe-mask scars).
4. `cargo test the_shipped_manifest_names_kinds_after_sim_blocks` — still green
   (additive `kinds` can't break a `contains_key` assertion).
5. `cargo test` (block_atlas has no new unit test; the cross-parse path is
   exercised end-to-end by the scatter at boot — a malformed cross kind fails
   `load_tiles` loudly).
6. Boot `--play` → stdout prints `FOLIAGE plants=N kinds=7` (N > 0 on any map
   with grass surface). Confirm plants sway in the wind field.
7. `VOXELFORGE_FOLIAGE=off --play` → no `FOLIAGE` line, world unchanged (A/B).

## Risks / notes

- The scatter runs in **play mode only** (`run_if(playing)`); editor/bench/shot
  lanes are byte-for-byte unchanged, matching `ScenePlugin`'s existing gate.
- The seven `tiles[]` entries still carry `"cross_block_pending": true`
  (`TileEntry` ignores it). It is now redundant; leave it or drop it later —
  the patch deliberately does not touch it to keep the diff focused.
- Scatter is a first wiring, not a density/tuning pass. Density (~1/6), scale
  jitter (0.6–1.3), and kind weighting are single constants/one-liners in
  `scene.rs` ready to tune once the look lands.
