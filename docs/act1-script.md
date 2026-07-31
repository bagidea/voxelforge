# Voxelforge — Act I Script & Story-Data Schema

> **Engine contract for Sun.** This document defines the schema that `assets/story/act1.json`
> is written in, the runtime model the engine must implement to play it, and a readable
> walkthrough of Act I so the team shares intent. The JSON is the source of truth; this
> doc is how you read it.
>
> Owner: **Rose** (Story data + schema) · Engine: **Sun** · Map props: **Shiba** · Canon: `docs/story-bible.md`
> Last updated: 2026-07-31 · In-game language: **English**

---

## 0. Status & coordination notes

- **This schema is canonical and supersedes the earlier `act1.json` stub** (which used
  `stages[]`, a `dialogues` map, and a `position:[x,y,z]` array in a non-map coordinate
  space). Build the engine to **this** schema. The stub was replaced on Director's brief,
  2026-07-31. If any agent already wrote code against the stub, the mapping is mechanical
  (`stages` → `objectives`, `dialogues` map → `dialogue[]` array, `position:[x,y,z]` →
  `world_position:{x,y,z}`).
- **Do not edit `client/src`.** This lane ships data only. Engine wiring is Sun's; map
  blocks are Shiba's (`maps/`, `scripts/gen_edhari.py`).
- **Coordinate space.** Every position is a **world voxel** in the `edhari` map: `x`,`z`
  horizontal, `y` up, one block = one unit, `north = −z` (the dungeon gate is at low z).
  All coordinates in this act were **verified against `maps/edhari.json`** on 2026-07-31.
- **Code-spawned vs map-placed.** Anything with `"code_spawned": true` is **not** a map
  block — the guard-post arena, Garren the Husk, the Hollow Reach threshold, campfires,
  doors. Sun spawns these from gameplay systems; Shiba does **not** need to bake them into
  the map. `lore_items` and the named regions that are *not* code-spawned are Shiba's to
  dress (see §6).
- **Toma — canon addition (2026-07-31).** A child of Edhari, proposed in
  `docs/character-design.md` §3.2 (Monanisa) and admitted to canon by the story-bible
  owner. **No LOCKED fact altered** — verified: the carved toy carries no teal (respects
  the Shaper-light rule); Toma is a villager (Auren is not); Toma is one of the three
  previously-unnamed Act 2 survivors (`story-bible.md` §5). Ships here as real playable
  content: NPC `toma`, optional q1 objective `o2_find_survivor`, dialogues
  `dlg_toma_first` / `dlg_toma_maren` / `dlg_maren_others`, lore item `lore_tomas_toy`
  (a pick-up that does not glow).
- **`docs/story-bible.md` — Toma canon written (2026-07-31).** Toma is now in the bible:
  §3 Supporting Cast (role, the non-glowing toy, the Act 2 survivor through-line), the §5
  Warden wording unified to "former village elder", §5 Act 2 + §6 drawing cross-referencing
  Toma, and §7 LOCKED (Toma + Warden) / OPEN (Toma's fate) updated. (An earlier edit was
  externally reset once; the Director confirmed Rose owns the bible for this pass, the
  write-up was re-applied, and verified to persist — `git diff` shows +26/-4, Toma appears 13×.)

---

## 1. The five-minute experience (what the player feels)

| Time | Quest | What happens | Mechanic taught (silently) |
|---|---|---|---|
| 0:00–1:00 | **q1 Embers** | Wake in ruins, ash falling, a lone campfire. Walk to it. A faint voice from the gate: *"Walk. It hears running."* | move · campfire = rest/anchor |
| 1:00–3:00 | **q2 A Voice in the Stone** | Reach the sealed gate; Maren speaks through a crack. Three things to ask; she answers two and a half. | dialogue · choices · objective marker |
| 3:00–6:00 | **q3 The Gatekeeper** | East to the guard post. Meet **Garren** — a villager, Unravelled, still guarding. Fight, probably die, wake at the fire, try again. | lock-on · dodge · stamina · light attack · **death = door, not wall** |
| 6:00–9:00 | **q4 What the Walls Remember** | Inside the post the way is broken. Place a block to cross. Read what the village did: they **fed** the thing below, for generations. It never left. | build (place) · environmental lore |
| 9:00–12:00 | **q5 The Sigil That Knew You** | Return to the gate the back way. The teal sigil — dark for 300 years — **blazes for Auren alone**. The gate opens. One breath, enormous, in the dark below. Cut. | culmination / payoff of the Auren-is-a-Shaper seed |

**Hook (first 60 s):** no menu, no text — ruin, ash, warmth, and one distant teal pulse.
A voice that says *walk, it hears running*. Tension between the safe fire and the alien
light up north. (See `opening` in the JSON.)

**Cliffhanger:** the gate opens for Auren and no one else; inside, a carving of a radiant
Shaper building Edhari whose silhouette is Auren's own; something vast stirs below; Maren's
last line — *"tell it Maren is still here."* (See `act_end` in the JSON.)

**Optional discovery — Toma:** exploring west of the campfire (q1 objective
`o2_find_survivor`, skippable) finds a child hiding in the west house behind their own
frightened drawing. Toma is the human stake the act needs — a named survivor you have
actually met, who gives a child's-eye account of the Hollow (*"he only takes the things
that are warm"*) and a plain carved toy that does not glow. The thread runs to Act 2: Toma
is one of the three survivors later found in stasis below.

---

## 2. The mystery of Edhari — layered reveals

The village's secret is **not** told; it is excavated, one layer at a time, through
objects and through what Maren refuses to say yet. Each `lore_item` carries a
`lore_layer` (1 = surface → 5 = the cliffhanger seed). The engine may gate deeper layers
behind quest progress (`requires`), but should never force the player to read them.

| Layer | What the player learns | Where |
|---|---|---|
| 1 | Something beautiful was here; it ended mid-act; people built this with care. | shelter handprint, mason's chisel |
| 2 | It came from below, moved upward; the villagers **knew**. | child's drawing, well inscription (+echo) |
| 3 | They didn't fight it — they **fed** it, for generations. It never left. It is **searching**, not eating. | offering bowl, ledger, gate graffiti, Maren's charcoal sigil |
| 4 | The teal is Shaper-light; the gate sigil is a Shaper door-mark — and it remembers who it was made for. | gate sigil carving |
| 5 | The thing below **built** this world. Its shadow wears Auren's shape. | Builder fresco + `act_end` |

Maren withholds layer 4–5 verbally ("I owe you the truth. I do not owe you all of it at
once."). The world tells the truth before she does. This is the design.

---

## 3. Top-level JSON shape

```jsonc
{
  "schema_version": "1.0.0",   // semver; bump on breaking schema change
  "act": 1,
  "act_title": "The Descent",
  "lang": "en",                // all in-game strings are this language
  "map": "edhari",             // which map file this act plays on
  "start_quest": "q1_embers",  // quest activated on new game

  "opening":      { ... },     // cold-open scene block (§4)
  "npcs":         [ ... ],     // roster referenced by giver / speaker (§5)
  "regions":      [ ... ],     // named zones with bounds (§7)
  "quests":       [ ... ],     // the 5-quest chain (§8)
  "dialogue":     [ ... ],     // authored lines + choices (§9)
  "lore_items":   [ ... ],     // environmental story objects (§6)
  "act_end":      { ... }      // cliffhanger sequence (§10)
}
```

`_comment_*` keys are documentation only — the engine MUST ignore any key starting with
`_comment` (forward-compat for author notes).

---

## 4. `opening` — cold open

```jsonc
"opening": {
  "id": "open_cold",
  "duration_s": 60,            // target length of the hook beat
  "no_menu": true,             // boot straight into the world
  "no_loading_text": true,
  "spawn": { "position": {x,y,z}, "facing": "north" },
  "scene": [ "stage direction", ... ],   // prose beats for the cinematic / lighting pass
  "mechanics_seeded": ["move","third_person_camera"],
  "story_seed": "one-line thematic seed"
}
```

The engine places the avatar at `spawn.position` facing `facing`. `scene[]` is **prose for
the look/lighting pass**, not literal text to render — there is no on-screen string during
the open (canon: trusted, no tutorial text).

---

## 5. `npcs` — speaker roster

```jsonc
"npcs": [
  { "id": "maren", "name": "Elder Maren", "role": "...",
    "voice": "...", "appearance": "...", "knows_and_withholds": "..." }
]
```

- `id` is referenced by `quest.giver` and `dialogue.speaker`.
- `echo` is a non-living speaker (a remembered voice); treat it like any speaker for
  display, but it has no world body.
- `toma` is a child of Edhari hiding in `west_house` — an optional discovery in q1
  (objective `o2_find_survivor`; real dialogue `dlg_toma_first` / `dlg_toma_maren`).
  Visual spec: `docs/character-design.md` §3.2. **Not** Shaper-touched — the plain carved
  wooden toy (`lore_tomas_toy`) deliberately carries no teal. See §0 for canon status.

---

## 6. `lore_items` — environmental story (Shiba places these)

```jsonc
"lore_items": [
  {
    "id": "lore_well_inscription",
    "kind": "inscription",                  // inscription | drawing | object | mural | note | echo
    "name": "Well inscription",             // short label (UI / journal)
    "world_position": { "x": 32, "y": 1, "z": 17 },
    "requires": null,                       // quest id that must be complete before it's readable (or null)
    "lore_layer": 2,                        // 1..5 mystery depth (§2) — for gating/journal ordering
    "subtitle": "physical description — what the player sees",
    "text": "the readable inscription / lore text shown on interact",
    "triggers_dialogue": "dlg_well_echo",   // optional: dialogue to play on read
    "tags": ["hollow","mystery"]
  }
]
```

**For Shiba:** each non-`code_spawned` `lore_item` is a dressable prop at a real map voxel.
`subtitle` is the art direction (what it looks like); `text` is what shows when the player
interacts. Snap `world_position` to the nearest matching real block (coordinates are
verified against `maps/edhari.json`). `kind` suggests the prop archetype (a carved text =
`inscription`; a wall picture = `drawing`/`mural`; a pickup-style object = `object`/`note`).

**Interaction model:**走近 → prompt → show `subtitle` (sight) then `text` (read). If
`triggers_dialogue` is set, play that dialogue node after the text. Reading is **never**
required to finish the act; it only deepens it.

---

## 7. `regions` — named zones

```jsonc
"regions": [
  { "id": "gate_square", "name": "The sealed gate",
    "bounds": { "x0": 27, "z0": 3, "x1": 37, "z1": 8 },
    "code_spawned": false,
    "note": "free-form note for Shiba/Sun" }
]
```

- A region is an axis-aligned box on the ground plane (`x0,z0`–`x1,z1`). The player is "in"
  the region when their `x,z` is inside it.
- `code_spawned: true` means the region is not backed by map geometry — Sun spawns it
  (guard-post arena, Hollow Reach threshold).

---

## 8. `quests` — the chain

```jsonc
"quests": [
  {
    "id": "q3_gatekeeper",
    "title": "The Gatekeeper",
    "subtitle": "one-line teaser",
    "giver": "maren",                       // npc id | "environment" | "auto"
    "trigger": { "type": "quest_complete", "quest": "q2_voice_in_stone" },
    "premise": "short in-engine quest log text",
    "objectives": [ ... ],                  // ordered steps (below)
    "mechanics_taught": ["lock_on","dodge", ...],
    "rewards": { ... },                     // granted on full completion
    "next": "q4_what_walls_remember"        // next quest id, or null at act end
  }
]
```

### `trigger.type` enum
`on_spawn` · `quest_complete` (+`quest`) · `enter_zone` (+`zone`) · `on_objective`
(+`objective`) · `on_defeat` (+`entity`) · `on_choice` (+`from`) · `on_interact`
(+`entity`). When the trigger fires, the quest becomes **active** (its first incomplete
objective is tracked).

### `objectives[]`
```jsonc
{ "id": "o3_defeat", "kind": "defeat", "target": "garren_husk",
  "position": {x,y,z}, "radius": 8, "count": 1,
  "hint": "optional one-line nudge (show sparingly)", "optional": false }
```
- **`kind` enum:** `approach` (horizontal `x,z` distance ≤ `radius` of `position`; **`y` is
  ignored** — the player walks, so a high-mounted target such as the gate sigil at `y=12` is
  reached by standing beneath it at ground level) · `reach_zone` (enter region `target`) ·
  `approach_entity` (horizontal `x,z` distance ≤ `radius` of entity `target`) · `interact`
  (use `target`, usually a `lore_item` id) · `listen` (reach the named dialogue node) ·
  `defeat` (kill entity `target`, `count` times) · `place_block` (place `count` blocks near
  `position`) · `survive_death` (die and respawn at least once — **always optional**).
- Objectives are **ordered**: complete them in array order. `optional: true` objectives can
  be skipped without blocking completion (used only for the death-teaching beat).
- A quest is **complete** when every non-optional objective is done → grant `rewards` →
  activate `next`.

### `rewards`
Any subset of: `heal` ("full"|partial) · `set_flag` (string) · `reveal_path` (compass
direction string) · `unlock_dialogue` (id) · `open_door` (id) · `activate_campfire` (id) ·
`advance_to` (quest id) · `unlock_act` (act id). Treat unknown reward keys as no-ops
(forward-compat).

---

## 9. `dialogue` — authored lines + choices

```jsonc
"dialogue": [
  {
    "id": "dlg_maren_gate",
    "quest": "q2_voice_in_stone",           // owning quest (for journal/vo mixing)
    "trigger": { "type": "enter_zone", "zone": "gate_square", "once": true },
    "speaker": "maren",                     // npc id
    "speaker_display": "Elder Maren",       // nameplate string
    "where": "stage direction (where/voice)",
    "lines": [ "line 1", "line 2", ... ],   // shown one at a time, advance on input
    "choices": [
      { "id": "c_creature", "label": "What took your people?",
        "next_dialogue": "dlg_maren_creature" },
      { "id": "c_go", "label": "I'll go east. Keep the seal.",
        "next_dialogue": null, "completes_objective": "o2_listen",
        "advances_quest": "q3_gatekeeper", "sets_flag": "maren_met" }
    ],
    "completes_objective": null,            // if no choices: objective completed on last line
    "advances_quest": null,
    "sets_flag": null
  }
]
```

### Runtime rules
- A dialogue **fires** when its `trigger` matches (and, if `once: true`, only the first
  time). `on_choice` triggers fire when the player picks the choice whose id matches
  `from` — this is how branch nodes (creature / wayin / who) are reached.
- Play `lines[]` sequentially. If `choices` is present, show them after the last line; a
  choice with `next_dialogue != null` chains to that node, otherwise the dialogue ends.
- A choice (or, when choiceless, the node itself) may `completes_objective`,
  `advances_quest`, and/or `sets_flag`. **This is how the story moves the quest state** —
  e.g. picking *"I'll go east"* completes the listen objective and advances to q3.
- `dlg_maren_gate` is a **hub**: three info branches each return to the same terminal
  choice, so the player can ask 0–3 questions in any order before committing. The engine
  does not need to loop — every branch node carries its own copy of the terminal choice.

> **Every spoken line is final author copy**, not placeholder. Maren's voice: elderly,
> urgent, never panicked, sparse. She withholds out of care (canon). Do not paraphrase.

---

## 10. `act_end` — cliffhanger sequence

```jsonc
"act_end": {
  "id": "end_act1",
  "title": "End of Act I — The Descent",
  "trigger_quest": "q5_sigil_that_knew_you",   // fires when this quest completes
  "beats": [ "cinematic prose beat", ... ],    // staged cutscene directions
  "final_line": "Maren, distant ...: \"...\"",
  "card": "ACT I · THE DESCENT — end. ...",     // the end-card string
  "cliffhanger_questions": [ ... ]              // for marketing/trailer, not in-game
}
```

Play `beats[]` as a staged sequence (lighting + camera + the gate opening + the carving +
the breath + the seal). Show `final_line`, then `card`. `cliffhanger_questions` is
meta-only — do not render it in-game.

---

## 11. Minimal engine checklist for Sun

1. Load `assets/story/act1.json`; ignore `_comment_*` keys.
2. On new game: play `opening`, place avatar at `opening.spawn`, activate `start_quest`.
3. Quest machine: track active quest + per-objective progress; fire `trigger`s; on
   completion grant `rewards` and activate `next`.
4. Region/position predicates for objective kinds (`approach`, `reach_zone`,
   `approach_entity`, `place_block`, `defeat`, `survive_death`, `interact`, `listen`).
5. Dialogue player: triggers, sequential `lines`, `choices` with chaining, and the
   side-effects (`completes_objective`/`advances_quest`/`sets_flag`).
6. Lore interactable: sight (`subtitle`) → read (`text`) → optional `triggers_dialogue`.
7. Spawn the `code_spawned` entities (Garren/husk, guard-post arena, inner campfire,
   doors, Hollow Reach threshold). Combat values live in `docs/combat-design.md` /
   `docs/first-playable-loop.md` — **not** duplicated here.
8. On `q5` completion: play `act_end`, set flag `act1_complete`, hand off to `act2`.

**Open engine questions for Sun → Rose:** none blocking. If you need a field the schema
lacks, add it and bump `schema_version`; tell Rose so the data file matches.

---

## 12. Canon cross-reference

All story beats honour `docs/story-bible.md` (LOCKED): world **Vaelthar**, village
**Edhari**, protagonist **Auren** (working name, voiceless), elder **Maren** behind the
gate, first enemy the **Guard Husk** (here named **Garren**, a former gatekeeper —
consistent with "remnants of villagers partially Unravelled"), antagonist **The Hollow**
(the last Shaper, grief-inverted, searching), teal `#4FC9D6` = Shaper-light, campfire =
Forge-point (rest/respawn), theme *creation = destruction*. **Canon-verbatim status:** the
**well inscription** readable text (`lore_well_inscription.text`) is reproduced verbatim from
the bible (`story-bible.md` §6 / `first-playable-loop.md` §Act 1). The **child's drawing** is
canon only in its *visual* (`lore_child_drawing.subtitle` — "figures fleeing something large",
torched corner); its readable `text` is original author prose expanding that image and is **not
bible-locked** — Sun/CEO may revise it freely without breaking canon. Nothing in this act
contradicts `docs/GAME-VISION.md` or `docs/first-playable-loop.md`.

**Toma** (child of Edhari) is added per `docs/character-design.md` §3.2 — additive canon,
no LOCKED conflict (see §0). **Character-design accuracy check (2026-07-31, vs this
bible):** the visual descriptions of **Auren**, **Elder Maren**, **The Warden**, and
**The Architect** in `docs/character-design.md` are **consistent** with the bible — **no
errors found**. The one pre-existing internal ambiguity (Warden "a former villager" in §5
vs "a former village elder" in §4) is now **resolved** — §5 unified to "former village
elder" to match §4 and character-design §2.2. Toma is now written into `story-bible.md`
itself — §3 Supporting Cast and §7 LOCKED/OPEN, with §5 Act 2 + §6 drawing cross-referencing
it (see §0).

*Story data + schema: Rose. Engine: Sun. Map props: Shiba. Canon sign-off: CEO.*
