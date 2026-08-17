# Player character art — replacing the orange capsule

Owner: Monanisa (art lane). Art-only deliverable — no `.rs`/Cargo touched.
Scope: the **playable avatar** the `FlyCam` spawns in `client/src/main.rs`
(currently a flat-orange `Capsule3d`, `main.rs:975-980`), not the NPC cast in
`characters.rs` (see "Two different bodies already exist" below — that's the
most important thing this doc found).

## Two different bodies already exist in this codebase

Before designing anything new, I read `characters.rs` + `equipment.rs` end to
end, because the Director asked. That surfaced something worth flagging
before the rest of this doc: **the game already has a fully sculpted,
equipment-slotted hero body** — `AUREN_BODY` in `characters.rs`, 113 boxes,
V-tapered torso, a built face, hair, six equipment slots (head/torso/legs/
hands/weapon/back) wired through `equipment.rs`'s `Palette`/`Surf`/`Mat`
system. It is real, it is detailed, and `Who::Auren` **is** the protagonist
per `docs/story-bible.md` (working name Auren, campfire-customisable,
Shaper-origin). It just isn't spawned anywhere the player controls — it only
appears via `VOXELFORGE_CHARSHOT` (cast line-up / gear-ladder shots) and the
story's NPC scenes.

So the actual gap isn't "no hero body exists," it's "the hero body that
already exists never gets attached to `FlyCam`." That's a code decision (which
body function `main.rs`'s spawn path calls) that's out of my remit — no
Rust touched here — but it belongs in the next handoff conversation, because
wiring `AUREN_BODY` onto the player is probably less work than building a new
rig from scratch, and it's already on-canon.

**What this doc delivers instead**, per the brief: a lightweight
**gameplay-LOD body** sized directly off the constants `FlyCam` already uses
(`PLAYER_HALF_W = 0.3`, `PLAYER_HEIGHT = 1.8`, `main.rs:1711-1712`), cheap
enough to run every frame in the main loop (14 boxes vs. Auren's 113), and
built from a palette that is deliberately compatible with — not identical
to — Auren's canon materials, so unifying the two later is a colour-and-scale
exercise, not a redesign.

## Box structure — 14 boxes, all sized off `PLAYER_HALF_W`/`PLAYER_HEIGHT`

Local space: origin at the feet (`y=0`), crown at `y = PLAYER_HEIGHT = 1.8`,
`x`/`z` centred on the capsule's own axis. Front is local `-Z` (same
convention `main.rs`'s face-block comment uses, so the body doesn't need its
own heading rule). All half-extents below are **inside or within ~0.09 m of**
`PLAYER_HALF_W` (0.3 m) — the arms are the one place that pokes past the
collider on purpose, the same way Auren's sculpted deltoids (§`characters.rs`
"the 14° roll IS the shoulder line") flare past the waist width. The capsule
collider stays the physics shape; this is the cosmetic mesh riding inside/on
it.

| # | Part | x range (m) | y range (m) | z half-depth | Material |
|---|---|---|---|---|---|
| 1–2 | Boots L/R | ∓0.11 ± 0.10 | 0.00 – 0.22 | 0.12 | Boots |
| 3–4 | Legs L/R | ∓0.11 ± 0.095 | 0.22 – 0.86 | 0.10 | Legwear |
| 5 | Belt trim | 0 ± 0.20 | 0.83 – 0.88 | 0.14 | Trim |
| 6 | Waist | 0 ± 0.16 | 0.86 – 1.06 | 0.13 | Tunic |
| 7 | Chest/shoulders | 0 ± 0.27 | 1.06 – 1.46 | 0.16 | Tunic |
| 8–9 | Arms L/R (cosmetic, pokes past collider) | ∓0.335 ± 0.055 | 0.60 – 1.40 | 0.06 | Tunic |
| 10–11 | Hands L/R | ∓0.335 ± 0.06 | 0.52 – 0.60 | 0.06 | Skin |
| 12 | Head | 0 ± 0.155 | 1.46 – 1.74 | 0.155 | Skin |
| 13 | Hair/cap | 0 ± 0.17 | 1.72 – 1.80 | 0.17 | Hair |
| 14 | Cloak (back panel, +Z side) | 0 ± 0.20 | 1.00 – 1.42, sits at z +0.16..+0.26 | 0.05 thick | Trim |

Notes for whoever wires this up:
- Chest half-width (0.27) stays *inside* `PLAYER_HALF_W` (0.30) on purpose —
  unlike the arms, the torso should never clip through a 1-block gap.
- Boots/legs/waist stack to exactly `y=0.88`, chest to `y=1.46`, head to
  `y=1.74`, hair to `y=1.80` — no gaps, no overlap math needed at the seams.
- This table is a spec for a `characters.rs`-style `&[Bx]` array, written in
  metres instead of the existing 8-vx-per-block voxel units — deliberately,
  since `PLAYER_HALF_W`/`PLAYER_HEIGHT` are already in metres and a LOD body
  should stay legible as "a fraction of the collider," not force a unit
  conversion on the next person.

## Palette — 6 colours

Cross-checked against the two things this body has to survive: the
golden-hour ground textures I shipped this pass (`docs/material-palette.md`)
and Garren/the Guard Husk's armour (`HuskPlate` `#B9A98C` H39° S24% V73%,
`HuskPlateDark` `#8A7A5C` H39° S33% V54% — the only other humanoid the player
stands next to in combat).

| Slot | Hex | H | S | V | Source | Why |
|---|---|---|---|---|---|---|
| Skin | `#D9B08C` | 28° | 36% | 85% | **Reused** — `equipment.rs::Surf::Skin` | Same face colour Auren already has in the cast shots; if the bodies unify later, skin never needs a re-grade. |
| Hair | `#2A1B12` | 23° | 57% | 17% | **Reused** — `equipment.rs::Surf::Hair` | Same reason — canon continuity, zero regrade cost. |
| Tunic | `#2E6E78` | 188° | 62% | 47% | New | The one cool-hued piece on the body. Every ground texture in `material-palette.md` sits at H18–102°; Husk's armour sits at H39°. A teal torso at H188° cannot be mistaken for either at a glance, in bright sun or dusk. It's also a quiet callback to the gate sigil ("the teal sigil blazes for Auren alone," `story-bible.md` §0) — the one Shaper-coded colour on the one Shaper-coded character. |
| Legwear | `#4A3220` | 26° | 57% | 29% | **Reused** — `equipment.rs::Surf::LeatherStrap` | Low value (29%) reads as "grounded" against every bright floor texture (sand 86%, snow 94%) without vanishing into Husk's armour, which sits nearly 30 points brighter (V54–73%). |
| Boots | `#2A1E16` | 24° | 48% | 17% | New, darker than any existing `Surf` | Near-black anchors the silhouette's base the same way a shadow does — matters most on snow, where anything mid-value risks reading as one more grey lump. |
| Trim (belt + cloak) | `#E8A23C` | 36° | 74% | 91% | New | Same hue family as Husk's armour (36° vs 39°) but nearly 3× its saturation (74% vs 24–33%) and pushed to near-max value — a flat colour swatch of trim next to a flat swatch of Husk armour is unmistakably different even though a hue wheel alone would put them side by side. Doubles as the warm counterweight to the cool tunic so the silhouette doesn't read as "all teal." |

**Why this survives golden hour specifically:** a warm key light pushes
every hue toward orange and lifts value — exactly the failure mode
`material-palette.md` documents for the block set. Tunic teal (188°) is far
enough from the push that it can't drift into the Husk/ground band in one
scene; trim amber (36°, S74%) is saturated enough that even after a warm
lift it stays visibly more chromatic than Husk's flat khaki. The two dark
values (hair 17%, boots 17%) don't have anywhere to drift *to* — they're
already near the bottom of the value range golden hour compresses toward.

## What's actually deliverable vs. what's still a decision

- **Delivered, art-only:** this doc, 5 texture swatches
  (`assets/textures/character/`), and the reference sheet below.
- **Not decided here, on purpose:** whether this LOD body or `AUREN_BODY`
  ends up on `FlyCam`, and whether the box table above becomes real geometry
  via a new `player.rs` or gets folded into `characters.rs`. Both are Rust
  changes and both are Poppy's/the build lane's call, flagged above so it
  isn't lost.

## Update, 2026-08-18: the decision landed on `AUREN_BODY`

Director's call: the 14-box LOD table above is **not** getting built. The
113-box `AUREN_BODY` (`characters.rs:208`) is the player mesh — wiring it
onto `FlyCam` is Kevin's lane, no `.rs` touched here either. What's still
mine is the palette, so this section maps the same 6 colours from
"Palette — 6 colours" above directly onto `AUREN_BODY`'s and `equipment.rs`'s
existing `Surf` set, instead of re-deriving a palette for a body that isn't
getting built.

I read `AUREN_BODY`'s full box array plus the six `Part`s in `adventurer()`
— `HOOD_TRAVEL` (head), `JERKIN_LEATHER` (torso), `BREECHES_TRAVEL` (legs),
`GLOVES_LEATHER` (hands), `CLOAK_HALF` (back), `SWORD_SHORT` (weapon) — to
find, box by box, which `Surf::` variant each of my 6 colours already is,
or should become.

### Base body (bare skin, no gear) — 2 of 6 already exact

| My colour | Hex | `Surf::` | Where on `AUREN_BODY` | Match |
|---|---|---|---|---|
| Skin | `#D9B08C` | `Surf::Skin` | Every naked-skin box — feet, legs, torso, hands, face (~70 of 113 boxes) | **Exact**, byte-for-byte |
| Hair | `#2A1B12` | `Surf::Hair` | Crown, back mass, side locks, fringe, nape tail (`characters.rs:352-361`) | **Exact**, byte-for-byte |

`AUREN_BODY` also carries `SkinShade` (#926C52, occlusion), `SkinFlush`
(#C48772, lips/nose-tip/knuckles), `EyeWhite`, `EyePupil` — these are
value/detail variants *of* Skin, not a 7th hue, so nothing in my palette
needs to touch them. The bare waist-wrap (`ClothLinen`/`ClothWalnut`) is
also untouched — it's hidden under the Legs slot the moment anything is
equipped, which is always (see `default_loadout`).

### Equipment slots — where the other 4 colours land

| Slot | Part (`adventurer()`) | Dominant existing `Surf::` | My colour | Match |
|---|---|---|---|---|
| Head | `HOOD_TRAVEL` | `ClothEspresso` (#3A2716) shell; `LeatherStrap` (#4A3220) collar + collar roll | Legwear `#4A3220` → collar only | **Exact** (collar); shell untouched |
| Torso | `JERKIN_LEATHER` | `LeatherMid` (#59402A) jerkin body; `LeatherStrap` under-skirt; `SteelBright`/`SteelDark` pauldron; `Brass` rivets | Tunic `#2E6E78` → **no existing slot** | **New variant needed** — see below |
| Legs | `BREECHES_TRAVEL` | `LeatherMid` breeches; `LeatherDark` tall boots + knife sheath; `LeatherWorn` boot cuffs; `LeatherStrap` boot straps + thigh wrap | Legwear `#4A3220` → boot straps + thigh wrap; Boots `#2A1E16` → tall boots | Legwear: **exact**. Boots: **near** (`LeatherDark` = #33241A, ΔRGB ≈ (9,6,4)) |
| Hands | `GLOVES_LEATHER` | `LeatherDark` glove shell + thumb; `LeatherMid` bracer; `LeatherStrap` bracer strap; `Brass` buckle | Legwear `#4A3220` → bracer strap; Boots `#2A1E16` → glove shell | Legwear: **exact**. Boots: **near** (same delta as above) |
| Back | `CLOAK_HALF` | `ClothEspresso` cape body; `ClothWalnut` frayed corners; `LeatherStrap` throat strap + roll ties; `Brass` clasp; `ClothLinen` bedroll | Legwear `#4A3220` → throat strap + roll ties | **Exact** |
| Weapon | `SWORD_SHORT` | Steel/wood/leather grip — no cloth or skin surface at all | *(none of my 6)* | Out of scope — weapon materials don't take a body palette |

### Trim `#E8A23C` — also new, don't confuse it with `Brass`

`Brass` (`#C08A3E`, H33° S52% V75%) is the metal-glint accent already on
every buckle/rivet/clasp across all 5 worn slots. My Trim was designed as a
**flat, non-metallic** warm accent (H36° S74% V91% — much hotter and
brighter, and `Cloth`/`Leather`-class roughness, not `Metal`) specifically
so a sash tie or a hood's mantle edge doesn't fight the buckles for the
"this catches the light" read. It's a second warm accent, not a Brass
reskin — best home is optional new stitching on `HOOD_TRAVEL`'s mantle
edge or `CLOAK_HALF`'s frayed corners (currently `ClothWalnut`), a call for
whoever picks this up next, not decided here.

### The 2 new `Surf` variants this needs, spec'd ready to paste

Only Tunic and Trim have no existing home. Following the file's own
`p(rgb, mat, rough, metal, refl)` pattern (`equipment.rs:267`):

```rust
// Cloth: roughness pinned high, zero metal, low reflectance — same family
// as ClothWalnut/ClothLinen. The one cool-hued cloth in the set.
Surf::ClothTeal   => p((0x2E, 0x6E, 0x78), Cloth, 0.95, 0.0, 0.07),

// Warm accent, deliberately hotter/brighter than Brass and non-metallic —
// a stitched/dyed trim, not a buckle. Slightly higher refl than plain
// cloth so it still pops as "the accent" next to matte ClothEspresso.
Surf::TrimGold    => p((0xE8, 0xA2, 0x3C), Cloth, 0.90, 0.0, 0.10),
```

### Net cost to unify the palette onto `AUREN_BODY`

- **Zero Rust changes** for 3 of 6 colours: Skin, Hair (exact reuse) and
  Legwear (exact reuse of `LeatherStrap`, already the connective thread
  across all 5 worn slots).
- **Recommend reuse, not a new variant**, for Boots — `LeatherDark` is
  close enough (ΔRGB ≈ 6-9 per channel) that a near-black boot doesn't
  justify its own `Surf` entry.
- **2 new `Surf` variants** — `ClothTeal`, `TrimGold` — cover Tunic and
  Trim, the two colours that don't exist anywhere in the current set. Both
  are spec'd above, ready for whoever owns `equipment.rs` to add; no `.rs`
  touched in this doc.
