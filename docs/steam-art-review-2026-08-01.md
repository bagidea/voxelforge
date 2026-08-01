# Voxelforge — Steam capsule art review (Layer 1)

> Reviewer: Flamingo (Design). Date: 2026-08-01.  
> **Update 2026-08-01 (Kevin):** Commit `6376138` resolved **F1** (upload sizes fixed to 920×430/1232×706/462×174/748×896/1438×810) and **F3** (small capsule, vertical capsule, page background all created). F2, F4–F10 remain open — see §4 options.
> **Confirmation pass 2026-08-01 (Flamingo), §6 below:** re-measured from disk. **F1 is closed — verified.** **F3 is only partly closed**: 3 of the 4 named assets exist, the **Library header 920×430 is still missing**, and all three "new" assets are derived crops of the same 1024² masters rather than the fresh compositions F3 asked for. F4/F5/F9/F10 reproduce unchanged.
> Subject: `docs/assets/steam/` — 4 capsules + 2 key-art masters, produced per `docs/steam-store-art.md`.
> Method: every claim below is measured, not eyeballed. Scripts: `scripts/_review_steam_capsules.py`,
> `scripts/_review_steam_safearea.py`. **These scripts are intentionally not in git** — `.gitignore:139`
> ignores `/scripts/_*.py` as one-shot review scripts, so they exist on the reviewer's disk only. Don't go
> looking for them in the tree; re-derive from the numbers here, or re-author them. Proof images:
> `docs/assets/steam/review/`.
> Spec source: Steamworks **Store Graphical Assets** + **Steam Library Assets** docs, fetched 2026-08-01
> (this closes the open item in `steam-store-art.md` §7: "confirm final px sizes against the Valve spec").
> No game code, no engine run, no art file was modified by this review.

---

## 0. Verdict in one line

**The art direction is right and the composition has real bones — but the file set cannot be submitted
to Steam as it stands.** Four blockers are spec/compliance (wrong upload sizes, no logotype, three
required assets missing, hero breaks Valve's safe area) and three are look-bible violations the
capsules inherited from the masters (palette collapsed to one hue, shadows crushed below the
documented floor, reserved logo zones aren't actually clear).

Scorecard: **5 PASS · 10 FIX**, of which **4 are hard submission blockers**.

---

## 1. What PASSES — keep this, don't re-do it

| # | Finding | Evidence |
|---|---|---|
| P1 | **All four shipped capsules are dimensionally exact and undistorted.** 460×215, 616×353, 600×900, 3840×1240 — every one is a pixel-exact match to its target, and every source crop band's aspect lands within the accepted squash tolerance of **≤ 0.25%** against its slot aspect: worst case **0.202%** (the hero), source = integer crop bounds. That is a PASS in practice, not a zero — at 0.202% a 100px feature drifts 0.2px, which is invisible. | §1 of the measure run: `EXACT-MATCH` on all four; slot ar 2.1395 / 1.7450 / 0.6667 / 3.0968. Band-vs-slot distortion measured per file in **§6.1** (0.033–0.202%) |
| P2 | **The library capsule is the strongest asset in the set — it survives to thumbnail.** Full-body Auren, and the ember pouch acts as the focal dot. Legible at 150×225, 100×150, and still readable at 64×96. | `review/sim-library-tiny.png` |
| P3 | **The value structure is genuine key art.** Squint to 40px and kill colour: a strong dark hero mass reads against a bright open sky, with clean figure/ground separation. That's the hard part of key art and it's already solved. | `review/squint-value-test.png`; squint-std 59–68 across capsules |
| P4 | **Highlights are not clipped anywhere.** 0.00% of pixels above L250 in every file — the filmic/ACES discipline from look-bible §2 #10 held. | §3: `blown(L>250)=0.00%` on all six files |
| P5 | **The art-direction beats from `steam-store-art.md` §3 are all actually present.** Auren off-centre right, facing into open space; god-ray shaft; Guard Husk silhouette distant and DOF-soft; ember pouch as the one warm point-light; voxel hard 90° edges throughout. The brief was executed, not drifted from. | Visual, confirmed on both masters |

---

## 2. What must be FIXED

### 🔴 P0 — hard blockers on Steam submission

**F1 · The upload sizes are half of what Valve requires.**
The numbers in `steam-store-art.md` §6 are Steam's *display* sizes, not its *upload* sizes. Valve's
current spec:

| Asset | Required upload | Shipped | Status |
|---|---|---|---|
| Header capsule | **920 × 430** | 460 × 215 | ½ size |
| Main capsule | **1232 × 706** | 616 × 353 | ½ size |
| Library capsule | 600 × 900 | 600 × 900 | ✅ |
| Library hero | 3840 × 1240 | 3840 × 1240 | ✅ (but see F4) |

Shipping half-size means Steam upscales on every hi-dpi display — the store page will look soft next
to competitors. *Fix:* regenerate header and main at 2×. Note this breaks the current pipeline: a
1232×706 capsule cannot come out of a 1024×1024 master without upscaling, so F1 forces F7.

**F2 · There is no logotype in any asset — Valve rejects capsules without one.**
The spec is explicit and repeats it per asset: *"The game's logotype should be easily legible against
the background"*, and for the small capsule, *"your logo should nearly fill the small capsule."*
`steam-store-art.md` §4 deliberately kept text out of the generated art — correct for the *masters*,
but the shipped *capsules* were never composited with a lockup, so the deliverable set is text-free
and non-compliant. There is also no **Library Logo** asset (required: transparent PNG, 1280px wide
and/or 720px tall) — that's the logo Steam overlays on the library hero, so the hero cannot function
without it.
*Fix:* commission the Voxelforge logotype, then add a compositing step to `make_steam_capsules.py`
that lays it into each capsule; export the standalone transparent Library Logo separately.

**F3 · Three required assets are missing entirely.**

| Missing asset | Required size | Why it matters |
|---|---|---|
| **Small capsule** | 462 × 174 | The most-seen asset on Steam — search results, wishlists, top-sellers, recommendations. Its absence is the single biggest visibility cost. |
| **Vertical capsule** | 748 × 896 | Required for front-page features, daily deals, seasonal sales. |
| **Library header** | 920 × 430 | Required library asset. |

(Optional, worth having: Page Background 1438×810.) The small capsule is *not* a crop of the header —
at 462×174 the logo has to nearly fill the frame, so it's a separate lockup-first design.
*Fix:* add all three to the crop script with their own compositions, not as derived crops.

**F4 · The library hero fails Valve's safe area — Auren's face will be cropped off.**
Valve: the safe area is the **centre 860 × 380 band** — a horizontal strip in the middle of the
3840 × 1240 frame, *not* the full height — and *"a main character's face should be entirely in the
safe area or risk being cropped"* during window resizing. On this asset that band is
**x 1490 – 2350, y 430 – 810**. Measured:

- Auren's dark mass: x 2253 – 3126, y 32 – 1239 → **11.1% inside horizontally, 31.5% vertically**;
  by true 2-D pixel overlap only **3.4%** of him lands inside the band.
- Auren's head/hood band: x 2344 – 2868, y 32 – 394 → **1.1% inside horizontally, 0.0% vertically.**
  The whole head sits *above* the safe area's top edge (y 430), so it is outside on both axes — not
  merely "tight". The 32px of frame headroom is irrelevant: the frame is not the crop boundary, the
  band is.

Proof: `review/hero-safe-area-check.png` (green = the real 860 × 380 band).
*Fix:* this cannot be re-cropped out of the existing master — Auren sits at the master's right edge and
runs the full height, while the safe area needs his face inside x 39–61% / y 35–65% of the frame. It
needs a dedicated hero master with Auren centred *and* his head at mid-height. Which is the same fix as
F5, so do them together.

**F5 · The library hero is a 3.75× upscale, and it measures like one.**
`steam-store-art.md` §7 flags this honestly; the measurement confirms it is not shippable.
Per-pixel gradient energy: **1.16 (hero) vs 3.89 (native 1024 master)** — roughly a third of the
high-frequency detail survives. This is the physically largest asset on screen in the Steam library.
*Fix:* render the hero master at native 3840px wide. Do not ship the Lanczos upscale.

### 🟠 P1 — not a blocker, but it stops the set reading as AAA

**F6 · Both "reserved" logo zones from `steam-store-art.md` §4 are, in fact, occupied.**
The doc promises clean negative space; measured, neither zone is clean:

| Promised clean zone | L std | edge energy | Verdict |
|---|---|---|---|
| Landscape master, left third | 50.3 | 13.11 | **BUSY** |
| Landscape master, left half | 57.8 | 13.28 | **BUSY** |
| Portrait master, top 25% | 64.7 | 11.97 | **BUSY** |
| Library capsule, top 25% | 52.1 | 10.93 | **BUSY** |

What's actually in there: the ruins silhouette runs diagonally through the whole left third, the Guard
Husk stands in it, the god-ray crosses it, and Auren's sword tip pushes into the left *half*. On the
portrait side, the hood starts at ~13% from the top, so it sits inside the band reserved for the title.
Proof: `review/logo-zone-overlay-landscape.png`, `review/logo-zone-overlay-library.png`.
*Consequence:* a title lockup here either lands on the Husk (killing the "distant danger" beat) or
needs a dark scrim behind it — and a scrim fights the cozy pillar.
*Fix:* in the re-render, drop the left-third ruins below the horizon line so the upper-left is open sky,
move the Husk right so it stays in frame but out of the lockup zone, and lower Auren in the portrait
frame so his hood starts below the 30% line.

**F7 · Both masters are 1024×1024 squares — this is the root cause of F1, F4 and F8.**
`key-art-landscape-master.png` and `key-art-portrait-master.png` are both perfectly square. Neither was
authored at the aspect it feeds, so every capsule is a band sliced out of a square — which is why the
hero can't reach its safe area, why the header band amputates the character, and why 2× sizes aren't
reachable. *Fix:* author masters **at target aspect**: a ~3.1:1 hero master at native 3840px, a ~1.75:1
landscape master at ≥1232px, and a true 2:3 portrait master. Rename accordingly — "landscape master"
that is square is a trap for whoever picks this up next.

**F8 · The header and main crops amputate the character read.**
The bands (y140–618 and y130–717) cut Auren at the chest. Lost: the sword, the cloak trail, and the
**ember pouch** — which the library-tile sim proves is the single element that reads best at small
size. What survives at 231×87 is a faceless brown hood. The cloak is also hard-cut by the right frame
edge (dark mass occupies **91.9%** of the header's right edge column, 66.3% of the main's).
*Fix:* lower the band to include the ember pouch, and shift Auren ~8% left so the cloak has air on the
right instead of running off it.

**F9 · The palette law is broken — the frame is monochrome, not golden-hour.**
`look-bible.md` §4: ~85% warm, with a small teal accent that is described as essential
("จุดเล็กๆ ตัดให้ภาพมีชีวิต"). Measured:

| File | warm % | teal % | median saturation |
|---|---|---|---|
| landscape master | 98.6 | 0.018 | 0.85 |
| portrait master | 99.7 | 0.019 | 0.96 |
| header capsule | 98.9 | 0.024 | 0.85 |
| main capsule | 98.7 | 0.020 | 0.85 |
| library capsule | 99.8 | **0.001** | 0.96 |
| library hero | 98.7 | 0.006 | 0.87 |

99% warm is not "85% warm with an accent" — it's one hue. At 0.02% of a 460×215 capsule the teal is
about **24 pixels**; in the library capsule it is gone. `steam-store-art.md` §5 records 0.02–0.03% as a
verified accent and dismisses the earlier 0.003–0.008% as compression noise — but those are the same
order of magnitude. Functionally there is no accent in either pass. Also absent: the sky blue
`#BCD3E0` and stone-beige `#B9A98C` that give the palette its neutral counterweight; median saturation
of 0.85–0.96 means nearly every pixel is fully-saturated orange.
*Fix:* let the zenith (top ~15%) drift toward `#BCD3E0`; let mid-ground stone read `#B9A98C`; and size
the teal emissive block so it survives the header crop — target **≥0.3% of frame area**, i.e. roughly a
27×27px block on a 1024 master, not a 6px dot.

**F10 · Shadows are crushed past the look-bible floor.**
Look-bible §4 sets the shadow floor at `#2A2030` (luma ≈ 35) and states shadows must never go pure
black. Measured share of pixels below L12:

- landscape master **13.45%** · library hero **13.76%** · main capsule **10.41%** · header **7.56%**

This is the same defect the in-engine rubric catches at gate G3 ("เงาถูกบดดำ", p05 ≥ 8) — the marketing
art is failing the game's own shadow law. It's also why the frame reads as post-apocalyptic dusk rather
than golden hour.
*Fix:* lift the shadow floor in grade. Target: <3% of pixels below L12, p5 ≥ 25.

---

## 3. Does it say "Voxelforge"? — against `look-bible.md`

| Look-bible requirement | Reads in the capsules? |
|---|---|
| Voxel hard 90° edges, no bevel | ✅ Unmistakable, at every size |
| Golden-hour low raking sun (15–25°) | ✅ |
| Volumetric god rays | ✅ Present, though faint after the header crop |
| Atmospheric fog / haze thickening with distance | ✅ Strong |
| Filmic tone-map, highlights not burnt | ✅ 0% clipped |
| Emissive + bloom | ✅ The ember pouch, and it's the best small-scale read in the set |
| Soft shadow / contact AO | ➖ Can't be judged — the figure is a silhouette |
| **PBR material read (wood grain, stone, roughness)** | ❌ Everything is silhouette. At capsule scale you cannot tell this is *voxel geometry × realistic shader stack* rather than a dark fantasy painting — and that contrast is the stated core of the identity (look-bible §0) |
| **Warm bounce / GI** | ❌ Unlit sides go to near-black (F10). Look-bible: *"GI คือความต่างระหว่างสวยกับแค่มี shader"* — the frame currently shows the "แค่มี shader" side |
| **Teal accent** | ❌ ~24 pixels (F9) |
| **Mood: อบอุ่น · เงียบสงบ · น่าอยู่ · "อยากเข้าไปนั่งจิบชา"** | ❌ **The biggest miss.** These read as post-apocalyptic dusk: ruins only, no dwelling, no life, crushed shadows. `steam-store-art.md` §2 explicitly excluded "night/horror lighting" and `GAME-VISION` pillar 2 is *souls weight in a **toy** world* — the toy/cozy half is absent. Right now the store page promises a grim survival game |

**Bottom line on identity:** the capsules sell the "souls" half of the pitch and none of the "cozy voxel"
half. That's a positioning problem, not just a colour problem — it will pull in the wrong wishlist
audience. The single highest-leverage change is F9 + F10 together: lifting the shadows and restoring the
cool counterweight puts warmth and material back on screen, which is what makes it read as Voxelforge
rather than as generic dark fantasy.

---

## 4. Recommended path — 3 options

| | Option A — **re-shoot the masters at target aspect** ⭐ | Option B — patch what exists | Option C — derive from an in-engine render |
|---|---|---|---|
| What | Author 3 masters natively: 3840×1240 hero (Auren centred), ≥1232×706 landscape, true 2:3 portrait — with F6/F9/F10 fixed in the prompt. Then re-crop + composite the logo. | Upscale + regrade the current masters, composite a logo, hand-build the 3 missing assets. | Wait for the engine look to pass Gate 3, then shoot key art in-engine at native resolution. |
| Fixes | F1 F4 F5 F6 F7 F8 F9 F10 | F2 F3 F9 F10 (partial) | Everything, plus the art is honest to the game |
| Leaves broken | F2, F3 (still need the logo + the 3 assets — unavoidable in every option) | **F4 and F5 cannot be fixed** — hero stays an upscale with the face outside the safe area | Nothing, but blocked on the engine |
| Effort | ~3 gen passes + a crop-script rewrite | ~1 day | Weeks — gated on Gate 3 |
| Risk | Regeneration drifts from the approved composition | Ships a soft, non-compliant hero | Schedule |

**Recommendation: A now, C later.** Option A is the only path that clears the submission blockers on a
store-page timeline, and it costs about the same as B once you count hand-building the three missing
assets. Then, once the engine look clears Gate 3, re-shoot the key art in-engine (C) — art rendered from
the actual game is always the stronger store asset, and by then the logo lockup and the crop pipeline
from A are already built and reusable.

**Do first, in this order:** F2 (logo — everything else composites onto it) → F7/F1 (re-author masters
at real sizes) → F4/F5 (hero) → F9/F10 (grade) → F3 (the three missing assets) → F6/F8 (crops).

---

## 5. Corrections to `steam-store-art.md`

Three statements in that doc should be updated when this is actioned:

1. §6 — the size table lists display sizes as if they were upload sizes. Header is 920×430 and main is
   1232×706. The §7 open item ("confirm final px sizes against the Valve spec") is now **closed: the
   numbers changed.**
2. §5 — "Verified 2026-07-31: ~0.03% / ~0.02% teal" is arithmetically true but the conclusion drawn from
   it isn't: 0.02% is not a functional accent, and it's 0.001% in the library capsule. The palette law
   is not met.
3. §4 — the reserved logo zones are described as delivered. Measured, all four are BUSY. The promise and
   the artwork disagree.

## 6. Confirmation pass — 2026-08-01, after commit `6376138`

Re-measured **from the files on disk**, not from this document. Runs:
`scripts/_review_steam_capsules.py` (updated to the new filenames — the old ones it pointed at were
deleted by `6376138`), `scripts/_review_steam_safearea.py`, `scripts/_review_steam_confirm.py` (new).
**None of these three are in git, by design** — `.gitignore:139` (`/scripts/_*.py`) treats `_`-prefixed
Python as one-shot review scratch, the same convention as `/scripts/_*.sh`. The edits this pass made to
`_review_steam_capsules.py` (the band-vs-slot aspect measurement in §6.1) therefore live on disk only and
will not appear in any commit — that is expected, not a lost change.
Spec re-fetched live from `partner.steamgames.com/doc/store/assets/{standard,libraryassets}` on the
same day rather than trusted from the commit message.

**Scope of the comparison — read this before trusting a "unchanged" below.** The baseline this pass
compares against is **the numbers written in §2 of this document**, not the earlier proof images.
`docs/assets/steam/review/` is gitignored (`.gitignore:143`) and the review scripts overwrite it in
place, so re-running them destroyed the previous PNGs before anything could be diffed against them —
there is no image-to-image proof that the old and new sheets agree, only that the measurements
re-derive to the same figures. Every "reproduces unchanged" in §6.3 therefore means *the metric came
back at the same value*, not *the proof image is byte-identical*. If image-level regression proof is
wanted later, the fix is to un-ignore that folder or write each run into a timestamped subfolder.

### 6.1 Dimensions — every shipped file vs Valve's current spec

| File | Measured | Valve requires | Verdict |
|---|---|---|---|
| `header-capsule-920x430.png` | 920 × 430 | 920 × 430 | ✅ PASS |
| `small-capsule-462x174.png` | 462 × 174 | 462 × 174 | ✅ PASS |
| `main-capsule-1232x706.png` | 1232 × 706 | 1232 × 706 | ✅ PASS |
| `vertical-capsule-748x896.png` | 748 × 896 | 748 × 896 | ✅ PASS |
| `page-background-1438x810.png` | 1438 × 810 | 1438 × 810 (optional) | ✅ PASS |
| `library-capsule-600x900.png` | 600 × 900 | 600 × 900 | ✅ PASS |
| `library-hero-3840x1240.png` | 3840 × 1240 | 3840 × 1240 | ✅ PASS |
| *(none)* | — | **920 × 430 library header** | ❌ MISSING |
| *(none)* | — | **1280 × 720 library logo, transparent** | ❌ MISSING |

All seven are `mode=RGB` (fine for capsules; the missing library logo is the one that must ship with
an alpha channel). Proof sheet: `docs/assets/steam/review/capsule-set-confirmation.png`.

**Correction — the aspect-distortion number.** An earlier draft of this section reported *"aspect
distortion 0.000% on all seven"*. That figure was tautological and has been withdrawn: it compared the
shipped file's own `w/h` against the same slot's `ew/eh`, and every file EXACT-MATCHes its slot, so it
could only ever return 0. It measured nothing. Squash/stretch actually happens one step earlier — in
the Lanczos call in `make_steam_capsules.py`, which forces a band of the square master into the slot's
aspect. The honest metric is that **band's** ar vs the target ar (`scripts/_review_steam_capsules.py`
now measures this; the bands are the real ones, proven by C1's maxdiff = 0 re-derivation):

| File | source band | band ar | slot ar | squash |
|---|---|---|---|---|
| `main-capsule-1232x706` | 1024 × 587 (y130–717) | 1.74446 | 1.74504 | 0.033% |
| `page-background-1438x810` | 1024 × 577 (y135–712) | 1.77470 | 1.77531 | 0.034% |
| `header-capsule-920x430` | 1024 × 479 (y140–619) | 2.13779 | 2.13953 | 0.082% |
| `small-capsule-462x174` | 1024 × 386 (y187–573) | 2.65285 | 2.65517 | 0.087% |
| `library-capsule-600x900` | 682 × 1024 (centred) | 0.66602 | 0.66667 | 0.098% |
| `vertical-capsule-748x896` | 854 × 1024 (centred) | 0.83398 | 0.83482 | 0.100% |
| `library-hero-3840x1240` | 1024 × 330 (y330–660) | 3.10303 | 3.09677 | **0.202%** |

Worst case **0.202%** (the hero), not 0.000%. Source: integer crop bounds — a 330px-tall band can't hit
3.09677 exactly, and `crop_vertical()` truncates its band width with `int()`. At 0.202% a 100px feature
drifts 0.2px, which is invisible, so this still **passes in practice** — but it is a tolerance, not a
zero, and it must not be quoted as proof of anything beyond "the bands were chosen sanely".

**The tolerance this review accepts is ≤ 0.25%**, and it is what P1 (§1) now stands on. It is set just
above the measured worst case rather than at a round number pulled from nowhere: integer crop bounds are
the only error source, so the ceiling is a property of the crop maths, not of taste. The rule for whoever
re-crops later: if a band exceeds 0.25%, it is no longer rounding — go find the real cause before shipping
it. (A band authored at target aspect — F7 — removes the error entirely.)

**F1 is closed.** The header/main doubling is exact (920×430 = 2×460×215, 1232×706 = 2×616×353,
ar 2.13953 and 1.74504 both unchanged), so the commit message's claim holds under measurement.

### 6.2 What the size table hides — the three findings the resize did not close

**C1 · Nothing was composited into any file — every asset is a bit-exact crop of the two masters.**
Re-deriving each output from `key-art-*-master.png` with the crop bands in `make_steam_capsules.py`
gives **maxdiff = 0** on all seven files. That is a clean pass on pipeline honesty, and simultaneously
hard proof that **F2 is untouched**: there is no logotype pixel anywhere in the set. It also means the
"new" small/vertical/page-background assets are **derived crops**, which is exactly what F3 said not to
do — at 462×174 Valve wants the logo to nearly fill the frame, and this file is a landscape band with no
logo at all. Measured as a lockup surface the small capsule is **BUSY** (L std 66.4) across the whole
frame, so a logo dropped on it later will need a scrim.

**C2 · The library header slot is still empty.** F3 listed three missing assets; two were created.
`library-header-920x430.png` was not, and neither was the Library Logo from F2 — and the logo is what
Steam overlays on the hero, so the hero still cannot function even though its dimensions pass.

**C3 · Resolution honesty got slightly worse in one place, not better.** Per-pixel gradient energy
against the native 1024 master (3.885):

| File | src scale | gradient x | Read |
|---|---|---|---|
| `library-capsule-600x900` | 0.59× | 3.742 | native, sharp |
| `header-capsule-920x430` | 0.90× | 3.362 | native, sharp |
| `main-capsule-1232x706` | 1.20× | 3.141 | mild upscale |
| `page-background-1438x810` | 1.40× | 2.797 | visible softening |
| `library-hero-3840x1240` | 3.75× | **1.162** | **F5 unchanged — still an upscale** |

Fixing F1 pushed the main capsule *past* the master's native width (1232 > 1024), so it is now a 1.2×
upscale where it used to be a downscale, and the new page background is a 1.4× upscale. Neither is
severe, but both are new soft spots that only disappear with F7 (author masters at target aspect).

### 6.3 Findings that reproduce unchanged

- **F4** — hero safe area, re-run byte-for-byte identical to §2: Auren x 2253–3126 / y 32–1239,
  **3.4% 2-D overlap**, head band y 32–394 entirely above the band's top edge (y 430) → `FACE WILL CROP`.
  Expected: `6376138` did not touch the hero.
- **F5** — 1.162 vs 3.885 gradient, unchanged.
- **F6** — all reserved logo zones still **BUSY**; the two new assets inherit it (vertical capsule top
  25%: std 55.5 / edgeE 10.77 · small capsule whole frame: std 66.4).
- **F8** — the crops still amputate and still hard-cut the cloak on the right edge: dark mass occupies
  **91.5%** of the header's right edge column, **94.3%** of the small capsule's (the worst in the set —
  the tightest band is also the one that runs off frame hardest), 66.3% main, 65.6% page background.
- **F9/F10** — palette law unchanged on the new files: warm 98.4–99.8%, teal 0.001–0.028%
  (main capsule 0.026%, still ~2 orders below the ≥0.3% target), crushed-below-L12 5.14–13.76%
  (`library-hero` 13.76%, `main` 10.44%, `page-background` 10.41%) against the <3% target.
  Highlights remain clean at 0.00% blown everywhere — P4 still holds.

### 6.4 Verdict of the confirmation pass

**Dimensionally the set is now correct and I would sign off on `6376138` for what it claims to be — a
dimensions-only fix.** Scorecard moves **5 PASS · 10 FIX → 7 PASS · 9 FIX**, from exactly two moves:
**F1 closes** (FIX → PASS), and **C1 pipeline honesty** (maxdiff = 0) enters as a genuinely new PASS
that wasn't on the original 15-item board. "Aspect fidelity" is deliberately **not** a third pass —
it is F1 measured a second way, and counting it would inflate the board (see the correction in §6.1;
the real source→slot distortion is 0.033–0.202%, not 0.000%). But the set is still **not submittable**: two
required slots are empty, no asset carries a logotype, and the hero fails the safe area. The order in
§4 is unchanged — F2 (logo) first, because C1 proves every asset is still waiting on it.

---

*Reviewed by Flamingo (Design). Layer 2 — Gate 3 after-frames vs `docs/look-acceptance-rubric.md` —
is armed and waiting on `docs/assets/gate3/*.png`; as of this writing the runlogs are still `[DryRun —
exe not launched]` and no frames exist.*
