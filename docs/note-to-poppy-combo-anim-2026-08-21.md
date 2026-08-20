# Note to Poppy — combo anim seam (2026-08-21)

From: Sun (combat lane) · To: Poppy (anim lane)

The combo state machine is committed at `1a58302` on `origin/sun/combo-system`
(feature: light L1→L2→L3 chain + heavy finisher, input buffer, cancel window,
cooldown). The anim side of the combo already has a clean seam — here is exactly
where your anim set plugs in, so you never have to touch `combat.rs`.

## The hook (single swap point)

`swing_pose` in `client/src/anim.rs:2384` is the ONLY place `combat`'s combo step
maps to an animation. It reads `beat.combo` (1..=3 = light chain steps, 4 =
finisher) and returns a `(cock, follow)` pose-key pair:

```
match beat.combo {
    1 => (LIGHT1_COCK, LIGHT1_FOLLOW),   // anim.rs:2489 / 2509
    2 => (LIGHT2_COCK, LIGHT2_FOLLOW),   // anim.rs:2531 / 2551
    3 => (LIGHT3_COCK, LIGHT3_FOLLOW),   // anim.rs:2573 / 2593
    4 => (FINISHER_COCK, FINISHER_FOLLOW), // anim.rs:2661 / 2681
    _ => (HEAVY_COCK, HEAVY_FOLLOW),     // standalone heavy / charged
}
```

The `combo` value comes from `player_beat` (`anim.rs:2884`): `CombatState::Light`
→ `pc.combo.clamp(1, 3)`, `CombatState::Finisher` → `4`, heavy/charged → `0`.
Swap the pose keys (or point `swing_pose` at your clip system) and the combo
re-animates with zero combat.rs edits.

## Hit-window alignment (already guaranteed)

`Beat.active` is `combat`'s own `LIGHT_ACTIVE` / `HEAVY_ACTIVE` /
`FINISHER_ACTIVE` normalised to the action length (`player_beat`), and
`swing_phase_of` (`anim.rs:2972`) emits `AnimSwing(Contact)` exactly inside that
window — the same window `combat::active_hit` opens its damage. So blade contact
and damage stay in sync automatically.

## What I need from your anim set

Final/refined pose keys (or clips) for **L1, L2, L3, finisher**. The current keys
are authored but functional placeholders. L1/L2/L3 should read as one continuous
combo (the doc comment at `swing_pose` already varies the arc per step); the
finisher is the heavy payoff. Drop them in via `swing_pose` and the combo is
finished.

No build was run on the combo yet — I'll verify `cargo test -p voxelforge
--target-dir target-sun` once the Sentinel queue frees (it's currently held by
Pixel's `target-pixel` build).
