# Voxelforge — Steam Store Art Direction

> Native desktop, Steam distribution (confirmed — see `docs/GAME-VISION.md` non-goals: no web/wasm, Steam only).
> This doc locks the **art direction for the store page**, not the in-game beauty shot. Source of truth for
> mood/palette/characters: `docs/look-bible.md` + `docs/character-bible.md`. No new art direction invented here —
> every choice below is a reapplication of those two docs to a marketing surface.
> Owner: Monanisa (Design). Last updated: 2026-07-31.

---

## 1. The one-line brief

**One key art, one hero, one golden-hour beat — cropped into every capsule Steam needs.**
The store page is the player's first frame of the game. It has to say, in under a second: *cozy voxel
world, real lighting, a soulslike weight underneath.* That's `look-bible.md §0` (voxel geometry × realistic
shader) plus `GAME-VISION.md` pillar 2 (souls weight in a toy world) — both in one image.

---

## 2. Mood

Warm, quiet, held-breath tension — not a genre-standard "hero mid-swing" action pose. Reference: the
golden-beauty-shot kitchen (`docs/assets/golden-beauty-shot-ref.png`) proves the identity reads even in a
*still, empty* scene; the key art should carry that same stillness, just outdoors and with Auren present.

- **Time of day:** golden hour, sun low and raking (15–25° above horizon per look-bible §2) — long soft
  shadows, not noon light.
- **Emotional pitch:** "a traveller has just arrived somewhere that used to be safe." Curiosity and unease
  in the same frame, not spectacle. This is what separates Voxelforge from a generic voxel-sandbox capsule.
- **Not:** dramatic mid-combat action shot, screaming boss face, neon, night/horror lighting. Those break
  the "cozy" pillar even if they'd read louder on a thumbnail.

## 3. Focal point & character placement

**Subject: Auren (hero)** — per `character-bible.md §1`, standing, three-quarter turn, cloak trailing on
one side (the single asymmetric read-point that has to survive thumbnail scale). Sword sheathed or held
low/relaxed, not mid-swing — the tension is atmospheric, not violent.

- **Placement:** off-center, roughly the right-hand third of the frame, facing/looking toward the open
  (left) two-thirds — classic key-art "looking into the space," and it happens to leave the logo lockup
  zone (see §4) unobstructed on the left.
- **Background:** Edhari village ruins silhouette + one god-ray shaft breaking through a gap in the
  structures, atmospheric gold fog softening the far distance (look-bible §1/§2, checklist #5/#6). A
  single small dark silhouette of a Guard Husk, distant and out of focus (DOF-soft), on the opposite side
  of the frame from Auren — reads as "danger is here" without turning this into an action shot.
  Reference for the Husk's silhouette-read: `docs/character-bible.md §2` (bulk + blank visor reads "wrong"
  even at silhouette scale — perfect for a background beat that shouldn't pull focus).
  Reference environment feel: `docs/assets/edhari-village-topdown.png`, `docs/assets/moodboard.png`.
- **Ember beat:** Auren's belt ember pouch (`#F4B860`→`#FFD98A`, character-bible §1) is the one warm
  point-light on the hero itself — small bloom only, echoes the sun without competing with it.

## 4. Negative space for logo/title (non-negotiable — do not generate text into the art)

- **Landscape master** (drives header, main capsule, library hero): reserve the **left third to left-half**
  of the frame as open sky/fog/god-ray — no hard-edged geometry crossing it — so the logo lockup + title
  text can sit there in a separate layer without fighting silhouettes.
- **Portrait master** (drives the library capsule, 2:3): reserve the **top ~25%** as open sky/fog above
  Auren's head for the title lockup, and keep the character's face/cloak-tell inside the safe-crop middle
  band (Steam's library grid thumbnails crop portrait capsules tighter than the full 600×900 — keep the
  read-critical silhouette centered, not edge-to-edge).
- Logo, title text, and any UI chrome (age rating badge, etc.) are **separate layers added after** the
  generated art — per Director's brief, generated images must never contain baked-in text; letterforms
  distort badly in current image-gen and the store team needs to swap title treatments independently of
  the art.

## 5. Palette discipline

Same law as every character/scene doc: **~85% warm walnut→amber, teal capped ~10–15%** (`look-bible.md §4`).
For the key art specifically:
- Sun/god-ray core: `#F4B860`→`#FFD98A`.
- Village wood/stone: `#6B4A2E` walnut, `#3A2716` espresso, `#B9A98C` stone-beige.
- Fog/haze: `#E8D8B8`, thickening toward the horizon.
- One teal accent, small and justified: a single distant emissive block (lore-consistent per
  character-bible §0 — teal = Shaper-origin, so it should read as "something not-quite-right in the
  ruins," not decoration) — capped well under the 15% ceiling, likely under 3% of frame area here since
  this is a village-native scene, not a Shaper-marked character.
  **Verified 2026-07-31:** pixel-sampled both masters after a regeneration pass — landscape master
  ~0.03% teal-hue pixels, portrait master ~0.02% (small, deliberate glow blocks, upper-right ruins in
  both). The first-pass generation had none (~0.003–0.008%, i.e. compression noise, not an actual
  accent) and was re-shot with the accent explicitly called out in the gen prompt. The accent sits high
  in the frame, so it survives the header/main capsule crops (wide vertical bands) but is cropped out of
  the library-hero (thin horizontal band, y330–660) and library-capsule (centered width crop) — that's a
  framing side-effect of where the block happened to land, not a re-introduced defect; §6's crop
  priorities (Auren + god-ray survive every crop) still win over guaranteeing the teal block in every
  capsule.
- Shadow floor: never pure black — `#2A2030` warm brown-violet per look-bible, even in the ruins' dark gaps.

## 6. Capsule crop plan

One landscape master + one portrait master, cropped (not separately generated per size) so the composition
stays consistent across every Steam surface. Sizes below are the ones Director provided 2026-07-31;
**if Sahara's Valve-2026 spec check comes back with different numbers, those numbers win — swap the
crop targets, the art direction in §1–§5 does not change.**

| Asset | Size (px) | Aspect | Source master | Crop notes |
|---|---|---|---|---|
| Library capsule | 600×900 | 2:3 portrait | Portrait master | Full-body Auren, top 25% reserved for logo |
| Header capsule | 460×215 | ~2.14:1 | Landscape master | Horizontal band through the god-ray + Auren's upper body; left half stays clear |
| Main capsule | 616×353 | ~1.75:1 | Landscape master | Same band as header, slightly taller — more fog/environment visible top/bottom |
| Library hero | 3840×1240 | ~3.1:1 ultrawide | Landscape master | Thinnest band — composition must keep Auren + god-ray inside the vertical-center third so an ultrawide crop doesn't clip the hero |

All final files land in `docs/assets/steam/`.

## 7. Deliverables checklist

- [x] This art-direction doc.
- [x] Landscape key art master (no text) — `docs/assets/steam/key-art-landscape-master.png`.
- [x] Portrait key art master (no text) — `docs/assets/steam/key-art-portrait-master.png`.
- [x] Cropped capsule set at the sizes in §6, all generated by `scripts/make_steam_capsules.py`
  (the script now actually reproduces every shipped file byte-for-byte when re-run — verified
  2026-07-31, see fix note below):
  - `docs/assets/steam/library-capsule-600x900.png`
  - `docs/assets/steam/header-capsule-460x215.png`
  - `docs/assets/steam/main-capsule-616x353.png`
  - `docs/assets/steam/library-hero-3840x1240.png` — **note:** this one is a Lanczos upscale of a crop
    from the 1024×1024 landscape master (our image-gen tool tops out at 1024²), so it's soft at full
    size/zoom. Fine as a placeholder for review; if it ships as-is on the store page, re-render the
    master at native 3840px resolution first (or run this file through an upscaler) rather than shipping
    the soft upscale.
- [x] **Fixed 2026-07-31:** the hero-crop call in `make_steam_capsules.py` used to pass
  `target_w=1024, target_h=330` — a no-op resize on an already-1024×330 crop band, which produces a
  1024×330 image, not the shipped 3840×1240 file. That meant re-running the script (as the item below
  instructs) would have silently produced a wrong-sized placeholder instead of the actual hero. The real
  3840×1240 upscale had happened outside the checked-in script and was never captured in it. Confirmed
  the fix reproduces the shipped file exactly: the corrected call
  (`crop_band(landscape, 330, 660, 3840, 1240, ...)`) now does the Lanczos upscale directly and its
  output is byte-identical to a from-scratch re-crop of the master. Re-running the whole script end to
  end regenerates all four capsule files from the two masters with no manual steps.
- [ ] Pending: confirm final px sizes against Sahara's 2026 Valve spec pull before calling this final-final
  (crop script is `scripts/make_steam_capsules.py` — re-run with new target sizes if they change; the
  script is now self-contained and reproducible, see fix note above).

---

*Design: Monanisa. Character/palette source of truth: `docs/character-bible.md`, `docs/look-bible.md`.
Pillars source: `docs/GAME-VISION.md`. This is a store-page marketing surface — it does not gate the
in-game golden-beauty-shot acceptance criteria in `docs/golden-beauty-shot.md`.*
