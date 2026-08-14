# Act 2 — Beat Outline (continuity from Act 1's ending)

> Owner: **Sun** (engine) · Full design source: `docs/act2-script.md` (Monanisa, 2026-08-14) ·
> This file is the **structure** — the handoff from Act 1 and the q6→q12 shape as a beat list —
> it does **not** re-author Monanisa's content. Where it disagrees with `act2-script.md`,
> `act2-script.md` wins.
>
> Companion: `docs/act1-gap-checklist.md` (what Act 1 still needs to "play to completion").

---

## 0. The one-line shape

Act 2 = one continuous descent through the gate Act 1 just opened, structured as **7 quests**
(q6→q12) across **3 chapters** compressed from `story-bible.md` §5. The player's through-line:
*find the three survivors, learn what the Hollow was, and be present when it starts to wake.*

---

## 1. The continuity bridge (Act 1 → Act 2)

This is the seam the directive asked for — "ต่อจากตอนจบ Act 1". Act 1's `act_end` already
hands off mechanically: `q5_sigil_that_knew_you` completes → `act_end` plays → `"unlock_act":
"act2"` → Act 2's `start_quest` (`q6_the_warden`) is the first thing that fires. No gap, no
cold open.

The cliffhanger Act 1 leaves the player on, and the exact threads Act 2 picks up:

| Act 1 leaves… | Act 2 picks up in… |
|---|---|
| The gate opened for **Auren alone**, then sealed behind them ("the way a door closes on a room someone means to come back to") | `q6` premise: recap + the gate is sealed behind you; everything from here is underground |
| **Toma** still hidden above, promised a return | `q9` — the emotional payoff: Toma's stasis pod, taken after Auren went down |
| **Maren** can no longer speak through gate-stone (it's sealed behind Auren) | her channel changes: she now speaks through **campfires** (Forge-points, already-locked canon) |
| `lore_masons_chisel` ("set down mid-stroke") and `dlg_well_echo` (the voice the `echo` NPC is an impression of) | **Bracken** (the mason, taken mid-stroke) and **Sela** (the offering-keeper) — Act 1's unnamed "three imprisoned survivors, one of them Toma" now named in full |
| `lore_village_ledger` records "a child's cradle" as tribute | `q10` The Cradle — the thematic convergence |

**Net:** nothing in Act 1 is contradicted. The Warden fight (bible §5 marks it Act 1's chapter
boss, `act1.json` never shipped it) lands as `q6` — Act 2's opener — because that's the first
mechanical opportunity. It is still, narratively, the closing beat of the Descent.

---

## 2. The 7 beats (q6 → q12)

Compressed from `act2-script.md` §1/§7. Times are design pacing, not milestones.

| # | Quest | Beat | Mechanic |
|---|---|---|---|
| **q6** | The Warden | Through the gate, into the first hall. The Warden — a former elder, barely a shape — still walks its rounds. Fall if you fall, rise at a new fire. Its death: **Auren's hand shapes a small block from nothing.** No one comments. | combat escalation · first Shaper Fragment · silent seed payoff |
| **q7** | Those We Left Below | Shaper Chamber I — a radiant mural, building with joy. A warm-lit stasis pod: **Bracken**, the mason, three centuries late for finishing a wall. | rescue · environmental lore |
| **q8** | The Rite They Kept | Shaper Chamber II — generations of joy, one small careful thing set apart. **Sela**, who led the offerings, recognises her own carved sigil. | rescue · dialogue |
| **q9** | The Warm Thing | Shaper Chamber III — the mural **cracks**. A third, smaller pod: **Toma**. | rescue · Act 1 payoff |
| **q10** | The Cradle | A sealed chamber holding a half-built, cradle-shaped structure — the shape the village once offered up. Maren (through fire) doesn't know what it lost; it hasn't stopped trying to finish it. | thematic culmination |
| **q11** | The Architect | The Hollow's semi-conscious fragment wakes, manifesting as a towering half-finished Shaper-form. The fight happens **inside its own unfinished work**. | boss combat · Fragments 2/3 |
| **q12** | One Slow Breath | A chamber too large to see the walls of. Something enormous, barely awake, draws one slow breath. Cut. | cliffhanger → Act III |

**Cliffhanger** (into Act III): the Hollow's full form, semi-dormant, having just lost the only
piece of itself capable of finishing anything; three survivors safe at a fire; Edhari still 300 m
of rubble away.

---

## 3. The mystery track (what Act 2 excavates)

Act 1 excavated *Edhari*; Act 2 excavates **what the Hollow was and what it lost**, on its own
1–5 scale (each `lore_item` carries its own `lore_layer`):

1. Built with joy, by someone, for people who loved it back (Chamber I).
2. Built across generations — and working on something small, careful, apart from the rest (Chamber II).
3. The joy stopped mid-act — a fracture from the inside, not an attack (Chamber III).
4. The thing it lost *is* the thing it was building — and the village's offerings fed it for centuries (The Cradle).
5. Not gone, not fully here — enormous, and still grieving (The Architect → `act_end`).

Deliberately unresolved (CEO's call, do not answer here): **what the Hollow actually lost.**

---

## 4. What the engine (me) has to add for Act 2

Cross-ref `act2-script.md` §10 — these are the deltas from Act 1's engine checklist:

1. **`maps/hollow_reach.json` must exist first.** Every `world_position`/`bounds` in
   `act2-script.md` is provisional (negative-y descent, not verified against a real map) — same
   snapping pass Act 1 got.
2. **A mid-quest `cutscene` block** (or equivalent) — q6 and q11 need a *wordless staged beat*
   that the current schema (`opening`/`act_end` only) has no home for. Design intent: wordless.
3. **A reward step that grants >1 flag at once** (q11's two-fragment drop) — array `set_flag`, or
   two same-trigger reward steps.
4. **New entities:** 3 rescue (`stasis_bracken/sela/toma`) + 2 combat (`the_warden`,
   `the_architect`).
5. **`q12` → `act_end` → `act2_complete` → hand off `act3`** (Act III not designed yet).

**Open questions I (Sun) owe Monanisa back** (from §10, not blocking): the representation for
#2 and #3 above — I'll ship whatever matches how Act 1 actually implemented the equivalent
patterns, and tell Monanisa so the JSON snippets get corrected to reality.

---

## 5. Canon guardrails (honour, don't touch)

- Teal `#4FC9D6` = Shaper-light: **only** on Warden/Architect, never on Bracken/Sela/Toma.
- Campfire = Forge-point (extended here to explain Maren's voice — already-locked).
- The Hollow gets **no dialogue** in Act 2 (preserve the mystery for Act III).
- Theme *creation = destruction*, most literally expressed by the cradle thread.
- Three survivors "one of them Toma" (bible §3/§5) named in full: Toma + Bracken + Sela (proposed).
