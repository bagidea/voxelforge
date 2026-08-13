# A6.2 (teal cool accent) — the gate is a phantom target; the defect is a render bug

**Flamingo (Designer) · 2026-08-14 · closes out
`docs/VERDICT-a6-teal-accent-2026-08-11.md` (Yamamoto) and the A6 point-2 row in
`docs/art-order-2026-08-09-composition.md`.**

A6.2 has been re-opened three times (`90ace55` → `4d24d30` → `8f09601`), each time
with a correct-looking fix that still didn't close it. This pass started by running
the gate on its own control before touching the art — and the control is why the
item kept re-opening.

---

## TL;DR

| | verdict | evidence |
|---|---|---|
| **`hero vs bg hue split ≥ 25°` is a phantom target** | **RETIRE → ADVISORY** | all 3 CEO-approved concept sheets score **1.36 / 1.44 / 5.84°**; two of the three "orange box" frames the gate was written to reject score **inside or above** that band (0.63 / 0.32 / **5.00°**). The statistic does not rank approved art above rejected art. |
| **The clasp not rendering** | **REAL DEFECT — still open after `8f09601` (now built + shot, see §7)** | pre-fix control: **0 px in 3/3 frames**, instrument proven live against a synthetic positive; post-fix: **still 0 px in 3/3**, on a binary proven by its own constants to carry the fix |
| Replacement hard check for A6.2 | **specified + instrument built + controlled** | `scripts/_flamingo_a62_accent_presence.py` |

---

## 1. The control run — why this item kept re-opening

Office rule, learned the expensive way and already applied once on this project:
**run the grader on the approved reference first. If the control can't score, the
instrument is broken, not the picture.** That rule retired the phantom P0 targets
on 2026-07-26 (`look-acceptance-rubric.md` §P0-axes calibration log). Nobody ever
ran it on A6's hue gate.

The gate, in `scripts/art_order_grade.py` `silhouette()`:

```python
dh = abs(float(hue[blob].mean()) - float(hue[ring].mean())) % 360.0
hue_sep = min(dh, 360.0 - dh)          # want >= 25.0
```

A **linear mean of hue over the whole hero blob**, differenced against a dilated
background ring. Run on the only art anyone has signed off
(`scripts/_flamingo_a62_huesep_control.py`):

| plate | mask | hero hue | bg-ring hue | **hue_sep** | want |
|---|---|---|---|---|---|
| **auren-hero concept** (approved) | gate blob | 23.42° | 24.81° | **1.39°** | ≥ 25 ❌ |
| **auren-hero concept** (approved) | ground-truth matte | 23.48° | 24.83° | **1.36°** | ≥ 25 ❌ |
| **elder-maren concept** (approved) | gate blob / matte | 22.70 / 23.71° | 26.01 / 25.14° | **3.31 / 1.44°** | ≥ 25 ❌ |
| **guard-husk concept** (approved) | gate blob / matte | 28.35 / 32.44° | 30.71 / 26.61° | **2.37 / 5.84°** | ≥ 25 ❌ |

Two independent mask definitions — the gate's own percentile blob, and
`grade_character.mask_from_ref`'s studio matte — agree to within 0.03–1.9°, so this
is not a masking artefact. **The approved character art fails the gate by 19–24
degrees.**

### It's worse than an unreachable number: the statistic is anti-correlated

`scripts/_flamingo_a62_metric_probe.py` measures the shipped statistic and four
candidate replacements on both populations — POSITIVE = the three approved concept
sheets, NEGATIVE = the pre-accent gate3 frames the art order itself diagnosed as
"hero เป็นกล่องส้ม ไม่ใช่ตัวละคร":

| population | plate | hue_sep(mean) | hue_sep(chroma-wt) | dE_ab(chroma) | hero hue IQR |
|---|---|---|---|---|---|
| POS approved concept | auren-hero | 1.36 | 1.80 | 10.21 | 4.40 |
| POS approved concept | elder-maren | 1.44 | 1.16 | 10.80 | 4.04 |
| POS approved concept | guard-husk | 5.84 | 6.13 | 4.77 | 5.08 |
| NEG pre-accent gate3 | boot | 0.63 | 0.15 | **11.66** | 4.26 |
| NEG pre-accent gate3 | walk | 0.32 | 0.30 | 1.14 | 4.14 |
| NEG pre-accent gate3 | combat | **5.00** | **5.89** | 5.42 | **7.76** |

Not one mean-based colour statistic separates them — on `dE_ab` and `hue IQR` the
**rejected** frames score *highest*. A gate whose best-case behaviour is to rank
approved art below rejected art cannot carry a FAIL.

### And the arithmetic says a brooch could never have moved it

Because the gate averages hue linearly, painting fraction `f` of the hero at the
accent hue moves it as `hue' = (1-f)·hero + f·185.8`. Closed-form, the fraction
needed to clear 25° is:

| plate | accent share required to pass |
|---|---|
| auren-hero concept | **16.2 % of the hero's pixels** (64,618 px) |
| elder-maren concept | 16.3 % |
| guard-husk concept | 12.5 % |

The clasp gem is 0.045 × 0.05 m on a character that renders 444 × 615 px at the
gate3 boom — about **255 px, ≈ 0.23 % of the hero mask**. It is short of the bar by
a factor of ~70. **No amount of z-fixing, and no plausible brooch size, was ever
going to close A6.2 as written.** Three engineering passes were spent chasing a
number the design could not reach.

---

## 2. Degeneracy catch — the "accent" the concept sheet doesn't have

The one candidate that *did* separate the populations was "% of hero pixels whose
hue is ≥45° off the background, at sat ≥ 0.25": approved auren **0.257 %**, every
NEG frame **0.000 %**. That looked like a derived target — auren's concept already
carries an accent, match its share.

It was fake. Those 1,021 pixels average **RGB (3.2, 4.1, 6.0)** — near-black shade,
where `sat = (max−min)/max` explodes on quantisation noise — spread over **642
disconnected 8–14 px specks**. Adding a real luminance floor:

| plate | L≥0 | L≥10 | L≥20 | L≥30 |
|---|---|---|---|---|
| auren-hero concept | 0.257 % (1021 px) | 0.016 % (65) | **0.002 % (7)** | 0.001 % (2) |
| elder-maren concept | 0.013 % (67) | 0.002 % (11) | **0.000 % (0)** | 0.000 % (0) |
| guard-husk concept | 0.066 % (302) | 0.025 % (116) | **0.007 % (32)** | 0.003 % (13) |
| gate3 boot/walk/combat | 0.000 % | 0.000 % | 0.000 % | 0.000 % |

**The approved Auren design contains no cool accent at all.** The teal brooch is a
*new* element, sanctioned by `look-bible.md:132`'s reserved Accent 2 — not by the
concept sheet. So there is no positive control for a cool-cut gate either, which is
exactly the condition under which this same art-order document already, correctly,
refused to set a line (`palette_break` stayed advisory "เพราะยังไม่มีเฟรม…ที่ใคร
เซ็นอนุมัติ → ตั้งเส้นจากอะไรไม่ได้นอกจากความรู้สึก"). A6's hue row is the one place
that rule got broken.

---

## 3. What A6.2 should gate instead

A6's written intent is legibility: *"แยกได้ด้วยความสว่างอย่างเดียว ไม่ได้แยกด้วยสี — พอฉากมี
จุดสว่างอื่น hero ก็หายไปกลืน"*. The part of that which is (a) a real defect, (b) has a
clean control, and (c) is what actually broke three times, is **whether the reserved
cool accent renders at all**.

**A6.2′ — cool accent renders and reads** (`scripts/_flamingo_a62_accent_presence.py`):

> On the idle/boot frame, pixels within ±25° of `#4FC9D6` (hue 185.8°) at
> `sat ≥ 0.25` **and `L ≥ 20`** must form at least one connected component whose
> minor axis is ≥ 4 px at the reference framing.
>
> * **negative control:** every pre-fix frame — 0 px, 3/3, two shoots, two builds.
> * **instrument control:** synthetic `#4FC9D6` patches planted at the predicted
>   clasp coordinate, at full value **and** at 45 % value, are both recovered at
>   exactly 255 px with the correct bbox → a 0-px reading means the art is missing,
>   not the scanner blind.
> * **the L ≥ 20 floor is load-bearing** — without it the scanner counts the 642
>   shadow-noise specks documented above. At the clasp's own screen location the
>   surface renders at L≈59, and `#4FC9D6` only falls under L 20 below 11.4 % of
>   full value, so the floor has >10× headroom against a legitimately shaded gem.

The old `hero vs bg hue split` number keeps being printed — as **ADVISORY**, the same
treatment `warmth / blue / sat` got on 2026-08-09 for the same reason (no threshold
exists that the reference passes). Full log in `docs/look-acceptance-rubric.md`.

---

## 4. The remaining defect, and why the last fix is the right one

`8f09601` is geometrically sound; I re-derived it independently rather than take
the commit message's word for it:

* `face_z = -0.16` on both actors ⇒ **−z is the character's front**, the third-person
  boom sits behind, so +z is the camera-facing side. The cloak at `base_pos.z = 0.08`
  is on the back. Consistent.
* torso body `Cuboid(0.46, 0.58, 0.28)` skinned at `(0, neck_y*0.5, 0)` spans
  torso-local **z ∈ [−0.14, 0.14], x ∈ [−0.23, 0.23], y ∈ [0, 0.58]**.
* the clasp sits at `(0.1785, 0.475, …)` — **inside that x/y footprint**, so the
  torso is an occluder at every z ≤ 0.14. The old housing span `[0.105, 0.14]` was
  *entirely* inside the body; the old gem span `[0.1275, 0.1475]` poked out 7.5 mm.
* no other part can reach it: the cloak slab tops out at z = 0.105, the right upper
  arm at z = 0.08, and the head cube's y-range `[0.59, 0.93]` never overlaps the
  clasp's `[0.435, 0.515]`.
* new spans — housing `[0.15, 0.185]`, gem `[0.1725, 0.1925]` — clear the deepest
  occluder (0.14) by 10 mm and 32.5 mm.

**Size deliberately not changed.** Enlarging the gem was considered and rejected:
it would buy ~nothing on the (now-retired) hue gate, and 15 × 17 px at the reference
framing already clears the 4-px read floor. Keeping z as the only variable also
keeps this shoot a clean single-variable A/B against the 08-11 plates.

---

## 5. Rubric regrade — and why the clasp cannot perturb it

Graded per `docs/look-acceptance-rubric.md`, on the de-HUDded gate3 plates, with
`grade_gate.py` as the authority for G3/G5/G6 (rubric:106) and `grade_axes.py
--profile gameplay` for the P0 axes (DOF fg:bg is framing-dependent and is a
documented trap on play frames, so it SKIPs rather than FAILs):

| plate | G3 | G5 | G6 | clip | micro | p95 | DOF |
|---|---|---|---|---|---|---|---|
| gate3 boot | **PASS** p05-L 20.6 % | **PASS** min(G,B) 244 | **PASS** R−B +97, L 83.8 | PASS 6.19 | PASS 5.16 | **FAIL** 102.8 | SKIP 0.77 |
| gate3 walk | — | — | — | PASS 2.99 | PASS 6.24 | **FAIL** 94.5 | SKIP 0.43 |
| gate3 combat | — | — | — | PASS 10.25 | PASS 5.03 | **FAIL** 98.3 | SKIP 1.35 |

`warmth / blue / sat` print ADVISORY per rubric:83. **p95 is a pre-existing FAIL on
the whole gate3 plate class, not something A6 touched** — and the rubric already
warns (rubric:117) that p95 is only an exposure axis on plates whose highlights are
lit geometry. That belongs to the exposure lane, not to this item; I am flagging
it, not closing it.

**The clasp cannot move any of these, and that is provable rather than asserted.**
The gem projects to ~255 px on a 2560×1360 frame = **0.0073 % of the frame** (0.21 %
of the 119,170-px hero mask). Every rubric axis above is a percentile, an extremum,
or a whole-frame standard deviation:

* **G3** is `p05` of the interior — adding *bright* pixels cannot lower a 5th percentile.
* **G5** takes the frame's brightest pixel; the lit gem reads L≈150 against the
  current brightest 254, so it cannot be selected.
* **G6** auto-locates the brightest patch *whose hue passes R>G>B* — a teal pixel is
  disqualified by the hue clause before the luminance clause is ever reached.
* **P0 percentiles** shift by at most the fraction of distribution mass moved:
  0.0073 % of pixels can displace p35/p75/p95 by 0.0073 percentile points.
* **micro-contrast** is a hi-pass std over 3.48 M pixels; a 15×17 patch contributes
  ~64 edge pixels.

So the rubric verdict for this plate class is carried by the **release** frames above
and is unchanged by the fix. What the fix has to prove is the render itself, and that
does not depend on optimisation level — which is why the accent proof can ride the
debug binary the quest lane is already building, without spending a release slot.

**Corroboration that isn't my own instrument:** `grade_character.py` C11 *material
clusters* currently reads **5.00** against the concept's **6.00**. Monanisa's design
note predicted exactly this — the housing reuses the rig's existing `trim` colour and
adds no cluster, so the gem is the 6th. **C11 going 5 → 6 confirms the gem is on
screen from a grader I did not write and did not tune for this.**

---

## 6. Status

| | status | evidence |
|---|---|---|
| A6.2 metric — `hue split ≥ 25°` | **CLOSED — retired to ADVISORY** | control run above: approved art scores 1.4–5.8°, rejected art scores up to 5.9° on the same axis; 16 % accent share required to pass |
| A6.2 replacement gate — accent renders + reads | **CLOSED — specified, built, controlled** | `_flamingo_a62_accent_presence.py`, negative + synthetic-positive controls both green |
| A6.2 art — clasp renders | 🔴 **STILL OPEN — `8f09601` compiled, shot, and did NOT close it** | §7: binary carries the fix by constant-scan, accent still 0 px 3/3, C11 still 5.00 |
| Rubric regrade | **CLOSED on the release plates** | §5 — G3/G5/G6 all PASS, clip+micro PASS, p95 a pre-existing plate-class FAIL; the clasp is bounded at 0.0073 % of the frame and provably cannot move any axis |

**Binary provenance is checked by constants, not timestamps.** `_flamingo_a62_binscan.py`
scans an exe for the little-endian f32 patterns of each generation's z constants. Run
against the current release exe it reproduces Yamamoto's 08-11 finding exactly —
`0.1225`/`0.1375` present once each, `0.1675`/`0.1825` absent — so the instrument is
validated on a binary of known state before it is trusted on a new one.

### Handoff — one line, not my file

`scripts/art_order_grade.py` is not in `docs/LANES.md` and isn't my lane, so I have
not edited it. The change it needs, for whoever owns it:

```python
# A6 hero_hue_sep: RETIRED to advisory 2026-08-14 — the three approved concept
# sheets score 1.36/1.44/5.84 on this statistic and the frames it was written to
# reject score up to 5.89. See docs/VERDICT-a6-teal-accent-2026-08-14.md.
v.add("A6", "hero vs bg hue split", SKIP, f"{sil['hue_sep']:.1f} deg", "advisory")
```

---

## 7. Post-compile addendum — the new gate fired, and it says the art is still missing

§4 and §6 above were written while `8f09601` had never been compiled. It has been
now, and the frames are in `docs/assets/gate3-a6-teal-2026-08-14/` (1280×640 — a
**different plate class** from the 2560×1360 plates of 08-11, which matters below).

**The binary really does carry the fix.** Not asserted from a timestamp — this
project has been burned by exactly that ("commit time is not build time"):

```
target-quest/debug/voxelforge.exe  (140,935,168 bytes)
  8f09601 HEAD      housing = 0.1675    occurrences: 1   <- present
  8f09601 HEAD      gem     = 0.1825    occurrences: 1   <- present
  4d24d30 superseded housing = 0.1225   occurrences: 0   <- absent
  4d24d30 superseded gem     = 0.1375   occurrences: 0   <- absent
```

**And the accent is still not there.** Three independent readings, one of which I
did not write:

| reading | result |
|---|---|
| `_flamingo_a62_accent_presence.py --gate` | **0 px, 3/3 frames** → `A6.2 ACCENT GATE: FAIL`, exit 1 |
| any teal on the hero at all (`--hero_crop`, loose: sat ≥ 0.15, L ≥ 12, hue ±20°) | **0 px** inside a 33,505-px hero mask |
| `grade_character.py` **C11 material clusters** | **5.00** vs concept 6.00 — the corroborator §5 *pre-registered* ("C11 going 5 → 6 confirms the gem is on screen") **has not moved** |

**The 0 is the art, not the instrument — re-proven on this plate class.** A control
that passed on the 2560×1360 plates does not transfer to a 1280×640 one for free,
because the 4-px read floor is denominated in pixels. So the synthetic positive was
re-planted at the 08-11 projection site mapped through the hero bbox
(`fx=0.631, fy=0.397` → **(668,380)**) at the size the gem would actually project to
here (**8×8**, from the hero-mask area ratio 33,505/119,170 = 0.281 → linear 0.53 →
15×17 becomes 8×9 ≈ 72 px):

| rung | planted | recovered | bbox | centre | L | verdict |
|---|---|---|---|---|---|---|
| unplanted | — | 0 | — | — | — | clean baseline |
| full value | 64 | **64** | 8×8 | (667,379) | 176.0 | RECOVERED |
| 45 % value | 64 | **64** | 8×8 | (667,379) | 79.0 | RECOVERED |

The gem would land at **2× the read floor** at this framing, so the 0 px is not a
resolution artefact. The gate also now returns **PASS** on those planted plates
(exit 0) — a gate that has only ever returned FAIL has not been shown to
discriminate, so this rung is required, not decorative.

**What this changes about the verdict:** nothing in §1–§3 — the hue-split retirement
stands on its own control run and is now landed in the rubric. What it changes is
§4: the occluder re-derivation was geometrically sound *and still insufficient*, so
the remaining cause is not the clasp's z against the torso. **A fourth z re-derive
argued from geometry alone is exactly the move that has now failed three times** —
the next step should establish whether the `extra_part` is spawned at all before
anything is moved again.

⚠️ **One honest limit on the tooling:** `_flamingo_a62_hero_crop.py`'s
"which extra_parts are on screen" table **cannot** answer that question. Hair, pouch
housing and pouch coal sit at hue 22.3 / 30.6 / 40.4°, and the whole warm character
falls inside ±20° of all three — it reports 33,476 / 33,475 / 33,194 px out of a
33,505-px mask, i.e. it is measuring "the character is warm", not "the pouch
rendered". Only the clasp row (hue 185.8°) is discriminating. Do not read that table
as evidence that the other extra_parts are fine.

— Flamingo (Designer)
