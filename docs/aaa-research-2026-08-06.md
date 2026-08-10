# AAA Look Research — 2026-08-06 (corrected)

> Director: Rose  
> Source of truth: `client/src/look.rs` (working tree, commit `f358809` base)  
> Cross-ref: `docs/aaa-gap-scorecard-2026-08-06.md` (Sun)  
> Constraint: **no `cargo build` in this session** — build queue owned by Yamamoto.

---

## Errata from first draft

The first version of this report was written against stale line numbers and stale values. The working tree already contains the following edits:

| constant | old report claimed | actual in working tree | location |
|---|---|---|---|
| `ambient` | `[0.96, 0.90, 0.60]` | `[0.96, 0.90, 0.48]` | `look.rs:601` |
| `ambient_lux` | `1100.0` | `2200.0` | `look.rs:602` |
| `PCSS_WIDTH` | `3.0` | `4.0` | `look.rs:354` |

This corrected report treats those values as **already applied** and reframes the conclusions as verification / next-sweep tasks rather than as edits to perform.

---

## 1. Composition + Lighting lane (values already applied)

### 1.1 What the scorecard says

Sun’s scorecard grades 5 frames against the golden reference. The two worst gaps are **G3 (interior darkness)** and **Warmth R−B**. The working tree has already addressed both by changing the ambient fill in `Hour::GOLDEN`:

| gap | target | worst frame | current value in tree | location |
|---|---|---|---|---|
| G3 p05-L | ≥ 8 % | gate3-walk **2.2 %** | `ambient_lux: 2200.0` | `look.rs:602` |
| Warmth R−B | ≥ +110 | gate3-combat **+80.4** | `ambient: [0.96, 0.90, 0.48]` | `look.rs:601` |

The warmest healthy frame, `wide-hero`, still sits at **R−B +108.6** (just under ref +118.5) and only passes because its open-shade fill is brighter.

### 1.2 Root cause (still valid)

`Hour::GOLDEN.ambient_lux` at the previous `1100 lux` was too dim for a 17° raking sun with `illuminance 11_000 lux`. Open shade (the midtone band that drives the warmth/sat axes) is lit almost entirely by the `AmbientLight`, so when it was under-powered the frame’s midtones dropped toward the shadow percentile and the warmth gap opened. The `gate3-*` frames are ruin/interior-heavy; they have far more open-shade limestone than `wide-hero` does, so the gap was largest there.

The ambient blue channel was dropped from `0.60 → 0.48` to stop `grade-vista`’s darkest shade from reading cold (`R−B = −4`). The `gate3-*` frames already had warm darkest shades (`R−B +18` to `+22`), so their G3 failure was **primarily a brightness problem**, not a colour problem.

### 1.3 Status

The edits are already in the working tree at `client/src/look.rs:601-602`:

```rust
// look.rs:601-602  (already applied)
ambient: [0.96, 0.90, 0.48],
ambient_lux: 2200.0,
```

The commit comment at `look.rs:578-600` documents the reasoning and the magenta envelope (`G−B 0.42`, `G/R 0.94`). **No further code change is recommended here until the frames are re-shot.**

### 1.4 Verification needed

1. Re-capture `grade-vista`, `gate3-boot`, `gate3-combat`, `gate3-walk` with the current tree (requires Yamamoto’s build).
2. Run `scripts/grade_gate.py` and `scripts/grade_midtone.py` on all four frames.
3. Run `scripts/colour_gate.py` to confirm magenta fraction stays under the gate.
4. Acceptable exit state: G3 PASS on all frames **and** Warmth R−B ≥ +110 on all frames.

If G3 still fails after the ambient lift, the next knob is **not** more lux — 2200 is already close to the hero shot’s 2800. Instead, sweep `VOXELFORGE_LOOK_AMBIENT=2000..2600` and `VOXELFORGE_LOOK_LIGHT=...,...,0.45..0.52` to find the sweet spot without blowing the magenta gate.

---

## 2. Fog / Haze lane

### 2.1 Current constants (baked as of `f358809`)

| const | value | location in `client/src/look.rs` | what it does |
|---|---|---|---|
| `HAZE_START` | `20.0` | `look.rs:111` | Dead zone: zero fog inside 20 blocks |
| `HAZE_FULL` | `150.0` | `look.rs:194` | Linear ramp ends; opaque by 150 blocks |
| `HAZE_DENSITY` | `0.0100` | `look.rs:257` | **Reference curve only** — used only when `VOXELFORGE_LOOK_HAZE=<density>` is set |
| `HAZE_DESAT` | `0.60` | `look.rs:281` | How far haze hue is washed toward white |
| `HAZE_GAIN` | `0.62` | `look.rs:291` | Haze brightness as fraction of `Hour::sky_gain` |
| `FOG_COLOR_DAY` | `[0.94, 0.66, 0.26]` | `look.rs:327` | Horizon hue the haze dissolves toward |
| `FOG_END` / `RENDER_RADIUS` | `320.0` | `look.rs:294, 304` | Streaming edge; haze must be opaque here |

**Do not touch `HAZE_FULL`.** Commit `f358809` already refit it from `250.0 → 150.0` against `ExpSq(0.0100)`. The scorecard frames were graded after that change; the remaining G3 failures are not a haze-distance problem.

### 2.2 Why the gate3 G3 failures are NOT a haze problem

Sun’s scorecard G3 deep dive (`docs/aaa-gap-scorecard-2026-08-06.md` §2) shows:

| frame | p05-L | darkest shade | R−B | G3 |
|---|---|---|---|---|
| grade-vista | 6.4 % | (16,16,20) | **−4** (cold) | FAIL |
| gate3-boot | 2.8 % | (18,0,0) | +18 (warm) | FAIL |
| gate3-combat | 3.8 % | (19,0,0) | +19 (warm) | FAIL |
| gate3-walk | 2.2 % | (22,0,0) | +22 (warm) | FAIL |

Three pieces of evidence point away from haze/fog as the cause of the gate3 failures:

1. **Geography.** `gate3-*` are ruin/interior frames. The dark pixels live on interior limestone walls and floors that are well inside the `HAZE_START = 20.0` dead zone (`look.rs:111`). At distances below 20 blocks the shipped `Linear{20,150}` ramp is **exactly 0 % opaque** (`look.rs:891-908`, `haze_falloff`). Haze cannot darken pixels it does not touch.

2. **Colour signature.** The darkest shades in all three gate3 frames are warm (`R−B +18` to `+22`), not cold. If the haze wash were the culprit, the shadow colour would lean toward the haze hue (`FOG_COLOR_DAY = [0.94,0.66,0.26]`, i.e. warm orange) or toward the zenith sky; it would not be `(18,0,0)`. The fact that green is crushed to 0 and red dominates is the signature of **too little ambient fill**, not the wrong colour of fill.

3. **Percentile shape.** `gate3-combat` has `p10-L = 8.6 %`, which already clears the 8 % threshold. Only the bottom 5 % of pixels are failing. That is exactly what happens when the `AmbientLight` floor is set too low: a thin tail of fully occluded pixels drops to black while the rest of the open-shade band is fine. A haze problem would darken a broad depth band and show up at higher percentiles.

### 2.3 Why grade-vista is different

`grade-vista` is the one frame where haze colour **does** matter. Its darkest 5 % reads `(16,16,20)` → `R−B = −4`, i.e. the shadow is cold/blue. That cold cast comes from open-shade pixels that are far enough from the camera to have haze mixed in, and the haze is desaturated sky (`HAZE_DESAT = 0.60` at `look.rs:281`). The fix for `grade-vista` is the same two-light adjustment already applied at `look.rs:601-602`: raise `ambient_lux` and drain `ambient[2]` to `0.48`. The haze constants themselves are correct for the Edhari scene.

### 2.4 Actionable recommendation

**No change to haze constants is required to close G3 on gate3.** The applied edit is at `client/src/look.rs:601-602` (`ambient` / `ambient_lux`).

If, after the ambient fix, the far skyline on `grade-vista` still reads too cold, the next haze knob to sweep is `HAZE_DESAT` at `look.rs:281` via the env hook:

```bash
VOXELFORGE_LOOK_HAZEDESAT=0.55
```

Do not commit a lower `HAZE_DESAT` without re-shooting `grade-vista` and re-running `colour_gate.py`, because washing the haze toward white changes the far-band hue and can re-open the magenta gate.

### 2.5 Verification needed

1. With `ambient_lux: 2200.0` / `ambient[2]: 0.48`, re-capture `gate3-boot`, `gate3-combat`, `gate3-walk`.
2. Confirm G3 p05-L ≥ 8 % on all three using `scripts/grade_g3.py`.
3. If `grade-vista` still shows `R−B < 0` in its darkest 5 %, sweep `VOXELFORGE_LOOK_HAZEDESAT=0.50..0.60` and pick the highest value that clears both G3 and `colour_gate.py`.

---

## 3. PBR / Materials lane

### 3.1 Current constants in `client/src/look.rs`

| const | value | location | what it does |
|---|---|---|---|
| `AO_THICKNESS` | `1.45` | `look.rs:344` | SSAO object thickness — how deep a crease must be before it darkens |
| `PCSS_WIDTH` | `4.0` | `look.rs:354` | PCSS penumbra width for the sun (Ultra tier only); **already raised from 3.0** |

The actual per-block surface table lives in `client/src/voxel.rs` (`block_surface`/`block_relief`/`block_material`). I read it; its roughness/reflectance/emissive values are sane and already gate-tested (voxel.rs:125-171, 189-220). No immediate change there.

### 3.2 What the scorecard says

Sun’s scorecard G4a (penumbra):

| frame | measured | target | status |
|---|---|---|---|
| wide-hero | 4 px | ≥ 5 px | ❌ |
| grade-vista | 3 px | ≥ 5 px | ❌ |
| gate3-boot | 8 px | ≥ 5 px | ✅ |
| gate3-combat | 5 px | ≥ 5 px | ✅ |
| gate3-walk | 9 px | ≥ 5 px | ✅ |

Only the two **vista/hero establishing shots** fail penumbra. Those are exactly the frames with the longest sun shadows (17° raking sun over open ground), where a fixed-size penumbra filter runs out of reach.

### 3.3 Root cause

`PCSS_WIDTH` was `3.0` and has been raised to `4.0` at `look.rs:354`. The commit comment at `look.rs:347-354` states this was done specifically because `wide-hero` read 4 px and `grade-vista` read 3 px against G4a’s 5 px floor. The gameplay frames pass because they are shot at High tier and take the Gaussian path instead.

Whether `4.0` is enough to clear the 5 px gate must be verified on a fresh capture. The previous 3.0 → 4.0 step is +33 % width; measured penumbra does not scale 1:1 with width because the shadow-map resolution and cascade split also matter.

`AO_THICKNESS = 1.45` is not the cause of the G3 failures. SSAO darkens only the diffuse indirect term (`look.rs:1073-1076`), so under an 11 000-lux key and 2 200-lux fill it can move a sunlit face by at most a few percent. The gate3 dark pixels are fully occluded/indirect pixels, so they take the full AO bite, but the bite is small compared with the ambient-lux shortfall. Fix the ambient fill first; only then re-measure AO if shadows still look sooty.

### 3.4 Actionable recommendation

The PCSS change is **already applied** at `client/src/look.rs:354`:

```rust
// look.rs:354  (already applied)
pub const PCSS_WIDTH: f32 = 4.0;
```

**Verification is the remaining work:**

1. Re-capture `wide-hero` and `grade-vista` at `LookQuality::Ultra`.
2. Run `scripts/measure_penumbra.py` and confirm median penumbra ≥ 5 px.
3. Re-run `scripts/grade_gate.py` G4a on all five frames.

If `4.0` still does not clear 5 px on the establishing shots, sweep via the env hook before committing a higher value:

```bash
VOXELFORGE_LOOK_PCSS=4.5
```

If `4.0` already clears, the value stays as-is.

**Do not change `AO_THICKNESS` now.** Wait until the `ambient_lux` fix is verified. If G3 still shows sooty contact corners after the fill is raised, then sweep `VOXELFORGE_LOOK_SSAO=1.0..1.8` and commit the value that clears `grade_g3.py` without killing the contact-shadow read.

---

## 4. Executive summary

| rank | gap | root cause | file + line | status | who |
|---|---|---|---|---|---|
| 🥇 | G3 crushed shadows + Warmth R−B | `ambient_lux` too low / `ambient[2]` too blue | `look.rs:601-602` | **Already applied:** `ambient[2] 0.60→0.48`, `ambient_lux 1100→2200` | Rose (verify) |
| 🥈 | Gate3 G3 misread as haze | Shadows live inside `HAZE_START=20` dead zone | `look.rs:111` | **No haze change needed** — fix is ambient lux above | Rose |
| 🥉 | Penumbra on wide-hero/vista | `PCSS_WIDTH` too small for open shots | `look.rs:354` | **Already applied:** `3.0→4.0` (Ultra); verify ≥5 px | Rose (verify) |
| 4 | Saturation on vista/boot | Likely pre-`POST_SATURATION=1.90` exe | `look.rs:450` | Rebuild + re-capture to confirm; do not raise blindly | Rose/Yamamoto |

**One line:** the code changes the first report asked for are already in the tree at `look.rs:601-602` and `look.rs:354`. The remaining work is **verification**: rebuild, re-capture the five frames, and confirm G3 / Warmth / G4a all pass.

