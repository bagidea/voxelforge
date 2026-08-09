# A6 — hero idle silhouette + cool accent, implement-ready spec

**For:** Yamamoto (owner of `client/src/anim.rs`, per `docs/LANES.md`)
**From:** Monanisa (Design)
**Date:** 2026-08-10
**Scope:** spec only, no code. I have not edited `client/src/anim.rs` — verified
clean (`git status --short client/src/anim.rs` → no output) before writing this.

## Read this first — the 17.9 number is 5 days stale

The brief for this doc cited silhouette complexity **17.9** (empty box = 16) as
today's number. That's the `docs/art-order-2026-08-09-composition.md` measurement
from a frame shot **2026-08-05**, before the cloak/hair/pouch existed. I re-shot a
real gate3 boot frame on the *current* binary today and re-measured with the same
formula (`grade_character.py`, isoperimetric P²/A):

| pose | today, real frame | target |
|---|---|---|
| **boot / idle** | **66.20** | ≥ 40 |

Full method + caveats: `docs/VERDICT-a6-composition-2026-08-10.md`. **① is already
closed** — the asymmetric mass below shipped in your own `be8333a`
(2026-08-06) and clears the bar by 65%. Nothing new needs adding for shape; what's
below documents what's already there so this file stands as the complete spec
Director asked for, and confirms it's real, not assumed.

## ① Asymmetric elements at idle (already in `extra_parts(Actor::Player)`)

Three pieces, all torso/head-local, all deliberately off-centre — that asymmetry
*is* the fix, not a side effect:

| piece | bone | size (blocks, W×H×D) | offset (bone-local) | hangs where |
|---|---|---|---|---|
| Half-cloak slab | cloak anchor (shoulder) | 0.30 × 0.62 × 0.05 | `cloak_anchor` base pos, spring-lagged | Draped from one shoulder down the back — the single biggest silhouette break, swings with `CLOAK_MOTION` damped spring so it reads as cloth, not a rigid slab |
| Hair crown mass | Head | 0.32 × 0.16 × 0.30 | (0.0, 0.36, 0.03) | Top of skull, forward-biased |
| Hair nape mass | Head | 0.28 × 0.14 × 0.22 | (0.0, 0.24, 0.15) | Lower, further back than the crown |
| Hair clump, stage-left (small) | Head | 0.09 × 0.10 × 0.12 | (−0.16, 0.28, 0.02) | Smaller lump, left |
| Hair clump, stage-right (big) | Head | 0.13 × 0.14 × 0.16 | (0.15, 0.30, 0.00) | Bigger lump, right — the L/R size mismatch is the point |
| Ember-pouch housing | Hips | 0.10 × 0.12 × 0.08 | (0.14, −0.10, −0.10) | One hip only, off the belt buckle centreline |
| Ember-pouch coal face | Hips | 0.05 × 0.06 × 0.03 | (0.14, −0.10, −0.135) | Inset on the housing |

Nothing here needs to change. If a future gate re-measures below 40 on a clean
(non-A5-occluded) frame, add mass by the same pattern — off-centre, on
head/shoulder/hip, never centred — not by resizing the core 11-piece body.

## ② Cool accent — exact value, patch applied + shot, still OPEN

> **Update, later same day:** Yamamoto applied this patch (`anim.rs` commit `90ace55`),
> it built, and got a real-frame reshoot — `docs/VERDICT-a6-teal-accent-2026-08-10.md`.
> Result: **still OPEN**. Hue delta measures 1.1° (worse-looking than, but statistically
> the same as, the 3.3° baseline below — not an improvement), and a whole-frame pixel
> scan for the clasp's own hue band finds **0 matching pixels** in either boot or combat
> frames. Root cause: the gem's *z*-offset (only 1.5cm past the anchor) sits behind the
> cloak's own rendered drape surface at that shoulder point, so it's fully occluded from
> the third-person camera — see §③ below, corrected.

Look-bible's own reserved slot, unused until now: `docs/look-bible.md:132`,
**Accent 2 (cool)**, `#4FC9D6` teal, cap ≤15% of frame (this piece measures
~0.02% at gate3 boot distance — nowhere near the cap).

Measured need: hero hue 30.2° vs background hue 33.4° — **3.3° apart**, i.e. the
whole rig (cloth `#6B4A2E`, trim `#573414`, skin `#D69E78`, steel `#B9A98C`) is one
warm hue family and separates from the background by brightness only. Full repro
in `docs/VERDICT-a6-composition-2026-08-10.md` §Item 2.

**Piece:** cloak-clasp brooch, at the exact point the cloak fastens over the
shoulder (reuses `cloak_anchor`'s own `base_pos` formula: `sh_x*0.7, sh_y*0.95,
0.08`, torso-local) — the one point on Auren's back that's always camera-facing
in third-person (boom sits behind the player; a front chest placement would never
render).

| piece | bone | size (blocks, W×H×D) | offset (torso-local) | color | material |
|---|---|---|---|---|---|
| Bezel housing | Torso | 0.09 × 0.08 × 0.035 | (0.1785, 0.475, 0.08) | `#573414` (reuses existing `trim`, adds 0 material clusters) | leather, roughness 0.5, metallic 0.15 |
| Gem inset | Torso | 0.045 × 0.05 × 0.02 | (0.1785, 0.475, 0.095) | **`#4FC9D6`** — `Color::srgb(0.310, 0.788, 0.839)` | polished, roughness 0.15, metallic 0.0 |

Ready-to-apply patch, diffed clean against current `anim.rs` HEAD, insertion point
right after the ember-pouch entries in `extra_parts(Actor::Player)`:
`docs/patches/monanisa-a6-teal-accent.patch`. Full rationale:
`docs/note-to-yamamoto-teal-accent-2026-08-10.md`.

This piece is now built and shot (see the update above) — everything in ① and the gem
itself compile into the rig, but the gem does not yet render visibly.

## ③ Confirmed: do not fix by scaling the character up

The gap was always shape/hue, not size — ① closed with off-centre mass added at
existing scale (66.20 vs 40 target, same body height as before), and ② is a
~0.05-block gem, not a resize. **No `size` change on the core 11-piece body or on
`Dims::of`'s scale constants is part of this spec.**

**Corrected 2026-08-10, post-reshoot:** this doc originally said that if the clasp
reads too small/subtle at play distance, the fix would be to widen the gem `PartSpec`'s
own `size`. That assumption was wrong and is retracted — the reshoot found the clasp
renders **0 pixels**, not a small/subtle number of them. It isn't a legibility problem,
it's a full-occlusion problem: the gem's *z*-offset sits behind the cloak's own drape
surface at that point. The correct tuning knob is the gem's *z*-position (push it
further out past the cloak's actual depth at the anchor), not its `size`. Full method:
`docs/VERDICT-a6-teal-accent-2026-08-10.md`.

## Acceptance (what to re-measure after applying the patch)

**Re-measured 2026-08-10, post-patch — see `docs/VERDICT-a6-teal-accent-2026-08-10.md`:**

1. `scripts/gate3_shoot.sh` → boot pose → `grade_character.py`: silhouette
   complexity should stay ≥ 40 (it's already 66.20; the clasp is small enough it
   won't move this number meaningfully). **Not re-measured this pass** — item 2 failed
   before this check mattered.
2. Hue check (10-line repro in the VERDICT doc, or `grade_character.py`): hero vs
   background hue delta should read clearly above the current 3.3°. **FAIL** — measured
   1.1°, i.e. no improvement; the clasp renders 0 pixels, so it cannot move the hue
   average at all.
3. Frame coverage of the teal piece stays under look-bible's ~15% cap (currently
   ~0.02% — not a real risk, just confirm the shot). **Moot** while the piece renders
   invisible — nothing to measure coverage of yet.

Item 2 stays open pending the z-offset fix described in §③ above.

— Monanisa
