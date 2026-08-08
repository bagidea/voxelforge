# `grade-vista` at Ultra renders as if PCSS were off — because it was never at Ultra

2026-08-09, Poppy. Measurement + source only: no `cargo`, no engine run — the build lane is
not mine and an LTO link was live on `target-yamamoto` throughout (`cargo build --release
--target-dir target-yamamoto`, verified by cmdline, not by a bare `tasklist`).

---

## The answer in one line

`main.rs` adds `SettingsPlugin` **after** `LookPlugin`, and `SettingsPlugin::build` calls
`app.insert_resource(settings.graphics)` — which *"overwrites any existing resource of the
same type"* (`bevy_app::App::insert_resource`, 0.19.0 `app.rs:440`). `settings.json` says
`"graphics": "High"`. So `VOXELFORGE_LOOK_QUALITY=ultra` is parsed, inserted, and thrown
away one line later, and **every `--play` plate this project has ever shot rendered at
High** — whatever the shoot script exported. At High, `let pcss = matches!(*quality,
LookQuality::Ultra)` is `false`, `soft_shadow_size = None`, and PCSS is off. Not weak: off.

`VOXELFORGE_LOOK_PCSS=16` moves 45.4 % of the frame from the same binary because
`pcss_width()` parses the env value **before** it consults the tier:

```rust
Ok(v) => v.trim().parse().ok().or(tier_on.then_some(PCSS_WIDTH)),   // look.rs:1396
```

so the override switches PCSS on regardless of tier. "The override at the same nominal
width moves the frame but Ultra does not" is not two facts. It is one: the override is the
only thing that has ever reached the light.

## Your shadow-map hypothesis: falsified, and it is good news

`look.rs:1058-1063`'s comment describes the 2048 fallback as a live hazard, which is why it
reads that way — but the line under it **is the fix**, and it is not new:

* `look.rs:1066` `.insert_resource(DirectionalLightShadowMap { size: 4096 })` landed in
  **8176d01, 2026-07-31 21:20**, unconditionally in `LookPlugin::build`.
* Every plate binary post-dates it: `_pixel_shotset_N6`'s exe stamps commit `d6418aa`
  (08-08) and the probe exe `B024BBC2…` stamps `ab551c2`/`3511582` (08-08). Both contain
  8176d01.
* The other three entry points insert 4096 too — `main.rs:415` (hero branch),
  `shot_main.rs:142`, `perf_main.rs:171`. Nothing in this repo inserts 2048; 2048 is only
  Bevy's default, and `insert_resource` beats it.

So the plates were shot on a 4K map. The map was never the problem, and PCSS 3.0/16.0's
sign-off condition is intact.

## The proof that it is the tier, not the map

**Static.** `main.rs:484` `LookPlugin` → `main.rs:485` `SettingsPlugin`; `settings_menu.rs:135`
`insert_resource(settings.graphics)`; `settings.json` `"graphics": "High"` (tracked, 780f78a).
The comment above that line already states the intent — *"AudioPlugin and LookPlugin may
have inserted their own defaults first; these calls override them"* — it just never
considered the capture override a thing worth exempting.

**Measured — the tier env is a no-op on the game binary.** `_poppy_tier_probe/`, one exe,
one camera, four runs, only `VOXELFORGE_LOOK_QUALITY` moving. Mean |ΔL| between the plates:

| pair | mean \|ΔL\| | %px >1 L | %px >4 L |
|---|---|---|---|
| low vs medium | 0.320 | 4.5 % | 0.70 % |
| low vs high | 0.345 | 4.5 % | 0.95 % |
| low vs **ultra** | 0.379 | 4.5 % | 1.01 % |
| medium vs ultra | 0.293 | 3.2 % | 0.85 % |
| high vs **ultra** | 0.390 | 5.4 % | 1.16 % |
| any tier vs `VOXELFORGE_LOOK_DISABLE=1` | **61.5** | **99.1 %** | 98.6 % |

Low carries no TAA, no volumetric fog and a Gaussian shadow filter; Ultra carries TAA, a
96-step ray-march, Ultra SSAO and PCSS (`insert_stack`, look.rs:1446-1518). Those two
cannot render 0.38 L apart — that is the repeat-shot floor (0.16–0.26 measured on three
identical shots of `grade-vista`). The look stack IS running: disabling it moves the frame
by 61.5. The **tier** is what never arrives. (`settings.json` was on disk with `"High"` at
the time — committed 780f78a, 08-01 01:12; the probe ran 08-01 12:16, and its logs carry
`SETTINGS loaded settings.json`.)

*What this leg does not prove on its own:* no script produced `_poppy_tier_probe/`, so
nothing on disk records the env of each of those four runs — only that four differently
named plates came out of four runs and that the fifth, `LOOK_DISABLE`, moved. I am not
resting the finding on it. The source path and the `perf_main` ladder below are the proof;
this table is what that proof predicts, confirmed.

**The clean A/B.** The same env var on `voxelforge_perf` — `perf_main.rs`, which does *not*
add `SettingsPlugin` — ladders properly (`_rose_look_perf_2026-08-04.log`, 600 frames/rung):

| tier | median ms |
|---|---|
| Low | 5.92 |
| Medium | 6.18 |
| High | 6.54 |
| **Ultra** | **16.61** |

One look.rs, one env var, two binaries: the one without `SettingsPlugin` shows a 2.5× cost
jump into Ultra, the one with it shows four identical frames. The difference between them is
that single `insert_resource`. (Same log: at Ultra, `no_pcss` 11.5 ms vs `full` 13.8 ms — PCSS
alone is ~2.3 ms, so it is not something a real Ultra frame could hide.)

**Not the feature flag.** The live rustc command line carries
`--cfg feature="experimental_pbr_pcss"`, so `dl.soft_shadow_size` is compiled in.

## What this costs, ranked

1. **There are no Ultra plates.** Not "grade-vista is the only Ultra plate" — zero. Every
   G-gate number, every look audit, every tier claim taken through `--play` describes High.
2. **N7 will reproduce the bug.** A `PCSS_WIDTH = 16.0` binary shot the usual way still
   renders PCSS-off. Flamingo needs one of the workarounds below or the sweep is wasted.
3. **The perf tier ladder and the PCSS width ladder stand.** `perf_main` bypasses the
   stomp, and `_pixel_pcss_ladder.ps1` sets `VOXELFORGE_LOOK_PCSS` per rung, which bypasses
   the tier. Both were measuring what they said. Nothing needs re-shooting there.
4. **`_poppy_pcss_probe`'s "default ≡ PCSS off"** (mean |ΔL| 0.10–0.23 against a 0.16–0.26
   repeat floor) is now explained rather than mysterious: default *was* off.

## Fixing it — zero-build workarounds first

For the N7 shot today, off the binary Flamingo is already building, pick either:

* `VOXELFORGE_LOOK_PCSS=16` on the capture — forces the width past the tier gate. Proven to
  reach the frame (45.4 % of px). Leaves the rest of Ultra (96-step fog, Ultra SSAO) still
  running as High, so it isolates PCSS and nothing else.
* or edit `settings.json` → `"graphics": "Ultra"` before shooting. That gives a genuinely
  full Ultra frame; remember to put it back to `"High"`, it is a tracked file.

## The patch, for Flamingo's lane

`docs/patch-tier-env-outranks-settings-2026-08-09.patch` — 69 insertions, 11 deletions,
`git apply --check` verified against the tree as it stands **including Flamingo's open
`POST_SATURATION` edit**. I did not touch `client/src/` and I have **not compiled it** — the
build lane is his.

```
git apply docs/patch-tier-env-outranks-settings-2026-08-09.patch
```

Three pieces:

1. `look.rs` — the tier-env parse becomes `pub fn quality_from_env()`, so both plugins ask
   the question the same way instead of one guessing.
2. `settings_menu.rs` — `if let Some(tier) = crate::look::quality_from_env() { settings.graphics = tier; }`
   before the insert. The env wins; the menu shows the tier that is actually running.
3. `look.rs` — a one-line receipt per tier change, printed into every capture's stdout log
   beside its plate:

   ```
   LOOK tier=Ultra pcss=Some(16.0) shadow_map=4096 sun=22deg/205deg illum=22000 contact=true
   ```

   This class of bug is only invisible because no artefact of a run ever recorded which
   tier reached the frame. It should never be invisible again — and it answers your
   "log the map size actually used" directly, from the resource, not from the source.

**After it lands, the falsifiable check** (no new tooling): shoot `grade-vista` twice off
the patched exe, `VOXELFORGE_LOOK_QUALITY=ultra` vs `=high`. They must now differ by roughly
what a forced `=16` differs by today (2.50 mean |ΔL|, 45 % of px), and the stdout log must
read `tier=Ultra pcss=Some(16.0) shadow_map=4096`. If either fails, this note is wrong and
I want to know.

## Files

* `docs/patch-tier-env-outranks-settings-2026-08-09.patch` — the fix, unapplied.
* `_poppy_tier_probe/boot-{low,medium,high,ultra,lookdisabled}.png` — the four-tier probe
  (2026-08-01, local capture, gitignored). Its logs carry `SETTINGS loaded settings.json`.
* `_rose_look_perf_2026-08-04.log` — the perf-bin tier ladder that does work.
