# A6 — hero silhouette/colour, verdict on a real gate3 frame shot today

**Monanisa (Design) · 2026-08-10 · closes `docs/art-order-2026-08-09-composition.md` A6**

A6 said the boot-pose hero reads as an orange box (silhouette complexity 17.9, same as
an empty box's 16) and separates from the background by brightness only, not hue. That
number is from the gate3 frame set dated **2026-08-05 15:38** — the art-order doc's own
preamble already warns the look stack moved since (`POST_SATURATION` 1.90→1.02) and
orders a re-shoot before judging colour. The same staleness applies here for a second,
bigger reason: `client/src/anim.rs` commit `be8333a` (**2026-08-06 16:24**, one day
AFTER that frame was shot) added the cloak spring-lag, 4 asymmetric hair clumps, and an
off-centre belt ember-pouch — none of which the 08-05 frame could possibly show. A6's
"it's a box" reading and the fix that already shipped for it never met.

This doc re-shoots and re-measures for real, so the verdict rests on today's binary and
today's pixels, not on stacking two more assumptions on top of a frame from before the
fix landed.

## Method

- **Binary:** `target-yamamoto/release/voxelforge.exe` (built 2026-08-09 02:27 — after
  `be8333a` and every other anim.rs silhouette commit; confirmed via
  `git log -S` + file mtimes, not assumed).
- **Colour grade:** `POST_SATURATION` 1.90→1.02 is a real fix (art-order preamble, and
  `client/src/look.rs`'s own dated comment on the constant) but is **uncommitted WIP in
  the working tree** as of today (Rose's lane, `git diff --stat client/src/look.rs`
  shows +94/-11) — not yet in any built exe. Applied it without a rebuild via the
  existing live-sweep hook: `VOXELFORGE_LOOK_GRADE=0.05,1.02,1.12,0.86` (temp/sat/mid/hi
  — the other three match the committed constants byte-for-byte, only `sat` moved).
  This hook has shipped in every build since `26b2ae6` (2026-08-01), so it's safe on
  this binary.
- **Shoot:** `scripts/gate3_shoot.sh`, `GATE3_SKIP_WAIT=1` (the freshness guard compares
  against *every* `client/src/*.rs` mtime including look.rs/settings_menu.rs, which
  both have today's uncommitted WIP touching their mtimes — that WIP is irrelevant to
  this shoot since the grade is applied by env override, not by the stale-vs-fresh
  binary check the guard exists for). All 3 frames: exit 0, `SHOT saved`, no panic.
- **De-HUD + measure:** `_flamingo_dehud2.py` → `_flamingo_char_dist_solve.py` (solves
  the real camera distance, since the boom shortens under collision — see caveat below)
  → `grade_character.py --ref-json _fl_char/ref-auren-hero.json`.
- Frames + charmask overlays + JSON: `docs/assets/gate3-a6-2026-08-10/`.

## ⚠️ Caveat that applies to all 3 frames — A5 contamination

Every gate3 default spawn/demo camera is currently boom-shortened by collision (A5,
open, Kevin's lane): requested boom 6.5 m, solved actual 5.24–5.42 m on all three. Look
at `gate3-after-boot-nohud2-charmask.png` / `-combat-`: the camera is jammed into a wall
corner, not standing back at a normal establishing distance. This is NOT something this
pass introduced — it's the pre-existing A5 bug, present in every current gate3 frame,
boot included. It moves the numbers below (a closer camera reads more silhouette detail
before AA/downsample eats it, so complexity likely reads a little high vs. a normal
establishing shot) but does not fabricate detail that isn't on the rig — the charmask
overlays show real notches from the hair clumps and cloak edge, not noise. **Re-verify
once A5 is fixed**; until then, treat the numbers as directionally strong, not final.

## Item 1 — silhouette complexity: CLOSED

C10 silhouette complexity (`grade_character.py`, isoperimetric P²/A — the exact number
the art-order table also used):

| pose | today (post-cloak, sat=1.02) | art-order 08-05 (pre-cloak) | target |
|---|---|---|---|
| **boot / idle** | **66.20** (89% of concept's 74.50, PASS ≥52.1) | 17.9 (box=16) | ≥40 |
| combat | 109.92 (PASS) | 52.6 | — |
| walk | 34.20 (FAIL) — **A5-occluded frame, see below, not graded** | 212 | — |

Boot is the pose art-order named as the one that matters ("ท่า idle คือท่าที่ผู้เล่นเห็น
บ่อยที่สุด") and it clears the ≥40 bar by 65%, on a real frame shot today. The cloak,
hair asymmetry and belt pouch that `be8333a` already added are doing exactly the job
A6 point 1 asked for — the gap was a stale-measurement gap, not a missing-art gap.
Walk's 34.20 is not a regression: that frame's mask is *also* clipped by the A5 wall
jam (see `gate3-after-walk-nohud2-charmask.png` — the mask bleeds into a stone pillar
face because the boom is pressed against it), so it isn't measuring the character
cleanly and shouldn't be read as a verdict on the walk pose.

## Item 2 — hue separation: STILL OPEN, fix designed + patched, not built

C2 in `grade_character.py` isn't the right axis for this (it grades absolute blue
level against the concept sheet, not hue distance from the background), so I measured
it directly off today's boot frame and mask:

```
hero  RGB (114.5, 70.3, 25.6)   hue 30.2°
bg    RGB (141.3, 91.5, 28.8)   hue 33.4°
hue delta: 3.3°        (ΔL: 21.1 — the value-only separation A6 already flagged)
```

Repro (any `-nohud2.png` + its `-charmask.png` from `grade_character.py`):

```python
diff = np.abs(mask_overlay - frame).sum(axis=2)
char_mask = diff > 20
bg_mask = <bbox ring> & ~char_mask
hue = lambda rgb: colorsys.rgb_to_hsv(*(rgb/255))[0] * 360
```

3.3° apart is the same finding art-order made from the stale frame — the cloak/hair/
pouch additions are all warm-family colours (walnut cloth, leather trim, warm ember
housing/glow), so they add shape without adding hue contrast. **This axis needed a new
piece, not a re-shoot**, and none of the code that ships today has one.

Fix: `docs/note-to-yamamoto-teal-accent-2026-08-10.md` + `docs/patches/monanisa-a6-teal-
accent.patch` — a cloak-clasp brooch (`#4FC9D6`, look-bible's own reserved cool accent)
at the shoulder point the cloak fastens, sized to stay far under the bible's ~15% frame
cap. `anim.rs` is Yamamoto's lane and I don't hold the build lock, so the patch is
written, diffed clean against HEAD, and handed off — **not applied, not built, not
shot**. Item 2 stays open until that lands and gets its own real-frame confirmation.

**Update 2026-08-10, later same day:** the patch landed (`anim.rs` commit `90ace55`),
built, and got its real-frame confirmation — Yamamoto's
`docs/VERDICT-a6-teal-accent-2026-08-10.md`. Result: **still OPEN**, and worse than the
risk flagged above — hue delta measures 1.1° (not an improvement on the 3.3° here), and
a whole-frame pixel scan for the clasp's own hue band finds **0 matching pixels** in
either boot or combat frames. The clasp compiles into the rig but is fully occluded
behind the cloak's own drape from the third-person camera angle. Root cause is a *z*-depth
problem (the gem sits ~1.5cm out from the anchor while the cloak's rendered surface
extends further than that at the shoulder line), not a sizing problem — see that doc for
the full method and the suggested z-offset fix.

## Summary

| | status | evidence |
|---|---|---|
| A6.1 shape reads as character, not box | **CLOSED** | real frame, iso 66.2 ≥ 40, today |
| A6.2 hue-separated from background | **STILL OPEN** | patch built + shot: `docs/VERDICT-a6-teal-accent-2026-08-10.md` — hue delta 1.1° (no improvement), clasp renders 0 px, fully occluded behind the cloak; needs a z-offset fix, not a size change |
| A6.3 don't grow the character | **N/A — untouched** | no size change proposed or made |

— Monanisa
