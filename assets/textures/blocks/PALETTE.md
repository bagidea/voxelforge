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
the script reproduces byte-identical output). `contact_sheet.png` in this
folder is a `10x` nearest-neighbour blow-up of all 12 for review; it is
**not** a game asset.

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
