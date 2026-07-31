# AAA Sprint Board

Integration lead: **yamamoto**. Base: local `main` (merged from
`poppy/third-person-controller` at `216fd6c`, 2026-07-31 — local merge only,
not pushed, not tagged, per CEO directive). Every lane branches from this
commit forward so all 8 lanes share one foundation instead of 8 forks of
whatever `main` happened to be on their laptop that day.

This board tracks *status*. File ownership (who may edit what) is the
authoritative table in [`docs/LANES.md`](LANES.md) — this doc doesn't repeat
it, it links to it. If the two ever disagree, `LANES.md` wins.

## The 8 lanes

| Lane | Owner | Files | Status |
|---|---|---|---|
| VFX | **pixel** → handed to **shiba** 2026-07-31 03:00 (pixel hit its session limit) | `client/src/vfx.rs`, `client/src/vfx_bridge.rs` | **Done — proof:** wired into `main.rs` via `vfx` + `vfx_bridge`; `cargo check --bins` EXIT=0 / 0 errors (director-run, 2026-07-31 ~03:5x, `target-vfx`); visual proof rendered from the `voxelforge_shot` bin → `docs/assets/vfx-00-before.png`, `vfx-01-impact.png`, `vfx-02-dissolve.png`, `vfx-03-campfire.png` (slash arc, impact debris, campfire embers all visibly present). Shiba's session was killed by the idle watchdog *after* the build finished and the frames rendered — the artefacts are real, the report never got sent. |
| Animation | **poppy** | `client/src/anim.rs`, `client/src/scene.rs` | Blocked, not poppy's bug — `anim.rs` is wired into `main.rs` (`AnimPlugin`) but fails to compile with 2× "invalid system set" on `.before(combat::husk_ai)` / `.after(combat::husk_ai)`. Root cause is upstream: `combat::husk_ai`'s signature contains a type error from the Audio lane's bug (see below), which poisons anything ordered against it. Expected to clear once Audio is fixed — re-verify, don't just assume. |
| Map | **shiba** | `maps/` | In progress — `maps/edhari.json` (Village of Edhari) already landed on `main` |
| Character | **monanisa** | docs + art (`docs/character-bible.md`, `docs/assets/characters/`) | **Done — proof:** the "paths don't exist yet" cell was stale. `docs/character-design.md` (249 lines) + all 6 concept renders landed in `d2ba784`; director-verified 2026-07-31 12:xx by opening the files (6 PNGs, 1.5–1.9 MB each, dated 07-31 02:18). Monanisa added `docs/character-bible.md` (205 lines) — the quick-reference silhouette/proportion/palette/body-language table with a teal-only-for-Shaper cross-check. Director spot-checked the two extremes by eye: `auren-hero-concept.png` = warm walnut/espresso, zero teal (correct — Auren's Shaper nature is dormant); `the-architect-concept.png` = fully teal-lit pre-Edhari construct (the one sanctioned exception to the 85%-warm rule). |
| Audio | **kevin** | `client/src/audio.rs` | **Blocked — one of two independent sources of the build failure (see note below the table).** `audio.rs` uses `EventReader<SfxEvent>` / `EventWriter<SfxEvent>` / `app.add_event::<SfxEvent>()`, but this repo's Bevy 0.19 has no such types — every other correctly-wired lane uses `bevy::ecs::message::{MessageReader, MessageWriter}` + `app.add_message::<T>()` (see `editor_camera.rs`, `import.rs`, `main.rs`, `scene.rs`, `vfx.rs`). Needs a rename at all `EventReader`/`EventWriter`/`add_event` call sites (17 error lines). Plus 3 more bugs local to `audio.rs`: unresolved `crate::Campsite` (needs `use crate::scene::Campsite;`), 6× "`Vec3` cannot be dereferenced" (`*position` on an already-owned `Vec3`, not a reference), and `Volume` type mismatch (Bevy 0.19's `GlobalVolume`/playback volume field is `Volume`, not `f32` — needs `Volume::Linear(...)`/`Volume::Decibels(...)`). Someone (audio lane, presumably) also wired `crate::audio::SfxEvent` into `client/src/combat.rs` (3 call sites) using the same wrong `EventWriter` name — that's a cross-lane edit into combat.rs that needs the same rename. |
| Story | **rose** | `docs/story-bible.md`, `assets/story/act1.json` | In progress — `story-bible.md` already landed on `main`; `act1.json` not started |
| Quest | **sun** | `client/src/quest.rs`, `client/src/dialogue_ui.rs` | **Blocked — the other independent source of the build failure (see note below the table), plus its own unrelated bugs.** `quest.rs` uses the same wrong `EventReader<DialogueUiEvent>` / `app.add_event::<DialogueUiEvent>()` API as Audio (lines 361, 572), and `dialogue_ui.rs` uses `EventWriter<DialogueUiEvent>` (line 30) — 15 + 5 error lines respectively, needing the same `EventReader`/`EventWriter`/`add_event` → `MessageReader`/`MessageWriter`/`add_message` rename. This did **not** come from the Audio lane's bug — it's the same mistake made independently in a second lane. `dialogue_ui.rs` had no listed owner as of pass #1; assigned to sun 2026-07-31 (was `LANES.md`'s only unowned new file). Unrelated bugs on top: `fire_dialogue_triggers` (line ~828) takes `mut dialogue: ResMut<DialogueState>` but the body writes to an undeclared `d` at lines 863–871 — looks like a copy-paste from `open_npc_dialogue` (which does have a `d: &mut DialogueState` param) with the rename to `dialogue` missed. Also `apply_choice` (line 599) sets `dialogue.walked_to_gate`, a field `DialogueState` doesn't have. Also a `&String` vs `&str` mismatch in `.contains(dlg.completes_objective.as_deref()...)` (line 850). `dialogue_ui.rs` also has 2 more errors of its own: `bevy_egui` 0.41's single-window `ctx` query now returns `Result<&mut Context, QuerySingleError>` (code treats it as `&Context` directly), and `egui::style::Margin` is private with `Margin::symmetric` now taking `i8` not `f32`. **Note (2026-07-31, post-report):** `quest.rs` on disk now shows the `d`/`dialogue` rename and a `walked_to_gate` field already applied — looks like sun landed a fix concurrently after the pass #1 log was captured. Not re-verified by a build; treat as unconfirmed until the next integration pass. |
| Research | **sahara** | `docs/research/` | In progress — `docs/research/combat-and-worldfeel-brief.md` already landed on `main` |

> ## ⚠️ READ THIS BEFORE THE TABLE — pass #2 (2026-07-31, director-verified)
>
> **The "40 errors / every lane Blocked" picture below is OUT OF DATE.** I ran
> the check myself, not from a lane report:
>
> ```
> cd client && CARGO_TARGET_DIR=../target-vfx cargo check --bins -j 2
> → EXIT=0 · 0 errors · 18 warnings
> ```
>
> Both binaries (`voxelforge`, `voxelforge_shot`) compile. All 8 lanes are wired
> into `main.rs`: anim · audio · quest · dialogue_ui · vfx · vfx_bridge ·
> editor_camera · gizmo. The `EventReader`/`EventWriter`/`add_event` →
> `MessageReader`/`MessageWriter`/`add_message` renames described below are
> **already applied** in Audio, Quest and `dialogue_ui`, and the Animation
> "invalid system set" cascade is gone with them. The per-lane Blocked cells for
> Audio / Animation / Quest are kept only as the historical record of pass #1.
>
> **Playability gate — director-run 2026-07-31 ~06:2x, all three against the
> SAME fresh binary `target-combat/debug/voxelforge.exe` (built 06:10, newer
> than every `client/src/*.rs`), one at a time:**
>
> | Proof | Result |
> |---|---|
> | `scripts/prove_combat.sh` | **PASS** — STRAFE / HIT (80→60) / HEAVY (60→15, −35 stamina) / KILL / DEATH / RESPAWN at campfire all `=> PASS`, exit 0, no panic |
> | `scripts/prove_playable.sh` (`BIN=` override, no rebuild) | **PASS** — 4/4 shots, `MAP_LOAD ok maps/edhari.json` 6483 blocks, `SPAWN_GROUND => PASS`, `PLAY_LOOK` 60.6°, `PLAY_WALK` 10.89 units grounded, before ≠ after |
> | quest (`--quest-demo` run directly on the same binary) | **FAIL** — gets through `QUEST_STAGE_COMPLETE q1_embers` → `QUEST_COMPLETE` → `QUEST_NEXT_OPEN q2_voice_in_stone` → `QUEST_WALK_TO_GATE reached gate_square`, then `QUEST_ACCEPT timeout — q3_gatekeeper not active => FAIL` + `QUEST_FATAL phase=99`. Note the demo process still exits 0 — **do not grade this by exit code**, grade by `=> FAIL`. Log: `_quest_proof/quest-demo-shino.log`. Owner: sun. |
>
> Run them **one at a time**: this machine ran out of process headroom when a
> workspace check overlapped 4 `rustc` jobs and died with `0xc0000409`
> (headroom, not a code bug).

**Correction (2026-07-31, after the CEO cross-checked `build-recovery.log`
line counts by hand):** pass #1 originally framed Audio's `EventReader`/
`EventWriter`/`add_event` mistake as *the* root cause cascading everywhere.
That undersold it — the same wrong Bevy-0.19 API was written independently
in **two** lanes: Audio (`audio.rs`, `SfxEvent`, 17 error lines, also wired
into `combat.rs`) and Quest (`quest.rs` + `dialogue_ui.rs`, `DialogueUiEvent`,
15 + 5 error lines). Neither caused the other; they're the same class of
mistake made twice. Only the Animation/`main.rs` cascade (2 + 1 error lines,
"invalid system set" on `combat::husk_ai`) traces specifically to Audio's
`SfxEvent` wiring in `combat.rs` — that part of the original framing was
correct.

Status is updated by the lane owner (or by yamamoto during an integration
pass, if the owner reports it). Values: `Not started` / `In progress` /
`Blocked — <reason>` / `Done — proof: <how it was verified>`.

## How a lane lands work

1. Work happens on the file(s) your row owns. Cross-lane edits are only for
   resolving a merge conflict — and if you hit one, report it (see below),
   don't just silently carry it.
2. Commit directly to local `main` (this is a single shared working copy, not
   a PR workflow — see `docs/LANES.md` for why the one-owner-per-file rule
   exists and what it cost the team the night it didn't).
3. Every commit message should name the lane, e.g.
   `vfx: add hit-spark particle burst on melee connect`.

## Integration cadence — every ~2 hours

yamamoto runs this pass roughly every 2 hours (not on a strict clock —
"roughly" because a build can run long and a check mid-build is worse than a
check ten minutes late):

1. `git log --oneline` on `main` to see what landed since the last pass.
2. `bash scripts/prove_combat.sh` — the combat proof gate. **Never weakened.**
   Any `=> FAIL` or missing `COMBAT_*` line is a real failure, not a report to
   soften.
3. `bash scripts/build_safe.sh build --bin voxelforge --target-dir target-int`
   — a general compile check across every lane's code, isolated in its own
   `target-int/` `CARGO_TARGET_DIR` (gitignored, same pattern as
   `target-combat/` / `target-review/`) so it never fights another lane's
   build lock.
4. **Watchdog:** both commands can run several minutes. Print a progress line
   at least every 2 minutes while either runs — an idle terminal past ~5
   minutes gets killed (`docs/LANES.md` "Long builds vs. the watchdog").
5. **Before reporting a result:** confirm the built `.exe` is newer than every
   source file touched since the last pass (`cargo check` passing proves
   nothing about behavior — a stale binary will happily reproduce the bug you
   already fixed). `git log --name-only` since the last pass + `stat`/`ls -la`
   on the binary is enough.
6. If either gate fails, identify which lane's commit(s) since the last pass
   are implicated (`git log --oneline --stat` over the new range, cross-referenced
   against the file-ownership table) and report: which lane, which commit,
   what broke, the raw failing output — not a paraphrase.
7. Cross-lane merge conflicts: resolve them (that's the integration lead's
   job), but report exactly what was resolved and how, so the lane owners can
   sanity-check the resolution against their intent.

## Rules (restated from `docs/LANES.md` — read that doc for the full "why")

- **Never weaken a proof script to make a change pass.** A gate that can't
  fail is worse than no gate.
- **`cargo check` green ≠ correct behavior.** Verify the binary is actually
  newer than the source you touched before trusting a run.
- **One owner per file.** Cross-lane edits are for conflict resolution only,
  and always get reported.
- **Echo progress ≥ every 2 minutes** during any long-running build/test so
  the watchdog doesn't kill it mid-run.

## Integration pass log

| # | When | Commits since last pass | `prove_combat.sh` | `cargo build --target-dir target-int` | Notes |
|---|---|---|---|---|---|
| 0 | 2026-07-31 (baseline, immediately after the merge) | n/a — this *is* the merge | PASS | n/a | Baseline pass, before any lane's loose files landed on disk. |
| 1 | 2026-07-31 | 0 new commits — lanes dropped loose (uncommitted) files: `vfx.rs`, `anim.rs`, `audio.rs`, `quest.rs`, `dialogue_ui.rs` + `main.rs`/`combat.rs` wiring | **BLOCKED — build fails before it can run** | **FAIL — 40 errors** (`cargo build --bin voxelforge --target-dir target-combat`, exit 101; ran on `target-combat` not `target-int`, see `docs/LANES.md` watchdog note) | See lane-by-lane breakdown below. Root cause is the same wrong Bevy-0.19 API (`EventReader`/`EventWriter`/`add_event` vs this repo's `MessageReader`/`MessageWriter`/`add_message`) written independently in **two** lanes — Audio (`audio.rs`) and Quest (`quest.rs`+`dialogue_ui.rs`) — see the correction note under the lane table. Audio's instance additionally cascades into Animation and `main.rs` via `combat.rs`. Full raw log: `build-recovery.log` (gitignored). |
| 2 | 2026-07-31 ~06:20 | 6 lanes landed fixes (Audio + Quest `Event*`→`Message*` rename, Audio `Vec3`/`Volume`/`Campsite`, Quest `walked_to_gate`/`DialogueState`, `bevy_egui` 0.41 API) | **PASS** — all 6 gates: STRAFE · HIT (80→60) · HEAVY (60→15, −35 stamina) · KILL · DEATH · RESPAWN (dist 0.00, full HP at campfire) | **PASS** — `cargo build --target-dir target-combat` EXIT=0, binary `06:10` newer than all `client/src/*.rs` | Combat lane closed — proven on fresh `target-combat/debug/voxelforge.exe`. Quest demo still FAIL (q3_gatekeeper timeout, owner: sun). Binary shared across all 3 proof scripts (combat/playable/quest). |
