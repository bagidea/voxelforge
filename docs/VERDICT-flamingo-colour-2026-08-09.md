# VERDICT — colour/gamut judgement (Flamingo, 2026-08-09, corrected)

Role: judge by eye + measured pixels. No source files edited.
Supersedes my earlier N6 verdict. Six corrections are marked **[FIX]**.

---

## CORRECTION 2 — 2026-08-09, after board review. Read this before §1 or §2.

**My 1.05 recommendation is withdrawn. The position is: hold at 1.02.**

Two things were wrong with the report below and both are my error:

1. **The ladder table in §1 is `grade-vista-nohud2.png` alone.** I presented it as
   the sweep's verdict. The CEO re-ran `hero` at 1.05 and got warmth **100.9 FAIL**,
   clip **48.8 FAIL**, honest sat **N-A** — against `hero` at 1.02: clip **22.1 PASS**,
   honest sat **80.5, readable**. I have since re-measured all 24 plates and both of
   his numbers reproduce exactly. 1.05 does not "pass both satisfiable gates"; it
   passes them **on one plate of four**, and it breaks the plate the board judges on.
2. **1.05 makes `hero` unmeasurable.** Once clip crosses 35 the co-gate fires and
   honest sat reports N-A by design. Trading a readable 80.5 for an N-A is going
   backwards, whatever warmth does on another plate.

Measured answer to "is there a saturation where **all four** plates pass clip ≤ 35":
**yes — 1.00 and 1.02, and nothing above.** 1.02 is the highest such value and it is
what ships today at `client/src/look.rs:578`. There is **no** rung where all four
plates pass warmth ≥ 110 (`s1-vista` peaks at 76.6 even at the retracted 1.90),
because 37.9 % of its graded band is sky — a band-definition bug, not a look value.

Replacement artefacts, per-plate, nothing collapsed:

- **Per-plate ladder, 4 plates × 6 rungs × 6 axes:** `docs/flamingo-per-plate-sat-ladder-2026-08-09.md`
  (regenerate: `python scripts/_flamingo_per_plate_ladder.py`; raw `_flamingo_per_plate_ladder.json`)
- **Evidence sheet, 1.02 vs 1.05 on all four plates:** `docs/assets/flamingo-perplate-102-vs-105-2026-08-09.png`
- **Gate spec for Rose:** `docs/spec-rose-chromatic-regate-2026-08-09.md`
- **Ambient/exposure ticket, split out of §3:** `docs/ticket-ambient-exposure-shadow-lift-2026-08-09.md`

Standing instruction until Rose's re-derive lands: **`POST_SATURATION` stays 1.02.
Nobody touches it**, including to "just try 1.03".

### ⚠ The 1.02 we are holding is NOT committed
Found while verifying the constant:

    git show HEAD:client/src/look.rs  →  POST_SATURATION = 1.90   (the retracted N6 value)
    working tree                      →  POST_SATURATION = 1.02   (uncommitted, 42 lines with its doc comment)

So "hold at 1.02" is currently true only in one uncommitted working tree. A clean
checkout, a `git checkout -- client/src/look.rs`, or a build on another machine gets
**1.90** — the value this verdict retracted, the one that clips 99.6–99.8 % of the
midtone band. I have not committed it: `look.rs` is Yamamoto's lane and the CEO's
instruction this round was to release nothing. **Yamamoto should land the 1.90 → 1.02
retraction as its own commit** so the frozen value survives a checkout. Until then,
treat any measurement from a fresh clone as N6-era, not as this ladder.

---

## 0) Measurement space — stated up front **[FIX]**

Every number below is in ONE of two spaces. They are NOT interchangeable and my
previous report mixed them without saying so.

| space | definition | who uses it |
|---|---|---|
| **GRADE** | 1024² LANCZOS, **whole frame, no sky mask**, midtone band = L∈[p35,p75] | `scripts/grade_axes.py` — the ruler that decides PASS/FAIL |
| **TERRAIN** | same, but sky dropped (`B>R` AND `L>median`) | diagnosis only |

Rule for Yamamoto: **tune against GRADE.** TERRAIN numbers are for understanding
*why*, never for claiming a pass. The golden ref is indoor and has 0.0 % sky, so
its GRADE and TERRAIN numbers are identical — ref-relative targets are safe in
either space; plate numbers are not.

Probe used (reproducible): `_flamingo_space_probe.py`

---

## 1) The headline: two live gates are now mutually unsatisfiable **[FIX]**

My earlier target "midtone B 10–22" was wrong and I withdraw it. `scripts/grade_axes.py:69`
carries a live `blue B (mid) <= 10.0`; B=15–22 FAILs it today. But lowering my
number does not fix the problem either — **there is no POST_SATURATION value that
satisfies `blue ≤ 10` and `clip ≤ 35` together on this content.**

Sat ladder — **⚠ ONE PLATE ONLY: `grade-vista-nohud2.png`**, GRADE space, both axes
monotone. This table is *not* the sweep's verdict; presenting it as one was the error
CORRECTION 2 retracts. For all four plates see
`docs/flamingo-per-plate-sat-ladder-2026-08-09.md`:

| POST_SATURATION | warmth R−B (≥110) | blue B (≤10) | mid clip % (≤35) | honest sat (≥90) |
|---|---|---|---|---|
| 1.00 | 103.6 FAIL | 27.5 FAIL | **0.01 PASS** | 77.9 FAIL |
| 1.02 *(shipped)* | 108.2 FAIL | 23.1 FAIL | **2.46 PASS** | 80.6 FAIL |
| **1.05** | **114.2 PASS** | 17.8 FAIL | **31.7 PASS** | 77.9 FAIL |
| 1.15 | 123.9 PASS | 12.1 FAIL | 56.7 FAIL | N-A |
| 1.30 | 136.9 PASS | **3.1 PASS** | 66.9 FAIL | N-A |
| 1.90 *(N6)* | 152.9 PASS | **0.04 PASS** | 99.8 FAIL | N-A |
| REF | 120.9 | 4.3 | 18.8 | 95.2 |

Proof it is not a tuning gap but an exclusion: clip is **100 % blue-driven** —
at every rung the railed fraction equals the `B==0` fraction to 2 d.p.
(R==0 stays 0.00 % until 1.90; nothing hits 255 until 1.90 at 0.80 %). So in this
pipeline the *only* mechanism that drives midtone B down to ≤10 is the swapchain
clamping B to 0 — which is exactly what the clip co-gate counts. `blue ≤ 10`
⟹ B==0 on ≥57 % of the band ⟹ `clip` FAIL. Interpolating the two monotone
curves: blue hits 10 at sat ≈1.19; clip is already 56.7 at 1.15.

Same for `honest sat ≥ 90`: it never exceeds ~81 anywhere clip is passing.

**Verdict: `blue` (and `sat_honest`) must be re-derived — Rose's lane, not
Yamamoto's.** They were calibrated on `golden-beauty-shot-ref.png`, an indoor
kitchen whose midtone band is warm wood with intrinsically near-zero blue.
Applied to outdoor plates whose midtone is grass/limestone/sky-lit ground, they
are only reachable by the damage the clip gate exists to catch.

### Second gate-design bug: sky inside the graded band
`s1-vista` is **34.5 %** sky inside its L[p35,p75] band (measured; every other
plate is ≤0.1 %). That is why it reads `blue 101.0 / warmth 75.5 FAIL` in GRADE
while TERRAIN gives `B 20.9 / R−B +175.9`. The plate is not less warm — half its
"midtone" is sky. No look change can fix that; it is a masking bug.

---

## 2) What Yamamoto should actually ship

**WITHDRAWN — see CORRECTION 2. The recommendation is `POST_SATURATION` stays 1.02.**
Struck text kept below so the reasoning that failed is on the record; every claim in
it is scoped to `grade-vista` and does not hold on `hero`.

~~**Recommended: `POST_SATURATION = 1.05`**~~ (currently `client/src/look.rs:578` = 1.02).

- ~~It is the *only* rung that passes both live, satisfiable chromatic gates
  (warmth 114.2, clip 31.7).~~ **True on `grade-vista` only.** On `hero` the same
  rung is warmth 100.9 FAIL / clip 48.8 FAIL / sat N-A. Shipped 1.02 misses warmth by
  1.8 *on that one plate*; it misses by 12.6 on `hero` and by 81.7 on `s1-vista`, so
  warmth was never 1.8 away from being solved.
- By eye on the ladder sheet: 1.00/1.05 keep material identity — limestone reads
  as stone, grass as grass, dirt as dirt. At 1.30 grass starts turning
  yellow-orange; at 1.90 every material collapses into one orange and shadows
  glow red. 1.05 is the last rung with material separation intact.
- Buy the remaining warmth from **light colour** (`TEMPERATURE`, ambient/sky
  gain), not from POST_SATURATION — saturation pushes R *down* on the
  green-dominant 60 % of the vista band (already documented at `look.rs:530`).

### Targets, in GRADE space, ref-relative
| axis | target | ref | note |
|---|---|---|---|
| warmth R−B (mid) | ≥ +110 | 120.9 | live gate, satisfiable |
| mid clip % | ≤ 35 | 18.8 | live gate, satisfiable; keep B off the 0 rail |
| blue B (mid) | **hold, do not chase** | 4.3 | blocked on Rose's re-derive |
| sat honest | **hold, do not chase** | 95.2 | same |

### Blue-ramp health (TERRAIN space, ref is space-invariant) **[FIX — statistic named]**
Two different statistics; my earlier "p85–95" row quoted neither correctly.

| statistic (explicit) | REF | grade-vista @1.05 | N6 @1.90 |
|---|---|---|---|
| band mean over L∈[p85,p95] → B / (B÷R) | 20.2 / **9.0 %** | — | 0.0 / 0.0 % |
| top-5 % mean (L ≥ p95) → B / (B÷R) | 96.0 / **37.9 %** | — / **53.9 %** | 0.9 / 0.4 % |
| corr(B, L) whole frame | 0.795 | **0.824** | 0.327 |

My earlier "ref = B 45.4 / B/R 19.4 % at p85–95" was not reproducible under that
label and is withdrawn. Use the two rows above, each with its statistic named.

**This axis is already fixed.** At 1.02–1.05 the blue ramp is alive and slightly
*richer* than ref (corr 0.82 vs 0.795; top-5 % B/R 53.9 % vs 37.9 %). "Highlights
don't bleach toward light" was a symptom of sat 1.90 only. No work needed here.

---

## 3) The N6 defect list, re-scored against the 1.05 plate

| # | earlier claim | status now |
|---|---|---|
| 1 | highlights don't cool toward light | **FIXED** by the 1.90→1.02 retraction (corr 0.824, top-5 % B/R 53.9 %) |
| 2 | every material saturates together | **FIXED** at ≤1.05 by eye; returns at 1.30 |
| 3 | shadows are red (145,0,0) instead of `#2A2030` | **HALF WITHDRAWN [FIX]** — see below |
| 4 | flat range, shadow floor lifted | **STILL OPEN**, and sat-independent |
| 5 | sky and ground on different clocks | still open (unmeasured — art call) |
| 6 | no contact shadow | still open |
| 7 | no aerial perspective | still open |

**On #3 [FIX]:** the golden ref's own darkest decile is `(45.3, 16.9, 0.8)` with
B==0 on 55.9 % of it — ref shadows are *also* red-leaning and blue-clipped. The
`#2A2030` (42,32,48) written at `docs/look-bible.md:130` matches neither the ref
nor any plate. So "wrong hue vs ref" was my error. What survives is a **level**
defect, not a hue defect:

| plate | shadow decile RGB | vs ref |
|---|---|---|
| REF | (45.3, 16.9, 0.8) | — |
| grade-vista @1.05 | (91.8, 47.3, 7.0) | R lifted 2.0× |
| grade-vista @1.90 | (119.0, 29.4, 0.0) | R lifted 2.6× |
| s4-raking @1.05 | (102.4, 64.8, 12.1) | R lifted 2.3× |

Shadow R barely moves across sat 1.00→1.05 (90.4→91.8), and frame Lstd is pinned
at 37.1 on every rung (ref 46.6). **Items 3-as-level and 4 are one bug and it is
not in POST_SATURATION** — it is ambient floor / exposure. Separate ticket.

---

## 4) "B is clipped on 98–99.7 % of pixels" — corrected **[FIX]**

That was a 6-of-8 number reported as 8-of-8. Whole-frame `B==0`, GRADE resample:

| plate | B==0 (frame) | B==0 (midtone band) |
|---|---|---|
| gate3-boot | 99.4 % | 99.8 % |
| gate3-combat | 98.7 % | 99.8 % |
| gate3-walk | 97.9 % | 99.7 % |
| grade-vista | 98.8 % | 99.8 % |
| hero | 99.6 % | 99.6 % |
| s3-clash | 98.4 % | 99.5 % |
| **s1-vista** | **50.0 %** | **38.3 %** |
| **s4-raking** | **58.0 %** | **54.9 %** |
| REF | 25.9 % | 18.8 % |

Correct statement: *six* N6 plates railed at 97.9–99.6 % frame-wide; the two
sky-bearing plates railed at 50–58 %. Still all far past ref — the conclusion
holds, the "whole set" phrasing did not.

---

## 5) Files to open

- Ladder sheet (ref + 4 rungs, captioned): `docs/assets/flamingo-sat-ladder-2026-08-09.png`
- Golden ref: `docs/assets/golden-beauty-shot-ref.png`
- N6 shotset (all 8, the 1.90 build):
  - `_pixel_shotset_N6/after/gate3-boot-nohud2.png`
  - `_pixel_shotset_N6/after/gate3-combat-nohud2.png`
  - `_pixel_shotset_N6/after/gate3-walk-nohud2.png`
  - `_pixel_shotset_N6/after/grade-vista-nohud2.png`
  - `_pixel_shotset_N6/after/hero-nohud2.png`
  - `_pixel_shotset_N6/after/s1-vista-nohud2.png`
  - `_pixel_shotset_N6/after/s3-clash-nohud2.png`
  - `_pixel_shotset_N6/after/s4-raking-nohud2.png`
- Sat sweep (4 plates each, real runs — `extra_env` set, `map_drift: false`, `paired: true`):
  `_yama_sat_sweep_{100,102,105,115,130,190}/after/`
- Ruler: `scripts/grade_axes.py` (TARGETS at line 64; `blue` at line 69)
- Constant under discussion: `client/src/look.rs:578`
- Probe: `_flamingo_space_probe.py`

**Correction to my earlier report:** I wrote that the after plates had not
arrived, citing the empty `_yama_gamut_probe_dry/after/`. That directory is the
dry run and is still empty — but the real sweep landed in six separate
`_yama_sat_sweep_*` directories, 24 plates, and I missed them. Everything in
sections 1–3 above is measured on those.

---

## 6) Hand-off

- **Yamamoto** — ~~`POST_SATURATION` 1.02 → 1.05~~ **HOLD at 1.02, do not touch it.**
  (CORRECTION 2: 1.05 fails clip on `hero` at 48.8 % and makes its honest sat
  unreadable. 1.00 and 1.02 are the only rungs where all four plates pass clip.)
  Warmth is bought from exposure/light colour — see the ambient/exposure ticket — not
  from sat. Do not chase `blue` or `sat_honest`; they are blocked on Rose.
- **Rose** — two gate bugs, both proven: `blue ≤10` / `sat_honest ≥90` are only
  reachable through the damage `clip` catches; and `s1-vista` is graded with sky
  inside its midtone band — **37.9 %** at the shipped 1.02, 34.2 % at the 1.90 plate
  (the "34.5 %" I quoted earlier was the 1.90 plate, not the shipped one).
  Full spec, acceptance criteria and the outdoor-ref request:
  `docs/spec-rose-chromatic-regate-2026-08-09.md`.
- **Separate ticket, now filed** — shadow floor lifted 2.0–2.5× vs ref on all four
  plates and frame Lstd 20–52 % short, with `hero` p95 110.7 failing on every rung.
  Ambient/exposure, not the colour lens:
  `docs/ticket-ambient-exposure-shadow-lift-2026-08-09.md`.

— END —
