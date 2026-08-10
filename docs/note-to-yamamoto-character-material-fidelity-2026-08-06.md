# Character fidelity pass — material/silhouette fixes for `anim.rs`

**For:** Yamamoto (owner of `client/src/anim.rs`, per `docs/LANES.md`)
**From:** Monanisa (Design) — CEO-assigned character fidelity pass against `docs/character-bible.md` +
`docs/character-design.md`
**Status:** Read-only review — no `client/src/` file was touched (build lock held by Rose this round,
and `anim.rs` is your lane). Every fix below is a value/one-line change for you to land, not a diff I made.
**Scope check:** I did not open or edit `client/src/look.rs`.

**Correction (2026-08-07):** the line numbers in the first version of this doc were wrong — they were
computed while `anim.rs` had unrelated in-flight edits landing on top of it (a `Clash` pose feature),
which shifted most of the blocks below by 5-7 lines, and I asserted "verified" without re-checking after
that shift. Every citation below has been re-read line-by-line off the current on-disk working tree
(not grep, not memory) as of this correction. Re-check against your own tree before pasting regardless —
if another lane has landed edits since this pass, the numbers can drift again.

## Why this exists

Director asked why the main cast still doesn't read AAA at medium shot despite the concept-art palette
being locked (character-bible.md §7). Answer: **the compiled rig in `anim.rs` never received the
character-bible palette or the LOCKED Husk face fix — it's still wearing the pre-bible placeholder
materials.** The concept art and this doc were never the problem; the gap is entirely between the art
spec and what `init_rig_assets`/`build_rig` actually assign. Every finding below is verified by reading
`client/src/anim.rs` directly (current tree, not the concept PNGs), ranked worst-first by what a player
actually sees at combat/medium distance.

---

## 1. 🔴 Guard Husk still spawns the visor-slit the LOCKED fix removed — `anim.rs:920-926`

**This is the highest-impact bug in the cast.** character-bible.md §2 (LOCKED, CEO-approved
2026-08-06): *"a completely blank, featureless voxel visor — no eyes, no mouth-slot cut into the helmet
block... fully smooth, blank stone slab across the whole face, zero negative space."* The concept art
was re-rendered specifically to fix this.

`build_rig` is generic across both actors and spawns `p.face` unconditionally:

```rust
// anim.rs:920 (comment) / 921-926 (call) — runs for BOTH Player and Husk, no actor gate
// The dark face block keeps the old "which way am I looking" read (local -Z).
skin(
    &p.face,
    &p.trim,
    Transform::from_xyz(0.0, d.face_y, d.face_z),
    head,
);
```

`p.face` for the Husk is a real, separately-colored box (`anim.rs:762`,
`face: meshes.add(Cuboid::new(0.34, 0.08, 0.06))`) painted in `p.trim` — a distinct dark color from the
head (see #3 below) — sitting on the front of the head at `face_y=0.30, face_z=-0.24`. That's a visible
horizontal slit on the Husk's face in every build. It's the exact "T-shaped visor slit... generic knight
helm" bug the bible's §2 fix already called out and fixed in the art — just never fixed in code, so the
in-game Husk still has it.

Unlike `p.face` for the Husk, note `cloak_anchor` a few lines earlier (`anim.rs:881`,
`let cloak_anchor = if actor == Actor::Player {`) already does the right thing. The face spawn needs the
same actor gate.

**Fix:**
```rust
if actor == Actor::Player {
    skin(&p.face, &p.trim, Transform::from_xyz(0.0, d.face_y, d.face_z), head);
}
```
(Keeps the Player's directional face-accent block exactly as-is — that one's correct per bible §1, it's
only the Husk copy that violates the LOCKED spec.)

---

## 2. 🔴 Player torso/sleeves are still the pre-bible neon orange, not the walnut/espresso palette — `anim.rs:734-738`

Torso + both sleeves (the largest single-color mass on Auren's silhouette at medium shot) are painted
from the `cloth` material, and the comment two lines above the struct literal says exactly why it's wrong:

```rust
// anim.rs:717-718
// The avatar keeps the orange it has always worn, so the play-mode screenshots
// still read as "the same guy" — just with limbs now.
...
// anim.rs:734-738
cloth: materials.add(StandardMaterial {
    base_color: Color::srgb(0.92, 0.38, 0.16),   // #EB6129 — saturated orange
    perceptual_roughness: 0.72,
    ..default()
}),
```

`#EB6129` is a highly-saturated toy-orange with nothing in common with character-bible.md §1's tunic
color `#6B4A2E` (walnut). This single material is why Auren doesn't read as "inside the golden-hour
look" even once framing/pose/hair are all correct — the dominant surface color on the body is fighting
the palette the rest of the doc locks. This is a leftover from before the character-bible existed, never
migrated.

**Fix:**
```rust
cloth: materials.add(StandardMaterial {
    base_color: Color::srgb(0.420, 0.290, 0.180),   // #6B4A2E, character-bible §1 tunic
    perceptual_roughness: 0.85,                       // widened from 0.72 — see §6 below
    ..default()
}),
```

---

## 3. 🟠 Guard Husk's armor is a cool blue-grey, not the bible's warm grey-beige stone — `anim.rs:772-782`

Same class of bug as #2, on the second character. Bible §2: armor plate `#B9A98C` warm grey-beige
("stone/wall"), crumbling edges `#8A7A5C` darker desaturated warm-grey. Actual in-tree:

```rust
// anim.rs:772-777 (torso + sleeves)
cloth: materials.add(StandardMaterial {
    base_color: Color::srgb(0.32, 0.34, 0.40),   // #52565F — cool blue-grey
    perceptual_roughness: 0.55,
    metallic: 0.30,
    ..default()
}),
// anim.rs:778-782 (pelvis + thighs + shins)
trim: materials.add(StandardMaterial {
    base_color: Color::srgb(0.18, 0.19, 0.24),   // #2E303D — cool dark blue-grey
    perceptual_roughness: 0.50,
    ..default()
}),
```

Both are shifted toward blue, not the "cool-neutral end of the *warm* palette" the bible explicitly
calls for (§2: "stone/wall grey-beige... this is the game's one character built almost entirely from the
cool-neutral end of the palette rather than walnut" — cool-*neutral*, not cool-*blue*). Right now the
Husk reads as sci-fi/fantasy plate armor, not ash-crumbled village stone, and the cross-character
palette audit (character-bible.md §6, "Husk ~80% warm coverage") is currently false against the actual
render.

**Fix:**
```rust
cloth: materials.add(StandardMaterial {
    base_color: Color::srgb(0.725, 0.663, 0.549),   // #B9A98C, character-bible §2 armor plate
    perceptual_roughness: 0.70,
    metallic: 0.0,                                    // stone, not metal-flake
    ..default()
}),
trim: materials.add(StandardMaterial {
    base_color: Color::srgb(0.541, 0.478, 0.361),   // #8A7A5C, character-bible §2 crumbling edges
    perceptual_roughness: 0.90,                       // rougher = damaged/crumbling read
    ..default()
}),
```
Also retint `skin` (`anim.rs:783-787`, the head-cube base color, dark cool grey-green
`srgb(0.22,0.25,0.24)`) toward the same warm-neutral family, e.g. `srgb(0.30, 0.27, 0.23)` — keep its
`perceptual_roughness: 0.85` as-is, that part is already correctly matte for the "blank slab" read once
#1 above stops layering a mismatched visor box on top of it.

---

## 4. 🟠 Metallic weapons/boots will render near-black — no `EnvironmentMapLight` exists anywhere in the codebase, and this exact failure mode is already documented in `hero.rs`

Searched the whole `client/src` tree: zero uses of `EnvironmentMapLight`. `hero.rs:218-219` states the
consequence directly, for the fridge material in the golden beauty shot:
> *"Fridge: dielectric but smooth + reflective => a real specular streak from the sun (true metallic
> would render black without an env-map)."*

`anim.rs`'s character `steel` materials use real `metallic` values, which is exactly the case that
comment warns against:

| Material | Location | metallic | Affected parts |
|---|---|---|---|
| Player `steel` | `anim.rs:749-754` | `0.75` | sword blade, both boots (`anim.rs:965-970`, see #5) |
| Husk `steel` | `anim.rs:788-793` | `0.60` | spearhead, both boots |
| Husk `cloth` | `anim.rs:775` | `0.30` | torso/sleeve armor plate (also fixed in #3 above) |

Under this scene's lighting (one directional sun + ambient fill, no reflection probe), these will read
dark-to-black instead of the "warm grey-beige iron, not a Shaper artefact" the bible specifies for the
sword (§1) or a lit stone-grey spearhead (§2) — the opposite of readable material separation, and it
actively hurts the rim/edge-against-dark-background ask, since a near-black blade in front of a dark
Husk-clash backdrop disappears instead of catching a rim highlight.

**Fix (mirrors the working `hero.rs` fridge pattern exactly):**
```rust
// Player steel — anim.rs:749-754
steel: materials.add(StandardMaterial {
    base_color: Color::srgb(0.725, 0.663, 0.549),   // #B9A98C, character-bible §1 sword blade
    perceptual_roughness: 0.18,                        // tight specular streak, not a diffuse blur
    metallic: 0.0,
    reflectance: 0.7,
    ..default()
}),
```
```rust
// Husk steel — anim.rs:788-793
steel: materials.add(StandardMaterial {
    base_color: Color::srgb(0.55, 0.51, 0.46),
    perceptual_roughness: 0.20,
    metallic: 0.0,
    reflectance: 0.7,
    ..default()
}),
```
Plus `metallic: 0.0` on Husk `cloth` per #3 (the `metallic: 0.30` field is `anim.rs:775` specifically).
This also gets you a real Fresnel-driven rim highlight at grazing angles for free — Bevy's PBR already
computes that from `reflectance`, no new rim-light system needed. That answers the "edge readability
against a dark scene" part of the brief for the metal reads; cloth/skin/leather don't get a Fresnel kick
from `reflectance` in the same way, so if edge separation still looks weak once this is in and re-shot,
that's a `look.rs` ask (rim/fill light), not something I can spec blind from outside an engine session —
flagging it, not solving it here.

---

## 5. 🟡 Player boots are painted with the sword's chrome-steel material, not leather — `anim.rs:965-970`

```rust
// anim.rs:964 (comment) / 965-970 (call)
// Sole flush with the ground plane, toe protruding forward (-Z).
skin(
    &p.foot,
    &p.steel,          // <- same material as the sword blade
    Transform::from_xyz(0.0, -d.shin + d.foot_h, -0.06),
    knee,
);
```
A traveller's boots (bible §1: "denser block-count on torso/boots for a traveller read") being literal
polished/metallic steel doesn't match "lived-in traveller," and compounds #4 (boots would render dark
along with the sword). Cheapest correct fix — reuse the leather/trim bucket that already paints the
cloak and legs:
```rust
skin(&p.foot, &p.trim, Transform::from_xyz(0.0, -d.shin + d.foot_h, -0.06), knee);
```
Husk boots can stay on `steel` (guard-issue armored boots are plausible) once #4's non-metallic fix
lands there.

---

## 6. 🟡 Skin/cloth/leather roughness values are bunched too close together to read as different materials

Brief asked specifically: material layers (skin/cloth/metal/leather) must separate by *roughness*, not
just color. Current spread is too narrow to do that job:

| Actor | skin | cloth | leather(trim) | steel |
|---|---|---|---|---|
| Player (now) | 0.68 | 0.72 | 0.80 | 0.30 (metallic) |
| Player (proposed) | **0.50** | **0.85** | **0.65** | **0.18** (dielectric, #4) |
| Husk (now) | 0.85 | 0.55 | 0.50 | 0.45 (metallic) |
| Husk (proposed) | 0.85 (keep) | **0.70** | **0.90** | **0.20** (dielectric, #4) |

Player's current 0.68→0.80 span (0.12 total) puts organic skin and woven cloth in the same "matte" band
— under one key light they differ only by hue, not by specular falloff shape. Widening skin down (subtle
organic sheen) and cloth up (fully diffuse) plus giving leather its own mid-band gives three visibly
distinct specular responses instead of one. Husk's `cloth`/`trim` are currently only 0.05 apart (0.55 vs
0.50) — practically the same material; the proposed spread also does double duty for #3 (armor plate
should look less rough/more "worn stone" than the crumbling edges, which should look chalkier/rougher).

---

## Priority order to land (worst-first, per the brief)

1. §1 — Husk face-slit gate (`anim.rs:920-926`) — single highest-impact fix, reverts a LOCKED regression
2. §2 — Player torso/sleeve recolor (`anim.rs:734-738`) — single largest wrong-color surface in the cast
3. §3 — Husk armor recolor (`anim.rs:772-782`)
4. §4 — steel/metallic → dielectric+reflectance across all 3 blocks (`anim.rs:749-754`, `788-793`, `775`)
5. §5 — boots off `steel` onto `trim` (`anim.rs:965-970`)
6. §6 — roughness spread widen (bundle into the same edits as #2/#3/#4, no separate pass needed)

All six are value edits inside blocks that already exist — no new fields, no new assets, no rig
restructuring. Happy to eyeball a render once you've got a build slot and flag anything that doesn't
land right, same loop as `docs/auren-extra-parts-spec.md`.

**Re-check before pasting:** these line numbers are accurate against the tree as read on 2026-08-07.
If your working copy has moved since (another lane's edits, or your own WIP), grep the `base_color`
hexes/comments quoted above rather than trusting the numbers blind — that's exactly the mistake this
correction is fixing.

## References
- `docs/character-bible.md` §1 (Auren), §2 (Husk) — palette + the LOCKED "blank visor" spec
- `docs/character-design.md` §1, §2.1 — narrative grounding for why these hexes matter
- `client/src/hero.rs:218-229` — the existing, working non-metallic-dielectric pattern this borrows
- `client/src/anim.rs` — all line numbers above re-verified line-by-line against the current working
  tree on 2026-08-07 (see correction note at top)
