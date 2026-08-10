# Act-1 runtime proof — armed, not yet fired

**Status: NOT COMPILED.** Nothing here has been through `cargo check`/`cargo build`;
the build lock is held. Everything below that says PASS was run headlessly
(`rustc --test` on a single Bevy-free file, Python, bash) and the command +
output is quoted. The one thing this is *for* — a real game process walking
q1 → q5 — has **not** been run. See "What is still unproven".

## Fire it

```bash
BIN=target-quest/debug/voxelforge.exe bash scripts/act1_runtime_proof.sh
```

Three runs of the real binary, ~2–4 min:

| scenario     | what the player does                                         |
|--------------|--------------------------------------------------------------|
| `none`       | the ordinary in-order route — the regression baseline         |
| `zone_early` | stands in `guard_post_east` **before** Maren hands out q3     |
| `kill_early` | puts Garren down while `o1_east` is still owed                |

Exit `0` all three pass · `1` a scenario failed · `2` the binary predates the fix.

## The rule the harness is built around

Every graded line is `println!`-ed by `client/src/quest.rs`. The driver grades
`$OUTDIR/<mode>.raw.log`, which is created by the `>` redirect on the game
process and is **never appended to** — the envelope and the grade go to a
separate `.grade.log` that is never read back. A harness that prints its own
`=> PASS` proves the harness runs, not the game.

The two scenario claims are ordering assertions on engine lines, so no single
line can fake them:

* **`zone_early`** — `QUEST_AREA enter=guard_post_east` appears **before**
  `QUEST_ACCEPT id=q3_gatekeeper`, appears **exactly once**, and
  `QUEST_STAGE_COMPLETE qid=q3_gatekeeper oid=o1_east => PASS (reach_zone)`
  appears **after** the accept. The region announced arrival once, on a visit
  that could not credit anything — and the objective still scored later. That is
  the presence-not-entry fix, stated in the engine's own words.
* **`kill_early`** — `oid=o3_defeat => PASS (defeat)` appears **before**
  `oid=o1_east => PASS (reach_zone)` **and before** `QUEST_AREA
  enter=guard_post_east`, and the engine's `QUEST_CHAOS ambush done at (x,z)`
  reports `x < 48` (`guard_post_east.x0`). The kill was credited out of order,
  from outside the region.

## How the scenarios are driven

`VOXELFORGE_QUEST_CHAOS=zone_early|kill_early` (`client/src/quest_chaos.rs`).
`quest_demo` prints `QUEST_CHAOS mode=<label>` on its first frame; the driver
greps it, so an env that was not honoured cannot be graded as a scenario.

* **zone_early** swaps `GATE_ROUTE` for `GATE_ROUTE_ZONE_EARLY`: the same five
  legs to the gate square, then a there-and-back along the proven z=8.5 lane
  into `guard_post_east` and up the x≈32 ramp to Maren. q3 is still `Locked` for
  the whole detour, so nothing can be credited on that visit.
* **kill_early** stops phase 2 at `AMBUSH_X = 46.0` — west of the region line —
  and holds there. Garren's patrol turns at x=47.5 and the leash is 6 blocks, so
  he walks into melee himself; the demo never steps east to meet him, which is
  what keeps the kill outside the region.

## Rule 2 — ordering before range/physics

Any failing scenario prints an `INPUT ORDERING TRIAGE` block first: R presses
emitted by `quest_demo` vs R presses seen by `check_block_place_triggers`
(the engine's own `QUEST_DEBUG_PRESS` / `QUEST_DEBUG_BLOCK` counters), plus the
`INPUT_TRACE` before/after snapshots. It answers "was the flag eaten by the
schedule?" **before** anyone re-tunes a radius. Three verdicts: never pressed
(a walk failure), eaten (fix the schedule), survived (the objective test refused
it — go look at distance/quest state).

## What has actually been run

```
$ ./_probe/quest_chaos_test.exe
test result: ok. 13 passed; 0 failed
$ ./_probe/quest_rules_test.exe
test result: ok. 13 passed; 0 failed
$ python scripts/act1_loop_audit.py
AUDIT ---- 0 FAIL, 5 WARN                       (6 new `chaos routes` checks, all ok)
$ bash scripts/_poppy_act1_proof_selftest.sh
=== SELFTEST: 11 passed, 0 failed ===           (3 positive, 8 negative controls)
```

`_poppy_act1_proof_selftest.sh` is the driver's own negative control: a fake
binary emits canned engine stdout and the driver must **fail** on the
pre-`fb71e4c` semantics (kill dropped / region spent), on a scenario that
silently ran the ordinary route, on an env that was not honoured, on an ambush
that happened inside the region, on a region that announced arrival twice, and
must **refuse** (exit 2) a binary with the marker strings stripped out. Run it
before trusting a green runtime run.

The audit's `chaos routes` checks were themselves put through negative controls
— `AMBUSH_X` moved inside the region, a drifted route leg, `POST_LANE_Z`
hardcoded again — each produced `1 FAIL` and the tree restored to `0 FAIL`.

## What is still unproven

* **The game has not been run.** No `QUEST_PROOF Act 1 full chain` line exists
  from this tree. The harness is armed; it has not fired.
* **Nothing here has been compiled.** `client/src/quest_chaos.rs` type-checks on
  its own (`rustc --test`), and `quest.rs`/`main.rs` parse clean
  (`rustfmt --emit stdout`, exit 0) — that is a syntax gate, not a type gate.
  The call sites into `quest_chaos` from `quest.rs` are unverified by a compiler.
* **The routes are geometry arguments, not walked ground.** The detour uses the
  lane `quest_demo` phase 2 already walks and the ramp phase 0 already climbs,
  and `act1_loop_audit.py` holds the numbers to their sources — but only a run
  proves the body fits through.
* Memory has `quest_demo` wedging at phase 2 on a map column at (43,7). The
  source says Shiba deleted it and the lane moved to z=8.5. Unconfirmed on a
  binary.
