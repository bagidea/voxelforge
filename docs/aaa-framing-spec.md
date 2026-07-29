# Voxelforge — AAA Composition & Framing Spec

> **Design-only document. No code changes.** Authored by Monanisa (Designer), 2026-07-26.
> Companion to `docs/look-bible.md` and `docs/golden-beauty-shot.md`.
> Purpose: give the engineer a precise brief to stage the camera and plan geometry expansion for both the tight hero and the wide establishing shot.

---

## 0. Reference Images (read side-by-side)

**Golden target (what we're aiming for):**
`docs/assets/golden-beauty-shot-ref.png`

**Current state (hero-charm2-final.png):**
`hero-charm2-final.png`

**Current wide state:**
`docs/assets/wide-hero-final.png`

---

## 1. Visual Gap Analysis — Tight Hero

Comparing `hero-charm2-final.png` directly against `golden-beauty-shot-ref.png`:

### A. Camera Height / Tilt — CRITICAL MISS

| | Golden Ref | charm2 Current | Gap |
|---|---|---|---|
| Camera eye height | Counter-level (~1.4m) | Above island, eye at y≈5.9 | Too high — looking DOWN at island top |
| Vertical tilt | ~5° downward or level | Steep downward (eye↔target Δy ≈ 2.7) | Needs to look ACROSS counter, not down at it |
| Camera angle feel | "Standing at the counter" | "Hovering above the island" | Wrong reading entirely |

**Effect:** We see the FLAT TOP of the island platform. The bowl should read as a bowl (cavity visible from above rim), but at this steep angle we just see a cream-coloured slab. The golden ref camera is level with the counter surface, looking across it — so the bowl's interior (concave shape) reads clearly.

### B. Subject Size & Placement — CRITICAL MISS

| | Golden Ref | charm2 Current | Gap |
|---|---|---|---|
| Bowl frame coverage | ~20–25% of frame area, lower-left | Island fills ~65% of frame, dead center | Far too large, wrong position |
| Rule-of-thirds position | LEFT-LOWER intersection point | Dead center (no thirds respected) | Off both axes |
| Breathing room around subject | Generous (wall, counter, window visible) | Almost none — island bleeds to all edges | Claustrophobic framing |

### C. Background / Negative Space — CRITICAL MISS

| | Golden Ref | charm2 Current | Gap |
|---|---|---|---|
| Window visibility | Left ⅓ of frame, full panel readable | Upper-left corner, heavily cropped | Window must be a composition anchor |
| God rays | 3 visible shafts across center field | Not visible | Major identity element missing |
| Room depth | Counter → mid-space → far-wall readable | No depth — island blocks background | No spatial story |
| Teal accent | Visible mid-right, ~12% frame area | Tiny, hidden under island | Accent must counter-balance subject |

### D. Depth of Field — PARTIAL MISS

| | Golden Ref | charm2 Current | Gap |
|---|---|---|---|
| Foreground focus | Bowl sharp (~f/2.8 feel) | Island/bowl blurred or poorly separated | Wrong focus distance |
| Background blur | Fridge, cabinet fully bokeh | Background flat/undifferentiated | No near/far separation |
| DOF story | "This bowl matters; everything else is dream" | No story told | DOF is the emotional hook |

---

## 2. Rule-of-Thirds Target — Tight Hero

```
┌─────────────────┬─────────────────┬─────────────────┐
│ WINDOW          │  Warm haze /    │  Cabinet top /  │
│ (LEFT UPPER ⅓)  │  dusty light    │  Ceiling edge   │
│ 3 god-ray shafts│  god rays cross │                 │
├─────────────────┼─────────────────┼─────────────────┤
│  Wall in warm   │  Counter        │  FRIDGE         │
│  bounce light   │  surface mid    │  (BG, blurred)  │
│  (no hard edge) │                 │  metallic spec  │
├─────────────────┼─────────────────┼─────────────────┤
│  BOWL ◉         │  Counter        │  TEAL ACCENT    │
│  (LEFT-LOWER ⅓) │  extends right  │  (RIGHT-LOWER)  │
│  SHARP / focus  │  warm wood      │  ≤12% of frame  │
└─────────────────┴─────────────────┴─────────────────┘

Key intersection points:
  ◉  BOWL   = left-lower ⅓ crossing  (SHARP: f-stop ≈ 2.8 equivalent)
  ↗  WINDOW = left-upper ⅓ area      (bright, not clipped)
  ◈  TEAL   = right-lower ⅓ area     (cool accent, not centered)
```

**Horizon line:** Counter surface should sit at approx the **middle horizontal third line**, so that:
- Upper ⅔ of frame = wall + window + god rays + cabinet (vertical storytelling)
- Lower ⅓ of frame = counter surface + subject bowl + teal accent (intimate horizontal storytelling)

---

## 3. AAA Camera Specification — Tight Hero

### Design Intent
Camera at the **counter-side edge of the kitchen**, at **floor-to-lower-body height** (~1.4m), looking **diagonally across the counter toward the window wall** — like a person standing at the far end of the kitchen peering along the counter toward the light.

The bowl should sit at the **near edge** of the counter, slightly left of the camera's center-line.

### Target Parameters

```
# Tight Hero — AAA Target
# (coordinate system: voxel units, floor y≈0, 1 unit = 1m)

Eye position:    offset LEFT of bowl's x  ·  counter height y≈1.4  ·  z pulled back ~5 units from island
Target position: looking TOWARD window wall  ·  y at counter-rim level (≈ bowl lip)  ·  z into the room
Tilt:            very slight downward (~5°) — just enough to see bowl interior
FOV:             48–52°  (narrower = more telephoto compression = better DOF separation)
DOF focal dist:  bowl-to-camera distance (≈ 1.5m from eye)  ·  aperture ~f/2.8 equivalent

Derived from current recipe baseline (eye 7.6, 5.9, -5.2 · target 7.6, 3.2, 6.0):
  → Lower eye Y from 5.9 to ≈ 1.4–2.0  (drop below island top)
  → Shift eye X left by ≈ 2–3 units so bowl is left-of-center
  → Raise target Y to ≈ 3.5–4.0  (look level or very slightly upward toward window)
  → Keep target Z at ≈ 6.0 (toward window wall) — direction is correct
  → Tighten DOF: focal = 1.5, aperture = 2.8–3.2

Suggested starting point to iterate from:
  VOXELFORGE_CAM=5.0,2.0,-5.0,7.6,3.8,6.0,50
  VOXELFORGE_DOF=1.5,2.8
```

### Why this works vs current
- Eye at y≈2.0 → **below the island top** → looking slightly upward at bowl rim → bowl reads as bowl not slab
- Eye X shifted left (5.0 vs 7.6) → bowl shifts to **left-lower ⅓** of frame
- Target Y raised (3.8 vs 3.2) → camera looks ACROSS counter, not steeply down
- DOF focal at 1.5m → **bowl sharp, fridge blurred** — creates depth story

### What to verify after adjustment
1. Window must be visible in LEFT ⅓ of upper frame — if not, shift eye x further left
2. God rays must cross the middle-upper frame area — check volumetric fog angle relative to sun
3. Bowl interior (concave shape) must be visible — adjust tilt until you "see into" the bowl
4. Teal accent must read in lower-right area, not occluded

---

## 4. AAA Camera Specification — Wide Establishing Shot

### Design Intent
A **3/4 establishing view** that reveals the entire kitchen space in one frame. Camera pulled to the rear-right corner of the kitchen (inside the room), shooting diagonally toward the front-left (window wall). This reads as: "welcome to this space, look how it's lit."

**Key constraint:** The camera must stay INSIDE the room footprint — going outside the room creates void around the frustum edges.

### Current Problem
`wide-hero-final.png` uses eye at z=-6.5, which is **outside the current 16×16 room**. At FOV 58°, the frustum clips into void on multiple edges. The light gate passes (G1–G6 PASS) but the geometry is broken: checker floor, blobby island, void borders visible.

### Target Parameters

```
# Wide Establishing — AAA Target (post-geometry-expansion)
# Room must be expanded first (see §5 below)

Eye position:    rear-right CORNER of expanded room  ·  y ≈ 5–6 (eye height standing)
                 This is INSIDE the room, not outside
Target position: looking toward front-left (window wall) — diagonal across the kitchen
FOV:             54–60°  (wider than tight hero to show the space)
DOF:             deep focus (DOF focal >> 6m)  ·  floor + counter + window all crisp
                 — deep focus is CORRECT for an establishing shot (no shallow bokeh)

Derived target:
  VOXELFORGE_CAM=18.0,5.5,2.0,4.0,3.0,16.0,58
  (assumes expanded room: x: 0..22, z: 0..20)
  VOXELFORGE_DOF=8.0,16.0  (keep deep — floor voxels must stay hard-edged, G1)
```

### Reference Composition Grid — Wide

```
┌────────────────────┬────────────────────┬────────────────────┐
│  Ceiling beams /   │  Far wall above    │  Window wall       │
│  right wall top    │  window            │  LEFT anchor       │
│                    │                    │  God rays + light  │
├────────────────────┼────────────────────┼────────────────────┤
│  Right side wall   │  ISLAND / COUNTER  │  Counter surface   │
│  (cabinets)        │  mid-ground        │  + window light    │
│                    │  bowl + teal accent│  spilling          │
├────────────────────┼────────────────────┼────────────────────┤
│  FLOOR             │  FLOOR             │  FLOOR             │
│  wood plank grain  │  checker → MUST    │  lit by window     │
│  warm              │  be replaced       │  patch of gold     │
└────────────────────┴────────────────────┴────────────────────┘

Subject island sits at the CENTER-LEFT intersection — 
readable as a kitchen island (not a wall of geometry)
```

---

## 5. Geometry Expansion — Required for Wide Shot

The current **16×16 block room** is too small for the AAA wide establishing shot. Pulling the camera back to the rear corner within the room while keeping FOV 58° requires the room to accommodate the full frustum without void.

### Void Analysis

At the current wide camera position (eye outside room at z=-6.5, FOV 58°):
- **Rear void**: Camera is 6.5 blocks outside room boundary → solid void behind camera frustum edge
- **Right wall void**: FOV 58° sweeps ~29° each side; at 14-unit depth, horizontal coverage ≈ 15 blocks → clips beyond x=15
- **Ceiling void**: Upper frustum edge clips above the room's ceiling height
- **DOF depth void**: At deep focus, far-field geometry (floor tiles near window wall) must terminate cleanly

### Minimum Geometry Changes Required

```
Priority 1 — MUST for any wide shot:
  ┌─────────────────────────────────────────────────────────┐
  │ A. Expand room footprint from 16×16 to 22×20           │
  │    + 3 blocks LEFT wall (x direction)                  │
  │    + 3 blocks RIGHT wall (x direction)                 │
  │    + 4 blocks DEPTH (z direction, window wall further) │
  │    Result: x: 0..21,  z: 0..19                        │
  │                                                         │
  │ B. Raise ceiling by 3 blocks                           │
  │    (current ceiling clips at wide FOV upper edge)      │
  │                                                         │
  │ C. Relocate camera INSIDE expanded room                │
  │    → camera at room's rear-right corner (x=20, z=2)   │
  │    → no more z=-6.5 outside-room camera                │
  └─────────────────────────────────────────────────────────┘

Priority 2 — Required for matching the golden ref look:
  ┌─────────────────────────────────────────────────────────┐
  │ D. Replace checker floor with wood-plank geometry       │
  │    Current: alternating dark/light checker tiles         │
  │    Target:  directional wood plank blocks, 1-wide       │
  │             running parallel to window wall (z-axis)    │
  │    Note: the Look Bible explicitly calls for wood grain │
  │                                                         │
  │ E. Re-block the island as a proper counter + bowl      │
  │    Current: blobby pale block cluster, no clear shape   │
  │    Target:  - Clean counter slab (L-shape or straight) │
  │             - Ceramic bowl object: square tapered form, │
  │               4-wide rim, inner cavity visible          │
  │               (stepped pyramid in reverse), matte white │
  │             - Teal accent block (1–2 blocks) alongside  │
  │    Rim must have 1-block-wide ledge visible from camera │
  │    height to read as "bowl" not "platform"             │
  └─────────────────────────────────────────────────────────┘

Priority 3 — Nice-to-have for full ref match:
  ┌─────────────────────────────────────────────────────────┐
  │ F. Add ceiling beams (2-3 blocks, dark wood)            │
  │    → establishes scale and depth in wide frame          │
  │                                                         │
  │ G. Add adjacent space hint (pantry door, corridor)      │
  │    → avoids the "box" feel of a 1-room scene            │
  │    → cheap: just 1–2 block door-frame cutout in wall    │
  └─────────────────────────────────────────────────────────┘
```

### Illustrated Room Expansion Plan

```
Before (16×16 top-down):
  ┌────────────────┐  ← window wall (z=16)
  │                │
  │   kitchen      │
  │   island ◯    │
  │                │
  └────────────────┘  ← rear wall (z=0)
  ↑ camera was outside here (z=-6.5) → VOID

After (22×20 top-down):
  ┌──────────────────────┐  ← window wall (z=20, extended +4)
  │                      │
  │   kitchen expanded   │
  │      island ◯        │
  │                      │
  │     rear area   ← CAMERA here (z=2, inside room)
  │     (vestibule) │ cam at x=20, right corner
  └──────────────────────┘
  +3←  original 16  →+3
```

---

## 6. Negative Space Rules (Both Shots)

**Tight hero:** Upper ⅔ of frame should be "air" — wall, haze, god rays, window. The counter occupies the lower ⅓. The bowl fills only the lower-left quadrant. If the bowl is larger than ~25% of total frame area → pull camera back or increase FOV slightly.

**Wide establishing:** The floor should occupy the lower ⅓ and feel *vast* for a voxel kitchen — this establishes scale. The island should not reach the upper-third line. Ceiling must be visible in the top band (~15% of frame height).

**Both shots:** Window must NEVER be cropped. It is the primary light source and the emotional anchor of the composition. If the window is not fully visible → shift camera x leftward until it appears.

---

## 7. Summary — What the Engineer Needs to Do

| Task | Type | Unblocked by | Priority |
|---|---|---|---|
| Lower eye height to ~1.4m for tight hero | Env param | Nothing — env-only | P0 |
| Shift eye X left to put bowl in left ⅓ | Env param | Nothing — env-only | P0 |
| Adjust target Y up for "across counter" look | Env param | Nothing — env-only | P0 |
| Tune DOF for bowl-sharp / fridge-blur | Env param | Camera height fix first | P0 |
| Rebuild bowl as stepped-taper ceramic shape | Geometry | Sign-off from CEO | P1 |
| Replace checker floor with wood planks | Geometry | Sign-off from CEO | P1 |
| Expand room footprint to 22×20 | Geometry | Sign-off from CEO | P1 |
| Raise ceiling by 3 blocks | Geometry | Room expansion | P1 |
| Add ceiling beams | Geometry | Room expansion | P2 |
| Add pantry door hint | Geometry | Room expansion | P2 |

**Geometry changes (P1+) are blocked on CEO/Director sign-off per the Flamingo framing lock note.**
Camera env params (P0) can be iterated immediately — they are env-only and gate-safe.

---

## 8. Acceptance Criteria — AAA Frame

A frame is "AAA composition" when ALL of the following hold:

- [ ] Bowl (hero subject) is at the **left-lower intersection** of the rule-of-thirds grid (±10%)
- [ ] Window is visible and **not cropped** in the upper-left ⅓ of frame
- [ ] Bowl interior (cavity) is **readable** — you can see the bowl is concave, not a flat slab
- [ ] God rays cross the middle-upper ⅓ of frame as readable shafts (at least 2 visible)
- [ ] Background (fridge/cabinet) is **blurred** (DOF separation obvious to untrained eye)
- [ ] Teal accent is visible in right-lower ⅓, occupying 8–15% of frame area
- [ ] No void visible at any frame edge
- [ ] Counter surface sits near the **middle horizontal third line** (± half a block)
- [ ] Frame passes all G1–G6 light gates (existing rubric in `docs/look-acceptance-rubric.md`)

---

*Authored by Monanisa. Counterpart implementation doc: `docs/golden-beauty-shot.md`. Camera param iteration is Poppy's domain — this doc defines the destination, not the path.*
