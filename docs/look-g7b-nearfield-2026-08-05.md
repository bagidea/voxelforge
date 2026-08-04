# G7b — the near-field haze floor, and the HAZE_DESAT bake (2026-08-05, flamingo)

Follow-on to `docs/look-g7-haze-2026-08-05.md`, which closed with two open bugs.
This closes **A2 (near-field haze floor)** and bakes the **`hd60`** row it left as
"measured, not yet baked". Bug **B (SSAO not biting)** is untouched and still open.

The instruments are committed — `scripts/_flamingo_g7_probe.ps1`,
`_flamingo_g7_probe2.ps1`, `_flamingo_g7_alpha.py`, `_flamingo_g7_a2mask.py`,
`_flamingo_g7_curvefit.py` — because the argument below is only as good as they
are, and the first version of one of them was wrong (see "the instrument was
wrong first").

---

## 1. The A2 diagnosis in the G7 record was wrong

G7 recorded: *"19.5 of movement on ground a few blocks from the camera, where
`ExponentialSquared` @ 0.0072 predicts < 2 % opacity. Something adds a near-field
term the density curve does not account for."*

There is no such term. Both halves of that sentence were checked against the
shipped binary and one of them does not survive.

**The distance is real.** `VOXELFORGE_LOOK_FOG=S,S+0.5` turns fog alpha into a
hard step at distance `S`: nearer than `S` a pixel is byte-identical to the
zero-fog frame, further it is 100 % haze colour. Ten of those frames make the
frame into a depth ruler. The pinned framing's foreground band (bottom 18 % of
rows, which is what A2 grades) brackets at **p05 16 / p50 20 / p95 25 blocks** —
so "a few blocks from the camera" is really 16–25, and `ExponentialSquared` @
0.0072 predicts 1.32 % / 2.05 % there, not "< 2 %" flat.

**The alpha is also real.** `VOXELFORGE_LOOK_FOG=-30000A,30000-30000A` holds alpha
CONSTANT at `A` over the whole depth range, so a ladder of those frames is a
per-pixel response curve: display value as a function of a *known* alpha, with
tonemap, bloom and grade all included. Reading the shipped frame back against it
gives the alpha the shader actually applied:

| measured depth | curve predicts | shader applied | excess |
|---|---|---|---|
| ≤ 16 | 1.32 % | **1.15 %** | 0.87× |
| ≤ 20 | 2.05 % | **1.59 %** | 0.78× |
| ≤ 25 | 3.19 % | **2.12 %** | 0.67× |
| ≤ 40 | 7.96 % | **5.01 %** | 0.63× |
| ≤ 75 | 25.29 % | **15.61 %** | 0.62× |

The curve is honoured — under-applied if anything, never over. **The near-field
floor does not exist.**

### What the 19.5 actually is

`delta ≈ alpha × |haze − surface|`, and the second factor is the one nobody
measured. Near-field ground in this framing is shadowed grass; the haze colour is
derived from the sky and sits several times its radiance. So 1.6 % of alpha still
moves it 19 display units — while the far band, already close to the haze colour
and deep in the tonemap's compressive region, turns 14–25 % of alpha into only
~44.

Which makes the ratio structural, not tunable. Re-scored across the entire G7
sweep on a mask that can carry the signal (§2):

| row | density / colour | far | near | ratio |
|---|---|---|---|---|
| h0068 | ρ 0.0068 | 40.19 | 17.29 | 2.33 |
| ship  | ρ 0.0072 | 44.28 | 18.92 | 2.34 |
| h0090 | ρ 0.0090 | 61.36 | 25.95 | 2.37 |
| h0120 | ρ 0.0120 | 85.40 | 35.63 | 2.40 |
| hd20 / hd60 | desat | 42.66 / 45.82 | 18.03 / 19.82 | 2.37 / 2.31 |
| hg045 / hg085 | gain | 34.20 / 57.73 | 14.66 / 23.88 | 2.33 / 2.42 |

Nearly doubling the density moves the ratio 2.33 → 2.40. **No value of any
`ExponentialSquared` constant reaches 2.5**, and that falloff has no offset
parameter — so the near field can only be bought back by a curve that starts
somewhere.

### The instrument was wrong first

The first alpha ladder was sampled at 0.05 / 0.10 / 0.20 / 0.40 / 1.00 and
reported a **6.5 % constant floor** — a clean, plausible confirmation of exactly
the bug that was being looked for. It was an artefact: interpolating a pixel
between a=0 and a=0.05 draws a straight chord under a *concave* tonemap response,
which over-reads alpha precisely in the 0–5 % near-field bucket. Re-sampling the
ladder at 0.5/1/2/3/5/8/12/16 % dropped the "floor" from 6.5 % to 0.36 %.

`_flamingo_g7_alpha.py --selftest` now holds each rung out and reads it back off
the rest, and gates on the range the argument lives in (α ≤ 16 %): worst error
+0.96 pp, on a hold-out that spans double the real gap. That number is the error
bar on the table above.

---

## 2. There is an instrument bug in A2 — and it is NOT what let this pass

`grade_g7.axis_a2` averages the top 18 % of rows against the bottom 18 %. In this
framing **59 % of the top band is sky** (21 533 of 36 480 px), and sky is the one
region distance fog can never move, because there is no geometry behind it to sit
in front of. Every one of those pixels enters the far mean as a hard zero. On the
G7 frame that dragged a real 2.36 down to the reported **0.97** — the shot's sky
fraction being graded, not the haze. The G7 record already made this argument in
the other direction ("the sky mask **cannot** show a distance-fog cost"); it
applies identically to the far band.

**That bug was not fixed to land this change, and this change does not depend on
it.** `scripts/grade_g7.py` is committed here **unmodified, byte for byte at
`HEAD`**, and the shipped frame clears it anyway:

| grader | frame | far | near | ratio | verdict |
|---|---|---|---|---|---|
| stock (`HEAD`) | G7b ship | 19.84 | 0.18 | **112.63** | **PASS** |
| stock (`HEAD`) | G7 ship (old curve) | 18.18 | 18.79 | 0.97 | FAIL |
| sky-masked (proposed) | G7b ship | 48.35 | 0.18 | 274.45 | PASS |
| sky-masked (proposed) | G7 ship (old curve) | 44.27 | 18.79 | 2.36 | FAIL |

Read the first column: the render earns the gate under the **original thresholds
and the original mask**. The sky mask is directionally *looser* — it raises both
graded terms in every case above — so under the office rule that gates get
stricter and never looser, it does not travel with a change that would benefit
from it. Both grader variants agree on both frames regardless, which is the
strongest thing that can be said for either.

The mask fix is real and still worth making, so it is handed over as a proposal
instead of applied: **`docs/patches/g7b-a2-sky-mask.patch`**, for the Director
(`scripts/grade_g7.py` is not in a `docs/LANES.md` lane). Its guardrails, for
whoever reviews it —

* The mask is derived from the **off** frame, so it is identical for every
  candidate and cannot be moved by the thing under test.
* It is measured, not guessed: sky pixels land in R 122–128 / G 160–167 /
  B 222–228, and **21 321 of the 21 962** the band contains have *exactly* zero
  delta under the shipped haze.
* `T_DEPTH_RATIO` is untouched at 2.5; the sky-included number is still printed
  on every run, so nothing is hidden by the mask.
* It does not launder the old frame through: the G7 render still FAILs at 2.36.

---

## 3. What shipped

| constant | was | now |
|---|---|---|
| haze falloff | `ExponentialSquared { density: 0.0072 }` | `Linear { start: HAZE_START, end: HAZE_FULL }` |
| `HAZE_START` | — | **20.0** blocks (new) |
| `HAZE_FULL` | — | **250.0** blocks (new) |
| `HAZE_DESAT` | 0.40 | **0.60** |

`HAZE_DENSITY` 0.0072 stays as the curve `VOXELFORGE_LOOK_HAZE=<density>` selects,
so the before/after against the previous default still comes out of one binary.

**`HAZE_START` 20** — bounded below by the camera (`BOOM_DIST` 6.5, so 20 is three
boom-lengths: the player, the block under them, everything in reach and the whole
melee volume take zero haze at any orbit pose) and above by the set (~55 blocks
deep; the ramp needs most of it). Measured against the pinned framing, it leaves
half the foreground band at exactly zero and the rest under 2.1 %.

**`HAZE_FULL` 250** — chosen to keep the look that was already signed off. Fitting
`Linear{20, end}` against `ExponentialSquared`(0.0072) over the framing's measured
depth span (26–78 blocks) puts the least-squares optimum at 248; 250 is the round
number next door, rms 0.80 pp and **max deviation 1.83 pp anywhere in the set**:

| d | 16 | 20 | 26 | 30 | 40 | 55 | 75 | 150 | 250 | 320 |
|---|---|---|---|---|---|---|---|---|---|---|
| ExpSq 0.0072 | 1.3 % | 2.1 % | 3.4 % | 4.6 % | 8.0 % | 14.5 % | 25.3 % | 68.9 % | 95.7 % | 99.5 % |
| Linear 20→250 | **0 %** | **0 %** | 2.6 % | 4.4 % | 8.7 % | 15.2 % | 23.9 % | 56.5 % | **100 %** | **100 %** |

Across everything the player can see, the two are the same picture. They differ
where it matters: zero across the gameplay foreground, and **actually closed**
past 250 rather than asymptoting to 99.5 % at the streaming edge — so the last
0.5 % of a popping chunk no longer shows through forever. `HAZE_FULL <=
RENDER_RADIUS` is a compile-time assert.

> **Handoff to the streaming lane (Kevin).** Nothing between `HAZE_FULL` (250) and
> `RENDER_RADIUS` (320) is visible any more — that 70-block shell is now pure draw
> cost. Tightening `RENDER_RADIUS` to `HAZE_FULL` is available and is your call;
> this lane did not take it, and `RENDER_RADIUS` is unchanged at 320.

**`HAZE_DESAT` 0.60** — the `hd60` row G7 measured and declined to ship without a
rebuild. It is now compiled in, and the table below was re-shot with
`VOXELFORGE_LOOK_HAZEDESAT` **unset** to prove the out-of-box frame reproduces it;
the `envhd60` row sets it explicitly as the cross-check.

---

## 4. Measured, on the rebuilt binary

One binary — `target-flamingo/perf/voxelforge.exe`, built 03:17:39, newer than
every source it contains (`look.rs` 03:12:45); build log 0 `^error` lines,
`Finished perf profile in 1m 08s`. Eleven rows, same pinned camera
(`VOXELFORGE_LOOK_CAM=35,-18,26`), only the haze env swapped. `ship` carries **no
haze env at all** — it is the out-of-box default, the only row quotable as the
ship number under the G7 record's "an env crutch is not a default" rule.

**A2 — graded by `scripts/grade_g7.py` at `HEAD`, unmodified** (`T_DEPTH_RATIO`
2.5, `T_FAR_DELTA` 6.0):

| row | what it is | far | near | ratio | A2 |
|---|---|---|---|---|---|
| **ship** | **shipped default, no env** | **19.84** | **0.18** | **112.63** | **PASS** |
| envhd60 | `HAZEDESAT=0.60` set explicitly | 19.84 | 0.21 | 96.34 | PASS |
| g7curve | G7 `ExpSq` 0.0072, same binary | 18.84 | 19.70 | 0.96 | **FAIL** |
| g7full | G7 curve + `HAZEDESAT=0.40` | 18.19 | 18.85 | 0.96 | **FAIL** |
| hd40 | ramp, `HAZEDESAT=0.40` | 19.17 | 0.24 | 78.63 | PASS |
| s16 / s24 | `HAZE_START` 16 / 24 | 21.84 / 17.71 | 1.62 / 0.24 | 13.44 / 74.54 | PASS |
| e220 / e280 | `HAZE_FULL` 220 / 280 | 22.24 / 17.88 | 0.21 / 0.26 | 104.65 / 69.04 | PASS |
| ssaooff | SSAO lifted out | 19.93 | 0.62 | 31.99 | PASS |

`g7curve` is the before/after that matters: the **same binary**, the **same
scene**, only the falloff swapped back to G7's, reproduces G7's reported 0.97 to
within noise (0.96) and still FAILs. Nothing but the curve moved.

Note what carries the pass. The far band moved 19.84 (needed 6.0) — but the ratio
is 112.63 because `HAZE_START` 20 puts the near band at **0.18**, i.e. a
structural zero, not a small number. A dead zone satisfies a ratio gate trivially,
so `T_FAR_DELTA` is the term doing real work here and the ratio is close to
uninformative on any row with a dead zone. Flagged rather than banked: if A2 is
ever re-cut, the far term is the half worth tightening.

**The bake reproduces out of the box.** `ship` (env unset) and `envhd60`
(`VOXELFORGE_LOOK_HAZEDESAT=0.60`) are identical on every graded number —
G3 p05 21.9, G5 spread 53.6, G6 R−B 145 / L 57.5, veg sat 74.2, veg hue 90.2,
micro-contrast 5.45, A2 far 19.84. The 0.18 / 0.21 near-band split is TAA jitter
on a band whose true value is zero. `HAZE_DESAT` 0.60 is compiled in, not env'd.

**G3 / G5 / G6 stay green, and the near-field floor lifted.** All eleven rows are
P/P/P (`_flamingo_g7b/g7.tsv`). Interior floor G3 p05 goes **17.5 → 21.9** from
zero-haze baseline to ship; micro-contrast 5.45, over the 5.0 gate.

**The two axes that do not pass, both pre-existing and both out of this lane:**

| axis | measured | needs | owner |
|---|---|---|---|
| B — AO bite | p99 1.31, coverage 0.40 % | 10.0 / 2.0 % | **rose** (`look.rs` SSAO) |
| C — vegetation | sat 74.2, hue 90.2 (13.1 % / −2.5 % closed) | 62.8 / 71.1 (40 %) | **shiba** (`voxel.rs` albedo) |

Neither regressed: axis B is byte-for-byte the G7 finding, and on axis C the
`HAZE_DESAT` 0.60 bake moves saturation the right way (79.8 zero-haze → 74.2
ship) without closing it. Both were open before this change and are open after —
see §5.

---

## 5. Still open

**B — SSAO is not biting.** Unchanged from the G7 record, and re-measured on the
G7b binary: `VOXELFORGE_LOOK_SSAO=off` vs on gives p99 darkening **1.31** (needs
≥ 10.0) and coverage **0.40 %** (needs ≥ 2.0) — G7 read 1.40 / 0.44 %, so this is
the same finding, not a regression. The layer is in the frame and the frame barely
changes. Not reachable by tuning. **This is the handoff to rose**, who owns
`look.rs`; the file is released with this commit and nothing here is mid-edit.

**Vegetation hue.** Axis C's hue half is still short of target and the remaining
distance is albedo — `BlockId::base_color` in `voxel.rs`, shiba's lane. `HAZE_DESAT`
0.60 moves saturation the right way (zero-haze 79.8 → ship **74.2**, gate 62.8) and
hue barely at all (ship **90.2**, gate 71.1, i.e. −2.5 % of the gap closed against
the 89.1 baseline). The haze cannot close this one; the palette has to.

**A2's ratio term is now nearly free.** With `HAZE_START` 20 the near band is a
structural zero (0.18), so `T_DEPTH_RATIO` 2.5 is cleared by construction on any
dead-zone curve and `T_FAR_DELTA` is the only term still discriminating. Worth
re-cutting when someone owns the gate; noted here so the 112.63 is not mistaken
for headroom.

**One new compiler warning, deliberately left.** `HAZE_DENSITY` is now
`never used` — the env hook parses its own float, so the const survives only as
the documented G7 default. It is left exactly as verified rather than annotated,
because touching `look.rs` after the graded build would have made the binary the
numbers above came from stale. Rose's call whether to `#[allow]` it or drop it.

**Proposed, not applied:** `docs/patches/g7b-a2-sky-mask.patch` — the A2 sky-mask
instrument fix from §2, for the Director. Not required by anything above.
