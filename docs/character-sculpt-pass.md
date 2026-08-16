# The sculpt pass — 2026-08-14

**The note this pass answers:** *"quality has to be AAA — if it still looks like
boxes glued together, it isn't there yet."* Judged by eye, from real frames,
against what modern Minecraft character mods look like.

Source of truth: `client/src/characters.rs` (bodies, stages, timeline),
`client/src/equipment.rs` (surfaces, geometry, slots, parts, the sculpt lever),
`client/src/char_shot_main.rs` (the isolated bin). Sheets built by
`scripts/_fl_char_quad.sh` → `scripts/_fl_char_sheet.py`.

---

## 1. What was actually wrong

The previous pass fixed **proportions** — 5.5 heads, shoulder:waist 1.97 — and
those numbers are correct and are *not* changed here. The body still looked like
stacked crates anyway, for three reasons that are all form, none of them colour:

| # | problem | why it reads as "boxes" |
|---|---|---|
| 1 | **Zero diagonals.** All 28 body boxes were axis-aligned. | Every silhouette edge was vertical or horizontal. That is the visual grammar of masonry. A shoulder, a hanging arm and a cloak are all *diagonals*, and there were none. |
| 2 | **Zero taper.** One box per limb segment. | A constant-width limb is a pipe. Anatomy is a sequence of widening and narrowing. |
| 3 | **No face.** The head was **one** skin cuboid. | At the shot camera that is ~110 px of blank tan where a viewer looks first. Nothing else on a character costs so little and pays so much. |

## 2. What was built

### 2.1 Rotation — new machinery

`Bx` gained a `rot: [f32; 3]` field (euler XYZ, **degrees**, about the box's own
centre) and a constructor `br(...)` beside the existing `b(...)`. `b` produces
`rot: [0,0,0]`, so every table written before this pass is byte-for-byte
unchanged in behaviour.

`Bx::top()` had to learn about it too — a rolled box reaches higher than its
authored span, so `top()` now returns the rotated-AABB extent rather than
`hi[1] * VX`. Height reports and portrait framing read that value.

Where the rotation went, and why:

| rotation | on | what it buys |
|---|---|---|
| ±14° | deltoids | the shoulder line **slopes** instead of being a lintel |
| ±4–6° | biceps, forearms, wrists, palms | arms hang away from the body |
| ±16° | thumbs | a hand with a thumb reads as a hand |
| ±2° | thighs | a stance, not a pair of columns |
| ±7° yaw | feet | toes point out; the figure stands, it isn't parked |
| 14/−6/−16° | three fringe locks | hair, not a helmet |
| 18–34° | cloak panels | the loudest silhouette change in the file |
| 45° | armour chamfers | a bevel that catches the key light in a hard band no flat plate face can produce |
| 45° about Y | sword blade | a diamond cross-section: two edges and a spine |

### 2.2 Taper

Every limb is a stack that narrows: bicep → elbow → forearm → wrist → palm →
knuckle row → finger block → thumb; thigh → knee → calf → shin → ankle → heel →
mid-foot → dropped toe. The foot's toe box is *lower* than the heel, which is
what gives an arch instead of a shelf.

### 2.3 A face

24 boxes for the head alone: jaw narrowing to a chin, midface, cranium, brow
ridge that **overhangs**, brow hair, eye socket, eye, pupil, nose bridge, nose
with a flushed tip, two nostrils, upper lip, mouth line, lower lip, yawed ears
with hollows. Depth order front-to-back is `nose tip −2.92 · brow −2.80 · pupil
−2.68 · eye −2.62 · socket −2.55`, so the brow genuinely overhangs the socket.

Four new surfaces carry it: `SkinShade`, `SkinFlush`, `EyeWhite`, `EyePupil`.
`SkinShade` is painted-in occlusion and that is deliberate — at this scale a
3-vx-deep eye socket casts no shadow the renderer can resolve, so the value
change has to be authored or the face flattens back into one lit slab.
`EyePupil` is roughness **0.14**, the sharpest highlight anywhere on the body:
it is what decides whether a character is looking at you.

### 2.4 The gear

Every part got a sculpt block, all of it **appended** (see §4). Chamfers on the
plate, rolled pauldron caps tilted outer-edge-down, a raked visor brow and a
swept crest, cloth that falls and flares instead of stacking, boot cuffs, knee
cops with a point, a diamond sword section, faceted pommels and ferrules, leaning
harvest stalks.

One real bug fixed on the way: `TROUSERS_WORK`'s shoes started at `y = 0.9` and
floated a voxel above the sole, leaving bare skin under a fully dressed villager.
Invisible at 720p, obvious the moment the sheet got bigger. They now reach the
ground.

## 3. Anchors that did NOT move

Gear in `equipment.rs` is authored against the body in absolute coordinates. So
the sculpt pass was constrained to keep every anchor the gear hangs off:

* skull top `y = 19.9`
* shoulder span `±5.9`
* waist `±3.0`
* right-hand centre `(4.85, 7.3, −0.35)`
* total height 20.5 vx = 2.56 blocks, 5.5 heads

Hands were re-authored to stay inside `y 6.4–8.1` specifically so a plate
gauntlet still covers a hand completely. A finger poking through a gauntlet makes
a set look *cheaper*, not richer.

## 4. How the before/after is honest

`VOXELFORGE_SCULPT=0` renders the pre-sculpt tables from **the same binary**:
every part truncated to `equipment::presculpt_len(id)` boxes, and Auren's body
swapped for `AUREN_V2`. Every sculpt box was appended, so the old part is exactly
the first *N* entries of the new one — there is no second table to drift.

Two binaries would not do. This lane has been burned by exactly that before: two
exes can differ by driver state, shader-cache warmth, an unrelated commit from
another lane, or nothing at all. Here it is one process, one camera (`AB_CAM`,
baked, not derived from the subject's height), one sun, one frame counter, and
the only difference is how many boxes came out of the table.

`equipment::assert_presculpt_prefixes()` runs at startup on **every** run — after
plates too, not just before ones — and prints `SCULPT_PREFIX …` into the runlog
beside the plate it is vouching for. A stale or missing count is reported rather
than silently rendering an identical pair.

## 5. The stage

```text
VOXELFORGE_CHARSHOT=quad     # bare → villager → adventurer → warplate
VOXELFORGE_SCULPT=0|1        # pre-sculpt tables / the sculpt pass
VOXELFORGE_RES=1440,1080     # frame size; the camera is untouched (fov is vertical)
VOXELFORGE_SHOT=<stem>.png   # → <stem>-1-bare.png … <stem>-4-warplate.png
```

Four plates, **one entity**, four `equip_loadout` calls, no respawn. Every
capture prints the root entity id (`SWAP_CAPTURE n/4 root=…`), so the four frames
can be checked to have come off one body — that check is the evidence, not the
prose above it.

Run the whole thing with:

```bash
scripts/_fl_char_quad.sh 1440,1080
```
