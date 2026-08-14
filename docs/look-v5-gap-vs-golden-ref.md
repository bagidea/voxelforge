# Look v5 — gap read against `docs/assets/golden-beauty-shot-ref.png`

Poppy, 2026-08-15. Compares the signed-off beauty reference against the three
v4 after-plates committed in `b361a9d`:

- `docs/assets/look/outdoor-noon_after.png`
- `docs/assets/look/evening-raking_after.png`
- `docs/assets/look/night-firelit_after.png`

All three were shot from ONE binary (`59a76ae7…`, perf profile) with only
`VOXELFORGE_LOOK_GEN` moving, so before = v2 and after = v3 differ by the grade
and nothing else.

## The numbers the eye reads

Rec.709 luma percentiles, one grader over all four plates:

| plate | p05 | p50 | p95 | p95/p05 | L&lt;8 | any ch ≥250 |
|---|---|---|---|---|---|---|
| **golden-beauty-shot-ref** | 21.86 | 55.51 | 165.83 | **7.59** | **0.00 %** | 4.89 % |
| outdoor-noon_after | 48.47 | 93.01 | 141.75 | 2.92 | 0.27 % | 0.28 % |
| evening-raking_after | 18.68 | 71.96 | 120.60 | 6.45 | 1.89 % | 0.36 % |
| night-firelit_after | 19.63 | 39.27 | 155.49 | 7.92 | 0.08 % | 3.48 % |

## Gaps, worst first

1. **Contrast ratio — `outdoor-noon` is the outlier, not the good one.**
   p95/p05 = 2.92 against the reference's 7.59. Everything in that frame is
   packed into one milky mid band (p50 93.0 riding under a p95 of only 141.8).
   Evening and night already sit near the reference's ratio — but see (2) for
   which end they bought it from.

2. **Shadow floor bottoms out — the reference's never does.** 1.89 % of
   `evening-raking_after` is below L=8 and its p01 is 5.85; the reference has
   literally zero pixels there and a p01 of 18.38. Its darkest cabinet recess
   still carries hue and readable edges. Ours go to hole-black — the wall on the
   right of `night-firelit_after` is the clearest case, and both day plates have
   unlit faces reading as holes in the skyline.

3. **Sky-to-ground colour bleed is missing on the faces that need it.** The
   reference's shadow side is a warm brown that clearly came off the floor and
   the wood; ours is a neutral-to-cool mud. This is the same defect as (2) seen
   in hue instead of level — the fill that should be arriving there is short.

4. **Warmth is going DOWN across v2 → v3**, which is backwards. Measured
   R−B over the midtone band: 79.15 → 73.26 evening-raking, 43.10 → 37.50
   night-firelit. v3's only new cool light is the rim kicker.

5. **Rim separation is present but not paid for.** The kicker draws its edge,
   and it also lands on every face turned within 90° of it (no shadow map), so
   it is buying separation with frame-wide desaturation — gap (4).

6. **No volumetric shafts.** The reference has real light shafts through the
   window; `evening-raking` is the scene built for them and has none.
   NOT PICKED THIS ROUND — the known blocker is "no occluders" in `scene.rs`,
   which is not this lane's file, and the memory note on it is explicit that
   god rays never render in play. Flagged for the scene lane, not guessed at.

7. **Contact shadows read but do not ground.** Block-to-grass seams are the
   same width everywhere regardless of how far the caster is. Second-order
   next to (1)–(4) and partly a `scene.rs` geometry question.

## Picked for v5

**(A) The shadow floor and its colour — gaps 2, 3, 4, 5.**
**(B) The tonal ramp — gap 1.**

Both are root-caused below rather than dialled. Gap 6 is handed off.

### Root cause A — the v3 IBL budget transfer was short, and it was short by a factor it never modelled

v3's design moves 770 lux out of the flat `AmbientLight` (1150 → 380) and into a
hemispherical environment light, sized by `E = π·L`: 260 nits ≈ 817 lux. The
table in `IBL_NITS_DAY`'s own doc claims every surface lands within 3 % of its v2
value.

It does not. `EnvironmentMapLight::hemispherical_gradient` does not build a
hemisphere of radiance `L` — it builds one of `L × the map's own colour`, and the
colours handed to it are `Hour::sky_fill` and `Hour::bounce`, neither of which is
white. In linear space GOLDEN's are 0.62 and 0.59 mean, so the map delivered
~0.60 × 817 = **490 lux, not 817** — 327 lux short on every surface in the table.

The measurement agrees **and agrees in the right order**: going v2 → v3, shadow
p05 moved 49.70 → 48.47 (noon, −2 %), 21.56 → 18.68 (evening, −13 %), 25.76 →
19.63 (night, −24 %). The deficit scales with how dark the map's colours are, and
NIGHT's are much darker (linear means 0.47 / 0.32 ⇒ albedo ~0.40, so 9 nits
delivered ~11 lux of the 28 it was budgeted). A term that was designed to be
neutral was the largest single subtraction in the generation.

Two smaller eaters in the same generation:

- `CONTACT_SHADOW_LENGTH_V3` went 0.75 → 1.10 (+47 %), priced against PCSS and
  never against the shade floor — and `look.rs`'s own `ssao` note states the rule
  it broke: an in-shade face is ~100 % fill and takes the full bite.
- `RIM_COLOR` [0.78, 0.88, 1.00] at 600 lux is a shadow-map-less directional, so
  it lands frame-wide, not just on the silhouette. That is gap 4.

**Fixes** (`client/src/look.rs`):

| constant | v4 | v5 | why |
|---|---|---|---|
| `IBL_NITS_DAY` | 260 | **440** | 260 / 0.60 — the 817 lux the table budgeted, actually delivered |
| `IBL_NITS_NIGHT` | 9 | **23** | 9 / 0.40 — same correction, night's map is darker |
| `CONTACT_SHADOW_LENGTH_V3` | 1.10 | **0.85** | +13 % over v2 instead of +47 %; the direction v3 argued for at a third of the price |
| `RIM_COLOR` R | 0.78 | **0.86** | halves the R−B deficit, keeps B &gt; G &gt; R so the edge still reads cool |
| `RIM_LUX_NIGHT` | 30 | **20** | it was shipping at ~30 % of the night shade budget, not the 24 % it was sized for |

`RIM_LUX` (day, 600) is untouched — the brief is a brighter rim, not a dimmer one,
and the day level was never the problem.

### Root cause B — the tonal ramp, and one thing the eye got wrong

**Nothing in this lane clips.** The scattered flat-white blocks on
`outdoor-noon_after` and the campfire core on `night-firelit_after` read as
blown and are not: 0.28 % and 3.48 % of pixels have any channel ≥250, against
the reference's own **4.89 %**. The reference clips more. A highlight shoulder
to "recover" them would have fixed a defect that does not exist and made the
real one worse — this was caught by measuring, after the first draft of this
change had already written the shoulder.

The real defect is range. `HIGHLIGHT_GAIN` 0.86 is compressing headroom nothing
is using, and `MIDTONE_CONTRAST` 1.12 leaves the mid band packed.

**Fixes** — three v3-only grade constants, so `gen=v2` still reproduces the
2026-08-14 frame byte-for-byte:

| constant | shared | v3 | why |
|---|---|---|---|
| `SHADOW_GAIN_V3` | 1.0 | **1.08** | shadow *contrast* crushes (measured, 13.7 %→3.3 %); *gain* is a multiply and can only lift the toe |
| `MIDTONE_CONTRAST_V3` | 1.12 | **1.18** | spread the packed band; not 1.30, which the shared note records as crushing |
| `HIGHLIGHT_CONTRAST_V3` | 1.12 | **1.20** | reference p95→p99 spans 61 points, this lane's noon plate 37 |
| `HIGHLIGHT_GAIN_V3` | 0.86 | **0.98** | give back headroom the clip column says is unused; not 1.00, the sky's saturation bound is real |

The midtone and highlight-gain defaults are swapped inside `grade_knobs`, not at
the `ColorGrading` literal, so `VOXELFORGE_LOOK_GRADE` keeps overriding them
under v3 — gating at the literal would have made the sweep hook silently dead on
the only generation still being tuned.

## Acceptance

`scripts/_poppy_lookv5_gate.py` is the brief's three clauses as three columns,
`AND`-ed into one exit code: `p05(after) >= 20.40`, `warmth(after) >=
warmth(before)`, `spread(after) >= spread(before)`.

Run against the **v4** plates it reports 7 of 9 clauses FAIL — that is the gate's
own negative control, and it was run before the v5 plates existed. A gate whose
first green run is also its first run has not been shown to be able to go red.

## One metric is mislabelled, and it is not being silently fixed

`_poppy_lookv3_pairs.py`'s `orient` is documented as top-vs-side face separation
found from a 3×3 luma gradient. Its body computes no gradient and filters
nothing — `flat = l.copy()` followed by two percentiles — so it is
`median(L >= p75) - median(L <= p25)`, a tonal SPREAD.

Kept as-is and renamed `spread` at the point of use. The v3 and v4 plates on
record were graded with this exact function, and swapping in a corrected
estimator now would make the v5 row incomparable to the rows it is supposed to
be judged against. Correcting it is its own change with its own re-baseline.

## Not done this round

Gap 6, volumetric shafts. The reference's strongest single feature is the light
shafts through the window, and `evening-raking` — the scene built for them — has
none. The known blocker is that the god-ray pass has no occluders, which lives
in `scene.rs`, not this lane. Raised for the scene lane rather than guessed at
from here.
