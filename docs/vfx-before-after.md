# VFX overhaul — before/after (Kevin, VFX lane)

Commits on `poppy/native-only`:

- `e9b5cd4` `feat(vfx): kevin VFX overhaul — impact/hit-flash/dust/trail` (+313 / −1)
- `7c3768e` `feat(vfx): add isolated Trail showcase beat` (+7 / −2)

Rendered through the isolated proof bin `voxelforge_vfx_proof`
(`client/src/vfx_proof_main.rs`), which `#[path]`-includes `vfx.rs` verbatim and
links no other module — so the pairs cannot be blocked by `main_menu.rs` /
`combat.rs` breakage from other lanes.

## What changed, and the numbers

### 1. Impact — blood + soft dust (the "damage landed" read)
`on_impact`, non-parry hits only:

- **Blood droplets** (new `blood` material, `srgb(0.55, 0.04, 0.05)` + a small
  vital emissive `rgb(0.9, 0.05, 0.05)`, roughness `0.35` so it reads wet):
  `drops = 6·power + 4`, thrown along the hit vector at `2.0–5.6 m/s` with
  `0.6–1.8 m/s` lift, gravity `18.0` (heavy fluid falls, it does not drift),
  drag `0.6`, TTL `0.45–0.85 s`. The CEO's pass: *blood must read clearly on
  target* — against the husk's grey armour the wet red is the hit telegraph.
- **Soft dust cloud** (new `smoke` material, `srgba(0.45, 0.43, 0.42, 0.18)`,
  `AlphaMode::Blend`, `unlit`): `puffs = 4·power + 3`, slow `0.7–2.0 m/s`,
  gravity `−0.3` (dust hangs then disperses), drag `2.6`, TTL `0.5–0.9 s`.
  Fast chips read "broke something"; a slow large puff reads "hit something
  heavy" — together the impact has mass, not just fireworks.

### 2. Hit-flash / trail / foot-dust — Bevy 0.19 visibility fix
Every `Mesh3d` spawn in the effect layer now carries an explicit
`Visibility::default()`. Before this, the impact flash shell, the parry ring,
the weapon-trail segments, footstep dust and embers spawned **without**
`Visibility` and did not render in Bevy 0.19 (the whole codebase — `hero.rs`,
`look.rs`, `characters.rs`, … — adds it explicitly; `vfx.rs` was the odd one
out). This is not a tuning change; it is "the effect draws at all".

### 3. Dust — ambient motes keep the scene alive
New `ambient_dust` system (wired into the plugin's `Update` set): one emitter
per active `Camera3d`, spawning a mote every `0.14 s` in a shell `1.0–6.0 m`
in front of the lens. Motes are `0.02–0.05 m` cubes of `air_dust`
(`srgba(0.74, 0.71, 0.65, 0.16)`, blend), drift at ~`0.12 m/s`, gravity `0`,
drag `0.15`, TTL `2.2–4.0 s`. Tuned to be faint texture, never a cloud that
draws the eye.

### 4. Campfire — smoke + sparks (was embers only)
`campfire_pulse`:
- **Smoke puffs** (`SMOKE_CHANCE 0.30` per ember beat): grey `smoke` puffs
  `0.12–0.30 m`, rising `0.8–1.5 m/s`, gravity `−0.6`, TTL `1.4–2.4 s`.
- **Spark pops** (`SPARK_CHANCE 0.06`): a bright chip that jumps clear,
  `2.0–3.6 m/s` up, gravity `4.0`, TTL `0.25–0.5 s`.

The fire now reads as breathing rather than a shower of sparks.

### 5. Death dissolve — blood burst + lingering pool
`on_unravel`, in addition to the existing ash dissolve:
- **Burst**: `26` droplets, `0.03–0.08 m`, `1.2–3.4 m/s` up, gravity `16`,
  drag `0.7`, TTL `0.5–0.9 s`, staggered `0–0.12 s`.
- **Pool** (`blood_pool`, `srgb(0.30, 0.02, 0.03)`, roughness `0.9`): a flat
  disc `(1.7·half.x, 0.03, 1.7·half.z)` at the feet, TTL `5.0 s`, hold `0.7 s`
  then it dries (shrinks). The dissolve carries the form away; the blood is
  what stays — the "it is dead" punctuation.

## The pairs

Rendered from `vfx::setup_showcase` (side-on fight framing, 18° key light per
`look-bible.md` §2, fixed 1/60 s clock, grab at frame 192 = 3.2 s). Each pair
is the **same beat, same timestamps, same pose** — the only difference is the
effect layer — so what the eye picks up between the two is the VFX and nothing
else.

| Pair | Before (emitters muted) | After (emitters live) |
|---|---|---|
| impact | `docs/assets/vfx-kevin/impact-after-muted.png` | `docs/assets/vfx-kevin/impact-after.png` |
| trail  | `docs/assets/vfx-kevin/trail-after-muted.png`  | `docs/assets/vfx-kevin/trail-after.png` |
| death  | — | `docs/assets/vfx-kevin/dissolve-after.png` |
