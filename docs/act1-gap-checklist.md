# Act 1 — "เล่นจบได้จริง" Gap Checklist

> Owner: **Sun** (Act 1 engine) · Spec: `docs/act1-script.md` + `assets/story/act1.json` ·
> Engine under review: `client/src/quest.rs` (3253 lines) · Branch: `poppy/native-only`.
> Method: every claim below is read from the code or the game's own stdout — not inferred from
> "the file exists". Verified against a real run (`_quest_proof/act1-runtime/*.raw.log`, 2026-08-14).

---

## 0. TL;DR

The Act 1 **engine** is essentially complete — all 8 items of the `act1-script.md` §11 checklist
are implemented. But Act 1 is **not** provably "playable to completion" yet, for one hard reason
and three content reasons:

1. 🔴 **The scripted walk wedges** at the x=45 gap → the q1→q5 runtime proof fails all 3 scenarios.
2. 🟠 **The Act-1 cliffhanger is console text, not a scene** — the gate opening / carving reveal
   / the breath / the seal are `println!`ed, never staged.
3. 🟠 **Toma never spawns** — the optional q1 survivor is in the data but has no world body.
4. 🟡 **Lore props may not be placed** — 11 lore items are readable in code, but their dressable
   map props are Shiba's lane and not yet confirmed on the map.

---

## 1. Engine checklist (act1-script.md §11) — status vs `quest.rs`

| # | Spec item | Status | Evidence |
|---|---|---|---|
| 1 | Load `act1.json`, ignore `_comment_*` | ✅ | `STORY_LOAD ok path=assets/story/act1.json quests=5 npcs=3 dialogues=12 regions=9`; full `StoryData` struct tree (quest.rs:33–262) |
| 2 | New game: play `opening`, place avatar at `spawn`, activate `start_quest` | ✅ | `QUEST_INIT start=q1_embers quests_loaded=5`; `SCENE_READY spawn=(32.5,2.6,32.5)` |
| 3 | Quest machine: track + fire triggers + rewards + `next` | ✅ | `QuestJournal`/`QuestStatus` (quest.rs:273–327), `check_area_triggers`/`check_approach_triggers`/`check_kill_triggers`, `complete_quest`, `advance_to_quest` |
| 4 | Objective predicates (approach / reach_zone / approach_entity / place_block / defeat / survive_death / interact / listen) | ✅ | `ObjectiveDef` + `quest_rules::ObjectiveLike`; `check_block_place_triggers` (1643), `check_kill_triggers` (1136), `zone_flag` (1210) |
| 5 | Dialogue player: triggers, lines, choices, side-effects | ✅ | `fire_dialogue_triggers` (1395), `resolve_dialogue_actions` (938), `apply_choice` (1015) — `QUEST_DIALOGUE fire id=dlg_maren_first_call` in the log |
| 6 | Lore interactable: sight → read → optional dialogue | ✅ | `lore_interact` (1511) — subtitle→text→`triggers_dialogue` |
| 7 | Spawn `code_spawned` entities (Garren, guard-post arena, inner campfire, doors, Hollow Reach threshold) | 🟠 **partial** | `spawn_garren` (805) ✅ on q3 Active. Doors / inner campfire / Hollow Reach threshold code-spawns **not** present in `quest.rs` |
| 8 | On q5: play `act_end`, set `act1_complete`, hand off | 🟠 **partial** | `check_act_end` (1790) fires + prints + sets flag, but there is **no staged sequence** (see §3.2) |

---

## 2. The blocker — runtime proof fails, all 3 scenarios

`BIN=target-quest/debug/voxelforge.exe bash scripts/act1_runtime_proof.sh` → **all three FAIL**:

| scenario | exit | checks failed |
|---|---|---|
| `none` | 1 | 19 |
| `zone_early` | 1 | 22 |
| `kill_early` | 1 | 22 |

**The single failure line** (in every `*.raw.log`):

```
QUEST_WALK_TO_GATE timeout leg=2 at (45.0,20.3) => FAIL
QUEST_FATAL phase=99
```

**Root cause (my lane — `quest.rs` `GATE_ROUTE`):**

`GATE_ROUTE` leg 2 is `(45.4, 22.0) → (45.4, 16.5)` — "north through the 1-wide gap between the
(44,19) wall and the east house". The current `maps/edhari.json` geometry at that pinch is:

| column | z=19 top-y | z=20 top-y |
|---|---|---|
| x=44 | **3** (gravel+leaves bush) | 1 (open) |
| x=45 | **1** (open — the gap) | 1 (open) |
| x=46 | **5** (east-house wall) | 5 (house wall) |

The gap is **one voxel wide** at z=19 (x=45), and the player collision body (≈1 block wide)
cannot centre in it: at x=45.4 its west edge overlaps the x=44 bush, at x=45.0 its west edge
still overlaps voxel 44. The body wedges at `(45.0,20.3)` — the corner of the bush (44,19) and
the house (46,20) — and the 90 s phase-0 timeout expires.

Because the demo never reaches `gate_square`, **q2's gate objective, the Maren dialogue, and
everything downstream (q3→q4→q5→act_end) never fire** in the automated proof. The engine below
the walk is not broken — it is simply never reached.

**Fix is a route change, not a map edit.** The x=32.5 column is surveyed clear (top-y ≤ 2, every
step ≤ 1) from the longhouse's north side straight up the x≈32 ramp onto the gate square — the
same ramp the original leg 4 already climbed. So `GATE_ROUTE` is collapsed to two legs:
`(32.5, 22.0) → (32.5, 6.4)`, deleting the east jog to x=45 entirely. `quest_chaos.rs`
`GATE_ROUTE_ZONE_EARLY` and `GATE_ROUTE_SHARED_LEGS` are updated to match (the `zone_early`
scenario shares the prefix verbatim, per `scripts/act1_loop_audit.py`).

---

## 3. Content beats missing for a *human* player

### 3.1 Toma never spawns 🔴 (optional discovery, but shipped content)

`act1.json` ships `npc "toma"`, objective `o2_find_survivor`, dialogues `dlg_toma_first` /
`dlg_toma_maren`, and lore item `lore_tomas_toy`. `spawn_npcs()` (quest.rs:774) spawns **Maren
only**. Toma has no world body, so the optional q1 survivor beat (the human stake the act is
designed around, per `act1-script.md` §1) is unreachable in-game. The canon/teal test for the
toy exists (quest.rs:2917), but no spawn.

### 3.2 The cliffhanger is console text, not a scene 🟠

`act1-script.md` §10 requires a staged `act_end`: gate opens → radiant-Shaper carving whose
silhouette is Auren's → something vast stirs → the seal → Maren's last line → end card.
`check_act_end()` (quest.rs:1790) only `println!`s the six `beats[]`, `final_line` and `card`,
then prints `ACT1_COMPLETE`. There is no camera move, no lighting, no gate-open animation.
(`VOXELFORGE_ACT1_SHOT` can capture one frame, which is what the screenshot proof below uses.)

### 3.3 Lore props are unconfirmed 🟡

`lore_interact` reads any of the 11 `lore_items` within 3.0 blocks of its `world_position`, but
whether each position has a visible dressable prop on the current map is Shiba's lane and has
not been swept. Reading is never required to finish the act (by design), so this is polish, not
a blocker.

---

## 4. What already works (read from the game's own stdout)

- `q1_embers` auto-completes: `QUEST_FEEDBACK q1_embers complete → text + sound`.
- First dialogue fires on the objective: `QUEST_DIALOGUE fire id=dlg_maren_first_call trigger=on_objective`.
- Area triggers fire: `spawn_shelter` → `campfire_square` → `village_road` → `east_house`.
- Story load, journal init, Maren spawn, flags — all present in the `none` run.

**Conclusion:** the engine is done; the two things standing between "engine exists" and
"Act 1 plays to completion" are the **walk route** (§2) and the **end-of-act scene** (§3.2).
