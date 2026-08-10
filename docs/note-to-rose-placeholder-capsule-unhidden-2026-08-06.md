# The legacy avatar capsule is visible in every `--play` session

**Found:** 2026-08-06, during the NOHUD beauty pass (Poppy).
**Binary:** `target-flamingo/release/voxelforge.exe`, 19:53.
**Owner of the fix:** whoever owns `dodge_parry.rs` (the dodge-ghost lane, `31f1d71`).
I did not touch it — the build lane is Rose's this round, and this needs a recompile.

## What it looks like

The player renders as a flat red pill with a dark block on its front, sitting *in front of*
the animated rig. The rig is still there and still animating — its head, sword arm and legs
poke out around the pill. See `_poppy_beauty/final/s2-hero-medium.png` and `s3-husk-clash.png`.

This is not a capture artifact. It happens in any `--play` / `--combat-demo` session, so it
is on screen for a real player too.

## Why

Two systems fight over the same `Visibility`, and the later one wins every frame.

`anim.rs::attach_rigs` hides the placeholder meshes the avatar shipped with, once, when it
dresses the actor in a rig:

```rust
// anim.rs:1096 — "Blank the placeholder meshes BEFORE the rig's own children exist"
for c in children.iter() {
    if let Ok(mut v) = vis_q.get_mut(c) { *v = Visibility::Hidden; }
}
```

`dodge_parry.rs::dodge_ghost_flash` then runs **every frame**, and its restore branch
un-hides anything carrying `PlayerBody` whenever the player is not in i-frames — which is
almost always:

```rust
// dodge_parry.rs:311 — the `else` (not in i-frames) branch
if *vis == Visibility::Hidden {
    *vis = Visibility::Visible;   // <-- also resurrects what attach_rigs hid
}
```

Both placeholder children (`main.rs:842` capsule, `main.rs:848` face block) carry
`PlayerBody`, which is exactly the pair that shows up in frame.

The restore is correct *for the strobe it owns* — it just cannot tell "I hid this three
frames ago" from "the rig lane hid this permanently at spawn".

## Suggested fix (your call, not mine)

Make the restore only undo the strobe's own hiding. Cheapest version: have
`dodge_ghost_flash` remember whether it hid the body this dodge, e.g. a `GhostStrobed`
marker inserted when it strobes and removed on restore, and gate the `Visible` write on it.
A rig-aware guard works too — skip the restore entirely when the actor has `Rigged`.

## Impact on the beauty pass

Shots 1 and 4 (wide / vista) are unaffected — the avatar is a few pixels.
Shots 2 and 3 are the character shots, so they are the ones this spoils. Both are on the
sheet flagged NOT SHIPPABLE.

**s2 (hero medium) really is a 2-minute re-shoot** once the capsule stays hidden — the
capsule is the only thing wrong with the frame, and the framing below still holds.

**s3 (husk clash) is NOT.** Correcting my earlier note: the capsule is only half of what
is wrong with s3. In the frame on disk the husk stands clearly *separated* from the hero
and the sword points down and behind — there is no blade contact, so it does not read as
a clash at all. `VOXELFORGE_ANIM_POSE=attack` parks the hero in an attack pose but nothing
synchronises the husk to it, and the shot lands at a fixed t=3.2s. Fixing the capsule
alone will produce a clean frame of two characters standing near each other, not the
"blades touching" beat the brief asked for. Getting that needs either a held contact pose
for both actors or a way to pin the shot to the impact frame — a gameplay/anim job, not a
camera one. I'd rather flag that than hand it back as a 2-minute re-shoot.

```
# hero medium -- framing still good, only the capsule blocks it
-Name s2-hero-medium -Mode "--play" \
  -Cine "34.2,2.9,28.8, 34.2,2.9,28.8, 32.5,2.0,32.4, 4"

# guard husk clash -- framing is a STARTING POINT, not a solved shot (see above)
-Name s3-husk-clash -Mode "--combat-demo" \
  -Cine "34.9,3.4,31.4, 34.9,3.4,31.4, 33.4,1.7,26.4, 4" \
  -Env "VOXELFORGE_ANIM_POSE=attack"
```

---

## RESOLVED IN SOURCE — 2026-08-07 (Yamamoto), not yet shot

Both blockers above were answered in source. Neither is a capture-time flag: there is
nothing for a shooter to remember to pass.

**1. Capsule.** `dodge_ghost_flash` now takes `Has<Rigged>` and returns early for a rigged
player (`client/src/dodge_parry.rs:284`) — the rig-aware guard suggested above, so the
strobe stops un-hiding what `attach_rigs` hid. Always-on.

**2. Blade contact.** `VOXELFORGE_ANIM_POSE=clash` (`client/src/anim.rs:429`) is a new
value of the existing capture hook. It parks the player at the light-1 contact frame *and*
drives the nearest `Actor::Husk` to its own contact frame via `override_husk_beat()`
(midpoint of the `(0.62, 0.80)` active window). Both actors are held, so the frame no
longer depends on where t=3.2s happens to land — the pin I said this would otherwise need.

The 19:53 `s3-husk-clash.log` on disk already contains **both** `ANIM_RIG_WEAPON spawn
actor=Player` and `actor=Husk`, so the separation in that frame was never a missing husk
rig — it was `=attack` posing one actor. `=clash` is the whole difference.

**Status: wired, not shot.** `scripts/_poppy_shotset.ps1` carries s3-clash in the canonical
set with `VOXELFORGE_ANIM_POSE=clash` and hard-fails the plate if either rig marker is
missing from the runlog. It has NOT been fired: the only exe on disk is still the 19:53
binary (sha `BF344EE7…`), which predates both fixes, and the script's own binary gate
refuses it. Re-shoot goes off the first green build.

```
# superseded by the block below -- kept only to show what shot the frame on disk
-Env "VOXELFORGE_ANIM_POSE=attack"

# current recipe
-Name s3-husk-clash -Mode "--combat-demo" \
  -Cine "34.9,3.4,31.4, 34.9,3.4,31.4, 33.4,1.7,26.4, 4" \
  -Env "VOXELFORGE_ANIM_POSE=clash"
```
