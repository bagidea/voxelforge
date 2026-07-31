# Voxelforge — Character Design

> **Visual language for every character in the vertical slice.** Silhouette, palette, and design-storytelling
> for the hero, the enemy escalation ladder, and the two story-connected village NPCs.
> Grounds every design decision in `docs/story-bible.md` (LOCKED narrative facts) and
> `docs/look-bible.md` (render/palette rules). Concept art: `docs/assets/characters/`.
> Last updated: 2026-07-31 · Owner: Monanisa (Design) · Scope: character lane only — no client code touched.

---

## 0. Shared Rules (apply to every character below)

1. **Hard 90° voxel edges only.** No bevels, no rounding, no organic curves — per look-bible §0/§1. A character
   is a stack of cubes at varying scale, not a smoothed mesh with a blocky texture. Silhouette reads as
   "built," never "sculpted."
2. **Golden-hour palette dominance.** ~85% of any character's surface area should sit in the warm walnut→amber
   range from look-bible §4. Teal (`#4FC9D6`) is reserved — **it only appears on characters of confirmed or
   suspected Shaper origin** (story-bible world rule: "The Teal is Shaper-light"). Village-native characters
   (Auren, Maren, Toma, Guard Husk) must NOT wear teal as decoration. If teal appears on a body, it is lore
   information, not styling.
3. **Silhouette-first read.** Every character must be identifiable from a pure black silhouette at combat
   distance (12–16 blocks, per first-playable-loop lock-on range) in under one second. Tested by: proportion
   (height/bulk), one asymmetric read-point (a cape, a weapon silhouette, an orbiting fragment), and a distinct
   headline shape.
4. **Emissive = narrative, not decoration.** Per look-bible checklist item #12, emissive blocks are a light
   source in Ultra tier. Every glowing element on a character (embers, cracks, cores) must be justified by
   story — nothing glows just to look nice.
5. **Engine-real texture budget.** Ultra tier target is 512² per material region (look-bible §3/§4) with
   albedo+normal+roughness. Concept art below is painted at a level of surface detail the shader stack can
   approximate — grain and crack detail should read as *texture + normal map*, not as geometry the voxel mesh
   would need extra polycount for.

---

## 1. Hero — Auren

**Concept art:** `docs/assets/characters/auren-hero-concept.png`

### Narrative grounding
Auren is a traveller who arrived in Edhari the night of the attack, searching for the sealed dungeon — not a
villager, not a stranger (story-bible §3). They carry dormant Shaper ability they don't yet know about. The
arc is **Survivor → Warrior → Shaper**, and the mechanical skill progression is meant to mirror it exactly.

### Silhouette
- Base proportions: humanoid, voxel-blocky, roughly Minecraft-Steve scale but with slightly richer
  block-density on the torso and boots for a "lived-in traveller" read rather than a toy-simple one.
- **The one asymmetric read-point:** a worn half-cloak slung over a single shoulder, trailing past the hip.
  Everything else on the silhouette is close-fitting (satchel, straps) — the cloak is the one shape that moves
  and catches wind, so it's what the eye locks onto at distance and in motion.
- A compact tool satchel + roll pack sits high on the back — visual shorthand for "arrived from elsewhere,
  still traveling," distinct from a Husk's armor-plate bulk or an NPC's stationary robes.
- Weapon: single blocky short sword (v1 has one weapon type per GAME-VISION §Non-Goals) — a leather-wrapped
  hilt with a crossguard read from any angle, so light/heavy attack telegraphs stay legible.

### Palette (hex)
| Region | Hex | Role |
|---|---|---|
| Cloak / leather outer | `#3A2716` (espresso) | Base silhouette mass, matches look-bible wood-dark |
| Tunic / undergarment | `#6B4A2E` (walnut) | Mid-value warmth, look-bible wood-mid |
| Straps / buckle | `#4A3220` | Detail read, slightly darker than tunic for contrast at distance |
| Skin | `#D9B08C` warm neutral | Kept simple/blocky — story-bible marks gender/presentation as **customisable at campfire (OPEN)**, so the face is a low-detail placeholder by design, not a final art pass |
| **Ember pouch (belt)** | `#F4B860` → `#FFD98A` glow, `#C88A4A` housing | The *one* pre-reveal hint of latent Shaper ability. Small, easy to miss on a first playthrough — it should NOT read as "magic item," it should read as "a warm coal they carry." Payoff lands when the dungeon gate sigil pulses for Auren alone (story-bible §3, Act 1). |
| Sword blade | `#B9A98C` warm grey-beige | Stone/iron read, no teal — this is a mundane weapon, not a Shaper artefact |

### Voxel/engine constraints respected
- Cloak is a single flat-panel voxel slab with a simple 2-frame flutter animation budget in mind (no cloth-sim
  ask) — reads as motion without physics cost.
- Ember pouch is one emissive block, small enough that Lite tier (bloom-only, no cast light) loses nothing —
  the read survives tier downgrade per look-bible §3 "same look, cheaper budget" test.

---

## 2. Enemy Escalation Ladder — Husk → Warden → Architect

All three are **the same design idea at three severities**: a person or Shaper losing cohesion to the
Unravelling (story-bible §2). The escalation is not "bigger monster," it's "further along the same
disintegration" — cracks widen, color drifts from dull amber toward pure teal, and body-loss becomes visible
as detached floating fragments rather than just crumbled edges. A player who fights all three in order should
feel the throughline without a word of dialogue.

### 2.1 Guard Husk (first enemy — LOCKED, first-playable-loop §Act 3)

**Concept art:** `docs/assets/characters/guard-husk-concept.png`

**Narrative grounding:** A former villager, partially Unravelled, still moving but no longer themself
(story-bible §4). The player is fighting a neighbour and the weight should be there even without exposition.

**Silhouette:** ~2.5 blocks tall (locked spec), bulkier than Auren through the shoulders and stance — reads as
"guard," not "traveller," at a glance. Slow lumbering gait per spec.

**The horror beat:** a completely blank, featureless voxel visor. No eyes, no mouth-slot. The emptiness *is*
the design — a Husk isn't scary because it's monstrous, it's wrong because there's no one home.

**Palette (hex):**
| Region | Hex | Role |
|---|---|---|
| Armor plate | `#B9A98C` warm grey-beige | Village-guard stone/iron, matches look-bible stone/wall color |
| Crumbling edges | `#8A7A5C` (darker, desaturated) | Ash-damaged block edges losing definition |
| Cracks / glow | `#C8763C` dull amber-grey (NOT teal) | This is early-stage, low-grade Unravelling energy — corrupted, not yet pure Shaper-chaos. Reserve full teal for Warden/Architect so it stays meaningful. |
| Ash particles | `#E8D8B8` warm cream, low opacity | Ties to look-bible fog/haze palette — the same "something burned" ash from Act 0 spawn |
| Weapon | `#6B4A2E` walnut haft, `#B9A98C` spearhead | Guard-issue equipment, mundane make |

**Voxel/engine constraints:** patrol animation only needs a 2-hit telegraphed combo (first-playable-loop
combat spec) — no complex rig, wind-up readable from a static arm-cock pose held 0.8s.

---

### 2.2 The Warden (Chapter 1 boss — story-bible §5 Act 1)

**Concept art:** `docs/assets/characters/the-warden-concept.png`

**Narrative grounding:** A former village elder, almost fully Unravelled, still wearing the clothes of a
village craftsperson (story-bible §4, §5). Defeating it drops the first Shaper Fragment and triggers Auren's
first involuntary shaping — the game's first hard confirmation that something is different about them.

**Silhouette:** ~4–5 blocks tall, visibly larger and heavier than a Husk. The key new read: **detached voxel
fragments orbiting the body** — chunks that have fully separated from the silhouette and hang nearby, slowly
rotating. This does double duty as character design and combat readability: fragment drift can telegraph
attack wind-up direction without needing a new animation language.

**Palette (hex):**
| Region | Hex | Role |
|---|---|---|
| Remaining clothing | `#3A2716` ashen espresso, desaturated | Scorched remains of craftsperson apron/tool-belt — barely recognisable, which is the point |
| Body-stone showing through | `#6B4A2E` → `#8A7A5C` | Where clothing has worn away entirely |
| Cracks / glow | **Mixed** `#C8763C` amber fading into `#4FC9D6` teal | The literal midpoint of the escalation — some cracks still read as corrupted-amber, others have crossed over to true Shaper-teal. This is the visual argument that the Warden is closer to the Hollow's nature than a Husk is. |
| Orbiting fragments | `#8A7A5C` stone, faint teal edge-glow | Detached but not gone — "the world remembers your shape" applies in reverse here: pieces that no longer remember being part of a person |

**Voxel/engine constraints:** orbiting fragments are a small fixed pool (6–10 detached voxel clusters) on a
simple orbit path — cheap to animate, expensive-looking payoff. Avoid particle-system dependency; these should
be real (if simple) rigged pieces so they can be used as telegraph readers.

---

### 2.3 The Architect (Chapter 4–5 boss, Act 2 — story-bible §5)

**Concept art:** `docs/assets/characters/the-architect-concept.png`

**Narrative grounding:** The Hollow's semi-conscious fragment, manifesting as a massive Shaper-form. Story-bible
is explicit: *"The fight is beautiful and terrible."* This is the design's job — the player's first real look
at what a Shaper actually is, and it must not read as a generic dark-magic final-boss silhouette. It should
read as **art that stopped mid-creation**, not as a monster.

**Silhouette:** ~8–10 blocks tall, tall and vertical rather than wide/hulking (deliberate contrast against the
Warden's bulk — this is a different kind of threat, not just "bigger"). Parts of the body trail off into
incomplete, floating geometric fragments instead of finishing as solid form — literally an unfinished sculpture,
echoing the sealed "cradle" room from story-bible §5 (something was being made when the grief hit, and it was
never finished — the Architect's own body carries that same unfinished quality).

**Palette (hex):**
| Region | Hex | Role |
|---|---|---|
| Core body | Near-black desaturated stone (`#1E1A1C`-range) | Deliberately the darkest character in the cast — everything else in the game is warm; this is the one figure allowed to break that rule, because it is pre-human, pre-Edhari |
| Cracks / veins | `#4FC9D6` teal, full saturation | Unlike the Husk/Warden's mixed or dull glow, this is **pure, undiluted Shaper-light** — confirms in the visual grammar established by story-bible's world rules that this is the genuine article, not a corrupted echo |
| Chest core | `#4FC9D6` → near-white hot centre | The Forge itself, still burning inside — this is the visual seed for both Ending A (drive Fragments into it) and Ending B (complete what it was building) |
| Chaos particles | Dark grey-violet, low-opacity swirl | Raw un-crystallised chaos (story-bible §2: "the Forge... conversion of chaos into form") — NOT ash. Ash is what burns; this is what was never shaped in the first place. Keep visually distinct from Husk/Warden ash particles. |

**Voxel/engine constraints:** this is the one character where an Ultra-tier-only budget is acceptable —
per GAME-VISION, v1 targets Ultra desktop only, so full teal emissive-as-light + volumetric chaos particles are
in scope. If a Lite tier is ever built (currently a non-goal), this fight is the first candidate for a
particle-count pass.

---

## 3. Village NPCs — Two Story-Connected

### 3.1 Elder Maren (LOCKED role — first-playable-loop §Act 2)

**Concept art:** `docs/assets/characters/elder-maren-concept.png`

**Narrative grounding:** Behind the dungeon gate, gives Auren their first objective, withholds the full truth
out of care rather than deception (story-bible §6). In the Milestone-1 slice she is only ever seen as *a
shadow and one hand reaching through the gap* — this full-body design exists for her eventual full reveal
(Chapter 2+) and any earlier promotional/cutscene use, kept consistent now so nothing has to be redesigned
later.

**Silhouette:** Rounded, stationary-read shape — shawl and layered robe silhouette (wide base, no weapon, no
aggressive lines) immediately reads "non-combatant, safe" against Husk/Warden's blocky-aggressive stances.
Walking stick adds a third contact point to the silhouette, reinforcing "elder" at a glance before any face
detail resolves.

**Palette (hex):**
| Region | Hex | Role |
|---|---|---|
| Robe / shawl | `#6B4A2E` walnut, `#3A2716` espresso trim | Same village-native warm palette as Auren — visually confirms "she belongs to this place," unlike the teal-marked Shaper enemies |
| Hair | Warm grey-white (`#C9C0B4`) | Only near-desaturated tone on her — reads as age without going cool/lifeless |
| **Forge-ember lantern** | `#F4B860` → `#FFD98A` glow | She is a keeper of a Forge-point herself — this lantern is the visual rhyme with the campfire (Forge-point cosmology, story-bible §2) and with Auren's belt-ember. Three characters, one motif, escalating in what it means. |

**Design rationale — why the lantern, not a staff-topper or a book:** the campfire is the game's rest/heal
anchor and the strongest recurring warm-light motif in the loop (first-playable-loop §Act 0, §Act 4). Giving
Maren a hand-carried version of that same light makes her legible as "protector of Forge-points" purely
visually, before she says a word — and it's the detail that should make an attentive player suspect she knows
more than her three lines let on.

---

### 3.2 Toma — second story-NPC ⟨canon as of 2026-07-31⟩

**Concept art:** `docs/assets/characters/toma-concept.png`

> **Canon status:** signed off by the story-bible owner (Rose) as of 2026-07-31 — see
> `docs/story-bible.md` §3 Supporting Cast. Originally proposed here to satisfy "2 village NPCs connected to
> the story," grounded in *existing* LOCKED material rather than inventing new lore threads; the story-bible
> owner's review confirmed no LOCKED fact was altered. Kept below as the original design-proposal rationale.

**Proposed narrative grounding:** first-playable-loop §Act 1 already places a specific, discoverable object in
the world: *"A child's drawing on a wood block wall — figures fleeing something large."* That drawing implies a
child without naming one. Story-bible §5 (Act 2) separately establishes three survivors held in stasis by
Unravelling energy, currently unnamed. **Proposal: the child who drew that picture is one of the three.** Naming
them now (Toma) turns a static environmental prop into a through-line: the player finds evidence of a specific
person in the first ten minutes, and — if this thread is picked up in Act 2 design — finds out what happened to
them. No mechanics or LOCKED facts change; this only assigns identity to something the story-bible already
implies exists.

**Silhouette:** Small — deliberately the shortest character in the cast, well under Husk/Auren height, so their
presence in any frame reads immediately as "child" and raises stakes without a line of dialogue. Rounder
proportions (bigger head-to-body ratio) than the adult cast, consistent with voxel-blocky "small = few, chunky
blocks" rather than a scaled-down adult rig.

**Palette (hex):**
| Region | Hex | Role |
|---|---|---|
| Overalls | `#3A2716` espresso, patched with a mismatched `#6B4A2E` panel | The patch is the story detail — a hand-me-down or self-mended garment, sells "ordinary village kid" without needing dialogue |
| Shirt | `#B9A98C` warm grey-beige | Keeps them visually part of the same warm village palette as Maren/Auren — no teal, not Shaper-touched (relevant if this thread is ever developed further) |
| **Carved wooden toy** | `#6B4A2E` walnut, no glow | Deliberately *not* emissive — this is the one important object in the cast that stays plain wood. It's a child mimicking the act of Shaper-building (story-bible §2: descendants "performing a diminished echo of a divine act") completely unaware of what that means. The absence of glow is the point — innocence versus the weight the player already understands. |

**Design rationale:** every other "block-crafted object" motif in the doc set (campfires, gate sigils, Shaper
Fragments) carries visible Forge-energy. Toma's toy is the one exception, and it should stay that way even if
this character is developed further — it's the clearest, gentlest expression of the game's theme ("creation and
destruction are the same act," story-bible §1) landing on someone who has no idea they're part of it.

---

## 4. Cross-Character Summary Table

| Character | Height | Silhouette read-point | Teal present? | Status |
|---|---|---|---|---|
| Auren (hero) | ~2 blocks | Asymmetric half-cloak | No (only latent ember, amber) | Design proposal — face/gender deliberately left customisable (story-bible OPEN item) |
| Guard Husk | 2.5 blocks (locked) | Blank visor, armor bulk | No — dull amber-grey only | LOCKED role, this doc = visual spec |
| The Warden | 4–5 blocks | Orbiting detached fragments | Partial — mixed amber/teal | Design proposal, grounded in LOCKED story-bible boss description |
| The Architect | 8–10 blocks | Unfinished/incomplete body shape | Full — pure teal | Design proposal, grounded in LOCKED story-bible boss description |
| Elder Maren | ~2 blocks | Rounded robe shape + carried lantern | No | LOCKED role, this doc = visual spec |
| Toma | ~1.2 blocks | Smallest in cast, plain wooden toy | No | **Canon, locked** (2026-07-31) |

---

*Design: Monanisa. Grounded in `docs/story-bible.md` (Sahara/CEO-owned narrative canon) and
`docs/look-bible.md` (render target). Concept art generated for internal art-direction reference — not final
in-engine assets. Engine implementation: Poppy.*
