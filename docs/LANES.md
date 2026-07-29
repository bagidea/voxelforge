# File lanes — who may edit what

One night of parallel work on Voxelforge cost us three collisions on a single
file: `client/src/scene.rs` was edited by three agents at once, one of those
edits left `{:.0f}` in the tree and broke the build for everyone else, and the
fall-through bug survived an extra hour because nobody owned the file end to
end. This page is the fix. It is a coordination contract, not a suggestion.

## The rule

**One owner per file at a time.** Before you edit a file in the table below,
check the Owner column. If it names someone other than you, do not edit it —
report what you need to the Director and it gets routed to the owner, or the
lane gets reassigned first.

If you need a change in another lane to finish your own work, say so and stop.
A blocked task reported in one line is cheap. A silent cross-lane edit costs
the other agent a build.

## Lanes

| Lane | Files | Owner | Notes |
|---|---|---|---|
| Player / scene / spawn | `client/src/scene.rs` | **poppy** | Spawn placement, campsite, the `SPAWN_GROUND` invariant. |
| Combat | `client/src/combat.rs` | **kevin** | Hit → die → respawn loop and its proof harness. |
| App wiring / CLI | `client/src/main.rs` | **kevin** | Modes, schedules, the `--play*` flags. Coordinate with the scene lane before changing what `boot_scene` receives. |
| Look / beauty shots | `client/src/hero.rs`, `client/src/shot_main.rs`, `docs/golden-beauty-shot.md`, `docs/look-acceptance-rubric.md` | **pixel** | `hero.rs` is a beauty-shot scene only — it holds no controller code. |
| Levels / map data | `maps/`, `scripts/gen_edhari.py` | **shiba** | Authored levels and the generators behind them. |
| Web / wasm parity | `scripts/web-verify.mjs`, `scripts/grade_web_parity.py`, `docs/web-parity-checklist.md` | **rose** | |
| Editor camera | `client/src/editor_camera.rs` | **yamamoto** | |
| Proof scripts | `scripts/prove_*.sh` | **shino (Director)** | These are the office's grading rules. Propose a gate; don't loosen one. |

Anything not listed: ask the Director before the first edit.

## Two rules that apply in every lane

**Never weaken a proof script to make your change pass.** `prove_playable.sh`
once printed `PASS 4/4` while `PLAY_WALK` said `FAIL`, because it graded only
"a PNG appeared and the process exited 0". A gate that can't fail is worse than
no gate — it launders a broken build into a green report. Gates get stricter,
never looser, and a missing assertion line is itself a failure.

**`cargo check` being green proves nothing about behaviour.** Two of tonight's
bugs compiled perfectly: a `(x, h, z)` / `(x, z, h)` tuple swap between two
same-typed functions, and a Bevy `B0001` query conflict that only panics at
runtime. Before reporting a run's result, confirm the binary is newer than
every source file you touched — a stale `.exe` will happily reproduce the old
behaviour and make you chase a bug you already fixed.

## Long builds vs. the watchdog

A full Voxelforge build runs about 4:44 and the idle watchdog cuts at 5:00.
Five agents were killed mid-build in one night. Print a progress line at least
every 2 minutes while a long command runs — `echo` inside the build loop is
enough. Don't go silent through a compile.
