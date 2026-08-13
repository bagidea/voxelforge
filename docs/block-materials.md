# Block material response — per-block relief and finish

**Author:** Poppy (Engineer) · **Scope:** `client/src/voxel.rs` only.
**Brief:** "วัสดุยังแบน" — the blocks read as coloured cubes, not as wood, stone
and plaster.

## 1. What was already there, and why it wasn't enough

`voxel.rs` already shipped two thirds of the answer before this pass:

* `greedy_mesh_chunk_split` — one mesh **per block type**, so each type can wear
  its own material and its own `Repeat`-sampled tile (real per-block texel
  density instead of one atlas tile stretched over a merged quad).
* `block_surface()` — a per-type `perceptual_roughness` / `reflectance` /
  `emissive` table, so wood is satin and stone is matte.

That is still **one flat answer across a whole face**. Every texel of a cobble
wall hands the sun the same normal and the same roughness, so the painted stones
in the tile stay a *picture* of stones: move the sun, and the entire wall
brightens together. Nothing on the surface is ever lit differently from anything
else on it. That is what "flat" is, and no amount of albedo painting fixes it —
which is also why swapping in a hand-painted atlas (Monanisa's Option B) would
not have fixed it either.

## 2. What this pass adds

Two derived maps per block type, both generated from the **same `tile_shade`
pattern that paints the albedo**:

| | function | format | what it buys |
|---|---|---|---|
| Normal map | `build_block_normal_map` | `Rgba8Unorm` (linear) | per-texel shading normal — a plank groove, a mortar joint and the gap between two cobbles now catch and lose the light on their own |
| Roughness map | `build_block_metallic_roughness` | `Rgba8Unorm` (linear) | hollows dusty, high points polished, plus coarse 4×4 patchiness so a long wall is not one uniform finish |

Deriving both from `tile_shade` rather than hand-authoring a second pattern is
the load-bearing decision: albedo, relief and finish cannot drift out of
register, because there is only one pattern. A normal map that disagrees with its
own albedo is exactly what reads as plastic.

Height is sampled **wrapped**, not clamped (`tile_height`). The tiles are sampled
in `Repeat` and every `tile_shade` pattern is seamless, so a clamped edge would
have baked a one-texel ridge into every block boundary — the same class of bug as
the old painted border, just in the normal instead of the colour.

### The tangent frame (the part that silently does nothing without it)

Bevy's PBR shader only applies a normal map under `VERTEX_TANGENTS`. A mesh with
no `ATTRIBUTE_TANGENT` drops the whole map on the floor — **no warning, no
error**, and the frame renders identical to the flat version. So the split sweep
now emits tangents, computed analytically: the quads are axis-aligned and their
UVs run straight down `(du, dv)`, so the frame is exact and needs no
`generate_tangents()` pass and has no seam where a generated frame would flip.
Handedness is derived from `cross(N, T) · B` so back faces flip with their normal
instead of being hard-coded per axis.

The atlas/LOD path deliberately gets **no** tangents and no maps: it stretches one
tile across a whole merged quad, so per-texel relief there would smear.

### Roughness factor, not just a texture

Bevy *multiplies* `perceptual_roughness` by the map's green channel. Hand the
same factor to the mapped and unmapped cases and every mapped surface is quietly
smoother than the table says, with nothing anywhere reporting it. So
`block_material` raises the factor to `roughness_ceiling()` = `base + spread`
whenever a roughness map is present, and the map dips down from there — the
effective roughness straddles the table's value instead of only cutting below it.
There is a unit test pinning exactly that.

## 3. The per-material grade

`relief` is a decision per material, not a global filter:

| material | relief | roughness spread | why |
|---|---|---|---|
| cobblestone, gravel | 1.00 | 0.12 | a pile of separate loose objects — strongest relief in the palette |
| grass, leaves, moss | 0.85 | 0.08 | the dark gaps in the tile are real holes between blades |
| brick | 0.80 | 0.12 | recessed mortar joints, flat-ish faces |
| wood | 0.75 | 0.16 | deepest grooves *and* the smoothest face between them — that spread is the read |
| dirt | 0.70 | 0.08 | clumped, no fine structure |
| stone, clay (default) | 0.55 | 0.10 | matte mineral |
| limestone | 0.45 | 0.10 | plaster: broad soft undulation, not detail |
| sand, red sand | 0.35 | 0.06 | grain finer than a texel — sparkle, not shape |
| snow | 0.25 | 0.18 | almost no shape, wide finish spread: that is why snow glitters |
| lamp | 0.15 | 0.00 | a glowing surface has no shading to modulate |
| **obsidian (pane)** | **0.00** | 0.03 | deliberately flat — the diagonal sheen is a reflection, not a ridge. Bumping a pane only frosts it and kills the mirror. It gets **no normal map at all.** |

## 4. A/B lever

`VOXELFORGE_FLAT_MATERIAL=1` builds the whole palette base-colour-only — no
normal map, no roughness map — which is exactly what shipped before this pass.

It exists for the same reason `main::atlas_mesh_forced` does: the fix gets
photographed **against itself out of one binary** — same exe, same map, same
seed, same camera, one variable, so a before/after pair can never be two
different builds arguing.

**No such pair is attached to this doc yet.** The lever is in the binary and the
maps are unit-tested, but relief amplitude is the one number here that can only
be judged from a rendered frame (see below), and this lane verified with
`cargo check`, not a render — `docs/LANES.md` reserves full `--bin voxelforge`
builds for the integration lead. The A/B shot is owed at the next integration
pass: same exe, `VOXELFORGE_FLAT_MATERIAL=1` vs unset.

Two more runtime knobs scale the amplitudes without a rebuild —
`VOXELFORGE_MAT_RELIEF` and `VOXELFORGE_MAT_ROUGH_VAR`, both defaulting to `1.0`,
both multiplying the whole palette so the per-material *grade* in §3 stays
intact. Relief amplitude is the one number here that can only be judged from a
rendered frame, and this binary costs a fat-LTO link per rebuild; a runtime scale
turns a sweep from hours into minutes. A negative or non-numeric value falls back
to `1.0` rather than inverting every surface in the game on a typo.

## 5. Verdict on Monanisa's palette pass (`docs/block-palette.md` §5)

Her doc asked the engineering lane to pick one. **Option A is the call. Option B
is declined.**

**Option A — landed.** *(Written as a hand-off request in the original pass; the
five hex values went in at `a3bb308`, 2026-08-07, and the whole §6.1 table is
audited at `72408ad`. Updated 2026-08-14 — this section used to say "NOT landed,
blocked on a lane", which was true when it was written and false from a3bb308
onward.)* The corrected values live in `sim/src/block.rs::base_color()`, which is
where `voxel.rs::tile_base()` reads every tile colour from — verified, not
assumed:

    $ git log --format='%h %ad %s' --date=short -- sim/src/block.rs | head -3
    72408ad 2026-08-14 fix(blocks): one source of truth per block colour + pin the designer palette
    183f5b3 2026-08-10 feat(play): land the 5 stale code files ...
    a3bb308 2026-08-07 feat(sim): complete block palette + track Rose lane harness/docs
    $ git show a3bb308^:sim/src/block.rs | grep 'Self::GRASS       =>'
                Self::GRASS       => [70, 160, 66],
    $ grep 'Self::GRASS       =>' sim/src/block.rs
                Self::GRASS       => [91, 140, 70],     // #5b8c46

What moved, kept as the record of the change:

| BlockId | was | now | hex |
|---|---|---|---|
| `GRASS` | `[70, 160, 66]` | `[91, 140, 70]` | `#5b8c46` |
| `WOOD` | `[120, 86, 52]` | `[156, 107, 58]` | `#9c6b3a` |
| `COBBLESTONE` | `[92, 92, 98]` | `[140, 138, 120]` | `#8c8a78` |
| `MOSS` | `[55, 100, 40]` | `[75, 110, 55]` | `#4b6e37` |
| `LIMESTONE` | `[200, 190, 160]` | `[222, 204, 168]` | `#decca8` |

All 15 designer-owned hexes now match §6.1 mechanically —
`scripts/block_palette_audit.py` exits 1 on drift, and four unit tests in
`sim/src/block.rs` pin the table. `lamp` is the one slot with no designer hex and
carries a `PROVISIONAL` engineer value; see
`docs/note-to-monanisa-lamp-albedo-gap-2026-08-14.md`.

The reasoning that sells it is her golden-hour correction, not the sampling:
`base_color()` is documented as *unshaded* and the engine lights it afterwards,
so shipping the raw median off a golden-hour photo would double-count the light.
She caught that and neutralised for it. That is the part a colour picker would
have got wrong.

**Option B — declined, and it would not have fixed the brief.** The brief was
"วัสดุยังแบน". Swapping the procedural tile for `block_atlas_v2.png` changes
*albedo*, and albedo is not what flat means here: every texel of a face was
handing the sun the same normal and the same roughness, so a painted picture of
stones stays a picture (§1). §2 fixes that; a hand-painted atlas would have
looked better in a still and behaved identically under a moving sun. It also
costs the native/web parity the procedural path exists for, and reintroduces the
asset-load path `voxel.rs` deliberately dropped. The art is not wasted — the
grain she painted is the same grain `tile_shade` now drives the relief from.

**Her §4 gap is already closed, differently.** Glass and lamp do have slots now:
`LAMP` is `BlockId(16)` in `voxel.rs` with an emissive pushed past 1.0 so bloom
catches it, and glass is served by `OBSIDIAN` as an alpha-blended pane
(`alpha: 0.66`, `AlphaMode::Blend`). So
`block_atlas_v2_bonus_glass_lamp.png` tiles 16–17 no longer block on
engineering. They stay unloaded for the Option B reason above.

**Her reference deliverable — tracked, and deliberately unloaded:**
`assets/textures/block_atlas_v2.png`, `assets/textures/block_atlas_v2_bonus_glass_lamp.png`,
`assets/textures/block_atlas_v1_procedural_BACKUP.png`, `docs/block-palette.md`
and `docs/assets/block-palette/*` all went into git at `644cc61` (2026-08-06).
They are **not loaded by any code** — grep-verified, and the check is worth
repeating before anyone "cleans up" an unreferenced PNG:

    $ grep -rn 'block_atlas_v2\|block_atlas_v1_procedural' --include=*.rs \
        --include=*.toml --include=*.json client/ sim/
    (no matches)

They are the record of where the five hex values came from; deleting them would
orphan the audit trail in her §2 table.

## 6. What this doc does NOT claim

Every statement in §1–§4 was re-checked against `client/src/voxel.rs` by grepping
the symbol, not by trusting the prose. What is still open:

* **The unit tests in `voxel.rs` are authored but have not been executed.**
  `cargo check --target-dir target-poppy` was green with zero errors; the client
  crate is bin-only, so `cargo test` means linking the full `voxelforge` binary,
  which is the integration lead's build lock — and on this machine that test
  binary dies at DLL init (`0xc0000142`) before running anything. Read every
  assertion in `voxel.rs` as *pinned in source*, not as *observed passing*, until
  an integration pass runs them. (`sim/src/block.rs`'s tests do not have this
  problem — `sim` is Bevy-free and its tests run headless via `rustc --test`.)
* **The §4 A/B pair is still owed.** `VOXELFORGE_FLAT_MATERIAL=1` is in the
  binary; no rendered before/after has been shot from it yet.

Closed since the first draft: the §5 palette hand-off (landed a3bb308, audited
72408ad).
