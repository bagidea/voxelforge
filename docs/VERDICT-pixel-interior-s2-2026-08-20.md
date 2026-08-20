# VERDICT — interior lighting, arm **s2** baked as the default recipe

**Date** 2026-08-20 · **Lane** Flamingo / pixel (interior look) · **Baked into**
`client/src/hero.rs` → `mod recipe` (9 light/grade consts)

This note exists because the first attempt at this round was cut by a 30-minute
wall-clock ceiling with the plates on disk but **nothing baked** — the chosen values
lived only in env vars inside a sweep script, which is the exact two-sources-of-truth
failure `mod recipe` was created to end. Everything needed to re-derive or overturn
this decision is written down here.

---

## 1. What was baked

Copied verbatim from `scripts/_pixel_hero_ladder2.ps1` arm **s2** (lines 63-69):

| const | PILE A (was) | s2 (now) | `hero.rs` line |
|---|---|---|---|
| `SUN` | `[19, 196, 26000]` | `[19, 196, **32000**]` | `:182` |
| `AMBIENT` | `3900` | **`1700`** | `:194` |
| `EXPOSURE` | `8.15` | **`8.80`** | `:214` |
| `GRADE` | `[-0.06, 0.80, 1.16]` | **`[-0.01, 1.00, 1.32]`** | `:232` |
| `SHOULDER` | `0.92` | **`0.72`** | `:244` |
| `BOUNCE` | `1.0` | **`2.6`** | `:277` |
| `BOUNCE2` | `1.9` | **`1.1`** | `:278` |
| `AMBCOLOR` | `[0.30, 0.45, 0.80]` | **`[0.38, 0.51, 0.80]`** | `:295` |
| `RIM` | `[0.46, 0.62, 1.0, 5200]` | `[0.46, 0.62, 1.0, **3400**]` | `:334` |

Line numbers are the const declarations as of this note (verified by grep, not
recalled). They are in declaration order, so if the file drifts, grep
`pub const <NAME>:` inside `mod recipe` rather than trusting the column.

Nothing else moved. Camera, DOF, fog, dust, shadow-map, `SUNCOLOR`,
`BOUNCE1_COLOR`, `BOUNCE2_COLOR`, `BLUESCALE` and the pane bands are still PILE A's,
still sourced from `scripts/render_wide_hero.sh`. Each const carries its own
"what moved and why" note in `hero.rs`; the old PILE A rationale was rewritten
rather than left standing next to a contradicting number.

`hero.rs` is the right file: `look.rs` is Poppy's lane and was not touched.

---

## 2. The plates

All at 1280×720, all from `target-pixel/release/voxelforge_shot.exe`, same camera
(`VOXELFORGE_CAM=7.6,5.9,-5.2,7.6,3.2,6.0,52`).

* Round 2 arms → `_pixel_hero/s1.png` · **`_pixel_hero/s2.png`** · `_pixel_hero/s3.png`
* Round 1 arms → `_pixel_hero/r1.png` … `r5.png`
* Context → `_pixel_hero/before_2026-08-05.png` · `_pixel_hero/after_baked.png`
* Per-arm run logs sit beside each PNG as `<tag>.log`.

Measured with `python scripts/_pixel_measure.py --mode interior <png>`.

---

## 3. Why s2, measured

| frame | hue bins | cluster hue spread | warm / cool | L mean | L p5 → p95 | teal cluster sat |
|---|---|---|---|---|---|---|
| hero-look-final (CEO ref) | 2/24 | 17.0° | 100 / 0 | 0.259 | — | — (sepia) |
| PILE A baked default | 18/24 | 84.6° | 13 / 87 | 0.426 | — | — (blue wash) |
| r1 | 16/24 | 176.4° | 78.1 / 21.9 | 0.372 | 0.147 → 0.743 | 0.62 |
| r3 | 19/24 | 179.2° | 67.1 / 32.9 | 0.391 | 0.157 → 0.773 | 0.58 |
| s1 | 17/24 | 178.5° | 77.8 / 22.2 | 0.338 | 0.135 → 0.716 | 0.89 |
| **s2** | **17/24** | **178.8°** | **77.2 / 22.8** | **0.330** | **0.129 → 0.714** | **0.89** |
| s3 | 19/24 | 179.7° | 65.2 / 34.8 | 0.349 | 0.137 → 0.718 | 0.78 |

Round 2's brief was: *hold r1's warm/cool ratio, buy back contrast and saturation.*

* **Ratio held.** s2 is 77.2 / 22.8 against r1's 78.1 / 21.9 — inside a point. s3
  drifts to 65 / 35, i.e. back toward the blue-wash direction this round exists to
  correct, so s3 fails the brief on its own terms even though it scores 2 more hue
  bins.
* **Saturation bought.** The teal glass cluster goes **0.62 (r1) → 0.89 (s2)** and
  the warm shade cluster 0.66 → 0.72. This is the direct answer to "r1/r3 read as a
  real room but a PALE one". s3 gives most of that back (teal 0.78, and its top
  cluster falls to sat 0.10 — a grey highlight).
* **Shadow floor deepened.** Against **r1**: p5 0.147 → 0.129, L mean 0.372 → 0.330,
  dark cluster `c1` rgb (107,44,22) → (92,35,15), while p95 barely moves
  (0.743 → 0.714). Against **s1** (the neighbouring round-2 arm, *not* the baseline)
  the same move is much smaller: p5 0.135 → 0.129, `c1` (95,37,16) → (92,35,15) — the
  two arms are near-identical here, see §"s2 vs s1" below. The r1→s2 drop matters
  because it happens *even though the key is 23 % brighter*: `EXPOSURE` went back up
  (8.15 → 8.80) and `SHOULDER` closed (0.92 → 0.72). That is brightness moved OUT of
  a global lift and INTO a directional throw, which is exactly the intent.
  (Re-measured 2026-08-20 with `_pixel_measure.py --mode interior`; an earlier draft
  of this bullet quoted s1's `c1` as if it were r1's.)

### s2 vs s1 — the honest version

s1 and s2 are near-identical in every global statistic (bins 17/17, ratio 77.8 vs
77.2, teal sat 0.89 both). s1 did not set `VOXELFORGE_SUN`, so it ran on the
then-baked 26000; s2's only real difference is the key at 32000 with exposure and
shoulder pulled back to absorb it. **The histogram cannot separate them**, and I am
not going to pretend it did.

The separation is visible, not numeric: at a key/fill ratio of 32000 : 1700 (18.8 : 1
vs s1's 15.3 : 1) the floor carries a readable diagonal wedge of light from the
window and the right-hand corner falls away into shade; the island's two visible
faces split into a lit side and a shaded side. That is the "the window is
unmistakably the light source" test, and it is a directional-concentration question
that a whole-frame luminance histogram is structurally unable to score. Compare
`s2.png` against `s3.png` at the floor: s3's floor is evenly lit corner to corner.

---

## 4. What this does NOT claim

* **Flatness is not measurably solved.** The L p5→p95 span is 0.596 (r1) → 0.585
  (s2) — slightly *narrower*, and cluster value spread 0.519 → 0.495. Round 2 bought
  its contrast in saturation and in the key/fill ratio, not in the luminance
  histogram. Anyone re-grading this should expect the flatness note to survive.
* **s2 is not claimed to beat the CEO's `hero-look-final`.** That plate is a 2-bin
  sepia frame; this lane is not trying to reproduce it.
* **No gate was run against s2.** This is a look verdict from measurement + eye, not
  a gate PASS.

---

## 5. Before / after

`_pixel_hero/s2_before_after_sheet.png` — three frames, one binary, one camera
(`7.6,5.9,-5.2 → 7.6,3.2,6.0`, fov 52, 1280×720):

| panel | plate | what it is |
|---|---|---|
| BEFORE | `after_baked.png` | the PILE A baked default (ev100 8.15, amb `[0.30,0.45,0.80]`@3900, rim 5200) |
| middle | `r1.png` | round 1's winner, reached by env only — never baked |
| AFTER | `s2.png` | the recipe now baked into `hero.rs` |

Captions on that sheet are generated by `scripts/_pixel_s2_sheet.py`, which reads
each plate's own `PILEA …` run-log echo rather than taking typed values — a caption
on that sheet cannot drift away from the frame it sits under.

**This sheet is NOT the no-env proof.** All three panels were shot with the levers
supplied by env; the AFTER panel is the s2 *arm*, not a frame from a binary with s2
compiled in. That proof is §6.

---

## 6. Bake proof — no-env reproduction

**Status: PENDING at the time of writing (build slot not free).** The box had four
other `cargo` processes from other lanes; a third concurrent cargo here runs it out
of commit headroom and the link dies with `0xc0000142 STATUS_DLL_INIT_FAILED`, a
FALSE fail that reads exactly like a code error. `scripts/_pixel_s2_bake_waiter.sh`
is detached and waiting for a free slot; it is fail-closed (it writes `GAVE-UP …
did NOT build` rather than pretending). Live status: `_pixel_s2_waiter.state`.

When it runs it shoots three frames and judges the build by `grep -c '^error'` on
the log plus an exe mtime+size change (never `tail`, which reads green after a
failure):

* **A** `baked_s2_noenv.png` — every `VOXELFORGE_*` look lever unset. This is the
  frame a fresh clone gets, and the only one that proves the bake.
* **B** `baked_s2_herocam.png` — baked default + the ladder camera, so it is
  comparable pixel-for-pixel with `s2.png`.
* **C** `baked_s2_envcheck.png` — same camera, s2 values passed EXPLICITLY as env.

**B ≈ C is the pass condition.** If they differ, some lever did not actually get
baked and the env is still doing the work.

---

## 7. Provenance / re-derivation

* Sweep script: `scripts/_pixel_hero_ladder2.ps1` (arm s2 = lines 63-69)
* Measurement: `scripts/_pixel_measure.py --mode interior`
* Prior round's reasoning: header comment of `_pixel_hero_ladder2.ps1` (lines 1-13)
* Superseded recipe: PILE A, commit `048fc04`, plate `docs/assets/wide-hero-final.png`

Env vars still override every one of these consts, so re-sweeping needs no
recompile — but the default no longer depends on them.
