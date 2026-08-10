# Voxelforge — Character Bible

> **Quick-reference visual bible** for the hero, the Husk, and the Edhari village NPCs — silhouette,
> voxel proportions, palette, body language, and the one-line "why this reads from a black silhouette"
> for each. This is the compressed, lookup-table version of `docs/character-design.md` (the full spec
> with narrative grounding and voxel/engine constraints); read that doc for the reasoning behind every
> choice, read this one when you just need the numbers and the swatches fast.
>
> Locked against: `docs/look-bible.md` (render/palette target) and `docs/GAME-VISION.md` (pillars).
> No new art direction invented here — every palette value below is pulled straight from look-bible §4.
> Concept art: `docs/assets/characters/` (already rendered — see paths per character below).
> Owner: Monanisa (Design) · Scope: docs + assets only, no `client/src/` touched.
> Last updated: 2026-08-06 — see §7 for the CEO-requested character-art review pass.

---

## 0. The rule this whole doc follows

Look-bible §4 locks the palette: **~85% warm walnut→amber, teal (`#4FC9D6`) capped at ~10–15% of a
frame and reserved as an accent.** Character-design.md tightens that one step further for the cast:
**teal is not decoration, it's lore.** It only appears on a body when that body carries confirmed or
suspected Shaper-origin (story-bible world rule: *"The Teal is Shaper-light"*). Every palette table
below is checked against that rule explicitly — if a character has no teal, that's a design decision,
not an oversight.

Second shared rule, from look-bible §0/§1: **hard 90° voxel edges only.** No bevels, no rounding. A
character is cubes at varying scale, not a smoothed mesh wearing a blocky texture — silhouette reads
as *built*, never *sculpted*. Every entry below assumes this without repeating it.

---

## 1. Auren — Hero

**Concept art:** `docs/assets/characters/auren-hero-concept.png`

| | |
|---|---|
| **Silhouette height** | ~2 blocks — Minecraft-Steve-adjacent scale, slightly denser block-count on torso/boots for a "traveller," not a toy-simple default |
| **One asymmetric read-point** | Half-cloak slung over one shoulder, trailing past the hip — the only shape on the body that moves, so it's what the eye locks onto in motion or at distance |
| **Secondary silhouette tell** | Tool satchel + roll pack riding high on the back — "arrived from elsewhere, still travelling," distinct from a Husk's armor bulk or an NPC's stationary robe |
| **Weapon** | Single blocky short sword, leather-wrapped hilt, crossguard readable from any angle (light/heavy telegraph legibility) |
| **Body language** | Alert, forward-leaning stance — a survivor, not yet a warrior. Nothing ornamental; every silhouette element (satchel, straps, cloak) reads as gear a traveller would actually carry |

**Palette**

| Region | Hex | Look-bible source |
|---|---|---|
| Cloak / leather outer | `#3A2716` espresso | Wood-dark |
| Tunic / undergarment | `#6B4A2E` walnut | Wood-mid |
| Straps / buckle | `#4A3220` | Darker than tunic, distance-readable detail |
| Skin | `#D9B08C` warm neutral | Kept low-detail by design — face/gender is campfire-customisable (story-bible OPEN item) |
| **Ember pouch (belt)** | `#F4B860` → `#FFD98A` glow, `#C88A4A` housing | Key light / warm bounce — the *only* pre-reveal Shaper hint. Small on purpose: reads as "a warm coal they carry," not "magic item" |
| Sword blade | `#B9A98C` warm grey-beige | Stone/iron — deliberately no teal; this is a mundane weapon |
| Hair | `#2A1B12` umber-black | New (2026-08-06, in-game asset pass) — sits darker than the espresso cloak so hair reads as its own material, still inside the walnut→espresso family, no new hue introduced |

**Why it matches look-bible:** ~90% of the surface sits in the walnut→espresso range (over the 85%
floor), the one warm-glow accent is the ember pouch — same amber family as the sun key light — and
there is zero teal, which is correct: Auren's Shaper nature is *dormant*, and the palette rule says
teal only shows once that's confirmed. The design is holding a card in reserve on purpose.

**Why it's memorable from a black silhouette:** the cloak is the single asymmetric shape in an
otherwise tight, close-fitting outline — at 12–16 blocks that's the one contour the eye can lock onto
in under a second, independent of any color read.

**2026-08-06 fix (CEO character-art review, see §7):** the first-pass render didn't deliver on this
doc's own spec — flat, symmetric standing pose (no "alert, forward-leaning" read at all) and a
face rendered with full sculpted detail (eyebrows, defined nose bridge, shaded mouth) against the
"kept low-detail by design, campfire-customizable" rule two paragraphs above. Re-rendered with a
forward-leaning, weight-on-front-foot stance and a simplified low-detail face; new concept art is
live at the path above, old version archived at
`docs/assets/characters/archive-before-2026-08-06/auren-hero-concept-BEFORE.png`.

**2026-08-06 fix #2 (CEO spot-check on the pose fix above):** that re-render's hair read as a flat
rectangular slab (less volume than the original) and the face plate sat visibly off-centre under it.
Re-rendered the hair as a clustered, layered voxel mass with real thickness on top/back/sides,
hairline re-centred, face plate re-centred under it — pose, cloak asymmetry, satchel, palette and
low-detail face rule all held from the fix above, only the head/hair region changed. New concept art
is live at the path above; the flat-hair version is archived at
`docs/assets/characters/archive-before-2026-08-06/auren-hero-concept-PASS1-flathair-2026-08-06.png`.
Side-by-side: `docs/assets/characters/auren-hero-concept-before-after-2026-08-06-hairfix.png`.

---

## 2. Guard Husk — First Enemy (LOCKED)

**Concept art:** `docs/assets/characters/guard-husk-concept.png`

| | |
|---|---|
| **Silhouette height** | ~2.5 blocks (locked spec) — bulkier through the shoulders than Auren, reads "guard" at a glance |
| **Gait / stance** | Slow, lumbering; wind-up telegraphed from a static arm-cock pose held 0.8s — no rig complexity needed for a 2-hit combo |
| **The horror beat** | A completely blank, featureless voxel visor — no eyes, no mouth-slot cut into the helmet block. The emptiness *is* the design: not scary because it's monstrous, wrong because there's no one home |
| **Weapon** | Guard-issue spear, walnut haft + stone spearhead — mundane equipment, not corrupted gear |

**Palette**

| Region | Hex | Look-bible source |
|---|---|---|
| Armor plate | `#B9A98C` warm grey-beige | Stone/wall |
| Crumbling edges | `#8A7A5C` darker, desaturated | Ash-damaged block edges losing definition |
| Cracks / glow | `#C8763C` dull amber-grey — **not teal** | Early-stage, low-grade Unravelling — corrupted, not yet pure Shaper-chaos |
| Ash particles | `#E8D8B8` warm cream, low opacity | Look-bible fog/haze — the same "something burned" ash from Act 0 spawn |
| Weapon | `#6B4A2E` walnut haft, `#B9A98C` spearhead | Wood-mid + stone |

**Why it matches look-bible:** stone/wall grey-beige and wood-mid dominate — this is the game's one
character built almost entirely from the *cool-neutral* end of the palette rather than walnut, which
is intentional: a Husk is armor and stone, not skin and cloth. The crack glow stays dull amber-grey,
deliberately short of the teal ceiling, so the escalation ladder (§3–4 below) has somewhere to go.

**Why it's memorable from a black silhouette:** bulk + slow lumbering gait reads "guard" before any
detail resolves; the blank visor (a literal void cut into the silhouette, not just a dark patch)
is legible as "wrong" even in a pure black cutout, which is the point — the horror is geometric, not
textural.

**2026-08-06 fix (CEO character-art review, see §7):** the first-pass render directly contradicted
this doc's own "no eyes, no mouth-slot cut into the helmet" line — it shipped a T-shaped visor slit,
which reads as a generic knight helm and undoes the entire "no one home" horror beat this character
exists for. Re-rendered with a fully smooth, blank stone slab across the whole face, zero negative
space. New concept art is live at the path above, old version archived at
`docs/assets/characters/archive-before-2026-08-06/guard-husk-concept-BEFORE.png`.

---

## 3. Elder Maren — Village NPC (LOCKED)

**Concept art:** `docs/assets/characters/elder-maren-concept.png`

| | |
|---|---|
| **Silhouette height** | ~2 blocks, rounded/stationary-read shape |
| **Read-point** | Wide-based shawl + layered robe silhouette — no weapon, no aggressive lines, immediately reads "non-combatant, safe" against the Husk/Warden's blocky-aggressive stance |
| **Third contact point** | Walking stick — reinforces "elder" before any face detail resolves |
| **Body language** | Stooped, gentle, holds the lantern forward — a keeper, not a fighter |

**Palette**

| Region | Hex | Look-bible source |
|---|---|---|
| Robe / shawl | `#6B4A2E` walnut, `#3A2716` espresso trim | Wood-mid / wood-dark — same village-native family as Auren |
| Hair | `#C9C0B4` warm grey-white | The only near-desaturated tone on her — reads as age without going cool/lifeless |
| **Forge-ember lantern** | `#F4B860` → `#FFD98A` glow | Key light — visual rhyme with the campfire and Auren's belt-ember: three characters, one motif |

**Why it matches look-bible:** textbook 85%+ walnut/espresso coverage, warm-glow accent from the
lantern instead of teal — she is unambiguously village-native, and the palette says so before she
speaks a line. No teal anywhere, correctly: she is not Shaper-marked (in the Milestone-1 slice).

**Why it's memorable from a black silhouette:** rounded, wide-base shape plus a third ground-contact
point (the stick) reads "elderly, stationary, safe" as a pure outline — the opposite silhouette
grammar from every combat character in the cast, which is itself the tell.

**2026-08-06 fix (CEO character-art review, see §7):** the first-pass render's walking stick was
blended into the folds of the robe — at silhouette scale the "third contact point" this table
promises effectively disappears, so the spec wasn't actually being delivered in the art. Re-rendered
with a thicker cane that extends visibly past the robe hem and past the body outline, so it reads as
its own separate shape even in a pure black cutout. New concept art is live at the path above, old
version archived at `docs/assets/characters/archive-before-2026-08-06/elder-maren-concept-BEFORE.png`.

---

## 4. Toma — Village NPC (canon, locked 2026-07-31)

**Concept art:** `docs/assets/characters/toma-concept.png`

| | |
|---|---|
| **Silhouette height** | ~1.2 blocks — deliberately the shortest character in the cast, so their presence in any frame reads "child" instantly |
| **Proportions** | Bigger head-to-body ratio than the adult cast — voxel-blocky "small = few, chunky blocks," not a scaled-down adult rig |
| **Read-point** | The carved wooden toy, held in both hands |
| **Body language** | Small, slightly wary/curious posture consistent with a child in a village that was just attacked |

**Palette**

| Region | Hex | Look-bible source |
|---|---|---|
| Overalls | `#3A2716` espresso, patched with mismatched `#6B4A2E` panel | Wood-dark/wood-mid — the patch sells "ordinary village kid, hand-me-down clothes" with zero dialogue |
| Shirt | `#B9A98C` warm grey-beige | Stone/wall-family neutral, keeps the palette in-family with Maren/Auren |
| **Carved wooden toy** | `#6B4A2E` walnut, **no glow** | The one important object in the cast that stays plain wood on purpose — see below |

**Why it matches look-bible:** full walnut/espresso/grey-beige coverage, zero teal, zero emissive —
this is the most "look-bible-default" palette in the entire cast, which is correct: Toma is the
control case, the character with nothing supernatural attached, so the reader's eye has a calibrated
baseline before it sees a glowing ember pouch or a cracked-teal Husk.

**Why it's memorable from a black silhouette:** height alone does the work — at roughly half the
height of every adult in the cast, Toma reads as "child" in a pure silhouette before any other design
element resolves, which raises stakes in any frame they're standing in.

**Design note on the toy's non-glow:** every other block-crafted object in the doc set (campfires,
gate sigils, Shaper Fragments) carries visible Forge-energy per look-bible's emissive rule (checklist
#12: emissive = narrative, never decoration). Toma's toy is the deliberate exception — a child echoing
the act of Shaper-building with zero awareness of what that means. The absence of glow, on an object
that in any other hands in this world would have one, is the entire point.

---

## 5. Escalation ladder (reference — Warden & Architect)

Not "village NPCs," but included here because they're already built, already validated against
look-bible, and they're the clearest proof the palette rule scales: **the same escalation idea (a
person losing cohesion to the Unravelling) told entirely through how much teal has crossed into an
otherwise warm palette.**

| Character | Height | Concept art | Cracks/glow | Teal coverage |
|---|---|---|---|---|
| Guard Husk (§2 above) | 2.5 blocks | `guard-husk-concept.png` | `#C8763C` dull amber-grey | None |
| **The Warden** | 4–5 blocks | `docs/assets/characters/the-warden-concept.png` | Mixed `#C8763C` amber fading into `#4FC9D6` teal | Partial — the literal visual midpoint |
| **The Architect** | 8–10 blocks | `docs/assets/characters/the-architect-concept.png` | `#4FC9D6` teal, full saturation, near-black `#1E1A1C` body | Full — the only character allowed to break the 85%-warm rule, because it is pre-Edhari, pre-human |

Confirmed against the rendered concept art: the Warden's orbiting detached fragments and mixed-crack
glow are present and read clearly at silhouette scale; the Architect's near-black body with pure-teal
veins and a hot-white chest core is the single coldest, darkest image in the entire character set —
correctly so, since look-bible's "always warm" rule is written with this one exception carved out
(character-design.md §2.3).

---

## 6. Cross-character palette audit

| Character | Warm coverage | Teal? | Passes look-bible §4 (85% warm / teal-capped)? |
|---|---|---|---|
| Auren | ~90% | No (dormant) | ✅ |
| Guard Husk | ~80% (stone-neutral, not walnut — by design) | No (dull amber only) | ✅ |
| Elder Maren | ~90% | No | ✅ |
| Toma | ~95% | No | ✅ |
| The Warden | ~60% | Partial (intentional escalation) | ✅ — reads as the midpoint, not a violation |
| The Architect | ~0% (deliberate exception) | Full | ✅ — look-bible's one carved-out exception, justified in-doc |

Every character with teal on their body is Shaper-marked or Shaper-adjacent; every village-native
character (Auren, Maren, Toma) carries zero teal. The rule from character-design.md §0.2 holds with
no exceptions across the cast.

---

## 7. 2026-08-06 CEO character-art review — 3-axis pass

CEO flagged the cast wasn't landing yet and handed Monanisa a character-art lane
(`assets/` character portion + this doc, plus the existing `client/src/settings_menu.rs` lane).
Every character above was re-scored eyes-on against three axes: **(1) does the silhouette read
black-cutout**, **(2) do proportions/pose sell the personality**, **(3) does the material/palette
sit inside the golden-hour look or read as flat generic blocks. Full cast (Auren, Guard Husk, Elder
Maren, Toma, Warden, Architect) was reviewed; axis 3 was already solid across the board — the
§6 palette audit holds — so every finding landed on axes 1–2.

**The 3 fixes made (worst-first):**

| Character | Axis | Problem found | Fix |
|---|---|---|---|
| Guard Husk | 1 (silhouette / spec) | Rendered T-shaped visor slit — a literal knight helm, not the "completely blank, no eyes, no mouth-slot" the doc itself specs. Undid the character's entire horror beat. | Fully blank stone slab, zero negative space |
| Auren | 2 (pose/personality) | Flat, symmetric standing pose — no "alert, forward-leaning survivor" read at all; face over-detailed against the "low-detail, campfire-customizable" rule | Forward-leaning, weight-on-front-foot stance; simplified face |
| Elder Maren | 1 (silhouette) | Walking stick blended into robe folds — the "third contact point" this doc promises doesn't actually read at silhouette scale | Thicker cane, extends past the robe hem and body outline |

**Not touched (checked, passed):** Toma — strongest read in the cast, no changes needed. Warden /
Architect — escalation ladder already reads correctly per §5/§6, no changes needed.

**Before/after proof (real renders, not mockups):**
- `docs/assets/characters/guard-husk-concept-before-after-2026-08-06.png`
- `docs/assets/characters/auren-hero-concept-before-after-2026-08-06.png`
- `docs/assets/characters/elder-maren-concept-before-after-2026-08-06.png`
- `docs/assets/characters/auren-hero-concept-before-after-2026-08-06-hairfix.png` — Auren hair/face
  fix #2 (§1), spotted on CEO re-check of the fix above

Old versions archived (not deleted) at `docs/assets/characters/archive-before-2026-08-06/`.
The three `*-concept.png` files at the top-level path (referenced per-character above) now point
at the fixed art.

**Lane note:** this pass only touches concept art + this doc, same as every prior entry in this
file. Flamingo's `docs/character-look-contract.md` measures the *in-engine* character against a
contract; `client/src/hero.rs`, `client/src/look.rs`, `client/src/anim.rs` were not opened or
edited. Whoever owns those files should treat this pass as the updated art reference to build
toward, not a code change to merge.

---

*Design: Monanisa. Full narrative grounding and voxel/engine constraints per character live in
`docs/character-design.md`. Palette source of truth: `docs/look-bible.md` §4. Concept art already
rendered and verified against spec — see per-character paths above. Not final in-engine assets;
engine implementation is Poppy's lane.*
