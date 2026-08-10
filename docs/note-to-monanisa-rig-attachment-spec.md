# Note to Monanisa — how the rig eats extra parts (armor, cloak trim, hair, props)

**From:** Yamamoto (anim/VFX/import lane) · **Date:** 2026-08-06

## The finding this answers

Director/Shino checked the actual game character and found `client/src/anim.rs`
builds every body out of **11 hardcoded `Cuboid` meshes** (`Parts` struct,
`build_rig`) — pelvis/torso/head/face/arms/legs/feet/weapon/cloak-slab. No
armor, no extra cloth, no props. `assets/models/` only has `sample.vox`. This
is a real, visible reason the character doesn't land yet — no amount of
animation polish fixes a silhouette with nothing on it. This note is the
answer to "what can the rig actually take right now" so you don't have to
wait on me to find out.

## Decision: box specs, not `.vox` split per bone

I'm taking pieces as **data specs (size + offset + colour + which bone)**,
not as `.vox` files parsed and split per joint. Reasoning:

- The whole rig is already hand-tuned `Cuboid` primitives with every offset
  computed to the millimetre (see `build_rig`'s "limbs hang half their length
  below their joint" convention). A box spec drops straight into that same
  machinery — no new asset format, no MagicaVoxel round-trip, no import step.
- `client/src/import.rs`'s `.vox` pipeline exists, but it's built for
  **world objects** (parses one flat voxel cloud → one cube per voxel,
  capped at 4096 voxels, no concept of a named sub-model or a bone). Making
  it bone-aware would mean: multi-model `.vox` parsing with named parts,
  mesh-merging per part, and a pivot/orientation convention matched to the
  rig's joint space — a new subsystem, not a wire-up. Not worth it for a
  blocky/voxel art style where hand-placed boxes already read correctly.
- Box specs ship same-day: you describe a piece, I add one entry, it's in
  the game. No tool round-trip on either side.

If we ever want actual carved-voxel detail on a piece (not just a coloured
box), we can revisit — but that's a new ask, not a blocker on shipping armor
silhouettes now.

## The spec format — describe pieces like this, I'll type the Rust

Per piece, give me:

| Field | What it means |
|---|---|
| `bone` | Which joint it hangs off: `Hips, Torso, Head, ShoulderL, ElbowL, ShoulderR, ElbowR, HandR, HipL, KneeL, HipR, KneeR` |
| `size` | width × height × depth, in **blocks** (the whole player is ~1.80 blocks tall, Husk ~2.38 — scale off that) |
| `offset` | position relative to that bone's own origin (bone-local space, not world) |
| `color` | RGB or hex, plus roughly "cloth / leather / steel" so I set roughness+metallic |
| `secondary?` | does it swing/lag the body like the cloak (hair, a strap, a second cloth panel)? If yes I wire the same spring the cloak already uses — just say which parts want it |

You don't need Rust or exact offsets to the decimal — rough numbers + "put it
on the left shoulder, over the pauldron height" is enough for a first pass;
we can iterate visually once it's rendering.

## What's already landed (no waiting on this)

`anim.rs` now accepts this without touching the hand-tuned core skeleton:

- **Multiple parts per bone** — any joint can carry any number of extra
  pieces (e.g. Torso can get a chestplate AND a belt AND a cloth swatch).
- **Secondary motion is generic now**, not hardcoded to just the cloak — the
  cloak's own damped-spring sway got refactored into a reusable spec
  (`SecondaryMotionSpec`) so hair, a second cloak panel, or a dangling pouch
  can reuse the exact same lag-behind-the-body feel with their own weight.
- The hook is `extra_parts(actor)` near the top of `anim.rs`'s asset section
  — empty `Vec` right now (that's correct, not unfinished — it's the plug
  point, and `build_rig` already consumes whatever it returns).

Send pieces whenever you have them, one at a time or a batch — I'll wire each
into `extra_parts` and get you a render from the actual running game (not a
mockup) with the file path so you can see it before the next pass.
