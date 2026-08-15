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
| Audio | `client/src/audio.rs` | **kevin** | SFX playback + `SfxEvent`. Uses Bevy's message API (`MessageReader`/`MessageWriter`/`app.add_message`), not the old `EventReader`/`EventWriter`/`add_event` — this repo's Bevy 0.19 has no such types. |
| Quest | `client/src/quest.rs` | **sun** | Quest journal, objectives, dialogue triggers, `DialogueUiEvent`. Same message-API rule as above. |
| Dialogue UI | `client/src/dialogue_ui.rs` | **sun** | egui dialogue box. Assigned to sun 2026-07-31 — file previously had no listed owner. `bevy_egui` 0.41's single-window `ctx` query returns a `Result`, not `&Context`/`&mut Context`, directly; `egui::style::Margin` fields are private and `Margin::symmetric` takes `i8`, not `f32`. |
| Look / beauty shots | `client/src/hero.rs`, `client/src/shot_main.rs`, `docs/golden-beauty-shot.md`, `docs/look-acceptance-rubric.md` | **pixel** | `hero.rs` is a beauty-shot scene only — it holds no controller code. |
| Levels / map data | `maps/`, `scripts/gen_edhari.py` | **shiba** | Authored levels and the generators behind them. |
| Look post stack | `client/src/look.rs`, `docs/look-bible.md`, `docs/look-tier-spec.md` | **rose** | `LookPlugin` — the post stack the *playable* game wears (Bokeh DoF · SSAO · PCSS · TAA · VolumetricFog). Distinct from pixel's beauty-shot lane: `hero.rs` renders the reference, `look.rs` ships it. Retired lane: web/wasm parity (`scripts/web-verify.mjs`, `grade_web_parity.py`) — the client went native-only at `65b1d12`. |
| Look perf probe | `client/src/perf_main.rs`, `docs/look-perf-methodology.md` | **poppy** | The `voxelforge_perf` bin. It `#[path]`-includes `look.rs` unmodified *on purpose* — measuring must never require an edit in rose's lane. |
| Settings / options UI | `client/src/settings_menu.rs` | **monanisa** | `SettingsPlugin` + the `settings_closed` run-condition `main.rs` gates the fly camera on. Assigned 2026-07-31 — file was new and unowned. Same `bevy_egui` 0.41 gotchas as the dialogue-UI row. |
| Editor UI panels | `client/src/editor_ui.rs`, `client/src/editor.rs`, `client/src/editor_config.rs`, `client/src/input_map.rs` | **kevin** | egui panels + state gate, editor mode/state, persisted editor config, key bindings. |
| Editor camera / gizmos | `client/src/editor_camera.rs`, `client/src/gizmo.rs` | **yamamoto** | |
| Animation / VFX / asset import | `client/src/anim.rs`, `client/src/vfx.rs`, `client/src/vfx_bridge.rs`, `client/src/import.rs` | **yamamoto** | Anim clips + the parry timeline, particle/impact VFX and the bridge that fires them, `.vox` import. |
| Dodge & parry | `client/src/dodge_parry.rs` | **kevin** | i-frames + parry→riposte; moves with the combat lane. |
| Voxel core / map loader | `client/src/voxel.rs`, `client/src/mapfile.rs` | **poppy** (on loan from shiba) | Greedy meshing + the JSON map schema the levels are graded against. Lent to poppy 2026-08-05 while shiba's brain is rate-limited; reverts to shiba when he is back. |
| Block palette / block table | `sim/src/block.rs` | **poppy** | The `base_color()` table `voxel.rs::tile_base()` reads. Assigned 2026-08-05 — file was unowned, which was blocking monanisa's palette hexes from landing. Colour *values* are monanisa's call; poppy only lands them. |
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

**Fire-and-forget is now the rule (CEO-approved 2026-07-31).** A cold build on
this box measured 15–17 minutes on 2026-07-31 — three times the watchdog's
5-minute idle cut. Three agents were killed mid-build that morning alone. So:
**never hold a foreground build.** Detach it, write to a log, and poll:

```bash
nohup bash scripts/build_safe.sh build --bin voxelforge \
  --target-dir target-int > _build.log 2>&1 &
echo $! > _build.pid                       # then poll every ~60–90s:
kill -0 "$(cat _build.pid)" && echo "still building $(wc -l < _build.log) lines"
```

Two consequences that are not optional:

- **A killed session does not mean a killed build.** The detached `cargo` keeps
  running after your agent dies. Before restarting anything, check for a live
  `cargo`/`rustc` (`tasklist | grep -i cargo`) — starting a second cold build
  on top of a live one is what produced the 19-process pile-up above.
- **Grade the exit code, never the tail.** `cargo ... | tail` reports *tail's*
  exit 0 even when cargo denied-access on a locked exe. Check
  `${PIPESTATUS[0]}`, or don't pipe at all, and confirm the binary's mtime moved.

## The build lock — only the integration lead runs a full build

The night of 2026-07-31, four to five lanes each kicked off their own cold
`cargo build --bin voxelforge` at the same time (`target-audio`, `target-quest`,
`target-anim`, plus the integration lead's own recovery build) — around 19
`rustc`/`cargo` processes on the box simultaneously. The watchdog killed the
integration lead's and sun's builds mid-run. A full build is already the
single most fragile thing this machine does (see `STATUS_DLL_INIT_FAILED`
below); stacking several of them at once multiplies both the wall-clock time
each one takes and the odds any one of them gets starved past the 5-minute
idle cut.

**The rule:**

- **Full `cargo build --bin voxelforge`** (or `scripts/build_safe.sh build
  --bin voxelforge ...`) is run by the **integration lead only**, during an
  integration pass, into `target-int` (or `target-combat` if `target-int` is
  unusable that day — see the watchdog note above).
- **Every other lane** verifies its own work with `cargo check` (or
  `scripts/build_safe.sh check`) scoped to **its own warm `target-<lane>`
  dir** — it catches the same type errors as a full build, spawns far fewer
  processes, and doesn't contend with anyone else's build lock.
- If you believe you need a full binary build to verify your change (e.g. a
  runtime proof script), say so and let the integration lead run it, or wait
  for the next integration pass rather than starting your own.

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

**Parallel lanes: `scripts/lane-build.ps1`.** `build_safe.sh` (above) is the
thin wrapper — gentle env defaults plus cargo's real exit code — but it does not
isolate a target dir or print a verdict. For a lane that needs its own
**binary** build, the standard entry point is `scripts/lane-build.ps1`. It
builds into a per-lane `target-<lane>/` so six parallel lanes never collide on
one target, waits for other cargo builds to drain first, refuses a lane only if
its `target-<lane>/` is actually in use right now (its `.cargo-lock` is held
open, or a `cargo`/`rustc`/`link.exe` is compiling into it — no hard-coded name
list, so `-Lane rose` is fine whenever rose isn't building; `-Force` overrides),
checks the exe is not locked before relinking, and prints a `VERDICT PASS/FAIL`
judged only from the full build log (`grep '^error'`) plus the real exit code
and the exe's mtime. It never pipes cargo and never runs `cargo clean`.

```powershell
.\scripts\lane-build.ps1 -Lane kevin                  # dev build -> target-kevin/, -j 2
.\scripts\lane-build.ps1 -Lane kevin -Profile release # -> target-kevin/release/
.\scripts\lane-build.ps1 -Lane kevin -Bin voxelforge_vfx_proof
.\scripts\lane-build.ps1 -Lane rose -NoWait           # skip the drain-wait; the busy gate still guards
.\scripts\lane-build.ps1 -Lane rose -Force            # build even if target-rose is in use (override)
.\scripts\lane-build.ps1 -SelfTest                    # prove the verdict catches (a)+(b) + the busy gate
```

**`lane-build.ps1` vs `build_safe.sh`.** Use `lane-build.ps1` when a lane needs
its own binary build in its own warm `target-<lane>/` and a verdict it can
trust. Use `scripts/build_safe.sh` (via the integration lead — see "The build
lock" above) for the full `cargo build --bin voxelforge` into
`target-int`/`target-combat`, and for `cargo check` scoped to a warm target
dir. Both share the same core rules: warm target only, never `cargo clean`,
never pipe cargo.

**Never mask a failure with a pipe.** `cargo build | tail` returns `tail`'s
exit code (0), not cargo's, so a failed build reads as success — the same class
of bug as a gate that can't fail. The wrapper never pipes cargo; if you ever
must, capture `${PIPESTATUS[0]}` and `exit` that, never `$?`.
