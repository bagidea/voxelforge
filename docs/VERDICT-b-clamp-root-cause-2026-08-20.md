# VERDICT — why the P0-ENV frame has zero cool pixels (2026-08-20)

Flamingo / look lane. Answers the CEO's question in one line, then shows the
work. **Read the "Do not open the lighting lane on this number" section before
deciding anything** — the two constants below are real, and they are not what
closes the 0.0 % → 9.5 % gap.

Frame under test: `hero-look-final-nohud2.png` (1280×640).
Reference: `docs/refs/ceo_ref_sunset_valley.jpg` (768×1376).
Every line reference is `client/src/look.rs` at `poppy/native-only` HEAD unless
said otherwise. Probes: `scripts/_flamingo_coolpix_probe.py`,
`scripts/_flamingo_shade_budget.py` (both read-only, both re-runnable).

---

## The answer — the two values that clamp B

Ranked by measured effect on the rig's summed shade B/R, each knocked back to
its own predecessor with everything else left shipped:

| # | constant | declared | shipped | was | apply path | Δ B/R |
|---|---|---|---|---|---|---|
| 1 | `AMBIENT_LUX_V3` spent on `Hour::GOLDEN.ambient` | `:624` / `:1749` | 620 lux × sRGB [0.96, 0.90, 0.48] | 380 | `hour()` `:1979-1984` → `apply_look_to_camera` `:2987-2988` (`ambient.color = light_colors().1; ambient.brightness = h.ambient_lux;`) → `light_colors()` `:2106-2118` | **+0.058** |
| 2 | `IBL_HORIZON_MIX` | `:593` | 0.68 | 0.55 | `ibl_env` `:3399-3411` (`mid = lerp_lin(top, bottom, IBL_HORIZON_MIX)` `:3402` → `EnvironmentMapLight::hemispherical_gradient(top, mid, bottom)` `:3405-3409`) | **+0.045** |
| 3 | `IBL_NITS_DAY` | `:530` | 330 nits | 440 | same site, `intensity: nits` `:3404` | +0.011 |

**#1 — `Hour::GOLDEN.ambient` at 620 lux is the warmest light in the day rig and
the only one that cannot be dodged.** In linear space its channel ratio is
**B/R 0.215** — warmer than the sun key itself (0.342). And it is the *flat*
term: `AmbientLight` gives every surface the same irradiance regardless of which
way it faces, so 620 lux of B/R 0.215 lands on 100 % of the frame at 100 %
strength, including the one surface class the cool `sky_fill` is trying to make
cool. It is a warmth floor under every pixel. It was **doubled, 380 → 620, on
2026-08-15**, deliberately, to buy warmth back (`:523-529` records the reasoning).

**#2 — `IBL_HORIZON_MIX = 0.68` is the one that gets the walls.** The horizon
band is what a *vertical* face integrates, and this frame is almost entirely
vertical faces. 0.68 drags that band from linear **B/R 0.806 → 0.590** by leaning
it on `Hour::bounce` [1.00, 0.78, 0.50] (B/R 0.214). Same 2026-08-15 move, same
motive.

### It is not the ones everyone suspects

- **Not the haze/fog.** `FOG_COLOR_DAY` `:405` is warm, but the shipped path runs
  it through the sky-tinted mix at `:2455-2457` / `:2474` and comes out B/R 0.810
  — the *least* warm term in the rig.
- **Not the tonemapper or the grade.** `:2599` / `EV_TRIM` (`:2542`, v4 only,
  default returns 0.0) are channel-neutral here.
- **Not the sky dome — and I said it was in my last report. That was wrong.**
  I named `sky_paint` / `sky_gradient_sunset.png` × `SKY_PAINT_GAIN 4.0` as the
  bigger of the two causes. **This frame is an interior. It contains no dome
  pixel at all.** The plate is in fact the *coolest* asset in the project
  (20.87 % of its own texels are B ≥ R, top rows B/R 1.410) and `SKY_RAMP_CURVE`
  0.45 → 0.30 (`:3943`, 2026-08-19) moved the framed band *cooler*, not warmer.
  Retract the claim entirely.

### Why nobody caught either move

Both are `2026-08-15`, both were graded against a **one-sided** gate: the look
lane has only ever had an R−B *floor* (a "is it warm enough" clause). There was
no cool-side gate at all until `cool_chroma_pct` landed **2026-08-18**, three
days after. A change that spends blue to buy warmth scores as a pure win under a
warmth-only rubric, every time, by construction.

---

## Do not open the lighting lane on this number

The 0.0 % vs 9.5 % figure is real but it is **not a lighting defect**, and the
two constants above will not move it. Three measurements, in order:

**1. The frame is not merely blue-poor, it is 31 levels clear of neutral.**

```
count(B >= R)        0 / 819 200 px      count(B >= R-30)     0 px
min(R - B) over the whole frame = 31     ref max(B - R) = +181
```

Not one pixel in 819 200 comes within 30 levels of neutral. There is no
"almost cool" region for a knob to push over the line.

**2. Undoing all three constants is worth +19 % on B, and buys zero cool pixels.**

Knocking `AMBIENT_LUX_V3`, `IBL_NITS_DAY` and `IBL_HORIZON_MIX` all the way back
to the 2026-08-14 rig takes the summed shade B/R **0.642 → 0.766**. Applied to
the real frame as a linear B gain:

```
B × 1.00  ->  0 px cool   min(R-B) 31.0     <- shipped
B × 1.19  ->  0 px cool   min(R-B) 14.8     <- all three constants undone
B × 1.30  ->  0 px cool   min(R-B)  6.0
B × 1.50  ->  1.43 % cool                   <- first cool pixel appears here
B × 3.50  ->  2.49 % cool                   <- frame-destroying, still a quarter of REF
```

The full rollback does not reach the *first* cool pixel. Even ×3.5 — an amount
that would wreck the look — tops out at 2.49 % against the reference's 9.52 %.

**3. The 9.5 % target comes from a different scene class.**

`ceo_ref_sunset_valley.jpg` is a portrait outdoor vista: open sky, snow, blue
water, and hectares of green foliage. `hero-look-final-nohud2.png` is a landscape
**interior kitchen** — wood, brick, plaster, warm albedo wall to wall, no sky in
frame. The reference's cool pixels are overwhelmingly *albedo* (water, snow,
sky), not *illumination*. Grading an interior's cool-pixel share against an
outdoor valley's is the `verify-golden-ref-covers-scene-class` trap, and the
rig cannot win it at any setting.

---

## What I'd actually do (impact vs effort)

1. **Fix the gate, not the lights** *(low effort, high impact)* — `cool_chroma_pct`
   needs a scene-class-matched reference before its number means anything.
   Either shoot an outdoor plate for the valley ref, or pair the interior frame
   with an interior ref. Until then the 9.5 % bar is unfalsifiable.
2. **Then re-open #1 and #2 on their own merit** *(medium effort)* — `AMBIENT_LUX_V3`
   620 and `IBL_HORIZON_MIX` 0.68 both passed under a warmth-only rubric and
   neither has ever been graded on the cool side. That is worth a one-binary
   A/B (`VOXELFORGE_LOOK_IBL`, `VOXELFORGE_LOOK_LIGHT`) regardless of the 9.5 %
   question. Expect a subtler frame, not a blue one.
3. **Do not touch the sky rig for this** — it is not implicated in this frame.

## Provenance

- `scripts/_flamingo_coolpix_probe.py` — cool-pixel census + per-band split +
  per-row plate ratios.
- `scripts/_flamingo_shade_budget.py` — per-channel linear shade budget from
  look.rs's own constants, plus the knock-back table. Every input literal
  carries its `look.rs` line number in-source.
- Neither script runs the engine, loads a frame it was not given, or writes
  anything. The B-gain ladder in §2 is a linear-space approximation applied to
  the shipped frame, labelled as such — it establishes a bound, not a render.
