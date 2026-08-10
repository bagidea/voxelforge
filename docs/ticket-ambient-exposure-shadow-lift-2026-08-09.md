# TICKET — lifted shadow floor + collapsed range. Ambient/exposure, NOT the colour lens

Opened: 2026-08-09 by Flamingo
Owner: **pixel** (look/rendering) with **Yamamoto** for `client/src/look.rs` constants
Explicitly NOT: `grade::POST_SATURATION`. Split out of the 2026-08-09 colour verdict
at the CEO's instruction so a saturation decision and an exposure decision stop
sharing one thread.

Evidence: `docs/flamingo-per-plate-sat-ladder-2026-08-09.md` (4 plates × 6 rungs,
every cell measured on the 1024² LANCZOS geometry `scripts/grade_axes.py` uses).
Raw: `_flamingo_per_plate_ladder.json`.

---

## 1) The defect, in two numbers per plate

Golden ref darkest decile = **(45.3, 16.9, 0.8)**, frame `Lstd` = **46.6**.
At the shipped `POST_SATURATION` 1.02:

| plate | shadow decile R | vs ref | frame Lstd | vs ref | p95 (target 150–185) |
|---|---|---|---|---|---|
| grade-vista | 90.9 | **2.0×** | 37.1 | −20 % | 169.4 PASS |
| hero | 108.5 | **2.4×** | 22.2 | **−52 %** | **110.7 FAIL** |
| s1-vista | 111.9 | **2.5×** | 30.3 | −35 % | 174.1 PASS |
| s4-raking | 101.6 | **2.2×** | 31.4 | −33 % | 163.1 PASS |

Two symptoms of one thing: the darks are lifted about 2–2.5× and the frame's
luminance spread is 20–52 % short of ref. `hero` is the acute case — its darks are
the most lifted *and* its highlights never reach the p95 floor, i.e. the whole
frame is squeezed into the middle. That is a range problem, not a hue problem.

## 2) Proof it is not the colour lens

Shadow-decile R across the full sat ladder (same plate, only `POST_SATURATION`
moving):

| plate | 1.00 | 1.02 | 1.05 | 1.15 | 1.30 | 1.90 |
|---|---|---|---|---|---|---|
| grade-vista | 90.4 | 90.9 | 91.8 | 94.5 | 98.5 | 119.0 |
| hero | 107.1 | 108.5 | 110.8 | 118.7 | 131.4 | 150.3 |
| s1-vista | 111.1 | 111.9 | 112.2 | 114.1 | 118.7 | 133.7 |
| s4-raking | 100.8 | 101.6 | 102.4 | 104.9 | 108.5 | 125.1 |

Honest reading: saturation is **not** perfectly invariant here — it does lift the
shadow decile, by +32 % at the retracted 1.90. But across the range actually in
play (1.00 → 1.05) it moves the number **+1.6 %** while the gap to ref is **+100 %**,
and the ~2× lift is already fully present at 1.00. So sat cannot be the cause, and
turning sat down to 1.00 does not fix it. `Lstd` is flatter still — 37.1 → 37.1 on
grade-vista, 22.0 → 22.3 on hero across 1.00–1.05.

## 3) Where the lever actually is (from the code's own measured notes)

`client/src/look.rs` DAY: `illuminance: 22_000`, `ambient_lux: 2200`, `ev100: 10.3`.

- `look.rs:692-712` documents the geometry: a horizontal ground plane receives
  `illuminance * sin(elev)` from the key, while open shade receives only the flat
  `ambient_lux`, which no shadow map can attenuate. Computed on the **shipped**
  constants above — `elev_deg: 22.0` (`look.rs:736`), `illuminance: 22_000`,
  `ambient_lux: 2200`:

  | | sun term (lux) | fill (lux) | key : fill | (key+fill) : fill |
  |---|---|---|---|---|
  | **shipped (22° / 22 k)** | 22000 × sin22° = **8241** | 2200 | **3.75** (1.91 stop) | **4.75** (2.25 stop) |
  | pre-fix (17° / 11 k) | 11000 × sin17° = 3216 | 2200 | 1.46 (0.55 stop) | 2.46 (1.30 stop) |

  ⚠️ **The `2.46` / `3212` written in that comment is the PRE-FIX state, not the
  current one.** The same comment block goes on to say "SO THE OTHER SIDE OF THE SAME
  RATIO WAS MOVED" and "AND WHY THE ANGLE MOVED TOO" — 17°/11 k was raised to
  22°/22 k on 2026-08-08 specifically to fix that ratio. An earlier revision of this
  ticket quoted 2.46 as the live value; it is not. Do **not** go hunting for a
  2.46 ratio in the running build — it does not exist there.
  (The comment's own convention is (key+fill):fill — 3216/2200 = 1.46, +1 fill = 2.46.
  Both columns are given so either reading is checkable.)
- So the shadow lift is **not** explained by a collapsed sun:fill ratio the way it was
  pre-fix — the ratio has already been bought up to ~1.9 stop and the shadow decile is
  *still* 2.0–2.5× ref across all four plates. Whatever is holding shadows up now is
  downstream of that ratio (fill floor + `ev100` + the tonemap), which is why §4 asks
  for the shadow decile and G3's p05-L measured in the *same* report.
- The same note records that cutting the fill works and is unshippable
  (`VOXELFORGE_LOOK_AMBIENT=200` opens the split, grass dip 0.000 → 0.752) because
  `ambient_lux` is G3's floor. The key end of that ratio has already been spent once
  (17°/11 k → 22°/22 k) and the shadows did not come down, so "raise the key again"
  is not the move either — the remaining budget is in `ev100` and the tonemap's
  toe, with `ambient_lux` constrained from below by G3.
- `look.rs:809-830` records an exposure ladder on a frozen 05:38 build where hero
  p95 went 90.3 → 100.7 and hero warmth 109.2 → 131.3 as `ev100` went 10.9 → 10.3.
  **Caveat, stated because it matters:** those numbers are a different build from
  this sat sweep and are not comparable cell-for-cell with §1's table. They are
  directional evidence that exposure moves p95 and warmth — which is the reason
  warmth should be bought here and not from `POST_SATURATION`.

## 4) Definition of done

1. Shadow decile R within **1.3×** of ref (≤ ~59) on at least 3 of the 4 sweep
   plates, without `ambient_lux` dropping below the level G3's p05-L floor needs —
   state the measured G3 number in the same report.
2. Frame `Lstd` ≥ **40** (ref 46.6) on grade-vista, s1-vista, s4-raking; state
   hero's own number and, if hero cannot reach it, say why in one sentence.
3. `hero` p95 inside **150–185** — it fails on all six rungs today, so this is the
   plate that proves the exposure fix landed.
4. Re-graded with `scripts/grade_axes.py` on a **freshly rendered** set of the same
   four plates, `POST_SATURATION` untouched at 1.02, and posted per plate. Not one
   plate speaking for four.
5. `clip` must not regress past 35 on any plate — lifting exposure can rail
   highlights, and this ticket is not allowed to buy range with clipping.

## 5) One doc bug found while measuring, for whoever owns the Look Bible

`docs/look-bible.md:130` specifies shadows as `#2A2030` = (42, 32, 48) — a cool
violet. The **golden reference itself** has a darkest decile of (45.3, 16.9, 0.8):
warm and blue-clipped on 55.9 % of it. The bible's hue matches neither the approved
ref nor any plate, and my own earlier report leaned on it to call the shadows "the
wrong hue". I withdrew that; the defect is **level**, not hue. The bible line still
needs reconciling with the ref it is supposed to describe — filing it here so it
does not get lost, but it is a docs decision, not part of this fix.

— Flamingo
