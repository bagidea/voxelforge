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
| Editor UI panels | `client/src/editor_ui.rs` | **kevin** | egui panels + state gate. |
| Editor camera | `client/src/editor_camera.rs` | **yamamoto** | |
| Proof scripts | `scripts/prove_*.sh` | **shino (Director)** | These are the office's grading rules. Propose a gate; don't loosen one. |
| Independent cross-checks | `scripts/_shino_*.py` | **shino (Director)** | Re-implementations written from the Rust source, *not* from another checker — they exist to be able to disagree with it. Don't delete one because another checker "already covers it". |
| Map verification | `scripts/verify_edhari_village.py` | **shiba** | Grades `maps/edhari.json` against the loader schema, so it moves with the map lane above. |

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

## Build safety — the `STATUS_DLL_INIT_FAILED` trap

A plain `cargo build` on this machine fails — not with a code error but with
the toolchain dying mid-spawn:

    error: process didn't exit successfully: `...rustc.exe ...`
      (exit code: 0xc0000142, STATUS_DLL_INIT_FAILED)
    error: linking with `link.exe` failed: exit code: 0xc0000142
    warning: build failed, waiting for other jobs to finish...
    BUILD FAILED exit=101

**What this is.** `0xc0000142` (`STATUS_DLL_INIT_FAILED`) fires while the OS
is **creating a process** — at spawn, before the new rustc/link.exe compiles a
line. It is not a code error, and the crate named in the message (`syn`,
`zerocopy-derive`, `ntapi`, …) is just the *victim of the moment*: it lands on
a **different crate every build**. Read that as "a spawn failed at random", not
"that crate is broken".

**What it is NOT.** Every theory below was suspected and then **disproven on
this box** — don't re-chase them:

| Disproven theory | Why it's ruled out |
|---|---|
| Out of RAM / commit charge | At the moment of failure: **9.2 GB RAM free**, commit **11.4 / 20.2 GB** — plenty of headroom. |
| Too many parallel rustc jobs | It crashes at `-j2` **and** at `CARGO_BUILD_JOBS=1` (one job). Parallelism is not the trigger. |
| `link.exe` itself is broken | `link.exe` invoked alone runs fine and prints its banner (`14.50.35729`). |
| Desktop-heap exhaustion | A stress test spawning **150 processes + 60 `link.exe`** failed **0** — not at this scale. |

**What we still don't know.** The OS-level reason a single process spawn dies
in DLL init is an **open case**, not closed. So treat any "this is the fix" as
provisional. The three env knobs the wrapper sets (`JOBS=2`, `DEBUG=0`,
`INCR=0`) are gentle defaults — less concurrency, lighter linking — and they do
**not** stop this failure (proven: `JOBS=1` still dies). Don't call them a cure.

**The lever that actually helps: spawn count.** Every process spawn is one
more roll of this die, and the rolls compound across a build. So minimise how
many crates cargo has to spawn rustc for:

- **Always build into a target dir that is already warm.** An incremental build
  re-spawns rustc only for the crates that changed — far fewer rolls.
- **Never `cargo clean` / delete `target` to "build clean".** That forces cargo
  to re-spawn rustc for *every* crate from scratch — maximum rolls, maximum
  chance of tripping the failure. A clean build is the most fragile thing you
  can do on this machine.

    bash scripts/build_safe.sh build --bin voxelforge
    bash scripts/build_safe.sh check

`scripts/build_safe.sh` sets the gentle defaults above, forwards every argument
to cargo, and returns cargo's **real** exit code (it never pipes cargo).

**Never mask a failure with a pipe.** `cargo build | tail` returns `tail`'s
exit code (0), not cargo's, so a failed build reads as success — the same class
of bug as a gate that can't fail. The wrapper never pipes cargo; if you ever
must, capture `${PIPESTATUS[0]}` and `exit` that, never `$?`.
