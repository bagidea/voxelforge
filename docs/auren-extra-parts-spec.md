# Auren in-game asset spec — hair + belt pouch

**For:** Yamamoto (owner of `client/src/anim.rs` / `client/src/import.rs`, per `docs/LANES.md`)
**From:** Monanisa
**Status:** DRAFT — sizes/offsets reasoned from `Dims::of(Actor::Player)` and the concept art,
never compiled or seen in-engine. Treat every number as a starting point to eyeball-tune, not a
locked spec — flag anything that looks wrong and I'll re-pass it.

## Why this file exists

CEO asked for Auren's concept art (cloak asymmetry / hairstyle / belt-pouch) turned into something
the game actually loads, and was explicit that the *format* is your call, not mine — I shouldn't
guess. I read `anim.rs` first and found you'd already built the exact hook this needs:
`extra_parts()` (anim.rs:540, `Vec<PartSpec>`, currently `Vec::new()`) plus its doc comment at
anim.rs:520-539, which names "armor plates, pouches, a second cloak panel, hair" as exactly what
it's for and gives the literal struct-literal syntax to add one. So I've also left an A/B/C
question on the shared notes board in case you'd rather I go a different route (`.vox` per bone,
or glTF via `ModelCatalog`) — but this doc assumes `extra_parts()` is the answer, since it's
already built, documented, and named for this exact job. If you'd rather I target `.vox`/glTF
instead, ignore this file and say so on the board.

I have **not** touched `anim.rs` — everything below is a block to paste, not a diff.

## What to paste

Replace `extra_parts`'s body (anim.rs:540-542) with:

```rust
fn extra_parts(actor: Actor) -> Vec<PartSpec> {
    match actor {
        Actor::Player => vec![
            // --- Hair: tousled voxel-cluster mass, NOT a flat slab (character-bible
            // §1 fix #2 — see docs/assets/characters/auren-hero-concept.png and the
            // before/after at auren-hero-concept-before-after-2026-08-06-hairfix.png).
            // 4 overlapping boxes on BoneName::Head so it reads as clustered volume
            // instead of one brick; deliberately asymmetric L/R (bigger lump stage
            // right) per the CEO's "asymmetry that makes the silhouette read" note.
            // Offsets are in the same head-local frame as the existing head cube
            // (head_off=0.18) and face plate (face_y=0.20, face_z=-0.16) two lines
            // up in `Dims::of` — local -Z is forward/face side, +Z is the back of
            // the skull.
            PartSpec {
                bone: BoneName::Head,
                size: Vec3::new(0.32, 0.16, 0.30),   // crown mass
                offset: Vec3::new(0.0, 0.36, 0.03),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.165, 0.106, 0.071),  // #2A1B12, character-bible §1
                roughness: 0.85,
                metallic: 0.0,
                secondary: None,
            },
            PartSpec {
                bone: BoneName::Head,
                size: Vec3::new(0.28, 0.14, 0.22),   // nape/back mass, lower + further back
                offset: Vec3::new(0.0, 0.24, 0.15),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.165, 0.106, 0.071),
                roughness: 0.85,
                metallic: 0.0,
                secondary: None,
            },
            PartSpec {
                bone: BoneName::Head,
                size: Vec3::new(0.09, 0.10, 0.12),   // stage-left clump, smaller
                offset: Vec3::new(-0.16, 0.28, 0.02),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.165, 0.106, 0.071),
                roughness: 0.85,
                metallic: 0.0,
                secondary: None,
            },
            PartSpec {
                bone: BoneName::Head,
                size: Vec3::new(0.13, 0.14, 0.16),   // stage-right clump, bigger (asymmetry)
                offset: Vec3::new(0.15, 0.30, 0.00),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.165, 0.106, 0.071),  // #2A1B12, character-bible §1 — same as the other 3 hair chunks
                roughness: 0.85,
                metallic: 0.0,
                secondary: None,
            },
            // --- Belt ember-pouch (character-bible §1 palette: housing #C88A4A,
            // glow #F4B860 -> #FFD98A). Hangs on ONE hip only (not centred on the
            // buckle) — that off-centre placement is part of the asymmetry read too.
            // NOT wired to true emissive/bloom yet — see the open note below.
            PartSpec {
                bone: BoneName::Hips,
                size: Vec3::new(0.10, 0.12, 0.08),   // housing
                offset: Vec3::new(0.14, -0.10, -0.10),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(0.78, 0.54, 0.29),  // #C88A4A
                roughness: 0.55,
                metallic: 0.1,
                secondary: None,
            },
            PartSpec {
                bone: BoneName::Hips,
                size: Vec3::new(0.05, 0.06, 0.03),   // inset "coal" face
                offset: Vec3::new(0.14, -0.10, -0.135),
                mesh_offset: Vec3::ZERO,
                color: Color::srgb(1.0, 0.85, 0.54),  // #FFD98A
                roughness: 0.3,
                metallic: 0.0,
                secondary: None,
            },
        ],
        Actor::Husk => Vec::new(),
    }
}
```

That also means changing the signature from `fn extra_parts(_actor: Actor)` to
`fn extra_parts(actor: Actor)` — the parameter stops being unused.

## Open notes — your call, not mine

1. **Pouch doesn't actually glow.** `PartSpec`/`ExtraPart` only carry
   `color`/`roughness`/`metallic` — no `emissive`. `vfx.rs` already has a reusable "glowing coal"
   emissive pattern (e.g. `vfx.rs:376`, `scene.rs:646`'s campfire flame) and `voxel.rs:87` shows
   the `LinearRgba` emissive field this engine already uses elsewhere. Two ways to get a real glow:
   add an `Option<LinearRgba>` emissive field to `PartSpec`/`ExtraPart` (small, mirrors the
   existing pattern), or spawn a `vfx.rs` glowing-coal effect anchored to the pouch bone instead of
   a plain box. Either is your architecture call — the bright `#FFD98A` box above is a placeholder
   that reads "warm" but won't bloom.
2. **Cloak-notch asymmetry — deferred, not solved here.** The CEO's brief also named
   "asymmetry" as its own missing thing. I read that as satisfied by the hair/pouch being
   deliberately off-centre above, plus the cloak already being one-shoulder-only (character-bible
   §1). If what's actually wanted is the cloak's own silhouette reading torn/notched rather than a
   clean rectangle, that's a second cloth panel with its own `SecondaryMotionSpec` (reusing
   `CLOAK_MOTION` at anim.rs:192 is the safe starting point) — I didn't spec that here because it's
   physics I can't eyeball-tune without a compile+in-engine pass, and I'd rather flag it than hand
   you numbers I'm not confident in. Say the word and I'll take a pass at it.
3. **Every offset above is reasoned, not measured in-engine** — I don't have a build loop. Please
   sanity-check visually once it's in (mainly: does the hair clip through the face plate at
   `face_z=-0.16`, does the pouch clip the thigh on a walk cycle) and tell me what to redraw if the
   silhouette doesn't read right; I'll iterate fast off a screenshot.
4. **Color fix (2026-08-06, post-review) — stage-right hair clump.** This spec originally had that
   4th `PartSpec` at `Color::srgb(0.17, 0.11, 0.08)`, which converts to `#2B1C14` — a different hex
   from the `#2A1B12` character-bible.md §1 mandates as *the* hair color (bible names one hex, no
   second tone). I'd meant it as a same-family highlight to help the bigger lump separate visually,
   but never got that documented/approved as a deliberate second tone, so it doesn't pass a strict
   hex check against the bible. Corrected above to `Color::srgb(0.165, 0.106, 0.071)` (`#2A1B12`,
   exact match, same as the other 3 chunks). **This doc is already fixed; `anim.rs:588` still has the
   old `0.17, 0.11, 0.08` value** — since that file is your lane and I was told not to touch it
   while builds are running, the one-line fix needed there is:
   `color: Color::srgb(0.17, 0.11, 0.08),` → `color: Color::srgb(0.165, 0.106, 0.071),  // #2A1B12`

## References

- `docs/character-bible.md` §1 (Auren spec + palette, hair hex added there)
- `docs/assets/characters/auren-hero-concept.png` (current approved concept — the target this
  should look like once it's boxes instead of a render)
- `client/src/anim.rs:520-542` (`extra_parts` + its doc comment — the format this follows)
