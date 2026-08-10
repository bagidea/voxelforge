# SPEC → Rose: re-derive `blue` and `sat_honest` so they work on an outdoor frame

From: Flamingo (design/look judgement)
Date: 2026-08-09
Status: **requested by the CEO; `POST_SATURATION` is frozen at 1.02 until this lands**
Scope: `scripts/grade_axes.py` — the `blue` target, the `sat_honest` target, and the
definition of the midtone band all three chromatic axes are measured over.
Out of scope: any value in `client/src/look.rs`. Nothing in this spec asks for a look change.

Evidence for every number here: `docs/flamingo-per-plate-sat-ladder-2026-08-09.md`
(4 plates × 6 rungs, each cell measured) and `_flamingo_band_probe.md`
(reproduce with `python scripts/_flamingo_band_probe.py`).

---

## 1) Why the current gate cannot be satisfied — per plate, not "the set"

Two axes are calibrated on `docs/assets/golden-beauty-shot-ref.png`, an **indoor
kitchen** with 0.0 % sky, whose midtone band is warm lit wood:

    blue        B (mid) <= 10.0     ref 4.3      grade_axes.py:69
    sat_honest  >= 90.0             ref 95.2     grade_axes.py:71

On the four outdoor plates in the sat sweep, the only mechanism in this pipeline
that drives midtone B down to ≤10 is the swapchain clamping B to **0** — which is
precisely what the `clip <= 35` co-gate exists to count. So `blue` is reachable
only through the damage another axis is there to catch. Per plate, measured:

| plate | lowest rung where `blue <= 10` | `clip` at that rung | verdict |
|---|---|---|---|
| grade-vista | ~1.19 *(interpolated: 12.1 @1.15 → 3.1 @1.30)* | already 56.7 @1.15 | reachable only past clip FAIL |
| hero | ~1.14 *(9.7 @1.15)* | 61.7 @1.15 | reachable only past clip FAIL |
| s1-vista | **never in the sweep** — 100.0 even at 1.90 | 57.8 @1.90 | unreachable at any rung |
| s4-raking | **never in the sweep** — 15.7 at 1.90 | 54.9 @1.90 | unreachable at any rung |

And `sat_honest >= 90` is never approached anywhere `clip` is passing: at the
shipped 1.02 the four plates read **80.6 / 80.5 / 42.6 / 53.8**. It is not a
tuning gap of a few points; on two plates it is a 35–48 point gap.

The consequence the board hit directly: at 1.05, `hero` clips at 48.8 %, which
fires the co-gate, which makes `hero`'s honest sat report **N-A**. Pushing the
look to chase `blue` makes the plate the board judges on *unreadable*. That is
the whole reason 1.05 was rejected.

## 2) The band itself is measuring different content on different plates

`blue`, `warmth` and `sat` are all means over `L ∈ [p35, p75]` of the **whole
frame**. Measured sky share inside that band, rung 1.02:

| plate | sky % of frame | **sky % inside the graded band** |
|---|---|---|
| REF golden (indoor) | 0.0 | 0.0 |
| grade-vista | 0.0 | 0.0 |
| hero | 0.1 | 0.0 |
| **s1-vista** | 18.5 | **37.9** |
| s4-raking | 19.7 | 0.0 |

`s1-vista` is the case to design against: **37.9 % of its "midtone" is sky**, so it
reads `blue 140.6 / warmth 26.5` — not because the scene is cold, but because the
statistic is averaging ground with sky. Drop sky *before* taking the percentiles
and the same pixels read `blue 91.0 / warmth 84.8`. (At the retracted 1.90 build
the same plate is 34.2 % sky in band — the earlier report's "34.5 %" was that
1.90 plate, not the shipped one. Both are measured; they are different rungs.)

`s4-raking` shows the mirror trap: 19.7 % sky in frame but **0.0 %** in band,
because its sky sits above p75. So the bug is not "outdoor plates are polluted" —
it is that the same definition samples sky on one plate and not on another, and
nothing in the output says which happened. A reader cannot tell the two apart.

## 3) What I am asking for

### 3a) Redefine the band so sky cannot enter it
Requirement, not implementation: the graded band must contain **only lit scene
geometry**, and the fraction excluded must be **printed on every graded frame**.
A number that silently changes meaning per plate is worse than a missing number.

Candidate that already measures sanely (`_flamingo_band_probe.py`): classify sky
as `B > R AND L > median(L)`, drop it, then take `[p35, p75]` **of the survivors**.
Effect at 1.02 — ref unchanged (it has no sky), the two sky-free plates unchanged
to ≤0.1, only the polluted plate moves:

| plate | warmth cur → terrain | blue cur → terrain | sat cur → terrain |
|---|---|---|---|
| REF golden | 120.9 → 120.9 | 4.3 → 4.3 | 95.2 → 95.2 |
| grade-vista | 108.2 → 108.2 | 23.1 → 23.1 | 80.6 → 80.6 |
| hero | 97.4 → 97.4 | 16.4 → 16.3 | 80.5 → 80.5 |
| s1-vista | 26.5 → **84.8** | 140.6 → **91.0** | 42.6 → 49.2 |
| s4-raking | 72.0 → 71.4 | 66.9 → **48.6** | 53.8 → 61.6 |

That mask is crude (a blue-lit water surface or a cyan roof would be eaten by it),
and I am not asking you to ship mine — I am asking that whatever mask ships be
**stated, reported per frame, and validated on a plate that has no sky at all** so
we can prove it is a no-op there. Note the mask leaves `s1-vista` at blue 91.0:
excluding sky does **not** rescue `blue <= 10`. It removes a masking bug; it does
not make an indoor target reachable outdoors. Those are two separate fixes.

### 3b) Re-derive `blue` and `sat_honest` against outdoor content
An outdoor midtone is grass, limestone and sky-lit ground. Sky-lit ground has real
blue in it — that is the aerial-perspective cue the Look Bible asks for elsewhere.
`blue <= 10` asks the game to delete it. What I need from you is a target derived
from content of the same class as the plates, with the derivation written down.

If a defensible outdoor target does not exist yet, my preference is explicit
**`blue`/`sat_honest` = ungated (reported, not graded) on outdoor profiles** over a
threshold nobody can satisfy — a gate that is only reachable through damage teaches
the team to route around gates, which is the failure mode we already paid for once
with the legacy clip-signature sat.

### 3c) Acceptance criteria for the re-derived gate
A candidate is acceptable when all five hold, each demonstrated on real plates:

1. **REF still self-grades 6/6** — `golden-beauty-shot-ref.png` must not regress.
2. **The known-bad build still FAILs** — every 1.90 plate in
   `_yama_sat_sweep_190/after/` must FAIL. A gate that stops catching N6 is not a fix.
3. **No axis is reachable only through another axis's failure.** For each graded
   axis, show one plate that passes it *while* `clip <= 35`. This is the exact
   property today's `blue` violates, and it should be a test, not a review habit.
4. **Every graded plate yields a readable number at the shipped 1.02** — no N-A on
   the four sweep plates at the value we ship.
5. **The excluded fraction is printed** (sky %, railed %) on every graded frame, so
   "which content was measured" is never inferred.

### 3d) The outdoor reference I need, to replace the indoor kitchen
One indoor 1024² kitchen cannot calibrate warmth/blue/sat for outdoor gameplay.
Minimum viable set — **three** approved outdoor references, because one plate is
how we got here:

| ref | why | must contain |
|---|---|---|
| **open vista, sky in frame** | calibrates a band definition that has to survive sky | ≥15 % sky, lit ground, a cast shadow |
| **raking / golden-hour ground**, little sky | calibrates warmth without sky in the band | large sunlit + shadowed ground of the same material |
| **enclosed / canyon**, no sky | the control — proves the new mask is a no-op where there is nothing to mask | 0 % sky |

Each must be CEO-approved as a look target (the same status the golden ref has),
captured through `voxelforge_shot.exe` at 1024² so it is HUD-free by construction,
and committed with provenance in `BEAUTY-SHOT-PROVENANCE.md`. Until then, keep
grading ref-relative against the indoor plate for `warmth`, `clip`, `micro`, `p95`
— those four are content-robust enough to have caught real defects — and stop
grading `blue`/`sat_honest` outdoors rather than grading them wrong.

## 4) What I am NOT claiming

- Not that the look is finished. Three plates fail `warmth` at 1.02, and `hero`
  fails `p95` on every rung of the ladder. Those are real, and they belong to the
  exposure/ambient ticket (`docs/ticket-ambient-exposure-shadow-lift-2026-08-09.md`),
  not to this one.
- Not that `blue <= 10` is wrong as physics. It is wrong *as a target for this
  content*, measured.
- Not that my sky mask is the right mask. It is the cheapest thing that reproduces
  the bug — evidence, not a design.

— Flamingo
