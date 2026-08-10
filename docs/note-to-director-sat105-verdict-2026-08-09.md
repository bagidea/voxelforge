# Yamamoto — verdict on POST_SATURATION 1.02 → 1.05, all named plates (2026-08-09)

Director's ask: don't accept Flamingo's 1.05 recommendation on the strength of
`grade-vista-nohud2.png` alone — set it, run `scripts/grade_axes.py` on every
named plate, report plate × axis, then decide. Judging only on the two live,
satisfiable gates per the verdict doc: **warmth R−B (mid) ≥110** and **mid
clip % ≤35**. `blue`/`sat_honest` are excluded — Rose is re-deriving those
gates; they are not reachable on this content without the damage `clip` exists
to catch (see `docs/VERDICT-flamingo-colour-2026-08-09.md`).

**Method.** No source edit — the constant was swept live via
`VOXELFORGE_LOOK_GRADE=0.05,{1.02|1.05},1.12,0.86` on the same exe (commit
`3511582`, sha256 `b024bbc2...`, verified no client/ diff to current HEAD
`698b0f0`), the identical mechanism the 1.02 default itself was tuned with
(`client/src/look.rs:562`, "no rebuild per rung"). All 8 named plates from
`_pixel_shotset_N6/manifest.json` (argv/cine/env copied verbatim), graded
`--profile gameplay`, `--profile hero`/DOF excluded (framing-dependent, not
relevant here). 4 plates (`grade-vista`, `hero`, `s1-vista`, `s4-raking`) were
real renders already on disk in `_yama_sat_sweep_{102,105}/after/`; the other
4 (`gate3-boot`, `gate3-combat`, `gate3-walk`, `s3-clash`) were re-shot fresh
for this check in `_yama_sat_sweep_{102,105}_extra/`.

## Result — warmth / clip, 1.02 (shipped) vs 1.05 (proposed)

| plate | warmth @1.02 | warmth @1.05 | clip% @1.02 | clip% @1.05 |
|---|---|---|---|---|
| grade-vista | 108.24 FAIL | **114.19 PASS** | 2.46 PASS | 31.72 PASS |
| hero | 97.41 FAIL | 100.90 FAIL | 22.07 PASS | **48.80 FAIL** |
| s1-vista* | 26.46 FAIL | 29.48 FAIL | 0.00 PASS | 0.14 PASS |
| s4-raking | 71.97 FAIL | 74.87 FAIL | 1.54 PASS | 5.09 PASS |
| gate3-boot | 97.31 FAIL | 101.62 FAIL | 5.85 PASS | **40.41 FAIL** |
| gate3-combat | 79.80 FAIL | 100.03 FAIL | 1.03 PASS | 29.59 PASS |
| gate3-walk | 87.56 FAIL | 91.85 FAIL | 0.36 PASS | 4.78 PASS |
| s3-clash | 102.64 FAIL | 105.92 FAIL | 11.52 PASS | **53.09 FAIL** |

\* s1-vista's warmth is additionally corrupted by the known sky-in-midtone-band
gate bug Flamingo flagged (34.5% sky inside the graded band) — its number is
suspect at both rungs, not a sat effect either way.

## Verdict: hold at 1.02. Do not ship 1.05.

The 1.05 rung only fixes warmth on the one plate it was measured on
(`grade-vista`). The other 6 ungated-by-sky plates stay warmth-FAIL at 1.05
too — the sat bump is not buying warmth broadly, consistent with `look.rs`'s
own standing note that saturation pushes R **down** on green-dominant content
and is "the wrong knob for warmth" (`look.rs:530`).

Worse, it breaks the co-gate that was previously clean everywhere: **clip
jumps past the 35% cap on 3 of 8 plates** — `hero` 22.1%→48.8%, `gate3-boot`
5.9%→40.4%, `s3-clash` 11.5%→53.1%. This is gamut-clip damage (blue channel
railed to 0 on nearly half the midtone band), the exact defect the 1.90→1.02
retraction existed to fix — 1.05 reopens it on almost half the shotset.
`hero` was already flagged as "the resistant plate, knife-edge between 1.00
and 1.05" in the standing code comment; this re-measurement (fresh exe, fresh
renders) confirms it, and shows two more plates share the same cliff.

**Net: 1.05 buys +1 plate on warmth, costs 3 plates on clip. Staying at 1.02.**
The remaining warmth gap (every plate still FAILs ≥110 at 1.02) has to come
from light colour (`TEMPERATURE`, ambient/sky gain) per Flamingo's own
hand-off note, not from another saturation push — this data is further proof
of that, not just Flamingo's.

## Note, unrelated to this check
`client/src/look.rs`'s 1.90→1.02 gamut-clip retraction (the baseline this
whole comparison used) is itself **still uncommitted** in the working tree —
HEAD (`698b0f0`) still ships 1.90. Flagging since every "@1.02" number above
is measured against an uncommitted state, not what's actually on `main`.

— Yamamoto
