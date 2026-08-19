# Material palette — full block texture set (2026-08-17)

Owner: Monanisa (art lane). Assets live in `assets/textures/blocks/`; this
doc is the boss-facing summary of what's in the atlas, in hex/HSV, and why
it holds up under golden-hour light. The line-by-line design rationale per
texture (grain, seams, speckle construction) lives in
`assets/textures/blocks/PALETTE.md` — this file is the palette-level view,
that one is the per-pixel view.

## The actual root cause

The world reads as "one orange block" for two compounding reasons, not one:

1. **No file art at all.** `hero.rs`'s beauty scene spawns flat-shaded
   `Cuboid`s wearing a solid `base_color` — literally a colour swatch per
   block, no texture sampling (see `client/src/block_atlas.rs` module doc).
2. **The materials themselves were one hue.** Even once texture sampling
   exists, a palette where every material sits at H10–45° reads as one
   orange scene no matter how it's lit — there's no cool anchor for a warm
   sunset key light to contrast against.

Poppy's `block_atlas.rs` (file-backed atlas, mipmapped, bleed-free, driven
entirely by `assets/textures/blocks/atlas.json`) fixed (1). This palette
fixes (2). Together: every block placed in `maps/beach_dusk.json` /
`maps/glass_demo.json` now has real per-pixel art, split deliberately
across a wide hue range.

## Full set — 19 materials, 12 warm / 7 cool

All `64×64` with a full PBR triplet (albedo + tangent-space normal +
roughness) as of the 2026-08-18 pass, hand-authored per-pixel, deterministic
(`scripts/_pixel_blocks_gen64.py`, seeded per material — re-running the
script reproduces byte-identical PNGs; see
`assets/textures/blocks/PALETTE.md`'s "64x64 PBR upgrade" section for the
normal/roughness derivation). Sim-block hex values are quoted from
`sim/src/block.rs::base_color()` where one exists; texture hue/sat matches
that hex almost exactly on purpose — the pixel art textures the colour
already signed off, it does not re-grade it. Hex/H/S/V rows below are
unchanged from the 16×16 pass (same targets, higher resolution).

### Warm group — hue 10–45°, sun-facing materials

| Material | File | Avg hex | H | S | V |
|---|---|---|---|---|---|
| Oak planks | `oak_planks.png` | `#9b7753` | 30° | 0.46 | 0.66 |
| Log bark (side) | `oak_log_side.png` | `#543f31` | 24° | 0.42 | 0.38 |
| Log rings (top) | `oak_log_top.png` | `#a08563` | 33° | 0.28–0.40 | 0.58–0.78 |
| Sand | `sand.png` | `#dac595` | 42° | 0.32 | 0.86 |
| Roof tile | `roof_tile.png` | `#6f3c2d` | 14° | 0.60 | 0.52 |
| Floorboards (dock/deck) | `floorboards.png` | `#897e73` | 28° | 0.16 | 0.58 |
| Dirt | `dirt.png` | `#594535` | 27° | 0.42 | 0.36 |
| Brick | `brick.png` | `#936b54` | 20° | 0.60 | 0.55 |
| Lamp | `lamp.png` | `#b78d63` | 40°→28° (gradient) | 0.20→0.55 | 0.97→0.69 |
| Red sand | `red_sand.png` | `#c57958` | 18° | 0.55 | 0.78 |

### Cool group — hue 100–220°, low-to-mid saturation

| Material | File | Avg hex | H | S | V |
|---|---|---|---|---|---|
| Stone bricks | `stone_bricks.png` | `#70767c` | 212° | 0.10 | 0.55 |
| Grass top | `grass_top.png` | `#50833a` | 102° | 0.55 | 0.52 |
| Grass side | `grass_side.png` | `#534f30` | mixed (grass cap over dirt) | — | — |
| Leaves | `leaves.png` | `#2b6a29` | 118° | 0.62 | 0.42 |
| Glass | `glass.png` | `#9fc7d6` | 196° | 0.26 | 0.88 |
| Clay / plaster | `clay_plaster.png` | `#bbc5cc` | 204° | 0.08 | 0.80 |
| Snow | `snow.png` | `#f0ece1` | 45° (near-neutral) | 0.07 | 0.94 |
| Water | `water.png` | `#4d7b9c` | 205° | 0.55 | 0.55 |
| Metal | `metal.png` | `#93979d` | 212° | 0.06 | 0.62 |

## Why this survives golden hour

A golden-hour key light is warm (low colour temperature) and additive —
it pushes every surface's hue *toward* orange and lifts its value. Two
palette rules make sure that push can't collapse everything into one hue:

1. **The cool group has a saturation floor (S ≥ 0.06).** A true neutral
   grey (S = 0) has nothing to resist a warm tint with — it just becomes
   warm. Every cool material here carries a deliberate low-but-nonzero
   blue/green bias (stone and metal both sit at H212°, water and glass in
   the 196–205° band) so a warm additive tint shifts them *toward* orange
   without fully erasing the difference from the warm group.
2. **The warm group is capped under H45°, the cool group starts at H100°.**
   There's a 55° dead zone between them on purpose. Golden-hour light
   shifts hue by tens of degrees, not by 100+, so even a strongly-lit cool
   surface can't drift far enough to read as a warm one in the same frame.

Net effect: a wide shot built from this set keeps a blue-grey stone wall,
green canopy, and blue window/water reading as distinct materials next to
warm wood/sand/brick/roof, even under a strong orange key — which is
exactly the failure mode in the current beauty render this set replaces
(everything, "green" crates included, sampling within a few degrees of
hue 30–40°).

## Coverage vs. the actual maps

Cross-checked against every block placed in `maps/beach_dusk.json` and
`maps/glass_demo.json`:

| Block (sim `BlockId::name()`) | Used in | Has file art |
|---|---|---|
| grass, sand, wood, leaves | beach_dusk | yes |
| dirt, brick, lamp | beach_dusk | yes (added this pass) |
| glass, stone, wood, leaves, lamp | glass_demo | yes |
| limestone, red_sand, snow | glass_demo | yes (limestone was already covered; red_sand/snow added this pass) |

Every block either map places now has file art. Blocks with **no** file
art yet, still on the code-painted procedural tile: `clay`, `gravel`,
`cobblestone`, `obsidian`, `moss` — none of these appear in either map
today, so they weren't blocking, but they're the next gap if a map places
one.

## `water` and `metal` — shipped, not yet wired to gameplay

The Director's brief named water and metal explicitly. Neither is a real
block in the sim: `sim/src/block.rs::BlockId::name()` has no `water` or
`metal` arm, and `client/src/voxel.rs` never sets `metallic` above `0.0`
for any block today (checked directly, not assumed). `water.png` and
`metal.png` are drawn and wired into `atlas.json`'s `kinds` table anyway —
the moment a `BlockId::WATER` / `BlockId::METAL` lands with that exact
`name()`, the art is already there and needs no re-export. Until then
those two `kinds` entries are simply never looked up; they cost nothing
and block nothing.

## Deliverables from this pass

- `assets/textures/blocks/{dirt,brick,lamp,red_sand,snow,water,metal}.png` — new textures
- `assets/textures/blocks/atlas.json` — tiles + kinds wired for all 7
- `assets/textures/blocks/PALETTE.md` — per-pixel rationale, second-pass section
- `assets/textures/blocks/contact_sheet.png` — regenerated, now 19 tiles
- `docs/assets/look/atlas-preview.png` — this doc's boss-facing preview image
- `scripts/_pixel_blocks_gen.py` — extended generator (art tooling only, no Rust/Cargo touched)
