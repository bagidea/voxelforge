# Sky & Aerial-Perspective Research — 2026-08-14

## Status: REVISED — Bevy 0.19 already ships a built-in atmospheric sky

The previous research thread concluded that Voxelforge would have to choose between
(a) an analytic Hosek-Wilkie model or (b) a hand-rolled Hillaire LUT pipeline on Bevy
0.19, and only upgrade to Bevy 0.16+ to get the built-in `Atmosphere`. That conclusion
was wrong.

**Verified fact:** `bevy = "0.19"` in `client/Cargo.toml:35` is enough. The built-in
atmospheric scattering system is already present in the 0.19 registry crates and is
public API. No upgrade, no port, no back-port is required.

## Evidence from the installed registry

| Symbol | Crate | File | Line | What it is |
|---|---|---|---|---|
| `bevy = "0.19"` | `client` | `client/Cargo.toml` | 35 | Dependency pinned to 0.19. |
| `Atmosphere` | `bevy_light-0.19.0` | `src/atmosphere.rs` | 35 | Component: `inner_radius`, `outer_radius`, `ground_albedo`, `medium: Handle<ScatteringMedium>`. |
| `Atmosphere::earth` | `bevy_light-0.19.0` | `src/atmosphere.rs` | 72 | Preset: Earth inner radius 6_360_000 m, outer 6_460_000 m, albedo 0.3. |
| `ScatteringMedium::earth` | `bevy_light-0.19.0` | `src/atmosphere.rs` | 195 | Built-in Earth medium (Rayleigh + Mie + Ozone). |
| `AtmospherePlugin` | `bevy_pbr-0.19.0` | `src/atmosphere/mod.rs` | 96 | Plugin that builds the LUT/render pipeline. |
| `AtmosphereSettings` | `bevy_pbr-0.19.0` | `src/atmosphere/mod.rs` | 289 | Camera component controlling LUT sizes, sample counts, max distance, and render mode. |
| `AtmosphereMode::LookupTexture` | `bevy_pbr-0.19.0` | `src/atmosphere/mod.rs` | 422 | Default: fast LUT-based scattering, best for scenes inside the atmosphere. |
| `AtmosphereMode::Raymarched` | `bevy_pbr-0.19.0` | `src/atmosphere/mod.rs` | 428 | Slower ray-marched mode, sharper volumetric shadows / long-distance views. |

The `Atmosphere` component is defined in `bevy_light` and re-exported at
`bevy_light::Atmosphere` (`bevy_light-0.19.0/src/lib.rs:44`). `bevy_pbr`'s atmosphere
module imports it from `bevy_light` (`bevy_pbr-0.19.0/src/atmosphere/mod.rs:57`), which
confirms the split and the public path.

## Sky-model options on Bevy 0.19

| # | Model | Implementation | Cost | Look | When to use |
|---|---|---|---|---|---|
| 1 | **Built-in Hillaire LUT** | `AtmospherePlugin` + `Atmosphere` + `AtmosphereSettings` on camera | Moderate (small LUTs, compute shaders required) | Physical blue/orange sky, aerial perspective, time-of-day | **Default for Voxelforge.** Replaces both the gradient dome and the current `DistanceFog` haze. |
| 2 | **Built-in Hillaire raymarched** | Same plugin, set `AtmosphereSettings::rendering_method` to `AtmosphereMode::Raymarched` | Higher per frame | Sharper volumetric shadows, better for orbit/cinematic shots | Optional Ultra-tier toggle or specific hero shots. |
| 3 | Hosek-Wilkie analytic | Third-party crate / custom shader | Low | Analytic, less physical, good clear-sky approximation | Only if the built-in LUT is too expensive on Low tier after profiling proves it. |
| 4 | Inner cloud dome + CPU noise texture | Spawn an inner sphere with a noise texture | Very low | Stylised soft clouds, cheap | Add on top of #1 or #3 if cloud detail is needed without Nubis raymarching. |

The built-in system also handles **aerial perspective** (scattering in front of terrain,
similar to distance fog but physically coupled to the sky). This is the path that should
replace the current `DistanceFog` haze.

## Why the current aerial perspective disappears

`client/src/look.rs` still builds haze with `DistanceFog` and `FogFalloff::Linear`:

```rust
// client/src/look.rs:1202-1205
None => FogFalloff::Linear {
    start: HAZE_START,   // 20.0 blocks
    end: HAZE_FULL,      // 150.0 blocks
},
```

`HAZE_FULL` is 150 blocks while the Edhari campsite is only ~55–64 blocks deep. On that
map the ramp reaches full opacity well past the back of the set, so the visible world
sits on the shallow, near-linear beginning of the ramp. The result is a haze that is
present mathematically but visually thin — exactly the “no aerial perspective / no depth”
complaint the grading gates track.

The built-in `Atmosphere` does not use a hand-tuned linear ramp. Its aerial-view LUT is
sampled per view frustum and tied to the planet scale and sun direction, so the horizon
glow and distance desaturation scale with the actual view geometry rather than with a
fixed `start/end` pair.

## Proposed real work order on 0.19 (no upgrade)

1. **Spike the built-in atmosphere in a branch.**
   - Add `AtmospherePlugin` to the app.
   - Spawn one entity with `Atmosphere::earth(handle)` + `GlobalTransform`.
   - Add `AtmosphereSettings` to the gameplay camera.
   - Remove / disable the current `DistanceFog` insert from `base_camera_look()` for the
     test so the two haze systems do not fight.
   - Keep `TonyMcMapface`, `Bloom`, `Exposure`, and the grade stack; only replace the sky
     dome and the haze.

2. **Calibrate planet scale to Voxelforge units.**
   - Default Earth radii are in metres; the world is in blocks. Scale the atmosphere
     entity’s transform so that the ground plane sits at the camera/avatar height and the
     horizon looks right. This is the single tuning parameter that replaces
     `HAZE_START`/`HAZE_FULL`.

3. **Lock the sky/horizon calibration.**
   - The previous dome was `unlit` and was accidentally multiplied by exposure; fix the
     same way as before: separate geometry exposure from the sky/unlit path.
   - Use `VOXELFORGE_LOOK_SKYPROBE=1` (already established) to measure the new dome.

4. **Decide cloud strategy.**
   - If the built-in sky alone is enough for the art target, stop.
   - If soft clouds are required, add option #4 (inner cloud dome + CPU noise texture)
     before considering Nubis raymarching.

5. **Tier the atmosphere.**
   - `AtmosphereMode::LookupTexture` as default for Low/Medium/High.
   - `AtmosphereMode::Raymarched` as the Ultra toggle only if Poppy’s profiling shows it
     is affordable and the sharper volumetric shadows are visible in our scenes.

6. **Re-grade gates before merging.**
   - Run the canonical plates (`grade-vista`, `s1-vista`, `s4-raking`, hero) with the new
     sky and no other changes. The colour axes will move because the sky and haze are now
     one physical system; the signed-off numbers will need to be re-negotiated, not
     matched by hacks.

## References

- `client/Cargo.toml:35` — Bevy 0.19 dependency.
- `C:\Users\BagIdea\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\bevy_light-0.19.0\src\atmosphere.rs:35,72,195` — `Atmosphere` component, Earth preset, Earth medium.
- `C:\Users\BagIdea\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\bevy_light-0.19.0\src\lib.rs:44` — public `Atmosphere` re-export.
- `C:\Users\BagIdea\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\bevy_pbr-0.19.0\src\atmosphere\mod.rs:57,96,289,422,428` — `Atmosphere` import, plugin, settings, render modes.
- `client/src/look.rs:1202-1205` — current `FogFalloff::Linear` default.
