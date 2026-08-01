# Gate 3 colour review — 2026-08-01 (Flamingo, Designer)

> ## VERDICT — updated 2026-08-01, re-grade after Poppy's fix (`26b2ae6`)
>
> | | pre-fix (`db25758`) | post-fix (`26b2ae6`, on disk now) |
> |---|---|---|
> | magenta cast | **FAIL** — 16–74% of frame | **PASS** — 0.08–0.21%, all 6 native frames |
> | G6 warm golden | **FAIL** | **PASS** — all 3 |
> | Pass 9a palette lock | **FAIL** — 2.8–38.6% warm-of-lit | **PASS** — 83.9–85.8% |
> | P0 chromatic axes | **FAIL** | **still FAIL** — warmth / blue / sat short of target |
>
> **The colour bug is closed. The look is not yet at target.**
>
> - ✅ **Cleared for playable-loop evidence** (`gate3-after-*`, `playable-*`, `edhari-load`). The frames are
>   chromatically honest: sunlit surfaces read `R > G > B`, the sky survives as blue, grass survives as green.
> - ⛔ **Still HOLD for the Steam store page.** `grade_axes.py` exits 1 on all three — warmth **85.1 / 100.7 /
>   87.7** (need ≥ 110), blue **15.1 / 10.3 / 9.5** (need ≤ 10), sat **85.6 / 91.5 / 89.0** (need ≥ 90).
> - 🎯 **The fix that clears all three is measured, not guessed** — §5. It is four constants, verified on all
>   three framings, and it *improves* every safety metric at the same time.
>
> §1–§3 below are the original review of the **pre-fix** build and are left unedited as the record of the bug.
> Their numbers no longer reproduce against `docs/assets/` — those PNGs were re-shot. Reproduce them from git:
> `git show db25758:docs/assets/gate3/gate3-after-boot.png > /tmp/pre-boot.png`. Verified: that file still
> returns magenta **39.99%**, sky **184.4, 139.0, 179.9**, gain **×1.98 / ×0.54 / ×0.55**, blue **82.05**,
> sat **57.15** — every figure in §1–§3 holds against the build it was written about.

![verdict card](assets/gate3/gate3-colour-verdict.png)

**Reproducing every number in this document.** All chroma percentages and the sky/gain evidence come from
`scripts/colour_gate.py`; the P0 axes come from the pre-existing `scripts/grade_axes.py`; G6 comes from the
rubric's own `scripts/grade_gate.py`. The card above is rendered by `scripts/make_gate3_verdict_card.py`,
which measures at render time and parses the authored sky out of `client/src/main.rs` — no number on it is
typed by hand.

```
python scripts/colour_gate.py docs/assets/gate3/gate3-after-*.png    # gates A / B / C per frame
python scripts/colour_gate.py --tsv docs/assets/*.png                # the percentage tables below
python scripts/grade_axes.py docs/assets/gate3/gate3-after-*.png     # P0 axes
python scripts/grade_gate.py docs/assets/gate3/gate3-after-boot.png  # G3 / G5 / G6 (rubric)
python scripts/make_gate3_verdict_card.py                            # re-render the card above
```

Anything below that those commands do not reproduce is an error in this document, not in the scripts.
The card above is the **re-rendered, post-fix** one: its verdict line, its pass/fail colours and every figure
on it are derived at render time from `colour_gate.py`'s own gate predicates and `grade_axes.TARGETS`, and it
stamps the mtime of each frame it measured in its own footer. The pre-fix card it replaced is kept as
evidence in §1.4.

---

## 1. Is the pink intentional? — **No. It is a bug.**

The proof does not depend on taste, scene, or framing. It is a single A/B on one authored constant.

`client/src/main.rs:358` authors the sky as `ClearColor(Color::srgb(0.53, 0.72, 0.92))` = **RGB 135, 184, 235** — a
blue, ordering **B > G > R**. `ClearColor` is a flat clear, not a lit surface: whatever the scene does, that
value is what enters the post stack.

Sky sample = the largest **flat** colour cluster in the top third of the frame. `ClearColor` paints a
perfectly flat clear, so that cluster *is* the sky — the sample needs no colour prior and so cannot beg the
question. This is Gate B in `scripts/colour_gate.py`; the values below are its output.

| build | frame | sky measured | ordering |
|---|---|---|---|
| authored source | `main.rs:358` | 135.2, 183.6, 234.6 | B > G > R |
| **Jul-31 build** (ungraded) | `thirdperson-walk.png` | **135.0, 184.0, 235.0** | B > G > R — matches source to **0.4/255** |
| **Aug-01 build** (graded) | `gate3-after-boot.png` | **184.4, 139.0, 179.9** | **R > B > G — magenta** |

Converted to linear, the Aug-01 build applies this per-channel gain to the authored sky:

```
R ×1.98    G ×0.54    B ×0.55        (gate3-after-boot; the Jul-31 build returns ×1.00 ×1.00 ×1.00)
```

**G and B are pulled down by the same factor — 1.8% apart — while R doubles.** That is a *red-only* push.
A correct warm/amber white balance orders the gains **R > G > B** (amber `#F4B860` is 1.00 / 0.75 / 0.39):
green must sit clearly above blue. Here green has been dragged all the way down onto blue, which is the
definition of a magenta cast. No exposure curve, tonemap shoulder, or golden-hour grade can turn a
`B > G > R` blue into `R ≈ B >> G`; only a chroma error can.

This also explains every surface at once: warm stone keeps R and reads hot pink; the blue sky loses its
green and reads lavender; grass loses its green and reads olive.

**Look Bible agreement:** §4 authors the palette as golden-hour amber → brown, sky `#BCD3E0` (a *pale warm
blue*), with a **teal** accent at ≤15%. There is no magenta, pink, or lavender anywhere in the palette, and
no mood entry that permits it ("ไม่ใช่ dramatic/horror/neon"). Nothing in the document asks for this.

**Not damage flash — independently confirmed.** Separate captures across three demo modes return the same
gain: `gate3-after-boot`, `playable-boot` and `edhari-load` are identical to two decimals
(**×1.98 / ×0.54 / ×0.55**), and `gate3-after-walk` sits within one hundredth of them
(×2.00 / ×0.55 / ×0.56). A red damage overlay *adds* red; it cannot multiply green down to 54% of its
authored value. HP is 100/100 in all three frames. (`gate3-after-combat` is not in that list because Gate B
*skips* it — its sky owns only 28% of the top edge, below the detector's floor. Gate A still fails it at
16.4%.)

### 1.4 The pre-fix verdict card — kept as the record of the bug

![pre-fix verdict card](assets/gate3/gate3-colour-verdict-prefix.png)

⚠️ **PRE-FIX (`db25758` frames). This card is history, not the current verdict.** It is the card that headed
this document until the re-grade, rendered 12:02 against the pre-fix PNGs — its FAIL headline, the magenta
`R > B > G` swatch and its numbers (magenta 40.0 / 73.7 / 16.4, blue 82.0 / 100.9 / 62.4, sat 57 / 50 / 65,
warm 17.5 / 2.8 / 38.6) are the §1–§3 build. Every one of them is reproducible from `git show db25758:` per
the note at the top. It is preserved because it is the clearest single-image statement of the bug; the live
verdict is the card at the head of this document.

Provenance — it is a byte-identical copy of the card as committed, not a re-render:

```sh
git show 10c335f:docs/assets/gate3/gate3-colour-verdict.png | md5sum   # 1e33fc0ce219555a9f5115205ef675c6
md5sum docs/assets/gate3/gate3-colour-verdict-prefix.png               # 1e33fc0ce219555a9f5115205ef675c6
```

It is the one file under `docs/assets/` exempted from Gate A for displaying magenta on purpose (6.99%).

---

## 2. Steam store page: **FAIL** on all three.

### Grading mode (per rubric §"กติกาการเลือกโหมดเกรด")

These are 1280×720 gameplay frames of an **outdoor** scene — neither the framing nor the scene of the
1024² indoor calibration ref. So I grade **only the framing-independent axes** and explicitly discount the
rest. In particular I am **not** counting the DOF fg:bg failure against these shots: the rubric already
records that axis as a known trap for non-hero framing (the CEO-approved baseline itself scores 0.17 vs a
3.0 target, by design). Same for p95 and micro-contrast — both PASS anyway.

### What actually fails

**P0 chromatic axes** (`scripts/grade_axes.py`, exit 1):

| axis | target | boot | walk | combat | REF |
|---|---|---|---|---|---|
| blue B (mid) | ≤ 10 | **82.1** | **100.9** | **62.4** | 4.3 |
| saturation (mid) | ≥ 90 | **57.2** | **49.8** | **65.4** | 96.1 |
| warmth R−B (mid) | ≥ 110 | **85.3** | **79.1** | **100.3** | 120.9 |

Blue is **6–10× over budget**. Even granting an outdoor scene a generous allowance for legitimately blue sky
pixels, that is not a scene effect — the sky in these frames isn't blue, it's magenta.

**Gate layer:**

- **G6 (warm golden) — FAIL.** Requires `R > G > B` on a sunlit patch. Sunlit stone in the boot frame reads
  **214.7 / 148.0 / 154.5 → B > G.** The ordering that defines the look is inverted.
- **Pass 9a (palette lock, ~85% of frame warm) — FAIL.** Warm-ordered (`R>G>B`) pixels **as a share of lit
  pixels** (`mean(R,G,B) > 25`): **boot 17.5% · walk 2.8% · combat 38.6%**, against golden ref **99.1%** and
  CEO baseline `wide-hero-final` **97.7%**.

  The denominator matters and an earlier draft of this doc got it wrong — it printed golden ref **89.7%**,
  which is the same pixels over *all* pixels including unlit ones. That is not comparable across these
  frames: the golden ref is an indoor shot that is only 90.6% lit, while `wide-hero-final` is 99.9% lit, so
  the all-pixel denominator silently docks the reference ~9 points and flatters the baseline. Share-of-lit is
  the exposure-independent form and is what the table above uses. Both columns are printed by
  `colour_gate.py --tsv` if you want to check the arithmetic:

  | frame | warm % of lit | warm % of frame | lit % |
  |---|---|---|---|
  | `golden-beauty-shot-ref` | **99.08** | 89.72 | 90.55 |
  | `wide-hero-final` (CEO baseline) | **97.74** | 97.63 | 99.89 |
  | `gate3-after-boot` | **17.55** | 16.02 | 91.27 |
  | `gate3-after-walk` | **2.83** | 2.80 | 99.01 |
  | `gate3-after-combat` | **38.59** | 34.77 | 90.12 |
- **G3 (bounce not blue) — passes on a technicality, and that is a rubric hole.** G3 fails only on `B > R`.
  Magenta keeps `R > B`, so it slips through. See §3.
- G1 (voxel edges), G2 (key direction), G4 (soft shadow + AO), G5 (no blow-out) all hold. The geometry and
  lighting work is fine — **this is purely a colour failure.**

### To make these frames shippable

1. **Fix the red-only channel push in the grade path**, then re-shoot. The target is the authored sky
   surviving to the frame roughly intact (a warm grade may shift it, but ordering must stay `B > G > R`) and
   sunlit surfaces reading `R > G > B`. Owner: Poppy (build lane).
2. **Re-run `scripts/grade_axes.py` on the new frames** and require the three chromatic axes to pass. Ignore
   DOF for non-hero framing.
3. **Then** re-grade G6 + Pass 9a by eye against `golden-beauty-shot-ref.png`.

Suspect list for whoever picks this up (I did not build or edit anything — read-only review):
`client/src/look.rs` `mod grade` (TEMPERATURE 0.10 / POST_SATURATION 1.02) and the light/ambient colour on the
playable path. Note the grade constants in `look.rs` are byte-identical to the ones in `hero.rs` that produced
the clean `wide-hero-final.png`, so the constants are probably **not** the culprit — look first at the light
colour feeding the playable scene, and at whether `temperature` is landing on the intended white-balance axis.

⚠️ **Blast radius beyond Gate 3.** `docs/assets/playable-boot.png`, `playable-walk-before/after.png` and
`edhari-load.png` — the playable-loop evidence committed in the *same* ship commit `db25758` — carry the
identical cast (`playable-boot`: magenta **39.9%**, blue B **82.0**, sky gain ×1.98/×0.54/×0.55 — the same
three numbers as `gate3-after-boot` to the decimal; `playable-walk-after` magenta **66.8%**).
**Every native frame captured today is affected**, not just these three. All of it needs re-shooting.

---

## 3. Proposed new gates for `scripts/gate3_shoot.sh` (`gate3_shoot.sh` not modified — the grader is committed)

The current gate is liveness-only: exit 0 · PNG ≥ 2 KiB · `SHOT saved` · no panic. It cannot see colour, so
"renders successfully" and "renders correctly" are the same result. Three checks, cheapest first.

**Status: implemented.** These three checks now live in **`scripts/colour_gate.py`** (exit 1 on failure),
which another lane built from this section's first draft while the review was being corrected. I validated it
against the full approved + broken set rather than taking it on trust, adopted it as the single grader, and
made two changes to it — both recorded below. Only `gate3_shoot.sh` itself is still untouched: wiring the
gate into the shoot is the remaining step and it is one line.

### Gate A — magenta detector *(primary; catches exactly this bug)*

Fail the shot if a meaningful share of pixels are lit and have green below **both** red and blue:

```
lit     = mean(R,G,B) > 25                     # ignore near-black
magenta = lit AND (G < R-8) AND (G < B-8)
FAIL if magenta.count / all_pixels > 2%
```

Pre-processing: the PNG at **native resolution**, `convert("RGB")`, **no resample, no crop, no HUD mask**.
The denominator is **all pixels**, not lit pixels — that is the conservative choice (a mostly-dark frame
cannot trip the gate on a small lit region). The `% of lit` column is printed too, for reading, not gating.

| frame | magenta % of frame | magenta % of lit |
|---|---|---|
| gate3-after-boot | 39.99% | 43.81% |
| gate3-after-walk | 73.73% | 74.47% |
| gate3-after-combat | 16.38% | 18.17% |
| golden-beauty-shot-ref | **0.00%** | 0.00% |
| wide-hero-final (CEO baseline) | **0.00%** | 0.00% |
| native-control-v3 | **0.00%** | 0.00% |
| hero-tilt-down-rose-verify | **0.00%** | 0.00% |
| thirdperson-walk (green outdoor scene) | **0.00%** | 0.00% |

Zero false positives across five approved frames spanning indoor amber, outdoor green, and both framings —
including a scene dominated by green, which is the case a naive "is it warm enough" check would trip on.
A 2% threshold leaves **8.2× headroom** below the mildest real failure (combat, 16.38%) and the approved
frames do not merely pass, they read exactly 0.00% — so the threshold could sit anywhere in 0–16% without
changing a single verdict in this set. **Scene- and framing-independent**, which is what makes it safe to run
on gameplay frames that can't be graded absolutely.

*(An earlier draft of this table printed 38.9 / 72.8 / 13.3 from a scratch script that has since been
replaced by the committed one. The committed numbers above are the correct ones; combat moved 3.1 points.
This is precisely why the grader is now in the repo.)*

### Gate B — ClearColor sky probe *(the sharpest signal; near-zero cost)*

The strongest evidence in this review was a single authored constant surviving to the frame. Gate B makes
that a check: take the largest flat colour cluster in the top third, assert `B > G > R`, and report its
**linear gain vs the `ClearColor` parsed out of `client/src/main.rs`**. It needs no baseline image, and it
reads ×1.00 / ×1.00 / ×1.00 on the clean Jul-31 build against ×1.98 / ×0.54 / ×0.55 on today's.

**On skipping frames with no sky — my first draft was wrong and the implementation is better.** I proposed
"skip when the sky-pixel count is below a floor", then measured it: no count floor works. The flat bright
mass covers 31.9% of the top on `thirdperson-walk` (real sky) but only 19.3% on `gate3-after-combat` (also
real sky) and 15.6% on `wide-hero-final` (an indoor amber wall). A brightness-only version I tried
false-**FAILED** `golden-beauty-shot-ref`. I had concluded sky presence wasn't recoverable from the PNG and
was going to make the gate opt-in. `colour_gate.py` solves it properly with a discriminator I missed: **the
sky runs off the top edge of the frame and a wall does not**, so it requires the flat cluster to own ≥30% of
the top two rows, plus a luminance floor. Verified on the full set: it correctly skips all four interiors and
samples `thirdperson-walk` at exactly 135.0/184.0/235.0. That is the better design and it is what ships.

Its stated cost, which is honest and worth repeating: the luminance floor also skips a genuine night sky, and
the top-edge rule skipped `gate3-after-combat` (28% — just under the line). A frame that wrongly *skips* Gate
B is still caught by Gate A, which is why A is the primary check.

### Gate C — sunlit ordering *(implemented as **advisory only** — this is my second change to the script)*

`colour_gate.py` added a third check I had not proposed: the brightest lit non-sky surface must read
`R > G > B` (rubric G6, "warm golden"). It is a good reading of the Look Bible and the wrong shape for a
fatal gate. I measured it against both ends of the set and it is wrong in **both** directions.

**The reason it cannot hold an exit code: it false-PASSES the exact bug this review is about.**

```
python scripts/colour_gate.py docs/assets/gate3/gate3-after-walk.png
  [FAIL] A magenta fraction    73.73%                            target <= 2%
  [FAIL] B sky order          185.3,140.2,181.0 -> R > B > G     target B > G > R
  [PASS] C sunlit order       211.3,154.4,154.4 -> R > G > B     target R > G > B (advisory)
```

That frame is **73.73% magenta** — the worst of the three — and Gate C reads **PASS** on it. The mechanism is
not subtle: magenta is itself a high-red ordering, so the harder the red cast pushes, the *better* Gate C
scores the frame. Note the sunlit sample `211.3, 154.4, 154.4` — G and B are equal to a tenth, which is the
magenta signature (green pulled down to blue), and C reports it as warm golden. A check that green-lights
the failure sitting next to it would have signed today's shoot off on its own while Gates A and B were both
failing. That is the primary reason it is advisory — not taste, and stronger than the false-FAIL below.

Second, and less serious, it **false-FAILS `thirdperson-walk.png`** — a clean, approved, *ungraded* outdoor
frame — because that scene's brightest surface is grass: **121.3, 199.6, 112.8 → G > R > B**. "Warm golden"
is a property of the amber hero grade, not of every frame the engine can render; as a fatal gate it would
fail legitimate green outdoor shots forever, the exact failure mode §"Gate D" below warns about for DOF.

So I demoted it: it prints `WARN`, does not feed the exit code, and is labelled advisory in the output. Kept
rather than deleted because the reading is still useful **context** next to A and B — but on this evidence it
must never be read as a colour verdict on its own. The magenta wash it was meant to catch is caught twice
over by Gates A and B, both of which fail the frame C passes.

### Gate D — chromatic axes via the existing grader *(reuses the single source of truth)*

`scripts/grade_axes.py` already exists, already owns `TARGETS`, and already exits 1 on failure. Call it per
shot but consume **only the three chromatic axes** (warmth, blue, sat):

```
python scripts/grade_axes.py "$png"     # parse the per-axis PASS/FAIL lines
```

**Do not gate on DOF/micro/p95 for gameplay frames** — the rubric records DOF fg:bg as intrinsically failing
for non-hero framing (baseline 0.17 vs target 3.0), so wiring the raw exit code in would fail every legitimate
gameplay shot forever and the gate would be switched off within a week. This is the trap to avoid.

### Also worth fixing while in there

- **Close the G3 hole in the rubric.** G3 currently fails only on `B > R`, which catches a blue wash but lets
  a magenta wash through (magenta keeps `R > B`). Add the ordering clause `G ≥ B` on shaded/neutral surfaces.
  Cheap, and it makes the human gate agree with Gate A.
- **Baseline-drift check.** Store the per-channel means of the last approved shot set beside the PNGs and warn
  when a new shot moves any channel mean by more than ~15%. Weaker than A/B (scene-dependent), so make it a
  **warning**, not a failure — otherwise legitimate scene changes will train people to ignore it.

### Suggested ordering

A, B and C are one call and cost one full-res pass; B self-skips where there is no sky and C only ever warns.
Run D last, since it does a 1024² LANCZOS resample per frame. Gate A alone would have caught this at 11:33
today. The remaining wiring in `gate3_shoot.sh` is:

```sh
python scripts/colour_gate.py "$png" || fail "$png: colour gate"
```

**Do not wire in `grade_axes.py`'s raw exit code** (see Gate D) — parse its three chromatic lines instead.

---

## 4. Re-grade after the fix — 2026-08-01, commit `26b2ae6`

Everything in §1–§3 above describes `db25758`. This section grades what is on disk **now**.

### 4.1 The magenta bug is closed

`python scripts/colour_gate.py docs/assets/gate3/gate3-after-*.png` → **exit 0**.

| frame | A magenta | B sky | B sky gain | C sunlit |
|---|---|---|---|---|
| boot | **0.21%** | 122.3, 152.2, 193.5 → `B > G > R` | ×0.81 / ×0.66 / ×0.65 | 187.8, 161.5, 96.0 → `R > G > B` |
| walk | **0.19%** | 123.2, 153.0, 193.9 → `B > G > R` | ×0.82 / ×0.67 / ×0.65 | 179.5, 150.7, 93.0 → `R > G > B` |
| combat | **0.08%** | SKIP (no sky above the top-edge floor) | — | 210.6, 170.8, 100.9 → `R > G > B` |

The gain ordering is now `R > G > B` with **B pulled furthest down** — the shape of a warm white balance —
against the pre-fix `R ×1.98 / G ×0.54 / B ×0.55`, where G and B were dragged down together. That was the
whole diagnosis in §1 and it is answered.

**Blast radius cleared too.** The four other frames §2 flagged carry the same fix — `playable-boot` 0.20%,
`playable-walk-before` 0.19%, `playable-walk-after` 0.18%, `edhari-load` 0.21%, all Gate A **PASS**
(`colour_gate.py --tsv`). Pre-fix they read 39.9% / 39.9% / 66.8% / 39.9%.

**Gate C now catches its own hole.** `colour_gate.py`'s Gate C has since been tightened from "R > G > B" to
"no R-dominant sample with G ≈ B", and it correctly **FAILS** the pre-fix walk frame it used to pass:
`211.3, 154.4, 154.4` — G and B equal to a tenth. The §3 objection is resolved in the script.

### 4.2 G6 (warm golden) — **PASS**, all three

Measured with the rubric's own tool, `scripts/grade_gate.py` (criterion: sunlit wood `R > G > B`,
`R−B ∈ [40, 210]`, `L ≥ 55`):

| frame | golden patch | RGB | R−B | L | G6 |
|---|---|---|---|---|---|
| boot | (598, 428) | 240, 180, 91 | **+150** | 73.0 | **PASS** |
| walk | (558, 396) | 199, 157, 112 | **+86** | 63.8 | **PASS** |
| combat | (470, 660) | 255, 245, 214 | **+41** | 96.0 | **PASS** (thin) |

By eye against `docs/assets/golden-beauty-shot-ref.png`: sunlit wood and stone read honey/amber, open shade
reads warm brown, and no surface reads pink, lavender or olive. The ordering that defines the look is the
right way round again. The three frames are less deeply amber than the ref — that is the P0-axes gap in §4.4,
not a G6 failure.

⚠️ **Combat's +41 is one point off the floor of 40, and it is an artefact.** The auto-locate landed on the
campfire's blown highlight (255, 245, 214 at L 96.0), not on sunlit wood. Read the boot and walk rows as the
real G6 evidence and re-check combat by hand if it is ever the shipped still.

**Discount G3 and G5 from the same run.** `grade_gate.py` prints `G3=F G5=F` on all three, and both are
framing artefacts of the kind §2 already rules out:

- **G5** locates its brightest pixel at **(15, 12) = 255, 255, 255** on every frame — that is inside the HUD
  text block (`FPS 60 | chunks 4 | …`), not a window. Verified: the brightest pixel anywhere in
  `golden-beauty-shot-ref.png` is 255, 248, 216, so pure white does not occur in the scene; and masking the
  top 60 rows drops the boot frame's brightest pixel to **246, 247, 250**. The golden ref has no HUD; these
  frames do. (Both figures are the brightest *pixel* by `R+G+B`. Read them that way: the masked frame's
  per-channel maxima are 255 / 247 / 250, but no single pixel holds all three — that combination was in an
  earlier draft of this line and does not exist in the image.)
- **G3** scores "interior p05-L ≥ 8%" — an indoor-calibration metric. These are outdoor frames with real
  occluded void. Its tone half still reads correctly: darkest shade `R−B` **+18 / +22 / +19**, `warm=True`
  on all three.

The two brightest-pixel figures are the only numbers in this document not printed by one of the four scripts,
so here is the exact command that prints them:

```sh
python - <<'PY'
import numpy as np; from PIL import Image
def top(p, mask=0):
    a = np.asarray(Image.open(p).convert("RGB")).astype(int)[mask:]
    i = np.unravel_index(a.sum(2).argmax(), a.shape[:2]); print(p, mask, tuple(a[i]))
top("docs/assets/golden-beauty-shot-ref.png")          # 255 248 216
top("docs/assets/gate3/gate3-after-boot.png", 60)      # 246 247 250
PY
```

Same trap as DOF fg:bg, which §2 already excludes for non-hero framing.

### 4.3 Pass 9a (palette lock) — **PASS**

Rubric §Pass 9 asks for **~85% of the frame in warm tone**, evidenced by "histogram leans warm (R mean
clearly > B mean), no cold zone covering the frame". Warm-ordered (`R>G>B`) share of **lit** pixels, from
`colour_gate.py --tsv`:

| frame | warm % of lit | warm % of frame | lit % | global R−B |
|---|---|---|---|---|
| `golden-beauty-shot-ref` | 99.08 | 89.72 | 90.55 | — |
| `wide-hero-final` (CEO baseline) | 97.74 | 97.63 | 99.89 | — |
| **gate3-after-boot** | **84.15** | 71.63 | 85.12 | **+63.8** |
| **gate3-after-walk** | **83.91** | 72.08 | 85.91 | **+66.2** |
| **gate3-after-combat** | **85.77** | 73.26 | 85.42 | **+73.5** |
| *(pre-fix, for scale)* | *17.55 / 2.83 / 38.59* | | | |

83.9–85.8% against "~85%", with R mean clearly above B mean on every frame and no cold zone anywhere. **PASS.**

**The ref's 99.08% is not the target here, and chasing it would be wrong.** `golden-beauty-shot-ref` is an
indoor kitchen with no sky and no grass, so nearly every lit pixel *can* be warm. These are outdoor frames
whose blue sky and green grass are correct per Look Bible §4. Driving warm-of-lit toward 99% would mean
turning the sky and the grass warm — which is the bug this review opened with, arriving by a different road.
**~85% is the ceiling this scene should have, and it is at it.**

### 4.4 P0 chromatic axes — **still FAIL**

`python scripts/grade_axes.py docs/assets/gate3/gate3-after-*.png` → **exit 1**.

| axis | target | boot | walk | combat | REF |
|---|---|---|---|---|---|
| warmth R−B (mid) | ≥ 110 | **85.14** | **100.70** | **87.74** | 120.9 |
| blue B (mid) | ≤ 10 | **15.06** | **10.28** | 9.46 ✅ | 4.3 |
| saturation (mid) | ≥ 90 | **85.59** | 91.51 ✅ | **89.01** | 96.1 |
| micro-contrast | ≥ 5 | 11.59 ✅ | 10.45 ✅ | 12.20 ✅ | 5.24 |
| highlight p95 | 150–185 | 156.04 ✅ | 150.51 ✅ | 165.12 ✅ | 165.8 |

DOF fg:bg (0.60 / 1.19 / 1.34) is **not counted** — §2's non-hero-framing rule.

The gap is real but small and one-directional: the frame is warm, it is just not warm *enough*, and the
midtone band still carries more blue than the look wants. §5 closes it.

---

## 5. Prescription for Poppy — the numbers that clear the axes

**Everything below is measured, not extrapolated.** 19 candidate frames shot through the existing env hooks
(`VOXELFORGE_LOOK_GRADE` / `VOXELFORGE_LOOK_LIGHT`) on the one release binary Poppy's own sweep used
(`target/release/voxelforge.exe`, 12:12) — no rebuild, no source touched. The harness is Poppy's:
`scripts/_poppy_colour_sweep.sh`. Reproduce any row with:

```sh
env VOXELFORGE_PLAY=1 VOXELFORGE_LOOK_QUALITY=high \
    VOXELFORGE_LOOK_GRADE="<temp>,<sat>,1.30,0.64" \
    VOXELFORGE_LOOK_LIGHT="<kR,kG,kB,aR,aG,aB>" \
    VOXELFORGE_SHOT=out.png ./target/release/voxelforge.exe
python scripts/_poppy_sweep_report.py out.png     # warmth / blue / sat + gates
python scripts/colour_gate.py --tsv out.png       # Pass 9a (warm % of lit)
```

Swap `VOXELFORGE_PLAY=1` for `VOXELFORGE_PLAY_DEMO=1` / `VOXELFORGE_COMBAT_DEMO=1` for the walk / combat
framings. The 19 frames and their logs are in `_flamingo_frontier/` (scratch, uncommitted) if you want to
look at them rather than re-shoot; the recommended row is `r-Lwidest.png` (boot), `t-walk-Lwidest.png`,
`t-combat-Lwidest.png`, and `s-rerun-Lwidest.png` is its independent re-shoot.

### 5.1 Which lever — the two obvious ones are both wrong

**TEMPERATURE is not the warmth lever.** It buys ≈ **+6 warmth per +0.01**, but it raises the sky's R at
≈ +11/0.01 against a *falling* sky G, so Gate B (`B > G > R`) dies. Measured, not modelled:

| temp | sat | sky R, G, B | Gate B margin (G−R) | Gate B |
|---|---|---|---|---|
| 0.02 | 1.70 | 98.1, 153.1, 215.9 | **+55.0** | PASS |
| 0.03 | 1.60 | 120.8, 150.3, 210.4 | +29.5 | PASS |
| 0.04 | 1.50 | 137.3, 147.6, 204.7 | +10.3 | PASS |
| 0.045 | 1.45 | 144.3, 146.9, 202.4 | **+2.6** | PASS (at the edge) |
| **0.05** | **1.40** | **150.1, 145.8, 199.8** | **−4.3** | **FAIL — `B > R > G`** |

`look.rs`'s note prices the *full* magenta onset (`R > B > G`) at ≈ 0.099 — true, and irrelevant, because
**Gate B fails 2× earlier**, at ≈ 0.048. And it costs the look before it costs the gate: at temp 0.04 /
sat 1.50 the shade reads brick-red by eye, and Pass 9a drops to **60.10**.

**POST_SATURATION alone destroys Pass 9a.** At temp 0.02 on the shipped lights:

| sat | warmth | Pass 9a (warm-of-lit) |
|---|---|---|
| 1.02 (shipped) | 85.2 | **84.15** |
| 1.50 | 106.1 | 82.73 |
| 1.70 | 110.2 | **76.69** |
| 1.85 | 112.7 | **67.48** |

Saturation reaches warmth 110 and hands back 7.5 points of Pass 9a to do it. The mechanism matters: it pushes
pixels whose G already sits near B *across* into `R > B > G` — the magenta-adjacent ordering — **without
tripping Gate A**, which needs `G < B − 8`. So:

> 🔑 **Pass 9a (warm-of-lit) is the early-warning metric for the magenta direction. Gate A is the late one.**
> Watch 9a while tuning; by the time Gate A moves, the frame has already drifted.

### 5.2 The lever that works — hold G high in the light colours while B comes down

All rows at temp **0.02** and sat **1.70**, boot framing:

| KEY sRGB | AMBIENT sRGB | warmth | blue | sat | **Pass 9a** | magenta | sky G−R |
|---|---|---|---|---|---|---|---|
| 1.00, 0.86, 0.66 *(shipped)* | 0.98, 0.78, 0.52 | 110.2 | 3.5 | 97.7 | 76.69 | 0.31% | +55.0 |
| 1.00, 0.88, 0.60 | 0.98, 0.80, 0.46 | 111.2 | 3.2 | 98.1 | 83.16 | 0.20% | +55.2 |
| 1.00, 0.90, 0.56 | 0.98, 0.82, 0.42 | 112.1 | 3.2 | 98.3 | 84.28 | 0.15% | +55.3 |
| **1.00, 0.92, 0.52** | **0.98, 0.84, 0.38** | **113.1** | **3.2** | **98.4** | **84.68** | **0.08%** | **+55.4** |
| 1.00, 0.94, 0.48 | 0.98, 0.86, 0.34 | 113.0 | 3.5 | 98.4 | 84.86 | 0.04% | +55.5 |
| 1.00, 0.754, 0.393 | 0.98, 0.80, 0.44 | 106.6 | 2.0 | 98.7 | **48.15** | 0.29% | +54.6 |

Warmth, Pass 9a, saturation and the magenta fraction **all improve together** down the first four rows. Row 5
is the turnover: warmth stops (113.1 → 113.0) and blue starts climbing again (3.20 → 3.55). Row 4 is the knee.

The last row is the one that stops this being a rule about `G − B` alone. Its key is the Look Bible's amber
hue (`#F4B860` → 1.00, 0.754, 0.393), whose G−B of **0.361** sits *above* the shipped 0.20 and between rows 3
and 4 — yet it drops key G to **0.754**, and Pass 9a collapses to **48.15** with lit% falling to 78.4. Same
gap, opposite result. **The driver is G held HIGH while B comes down**, not the gap by itself — which is also
why round 2's L3/L4/L5 ambient walk-downs dead-ended (Pass 9a 54.5–61.9). Note this does *not* argue against
Look Bible §4's palette: `#F4B860` is the colour of the **fire/amber surface**, and using it as the *light*
colour double-counts the warm albedo underneath it.

### 5.3 The prescription

```rust
// client/src/look.rs, mod grade
pub const TEMPERATURE:      f32      = 0.02;                 // UNCHANGED — do not raise it
pub const POST_SATURATION:  f32      = 1.70;                 // was 1.02
pub const KEY_COLOR:        [f32; 3] = [1.00, 0.92, 0.52];   // was [1.00, 0.86, 0.66]
pub const AMBIENT_COLOR:    [f32; 3] = [0.98, 0.84, 0.38];   // was [0.98, 0.78, 0.52]
// MIDTONE_CONTRAST 1.30 · HIGHLIGHT_CONTRAST 1.30 · HIGHLIGHT_GAIN 0.64 — unchanged
```

**Verified on all three framings, not boot only:**

| frame | warmth ≥ 110 | blue ≤ 10 | sat ≥ 90 | Pass 9a | magenta ≤ 2% | colour gate |
|---|---|---|---|---|---|---|
| boot | **113.10** ✅ | **3.20** ✅ | **98.39** ✅ | 84.68 | 0.08% | PASS |
| walk | **125.98** ✅ | **1.41** ✅ | **99.15** ✅ | 84.11 | 0.04% | PASS |
| combat | **113.83** ✅ | **1.51** ✅ | **99.13** ✅ | 84.34 | 0.03% | PASS |

Micro-contrast **11.47** and highlight p95 **160.72** also pass on boot, so **5 of 6 axes clear** and only the
DOF trap remains (0.65 fg:bg — excluded per §2).

**Reproducible.** The recommended row was shot twice, independently: warmth **113.10 / 113.01**, blue
**3.20 / 3.27**, sat **98.39 / 98.38**, Pass 9a **84.68 / 84.68**. Drift ≤ 0.10 — the same discipline
`rerun-t003-s160` applied to Poppy's own candidate.

**Margins at the worst frame:** warmth **+3.10** over target · blue **6.80** under the ceiling · sat **+8.38**
over the floor · magenta **25× under** the gate · Pass 9a **at or above** the shipped 83.9–85.8 band.

### 5.4 Safety envelope — the boundary that must not be crossed

The rule in one line: **G must stay clearly above B, in the light colours and in every sample.** Five
checkable forms of it, each with the measured number behind it:

1. **Light colours:** `G − B ≥ 0.30` **and** `G ≥ 0.85 × R`, on **both** key and ambient. The recommendation
   is 0.40 / 0.46 and G/R 0.92 / 0.86. The second clause is not decoration — §5.2's last row satisfies the
   first and fails badly.
2. **`TEMPERATURE ≤ 0.03`.** Measured: 0.05 breaks Gate B outright, 0.045 leaves +2.6 of sky margin.
   The recommendation stays at 0.02 and leaves **+55.4**.
3. **Sky (Gate B):** `B > G > R` with **`G − R ≥ +25`**. Shipped +29.9 · recommendation **+55.4** ·
   the bug **−45.4** (139.0 − 184.4).
4. **Sunlit (Gate C):** `R > G > B` with **`G − B ≥ +40`**. Approved frames: golden ref **+108.5**,
   `wide-hero-final` **+83.9**, `thirdperson-walk` **+86.8**. The bug read **G − B = 0.0**
   (211.3, 154.4, 154.4). The recommendation reads **+153.3**.
5. **Pass 9a (warm-of-lit) ≥ 83%.** The first axis to move when the frame drifts back toward magenta.

**Do not:**

- ❌ Raise `TEMPERATURE` to buy warmth. Every point costs sky margin and pushes shade toward brick-red.
- ❌ Raise `POST_SATURATION` past 1.70 on the shipped lights — 1.85 costs 9 points of Pass 9a for 2.5 of warmth.
- ❌ Darken the ambient to chase warmth (round 2's L3/L4/L5) — lit% and Pass 9a fall together.
- ❌ Put blue back into `AMBIENT_COLOR` to lift the shade. See the note below.

### 5.5 One open taste note, and it is yours to call

The recommendation drops ambient B **0.52 → 0.38**, so open shade reads deeper and more contrasty than the
shipped frames. G3's tone half still measures `warm=True`, and by eye it reads as golden-hour shade rather
than crushed — but it is close enough to be worth a second look on a large display. **If it does read
crushed, raise the ambient's brightness in `main.rs` (currently 380 lux) — not its blue**, which rule 1
above forbids.

Also still open, and out of my lane: `gate3_shoot.sh` is still not wired to `colour_gate.py`. §3's one line
would have caught the original bug at 11:33.

---

_§1–§3: read-only review of the pre-fix render (`db25758`) — no builds run, no crate touched. §4–§5: re-grade
of `26b2ae6` plus 19 candidate frames shot through the existing `VOXELFORGE_LOOK_*` env hooks on the
pre-existing release binary — no source edited, `client/src/` untouched, `gate3_shoot.sh` unmodified. Every
figure in this document comes from `scripts/colour_gate.py`, `scripts/grade_axes.py`, `scripts/grade_gate.py`
or `scripts/_poppy_sweep_report.py`; none is typed by hand. — Flamingo (Designer)_

---

## 6. Full-asset sweep — 64 files, 2026-08-01 (Kevin, Engineer; re-run by Flamingo 16:33)

`python scripts/colour_gate.py --allow docs/assets/.colour-gate-allow $(find docs/assets -name '*.png' | sort)` → **exit 0**.

All PNGs under `docs/assets/` pass with the allowlist applied. The sweep covers every asset the project
ships — hero frames, gameplay shots, character concepts, steam capsules, VFX reference pairs, and the verdict
cards themselves. This section is the one-stop reference for which files gate how.

> **Re-run 2026-08-01 16:33 (Flamingo), after the verdict card was re-rendered.** Two layers, both exit 0:
> **64 files** in the main lane — 63 when Kevin wrote this, plus `gate3-colour-verdict-prefix.png`; row
> 16 changed and row 16b is new — and **76 files** counting `docs/assets/magenta-fix/`, an untracked dir of
> before/after evidence pairs another author was writing while I edited this. Its before-frames are *supposed*
> to be magenta (9.6–66.5%) and are exempted by their own allowlist block; its `*-after` frames are not exempt
> and pass on their own (0.00–0.24%). The table in §6.1 is those 64 — I have not enumerated another
> author's uncommitted dir, and its rows will move until it lands.
> **On "64" vs git:** the sweep reads the disk, not the index. Of the 64, 56 are tracked,
> `gate3-colour-verdict-prefix.png` is untracked and lands with this change, and 7 are the review shots
> under `docs/assets/steam/review/`, which `.gitignore:143` keeps out of the repo. A clean CI checkout
> therefore sweeps 57 files, 7 exempt, 50 fully gated — same verdict, smaller set.

### 6.1 Full sweep table (64 files, Gate A via `--tsv`)

| # | file | magenta% | magenta%(lit) | warm%(lit) | lit% | Gate A |
|---|---|---|---|---|---|---|
| 1 | `archive/native-control-v3-tiltB-recipe-REJECTED.png` | 0.00 | 0.00 | 97.75 | 100.00 | PASS |
| 2 | `archive/tiltdown-baseline-reverify.png` | 0.00 | 0.00 | 97.74 | 99.89 | PASS |
| 3 | `archive/wasm-first-frame-v2-Gl-NOTGRADEABLE.png` | 0.00 | 0.00 | 75.75 | 99.88 | PASS |
| 4 | `characters/auren-hero-concept.png` | 0.00 | 0.00 | 100.00 | 84.32 | PASS |
| 5 | `characters/elder-maren-concept.png` | 0.00 | 0.00 | 100.00 | 89.12 | PASS |
| 6 | `characters/guard-husk-concept.png` | 0.00 | 0.00 | 98.06 | 92.10 | PASS |
| 7 | `characters/the-architect-concept.png` | 0.00 | 0.00 | 41.28 | 65.59 | PASS |
| 8 | `characters/the-warden-concept.png` | 0.00 | 0.00 | 99.46 | 79.94 | PASS |
| 9 | `characters/toma-concept.png` | 0.00 | 0.00 | 98.66 | 94.29 | PASS |
| 10 | `edhari-load-test.png` | 0.02 | 0.02 | 9.48 | 99.74 | PASS |
| 11 | `edhari-load.png` | 0.21 | 0.25 | 84.14 | 85.14 | PASS |
| 12 | `edhari-village-topdown.png` | 0.00 | 0.00 | 13.19 | 48.29 | PASS |
| 13 | `gate3/gate3-after-boot.png` | 0.21 | 0.25 | 84.15 | 85.12 | PASS |
| 14 | `gate3/gate3-after-combat.png` | 0.08 | 0.10 | 85.77 | 85.42 | PASS |
| 15 | `gate3/gate3-after-walk.png` | 0.19 | 0.22 | 83.91 | 85.91 | PASS |
| 16 | `gate3/gate3-colour-verdict.png` | 0.22 | 0.22 | 5.97 | 100.00 | PASS |
| 16b | `gate3/gate3-colour-verdict-prefix.png` | **6.99** | **6.99** | 5.22 | 100.00 | EXEMPT |
| 17 | `golden-beauty-shot-ref.png` | 0.00 | 0.00 | 99.08 | 90.55 | PASS |
| 18 | `hero-tilt-down-rose-verify.png` | 0.00 | 0.00 | 92.06 | 99.99 | PASS |
| 19 | `moodboard.png` | 0.00 | 0.00 | 99.79 | 76.71 | PASS |
| 20 | `native-control-v3.png` | 0.00 | 0.00 | 97.74 | 99.89 | PASS |
| 21 | `pairs/impact-a-before-control.png` | 0.12 | 0.12 | 71.32 | 100.00 | PASS |
| 22 | `pairs/impact-a-before.png` | 0.12 | 0.12 | 71.32 | 100.00 | PASS |
| 23 | `pairs/impact-b-after.png` | 0.02 | 0.02 | 73.68 | 100.00 | PASS |
| 24 | `pairs/parry-a-before.png` | 0.12 | 0.12 | 71.32 | 100.00 | PASS |
| 25 | `pairs/parry-b-after.png` | 0.04 | 0.04 | 74.14 | 100.00 | PASS |
| 26 | `pairs/stagger-a-before.png` | 0.21 | 0.21 | 71.28 | 100.00 | PASS |
| 27 | `pairs/stagger-b-after.png` | 0.10 | 0.10 | 71.31 | 100.00 | PASS |
| 28 | `pairs/vfx-pairs-sheet.png` | 0.09 | 0.09 | 67.01 | 92.92 | PASS |
| 29 | `playable-boot.png` | 0.20 | 0.23 | 84.17 | 85.15 | PASS |
| 30 | `playable-walk-after.png` | 0.18 | 0.21 | 83.92 | 85.88 | PASS |
| 31 | `playable-walk-before.png` | 0.19 | 0.23 | 84.15 | 85.12 | PASS |
| 32 | `ship/cleanroom-hero.png` | 0.00 | 0.00 | 60.76 | 99.68 | PASS |
| 33 | `steam/header-capsule-920x430.png` | 0.00 | 0.00 | 99.28 | 76.39 | PASS |
| 34 | `steam/key-art-landscape-master.png` | 0.00 | 0.00 | 99.62 | 68.56 | PASS |
| 35 | `steam/key-art-portrait-master.png` | 0.00 | 0.00 | 99.92 | 63.45 | PASS |
| 36 | `steam/library-capsule-600x900.png` | 0.00 | 0.00 | 99.99 | 68.11 | PASS |
| 37 | `steam/library-hero-3840x1240.png` | 0.00 | 0.00 | 100.00 | 68.85 | PASS |
| 38 | `steam/main-capsule-1232x706.png` | 0.00 | 0.00 | 99.39 | 73.67 | PASS |
| 39 | `steam/page-background-1438x810.png` | 0.00 | 0.00 | 99.38 | 73.66 | PASS |
| 40 | `steam/small-capsule-462x174.png` | 0.00 | 0.00 | 99.09 | 76.03 | PASS |
| 41 | `steam/vertical-capsule-748x896.png` | 0.00 | 0.00 | 99.94 | 68.86 | PASS |
| 42 | `steam/review/capsule-set-confirmation.png` | 0.07 | 0.08 | 29.78 | 90.10 | PASS |
| 43 | `steam/review/hero-safe-area-check.png` | 0.00 | 0.00 | 96.73 | 70.43 | PASS |
| 44 | `steam/review/logo-zone-overlay-landscape.png` | 0.83 | **1.15** | 81.29 | 71.97 | PASS |
| 45 | `steam/review/logo-zone-overlay-library.png` | 0.00 | 0.00 | 94.93 | 69.83 | PASS |
| 46 | `steam/review/sim-header-at-real-sizes.png` | 0.00 | 0.00 | 41.79 | 88.58 | PASS |
| 47 | `steam/review/sim-library-tiny.png` | 0.00 | 0.00 | 36.23 | 85.64 | PASS |
| 48 | `steam/review/squint-value-test.png` | 0.00 | 0.00 | 0.00 | 88.82 | PASS |
| 49 | `thirdperson-walk.png` | 0.00 | 0.00 | 20.86 | 94.28 | PASS |
| 50 | `vfx-00-before.png` | 0.39 | 0.39 | 61.77 | 100.00 | PASS |
| 51 | `vfx-01-impact.png` | 0.05 | 0.05 | 71.52 | 100.00 | PASS |
| 52 | `vfx-02-dissolve.png` | 0.04 | 0.04 | 70.47 | 100.00 | PASS |
| 53 | `vfx-03-campfire.png` | 0.38 | 0.38 | 61.76 | 100.00 | PASS |
| 54 | `voxelfix/after-fix.png` | 0.00 | 0.00 | 97.13 | 99.88 | PASS |
| 55 | `voxelfix/dupprobe.png` | 0.00 | 0.00 | 97.13 | 99.88 | PASS |
| 56 | `voxelfix/voxel-hole-before-after.png` | 0.00 | 0.00 | 82.87 | 92.92 | PASS |
| 57 | `wasm-first-frame.png` | 0.00 | 0.00 | 0.70 | 0.48 | PASS |
| 58 | `wasm-hero-v3.png` | 0.00 | 0.00 | 97.13 | 99.89 | PASS |
| 59 | `web-parity-compare.png` | 0.00 | 0.00 | 96.92 | 70.87 | PASS |
| 60 | `web-parity-v3-compare.png` | 0.00 | 0.00 | 92.43 | 85.79 | PASS |
| 61 | `web-parity-v3-signoff-evidence.png` | 0.00 | 0.00 | 57.43 | 86.54 | PASS |
| 62 | `wide-hero-final.png` | 0.00 | 0.00 | 97.74 | 99.89 | PASS |
| 63 | `wide-tiltB.png` | 0.00 | 0.00 | 92.71 | 100.00 | PASS |

**Key observations:**
- **40 of 64 files** measure **exactly 0.00% magenta**, and **63 of 64 sit under the 2% Gate A limit** (the old "62 of 63 measure 0.00%" line did not survive a recount — the table it summarises has 23 non-zero rows; counted with `--tsv | awk`). The only file above the gate is `gate3-colour-verdict-prefix.png` (6.99%) — the pre-fix verdict card, which intentionally displays the magenta bug as the record of it (§1.4). It is fully exempt via the allowlist. The re-rendered current card measures **0.22%** and passes Gate A unaided, so it carries no exemption.
- `steam/review/logo-zone-overlay-landscape.png` has the highest magenta among passing files at 0.83% (1.15% of lit) — well under the 2% gate with 2.4× headroom.
- The steam capsule files (rows 33–41) all show 0.00% magenta — Gate A confirms the hand-painted artwork has no magenta cast.
- `vfx-00-before.png` and `vfx-03-campfire.png` at 0.39% are the highest among rendered frames.

### 6.2 Gate C false positives — one fixed structurally, one left

Both files are near-neutral surfaces with magenta = 0.00%, and both used to **FAIL Gate C** (sunlit order):
the magenta signature requires R dominant AND G crushed onto B, and a near-gray surface satisfies the second
half by definition while R edges ahead by a point or two on the first. Re-measured 16:33 — one of them no
longer trips at all:

| file | sunlit RGB | G−B | Gate A | Gate B | Gate C | root cause |
|---|---|---|---|---|---|---|
| `edhari-village-topdown.png` | 170.9, 170.3, 169.5 | **+0.8** | 0.00% PASS | SKIP | **PASS** (was FAIL) | top-down mid-gray village surface, R~G~B within 1.4 points — under `R_DOMINANT_MARGIN`, so it is no longer read as red-dominant |
| `archive/wasm-first-frame-v2-Gl-NOTGRADEABLE.png` | 173.2, 165.8, 160.2 | **+5.6** | 0.00% PASS | PASS | **FAIL** | first-frame render of a nearly-monochrome scene, G−B < SUN_GB_MIN (20); its R−G lead of 7.4 clears the margin, so this one is still a genuine false positive and stays exempt (via the `*NOTGRADEABLE*` glob) |

Both have magenta = 0.00% and pass Gates A+B. The remaining Gate C failure is a structural limitation of
checking "red-dominant with G ≈ B" on near-neutral surfaces. **Preferring the structural fix to the
allowlist is the rule here**: an exemption silences one file forever, a margin fixes every near-gray frame
the project will ever render.

**Why the old report marked them PASS and why that was wrong:** a PASS label would have implied the
files passed all three gates, which at the time neither did. The honest entry for a file the gate is
failing is **FAIL C** plus the reason it can be disregarded — which is what this table records for the
one file still failing. `edhari-village-topdown.png`'s PASS above is a different thing entirely: the gate
itself changed and now reads it correctly, so there is nothing left to disregard.

### 6.3 Allowlist — `docs/assets/.colour-gate-allow`

To make `colour_gate.py` usable in CI (`exit 0` on a clean repo), an allowlist exempts three categories
of files that are known to gate-fail for documented reasons:

| group | # files | exemption | rationale |
|---|---|---|---|
| **Verdict cards** | 1 (`gate3-colour-verdict-prefix.png`) | ALL gates | The **pre-fix** card only — it intentionally displays the magenta bug as the record of it (§1.4). The current card is deliberately *not* listed: it measures 0.22% and passes unaided, and an exemption on it would hide a regression in the card itself |
| **REJECTED / NOTGRADEABLE frames** | 2 | ALL gates | Preserved for reference; not meant to pass colour gate |
| **Steam capsule artwork** | 4 (`header`, `main`, `page-background`, `small`) | Gate B only | Hand-painted, no rendered sky — B sky order is meaningless. Gate A (magenta) still enforced |
| ~~**Gate C false positives**~~ | ~~1 (`edhari-village-topdown.png`)~~ | — | **No longer allowlisted.** `colour_gate.py` grew an `R_DOMINANT_MARGIN` (R must lead G by a real margin), so a mid-gray surface at R~G~B within 2 points no longer trips Gate C at all. Fixed structurally instead of exempted — §6.2's first row now reads PASS |

Total exempted: **7 of the 64 files this sweep covers** (everything under `docs/assets/` except the
`magenta-fix/` before-frame archive), all with per-file rationale in the allowlist file. Verified,
not counted by hand — the list is exactly the `EXEMPT` lines of the sweep:

```sh
python scripts/colour_gate.py --allow docs/assets/.colour-gate-allow \
  $(find docs/assets -name '*.png' -not -path '*magenta-fix*' | sort) \
  | awk '/^docs/{f=$1} /EXEMPT/{print f}' | sort -u
```

**Gate A is still enforced on everything except the pre-fix verdict card and REJECTED/NOTGRADEABLE frames.**
A real magenta render placed anywhere under `docs/assets/` with a non-exempt name **will** fail the gate
and produce `exit 1`. Copying `gate3-colour-verdict-prefix.png` to any name not matching an allowlist entry
correctly fails: `[FAIL] A magenta fraction 6.99% → COLOUR GATE FAIL → exit 1`.

**Allowlist file format** (`docs/assets/.colour-gate-allow`):
```
# <glob>  <reason text…>  [gates]
docs/assets/gate3/gate3-colour-verdict-prefix.png  pre-fix verdict card, intentionally shows magenta bug
docs/assets/**/*REJECTED*.png  REJECTED frame, preserved for reference
docs/assets/steam/header-capsule-920x430.png  hand-painted steam capsule  B
```
- `glob` — relative path from repo root; `**` supported for zero-or-more directories
- `reason` — free text explanation (rest of line minus optional trailing gate letters)
- `gates` — optional trailing token `A` / `B` / `C` / `A,B` to exempt specific gates; omit for full exemption

### 6.4 CI usage

```sh
# Full sweep — exit 0 on a clean repo. Same file set as §6.3: everything under docs/assets/ except the
# magenta-fix/ before-frame archive (every file in there is an allowlisted before-frame, so including it
# only inflates the counts: 76 files / 15 exempt, still exit 0).
python scripts/colour_gate.py --allow docs/assets/.colour-gate-allow \
  $(find docs/assets -name '*.png' -not -path '*magenta-fix*' | sort)

# Gate-only (no allowlist) — for reviewing new frames before allowlisting
python scripts/colour_gate.py my-new-frame.png
```

The allowlist is intentionally narrow: **8 allowlist patterns → 7 files exempt, 57 under full Gate
A+B+C** (8 patterns but only 7 files because the 8th, `magenta-fix/*-before*.png`, matches nothing in
this sweep — that directory is excluded above; the two glob entries happen to match one file each here).
If someone commits a new frame with a real magenta cast, the CI gate catches it — the magenta must be in
a file whose name contains `REJECTED` or `NOTGRADEABLE` to slip through.

_§6: full-asset sweep, allowlist implementation, and CI integration — Kevin (Engineer), 2026-08-01_
