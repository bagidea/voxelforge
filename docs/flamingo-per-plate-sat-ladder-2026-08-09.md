# PER-PLATE sat ladder — 4 plates x 6 rungs, every cell measured

Source: `_yama_sat_sweep_{100,102,105,115,130,190}/after/*-nohud2.png` (24 plates).
Ruler: `scripts/grade_axes.py`, profile **gameplay** (6 axes; DOF excluded there by
design — framing-dependent). Every table below is ONE named plate. No plate's
number is quoted for another plate, and nothing is averaged.

Gate: warmth >= 110 · blue <= 10 · clip <= 35 · sat(honest) >= 90 · micro >= 5 · p95 150..185
REF (golden, per grade_axes TARGETS): warmth 120.9 · blue 4.3 · clip 18.8 · sat 95.2 · micro 5.24 · p95 165.8

## plate: `grade-vista-nohud2.png`

| POST_SATURATION | warmth R-B (mid) | blue B (mid) | mid clip % | sat (honest) | micro-contrast | p95 | axes passed |
|---|---|---|---|---|---|---|---|
| 1.00 | 103.6 FAIL | 27.5 FAIL | 0.0 PASS | 77.9 FAIL | 7.0 PASS | 169.5 PASS | **3/6** |
| 1.02 *(shipped)* | 108.2 FAIL | 23.1 FAIL | 2.5 PASS | 80.6 FAIL | 7.0 PASS | 169.4 PASS | **3/6** |
| 1.05 | 114.2 PASS | 17.8 FAIL | 31.7 PASS | 77.9 FAIL | 7.0 PASS | 169.2 PASS | **4/6** |
| 1.15 | 123.9 PASS | 12.1 FAIL | 56.7 FAIL | N-A *(unreadable, clip>35)* | 6.9 PASS | 168.6 PASS | **3/5** |
| 1.30 | 136.9 PASS | 3.1 PASS | 66.9 FAIL | N-A *(unreadable, clip>35)* | 6.8 PASS | 167.6 PASS | **4/5** |
| 1.90 *(N6, retracted)* | 152.9 PASS | 0.0 PASS | 99.8 FAIL | N-A *(unreadable, clip>35)* | 6.8 PASS | 161.5 PASS | **4/5** |

## plate: `hero-nohud2.png`

| POST_SATURATION | warmth R-B (mid) | blue B (mid) | mid clip % | sat (honest) | micro-contrast | p95 | axes passed |
|---|---|---|---|---|---|---|---|
| 1.00 | 93.3 FAIL | 20.0 FAIL | 0.1 PASS | 81.7 FAIL | 5.1 PASS | 110.9 FAIL | **2/6** |
| 1.02 *(shipped)* | 97.4 FAIL | 16.4 FAIL | 22.1 PASS | 80.5 FAIL | 5.1 PASS | 110.7 FAIL | **2/6** |
| 1.05 | 100.9 FAIL | 13.5 FAIL | 48.8 FAIL | N-A *(unreadable, clip>35)* | 5.0 PASS | 110.2 FAIL | **1/5** |
| 1.15 | 107.2 FAIL | 9.7 PASS | 61.7 FAIL | N-A *(unreadable, clip>35)* | 5.0 FAIL | 109.8 FAIL | **1/5** |
| 1.30 | 118.2 PASS | 2.2 PASS | 69.1 FAIL | N-A *(unreadable, clip>35)* | 4.9 FAIL | 109.2 FAIL | **2/5** |
| 1.90 *(N6, retracted)* | 132.6 PASS | 0.1 PASS | 99.6 FAIL | N-A *(unreadable, clip>35)* | 4.9 FAIL | 105.9 FAIL | **2/5** |

## plate: `s1-vista-nohud2.png`

| POST_SATURATION | warmth R-B (mid) | blue B (mid) | mid clip % | sat (honest) | micro-contrast | p95 | axes passed |
|---|---|---|---|---|---|---|---|
| 1.00 | 28.5 FAIL | 138.8 FAIL | 0.0 PASS | 42.1 FAIL | 8.2 PASS | 174.0 PASS | **3/6** |
| 1.02 *(shipped)* | 26.5 FAIL | 140.6 FAIL | 0.0 PASS | 42.6 FAIL | 8.1 PASS | 174.1 PASS | **3/6** |
| 1.05 | 29.5 FAIL | 138.1 FAIL | 0.1 PASS | 44.1 FAIL | 8.2 PASS | 173.8 PASS | **3/6** |
| 1.15 | 28.0 FAIL | 138.8 FAIL | 3.9 PASS | 46.3 FAIL | 8.0 PASS | 173.3 PASS | **3/6** |
| 1.30 | 40.4 FAIL | 128.5 FAIL | 11.3 PASS | 49.5 FAIL | 8.0 PASS | 172.8 PASS | **3/6** |
| 1.90 *(N6, retracted)* | 76.6 FAIL | 100.0 FAIL | 57.8 FAIL | N-A *(unreadable, clip>35)* | 7.5 PASS | 168.5 PASS | **2/5** |

## plate: `s4-raking-nohud2.png`

| POST_SATURATION | warmth R-B (mid) | blue B (mid) | mid clip % | sat (honest) | micro-contrast | p95 | axes passed |
|---|---|---|---|---|---|---|---|
| 1.00 | 69.9 FAIL | 68.4 FAIL | 0.0 PASS | 53.1 FAIL | 5.4 PASS | 163.3 PASS | **3/6** |
| 1.02 *(shipped)* | 72.0 FAIL | 66.9 FAIL | 1.5 PASS | 53.8 FAIL | 5.4 PASS | 163.1 PASS | **3/6** |
| 1.05 | 74.9 FAIL | 64.6 FAIL | 5.1 PASS | 54.2 FAIL | 5.3 PASS | 163.2 PASS | **3/6** |
| 1.15 | 84.0 FAIL | 57.3 FAIL | 13.9 PASS | 56.8 FAIL | 5.3 PASS | 163.1 PASS | **3/6** |
| 1.30 | 96.2 FAIL | 47.4 FAIL | 25.5 PASS | 60.5 FAIL | 5.2 PASS | 162.9 PASS | **3/6** |
| 1.90 *(N6, retracted)* | 135.6 PASS | 15.7 FAIL | 54.9 FAIL | N-A *(unreadable, clip>35)* | 5.3 PASS | 160.9 PASS | **3/5** |

## Cross-plate clip matrix — the co-gate, all 4 plates side by side

| POST_SATURATION | grade-vista | hero | s1-vista | s4-raking | all 4 <= 35 ? |
|---|---|---|---|---|---|
| 1.00 | 0.0 PASS | 0.1 PASS | 0.0 PASS | 0.0 PASS | **YES** |
| 1.02 | 2.5 PASS | 22.1 PASS | 0.0 PASS | 1.5 PASS | **YES** |
| 1.05 | 31.7 PASS | 48.8 FAIL | 0.1 PASS | 5.1 PASS | no |
| 1.15 | 56.7 FAIL | 61.7 FAIL | 3.9 PASS | 13.9 PASS | no |
| 1.30 | 66.9 FAIL | 69.1 FAIL | 11.3 PASS | 25.5 PASS | no |
| 1.90 | 99.8 FAIL | 99.6 FAIL | 57.8 FAIL | 54.9 FAIL | no |

## Cross-plate warmth matrix (the other satisfiable axis)

| POST_SATURATION | grade-vista | hero | s1-vista | s4-raking | all 4 >= 110 ? |
|---|---|---|---|---|---|
| 1.00 | 103.6 FAIL | 93.3 FAIL | 28.5 FAIL | 69.9 FAIL | no |
| 1.02 | 108.2 FAIL | 97.4 FAIL | 26.5 FAIL | 72.0 FAIL | no |
| 1.05 | 114.2 PASS | 100.9 FAIL | 29.5 FAIL | 74.9 FAIL | no |
| 1.15 | 123.9 PASS | 107.2 FAIL | 28.0 FAIL | 84.0 FAIL | no |
| 1.30 | 136.9 PASS | 118.2 PASS | 40.4 FAIL | 96.2 FAIL | no |
| 1.90 | 152.9 PASS | 132.6 PASS | 76.6 FAIL | 135.6 PASS | no |

## Both satisfiable axes, all four plates at once

| POST_SATURATION | plates passing clip<=35 | plates passing warmth>=110 | plates passing BOTH |
|---|---|---|---|
| 1.00 | 4/4 grade-vista· hero· s1-vista· s4-raking | 0/4 — | **0/4** — |
| 1.02 | 4/4 grade-vista· hero· s1-vista· s4-raking | 0/4 — | **0/4** — |
| 1.05 | 3/4 grade-vista· s1-vista· s4-raking | 1/4 grade-vista | **1/4** grade-vista |
| 1.15 | 2/4 s1-vista· s4-raking | 1/4 grade-vista | **0/4** — |
| 1.30 | 2/4 s1-vista· s4-raking | 2/4 grade-vista· hero | **0/4** — |
| 1.90 | 0/4 — | 3/4 grade-vista· hero· s4-raking | **0/4** — |

## Shadow floor + Lstd across the ladder (NOT sat axes — ambient/exposure ticket)

Darkest-decile mean RGB and whole-frame Lstd, same 1024 geometry. REF golden = shadow (45.3, 16.9, 0.8), Lstd 46.6.

| plate | 1.00 shadowR / Lstd | 1.02 shadowR / Lstd | 1.05 shadowR / Lstd | 1.15 shadowR / Lstd | 1.30 shadowR / Lstd | 1.90 shadowR / Lstd |
|---|---|---|---|---|---|---|
| grade-vista | 90.4 / 37.1 | 90.9 / 37.1 | 91.8 / 37.1 | 94.5 / 37.0 | 98.5 / 36.9 | 119.0 / 36.6 |
| hero | 107.1 / 22.0 | 108.5 / 22.2 | 110.8 / 22.3 | 118.7 / 22.6 | 131.4 / 22.8 | 150.3 / 22.7 |
| s1-vista | 111.1 / 30.4 | 111.9 / 30.3 | 112.2 / 30.5 | 114.1 / 30.6 | 118.7 / 30.6 | 133.7 / 30.1 |
| s4-raking | 100.8 / 31.4 | 101.6 / 31.4 | 102.4 / 31.5 | 104.9 / 31.6 | 108.5 / 31.6 | 125.1 / 31.5 |

## ANSWER — is there a POST_SATURATION where ALL FOUR plates pass clip <= 35?

**Yes, and only these: 1.00, 1.02.** The highest is **1.02** — the value shipped today at `client/src/look.rs:578`. Every rung at 1.05 and above loses at least one plate (`hero` first, at 48.8 %).

Rungs where the honest-sat axis is still READABLE on all four plates (i.e. no plate has gone N-A): **1.00, 1.02**. Above that, the plate the board judges on stops producing a number at all.

And the question nobody asked but which decides the lane: **there is no rung where all four plates pass warmth >= 110** — `s1-vista` peaks at 76.6 even at the retracted 1.90, because 37.9 % of its graded band is sky. That is a band-definition bug, not a look value to tune. See the spec handed to Rose.

