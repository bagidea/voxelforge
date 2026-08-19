# Block texture palette — warm/cool split

Reference: CEO's Minecraft + shader sunset screenshot vs. our
`_fl_beauty_20260816/beauty-wide-nohud2.png`. The reference reads as a
warm *sunset light* falling on a scene whose **materials** span a wide
hue range (blue sky/water, saturated green trees, grey stone/dock posts,
warm wood/roof/sand). Ours currently reads as one orange hue across the
whole frame. That's not a lighting-only problem — it's also a materials
problem: everything the camera sees is warm-hued to begin with, so there
is no cool anchor left for the sunset light to contrast against.

This set exists to fix the materials half of that. Every texture below
was assigned a **deliberate HSV target** before drawing, split into two
groups on purpose, so a single frame built from these can never fully
collapse into one hue no matter how warm the sun gets:

- **Warm group** (sun-facing, wood/earth materials) — hue 10–45°
- **Cool group** (sky/foliage/mineral materials) — hue 100–220°, low-to-mid
  saturation so the warm key light can still gently tint them without
  erasing the hue difference

Real assets are `16x16` PNG, hand-authored per-pixel by
`scripts/_pixel_blocks_gen.py` (deterministic per-material seed — re-running
the script reproduces byte-identical output, confirmed on the first 12 when
the second pass below added 7 more). `contact_sheet.png` in this folder is
a `10x` nearest-neighbour blow-up of all 19 for review; it is **not** a
game asset.

## Warm group

| Material | File | Avg | H | S | V | Why |
|---|---|---|---|---|---|---|
| Oak planks | `oak_planks.png` | `#9b7753` | 30° | 0.46 | 0.66 | Base interior-wall wood. Mid-value so it doesn't blow out under the sun key light; seam lines every 4px darken -0.16V to read as distinct boards at block scale. |
| Log — bark (side) | `oak_log_side.png` | `#543f31` | 24° | 0.42 | 0.38 | Deliberately darker/duller than planks (-0.28V) so log posts silhouette against plank walls instead of blending into one wood mass, same failure mode as the current all-orange frame. |
| Log — cross-section (top) | `oak_log_top.png` | `#a08563` | 33° | 0.28–0.40 | 0.58–0.78 | Concentric rings + bark rim; brightest of the wood set — reads correctly on horizontal beam ends/stumps under top light. |
| Sand | `sand.png` | `#dac595` | 42° | 0.32 | 0.86 | Palest, highest-value warm texture — sand should be the brightest warm surface in frame (beach/shore), not compete with wood mid-tones. |
| Roof tile | `roof_tile.png` | `#6f3c2d` | 14° | 0.60 | 0.52 | Pushed to the reddest hue and highest saturation in the warm group on purpose — terracotta roof is the one warm material in the reference that's allowed to read almost as an accent colour, not a neutral wood tone. |
| Floorboards (deck/dock) | `floorboards.png` | `#897e73` | 28° | 0.16 | 0.58 | Same hue family as `oak_planks` but desaturated ~3x and mid-value — weathered dock/deck wood, distinct enough from interior planks that a dock built next to a cabin (as in the reference) doesn't read as one uniform wood block. |

## Cool group

| Material | File | Avg | H | S | V | Why |
|---|---|---|---|---|---|---|
| Stone bricks | `stone_bricks.png` | `#70767c` | 212° | 0.10 | 0.55 | Low-sat blue-grey, not neutral grey — a true neutral (S≈0) still reads "warm" once a warm key light hits it, a slight blue bias is what survives that tint and keeps stone stone. |
| Grass — top | `grass_top.png` | `#50833a` | 102° | 0.55 | 0.52 | Highest-saturation green in the set; grass top is the main green mass in any wide shot and needs to survive strong orange ambient without desaturating to olive. |
| Grass — side | `grass_side.png` | `#534f30` | mixed | mixed | mixed | Composite tile: grass-green cap (rows 0–2, irregular edge) over dirt (hue 26°, S 0.46, V 0.34). Average reads muddy by design — it's two materials in one texture, graded independently. |
| Leaves | `leaves.png` | `#2b6a29` | 118° | 0.62 | 0.42 | Darkest, most saturated green — canopy needs to hold shape (depth via dappled dark gaps) instead of flattening into a green silhouette the way the current beauty shot's crate reads as a flat block. |
| Glass | `glass.png` | `#9fc7d6` | 196° | 0.26 | 0.88 | Coolest, brightest texture in the whole set — glass is meant to be the one material that visually pulls toward "sky colour" regardless of what's lighting the room, per the reference's window panes. |
| Clay / plaster | `clay_plaster.png` | `#bbc5cc` | 204° | 0.08 | 0.80 | The one wall material given a cool cast instead of warm — an off-white lime-plaster look, so not every wall in a build defaults to the oak_planks hue. Small hue-222° flecks simulate trowel variation. |

## Second pass — map-verified gap fill (2026-08-17)

The first pass covered the beauty-shot hue fix; it left the atlas comment's
own "still procedural" list unresolved. Cross-checked against every block
`maps/beach_dusk.json` and `maps/glass_demo.json` actually place: **dirt,
brick, lamp, red_sand, snow** are all placed in one of those two maps and
had no file art. Added them, plus **water** and **metal** which the
Director asked for by name — those two are not in either map and no
`BlockId` for either exists in `sim/src/block.rs` today (`grep name()`
confirms it, and `client/src/voxel.rs` never sets `metallic > 0.0`), so
they ship as ready-to-wire art with no gameplay consumer yet. All seven
match the sim's own `base_color()` hex where one exists — the pixel art
is a *textured* version of the flat colour already approved, not a
new hue.

### Warm additions

| Material | File | Avg | H | S | V | Why |
|---|---|---|---|---|---|---|
| Dirt | `dirt.png` | `#594535` | 27° | 0.42 | 0.36 | Matches `BlockId::DIRT` (`#6b5540`) exactly on hue/sat; darkened ~0.06V and given crumb/pebble/root speckle so a dirt block reads as ground texture next to `grass_side`'s flat dirt band, not a duplicate of it. |
| Brick | `brick.png` | `#936b54` | 20° | 0.60 | 0.55 | Matches `BlockId::BRICK` (`#965a3c`). Courses use the same offset-row construction as `stone_bricks.png` but with **light** mortar (H38 V0.62) between **dark** brick — the inverse of stone's dark-mortar/light-stone relationship, which is what makes fired clay brick read as brick instead of masonry. |
| Lamp | `lamp.png` | `#b78d63` | 40°→28° | 0.20→0.55 | 0.97→0.69 | Radial gradient toward the sim's existing `#ffc476` glow hex at centre, cooling/darkening to the rim; four 2×2 corner texels dropped to near-black (H25 S0.35 V0.22) as a lantern-cage frame so the tile silhouettes as a fixture, not a glow decal. |
| Red sand | `red_sand.png` | `#c57958` | 18° | 0.55 | 0.78 | Matches `BlockId::RED_SAND` (`#c88246`). Same grain construction as `sand.png`, hue pulled ~24° redder and saturation raised so the two are unmistakable side-by-side in a beach/desert transition. |

### Cool additions

| Material | File | Avg | H | S | V | Why |
|---|---|---|---|---|---|---|
| Snow | `snow.png` | `#f0ece1` | 45° | 0.07 | 0.94 | Matches `BlockId::SNOW` (`#f0ece0`) almost to the texel. Sits right at the warm/cool boundary by design — S is low enough that a warm key light can't visibly push it warm, with occasional H205 sparkle flecks (~3% of texels) as the one deliberately-cool tell. |
| Water | `water.png` | `#4d7b9c` | 205° | 0.55 | 0.55 | No `BlockId` yet. Placed deep in the cool range, more saturated than `glass.png` (0.55 vs 0.26) since a body of water needs to read as a colour, not a clear pane. Horizontal wave bands + sparse sun-glint flecks (H+20 V+0.20) instead of flat fill. |
| Metal | `metal.png` | `#93979d` | 212° | 0.06 | 0.62 | No `BlockId` yet. Same blue-grey hue family as `stone_bricks` (212°) but near-neutral saturation and a diagonal brushed-sheen sine plus a 5px rivet grid, so at a glance it reads as fabricated panel, not natural stone. |

## How to keep this from drifting back to one-hue

- Never add a new material at hue 15–45° without also checking the frame
  still has something ≥100° in shot — that's the actual bug in the current
  beauty render (everything, including the "green" and "cyan" crates, is
  closer to hue 30–40° once you sample it — see `look-perf-and-buildlock`
  and related memory notes on ambient/ev100 not being the whole story).
- Saturation floor: nothing in the cool group should go below S 0.06,
  or a warm key light desaturates it into "warm" by simple additive tint.
- This is a texture/palette proposal for whoever wires up UV/material
  mapping next (the current beauty shot renders flat-shaded solid cubes,
  no texture sampling) — it does not by itself change any rendered frame.

## 64x64 PBR upgrade (2026-08-18)

The single biggest gap vs a realistic texture pack was resolution + no PBR:
every block above was `16x16`, albedo-only. All 19 are now `64x64` with a
full PBR triplet per block — `<name>.png` (albedo), `<name>_n.png`
(tangent-space normal map), `<name>_r.png` (roughness, grayscale,
white = rough / black = mirror-smooth). Same HSV targets/hue rationale as
every row above — the palette did not move, only the resolution and the
amount of real per-material structure drawn into it (brick/stone mortar
courses, individual wood-grain fibers, log rings, per-grain sand/dirt
speckle, a lantern cage, brushed-metal rivets, water sun-glint, snow
sparkle). Normal/roughness are derived from a height field authored in the
*same* per-pixel pass as the albedo (grooves recess + roughen, highlights
raise + smooth), so the bump/gloss detail is tied to what the albedo
actually draws rather than decorrelated noise.

Generator: `scripts/_pixel_blocks_gen64.py` (deterministic, same
per-material seeding as the retired `scripts/_pixel_blocks_gen.py`). Old
16x16 set backed up verbatim at `assets/textures/blocks_16px_backup/`.
`contact_sheet.png` is now a before(16px)/after(64px albedo) comparison per
block; `contact_sheet_pbr.png` is a new albedo/normal/roughness triplet
sheet for the 64px set. `atlas.json`'s `tile_px` is now `64`, and every
tile entry gained `normal`/`roughness` keys — inert extra JSON the current
loader (`TileEntry` in `block_atlas.rs`, no `deny_unknown_fields`) ignores
until the render lane wires PBR sampling into the pipeline. No Rust
changed as part of this pass.

## Environment pack — sky / water re-pass / vegetation (2026-08-18)

Reference for all three: `docs/refs/ceo_ref_sunset_valley.jpg` (CEO-provided
sunset-valley shot), sampled directly for hex values rather than eyeballed.
Generator: `scripts/_monanisa_env_pack_gen.py`, same "author a height field,
derive normal + roughness from it" pipeline as `_pixel_blocks_gen64.py`.

### Sky (`assets/textures/sky/`, not part of this block atlas)

Not a block material — a different asset family for the cloud-layer render
code (separate lane). Contact sheet: `assets/textures/sky/sky_contact_sheet.png`.

| File | What | Notes |
|---|---|---|
| `sky_gradient_sunset.png` | vertical sky ramp, zenith (top) → horizon (bottom) | 8-stop gradient, colours are the actual pixels sampled off the reference's sky column: `#3a2f52` indigo zenith → `#5c3f68` → `#96536f` → `#d06d72` → `#f3835f` → `#ffab5e` → `#ffd08a` → `#fff0c2` pale gold horizon glow. Column-uniform (reads as a clean 1D LUT), tiny per-channel dither to hide 8-bit banding once stretched. |
| `sky_clouds.png` | tileable cumulus texture, RGBA (alpha = coverage) | Built from tileable value-noise fbm (random-per-vertex noise on a wrapping integer-resolution grid, several octaves) with a light domain warp, not a sine sum — exactly periodic at 64px with zero per-pixel noise, since noise keyed off absolute x/y would break the seam. Warm-lit crown / cool-shadow base shading from the same field. Two earlier passes both failed a 2x2 tiled-zoom check: a 4-term then an 8-term integer-frequency sine sum both interfered into a visible diagonal argyle/lattice (a handful of plane waves reproduces its own interference no matter how many mismatched terms you add), and a bare small-grid value-noise fbm (no warp) interpolated into an equally regular diamond/saddle shape. Fixed by domain-warping the fbm lookup coordinate with a second independent noise field — bends the interpolation into organic curls — verified clean via an actual 2x2 tiled-zoom render, not by inspection of the single tile. |
| `sun_disk.png` | radial glow sprite, RGBA | Warm white core → gold → transparent haze, plus a faint horizontal glint band evoking the reference's horizon flare. |

### Water re-pass (`assets/textures/blocks/water*.png`)

Checked the existing `water.png` from the first PBR pass against the new
64px set: **not good enough**. A 512x512 nearest-neighbour zoom showed flat
blue with scattered light/dark speckle and no visible wave *shape* — reads
as noise, not water — and its wave terms used non-integer frequencies, so
the tile did not even wrap perfectly at 64px. Re-authored:

- Same HSV target as before, unchanged: `205deg / S0.55 / V0.55` — nothing
  downstream that reads this hue moves.
- Ripple field is two low-frequency directional swell terms (INTEGER x/y
  cycle counts, so exactly period-64) whose *phase* is domain-warped by a
  tileable noise field, plus a separate higher-octave noise "chop" layer on
  top, with the chop weighted as the dominant term. A first pass used the
  bare swell terms (even mildly phase-warped, on power-of-two noise grids)
  and still showed a clear diagonal band + regular diamond-dot lattice at a
  real 2x2 tiled-zoom render — two sine/noise terms alone still reproduces
  their own interference. Fixed with a stronger warp, chop-dominant
  weighting, and non-power-of-two noise grid sizes (harmonically related
  grid sizes re-align octave to octave into the same lattice failure).
- The height field always follows the same smooth ripple value, glint
  pixels included — an earlier pass punched an extra one-off spike into
  glint pixels, which a normal map turns into a sharp colored +-shaped
  artifact per pixel; at 2x2 zoom that read as its own scattered dot
  lattice on top of the wave bands. `water_n.png` shows real organic wave
  bump structure, confirmed via an actual 2x2 tiled-zoom render (not just
  inspection of the single 64px tile).
- Roughness is no longer a flat override: it runs through the same
  height-coupled `emit()` every other material uses, so wave crests read
  glossier than troughs, plus sparse near-mirror sun-glint texels (albedo +
  roughness only now, no height spike).
- Old version backed up verbatim at `assets/textures/blocks/_water_before_repass.png`.
  Side-by-side: `assets/textures/blocks/water_before_after.png`.

### Vegetation cross-sprites (`assets/textures/blocks/vegetation/`)

Seven 64x64 RGBA alpha-cutout billboards for the cross-quad render mode the
engine does not have yet (two intersecting quads, standard "flower/grass"
technique) — built from simple signed-distance primitives (capsule
stems/blades/twigs, polar-lobe petal functions, unions of circles for
foliage clumps), Porter-Duff over-composited onto a transparent canvas, same
emit-derived normal/roughness convention as every block. Contact sheet:
`assets/textures/blocks/vegetation/vegetation_contact_sheet.png`.

| File | What |
|---|---|
| `flower_red.png` | poppy-style: 6-lobe red petals, dark centre disc, green stem + 2 leaves |
| `flower_pink.png` | blossom-style: 5 rounded pink petals, tiny yellow stamen flecks |
| `flower_white.png` | daisy: 13 thin white petals, yellow speckled centre |
| `foliage_bush.png` | leafy shrub clump: union of 7 lobes, small soft leaf-gap stipple (kept small/numerous on purpose — an early pass used a few big dark craters + 2 centred berries and it read as a face on review; redone as scattered small gaps + 3 off-centre berries) |
| `grass_tall.png` | 7 tapering blades, dark-green base to yellow-green tip, gentle per-blade sway |
| `leaf_birch.png` | twig + 8 small oval leaves, light warm yellow-green (H86, distinctly lighter than the H118 `leaves.png` block so a birch canopy doesn't read as the same species as oak/generic leaves) |
| `leaf_pine.png` | twig + 8 needle bursts (~11 needles each), dark blue-green (H150) |

Registered in `atlas.json`'s `tiles` list only, deliberately with **no**
`kinds` entry — same "ready to wire, currently inert" precedent as
`water`/`metal` in the first PBR pass. `load_tiles` validates + packs these
seven (dimension-checked, 64x64 confirmed on every file), but nothing
references them by kind, so today's render is unaffected. A naive `kinds`
`"all"` mapping would be actively wrong for these — there is no cross-quad
face mode yet, so it would wrap the sprite onto all 6 cube faces — so that
part is left for whoever adds cross-block rendering.

Combined overview of all three groups: `assets/textures/env_pack_contact_sheet.png`.

## Palette breadth + micro-detail polish (2026-08-18, second pass)

Director's brief, sourced from `docs/art-gap-vs-ceo-ref-2026-08-18.md` §G6/§G7
(measured against the CEO's sunset-valley reference, resampled to equal
area): the block set uses only **6 of 36** hue bins vs the reference's
**15**, hue diversity (Shannon entropy) **2.16 vs 3.90 bits**, and detail
per screen-area (Sobel gradient mean) **30.8 vs 58.6 (53%)**. §G6 is
explicit that this is **not** a saturation problem — our saturation is
already 134% of the reference's — it's a **hue-width** problem: all 19
materials sat in the same 10–45°/100–220° two-lane split from the first
pass, so no single texture on its own opened the hue wheel further.

This pass does not move any base HSV target above (nothing in the tables
above changed) — it layers a **second, material-appropriate accent pass**
on top of every one of the 19 existing 64×64 tiles, adding secondary colors
that are a genuinely different **hue**, not a brightness/value tweak of the
same one, plus extra structured micro-detail for the Sobel gap. Every
accent is real weathering/material logic, not decoration for its own sake:

| Material | Accent(s) added | New hue(s) | Why this material, specifically |
|---|---|---|---|
| `oak_planks` | small blue-grey nail heads, 3 per plank | ~208° | Interior wood realistically has iron fixings; a cool metal fleck against warm wood is the single cheapest genuine hue contrast available on this tile |
| `oak_log_side` | lichen patches on bark | ~96° | Bark is the most common lichen substrate outdoors — free green without inventing a new material |
| `oak_log_top` | spalting streaks + rim lichen | ~200° / ~98° | Cut log ends spalt (fungal blue-grey staining) near the outer rings before bark lichen takes over the rim |
| `stone_bricks` | moss in mortar cracks + rust bleed from iron cramps | ~97° / ~24° | Both are the two classic weathering tells on old masonry; moss needs the damp mortar line, rust needs to look like it's dripping from something metal, so both are confined to mortar pixels only |
| `sand` | shell fragments, dark mineral grains, rare sea-glass fleck | ~20°(low S) / ~30°(low S) / ~172° | Beach sand is never one mineral — shell + basalt + the odd sea-glass shard is the real composition, and it directly answers "beach_dusk has no cool material" from the art-gap doc |
| `grass_top` | wildflower speckle (yellow + white) | ~50° / ~40°(low S) | Cheapest possible hue win: real turf is never a flat green, it's green + whatever's flowering in it |
| `grass_side` | clay pebble fleck in the dirt band + moss at the cap/dirt seam | ~202° / ~94° | Same logic as `dirt` below, applied only to the dirt portion of this composite tile |
| `leaves` | red berry clusters + warm autumn-edge dapple | ~355° / ~46° | REF's canopy hue diversity comes from exactly this — fruit + mixed-season leaf color, not a single uniform green |
| `roof_tile` | moss in the tile grooves + pale lime efflorescence | ~95° / ~45°(low S) | Terracotta roofing always weathers this way in the shaded low points between tiles |
| `glass` | rust bleed at the frame/mullion only | ~24° | Keeps the pane itself clean/cool — only the iron frame corrodes |
| `clay_plaster` | damp/mold patch low on the wall + ochre stain | ~100° / ~40° | Lime plaster stains from ground damp and iron-oxide runoff; kept to the lower half of the tile (damp rises from the base) |
| `floorboards` | algae stain near board seams + nail heads | ~98° / ~208° | A dock/deck gets algae in the seams where water sits, same nail-head logic as `oak_planks` |
| `dirt` | blue-grey clay lumps + small moss clumps | ~201° / ~95° | Real dirt is a mix of clay mineral and organic matter, not one uniform crumb color |
| `brick` | occasional whole "clinker" (overfired blue-black) brick + moss in mortar | ~220°(low S,V) | A real reclaimed-brick wall always has a few overfired units — this is a standard masonry-realism trick, not invented texture |
| `lamp` | verdigris patina on the corner cage frame | ~152° | The cage reads as bronze/copper hardware; patina is what that metal does outdoors |
| `red_sand` | dark mineral grains, olivine (green volcanic sand) fleck, pale shell fleck | ~15°(low S) / ~110° / ~25°(low S) | Same beach-composite logic as `sand`; olivine green-sand grains are a real (if exotic) beach mineral and the single biggest hue swing available on a red material |
| `snow` | rare trodden-mud fleck + directional grain | ~32°(low S) | Kept deliberately sparse — snow should stay near-neutral, this is just enough to avoid a perfectly flat sheet |
| `water` | golden sun-glint sparkle (was previously the *same* hue as the water under it) + sparse algae patch | ~46° / ~150° | The old glint (`emit(... h=H+jitter ...)`) boosted only V on the same 205° hue, i.e. a brighter blue, not a glint — a sunset sun-glint on water is warm-gold, so this was a genuine hue bug as well as a palette-breadth one |
| `metal` | rust patches + verdigris patches | ~24° / ~155° | The two standard oxidation states of a mixed steel/copper panel — also the strongest single hue swing in the set since the base metal is near-neutral |

Every accent is an **irregular, angle-wobbled blob** (`scatter_blobs` +
`paint_blobs` in the new generator), not a circle and not per-pixel static —
placement is seeded per material+accent name (deterministic, re-running
reproduces byte-identical output) and, where the material has real internal
structure, **confined to where the weathering would actually occur**
(mortar-only for stone/brick moss+rust, frame-only for glass rust, cage
corners only for lamp patina, dirt-band-only for `grass_side`'s clay). This
was a deliberate reaction to the art-gap doc's own warning under §G7: a
prior `outdoor-noon_after` frame scored well on Sobel density partly from
"noise that doesn't read as detail" (unreadable red speckle on stone) — so
every accent here is a recognizable *thing* (a berry, a nail head, a moss
patch), not texture-shaped noise.

Every accent write updates albedo **and** the height/roughness fields in
the same blended pass (`paint_accent`), so normal/roughness regenerate from
what's actually drawn — moss reads rougher and slightly raised, rust reads
mid-rough, sun-glint reads near-mirror-smooth — same "height field IS the
bump/gloss source" convention as every pass before this one, not
decorrelated noise pasted on top.

Verified against the 2x2 tiled-zoom-check failure mode that bit the cloud
and water textures earlier in this file (`_v3_2x2_tile_check.png`, built
and inspected, then deleted as scratch once confirmed clean): every accent
blob computes its distance/angle from an **unwrapped** center coordinate
before wrapping the write position with `% SIZE`, so a blob whose center
sits near x=0 or y=0 still renders as one continuous organic shape across
the seam instead of splitting into two mismatched half-blobs — checked
directly on all 19 materials tiled 2×2, no seam break, no argyle/lattice
interference (the wobble is a function of per-blob *angle*, not of x/y
position, so it can't self-interfere the way the old sine-sum clouds did).

Old (pre-polish) 64×64 set backed up verbatim at
`assets/textures/blocks_64px_prepolish_backup/` before this script wrote a
single pixel. Generator: `scripts/_pixel_blocks_gen64_v3_palette.py` — it
imports `_pixel_blocks_gen64.py` as a module and calls its `make_*`
functions directly for the base layer (so the base hue targets/tables above
stay the single source of truth), then applies the accent pass on top.
`contact_sheet.png` / `contact_sheet_pbr.png` (the canonical sheets
referenced above) were regenerated from the polished files, so they now
show the current state; `contact_sheet_v3_palette.png` is the dedicated
before(pre-polish)/after(polished) sheet for this pass specifically. No
`atlas.json` structure change — same 19 file names, same `tile_px: 64`.

**What this does and does not close:** measuring the actual `hue90`/
`entropy`/`Sobel` numbers requires a fresh in-engine render + the
`_pixel_artgap_*` measurement harness (Flamingo's lane, not mine, and it
needs a `maps/beach_dusk.json` render which this pass didn't produce) — the
claim here is scoped to what a texture-only pass can state directly: every
one of the 19 tiles now carries at least one, usually two, accent hues
outside its original 10–45°/100–220° band, and every accent adds real
edge-gradient structure (moss/berry/rust/nail-head blob edges are strong,
readable Sobel contributors, not smoothed-in noise). Whether that is enough
to clear the 15-bin/58.6-Sobel targets **in a full scene render** is a
follow-up measurement, not something claimed here.
