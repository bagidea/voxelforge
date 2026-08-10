# Where the husk → quest loop actually breaks

> **Author:** Poppy · **Date:** 2026-08-11 · **Branch:** `poppy/native-only`
> **Baseline:** `HEAD` = `0096ed6` (line numbers below are `git show HEAD:client/src/quest.rs`)
> **Method:** source trace + the shipped `assets/story/act1.json` + the run log
> `logs-quest-run.txt`. No new binary was built (Rose holds the build lock).

## Summary

The loop is not broken by ranges, physics, input routing or the map. It is broken
by **four dead links in the quest engine**, and their combined effect is worse
than any one of them: **playing the act the way the script asks locks the player
out of finishing it, while killing a husk at the wrong moment skips the story
beat and opens the world anyway.**

| # | Defect | Where (HEAD) | Consequence |
|---|---|---|---|
| **D1** | `enter_zone` dialogue triggers read a journal flag `entered_<zone>` that **nothing in the binary ever writes** | read at `quest.rs:1238`; `grep -rn 'entered_' client/src` returns that one line and nothing else | Maren never speaks at the gate. `q2.o2_listen` (kind `listen`) can only be completed by a choice inside `dlg_maren_gate`, so **q2 cannot complete on the designed path** → q3 never Active → Garren never spawns → the `defeat` objective is never armed |
| **D2** | `on_defeat` dialogue trigger is hardcoded `false`, with a comment claiming `check_kill_triggers` handles it | `quest.rs:1252-1254`; `check_kill_triggers` (`:1079`) takes no `DialogueState` | Killing Garren produces no Maren line, so her `c_on` choice — the scripted hand-off to q4 — never appears |
| **D3** | Quest rewards `open_door` / `activate_campfire` are parsed and **never read** | declared `quest.rs:175,177`; `git grep 'open_door\|activate_campfire' HEAD -- client/src` returns only those two declarations | The sealed village gate has exactly **one** opener in the whole binary: `open_gate_door` is called from a single site, `quest.rs:589`, inside `on_enemy_died` |
| **D4** | `on_objective` dialogue fires as soon as its quest goes Active instead of when the named objective completes | `quest.rs:1241-1250` | `dlg_maren_cliffhanger` carries `completes_objective: o3_end`, so q5's last objective could be scored **before the player walks through the gate**; Maren's "first call" plays over the wake-up |

Two more, found in the same pass and fixed alongside (`quest_rules.rs`, commit
`fb71e4c`) — both are "the loop dead-ends and never recovers":

| # | Defect | Consequence |
|---|---|---|
| **D5** | `check_area_triggers` evaluated `reach_zone` objectives **inside** the once-only `entered` branch | A region is spent the first time the player stands in it. Walk east past the guard post while Maren is still talking and q3's `o1_east` gets no second chance — q4 and q5 sit behind it. Arrival stays a one-off; the objective test is now a predicate on where the player *is* |
| **D6** | `check_kill_triggers` credited a kill only to the objective under `current_objective`, and every completion did `current_objective += 1` | A body despawns the frame after it dies, so a kill arriving while an earlier objective is outstanding was dropped with no second corpse to offer. `first_incomplete` replaces `+= 1` so an out-of-order completion cannot step over what the player still owes |

### The trap D1 + D3 make together

`on_enemy_died` (`quest.rs:550`) fires on **any** enemy death while q2 is Active.
It force-scores `o1_gate` + `o2_listen` by hand, completes q2, and opens the gate.

* Kill the village husk on the walk north → q2 is completed **without Maren**,
  the beat she exists for is skipped, and the gate opens for the wrong kill.
* Do it properly (walk to the gate, hear her out, pick "I'll go east") → q2
  completes and **the gate is never opened by anything** → q5's `o2_enter`
  (`hollow_reach_intro`, z 0-2, on the far side of that gate) is unreachable →
  Act 1 cannot end.

Evidence that the act stops exactly where D1 predicts — `logs-quest-run.txt`,
last quest lines of the run:

```
QUEST_AREA enter=gate_square player=(32.5,7.9)
QUEST_STAGE_COMPLETE qid=q2_voice_in_stone oid=o1_gate => PASS (reach_zone)
  ✗ FAIL: exit=127 missing-QUEST_ACCEPT missing-QUEST_KILL QUEST_PROOF-not-PASS
```

`o1_gate` (reach_zone) fires; `o2_listen` — the objective on the other side of
the dead dialogue trigger — never does, and the run ends there.

## The patch

All in `client/src/quest.rs`.

1. **`zone_flag(zone) -> String`** — one definition of the `entered_<zone>` key.
   `check_area_triggers` now writes it (and logs `QUEST_FLAG set=…`);
   `dialogue_should_fire` reads it through the same function, so the two halves
   cannot drift apart again.
2. **`KillLog.defeated_targets`** — `check_kill_triggers` records the story id of
   every satisfied `defeat` objective (`"garren_husk"`). `on_defeat` dialogue now
   matches against it.
3. **`dialogue_should_fire(dlg, journal, kills)`** — the trigger rules pulled out
   of the system as a pure function, so they can be tested. `on_objective` now
   requires the objective to be **completed** (D4). The quest gate is
   `Active || Completed`, because the event a dialogue hangs off is usually the
   same event that finishes its quest (Garren's death completes q3 in the same
   pass that records the kill).
4. **`QuestJournal.pending_world_rewards` + `apply_world_rewards`** —
   `complete_quest` records `open_door` / `activate_campfire`; a new system with
   the world handles carries them out. q4's `gate_chamber_door` is what unseals
   the gate and lights the sigil now, which is the beat q5's premise assumes.
   Unmapped ids are logged (`guard_post_door` has no geometry in the code-spawned
   arena) rather than silently dropped.
5. **`on_enemy_died` deleted.** Its two jobs are now done properly and in order:
   the Maren line by (2), the world change by (4). No kill can skip q2 any more.

### Tests

Eight new tests at the bottom of `quest.rs`, driving the **shipped** functions
(`zone_flag`, `dialogue_should_fire`, `apply_choice`, `complete_dialogue_objective`,
`complete_quest`) against the shipped `act1.json`:

* `gate_dialogue_fires_once_the_area_trigger_marks_the_zone`
* `zone_flag_is_the_only_spelling_of_the_entered_key`
* `q2_completes_through_apply_choice_and_hands_over_q3`
* `maren_speaks_after_garren_falls`
* `cliffhanger_waits_for_the_player_to_step_through_the_gate`
* `finishing_q4_queues_the_gate_open`
* `finishing_q3_queues_its_door_and_campfire`
* `act1_closes_on_the_scripted_path_with_no_stray_kill`

The pre-existing tests in that module mostly push objective ids onto the journal
by hand and then assert the push happened — which is why all of them "passed"
while the loop was unplayable. The new ones assert on state produced by
production code.

They had in fact never run at all: `act1()` read `assets/story/act1.json`
relative to the cwd, and Cargo runs a test binary from the **package** root
(`client/`), where no `assets/` exists — so every test that touched the story
data panicked on load. With the path fixed to try `../` as well, three of the old
tests turned out to assert things `act1.json` does not say, and were corrected:

* `trigger_quest_complete_unlocks_next` claimed q2's trigger is `quest_complete`;
  it is `enter_zone gate_square` (q3 is the one with `quest_complete`).
* `dlg_maren_gate_has_five_choices_as_hub` pinned `choices[0].label` to wording
  and an order the data does not have; it now pins the four flavour branches by
  id and asserts the last choice is `c_go` carrying `o2_listen` + `q3_gatekeeper`.
* `lore_items_load_correctly` treated the substring "glow" in Toma's toy as a
  canon violation — the subtitle uses it to say "It carries **no glow**". It now
  matches "teal" and additionally asserts the denial is still present.

## What still needs a real run

The tests cover the state machine, not the frame loop. On the next build:

1. `--quest-demo` end to end. Phase 1 (walk up → `E` → `Enter`×4 → `5`) now does
   real work: before this patch the village husk's death force-completed q2 and
   phase 1 was skipped. Watch for `QUEST_DIALOGUE fire id=dlg_maren_gate`.
2. `QUEST_REWARD queue door=gate_chamber_door from=q4_what_walls_remember`
   followed by `QUEST_DOOR gate opened …` — then phase 5 walking to z < 3.
3. `QUEST_DIALOGUE fire id=dlg_maren_after_husk trigger=on_defeat` after
   `QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o3_defeat`.
4. That the auto-fired gate dialogue does not fight the demo's key schedule —
   nothing outside `dialogue_ui.rs` reads `DialogueState`, so movement and combat
   are not gated by an open box, but the demo presses `Enter` either way.

## Not in scope (found, not fixed)

* `q1.rewards.heal` and `unlock_dialogue` are still parsed and unread.
* q4's three objectives are strictly ordered by `current_objective`: reading the
  ledger before bridging the gap does nothing and gives the player no feedback.
* A `listen` objective is only completable through a dialogue choice. Escaping
  out of an auto-fired dialogue leaves the E-key on the NPC as the only way back
  in (`npc_interact`, range 5.0) and nothing tells the player that.
* `survive_death` (q3's optional `o4_if_you_fall`) has no handler at all.
