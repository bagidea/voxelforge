# Shot queue — Flamingo, 2026-08-19 (for Poppy: build + render)

Two lanes were assigned: **god rays** and **water**. Water got new code; god rays
did not, and the reason is in §2. **Both still need frames** — this lane did not
run `cargo`, so nothing below has been seen.

Build once, shoot everything out of that ONE binary. Every A/B here is an env
swap on a single build, never two builds, because a rebuilt binary is not a
controlled comparison in this repo (see `commit-time-is-not-build-time`).

---

## 1 · Water (new: `assets/shaders/water.wgsl`, `client/src/water.rs`)

Map must have real water. `river_sunset` has 2011 water blocks and renders them
(`skipped=0`), so it is the map of record; `beach_dusk` gained its sea in
`a7dc0d7`.

| # | env | what it proves |
|---|-----|----------------|
| W0 | `VOXELFORGE_WATER=off` | **BEFORE.** The shipped `StandardMaterial` water — flat, still, `alpha: 0.62`. |
| W1 | *(nothing set)* | **AFTER.** All three terms: moving waves + Fresnel sky + depth grade. |
| W2 | `VOXELFORGE_WATER_WAVE=0` | Isolates Fresnel/glint from the waves — a flat mirror that still takes the sky. If W1 and W2 look the same, term 1 is not reaching the frame. |
| W3 | `VOXELFORGE_WATER_DEPTH=0.5` | Forces the depth grade hard: shoreline should go clear, channel dark. If W1 and W3 look the same, term 3 is not reaching the frame (check a `DepthPrepass` is on the camera). |

Same camera, same hour, same tier for all four. Console must print
`WATER extended material built (N face slot(s) so far) …` — if that line never
appears, the swap never ran and W1 is secretly W0.

**Expected failure mode to watch for:** amplitude is deliberately low (0.055).
If the water reads as *sparkly noise* rather than a swell, that is the exact
trap this project has hit before — a contrast score goes up, the picture gets
worse. Judge the frame.

---

## 2 · God rays — no new code, and here is the evidence for that call

The machinery is already shipped and reachable. Verified by reading, not assumed:

* `look.rs:2215` registers `play_fog_volume` in `Update` with **no `run_if`**.
* `play_fog_density()` defaults to `Some(0.030)` — only `VOXELFORGE_LOOK_VFOG=off`
  disables it.
* `VolumetricFog` is inserted on the camera at **High** (the normal tier) and
  Ultra; `VolumetricLight` goes on the sun in the same tiers (`look.rs:3039`).

So the earlier note in office memory — *"god rays never render in play (no
FogVolume)"* — is **stale**; A2 closed it. Writing another shader on top of a
working ray-march would move a number without moving the picture.

The real remaining gap is **occluders**: a volumetric shaft only exists where
something blocks the sun. That is scene work, not shader work. But that claim is
also unproven until someone shoots it, so:

| # | env | what it proves |
|---|-----|----------------|
| G0 | `VOXELFORGE_LOOK_VFOG=off` | **BEFORE.** The ray-march running in a vacuum. |
| G1 | *(nothing set)* | **AFTER.** Same camera, medium present. |

**Camera choice is the whole experiment.** Point it so the sun sits *behind*
geometry — a treeline, a ridge, a building edge — within ~40° of the view
direction. A camera with clear sky between the eye and the sun has nothing to
cast a shaft and will show G0 ≈ G1 no matter how good the code is; that is a bad
camera, not a bad feature. Tier must be High or Ultra (Low/Medium carry no
`VolumetricLight`).

Read the result as one of three, and say which:

1. **G1 has readable shafts** → item 1 is done; the memory note was just stale.
2. **G1 ≈ G0 at a camera that DOES have the sun behind a treeline** → the
   medium/scattering needs tuning, and that is a real code task.
3. **No camera in the shipped maps can put the sun behind geometry** → the gap
   is world-building, and it belongs to the scene lane, not to look.

Do not report "god rays are fine" without shooting G0/G1. That is the hole this
document exists to close.

---

## 3 · Compile risk, stated plainly

Nothing here has been through `naga` or `rustc` in-crate — the build is Poppy's
queue slot and this lane was fenced off `cargo`. What HAS been checked:

* Every Bevy import verified against the real 0.19 sources in the cargo registry
  (`bevy::shader::ShaderRef`, `AsBindGroup` in `bevy_render`, `ExtendedMaterial` /
  `MaterialExtension` in `bevy_pbr`, `MATERIAL_BIND_GROUP`, `prepass_utils::prepass_depth`,
  `depth_ndc_to_view_z`, `view.world_position`, `globals.time`).
* The Rust↔WGSL uniform ABI is gated by a test that PARSES the shader
  (`water::tests::the_wgsl_uniform_mirrors_the_rust_struct`), and that gate was
  proven to go red on a planted field rename — run it without cargo via
  `rustc --test _flamingo_wgsl_abi_check.rs && ./_flamingo_wgsl_abi_check`.

If the WGSL fails to compile, the message will name a line in `water.wgsl`;
paste it back and this lane fixes it rather than guessing a second time.
