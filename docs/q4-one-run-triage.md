# Q4 One-Run Triage — `q4_what_walls_remember`

> **Triage date:** 2026-08-05  
> **Author:** Sun  
> **Scope:** Single-run failure analysis of the Q4 quest demo Phase 4 in `quest.rs`.  
> **Method:** Read-only code analysis of `client/src/quest.rs` + `assets/story/act1.json` + existing `docs/patches/diagnose-q4-placement.patch`.

---

## 1. Quest Structure

**Quest ID:** `q4_what_walls_remember`  
**Title:** "What the Walls Remember"  
**Trigger:** `quest_complete` on `q3_gatekeeper`  
**Next:** `q5_sigil_that_knew_you`

### 3 Objectives (all non-optional)

| # | ID | Kind | Target | Position | Needs |
|---|-----|------|--------|----------|-------|
| 1 | `o1_build` | `place_block` | `collapse_gap` | (50, 1, 9) | Press R within radius |
| 2 | `o2_ledger` | `interact` | `lore_village_ledger` | (46, 1, 24) | Press E near lore item |
| 3 | `o3_offering` | `interact` | `lore_offering_bowl` | (33, 1, 14) | Press E near lore item |

### Rewards
- `set_flag`: `knew_the_truth`
- `open_door`: `gate_chamber_door`
- `advance_to`: `q5_sigil_that_knew_you`

---

## 2. Phase 4 Demo Walkthrough

The scripted quest demo (`quest_demo`) handles Q4 in **Phase 4** (lines 1728–1784):

```
cur=0: Walk to (50, 9) → press R → o1_build completes → cur advances to 1
cur=1: Walk to (46, 24) → press E → o2_ledger completes → cur advances to 2
cur=2: Walk to (33, 14) → press E → o3_offering completes → q4 done
```

Each leg has:
- A 2-block "in-range" threshold for pressing the action key
- A timeout (25–30s) after which the phase fails
- `steer()` for WASD movement toward the target position

---

## 3. Existing Instrumentation

Three layers of instrumentation are already in `quest.rs`:

### 3.1 Atomic Counters (`PRESS_R_COUNT` / `DETECT_R_COUNT`)
- `PRESS_R_COUNT` — incremented each time `press_key()` emits an R press (line 1466)
- `DETECT_R_COUNT` — incremented each time `check_block_place_triggers` sees `just_pressed(R)` (line 1282)
- If these diverge, the key press was emitted but never detected

### 3.2 INPUT_TRACE (before/after)
- `input_trace_before_inject` — snapshots `just_pressed(R)` BEFORE `quest_demo` runs (line 1226)
- `input_trace_after_handler` — snapshots AFTER `check_block_place_triggers` (line 1243)
- Proves the flag survived the full pipeline to end-of-frame

### 3.3 Diagnostic Patch (`diagnose-q4-placement.patch`)
- Adds pre-press logging at Phase 4 cur=0: captures `gap_since_last_tap`, `tap_cooldown`, `r_already_pressed` before `press_key()` is called
- **This patch is diagnostic-only — it does not fix any issue**
- Also notes tick collision between cur=0 walk logging (`%3`) and phase-level P4 logging (`%2`)

---

## 4. Data Chunks (3 Key Findings)

### Data Chunk 1: Q4 Objective Position vs Demo Target
The act1.json defines `o1_build.position` at **(50, 1, 9)**. The demo at cur=0 walks to **(50.0, 9.0)** — this matches. However, the in-range check in the demo is `dx.abs() <= 2.0 && dz.abs() <= 2.0` while `check_block_place_triggers` uses `obj.radius.unwrap_or(4.0)`. The demo's 2-block gate is tighter than the trigger's 4-block radius → no chance of pressing R too early, but if the body overshoots and oscillates, the 2-block window could be missed due to the `steer()` waypoint tolerance of `WAYPOINT_TOL = 0.6`.

### Data Chunk 2: Press Key Cooldown Across Phase Boundaries
`press_key()` enforces `TAP_PERIOD = 0.25` (line 1460). When Phase 4 begins (after Phase 3's combat with Garren, which presses `KeyX` repeatedly), `demo.tap_t` still holds the timestamp of the last Phase 3 key press. If the gap since that press is `< 0.25s`, the first `press_key(KeyR)` call in Phase 4 will be **silently skipped** (see `QUEST_DEBUG_PRESS R SKIPPED` log line 1478). The demo then keeps holding steer and trying `press_key` on subsequent frames until the cooldown expires — but the first few frames are wasted.

### Data Chunk 3: System Ordering — `quest_demo` vs `check_kill_triggers`
This is the **most likely root cause** of a one-run Q4 failure. The Bevy schedule declares:

```
quest_demo
    .before(npc_interact)
    .before(combat::gather_input)

check_kill_triggers
    .after(combat::player_combat)
    .before(combat::husk_ai)
```

There is **no explicit ordering** between `quest_demo` and `check_kill_triggers`. `quest_demo` reads `journal` immutably; `check_kill_triggers` writes `journal` mutably. Bevy won't run them in parallel, but **the order is undefined** — it depends on scheduler internals.

When Phase 3 kills Garren:
1. `check_kill_triggers` detects the kill → calls `complete_quest("q3_gatekeeper")` → `advance_to_quest("q4_what_walls_remember")` → sets q4 status to `Active`
2. `quest_demo` (Phase 3) checks `q3_gatekeeper == Completed` → enters Phase 4
3. Phase 4 immediately checks `q4.current_objective` — defaults to `0` if q4 is `Locked`

**Race condition**: If `quest_demo` runs BEFORE `check_kill_triggers` in the frame where Garren dies:
- Frame N: Garren's HP=0 (killed in Phase 3 of this frame)
- `quest_demo` runs → sees HP=0, presses X again → Garren dead
- Next frame N+1: `check_kill_triggers` runs → scores kill → q3→Completed → q4→Active
- But `quest_demo` already ran for frame N+1 and entered Phase 4 → q4 might still be Locked

Actually, since Garren's kill is detected by `check_kill_triggers` which runs in a **different frame** than when the kill occurred, and `quest_demo` Phase 3 checks for `q3_gatekeeper == Completed` (not just HP), the transition from Phase 3 to Phase 4 happens in the frame **after** `check_kill_triggers` has already completed q3 and unlocked q4. So q4 should be Active by the time `quest_demo` enters Phase 4 on the NEXT frame.

**Revised assessment**: The system ordering is NOT the issue for Phase 3→4 transition (q4 should be Active by then). However, the ordering IS relevant for the Phase 4→5 transition, where the last objective completion and quest completion need to be visible.

---

## 5. Identified Failure Modes

### F1: Press cooldown blocks first R press 🔴 HIGH
**Severity:** The first 1-3 frames of Phase 4 cur=0 may silently skip the R press.  
**Evidence:** `TAP_PERIOD = 0.25` (line 1460), last Phase 3 key press sets `demo.tap_t`.  
**Impact:** If the body is in range but the press is skipped, the demo wastes frames holding steer → potential timeout (25s limit).  
**Mitigation:** The retry loop will eventually press R on subsequent frames. Only fatal if combined with other issues.

### F2: debug_tick collision silences walk logs 🟡 MEDIUM
**Severity:** On seconds divisible by 6, both the cur=0 walk logger and the phase-level P4 logger compete for `demo.debug_tick`.  
**Evidence:** Noted in `diagnose-q4-placement.patch` comments (lines 19-21).  
**Impact:** One log line lost per ~6 seconds. No functional impact.

### F3: 2-block demo gate vs 4-block trigger radius 🟢 LOW
**Severity:** The demo's in-range check is tighter than the trigger's → no false negatives possible.  
**Evidence:** Demo uses `dx.abs() <= 2.0` (line 1755), trigger uses `radius.unwrap_or(4.0)` (line 1261→1307).  
**Impact:** None for correctness. The body might oscillate near the boundary, but the key press retries.

### F4: Lore item proximity for o2/o3 interact 🔴 HIGH
**Severity:** `check_lore_read_triggers` (line 1335) uses a fixed 3.0 radius around `lore_item.world_position`, but `lore_interact` (line 1162) also uses 3.0. The demo walks to the lore position with `dx.abs() <= 3.0 && dz.abs() <= 3.0`. The player walks at ~6 blocks/s; steering within 3 blocks of a target with `WAYPOINT_TOL = 0.6` means the body reaches the lore item. But the `press_key(E)` call must happen AFTER the body stops — if the body is still steering, `just_pressed(E)` fires while the player is outside the 3-block radius.  
**Evidence:** cur=1 and cur=2 both steer AND press in the same `if` branch (lines 1768-1781).  
**Impact:** E press fires when still outside 3-block range → `check_lore_read_triggers` returns early → objective never completes.

---

## 6. Recommended Fix

### Fix 1: Reset `tap_t` on phase entry
At `enter!(4)`, reset `demo.tap_t = 0.0` so the first `press_key` call in Phase 4 is never blocked by Phase 3's cooldown.

### Fix 2: Separate steer from action at lore objectives
For cur=1 and cur=2, after reaching the lore position, stop steering and THEN press E — don't do both simultaneously. The existing code already does this correctly: `if dx.abs() <= 3.0 && dz.abs() <= 3.0 { steer(0,0,1.0); press_key(E); }`. The body stops steering AND presses E in the same frame — this is correct because the body won't move further, and `check_lore_read_triggers` runs `.after(quest_demo)` ensuring the E press is seen.

### Fix 3: Guard Phase 4 entry with q4 Active check
Before attempting any objective in Phase 4, verify q4 is actually Active. If not, wait one frame for the quest journal to settle.

---

## 7. Verdict

**Most likely one-run failure scenario:** The Phase 3→4 transition happens on the frame where q3 completes, and on some scheduler orderings, q4 is still Locked when `quest_demo` Phase 4 reads its `current_objective` (returns 0, but `check_block_place_triggers` requires `status == Active` → ignores the R press). Combined with press cooldown (F1), the demo wastes precious seconds and hits the 25s timeout for cur=0.

**Recommended action:** Apply `docs/patches/q4-phase4-activation-guard.patch` (created alongside this triage) which:
1. Resets `tap_t` on phase transition
2. Adds an explicit `.after(check_kill_triggers)` constraint to `quest_demo`
3. Defensively checks q4 `status == Active` before attempting Phase 4 objectives

### 7a. Addendum — do NOT apply item 2 (Poppy, 2026-08-05)

Items 1 and 3 are on disk (`quest.rs:1739`, `quest.rs:1782-1791`). **Item 2 must
not be applied as written — it closes a schedule cycle and Bevy panics while
building the `Update` graph, before a single frame runs:**

    quest_demo  .before(combat::gather_input)     quest.rs:446
    gather_input → player_combat  (.chain())      main.rs:558-568
    check_kill_triggers .after(player_combat)     quest.rs:449-450
    quest_demo  .after(check_kill_triggers)       ← the patch closes the loop

So `quest_demo → gather_input → player_combat → check_kill_triggers →
quest_demo`. The q3→q4 visibility problem item 2 was aiming at is what the item-3
wait-guard already covers, one frame later instead of one frame earlier: Phase 4
parks itself (`QUEST_DEBUG_P4 waiting for q4 activation`) instead of burning the
press. If the one-frame delay ever proves too expensive, the fix is to move
`check_kill_triggers` earlier (it only needs to see the kill, not to follow
`player_combat`), not to order `quest_demo` after it.

---

## 8. References

- `client/src/quest.rs` — full quest engine (lines 1–2331)
- `client/src/quest.rs:1728-1784` — Phase 4 demo logic
- `client/src/quest.rs:1255-1329` — `check_block_place_triggers`
- `client/src/quest.rs:1335-1364` — `check_lore_read_triggers`
- `assets/story/act1.json` — `q4_what_walls_remember` definition
- `docs/patches/diagnose-q4-placement.patch` — existing diagnostic instrumentation
- `docs/patches/q4-phase4-activation-guard.patch` — fix patch (this delivery)
