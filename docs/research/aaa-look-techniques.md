# AAA Look Techniques — Clouds, Sun Disc, God Rays, Water

**Status:** research-only. No renderer code (`look.rs`, `sim`, `voxel.rs`, `maps`, `assets`) was touched.
**Owner of this lane:** the look lane (Poppy) / water lane (Rose). This doc hands them the *techniques*, the *primary sources*, and the *gap vs the boss's ruler* (modern Minecraft shader mods: Complementary / BSL / Iris).
**Method:** every URL below was opened and confirmed reachable (HTTP 200) before it was written down. Where a URL 404'd or did not exist, it is stated as "not found" — nothing is cited from memory. Bevy facts are cited against the actual `0.19` crate source in the local cargo registry (primary), same discipline as `docs/sky-research-2026-08-14.md`.

---

## 0. What `look.rs` already ships — the base to build on

`client/src/look.rs` (3,811 lines) already wears, per camera and per sun:

| System | Where | What it is today |
|---|---|---|
| Physical sky (Bevy 0.19 Hillaire LUT) | `atmosphere_sky()` `look.rs:3411`; `atmos_mode()` `:3368` | `Atmosphere::earth(medium)` + `ScatteringMedium::earth(256,256)` + `AtmosphereSettings { aerial_view_lut_max_distance: HAZE_FULL, rendering_method: lut\|raymarched }`. Shipped default = LUT mode. |
| Gradient sky dome (fallback) | `sky_dome()` `look.rs:3633`; `sky_gradient()` `:3525` | Vertex-coloured UV sphere (`SKY_DOME_RADIUS=640`), unlit. On only when atmosphere is off. |
| God-ray medium | `play_fog_volume()` `look.rs:3776` | `FogVolume { density_factor: 0.030, scattering: 0.55 }` centred on camera. |
| God-ray march | `insert_stack()` `look.rs:2662` | `VolumetricFog` on camera — High `step_count: 32`, Ultra `step_count: 96`. |
| God-ray light | `apply_look_to_sun()` `look.rs:2851` | `VolumetricLight` on the sun (High/Ultra only). |
| Sun + shadows | `apply_look_to_sun()`; `Hour` `look.rs:1484` | `DirectionalLight` (elev 22°, azim 205°, 22 000 lux), 4-cascade CSM, PCSS (opt-in), contact shadows. |
| Sun-hugging haze glow | `distance_fog()` `look.rs:2342`; `FOG_SUN_GLOW` `:405` | `DistanceFog::directional_light_color/exponent` — warm glow term around the sun direction. |
| Image-based light | `ibl_env()` `look.rs:3199` | `EnvironmentMapLight::hemispherical_gradient` (v3+). |
| Post | `base_camera_look()` `look.rs:2440` | TonyMcMapface, `ColorGrading`, `Exposure`, emissive-only `Bloom`. |

**What is *missing* (the gap this doc is about):** no volumetric **clouds** of any kind, no **sun disc** drawn by us (Bevy's atmosphere draws it, or nothing does when the dome/clear-color path is live), no **limb darkening**, no **water surface** (planar reflection / SSR / refraction / foam). Bevy ships *god rays* and the *physical sky* and we already use both; it ships **no** clouds, **no** water, and its SSR is **deferred-only** (see §2.2).

---

## 1. Sky — clouds, sun disc, god rays

### 1.1 The AAA reference model (what "reference-grade" means here)

Two talks define the modern standard; both are the canonical primary sources and both are free:

- **Sébastien Hillaire — "Physically Based Sky, Atmosphere and Cloud Rendering in Frostbite"** (SIGGRAPH 2016, *Advances in Real-Time Rendering*). Course notes PDF: `https://media.contentapi.ea.com/content/dam/eacom/frostbite/files/s2016-pbs-frostbite-sky-clouds-new.pdf`. Author index: `https://sebh.github.io/publications/index.html`. This is the exact same LUT-atmosphere family Bevy 0.19 ships (Bevy's `Atmosphere`/`ScatteringMedium` is a port of Hillaire's transmittance-LUT → multiple-scattering-LUT → sky-view-LUT design).
- **Andrew Schneider — "The Real-time Volumetric Cloudscapes of Horizon: Zero Dawn"** (SIGGRAPH 2015). PDF: `http://advances.realtimerendering.com/s2015/The%20Real-time%20Volumetric%20Cloudscapes%20of%20Horizon%20-%20Zero%20Dawn%20-%20ARTR.pdf`. Guerrilla page: `https://www.guerrilla-games.com/read/the-real-time-volumetric-cloudscapes-of-horizon-zero-dawn`.

### 1.2 Volumetric clouds — raymarched vs billboard

**Raymarched (AAA standard).** Each sky pixel marches along the view ray through a 3D density field, then marches a second time toward the sun to light each sample. The model has four pieces:

1. **Density** — a low-frequency 3D "shape" noise (Perlin–Worley) for the billowing macro form, eroded by a high-frequency 3D Worley "detail" noise, both modulated by a 2D **weather map** (coverage / cloud type). `density = remap(shape × weather) − erosion(detail)`. Noise is a precomputed tiled 3D texture, never generated per-sample.
2. **Extinction** — Beer–Lambert transmittance `T = exp(−σ·d)` accumulated along both the view and light marches.
3. **In-scattering + phase** — Henyey–Greenstein `(1−g²)/(4π(1+g²−2g·cosθ)^(3/2))`, usually a double-lobe blend for the forward "silver lining"; multiple scattering is faked with an energy-conserving "powder"/multi-scatter term, not a second bounce.
4. **Anti-banding** — blue-noise dither, temporal reprojection, and rendering at reduced/checkerboard resolution with a reconstruction upsample.

Cost: ~64–128 view samples + ~6–16 light samples per lit step (Frostbite uses two-level coarse→refine marching).

**Billboard / 2D impostor / mesh clouds.** Flat camera-facing alpha cards (or a low-poly shell) textured with cloud sprites. Cost is a handful of quads; there is no real extinction, no in-scattering, no fly-through, and it reads flat under a low sun and at the horizon. There is **no single canonical primary paper** for billboards — the Schneider and Hillaire talks exist precisely to motivate *replacing* them. (See §3: this is effectively what Minecraft shaderpacks still use, at the "cheap 2D-noise" end.)

**What we already have vs. need.** We already run Bevy's Hillaire atmosphere (the sky and its in-scatter). Volumetric clouds are a *new* system. The only native-Bevy reference with real source is **evroon/bevy-volumetric-clouds** (`https://github.com/evroon/bevy-volumetric-clouds`, wgpu, credits Schneider + Hillaire) — but its README states clouds live on a **depth-less skybox** (can't fly into them), no integration with Bevy's own atmosphere, render res fixed at 1920×1080. The most complete feature reference is **twrwr/Meteoros** (`https://github.com/twrwr/Meteoros`, C++/Vulkan — Perlin–Worley + Worley tiling, remap erosion, Beer–Lambert + two-term HG, god rays as screen-space radial blur). Bevy upstream tracks this in `https://github.com/bevyengine/bevy/issues/17895` ("Support Volumetric Clouds").

**Nubis correction (worth recording).** Nubis is **not** open source — there is no Guerrilla GitHub repo (`github.com/GuerrillaGames/Nubis` → 404, confirmed). The canonical pages are the talk pages (no code): `https://www.guerrilla-games.com/read/nubis-authoring-real-time-volumetric-cloudscapes-with-the-decima-engine` and `https://www.guerrilla-games.com/read/nubis-evolved`. "Real source" = the third-party re-implementations above.

### 1.3 Sun disc + solar limb darkening

**Physics.** The Sun's photosphere is hotter at depth. Looking at the limb, the line of sight is tangential and reaches only cooler, higher layers, so the edge is dimmer — and because short wavelengths fall off faster with temperature, the limb is also **redder**. The empirical laws (primary source — `https://en.wikipedia.org/wiki/Limb_darkening`, formula and coefficients on-page; secondary physics text — Tatum, *Stellar Atmospheres* Ch. 6, `https://phys.libretexts.org/Bookshelves/Astronomy__Cosmology/Stellar_Atmospheres_(Tatum)/06%3A_Limb_Darkening`):

- Linear law: `I(μ)/I(0) = 1 − u(1 − μ)`, `μ = cos θ = √(1 − r²)` (r = normalised disc radius, 0 centre → 1 limb), `u = (I_center − I_limb)/I_center`.
- Polynomial law: `I(ψ)/I(0) = Σ a_k cosᵏ ψ`, `Σ a_k = 1`; Sun @ 550 nm ≈ `a₀=0.3, a₁=0.93, a₂=−0.23`.
- Wavelength: darkening is much stronger in blue/UV than red (u ≈ 0.95 @ 320 nm vs ≈ 0.56 @ 600 nm) — hence the red rim.

**How the engines actually do it.** Frostbite renders the sun as a disc of correct angular size (~0.5°), coloured by the solar radiance attenuated through the **transmittance LUT** along the view ray — white at zenith, orange/red near the horizon; the surrounding aureole is Mie in-scatter falling out of the sky-view LUT. **Frostbite does *not* model photospheric limb darkening as a distinct term** — the edge falloff most games show is atmospheric (transmittance + glow), not the B1 physics. True limb darkening is a cheap add-on: multiply the disc by the linear/polynomial law (a radial falloff) before compositing. (Cite: Hillaire Frostbite PDF, §1.1.)

**Our gap.** Bevy's `Atmosphere` already draws the sun disc + aureole when the atmosphere path is live (`VOXELFORGE_LOOK_ATMOS` unset/`lut`/`raymarched`). When the atmosphere is off (`_LOOK_ATMOS=off`, dome/clear-colour path), **nothing draws a sun disc** — the only "sun" is the `FOG_SUN_GLOW` warm haze term in `distance_fog()`. Adding a real disc + limb darkening is a small shader/quad addition; the limb-darkening law is the one physically-meaningful upgrade that neither the atmosphere path nor the dome path currently expresses.

### 1.4 God rays — screen-space radial blur vs world-space volumetric

- **Screen-space radial blur (Mitchell).** From the screen-space position of the sun, radially blur the bright sky/sun pixels outward and additively composite → shafts. Cheap (a few separable radial blur passes), no 3D data, but blind to off-screen occluders/sun and it can't model real occlusion. Canonical primary: **Kenny Mitchell, "Volumetric Light Scattering as a Post-Process"**, GPU Gems 3 Ch. 13 — `https://developer.nvidia.com/gpugems/gpugems3/part-ii-light-and-shadows/chapter-13-volumetric-light-scattering-post-process`.
- **World-space / physically-based.** Raymarch each pixel toward the sun (or sample a shadow map / cloud transmittance), accumulating Beer–Lambert extinction + HG in-scattering. Honors occluders and is consistent with the cloud/aerial-perspective scatter; more expensive. This is the Frostbite direction and the direction Bevy already ships.

**Our gap — god rays are already done, in the world-space style.** Bevy 0.19's `VolumetricFog` (camera) + `VolumetricLight` (sun) + `FogVolume` (medium) is exactly the world-space shadow-map raymarch. It shipped via `bevyengine/bevy#13057` (pcwalton — `https://github.com/bevyengine/bevy/pull/13057`) and lives in `bevy_light-0.19.0/src/volumetric.rs` (`VolumetricFog` :25, `VolumetricLight` :16, `FogVolume` :78 — doc comments literally say "light shafts / god rays"). `look.rs` already wires all three (`play_fog_volume()` + `insert_stack()` + `apply_look_to_sun()`). The Mitchell screen-space blur is the **cheaper fallback** for tiers that can't afford the 32/96-step march — worth keeping in the toolbox, not a gap.

---

## 2. Water (for Rose)

### 2.1 Planar reflection

Render the scene a second time from a camera mirrored across the water plane, with an **oblique near-plane clip** so geometry below the water can't bleed into the reflection, then sample that texture on the water. Correct, reflection-of-everything result (captures off-screen geometry SSR can't); cost = a full second scene render — Unreal documents "the entire scene is rendered twice" (≈23 ms heavy, ≈1.67 ms simple mobile). The two correctness gotchas: (1) mirroring flips polygon winding, so **cull mode must be inverted** for the mirrored camera; (2) the **oblique near plane** — replace the projection's near-clip row with the water plane expressed in the reflected camera's clip space (Lengyel: plane replaces row 3 for depth range `[0,1]`, else plane−row4).

Primary sources (all verified): **Bevy's own `examples/3d/mirror.rs`** (the reference Rose should copy — second `Camera3d` with `order: -1`, `RenderTarget::Image`, `invert_culling: true`, `calculate_mirror_camera_transform_and_projection` doing a Householder reflection + `near_clip_plane`) — `https://raw.githubusercontent.com/bevyengine/bevy/main/examples/3d/mirror.rs` (live demo `https://bevy.org/examples-webgpu/3d-rendering/mirror/`); **Eric Lengyel, "Oblique Near-Plane Clipping"** — `https://terathon.com/lengyel/Lengyel-Oblique.pdf`; **Unreal "Planar Reflections"** — `https://dev.epicgames.com/documentation/en-us/unreal-engine/planar-reflections-in-unreal-engine`. Classic physically-based reflection/refraction + Fresnel baseline: **Mark Finch, "Effective Water Simulation from Physical Models"**, GPU Gems 1 Ch. 1 — `https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-effective-water-simulation-physical-models`.

### 2.2 Screen-space reflections (SSR)

Raymarch the reflected view ray in screen space against the depth buffer, then resolve the hit colour. Cheap-ish, no second scene render, but only reflects what is already on screen (holes at grazing angles and off-screen). Two marching styles exist — the Hi-Z / hierarchical traversal (McGuire–Mara) and **linear stepping + refinement** (what Bevy actually ships: `step_count` from ray pixel length, `bias = 0.000002`, a `depth_thickness` hit test, and a bisection + optional secant root-finder in `crates/bevy_pbr/src/ssr/raymarch.wesl` — *not* Hi-Z). Primary: **McGuire & Mara, "Efficient GPU Screen-Space Ray Tracing"**, JCGT 3(4) 2014 — `https://jcgt.org/published/0003/04/04/paper.pdf` (landing `https://jcgt.org/published/0003/04/04/`). Production extension (stochastic, denoised): **Stachowiak, "Stochastic Screen-Space Reflections"**, SIGGRAPH 2015 — `https://www.ea.com/frostbite/news/stochastic-screen-space-reflections` (slides `http://advances.realtimerendering.com/s2015/Stochastic%20Screen-Space%20Reflections.pptx`).

**Bevy 0.19 already ships SSR — but it is deferred-only.** `bevy_pbr-0.19.0/src/ssr/mod.rs` — the component doc says verbatim: "Screen-space reflections are currently only supported with deferred rendering" (`:54`), and `ScreenSpaceReflections` carries `#[require(DepthPrepass, DeferredPrepass)]` (`:80`). Voxelforge's `look.rs` renders **forward** (`StandardMaterial`, no deferred prepass), so the built-in `ScreenSpaceReflections` **cannot be dropped in as-is**. Rose's options: (a) hand-roll a forward-path SSR pass using `raymarch.wesl` as the template, or (b) keep SSR on the shelf and use planar reflection for the one hero water body — the cheaper, higher-quality choice for a stylized voxel game with a bounded water surface.

### 2.3 Refraction

Grab the already-rendered opaque scene (a screen-space "grab pass"), sample it offset by the perturbed water normal (Snell's law, water IOR ≈ 1.33), and attenuate by depth/Beer–Lambert absorption so deep water reads darker/greener. Primary: GPU Gems 1 Ch. 1 (Finch) covers Fresnel + reflection + refraction together; the cleanest **canonical open-source implementation** is Unity's **Boat Attack `WaterCommon.hlsl`** — `https://raw.githubusercontent.com/Unity-Technologies/BoatAttack/master/Packages/com.verasl.water-system/Shaders/WaterCommon.hlsl` (`Refraction(distortion, depth, depthMulti)` samples `_CameraOpaqueTexture`, offset `distortion = viewNormal.xz * saturate(depth * 0.005)`, then `* Absorption(depth * depthMulti)`); engine models: **Unreal "Single Layer Water"** — `https://dev.epicgames.com/documentation/en-us/unreal-engine/single-layer-water-shading-model-in-unreal-engine` (homogeneous volume, per-channel absorption/scattering coefficients) and **Unity HDRP Water materials** — `https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-materials-in-the-water-system.html`.

**Bevy 0.19 already ships this in the forward path.** `StandardMaterial::specular_transmission` (`bevy_pbr-0.19.0/src/pbr_material.rs:260`, default `0.0`) with `StandardMaterial::ior` (`:309`, doc table lists **Water = 1.33** at `:318`) and `thickness` — driven by `ScreenSpaceTransmissionPlugin` (`bevy_pbr-0.19.0/src/lib.rs:238`), whose node runs `.after(main_opaque_pass_3d)` (`transmission/mod.rs:57`), i.e. **forward, not deferred**. Setting `specular_transmission: 0` disables the screen-space refraction effect entirely (`transmission/mod.rs:78`). So Rose gets screen-space refraction essentially for free from `StandardMaterial` fields; the water-specific work is the normal perturbation, the Fresnel term, and the absorption colour.

### 2.4 Foam

A shader mask that brightens water where it is shallow / turbulent. Three standard sources: **depth-difference shoreline foam** (water-to-occluder depth is small), **wave-tip/crest foam** (from encoded wave height), and **dynamic foam** (wakes/interaction, a separate mask). There is a **canonical open-source implementation**: Unity's Boat Attack `WaterCommon.hlsl` — shoreline `edgeFoam = saturate((1 - min(depth.x,depth.y)*0.5 - 0.25) + depthAdd) * depthEdge`, wave-tip `waveFoam = saturate(waveHeight - 0.75*0.5)`, a half-res `_WaterFXMap` (R = foam mask, G/B = normal XZ, A = displacement), combined `foamMask = saturate(length(foamMap * max(waveFoam, edgeFoam, waterFX.r*2)) * 1.5 - 0.1)` then `lerp(waterColor, foam, foamMask)`. URL: `https://raw.githubusercontent.com/Unity-Technologies/BoatAttack/master/Packages/com.verasl.water-system/Shaders/WaterCommon.hlsl`. Engine doc corroboration: **Unity HDRP water foam** — `https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-foam-in-the-water-system.html`. (A dedicated *paper* for foam does not exist — it is an engine/shaderpack convention; Boat Attack is the best primary source.)

### 2.5 Bevy water summary

| Piece | Bevy 0.19 status | Rose's work |
|---|---|---|
| Refraction (screen-space) | ✅ built-in, forward — `specular_transmission` + `ior` + `thickness` | normal perturb + Fresnel + absorption colour (port Boat Attack's `Refraction()`) |
| SSR | ⚠️ built-in but **deferred-only** (`ssr/mod.rs:54`); shader `raymarch.wesl` usable as a template | hand-roll forward SSR, or skip in favour of planar |
| Planar reflection | ⚠️ no component, but a **working engine example** `examples/3d/mirror.rs` (mirror camera + oblique clip + `invert_culling`) | adapt `mirror.rs` to the water plane |
| Foam | ❌ none | port Boat Attack `WaterCommon.hlsl` foam (shoreline + wave-tip + dynamic) |
| Water surface (waves/normals) | ❌ none in-tree; `bevy_water` (Neopallium) = waves-only, no refl/refr/foam; `bevy_simple_water` = deferred + optional SSR | authored water material (Gerstner or multi-octave normals) |

---

## 3. The ruler — what modern Minecraft shader mods actually do

The boss's reference. Findings below are read from the actual shader source, not from screenshots.

### 3.1 Complementary Reimagined (`ComplementaryDevelopment/ComplementaryReimagined`)

- **Clouds — cheap "volumetric-looking" 2D noise, not a 3D density field.** The OptiFine-style cloud code is `shaders/lib/atmospherics/clouds/mainClouds.glsl` → `reimaginedClouds.glsl` / `unboundClouds.glsl`. `GetVolumetricClouds()` marches along the view ray only **through a horizontal altitude band** (`planeDistanceDif`), `sampleCount = max(planeDistanceDif/8, 12)` capped at `min(sampleCount, 30)` (`reimaginedClouds.glsl`). Each step tests `GetCloudNoise()` = a **2D noise texture sample** (`texture2D(colortex3/gaux4, coord).b`), thresholded (`noise > threshold*0.5+0.25`); lighting marches `Get2DCloudSample()` toward the light. The noise field is literally a **static PNG** — `lib/textures/cloud-water.png`, bound to `colortex3`/`gaux4` in `shaders.properties`. So: **≤30 steps through a 2D-noise image** — nothing like Horizon's Perlin–Worley 3D field. (`mainClouds.glsl` adds `InterleavedGradientNoiseForClouds` dithering and `GetShadowOnCloud` shadow-map sampling; `cloudCoord.glsl` credits SixthSurge, Photon's author.) The "Unbound" variant (`unboundClouds.glsl`) optionally uses a small-step `Noise3D()` (sampleCount 2–4), still capped at 30.
- **Sky / sun — vanilla texture, not analytical.** `shaders/lib/atmospherics/sky.glsl` + `shaders/lib/colors/skyColors.glsl` draw the sky in `gbuffers_skytextured.fsh`; the sun there is the **vanilla textured sun quad** (`renderStage == MC_RENDER_STAGE_SUN`), sharpened with a high-power curve and warm-tinted (`pow(dot(color,color)*0.45, 6.0-5.0*rainFactor)` then `* mix(vec3(1.1,0.55,0.0), vec3(0.35), ...)`). The glow is a `pow(VdotS, glareScatter)` term in `sky.glsl`; the flare is `shaders/lib/misc/lensFlare.glsl` (screen-space, 4-point **depth-occlusion** test so it fades behind terrain). No photospheric limb darkening.
- **Water.** `shaders/lib/materials/specificMaterials/translucents/water.glsl` (multi-octave 2D normal maps + noise-threshold foam `pow2(clamp((foamThreshold+yPosDif)/foamThreshold,0,1))`) + `shaders/lib/materials/materialMethods/reflections.glsl` (screen-space ray-marched SSR, **plus an optional voxel ray-traced path "WSR"** over a `512×64×512` scene voxel volume in `reflectionVoxelization.glsl`) + `refraction.glsl` (screen-space) + `shaders/lib/atmospherics/fog/waterFog.glsl`. Foam is depth-based.
- **God rays / light shafts.** Screen-space march in `shaders/lib/atmospherics/volumetricLight/volumetricLight.glsl` (`GetVolumetricLight`, adaptive near/far sampling; density from `texelFetch(shadowtex0, ...)` — the **sun shadow map**), not a world-space 3D raymarch; plus an End-dimension beam variant (`enderBeams.glsl`).

Repo (verified): `https://github.com/ComplementaryDevelopment/ComplementaryReimagined` — the paths above are from its `shaders/` tree (`git/trees/HEAD?recursive=1`).

### 3.2 BSL Shaders

- **The original repo is deleted, but forks survive.** `github.com/CaptTatsu/BSLShaders` → 404 (confirmed: the repo was removed; the `CaptTatsu` org now holds only `BSLShadersLang`/`BSLShadersMapping`, the block-ID mapping + translations). Readable source lives in forks — `github.com/Noisysundae/bsl-ns` (carries the header *"BSL Shaders v8 Series by Capt Tatsu"*) and the 2020-era `github.com/bradleyq/BSLextended` (BSL++). Official distribution stays at `capttatsu.com/bslshaders/` + CurseForge.
- Primary pages (verified): official site `https://capttatsu.com/bslshaders/`; distribution `https://www.curseforge.com/minecraft/shaders/bsl-shaders`.
- **Technique family — same OptiFine lineage as Complementary, with these specifics (verified in `Noisysundae/bsl-ns`):** clouds = `shaders/lib/atmospherics/clouds.glsl` ray-marched **2D `noisetex` layers** (`CloudSample`), thickness faked by a per-step `detailZ = floor(currentStep*CLOUD_THICKNESS+0.5)*0.04` offset (no true 3D field); sun = the vanilla textured quad (`ROUND_SUN_MOON`, `renderStage == MC_RENDER_STAGE_SUN`) with a `pow` glow in `sky.glsl` and an anamorphic `lensFlare.glsl`; god rays = screen-space `volumetricLight.glsl` (`GetLightShafts`, comment "from Robobo1221, modified") sampling `shadowtex0/1`; water = `gbuffers_water.glsl` heightmap (`GetWaterHeightMap` + `GetParallaxWaves` 4-iter) + finite-difference normals + SSR (`reflections/raytrace.glsl`) + screen-space refraction + `pow(1+dot(normal,view),8)` fresnel. The `_poppy_lookv2/ref/bsl-01.jpeg` reference already in the repo is the night-plate the `Hour::NIGHT` rig is tuned against.

### 3.3 Iris Shaders

- Iris is the **shader-loader mod**, not a shaderpack — it implements the OptiFine pipeline (the gbuffers/deferred/composite stages the packs above plug into), so its relevance to us is architectural, not visual. Repo (verified): `https://github.com/IrisShaders/Iris`.
- Its value to the comparison: it defines the *stage graph* (gbuffers → deferred → composite) that Complementary/BSL target. That graph is why the packs can do screen-space passes (reflections, god rays, bloom) so cheaply — they have a deferred GBuffer to read. Voxelforge renders forward, which is precisely why Bevy's SSR is out of reach for us (§2.2) and why every screen-space water effect has to be added explicitly.

### 3.4 Photon — the physically-based ceiling (`sixthsurge/photon`)

The most physically-based pack in the set — the "if we go all the way, this is the target" reference.

- **Clouds — named cloud types, still a 2D coverage map.** `shaders/include/sky/clouds.glsl` (`draw_clouds`) layers cumulus → cirrus → altocumulus → cumulus-congestus → noctilucent; `clouds/cumulus.glsl` density samples a **precomputed 2D coverage map** (`texture(colortex8, coverage_map_uv).z`) with altitude shaping ("carve egg shape"), rendered at reduced res and TAAU-upscaled (`d1_clouds.fsh` → `d2_clouds_upscaling.fsh`). So even Photon's clouds are 2D-map raymarch — not Horizon's 3D field.
- **Sun disc — analytical.** `shaders/include/sky/sky.glsl` `draw_sun()`: `center_to_edge = max0(sun_angular_radius - fast_acos(nu))`, luminance normalised by the disc solid angle; the limb is handled by `nvidia_phase_area(nu, 0.85, 1.0, sun_angular_radius)` (Mie phase averaged over the disc) inside `atmosphere.glsl`'s full Rayleigh/Mie single-scatter atmosphere with a `256×64` transmittance LUT. `gbuffers_skytextured.fsh` **discards** the vanilla sun halo — the real sun is `draw_sun`.
- **God rays — world-space crepuscular rays.** `shaders/include/sky/crepuscular_rays.glsl` `draw_crepuscular_rays(cloud_shadow_map, ...)` marches a spherical shell with Rayleigh/Mie extinction coefficients, not a screen-space pass.
- **Water — Gerstner waves + roughness-aware SSR + physical fresnel.** `shaders/include/surface/water_normal.glsl` (`gerstner_wave`), `lighting/specular_lighting.glsl` (`get_specular_reflection` with `SSR_RAY_COUNT`/`ssr_multiplier = sqr(1-roughness)`, `fresnel_schlick`/`fresnel_dielectric`), screen-space refraction written in `gbuffers_all_translucent.fsh`.

### 3.5 Solas Shader (`Septonious/Solas-Shader`) — brief

BSL-lineage. Clouds = ray-marched 2D-noise volumetric **or** a flat 2D `planarClouds.glsl` layer (same `detailZ` thickness fake); sun = **procedural** `pow` disc (no texture) in `atmosphere/sunMoon.glsl`; god rays = screen-space `atmosphere/volumetrics.glsl`; water = `water/waterNormals.glsl` heightmap + SSR + sky fallback. Not materially different from BSL for our purposes — listed for completeness.

### 3.6 Gap analysis vs Voxelforge

| Thing | Complementary/BSL | Voxelforge today | Verdict |
|---|---|---|---|
| Clouds | 2D-noise altitude-band march, ≤30 steps | none | **Gap.** Even Photon (the most physical pack) ray-marches a 2D coverage map — **nobody in the ruler set uses a true 3D noise field.** Our billboard-first, 2D-noise march is on-par with the ruler; a real raymarched field would be *above* it. |
| Sun disc | vanilla texture + `pow` sharpen (Complementary/BSL); Photon = analytical disc | only via Bevy atmosphere (or nothing on dome path) | **Partial gap.** No authored disc/glare of our own; limb darkening is the cheap physical upgrade, and Photon is the source to read if we want the analytical version. |
| God rays | screen-space composite (Complementary/BSL); Photon = world-space | **world-space** (Bevy `VolumetricFog`/`VolumetricLight`) | **We are ahead.** Ours is world-space like Photon's, physically correct; theirs is cheaper. Keep the march, add a screen-space fallback for Low tier. |
| Water | SSR + refraction + depth foam (Complementary adds optional voxel-RT "WSR"; Photon adds Gerstner + physical fresnel) | none | **Gap.** Refraction is free from Bevy (`specular_transmission`); reflections and foam are the real work; Gerstner + the WSR voxel path are the two "above the ruler" extras. |
| Tone/grade | pack-specific LUTs | TonyMcMapface + ColorGrading | No action — already our lane's identity. |

---

## 4. Cost/benefit — cheapest first, mapped to the actual `look.rs` systems

Ranked by **visual payoff per unit of effort**, cheapest first. "Touch" names the real system/struct in `look.rs` (or the Bevy crate) each item lands in.

| # | Item | What it buys | Effort | Touch points |
|---|---|---|---|---|
| 1 | **Sun-disc limb darkening** | Physically-plausible sun edge (redder rim), fixes the "flat white circle at the horizon" complaint | tiny — a radial falloff on the disc | `apply_look_to_sun()` / new `SunDisc` component alongside `Hour::sun_dir()`; or a quad in `sky_dome()` for the dome path |
| 2 | **Authored sun disc + glare on the dome/clear-colour path** | Sun exists even when `_LOOK_ATMOS=off` (dome path today has no sun) | small | `sky_dome()` / `build_sky_dome_mesh()`; reuse `FOG_SUN_GLOW` / `FOG_SUN_EXPONENT` |
| 3 | **Screen-space god-ray fallback (Mitchell)** | Low-tier shafts without the 32/96-step march | small (a radial-blur post pass) | `insert_stack()` `Low` arm (currently no volumetrics); sun screen-pos from `Hour::sun_dir()` |
| 4 | **Water refraction** | Water reads as water (refraction + Fresnel + absorption) | small — Bevy does the grab | `StandardMaterial { specular_transmission, ior: 1.33, thickness }` + a water normal map; new water material, not `look.rs` |
| 5 | **Billboard cloud layer** | Sky stops reading as a flat dome | small–medium | new `CloudLayer` system next to `sky_dome()`; feed `Hour::sun_dir()` for lighting |
| 6 | **Water planar reflection** | Correct reflections on the one hero water body | medium (mirror camera + oblique clip) | new `WaterPlane` system; only if a full scene re-render is affordable |
| 7 | **Depth-foam on water** | Shoreline reads grounded | medium | water material shader (depth of water fragment vs scene) |
| 8 | **Forward-path SSR (hand-rolled)** | Reflections that work in our forward pipeline | large | new `Ssr` pass; **not** Bevy's `ScreenSpaceReflections` (deferred-only) |
| 9 | **Raymarched volumetric clouds (Horizon/Frostbite)** | True AAA clouds, fly-through, silver lining | large (3D noise textures, two-level march, reprojection, upscale) | new `VolumetricClouds` system alongside `atmosphere_sky()`; reference `evroon/bevy-volumetric-clouds` + `twrwr/Meteoros` |

Recommended order for the look lane: **1 → 2 → 3 → 5**, then hand **4 → 6 → 7** to Rose for water. **8** and **9** are the expensive end and should wait on Poppy's per-effect ms budget.

---

## 5. Primary sources (all verified reachable at time of writing)

### Sky / clouds / god rays
- Hillaire, Frostbite SIGGRAPH 2016 course notes — `https://media.contentapi.ea.com/content/dam/eacom/frostbite/files/s2016-pbs-frostbite-sky-clouds-new.pdf`
- Hillaire author index — `https://sebh.github.io/publications/index.html`
- Schneider, Horizon Zero Dawn clouds, SIGGRAPH 2015 — `http://advances.realtimerendering.com/s2015/The%20Real-time%20Volumetric%20Cloudscapes%20of%20Horizon%20-%20Zero%20Dawn%20-%20ARTR.pdf`; `https://www.guerrilla-games.com/read/the-real-time-volumetric-cloudscapes-of-horizon-zero-dawn`
- Nubis (talk pages, no code) — `https://www.guerrilla-games.com/read/nubis-authoring-real-time-volumetric-cloudscapes-with-the-decima-engine`; `https://www.guerrilla-games.com/read/nubis-evolved`
- bevy-volumetric-clouds (native Bevy port) — `https://github.com/evroon/bevy-volumetric-clouds`
- Meteoros (Vulkan Decima-style clouds) — `https://github.com/twrwr/Meteoros`
- Bevy volumetric-clouds tracking issue — `https://github.com/bevyengine/bevy/issues/17895`
- Solar limb darkening — `https://en.wikipedia.org/wiki/Limb_darkening`; Tatum, Stellar Atmospheres Ch. 6 — `https://phys.libretexts.org/Bookshelves/Astronomy__Cosmology/Stellar_Atmospheres_(Tatum)/06%3A_Limb_Darkening`
- Mitchell, GPU Gems 3 Ch. 13 (radial-blur god rays) — `https://developer.nvidia.com/gpugems/gpugems3/part-ii-light-and-shadows/chapter-13-volumetric-light-scattering-post-process`
- Bevy volumetric fog/light PR (origin of `VolumetricFog`) — `https://github.com/bevyengine/bevy/pull/13057`
- bevy_atmosphere (procedural sky crate) — `https://github.com/JonahPlusPlus/bevy_atmosphere`; `https://docs.rs/crate/bevy_atmosphere/latest`

### Water
- Finch, GPU Gems 1 Ch. 1 (reflection/refraction/Fresnel) — `https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-effective-water-simulation-physical-models`
- Bevy planar-reflection example (the one to copy) — `https://raw.githubusercontent.com/bevyengine/bevy/main/examples/3d/mirror.rs`; live `https://bevy.org/examples-webgpu/3d-rendering/mirror/`
- Lengyel, Oblique Near-Plane Clipping — `https://terathon.com/lengyel/Lengyel-Oblique.pdf`
- Unreal Planar Reflections — `https://dev.epicgames.com/documentation/en-us/unreal-engine/planar-reflections-in-unreal-engine`
- McGuire & Mara, Efficient GPU Screen-Space Ray Tracing (JCGT 2014) — `https://jcgt.org/published/0003/04/04/paper.pdf`
- Stachowiak, Stochastic Screen-Space Reflections (SIGGRAPH 2015) — `https://www.ea.com/frostbite/news/stochastic-screen-space-reflections`
- Bevy SSR shader (linear march + bisection/secant, not Hi-Z) — `https://raw.githubusercontent.com/bevyengine/bevy/main/crates/bevy_pbr/src/ssr/raymarch.wesl`
- Boat Attack `WaterCommon.hlsl` (canonical refraction + foam) — `https://raw.githubusercontent.com/Unity-Technologies/BoatAttack/master/Packages/com.verasl.water-system/Shaders/WaterCommon.hlsl`
- Unreal Single Layer Water — `https://dev.epicgames.com/documentation/en-us/unreal-engine/single-layer-water-shading-model-in-unreal-engine`
- Unity HDRP Water materials / foam — `https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-materials-in-the-water-system.html`; `https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-foam-in-the-water-system.html`
- bevy_water (waves only, no refl/refr/foam) — `https://github.com/Neopallium/bevy_water`
- bevy_simple_water (deferred + optional SSR) — `https://github.com/GuillaumeDelorme/bevy_simple_water`

### Minecraft shader mods
- Complementary Reimagined — `https://github.com/ComplementaryDevelopment/ComplementaryReimagined` (cloud/sun/godray/water paths in §3.1)
- BSL Shaders — official `https://capttatsu.com/bslshaders/`; distribution `https://www.curseforge.com/minecraft/shaders/bsl-shaders` (Cloudflare bot-blocks non-browser clients → 403 via curl; page is real); original `CaptTatsu/BSLShaders` repo **deleted (404)** — source survives in forks `https://github.com/Noisysundae/bsl-ns` (BSL v8) and `https://github.com/bradleyq/BSLextended` (BSL++)
- Iris Shaders — `https://github.com/IrisShaders/Iris`
- Photon — `https://github.com/sixthsurge/photon` (the physically-based ceiling: analytical sun disc, Gerstner water)
- Solas Shader — `https://github.com/Septonious/Solas-Shader` (BSL-lineage)

### Bevy 0.19 crate source (local registry — primary)
- `bevy_light-0.19.0/src/atmosphere.rs` (`Atmosphere` :35, `ScatteringMedium` :133)
- `bevy_light-0.19.0/src/volumetric.rs` (`VolumetricLight` :16, `VolumetricFog` :25, `FogVolume` :78)
- `bevy_pbr-0.19.0/src/atmosphere/mod.rs` (`AtmosphereSettings` :289, `AtmosphereMode` :415)
- `bevy_pbr-0.19.0/src/ssr/mod.rs` (`ScreenSpaceReflections` — deferred-only, :54/:80)
- `bevy_pbr-0.19.0/src/transmission/mod.rs` (`ScreenSpaceTransmissionPlugin`, forward, :27/:57/:78)
- `bevy_pbr-0.19.0/src/pbr_material.rs` (`specular_transmission` :260, `ior` :309, water 1.33 :318)

**Not found / corrected while writing:** Nubis is not open source (no Guerrilla GitHub repo; `github.com/GuerrillaGames/Nubis` 404); BSL's original `github.com/CaptTatsu/BSLShaders` repo is **deleted (404)** and its source survives only in forks; Hillaire's SIGGRAPH 2016 sky/cloud paper contains **no water section** (confirmed by full-text extraction) — water cites Stachowiak's SSR talk instead; a dedicated *paper* for shader foam does not exist (the canonical source is Boat Attack's open-source `WaterCommon.hlsl`, not a paper). The one bot-blocked-but-real URL is CurseForge's BSL page (403 to curl/Cloudflare), noted inline.
