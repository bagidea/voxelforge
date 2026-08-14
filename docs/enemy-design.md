# Enemy visual design — Monanisa

CEO brief: the live enemy has to read as a threat, not a box that walks.
This doc is the spec behind `client/src/enemies.rs` — three new silhouettes,
the part-slot system they're built from, and the exact swap points for
whoever wires them into gameplay (`combat.rs`, Rose's lane — not edited here).

## The problem, verified in the source

The only enemy currently spawned in the real game is the Guard Husk
(`client/src/combat.rs::spawn_guard_husk`, "Spawning" section): a torso
`Cuboid(0.9, 1.4, 0.6)`, a head `Cuboid(0.55, 0.55, 0.55)`, and an arm
`Cuboid(0.28, 1.0, 0.28)`, all flat grey-blue (`#525763` armour /
`#2E303D` head), no eyes, no asymmetry, no material detail. It is spawned
from `scene.rs` (`combat::spawn_guard_husk(...)` call). Exactly the "box that
walks" the brief describes — confirmed by reading the code, not assumed.

## Design rules (all three enemies follow these)

1. **Broken silhouette, always.** Every design carries at least one shape
   that makes the outline read as *wrong* even as a flat black cutout: an arm
   too long, a spine hunched into the wrong curve, a head pushed off-centre.
   Symmetric = safe = not scary; every body here breaks its own mirror line
   somewhere.
2. **Dark and matte, not lit-panel grey.** Base tones sit in the `#14–#2E`
   luminance band with roughness 0.7–0.95 (see palette below) — they eat
   light instead of reflecting it, so they read as a shape cut out of the
   environment rather than a prop dropped into it.
3. **Eyes (and only eyes/cracks) glow.** Emissive is spent on one thing: the
   read-point that survives in the dark. Nowhere else on the body emits — a
   glowing knee would just be confusing.
4. **Danger has to be legible in silhouette, not just in texture**: horns,
   claws, spikes, and armour damage are all separate geometry, not paint —
   every "this is dangerous" cue is a shape you'd still see as a black
   cutout.
5. **Hard 90° edges only**, `Cuboid` parts, authored in ⅛-block (`VX`) units —
   same grammar `characters.rs` (Flamingo's file) already established for the
   Act I cast, so the whole game keeps one visual language.

## Palette (every value is a literal `Tone` in `enemies.rs`)

| Tone | Hex | Use | Emissive |
|---|---|---|---|
| `Hide` | `#2E2A26` | organic base skin (Reaver, Stalker) | — |
| `HideDark` | `#1C1916` | crevice/shadow hide | — |
| `Bone` | `#C9BFA0` | claws, spikes, exposed bone | — |
| `Rag` | `#1F1A16` | torn cloth wraps | — |
| `Iron` | `#23262B` | Sentinel armour plate | — |
| `IronDark` | `#14161A` | soot/damage iron | — |
| `BloodCrack` | `#6E1B14` | corruption cracks | dull red |
| `GhoulGlow` | `#7CFF6E` | Reaver eyes | strong sickly green |
| `SentinelGlow` | `#8FD6FF` | Sentinel eye-slits | cold dead-iron blue |
| `StalkerGlow` | `#FF3B1E` | Stalker eyes | strong predator red |

No colour here overlaps the Act I cast's palette (`characters.rs::Tone`) or
Garren's amber `Crack`. Two separate checks, not one: `BloodCrack` (dried
blood, dark red) keeps the Sentinel's *corruption* distinct from Garren's
amber corruption; `SentinelGlow` (cold blue, revised from an earlier amber
that sat too close to Garren's hue) keeps the Sentinel's *eyes* distinct too
— a warm-amber glow next to a warm-amber crack would have read as "the same
monster" even with the crack colour fixed. The three new eye-glows now span
green / blue / red, so the trio (and Garren) each own a different hue instead
of clustering in the warm-orange band.

## The three designs

### Ghoul Reaver (`reaver`) — low-tier scavenger, ~1.9 blocks
Hunched torso (two slabs, the upper one pitched forward as it rises), an
elongated skull thrust off the hunch with a protruding jaw, irregular
unevenly-spaced spine spikes, sickly green eyes at uneven heights. The read
point: **one grossly overlong right arm** whose forearm reaches past the knee
and ends in three splayed bone claws within half a block of the ground — the
"overlong arm" the brief asked for, built as geometry, not implied by a pose.

### Bone Sentinel (`sentinel`) — the Guard Husk replacement, ~2.7 blocks + weapon
Same combat role as the current husk, genuinely dangerous silhouette: an
oversized horned pauldron on the weapon-side shoulder (horn spike stands a
full block above the helm — same "names it at a distance" trick Garren's
spear uses), the OTHER pauldron small and chipped — asymmetric on purpose.
The helm keeps the "one blank slab, no visor slit" idea (the emptiness reads
as horror) but adds two glowing cold-blue eye-slits at deliberately uneven
heights, so it reads as *watching* rather than merely faceless — a dead,
frozen stare rather than Garren's warm amber. A jagged serrated
cleaver-glaive stands taller than the helm. Dried-blood-red cracks, not
Garren's amber, so the two don't read as one character.

### Thornclaw Stalker (`stalker`) — low predator, ~1.3 blocks (crouched)
Deliberately the shortest and lowest of the three — a predator that stalks
close to the ground. Spine ridge tapers straight into a thin, whip-like
barbed tail. One grossly oversized sickle claw held low and forward (a
mantis-strike read) against a normal-sized off-arm. Close-set red-orange eyes
sit at a predator's gaze height, not a person's. Built as a hunched **biped**
crouch (haunches, not four separate legs) specifically so it stays compatible
with the existing walk/attack rig instead of needing a new quadruped
animation system.

Height ladder across the three (1.3 → 1.9 → 2.7 blocks) reads as an
escalation on purpose: low fast predator → mid scavenger swarm → tall
dangerous elite.

## Part-slot system

`client/src/enemies.rs` mirrors `characters.rs`'s pattern exactly:
`Tone` (palette) → `Bx` (one axis-aligned box, `lo`/`hi` in `VX` units +
`tone`) → a `static &[Bx]` per body → `spawn_enemy(commands, meshes,
materials, kind, feet, yaw) -> Entity`. Same feet-centred origin, same
yaw-0-faces−Z convention as the husk and the cast, so a body can be dropped
into `scene.rs` with the same call shape `spawn_character` already uses.
Adding a fourth enemy later is: one new `Tone` if the palette needs it, one
new `static REAVER2: &[Bx]`, one new `EnemyKind` arm — no other file touched.

## Integration proposal (for Rose / kevin — not applied here)

`combat.rs::spawn_guard_husk` currently builds its 3-cuboid body inline. The
proposed swap is additive only:

```rust
// in combat.rs, spawn_guard_husk — replace the inline torso/head/arm spawn with:
crate::enemies::spawn_enemy(commands, meshes, materials, crate::enemies::EnemyKind::Sentinel, feet, 0.0);
```

`Enemy`/`HuskState`/AI stays exactly as-is — this only swaps what gets
attached as children under the same root entity, so the hit-box story
(`Health`, `Poise`, the telegraph arm marker `HuskArm`) is a separate
conversation with Rose about which child mesh keeps that marker component
(the Sentinel's `weapon arm, forearm/gauntlet` box is the natural `HuskArm`
substitute). Reaver/Stalker becoming new spawnable archetypes (not just a
reskin of the one husk) is an AI-behaviour decision — Rose's call, flagged
here, not decided here.

## Proof

Rendered via the isolated `voxelforge_shot` bin (`client/src/shot_main.rs`,
new `VOXELFORGE_ENEMYSHOT` stage — additive hook, same pattern as the
existing `VOXELFORGE_CHARSHOT`), **not** `VOXELFORGE_PLAY=1`:

- `VOXELFORGE_ENEMYSHOT=before` — the live `combat.rs` husk, reproduced
  byte-for-byte in `enemies::spawn_legacy_guard_husk` (exact same Cuboid
  sizes/colours/offsets as the shipped function — this is a faithful replica
  for comparison, not a new design).
- `VOXELFORGE_ENEMYSHOT=sentinel:day` — the redesigned husk-equivalent, same
  daylight/camera framing as `before`, for a direct apples-to-apples
  silhouette A/B.
- `VOXELFORGE_ENEMYSHOT=sentinel` (night) — the same body, dark/moonlit, to
  prove the eye-glow claim actually reads in the dark.
- `VOXELFORGE_ENEMYSHOT=line` — all three new kinds in one frame, dark/moonlit.

PNG paths + the rendered captures are tracked in the office report to the
CEO, not duplicated here — this doc only states the render recipe.
