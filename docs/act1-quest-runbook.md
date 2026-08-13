# Act 1 Quest Runtime Proof — Runbook

The Act-1 quest loop (q1 → q5) is proven by driving a **real game process** with `--quest-demo`.
The harness `scripts/act1_runtime_proof.sh` fires **three** walk-throughs: `none` (the in-order
baseline), `zone_early` and `kill_early` (the two out-of-order routes that used to kill the loop).
This file is the single place to run it and read the verdict.

## (a) The command + the logs it writes

Run from the repo root. No build here — point `BIN` at the binary that carries the fix.

```bash
cd E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge
BIN=target-quest/debug/voxelforge.exe bash scripts/act1_runtime_proof.sh
```

- One scenario only: append the mode — `… act1_runtime_proof.sh kill_early`.
- Logs (per scenario, `$mode ∈ none | zone_early | kill_early`):
  - `_quest_proof/act1-runtime/$mode.raw.log` — **game stdout only**; this is what gets graded.
  - `_quest_proof/act1-runtime/$mode.grade.log` — envelope + PASS/FAIL grade + triage (never graded itself).
- Exit codes: `0` all scenarios PASS · `1` a scenario FAILED · `2` binary not ready.

⚠️ **Gate 0 first.** The harness scans the exe for marker strings that exist only in the patched
source and exits `2` (`NOT_READY`) if any is missing. `NOT_READY` means the build has not landed —
do not grade, rebuild first.

## (b) Markers — PASS vs FAIL

**PASS** — the summary line at the end:
```
ACT1_RUNTIME_PROOF: PASS — every gate above was read out of the game's own stdout.
```
Each scenario must also emit the full `grade_full_chain`; the load-bearing ones:
```
QUEST_ACCEPT id=q3_gatekeeper
QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o3_defeat => PASS (defeat)
QUEST_DOOR gate opened
QUEST_STAGE_COMPLETE qid=q5_sigil_that_knew_you oid=o2_enter => PASS (reach_zone)   ← the brake fix
QUEST_COMPLETE id=q5_sigil_that_knew_you => PASS
QUEST_PROOF Act 1 full chain ALL GATES PASS => PASS
```

**FAIL** — any of these in `*.raw.log`, or process exit ≠ 0:
```
=> FAIL                                          (any self-graded line)
QUEST_FATAL                                      (phase 99 = timed out)
QUEST_STAGE_COMPLETE q5 gate => FAIL (timeout …)  ← brake regressed: o2_enter never scored
QUEST_STAGE_COMPLETE q5 cliffhanger => FAIL (timeout)
panic / B0001                                    (process-level)
```

## (c) Out-of-order routes + hang risk

| route | what it proves | hang risk (from source) |
|---|---|---|
| `none` | in-order baseline; the phase-5 brake lives here | **confident no hang** — every phase timeouts to FAIL → AppExit |
| `zone_early` | stand in `guard_post_east` before q3 → `o1_east` still scores on the return | **confident no hang** — phase-0 detour is a fixed 10-leg route with a 90 s cap |
| `kill_early` | kill Garren while `o1_east` owed → `o3_defeat` credited out of order | **confident no hang** — ambush has its own 60 s cap, then resumes the bounded walk |

All three are bounded *inside* `quest_demo`: every phase has a `phase_t > N` → `phase = 99` →
`QUEST_FATAL` → `AppExit(1)`. The only hang source-reading cannot rule out is **process-level**
(panic / GPU / deadlock outside `quest_demo`). Signal: the process sits > ~3 min with no new log
lines — that is a hang, not a quest bug.

### The `QUEST_ACCEPT timeout — q3_gatekeeper not active` case (quest.rs:2135)

Read the condition at quest.rs:2090 directly:

```rust
let q3_active = journal.quests.get("q3_gatekeeper")
    .map(|p| p.status == QuestStatus::Active).unwrap_or(false);
```

**This read is clean — I found no miss in it.** When the demo's Digit5 lands on choice index 4
(`c_go`, "I'll go east", `assets/story/act1.json:184`), `apply_choice` runs both
`complete_dialogue_objective(q2, o2_listen)` and `advance_to_quest(q3)`. Both paths end with q3
`Active` — the first via q2's next-chain in `complete_quest`, the second directly in
`advance_to_quest` — so 2090 sees it on the following frame.

If line 2135 actually prints, the failure is **upstream of 2090** — `c_go` never landed: the
dialogue never opened, or the E→Enter→Digit5 cadence never advanced to the choice. *Which* step
failed I cannot resolve from source alone; it needs the log. Distinguish in `*.raw.log`:
- `QUEST_INTERACT npc=maren dialogue=open` / `QUEST_DIALOGUE fire id=dlg_maren_gate` — did the dialogue open at all?
- absence of `QUEST_ACCEPT id=q3_gatekeeper` before the timeout line — the choice never applied.

So for this one case: **the 2090 read is not the suspect, but the upstream step that broke is
"must read the log"** — the read itself is not where the miss can be.
