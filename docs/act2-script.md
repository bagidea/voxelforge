# Voxelforge — Act II Script & Story-Data Schema

> **Engine contract, drafted for Sun/Rose to build `assets/story/act2.json` against.** Same
> schema as Act I (`docs/act1-script.md` §3–11) — this doc does not redefine that schema,
> only supplies Act II's content against it, plus the small number of proposed extensions
> flagged in §0. The JSON will be the source of truth once built; this doc is how you read
> the intent behind it.
>
> Owner: **Monanisa** (Design — picking up the story lane; Rose owns the Act I data +
> schema and is unavailable this pass) · Canon: `docs/story-bible.md` · Precedent:
> `docs/act1-script.md`, `assets/story/act1.json`
> Last updated: 2026-08-14 · In-game language: **English**

---

## 0. Status & coordination notes

- **No `assets/story/act2.json` exists yet.** This document is the design source Rose/Sun
  build it from — same relationship `act1-script.md` had to `act1.json` before that file
  existed. Follow the schema already defined in `docs/act1-script.md` §3–11 verbatim; it is
  not repeated here except where Act II needs something new (below).
- **No `maps/hollow_reach.json` exists yet either.** Every `world_position` in this doc is
  **provisional** — placed to be internally consistent (relative distances, a coherent
  descent) but **not verified against a real map file**, unlike Act I's coordinates (which
  were checked against `maps/edhari.json`). Shiba/Sun: treat every position below as a
  starting point to snap once the map is built, the same way Act I positions were snapped
  to real blocks. Do not ship these numbers as verified.
- **Where Act II starts, mechanically.** `act1.json`'s `act_end` hands off with
  `"unlock_act": "act2"` after `q5_sigil_that_knew_you`. This doc's `start_quest`
  (`q6_the_warden`) is the first thing that happens after the Act I cliffhanger — no gap.
- **The Warden fight resolves a gap between the story bible and the shipped Act I data.**
  `story-bible.md` §5 lists **The Warden** as Act I's chapter boss ("former village elder,
  almost fully Unravelled... drops the first Shaper Fragment"), and §7 marks it **LOCKED**.
  But the actual `act1.json` five-quest chain never fights it — Act I's shipped vertical
  slice ends at the gate opening, before that encounter. Nothing in the bible is being
  changed or contradicted: the Warden fight simply hadn't happened in playable content yet.
  It is placed here, as `q6`, because this is the first opportunity — narratively it is
  still the closing beat of "the Descent," mechanically it ships as Act II's opening quest.
  Story-bible §5 gets a one-line clarifying note for this in the doc-sync pass below (§12).
- **New NPCs — Bracken and Sela — are proposed canon**, following the same bar Toma cleared
  (`story-bible.md` §3): grounded in *existing* LOCKED material, not inventing new threads.
  Story-bible §5 already establishes "three imprisoned survivors, one of them Toma" without
  naming the other two. Bracken and Sela fill that already-implied gap. Bracken is proposed
  as the mason who dropped `lore_masons_chisel` (Act I, "as if set down mid-stroke" — payoff:
  he was taken mid-stroke, not killed). Sela is proposed as the keeper of the offering rite
  behind `lore_well_inscription` / `dlg_well_echo` — the voice the `echo` NPC is an
  impression *of*. Neither is Shaper-touched; neither carries teal. No LOCKED fact altered.
- **Proposed schema addition — a `cutscene` block on individual quests.** `opening` and
  `act_end` already carry a `beats[]` array for wordless staged sequences (§4, §10 of
  `act1-script.md`). Act II needs the same thing mid-chain once: the Warden's death triggers
  "Auren's hand involuntarily shapes a small block from nothing... the game immediately
  moves on. No dialogue about it" (`story-bible.md` §5). That's not a `dialogue` node (there
  are no lines) and not a reward (it's not mechanical) — it's a beat. Proposed shape,
  mirroring `opening`/`act_end`:
  ```jsonc
  // optional, on any quest object
  "cutscene": {
    "trigger_objective": "o3_defeat",   // plays once this objective completes
    "beats": [ "prose stage direction, no dialogue" ]
  }
  ```
  This is additive and forward-compatible per the existing `_comment_*`/unknown-key rules —
  flagging it here per the engine checklist's own instruction ("if you need a field the
  schema lacks, add it and bump `schema_version`; tell Rose"). Not blocking: if Sun would
  rather represent this as a `dialogue` node with `lines: []` and no nameplate, that works
  too and needs no schema change. Design intent either way: **wordless.**
- **Shaper Fragments are modelled as `set_flag`s** (`shaper_fragment_1` / `_2` / `_3`), the
  same pattern Act I already uses for narrative state (`campfire_anchored`, `maren_met`).
  No inventory/count field is proposed — if Sun wants one for a future Act III tally UI,
  that's a separate, later schema addition.
- **Maren's channel changes.** The gate is sealed behind Auren now (`act1.json` `act_end`
  beat 6: "the gate seals shut... the way a door closes on a room someone means to come back
  to"). She can no longer speak through gate-stone. This doc has her speak through
  **campfires** instead — already-locked cosmology (`story-bible.md` §2: "Campfires are
  Forge-points... you are re-formed here"; §6: "every campfire you activate is you,
  rebuilding"). No new mechanic: it's the existing Forge-point rule applied to a new
  situation, made explicit in dialogue (`dlg_maren_after_warden`, below).
- **Do not edit `client/src`.** Design/data lane only, same boundary Rose held for Act I.

---

## 1. The twenty-five minute experience (what the player feels)

Act II is three story chapters (`story-bible.md` §5, Chapters 3–5) compressed into one
quest chain — roughly double Act I's five-quest slice. Times are design-target pacing, not
milestone-locked the way Act I's 5-minute table was.

| Time (approx.) | Quest | What happens | Mechanic taught (silently) |
|---|---|---|---|
| 0:00–4:00 | **q6 The Warden** | Through the gate, down into the first hall. The Warden — a former elder, barely a shape anymore — still walks its old rounds. Fight, fall if you fall, rise at a new fire. Its death: Auren's hand shapes a small block from nothing. No one comments. | combat escalation · first Shaper Fragment · the seed pays off silently |
| 4:00–8:00 | **q7 Those We Left Below** | First Shaper Chamber: a mural of someone radiant, building this world with joy. A stasis pod, warm-lit from inside. Inside it: **Bracken**, the mason — alive, three centuries late for finishing a wall. | interact · environmental lore · rescue |
| 8:00–12:00 | **q8 The Rite They Kept** | Second Shaper Chamber: the joy continues, generation after generation, and something small and deliberate is being shaped apart from the rest. A second pod: **Sela**, who led the offerings. She recognises her own carved sigil on the wall. | environmental lore · rescue · dialogue |
| 12:00–16:00 | **q9 The Warm Thing** | Third Shaper Chamber: the mural cracks — literal fractures through the stone joy. A third pod, smaller than the others. **Toma** — taken from Edhari after Auren went down, while the promise "I'll come back for you" was still true. | environmental lore · rescue · emotional payoff of the Act I thread |
| 16:00–20:00 | **q10 The Cradle** | A sealed personal chamber. Inside: a half-built structure, unmistakably cradle-shaped, unmistakably the same shape the village once offered up (Act I ledger, "a child's cradle"). Maren, through the fire: she doesn't know what it lost. She knows it hasn't stopped trying to finish it. | dialogue · thematic culmination |
| 20:00–25:00 | **q11 The Architect** | The Hollow's semi-conscious fragment wakes in the cradle-room, manifesting as a towering, half-finished Shaper-form. The fight happens inside its own unfinished work. | boss combat · Shaper Fragment 2/3 payoff |
| 25:00–27:00 | **q12 One Slow Breath** | Beyond the wreck of the fight: a chamber too large to see the walls of. Something enormous, barely awake, draws one slow breath. Cut. | culmination / cliffhanger into Act III |

**Recap beat (first 30 s, folded into `q6`'s premise, not a separate cutscene):** the gate
opened for Auren alone; it has sealed again behind them; Toma is still hidden above,
promised a return. Everything from here is underground.

**Cliffhanger:** the Hollow's full form, visible for the first time, filling a chamber too
large to see the edges of — semi-dormant, barely aware, having just lost the only piece of
itself capable of finishing anything. Three survivors are safe at a lit fire at the chamber's
edge. Edhari, and Toma's promise, are still three hundred metres of rubble away. (See
`act_end` in §10.)

---

## 2. The mystery of the Deep — layered reveals

Act I excavated the mystery of *Edhari* (five layers, fully paid off by its `act_end`). Act
II excavates a different, deeper mystery — **what the Hollow was, and what it lost** — on
its own 1–5 scale. Each `lore_item` below carries its own `lore_layer` for this new track;
do not merge the numbering with Act I's.

| Layer | What the player learns | Where |
|---|---|---|
| 1 | This place was built with joy, by someone, for people who loved it back. | Shaper Chamber I mural, Bracken's testimony |
| 2 | It built across generations, the same devotion every time — and it was working on something apart from the rest, something smaller, something careful. | Shaper Chamber II mural, Sela's testimony |
| 3 | The joy stopped mid-act. Not a war, not a monster arriving — a fracture, from the inside. | Shaper Chamber III mural (physically cracked) |
| 4 | The thing it lost and the thing it was building are the same thing — and the village's own offerings, across centuries, fed straight into that unfinished work without anyone knowing why. | The Cradle (q10), Maren's reveal, the ledger callback |
| 5 | It is not gone. It is not fully here either. It is enormous, and it is still, in whatever way something like that can be, grieving. | The Architect fight, `act_end` |

Maren still withholds by design (canon, `act1-script.md` §2/§9): she tells Auren everything
she actually knows in `q10`, and it still isn't the whole answer, because she doesn't have
it. The story-bible marks "what the Hollow lost" as deliberately **OPEN** (§7) — this act
sharpens the question without answering it. Do not resolve it here; that is the CEO's call.

---

## 3. Top-level JSON shape (unchanged from Act I)

Same shape as `act1-script.md` §3, with Act II's own values:

```jsonc
{
  "schema_version": "1.0.0",         // bump only if the proposed cutscene block (§0) lands
  "act": 2,
  "act_title": "The Truth in the Deep",
  "lang": "en",
  "map": "hollow_reach",             // NEW map — does not exist yet, see §0
  "start_quest": "q6_the_warden",

  "npcs": [ ... ],        // §5 below (adds bracken, sela; reuses maren, echo, toma)
  "regions": [ ... ],     // §7 below — all provisional/code_spawned, no map yet
  "quests": [ ... ],      // §8 below — q6 through q12
  "dialogue": [ ... ],    // §9 below
  "lore_items": [ ... ],  // §6 below
  "act_end": { ... }      // §10 below
}
```

No `opening` block: Act II has no cold open of its own — it continues directly from Act I's
`act_end`. `q6`'s `premise` carries the recap instead (§8).

---

## 4. `npcs` — speaker roster additions

Reuses `maren`, `echo`, and `toma` from Act I's roster verbatim (same `id`s — do not
redefine them, just extend the same array). Adds:

```jsonc
{
  "id": "bracken",
  "name": "Bracken",
  "role": "The village mason. One of the three Act II survivors. Proposed as the owner of lore_masons_chisel (Act I) — taken mid-stroke, not killed.",
  "voice": "Blunt, practical, a little too calm for what just happened to him — the calm of a craftsman who steadies his hands by naming what's in front of him. Talks about the chamber's stonework before he talks about himself.",
  "appearance": "Broad, heavyset, forearms scarred from decades of block-work. No teal — village-native, not Shaper-touched.",
  "canon_status": "Proposed 2026-08-14, grounded in Act I's lore_masons_chisel and story-bible §5's unnamed-survivors gap. No LOCKED fact altered."
},
{
  "id": "sela",
  "name": "Sela",
  "role": "Kept the offering rite — the voice the 'echo' NPC (Act I, dlg_well_echo) is an impression of. One of the three Act II survivors.",
  "voice": "Ritual cadence even in ordinary speech — she has said some of these words so many times they come out half-chanted. Grief for the rite's failure sits closer to the surface than fear for herself.",
  "appearance": "Older than Maren reads at a glance, though the two have never met. Carries no lantern, no light of her own — the offerings were never hers to keep warm.",
  "canon_status": "Proposed 2026-08-14, grounded in Act I's lore_well_inscription/dlg_well_echo and story-bible §5's unnamed-survivors gap. No LOCKED fact altered."
}
```

`the_warden` and `the_architect` are **code-spawned combat entities**, not roster NPCs (same
treatment Act I gave `garren_husk`) — visual specs already exist in `docs/character-design.md`
§2.2/§2.3; do not redesign here.

---

## 5. `lore_items` — the three Shaper Chambers + the Cradle

```jsonc
[
  {
    "id": "lore_chamber_one_mural",
    "kind": "mural",
    "name": "Shaper Chamber I — the raising",
    "world_position": { "x": 30, "y": -20, "z": -46 },
    "requires": null,
    "lore_layer": 1,
    "subtitle": "A carved wall, unbroken by time in a way nothing above ground was. A radiant figure lifts blocks into a skyline; small figures below build alongside it, not in fear of it.",
    "text": "This is the oldest image of the Builder anywhere in the Reach — before it was tired, before it was owed anything. It worked beside people, not above them. Whatever it became, it did not start this way.",
    "triggers_dialogue": "dlg_vision_chamber_one",
    "tags": ["shaper", "hollow", "joy"]
  },
  {
    "id": "lore_chamber_two_mural",
    "kind": "mural",
    "name": "Shaper Chamber II — the long devotion",
    "world_position": { "x": 30, "y": -34, "z": -71 },
    "requires": null,
    "lore_layer": 2,
    "subtitle": "The same scene, repeated in relief down the wall's whole length — generation after generation of the same figures, aging and renewing, the Builder unchanged among them. In one panel, set apart from the rest, it shapes something small with both hands, carefully, alone.",
    "text": "Every other panel shows building at scale — walls, towers, whole streets. This one panel is the only time it is shown making something a person could hold. It is not labelled. It did not need to be, to whoever carved this.",
    "triggers_dialogue": "dlg_vision_chamber_two",
    "tags": ["shaper", "hollow", "cradle-seed"]
  },
  {
    "id": "lore_chamber_three_mural",
    "kind": "mural",
    "name": "Shaper Chamber III — the fracture",
    "world_position": { "x": 30, "y": -48, "z": -96 },
    "requires": null,
    "lore_layer": 3,
    "subtitle": "The relief is cracked here — not chipped by accident, but split through, as if the stone itself flinched. The small held thing from the last panel is unfinished, dropped mid-shape. The Builder stands alone; every other figure is gone from the frame.",
    "text": "Nothing here says what happened. The absence is the statement — three hundred generations of company, and then, in one panel, none. It did not stop building because it was attacked. It stopped because it was, all at once, alone.",
    "triggers_dialogue": "dlg_vision_chamber_three",
    "tags": ["shaper", "hollow", "grief"]
  },
  {
    "id": "lore_ledger_echo",
    "kind": "object",
    "name": "A child's cradle, finished",
    "world_position": { "x": 31, "y": -60, "z": -110 },
    "requires": "q9_the_warm_thing",
    "lore_layer": 4,
    "subtitle": "Just inside the sealed chamber, dwarfed by everything around it: one small, complete, ordinary wooden cradle — village work, not Shaper work, generations old, left here like an offering because that is exactly what it was.",
    "text": "The village ledger (Act I: lore_village_ledger) records a cradle given as tribute, one line among many. It was never a strange gift. It was the one offering that happened to match, by pure grieving coincidence, what the thing underneath the village was already, uselessly, trying to finish for itself.",
    "tags": ["offerings", "cradle", "hollow", "theme"]
  }
]
```

**For Shiba (once `maps/hollow_reach.json` exists):** murals are `kind: mural`, dressed the
same way Act I's `lore_guardpost_mural` was — a wall relief, not a freestanding object. The
finished village cradle (`lore_ledger_echo`) should read as small and plain next to the
half-built Shaper-scale cradle-structure in the same room (that structure is the `q11`
Architect arena, code-spawned — see §7 — not a `lore_item`, since the player fights inside
it rather than reading it).

---

## 6. `regions` — named zones (all provisional / code-spawned)

```jsonc
[
  { "id": "warden_hall",      "name": "The Warden's hall",         "bounds": { "x0": 24, "z0": -26, "x1": 40, "z1": -14 }, "code_spawned": true, "note": "First hall past the sealed gate. No map file yet — provisional box, see §0." },
  { "id": "chamber_one",      "name": "Shaper Chamber I",          "bounds": { "x0": 22, "z0": -52, "x1": 38, "z1": -40 }, "code_spawned": true, "note": "Bracken's stasis pod." },
  { "id": "chamber_two",      "name": "Shaper Chamber II",         "bounds": { "x0": 22, "z0": -77, "x1": 38, "z1": -65 }, "code_spawned": true, "note": "Sela's stasis pod." },
  { "id": "chamber_three",    "name": "Shaper Chamber III",        "bounds": { "x0": 22, "z0": -102, "x1": 38, "z1": -90 }, "code_spawned": true, "note": "Toma's stasis pod. Smaller pod than the other two — deliberate, do not resize to match." },
  { "id": "the_cradle_room",  "name": "The Hollow's sealed chamber", "bounds": { "x0": 18, "z0": -122, "x1": 44, "z1": -102 }, "code_spawned": true, "note": "Also the q11 Architect arena — the half-finished cradle-structure IS the arena geometry (story-bible §5)." },
  { "id": "the_deep_chamber", "name": "Where the Hollow sleeps",   "bounds": { "x0": 10, "z0": -160, "x1": 52, "z1": -128 }, "code_spawned": true, "note": "act_end only. Player does not fight here — sight-line reveal, not an arena." }
]
```

---

## 7. `quests` — the chain (q6 → q12)

### q6 — The Warden

```jsonc
{
  "id": "q6_the_warden",
  "title": "The Warden",
  "subtitle": "It kept this hall for three hundred years. It doesn't know the shift ended.",
  "giver": "environment",
  "trigger": { "type": "on_spawn" },
  "premise": "The gate sealed behind you the moment you stepped through it. Ahead, down, the first hall of the Reach — and something in it that has been walking the same slow circuit since before Edhari had a name for itself.",
  "objectives": [
    { "id": "o1_descend", "kind": "reach_zone", "target": "warden_hall", "position": { "x": 30, "y": -8, "z": -20 }, "radius": 10, "hint": null, "optional": false },
    { "id": "o2_observe", "kind": "approach_entity", "target": "the_warden", "position": { "x": 30, "y": -8, "z": -24 }, "radius": 12, "hint": "It moves like Garren did. Slower. Heavier. Watch it complete one full circuit before you engage.", "optional": false },
    { "id": "o3_defeat", "kind": "defeat", "target": "the_warden", "hint": null, "optional": false },
    { "id": "o4_if_you_fall", "kind": "survive_death", "target": "warden_hall", "hint": "Rest is close now, even down here. Get up.", "optional": true }
  ],
  "cutscene": {
    "trigger_objective": "o3_defeat",
    "beats": [
      "It comes apart the way ash comes apart in wind — no death-throe, just a slow letting-go of shape.",
      "In the last block of it, something small and formless drifts free and settles into Auren's open hand. It solidifies into a fist-sized shard, pulsing the same teal as the gate sigil.",
      "Auren's other hand moves before they decide to move it — closes around empty air, and a small cube of raw stone crystallises between their fingers, whole, from nothing.",
      "It falls to the floor. Auren looks at their own hand like it belongs to someone else. Nothing is said. The path continues."
    ]
  },
  "mechanics_taught": ["combat_escalation", "shaper_fragment_pickup"],
  "rewards": { "set_flag": "shaper_fragment_1", "activate_campfire": "cp_warden_hall", "unlock_dialogue": "dlg_maren_after_warden", "advance_to": "q7_those_we_left_below" },
  "next": "q7_those_we_left_below"
}
```

### q7 — Those We Left Below

```jsonc
{
  "id": "q7_those_we_left_below",
  "title": "Those We Left Below",
  "subtitle": "A mural too old for Edhari. A light that shouldn't still be lit.",
  "giver": "maren",
  "trigger": { "type": "quest_complete", "quest": "q6_the_warden" },
  "premise": "The hall opens into a chamber older than anything above — a wall carved with a figure that built rather than broke. And past it, a shape in warm light that Maren's voice, thin through the new fire, tells you not to fear.",
  "objectives": [
    { "id": "o1_enter",   "kind": "reach_zone", "target": "chamber_one", "position": { "x": 30, "y": -20, "z": -46 }, "radius": 8, "hint": null, "optional": false },
    { "id": "o2_mural",   "kind": "interact", "target": "lore_chamber_one_mural", "position": { "x": 30, "y": -20, "z": -46 }, "hint": null, "optional": false },
    { "id": "o3_bracken", "kind": "interact", "target": "stasis_bracken", "position": { "x": 27, "y": -20, "z": -44 }, "hint": "The light in the far pod is warm, not hostile. Go to it.", "optional": false }
  ],
  "mechanics_taught": ["environmental_lore", "npc_rescue"],
  "rewards": { "set_flag": "bracken_freed", "unlock_dialogue": "dlg_bracken_first", "advance_to": "q8_the_rite_they_kept" },
  "next": "q8_the_rite_they_kept"
}
```

### q8 — The Rite They Kept

```jsonc
{
  "id": "q8_the_rite_they_kept",
  "title": "The Rite They Kept",
  "subtitle": "She led the offerings. She still doesn't know if they were ever answered.",
  "giver": "environment",
  "trigger": { "type": "quest_complete", "quest": "q7_those_we_left_below" },
  "premise": "Deeper still, the same devotion repeats down a longer wall — and one panel, set apart, shows the Builder making something small enough to hold. A second pod waits ahead, and Bracken, walking with you now, goes quiet when he sees who's inside it.",
  "objectives": [
    { "id": "o1_enter", "kind": "reach_zone", "target": "chamber_two", "position": { "x": 30, "y": -34, "z": -71 }, "radius": 8, "hint": null, "optional": false },
    { "id": "o2_mural", "kind": "interact", "target": "lore_chamber_two_mural", "position": { "x": 30, "y": -34, "z": -71 }, "hint": null, "optional": false },
    { "id": "o3_sela",  "kind": "interact", "target": "stasis_sela", "position": { "x": 33, "y": -34, "z": -69 }, "hint": null, "optional": false }
  ],
  "mechanics_taught": ["environmental_lore", "npc_rescue"],
  "rewards": { "set_flag": "sela_freed", "unlock_dialogue": "dlg_sela_first", "advance_to": "q9_the_warm_thing" },
  "next": "q9_the_warm_thing"
}
```

### q9 — The Warm Thing

```jsonc
{
  "id": "q9_the_warm_thing",
  "title": "The Warm Thing",
  "subtitle": "He only takes the things that are warm. He came back for the last one.",
  "giver": "environment",
  "trigger": { "type": "quest_complete", "quest": "q8_the_rite_they_kept" },
  "premise": "The third mural is cracked clean through, the small held shape in it abandoned mid-form. Ahead, the smallest pod in the Reach. It should not be here. It was supposed to still be hiding in the west house, waiting for you to come back up.",
  "objectives": [
    { "id": "o1_enter", "kind": "reach_zone", "target": "chamber_three", "position": { "x": 30, "y": -48, "z": -96 }, "radius": 8, "hint": null, "optional": false },
    { "id": "o2_mural", "kind": "interact", "target": "lore_chamber_three_mural", "position": { "x": 30, "y": -48, "z": -96 }, "hint": null, "optional": false },
    { "id": "o3_toma",  "kind": "interact", "target": "stasis_toma", "position": { "x": 30, "y": -48, "z": -94 }, "hint": null, "optional": false }
  ],
  "mechanics_taught": ["environmental_lore", "npc_rescue", "emotional_payoff"],
  "rewards": { "set_flag": "toma_freed", "activate_campfire": "cp_chamber_three", "unlock_dialogue": "dlg_toma_reunion", "advance_to": "q10_the_cradle" },
  "next": "q10_the_cradle"
}
```

### q10 — The Cradle

```jsonc
{
  "id": "q10_the_cradle",
  "title": "The Cradle",
  "subtitle": "It has been three hundred years. It is not close to finished. It has not stopped.",
  "giver": "maren",
  "trigger": { "type": "quest_complete", "quest": "q9_the_warm_thing" },
  "premise": "One door left, sealed tighter than any before it. Toma stays at the new fire with Bracken and Sela — this room is not for a child. Inside: a structure that can only be a cradle, Shaper-scale, unfinished, and Maren, through the flame at your back, finally says what she actually knows.",
  "objectives": [
    { "id": "o1_enter",   "kind": "reach_zone", "target": "the_cradle_room", "position": { "x": 30, "y": -60, "z": -110 }, "radius": 10, "hint": null, "optional": false },
    { "id": "o2_relic",   "kind": "interact", "target": "lore_ledger_echo", "position": { "x": 31, "y": -60, "z": -110 }, "hint": null, "optional": false },
    { "id": "o3_listen",  "kind": "listen", "target": "dlg_maren_the_cradle", "hint": null, "optional": false }
  ],
  "mechanics_taught": ["dialogue", "thematic_culmination"],
  "rewards": { "set_flag": "knows_the_cradle", "advance_to": "q11_the_architect" },
  "next": "q11_the_architect"
}
```

### q11 — The Architect

```jsonc
{
  "id": "q11_the_architect",
  "title": "The Architect",
  "subtitle": "It wakes because you are standing in its unfinished work.",
  "giver": "environment",
  "trigger": { "type": "on_objective", "objective": "o3_listen" },
  "premise": "The cradle-structure shudders and rises around you — not a separate monster, but the room itself, gathering into a shape. Ten blocks tall, half-finished, trailing off into geometry that never got made. This is not a fight against a stranger.",
  "objectives": [
    { "id": "o1_defeat", "kind": "defeat", "target": "the_architect", "position": { "x": 30, "y": -60, "z": -114 }, "radius": 16, "hint": null, "optional": false },
    { "id": "o2_if_you_fall", "kind": "survive_death", "target": "the_cradle_room", "hint": "You wake at the fire outside. The room will still be here.", "optional": true }
  ],
  "cutscene": {
    "trigger_objective": "o1_defeat",
    "beats": [
      "It doesn't fall so much as stop insisting on its shape. What's left settles into the unfinished structure it rose from, indistinguishable now from the stone it always was.",
      "Two more shards drift free and settle into Auren's palm alongside the first — the fragment count complete, for now.",
      "The far wall of the cradle-room is not a wall. It never was. It's a door, and beyond it there is no visible ceiling at all."
    ]
  },
  "mechanics_taught": ["boss_combat", "shaper_fragment_2_3"],
  "rewards": { "set_flag": "shaper_fragment_2", "advance_to": "q12_one_slow_breath" },
  "next": "q12_one_slow_breath"
}
```

*(A second `set_flag: "shaper_fragment_3"` fires alongside `shaper_fragment_2` in the same
reward step — both shards land in the same beat per the cutscene above; listing one reward
key is a schema limitation of the illustrative snippet above, not a design intent. Rose/Sun:
either allow `set_flag` to take an array, or split into two same-trigger reward steps —
whichever matches how `q6`'s single-flag pattern was actually implemented.)*

### q12 — One Slow Breath

```jsonc
{
  "id": "q12_one_slow_breath",
  "title": "One Slow Breath",
  "subtitle": "You can finally see how big the thing you're looking for actually is.",
  "giver": "environment",
  "trigger": { "type": "quest_complete", "quest": "q11_the_architect" },
  "premise": "Through the door that was never a wall: a chamber with no visible edges. Something fills most of it. It is not moving. It is, in whatever way something that size can be, asleep.",
  "objectives": [
    { "id": "o1_witness", "kind": "reach_zone", "target": "the_deep_chamber", "position": { "x": 30, "y": -75, "z": -140 }, "radius": 12, "hint": null, "optional": false },
    { "id": "o2_end",     "kind": "listen", "target": "dlg_maren_act2_end", "hint": null, "optional": false }
  ],
  "mechanics_taught": ["culmination"],
  "rewards": { "set_flag": "act2_complete", "activate_campfire": "cp_edge_of_hollow", "unlock_act": "act3" },
  "next": null
}
```

*(`cp_edge_of_hollow` is deliberately named to match `story-bible.md` §5 Act III's world
state: "The final campfire burns at the edge of the Hollow's chamber." This is that fire —
Act III opens from it.)*

---

## 8. `dialogue` — authored lines + choices

```jsonc
[
  {
    "id": "dlg_maren_after_warden",
    "quest": "q6_the_warden",
    "trigger": { "type": "on_objective", "objective": "o3_defeat", "once": true },
    "speaker": "maren",
    "speaker_display": "Elder Maren",
    "where": "Not through stone this time — through the fire beside you, low and close, the way a voice sounds through smoke.",
    "lines": [
      "There. I felt that, all the way up here. The seal thinning was one thing. That was another.",
      "The fire carries me now, not the gate — did you know that? I didn't, not really, until just now. Every flame you light down there, some part of me is standing next to it.",
      "That was Renna. Elder before me, and the one before that's before me. She kept this hall the way I keep the gate. I'm sorry you had to be the one to finish it for her.",
      "Whatever's in your hand right now — don't put it down. I think you're going to need all of it before this ends."
    ],
    "choices": null,
    "completes_objective": null,
    "advances_quest": null,
    "sets_flag": "warden_named"
  },
  {
    "id": "dlg_vision_chamber_one",
    "quest": "q7_those_we_left_below",
    "trigger": { "type": "on_interact", "entity": "lore_chamber_one_mural", "once": true },
    "speaker": "echo",
    "speaker_display": "A voice remembered in the stone",
    "where": "The mural, for one breath, seems to move — not animation, just the memory of motion caught in carved light.",
    "lines": [
      "— and on the third day the mountain had a name, because we gave it one, and the one who raised it laughed the way you laugh at something you're proud of, not something you fear.",
      "We built beside it. Not for it. Beside it. I want that written somewhere that lasts, because I don't think it will always be true."
    ],
    "choices": null
  },
  {
    "id": "dlg_bracken_first",
    "quest": "q7_those_we_left_below",
    "trigger": { "type": "on_interact", "entity": "stasis_bracken", "once": true },
    "speaker": "bracken",
    "speaker_display": "Bracken",
    "where": "The pod's light dims as the stasis breaks. A big, scarred man sags forward, catches himself on the wall, and immediately starts looking at the wall instead of at you.",
    "lines": [
      "...that's good stone. That's — that's really good stone, whoever cut this chamber knew what they were doing.",
      "Sorry. Sorry. Give me a second. Last thing I remember I had a chisel in my hand and a wall half-finished at the Grennet house.",
      "How long. Just tell me the number. I do better with numbers than with... whatever this feeling is.",
      "Three hundred years. Right. Okay. The wall's definitely finished by now, one way or another."
    ],
    "choices": [
      { "id": "c_bracken_ok", "label": "You're safe. Can you walk?", "next_dialogue": null }
    ],
    "sets_flag": "bracken_spoken"
  },
  {
    "id": "dlg_vision_chamber_two",
    "quest": "q8_the_rite_they_kept",
    "trigger": { "type": "on_interact", "entity": "lore_chamber_two_mural", "once": true },
    "speaker": "echo",
    "speaker_display": "A voice remembered in the stone",
    "where": "Further down the same wall — the carving style shifts subtly, generations of different hands continuing the same image.",
    "lines": [
      "— every year the same thanks, every year the same joy, and every year, off to the side where the children wouldn't ask about it, the small work continued.",
      "We never asked what it was for. You don't ask a friend what a gift is for. You just notice, over the years, how carefully it's being made."
    ],
    "choices": null
  },
  {
    "id": "dlg_sela_first",
    "quest": "q8_the_rite_they_kept",
    "trigger": { "type": "on_interact", "entity": "stasis_sela", "once": true },
    "speaker": "sela",
    "speaker_display": "Sela",
    "where": "The pod opens on a woman already speaking before her eyes fully focus — mid-recitation, as if the stasis simply paused her mid-sentence.",
    "lines": [
      "— fed him everything, and he left anyway, and — oh. Oh, I'm not saying that at the well anymore, am I.",
      "That's my hand on that mural back there. That sigil. I drew it a hundred times and never once believed it did anything.",
      "Bracken. You too. Small mercy, waking up next to someone who remembers the same sky I do.",
      "Ask me what the offerings were for and I'll tell you exactly what I told everyone else: I don't know. I never knew. I only knew it mattered to keep asking."
    ],
    "choices": [
      { "id": "c_sela_ok", "label": "You kept a village alive not knowing. That's not nothing.", "next_dialogue": null }
    ],
    "sets_flag": "sela_spoken"
  },
  {
    "id": "dlg_vision_chamber_three",
    "quest": "q9_the_warm_thing",
    "trigger": { "type": "on_interact", "entity": "lore_chamber_three_mural", "once": true },
    "speaker": "echo",
    "speaker_display": "A voice remembered in the stone",
    "where": "The crack running through the relief catches what little light there is and holds it, wrong, like a wound that never closed.",
    "lines": [
      "— and then. And then. I don't have the rest. No one who carved this had the rest.",
      "Whatever happened here, it happened to him, not because of anything we did. I want that understood, whoever finds this. We didn't do this to him."
    ],
    "choices": null
  },
  {
    "id": "dlg_toma_reunion",
    "quest": "q9_the_warm_thing",
    "trigger": { "type": "on_interact", "entity": "stasis_toma", "once": true },
    "speaker": "toma",
    "speaker_display": "Toma",
    "where": "The smallest pod in the Reach opens on the smallest shape in it — Toma, still clutching the wooden toy, awake before the light even finishes fading.",
    "lines": [
      "I said I was good at hiding.",
      "He came back. After you went down. I heard the floor open again and I thought — I thought if I stayed very still like last time —",
      "It didn't hurt. I want you to know that first, before anything else. It didn't hurt, it just got dark and warm and then you were here.",
      "You said you'd come back up for me. You came down instead. I think that's better, actually. I didn't want to wait alone anymore."
    ],
    "choices": [
      { "id": "c_toma_safe", "label": "You're not alone now. Stay by the fire — I'll finish this.", "next_dialogue": null }
    ],
    "completes_objective": null,
    "sets_flag": "toma_reunited"
  },
  {
    "id": "dlg_maren_the_cradle",
    "quest": "q10_the_cradle",
    "trigger": { "type": "reach_zone_secondary", "zone": "the_cradle_room", "once": true, "note_for_sun": "if enter_zone must be first-trigger-only per o1_enter, wire this to fire after o2_relic instead — the important thing is it plays before o1_defeat in q11, not the exact trigger mechanism" },
    "speaker": "maren",
    "speaker_display": "Elder Maren",
    "where": "Through the fire, and for the first time since Auren has known her, genuinely quiet for several seconds before she starts.",
    "lines": [
      "I know that shape. Everyone in Edhari who ever gave an offering knew that shape, and none of us ever said its name out loud, because saying it felt like admitting we understood something we had no right to understand.",
      "A cradle. We gave him a cradle, once, a real one, small enough for a person to have carried it down themselves. It's in the ledger. One line, between a tally of grain and a tally of stone.",
      "I don't know what he lost. I have theories, and every one of them is a guess dressed up as an old woman's certainty. A child. A promise. Himself, before whatever this was happened to him. I don't know.",
      "What I do know: he hasn't stopped. Three hundred years, and he is still, right now, trying to finish building whatever that was. He doesn't know it's been centuries. Grief like that doesn't keep time.",
      "Go carefully. Whatever wakes in there — it isn't waking to fight you. It's waking because someone is standing near its work again, for the first time since it was left alone."
    ],
    "choices": null,
    "completes_objective": "o3_listen",
    "advances_quest": null,
    "sets_flag": "cradle_revealed"
  },
  {
    "id": "dlg_maren_act2_end",
    "quest": "q12_one_slow_breath",
    "trigger": { "type": "on_objective", "objective": "o1_witness", "once": true },
    "speaker": "maren",
    "speaker_display": "Elder Maren",
    "where": "Her voice, very small now, carried an impossible distance through a fire that shouldn't reach this far.",
    "lines": [
      "That's him. All of him, or as much of him as still holds together.",
      "I used to think the stories made him sound bigger than he was, to make the fear make sense. They didn't. If anything they made him small enough to hate. That was easier than this.",
      "Toma and the others are safe by your fire. Edhari is still standing, for now — thinner than it was, but standing. You don't have to finish this today.",
      "But I don't think it's going to wait for a today that isn't today. Rest first. Then decide what kind of ending you're walking toward."
    ],
    "choices": null,
    "completes_objective": "o2_end",
    "sets_flag": "act2_complete",
    "advances_quest": null
  }
]
```

> Same author-copy rule as Act I (`act1-script.md` §9): every line above is final, not
> placeholder. Bracken's voice: deflects emotion into craft-talk, steadies through naming
> things. Sela's voice: ritual cadence, comfortable with not-knowing in a way Maren isn't.
> Toma's voice: still the specific-wrong-detail child register from Act I — carried through,
> not reset.

---

## 9. `act_end` — cliffhanger sequence

```jsonc
{
  "id": "end_act2",
  "title": "End of Act II — The Truth in the Deep",
  "trigger_quest": "q12_one_slow_breath",
  "beats": [
    "The chamber has no far wall that torchlight can find. What fills most of it does not have a clean silhouette — it is less a body than a held breath given mass, half-dissolved at every edge into the same fine ash that has been falling over Edhari since Act 0.",
    "It is not moving. It is not still, either, in the way a stopped clock is still — there is a slow rise and fall to it, vast and unhurried, the scale of tide rather than breath.",
    "Three shards sit warm in Auren's satchel, all the Shaper Fragments the Reach had to give. It is not clear, standing here, whether that is enough.",
    "Behind, far behind, three survivors and a fire that reaches further than any fire should. Toma is asleep against Bracken's shoulder before the walk back even starts.",
    "Nothing wakes. Nothing needs to, yet. The descent is over. What comes next is a choice, and the game does not pretend Auren is ready to make it."
  ],
  "final_line": "Maren, through the fire, quieter than she has ever been: \"Rest first. Then decide what kind of ending you're walking toward.\"",
  "card": "ACT II  ·  THE TRUTH IN THE DEEP  —  end.  Continue in Act III: The Reshaping.",
  "cliffhanger_questions": [
    "What did the Hollow actually lose — a child, a promise, or itself?",
    "Three Shaper Fragments are gathered. Is that a weapon, a key, or both?",
    "Bracken, Sela, and Toma are safe at the edge of the Hollow's chamber — but Edhari above is still thinning. How long does that fire actually hold?",
    "Maren finally ran out of things she was withholding. What happens to a story once its keeper has told all of it?"
  ]
}
```

---

## 10. Engine checklist for Sun (delta from Act I's checklist)

Everything in `act1-script.md` §11 applies unchanged. Additional for Act II:

1. `maps/hollow_reach.json` needs to exist before any `world_position` here can be verified
   — treat every coordinate in this doc as provisional until then (§0).
2. Decide and implement the `cutscene` block proposal (§0) or an equivalent — `q6` and `q11`
   both need a wordless staged beat mid-quest, which the current schema (`opening`/`act_end`
   only) doesn't have a home for.
3. Decide how a single reward step grants more than one flag at once (`q11`'s two-fragment
   drop, §7) — array value on `set_flag`, or two same-trigger reward steps.
4. Three new code-spawned rescue entities (`stasis_bracken`, `stasis_sela`, `stasis_toma`)
   and two new combat entities (`the_warden`, `the_architect`) — combat values live in
   `docs/combat-design.md`, visual specs in `docs/character-design.md` §2.2/§2.3, **not**
   duplicated here.
5. On `q12` completion: play `act_end`, set flag `act2_complete`, hand off to `act3` (not
   yet designed — out of scope for this doc).

**Open engine questions for Sun → Monanisa:** the two schema items in points 2–3 above. Not
blocking — ship with whatever representation matches how Act I's equivalent patterns were
actually implemented, and tell Monanisa so this doc's JSON snippets get corrected to match
reality rather than intent.

---

## 11. Canon cross-reference

All story beats honour `docs/story-bible.md` (LOCKED): world **Vaelthar**, antagonist **The
Hollow** (the last Shaper, grief-inverted, searching — still given no dialogue here,
preserving that mystery for Act III per the restraint the bible's own tone section asks
for), teal `#4FC9D6` = Shaper-light (present only on the Warden/Architect per the escalation
ladder in `character-design.md` §2, never on Bracken/Sela/Toma), campfire = Forge-point
(extended here to explain how Maren's voice travels, §0), theme *creation = destruction*
(the cradle thread is the theme's most literal expression yet — an act of care, repeated by
a village that didn't understand it, converging on an act of care the Hollow couldn't
finish). Chapter boss **The Warden** and its Shaper Fragment drop (`story-bible.md` §5)
lands here as `q6`, resolving the Act I/Act II boundary gap noted in §0 without altering the
LOCKED line describing it. Chapter boss **The Architect**, the "beautiful and terrible"
fight inside "the half-finished structure from the cradle room" (`story-bible.md` §5), lands
as `q11` exactly as specified. The three survivors, "one of them Toma" (`story-bible.md`
§3/§5), are named in full here: Toma (existing canon) plus **Bracken** and **Sela**
(proposed canon, §0/§4 — no LOCKED fact altered, same bar Toma's own addition cleared).
Nothing in this act contradicts `docs/GAME-VISION.md`, `docs/first-playable-loop.md`, or
`docs/character-design.md`.

**What remains deliberately unresolved**, per story-bible §7 OPEN items: what the Hollow
actually lost (§2 above sharpens the question, does not answer it), the canonical ending,
and Maren's/Toma's ultimate fates. This act does not touch any of those calls — they stay
with the CEO.

*Story data + schema (Act I precedent): Rose. This document (Act II): Monanisa. Engine: Sun
(pending). Map props: Shiba (pending — `maps/hollow_reach.json` does not exist yet). Canon
sign-off: CEO.*
