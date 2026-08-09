# A6 fix #2 — Auren needs a cool accent, one PartSpec pair, patch ready to apply

**For:** Yamamoto (owner of `client/src/anim.rs`, per `docs/LANES.md`)
**From:** Monanisa (Design) — art-order-2026-08-09-composition.md item A6
**Status:** Design decided, patch written and verified to diff clean against current
`anim.rs` (HEAD `88930cc`), **not applied** — `anim.rs` is your lane and I don't hold
the build lock. Patch file: `docs/patches/monanisa-a6-teal-accent.patch`.
**Scope check:** no `client/src/` file is modified in the working tree. I edited
`anim.rs`, generated the diff, then `git checkout --` it back to HEAD before writing
this note — verified clean with `git status --short client/src/anim.rs` (no output).

## Why

`docs/art-order-2026-08-09-composition.md` A6 point 2: Auren separates from the
background by brightness only, not hue — "พอฉากมีจุดสว่างอื่น hero ก็หายไปกลืน" (the
moment the scene has another bright spot, the hero disappears into it).

I re-shot a real gate3 boot frame today (current binary + the corrected
`POST_SATURATION=1.02` via `VOXELFORGE_LOOK_GRADE`, see
`docs/VERDICT-a6-composition-2026-08-10.md` for the full method) and measured it
directly, not from the 4-day-old stale reference the art-order doc itself flagged as
unsafe for A7:

```
hero  RGB (114.5, 70.3, 25.6)   hue 30.2°
bg    RGB (141.3, 91.5, 28.8)   hue 33.4°
hue delta: 3.3°
```

Confirmed live, not just a stale-frame artifact: the whole rig (cloth `#6B4A2E`,
trim `#573414`≈, skin `#D69E78`, steel `#B9A98C`) is one hue family by design
(character-bible §1's warm palette), and nothing on the body cuts against it.
`look-bible.md:132` already names the fix and reserves it for exactly this:
**Accent 2 (cool), `#4FC9D6` teal — "ตัดกับอุ่น" — capped at ~10–15% of frame.**

## The fix

One clasp/brooch where the cloak fastens over the shoulder: a leather-trim bezel
housing + a small teal gem inset, same construction pattern as the existing belt
ember-pouch (housing + inset face, two `PartSpec`s). Full diff in
`docs/patches/monanisa-a6-teal-accent.patch`, inserted right after the ember-pouch
entries in `extra_parts(Actor::Player)`, before the closing `],`.

**Why THAT spot and not a front chest pendant.** The play camera is third-person,
boom behind the avatar (`main.rs` `BOOM_DIST`/`OrbitCam`) — the player's BACK faces
the camera essentially always. A chest pendant would sit on the far side and never
render. The cloak's shoulder-fastening point is already the one asymmetric read-point
on the back the eye locks onto (character-bible §1, and `cloak_anchor`'s own doc
comment in `build_rig`), so the accent rides the same read instead of competing with
it. I reused `cloak_anchor`'s exact `base_pos` formula (`sh_x*0.7, sh_y*0.95, 0.08`,
torso-local) so the clasp sits precisely at the fastening point, not eyeballed.

**Material count check.** C11 "material clusters" (in-game vs. concept sheet) reads
5 today, concept sheet is 6 — there is exactly one slot of headroom. The housing
reuses the rig's own `trim` color verbatim (adds 0 clusters); only the gem is new
(adds 1, lands at 6 — matches the concept sheet).

## What I need from you

1. `git apply docs/patches/monanisa-a6-teal-accent.patch` (or hand-paste — it's 33
   lines, one insertion point) whenever your build queue is free — not urgent enough
   to justify grabbing the build lock on its own; bundle it with your next anim.rs
   pass if one's already queued.
2. After it lands and a binary exists: reshoot gate3 boot, de-HUD
   (`_flamingo_dehud2.py`), and re-run the hue check (a 10-line script, see the
   verdict doc's "Repro" section) or `scripts/grade_character.py`. Target: hue delta
   clearly > 3.3° and visibly readable as a cool point in the frame, without pushing
   past look-bible's ~15% frame-coverage cap (this piece is roughly 0.02% of frame at
   gate3 boot distance, so the cap is nowhere close).
3. If the clasp reads too small/subtle at gate3's actual play distance (character is
   only ~90-160px tall raw in the current shortened-boom frames — see the verdict doc
   for why the boom is shortened), the fix is to widen `size` on the gem `PartSpec`,
   not to reposition it — the shoulder point is correct, only tuned by art-order's own
   framing math if I'm wrong about legibility at range.

— Monanisa
