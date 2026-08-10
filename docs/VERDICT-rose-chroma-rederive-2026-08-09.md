# VERDICT — blue / sat_honest / warmth re-derive + sky-band fix (Rose, 2026-08-09)

Answers Director's three asks (1) the unsatisfiable chromatic gates, (2) the
s1-vista sky-in-band masking bug. No `client/src/look.rs` edit; this is the
ruler, not the look. Supersedes the live `blue <=10` / `sat >=90` / `warmth >=110`
hard gates — they are retired to **advisory**, with evidence.

Probes: `scripts/_flamingo_band_probe.py`, `scripts/_rose_blue_ramp_probe.py`.
Locked by: `scripts/tests/test_grade_axes_chroma_rederive.py`.

---

## 0) Headline

`warmth`, `blue`, `sat_honest` can no longer be single-number HARD gates — proven
two ways. The sole honest chromatic-damage gate is **`clip`**. The three B-dependent
axes stay MEASURED + PRINTED (advisory, tune-toward-REF) so Yamamoto loses no signal,
but they no longer false-FAIL a healthy outdoor frame nor false-PASS a B-clamped wreck.

This is the "new structure" the Director's fallback clause allowed, chosen because
no per-scene-class or re-thresholded version is honest (shown below).

---

## 1) Defect 1 — clamp-INVERTED (warmth + blue reward damage)

All three axes ride on B. When `POST_SATURATION` over-drives and the swapchain clamps
B→0, **warmth R−B inflates** and **blue B collapses**. So a wrecked frame scores
BETTER on the retired gates than a healthy one:

| plate (GRADE→TERRAIN band) | clip | warmth | blue | retired warmth≥110 | retired blue≤10 |
|---|---|---|---|---|---|
| **N6 grade-vista @1.90 (WRECKED)** | 99.8% | **153.1** | **0.04** | **would PASS** | **would PASS** |
| **N6 hero @1.90 (WRECKED)** | 99.6% | **132.6** | **0.06** | **would PASS** | **would PASS** |
| N6 gate3-boot @1.90 | 99.8% | 132.7 | 0.05 | would PASS | would PASS |
| s1-vista @1.02 (HEALTHY) | 0.0% | 84.8 | 91.0 | would FAIL | would FAIL |
| s4-raking @1.02 (HEALTHY) | 3.5% | 71.4 | 48.6 | would FAIL | would FAIL |
| grade-vista @1.02 (HEALTHY) | 2.5% | 108.2 | 23.1 | would FAIL | would FAIL |
| hero @1.02 (HEALTHY) | 22.1% | 97.4 | 16.4 | would FAIL | would FAIL |

The retired gates call the wrecked frame a PASS and the healthy frames FAILs. That is
a gate that rewards the exact damage it exists to catch. (sat is honest only because
the clip co-gate forces it N-A; by itself legacy sat graded the same wreck as "100% sat".)

Pinned in test leg B so nobody can re-add them without confronting these numbers.

## 2) Defect 2 — scene-class SPREAD (no single honest threshold)

Even un-clamped, good outdoor plates span a range no one threshold fits. Calibrated on
the INDOOR ref (warm wood), the targets are unreachable outdoors without the clamping
that breaks the image:

| axis | REF (indoor) | good outdoor @1.02 range | a single target that… |
|---|---|---|---|
| warmth R−B | 120.9 | 71 – 108 | ≥110 fails 4/4 outdoor; ≤71 fails REF |
| blue B | 4.3 | 16 – 91 | ≤10 fails 4/4 outdoor; ≥91 fails REF, passes N6 |
| sat honest | 95.2 | 49 – 81 | ≥90 fails 4/4 outdoor; ≤49 passes N6 too |

Per-scene-class was rejected: even within "outdoor", blue spans 16 (grass vista) to 91
(sky-lit vista) — there is no per-class reference frame to calibrate against, so any
class target would be a number I invent, not one I measure. clip is the one statistic
whose threshold is measurable against a real known-bad (N6) and a real known-good (REF,
@1.02 ship).

## 3) The separator — clip

clip (terrain-midtone % railed) cleanly separates every shipped-good plate from every
N6 plate, with a wide gap and no per-scene dependence:

| | clip range | verdict (gate ≤35) |
|---|---|---|
| REF | 18.8% | PASS |
| good @1.02 (grade-vista / hero / s1-vista / s4-raking) | 0.0 – 22.1% | PASS (all 4) |
| N6 @1.90 (all 8 plates) | 58.0 – 99.8% | FAIL (all 8) |

Gap: good ≤22.1%, bad ≥58.0%. `clip ≤35` is the sole chromatic-damage gate.

## 4) Task-2 fix — sky inside the graded band

The midtone band was `L[p35,p75]` over the WHOLE frame. `s1-vista` had **37.9% sky
inside that band** (every other plate ≤0.1%) → it read `blue 101 / warmth 26` on a
fine frame. Fix: drop sky (`B>R AND L>median`) BEFORE the percentiles, band on terrain.

REF is indoor, 0% sky → terrain band == old band **bit-for-bit** (|Δ|=0.00e+00 on
warmth/blue/clip — pinned in test leg D). So every REF-relative number is unchanged;
only the two sky-bearing plates move toward sane values:

| plate | blue CUR→TER | warmth CUR→TER | clip CUR→TER |
|---|---|---|---|
| REF | 4.3 → 4.3 (no sky) | 120.9 → 120.9 | 18.8 → 18.8 |
| grade-vista | 23.1 → 23.1 (no sky in band) | 108.2 → 108.2 | 2.5 → 2.5 |
| s1-vista | **140.6 → 91.0** | **26.5 → 84.8** | 0.0 → 0.0 |

(91 is still sky-lit distant terrain — honest for a vista; it just no longer FAILs a
gate, because blue is advisory.)

---

## 5) What shipped (grade_axes.py, this commit)

- `ADVISORY = {"warmth","blue","sat"}` — measured + printed `[ADV]`, never set PASS/FAIL.
- `clip` (≤35), `dof` (hero), `micro`, `p95` — the only HARD gates.
- midtone band = terrain band (sky dropped pre-percentile); REF-invariant.
- `make_gate3_verdict_card.axes_pass` skips ADVISORY (clip carries the gameplay verdict).
- Diagnostic sheets (`_flamingo_sat_ladder_sheet`, `_poppy_sweep_report`) still print all
  four values; their legacy per-rung pass/fail tags are stale display, not the gate.

## 6) What this does NOT do

- Does not touch `client/src/look.rs` or Poppy's lane.
- Does not pick `POST_SATURATION` — that's Yamamoto. The gate now says: ship @1.02
  (clean, clip ≤22%) is PASS on every plate; @1.05 pushes hero to clip 48.8% (honest
  FAIL on clip for that one plate). Yamamoto decides; the gate no longer lies either way.
- Does not add a COLD/desaturated hard gate — none exists today (N6 is the only
  known-bad, and it's over-saturated, caught by clip). A future cold-frame known-bad can
  re-earn a chromatic gate, but only with its own calibrated threshold, never the indoor
  ref's.

## 7) Proof it runs

```
REF (hero)             => ALL AXES PASS      clip 18.8 [PASS]   warmth/blue/sat [ADV]
grade-vista @1.02 (gp) => ALL AXES PASS      clip 2.5  [PASS]
s1-vista   @1.02 (gp)  => ALL AXES PASS      clip 0.0  [PASS]   (was false-FAIL on blue)
s4-raking  @1.02 (gp)  => ALL AXES PASS      clip 3.5  [PASS]
N6 grade-vista @1.90   => FAIL               clip 99.8 [FAIL]   warmth 153 / blue 0.04 [ADV]
```

— END —
