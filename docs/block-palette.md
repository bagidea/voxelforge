# Block Texture Palette — cozy-cabin reference pass

**Author:** Monanisa (Designer) · **Scope:** `assets/` + `docs/` only — no `.rs` files touched.
**Reference:** two CEO-supplied photos of a warm cottage/cabin build —
`1785603809332_..._n.jpg` (interior, top-down) and
`1785603809335_..._n.jpg` (exterior, front elevation). Mood: warm oak,
soft cream plaster, mossy grey stone, natural (not neon) grass and
leaves, glowing lanterns.

## 0. Why this exists

`client/src/voxel.rs::build_atlas()` does **not** load a PNG — it procedurally
paints a 16-tile strip (`N_TILES=16 × TILE_PX=16px` = 256×16) at runtime,
sourcing each tile's flat colour from `BlockId::base_color()` in
`sim/src/block.rs`, plus a cheap per-texel dither. There was no atlas asset
on disk before this pass. That's why "back up the old atlas before
overwriting" means backing up **what the code currently generates**, not an
existing file — see §3.

The CEO flagged neon-green grass as a repeat mistake. Checked: current
`GRASS` constant is `[70,160,66]` (`#46a042`) — not literally neon, but flat
colour + dither with no grain reads synthetic/plasticky once lit. Fixed by
(a) desaturating toward a calmer natural green and (b) adding real blade
texture so it reads as a material, not a colour swatch.

## 1. Sampling method (script-extracted, not guessed)

Used a Python/PIL script to grid-map both reference photos, crop candidate
regions per material, and take the **per-channel median** of each crop
(robust to a stray highlight pixel) — never eyeballed a colour picker.
Multiple candidate regions were sampled per material; contact sheets were
visually reviewed before picking the final region. See raw samples below.

**Important correction applied:** both photos are shot at golden hour with
a strong warm colour grade. `base_color()` is documented in `block.rs` as
"unshaded" — the engine lights it afterward. Feeding it a literal
warm-lit pixel double-counts the lighting and pushes everything toward
orange (a cream wall samples at `#aa8445`, grass samples at `#c1c01c`,
i.e. *more* golden than the CEO's "not neon" ask would want in the other
direction). So the shipped hex per material is a **corrected albedo**:
anchored to the real sample, then neutralized back toward what that
material actually is (cream plaster is cream, not caramel; grass is
green, not golden-yellow) — standard texture-art practice, not a guess.
Both the raw sample and the corrected final are in the table so this is
auditable.

| Material | Source crop (image, region) | Raw median sample | Corrected/final hex |
|---|---|---|---|
| Warm oak wood | img1, dining-bench top (450,380)-(560,460) | `#a36834` | **`#9c6b3a`** |
| Cream limewash wall | img2, porch wall in shade (438,508)-(462,528) | `#aa8445` (golden-biased) | **`#decca8`** |
| Mossy grey fieldstone | img2, stone pavers (150,800)-(250,850) | `#424135` (shadowed) | **`#8c8a78`** |
| Moss accent | img2, moss on stone ledge (1350,740)-(1420,800) | `#d2c829` (golden-biased) | **`#4b6e37`** |
| Natural grass | img2 lawn (1150,850)-(1300,915) + img1 grass runner (860,400)-(950,480) | `#c1c01c` / `#4a4608` (both golden-shifted) | **`#5b8c46`** |
| Leaves | img2, tree canopy (30,20)-(200,150) | `#84852c` (golden-biased) | **`#3a7436`** |
| Glass (bonus, see §4) | img2, window pane (380,260)-(430,300) | `#648885` | **`#7e96b2`** |
| Lamp glow (bonus, see §4) | img1, lantern flame core, brightest px (1038,24)-(1050,36) | `#ffb76e` (max, used directly) | **`#fff0ce`→`#ffb25a` gradient** |

Dark pine trusses were also checked for — there is no wood in either
reference that's a *different hue family* from the oak, only a darker
shadow-value of the same warm brown (`#52280c`, `#311106` from the
bookshelf/beam-shadow crops). There's no dedicated `BlockId` slot for a
second wood tone anyway (see §4), so this was not force-mapped onto an
unrelated block.

## 2. Full 16-tile palette (matches `BlockId` 0–15 order exactly)

Only 5 of 16 slots were in the CEO's requested list and got new colours;
the other slots keep their **existing** `sim/src/block.rs` value untouched
(out of scope for this pass) — they just got real painted texture instead
of flat colour + dither in the new atlas asset (§3).

| # | BlockId | Hex | Changed? | Texture treatment |
|---|---|---|---|---|
| 0 | air | — (transparent) | — | n/a, never rendered |
| 1 | grass | `#5b8c46` | **yes** | vertical blade strokes, light/dark |
| 2 | dirt | `#7c5838` | no | mottled blotch clusters |
| 3 | stone | `#80808a` | no | cracked-slab streaks |
| 4 | sand | `#d6ca94` | no | fine grain speckle |
| 5 | wood | `#9c6b3a` | **yes** | 4 vertical oak planks + grain |
| 6 | leaves | `#3a7436` | **yes** | mottled leaf clusters |
| 7 | snow | `#f0f5fa` | no | flecks + sparkle |
| 8 | red_sand | `#c88246` | no | fine grain speckle |
| 9 | clay | `#8c96a8` | no | horizontal banding |
| 10 | gravel | `#6e645e` | no | pebble blobs |
| 11 | cobblestone | `#8c8a78` | **yes** | 4-stone coursing + moss flecks on ledges |
| 12 | obsidian | `#14121c` | no | near-black + specular glints |
| 13 | brick | `#965a3c` | no | brick coursing, cream mortar |
| 14 | moss | `#4b6e37` | **yes** | dense green blotches, darker/richer than grass |
| 15 | limestone | `#decca8` | **yes** | cream plaster, subtle panel seams |

## 3. Files delivered

- **`assets/textures/block_atlas_v2.png`** — the new hand-painted atlas.
  256×16px, RGBA, 16 tiles of 16×16px in the exact order above. Drop-in
  compatible with the UV math already in `voxel.rs::greedy_mesh_chunk`
  (`t / N_TILES` per tile index) — **no layout change needed**.
- **`assets/textures/block_atlas_v1_procedural_BACKUP.png`** — a pixel-exact
  re-render of what `build_atlas()` currently produces from today's
  `base_color()` constants (same hash-dither formula, replicated in
  Python). Since there was never a shipped PNG to begin with, this is the
  faithful "before" backup the brief asked for — nothing was overwritten,
  because nothing existed. Keep this for before/after comparisons.
- **`assets/textures/block_atlas_v2_bonus_glass_lamp.png`** — 32×16px, 2
  extra tiles (glass, lamp-glow) for materials the CEO asked for that have
  **no `BlockId` slot yet** (see §4). Kept out of the 16-wide main atlas on
  purpose so it can't be mistaken for a drop-in replacement.
- **`docs/assets/block-palette/block-atlas-contact-sheet.png`** — every
  tile (incl. the 2 bonus ones) blown up 8× with labels + hex, checkerboard
  background behind the transparent/translucent ones.
- **`docs/assets/block-palette/block-cohesion-mockup.png`** — a small
  built scene (wood corner trim, cream wall, glass window, lit lamp,
  mossy cobblestone base, grass ground) using only the new tiles, tiled
  edge-to-edge, to prove the set reads as one cohesive material family —
  not just 16 isolated swatches.
- **`docs/assets/block-palette/before-after-changed-materials.png`** — the
  5 changed materials, old procedural tile vs. new painted tile, side by
  side.

Nothing else in `assets/` or `docs/` was modified or deleted.

## 4. Gap: glass & lamp have no `BlockId` yet

The CEO's reference list included กระจก (glass) and โคมไฟเรืองแสง (glowing
lamp). `sim/src/block.rs` defines exactly 16 `BlockId`s (0–15, all used by
`N_TILES=16`) and neither exists. Adding them isn't an assets/docs change:

- Glass needs a new `BlockId::GLASS`, an alpha-blended (not opaque)
  render path in the mesher/material (`greedy_mesh_chunk` currently treats
  every non-air block as fully opaque via `is_opaque()`), and a slot in
  `ALL_PLACEABLE`.
- Lamp needs a new `BlockId::LAMP` plus an emissive material (the
  `StandardMaterial` atlas hookup in `main.rs` would need an emissive
  texture/channel, not just base colour).

Both are real engineering changes to `.rs` files I'm not touching this
pass. The textures are ready now (`block_atlas_v2_bonus_glass_lamp.png`,
tiles 16–17) so whoever owns `block.rs`/`voxel.rs` can wire them in without
waiting on art. Flagging this now rather than silently faking a mapping
onto an unrelated existing block (e.g. painting "glass" onto the `clay`
slot) which would mislabel the material in the HUD/pick row.

## 5. Integration options for Poppy (pick one — both are gate-safe on the art side)

**Option A — low-risk, fast (recommended first pass):** edit only the 5
changed `[u8;3]` literals inside `BlockId::base_color()`
(`sim/src/block.rs`) to the "Corrected/final hex" values in §2 for
`GRASS`, `WOOD`, `COBBLESTONE`, `MOSS`, `LIMESTONE`. Zero new asset
loading, the procedural atlas keeps working exactly as designed (the
"identical on native and web, no asset-path headaches" comment in
`voxel.rs` stays true), just better base colours feeding the existing
dither. Lowest blast radius.

**Option B — full fidelity:** point `build_atlas()` at
`assets/textures/block_atlas_v2.png` (load via Bevy's asset server or
`include_bytes!`) instead of generating procedurally. Gets the real
painted grain (plank lines, brick coursing, moss flecks, cracked stone)
that flat colour + dither can never produce. UV math doesn't need to
change — tile layout already matches. Tradeoff: reintroduces an
asset-loading path, which is exactly what the original procedural
approach was written to avoid (native/web parity) — worth Poppy
double-checking the web build story before switching.

Either way, §4's glass/lamp gap is a separate follow-up once a code owner
adds the `BlockId`s.

---

## 6. v3 — full 16-slot pass for the golden-hour grade (2026-08-06)

**Trigger:** `docs/look-bible.md` §4's "Palette · Mood · Texture" target (amber-gold
key `#F4B860`, warm bounce `#C88A4A`, warm grey-beige stone `#B9A98C`, ~85%
warm / ≤10–15% cool teal accent) plus the warm haze/grade `look.rs` (rose's
lane) is currently baking. §2 above only touched the 5–6 slots the CEO's
photo brief called out; the other 9 still carry the **original, un-audited**
`base_color()` values — several of which actively fight a warm grade. This
pass covers all 16 slots so the full atlas reads as one material family
under golden-hour light, not 6 corrected tiles next to 9 untouched ones.

**Same rule as §1 applies:** `base_color()` is documented **unshaded** —
`look.rs` already adds the warmth (sun colour, warm haze, ACES-ish tone
curve). These are *corrected albedo*, not pre-toned hero-shot pixels. Two
failure modes were being actively designed against:
1. **Feeding it warm-lit pixels** double-counts the grade (§1's lesson —
   still true).
2. **Leaving it a cold/neutral pixel** undershoots — a couple of the
   untouched originals (`stone`, `clay`, `snow`) have `B ≥ R` (blue-leaning
   or blue-equal), which is the one combination a warm key + warm haze
   renders *worst*: cool greys pick up a muddy, faintly green cast under
   warm bounce light instead of harmonizing with it. Those needed a small
   warm-neutral correction even though nobody asked for them by name.

### 6.1 Full table — old → new, one-line reasoning each

| BlockId | `block.rs` line | Old hex | **New hex** | Changed? | Why |
|---|---|---|---|---|---|
| grass | 86 | `#46a042` | **`#5b8c46`** | carried from §2 | already corrected off neon; unchanged this pass |
| dirt | 87 | `#7c5838` | **`#6b5540`** | **yes** | was nearly hue-identical to `wood` (124,88,56 vs 120,86,52) — under warm haze the two blocks melted into one brown; pulled dirt toward a neutral, less-saturated umber so soil reads as *soil*, not "the other wood" |
| stone | 88 | `#80808a` | **`#8f8776`** | **yes** | old value has `B(138) > R(128)` — a cool blue-grey slab that clashes with a warm key/haze (picks up a muddy cast instead of harmonizing); warmed toward the look-bible's `#B9A98C` warm-grey-beige target while staying desaturated enough to still read as bare stone |
| sand | 89 | `#d6ca94` | `#d6ca94` | no | already warm tan, already fits the family |
| wood | 90 | `#785634` | **`#9c6b3a`** | carried from §2 | already corrected (photo-sampled oak); unchanged this pass |
| leaves | 91 | `#308230` | **`#3a7436`** | carried from §2 | already corrected off saturated green; unchanged this pass |
| snow | 92 | `#f0f5fa` | **`#f0ece0`** | **yes** | old value is icy blue-white (`B > R`) — a "cold hole" punched in every warm-lit frame it appears in; shifted to a warm off-white so it still reads bright/desaturated as snow without visually fighting warm GI/haze (mirrors the look-bible rule that in-shadow areas get warm bounce, not blue) |
| red_sand | 93 | `#c88246` | `#c88246` | no | already warm terracotta-orange, distinct from wood/brick, fits as-is |
| clay | 94 | `#8c96a8` | **`#7e96a0`** | **yes** | old value was the single coldest, muddiest outlier in the palette (`B(168) > R(140)`, low-chroma blue-grey — reads dirty, not intentional); rather than force it warm-neutral like stone, gave it a clean **teal-grey** identity instead — the look-bible explicitly wants a ~10–15% cool-teal accent (`#4FC9D6` family) so builders have *one* legitimate cool material instead of an accidental clash. Use sparingly, same budget rule as the bible's teal accent |
| gravel | 95 | `#6e645e` | `#6e645e` | no | already warm neutral dark grey-brown, fits |
| cobblestone | 96 | `#5c5c62` | **`#8c8a78`** | carried from §2 | already corrected (mossy warm stone); unchanged this pass |
| obsidian | 97 | `#14121c` | **`#1a1620`** | **yes (subtle)** | small warm-violet nudge (was 20/18/28, now 26/22/32) so it sits in the *same* hue family as the look-bible's shadow colour `#2A2030` — deep-shadow areas and obsidian blocks now read as one consistent "warm-violet dark" instead of two unrelated blacks. Still reads near-black; not a visible repaint |
| brick | 98 | `#965a3c` | `#965a3c` | no | already a correct warm terracotta, fits as-is |
| moss | 99 | `#376428` | **`#4b6e37`** | carried from §2 | already corrected (richer/darker than grass); unchanged this pass |
| limestone | 100 | `#c8bea0` | **`#decca8`** | carried from §2 | already corrected (cream plaster); unchanged this pass |

**Net new changes this pass (not already covered by §2):** `dirt`, `stone`,
`snow`, `clay`, `obsidian` — 5 slots. Combined with §2's 6, that's 11 of 15
placeable blocks now audited against the warm grade; `sand`, `red_sand`,
`gravel`, `brick` were checked and are genuinely fine unchanged.

### 6.2 Swatch sheet (real rendered PNG, not just numbers)

`docs/assets/block-palette/block-palette-v3-golden-hour-swatch.png` — all 15
placeable tiles, old-vs-new split swatch for every changed slot (labelled
"CHANGED"), hex + RGB under each. Rendered with Python/PIL, flat colour
(matches what `base_color()` feeds the procedural atlas before dither) —
open it directly to eyeball the family cohesion, not just read hex codes.

### 6.3 For Poppy — where this lands in `sim/src/block.rs::base_color()`

All 15 values are single `[u8; 3]` literal edits inside the existing
`match` at lines 85–101 (`base_color()`), same shape as §5 Option A — no
struct change, no new `BlockId`, no atlas/loading change:

```
line 86  GRASS       -> [91, 140, 70]     // #5b8c46 (unchanged from §2 if already landed)
line 87  DIRT        -> [107, 85, 64]     // #6b5540
line 88  STONE       -> [143, 135, 118]   // #8f8776
line 89  SAND        -> [214, 202, 148]   // #d6ca94 (no change)
line 90  WOOD        -> [156, 107, 58]    // #9c6b3a (unchanged from §2 if already landed)
line 91  LEAVES      -> [58, 116, 54]     // #3a7436 (unchanged from §2 if already landed)
line 92  SNOW        -> [240, 236, 224]   // #f0ece0
line 93  RED_SAND    -> [200, 130, 70]    // #c88246 (no change)
line 94  CLAY        -> [126, 150, 160]   // #7e96a0
line 95  GRAVEL      -> [110, 100, 94]    // #6e645e (no change)
line 96  COBBLESTONE -> [140, 138, 120]   // #8c8a78 (unchanged from §2 if already landed)
line 97  OBSIDIAN    -> [26, 22, 32]      // #1a1620
line 98  BRICK       -> [150, 90, 60]     // #965a3c (no change)
line 99  MOSS        -> [75, 110, 55]     // #4b6e37 (unchanged from §2 if already landed)
line 100 LIMESTONE   -> [222, 204, 168]   // #decca8 (unchanged from §2 if already landed)
```

Checked against current `sim/src/block.rs` on 2026-08-06: **none of §2's
6 carried-over corrections have landed yet** (the file still reads the
original `[70,160,66]` grass etc.), so all 11 changed lines above are live
diffs against what's on disk today, not just the 5 new ones. `sand`,
`red_sand`, `gravel`, `brick` (4 lines) are listed for completeness only —
no edit needed there.

I have not touched `sim/src/block.rs` or run `cargo` — this section is
spec only, same as §5.
