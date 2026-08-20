# Research: Shader-pack Gap vs Voxelforge — 2026-08-20

**Owner:** Sahara (Researcher)  
**Branch:** `poppy/native-only` · deliverable baseline commit `2dac572`  
**Bevy version:** `0.19` (`client/Cargo.toml:100`)  
**CEO refs used:** `docs/refs/ceo_ref_sunset_valley.jpg` (1 still) + `docs/assets/look/p0env-baseline-16plates-2026-08-20.json` (16 plates) + `_poppy_artgap_outdoor/*-nohud2.png` (4 live frames)  

---

## 1. What this doc answers

(a) Real technical names of the techniques that make Minecraft + shader packs (BSL / Complementary / SEUS / Rethinking Voxels) look better than Voxelforge.  
(b) Whether each technique already exists in our code, verified by `grep` / file read, not guessed.  
(c) Image impact (High / Medium / Low) and implementation effort.  
(d) For the top 5, concrete Bevy 0.19 implementation path with real API / feature names checked against `client/Cargo.toml`.  
(e) Minecraft block-shape variety vs what Voxelforge actually renders today.

---

## 2. Reference summary — where shader packs win

From `docs/aaa-scoreboard-live.md` (2026-08-19 19:48, `_poppy_sky_g80_HEAD.png` vs CEO ref) the 6 GAP axes are:

| Axis | REF | Ours | % of REF | Owner | What it means visually |
|---|---|---:|---:|---|---|
| cool/water chroma | 2.96 | 0.15 | 5% | world | no cyan/blue water or sky contribution |
| emissive light points | 159 | 11 | 7% | world | almost no lanterns / campfires / glowstone in the world |
| sky tonal gradient | 172.58 | 34.30 | 20% | renderer | sky is a flat band |
| sky hue range | 60.0° | 20.0° | 33% | renderer | sky colour span is narrow |
| sky brighter than ground | 1.80 | 0.84 | 47% | renderer | sky not bright enough at golden hour |
| palette breadth | 15 bins | 8 bins | 53% | art | block palette is narrow |

The 16-plate baseline (`docs/VERDICT-p0env-controls-baseline-2026-08-20.md` §3) adds: our `edge_density` 9–21 vs REF 58.63, `far_micro` 2.9–7.9 vs REF 15.61, and 12/16 plates clip the B channel to ~0 which locks the colour axes regardless of tuning.

---

## 3. Technique inventory — sorted by impact-to-effort

### Legend
- **Have?** = confirmed by code read.
- **Impact:** H = high, M = medium, L = low.
- **Effort:** tiny / small / medium / large.

| # | Technique (real name) | Have? | Evidence in code | Impact | Effort | Why it closes the gap |
|---|---|:---:|:---|:---:|:---:|:---|
| 1 | **Volumetric clouds / cloud layer** | ❌ | `look.rs` has `Atmosphere`, `DistanceFog`, `FogVolume`, god rays, but no cloud system. `docs/research/aaa-look-techniques.md` §1.2 confirms none. | H | medium | Adds sky gradient, hue span, and light scattering; directly attacks `sky tonal gradient`, `sky hue range`, `sky brighter than ground`. |
| 2 | **Screen-space planar reflection + water SSR** | ❌ partly | `water.rs` has `WaterExtension` with Fresnel + Beer's-law depth tint + animated wave normals (`assets/shaders/water.wgsl`). No planar reflection, no SSR, no foam, no refraction. `StandardMaterial::specular_transmission` is unused. | H | medium | Fixes cool/water chroma and makes water read as water; reflection is the single biggest water win. |
| 3 | **Coloured / emissive block lights** | ❌ partly | `LAMP` block exists (`sim/src/block.rs:32`) and `vfx.rs` has particles/sparks, but the world has only 0–10 `emissive_blobs` per frame vs REF 159. No point-light entities tied to blocks. | H | small | Adds emissive light points, warms night shots, breaks up flat shadow. |
| 4 | **TAA — Temporal Anti-Aliasing** | ✅ | `look.rs:44` imports `bevy::anti_alias::taa::TemporalAntiAliasing`; inserted in High/Ultra tiers (`look.rs:2881`, `2902`, `2920`). | M–H | already in | Already shipped; keeps thin grass edges and far silhouette stable. |
| 5 | **PCSS soft shadows** | ✅ | Feature flag `experimental_pbr_pcss` (`client/Cargo.toml:92-93`); `look.rs:673-1105` implements `PCSS_WIDTH_V3`, tiered toggles, env override `VOXELFORGE_LOOK_PCSS`. | M–H | already in | Soft penumbras on outdoor geometry; widening helps contact detail. |
| 6 | **SSAO + contact shadows** | ✅ | `ScreenSpaceAmbientOcclusion` + `ContactShadows` imported (`look.rs:55-56`) and inserted per tier (`look.rs:2707`, `2738`). | M | already in | Ground contact detail; already running. |
| 7 | **PBR normal/roughness/occlusion maps** | ✅ | `voxel.rs:845-910` builds `StandardMaterial` with `normal_map_texture`, `metallic_roughness_texture`, `occlusion_texture`; tangents injected (`voxel.rs:1176`). `assets/textures/blocks/*_n.png`, `*_r.png` loaded by `block_atlas.rs`. | M | already in | Surface detail; 64px tiles authored 2026-08-18. |
| 8 | **Greedy meshing + per-block repeat UVs** | ✅ | `greedy_mesh_chunk` / `greedy_mesh_chunk_split` (`voxel.rs:1188-1212`), `ImageAddressMode::Repeat` (`voxel.rs:2104`). | M | already in | Efficient draw calls; separate path for near editable world. |
| 9 | **Volumetric fog / god rays** | ✅ | `VolumetricFog`, `VolumetricLight`, `FogVolume` imported (`look.rs:50-55`); High/Ultra insert full step march (`look.rs:2908`, `2921`). | M | already in | World-space shafts; already shipped. |
| 10 | **Physical atmosphere (Hillaire LUT / raymarched)** | ✅ | `Atmosphere::earth(medium)` + `AtmosphereSettings` (`look.rs:3545-3650`); LUT default, `raymarched` env lever. | M–H | already in | Sky gradient base; still needs clouds/sun disc to match CEO ref. |
| 11 | **Bloom (emissive-only)** | ✅ | `Bloom` + `BloomPrefilter` (`look.rs:58`); `base_camera_look()` sets threshold/intensity (`look.rs:2656-2687`). | M | already in | Glow on emissive sources; current setting is conservative. |
| 12 | **Exposure (ev100)** | ✅ | `bevy::camera::Exposure` (`look.rs:46`); `Exposure { ev100: ... }` inserted in `base_camera_look` (`look.rs:2653`). | M | already in | Sets overall brightness; tuning can help sky/ground ratio. |
| 13 | **Tonemapping + colour grading** | ✅ | `Tonemapping` (`look.rs:47`), `ColorGrading`/`ColorGradingGlobal`/`ColorGradingSection` (`look.rs:58-59`, `look.rs:2604-2644`). | M | already in | Already the look identity. |
| 14 | **Depth of field** | ✅ | `DepthOfField` imported (`look.rs:59`) but explicitly NOT inserted at any tier (`look.rs:2813-2815`). | L | tiny to enable | Not used in gameplay; could help cinematic shots. |
| 15 | **Foliage wind vertex shader** | ✅ | `FoliageMaterial = ExtendedMaterial<StandardMaterial, FoliageWindExt>` (`foliage.rs:28-107`); `assets/shaders/foliage_wind.wgsl`. | M | already in | Living grass/trees; already shipped. |
| 16 | **Parallax / POM (Parallax Occlusion Mapping)** | ❌ | Not found. `voxel.rs` only uses normal maps, no displacement/parallax lookups. | L–M | medium | Extra micro-relief on brick/stone; adds edge density. |
| 17 | **Subsurface scattering on leaves** | ❌ | `StandardMaterial` supports `diffuse_transmission` in Bevy 0.19, but `voxel.rs` never sets it for leaves. | L–M | small | Back-lit leaves read softer. |
| 18 | **Motion blur** | ❌ | Not found in `look.rs` or camera code. | L | small | Cinematic motion; gameplay rarely wants it. |
| 19 | **Rain / wetness / puddles** | ❌ | No weather system found; `DistanceFog` is the only haze control. | M | medium | Adds specular variation and story. |
| 20 | **Voxel AO (vertex ambient occlusion)** | ✅ | Mesher-side AO in `voxel.rs:312-1376`; `build_face_occlusion` builds per-face occlusion maps (`voxel.rs:2026`). | M | already in | Corner darkening in block joints. |

### What the shader packs actually do (sources)

| Pack | Key techniques | Source |
|---|---|---|
| **CaptTatsu’s BSL Shaders** | screen-space reflections, 2D-noise volumetric clouds, screen-space god rays, POM/PBR, TAA, bloom, DoF, realistic water heightmap + SSR | [shadersmods.com](https://shadersmods.com/capttatsus-bsl-shaders-mod/) + fork `Noisysundae/bsl-ns` cited in `docs/research/aaa-look-techniques.md` |
| **Complementary Shaders** | SSAO, SSR, ray-traced-style reflections (WSR voxel path), volumetric clouds/fog/light, PBR/POM, atmospheric sky, coloured shadows, scene-aware coloured lighting, water refraction/reflection | [shadersmods.com](https://shadersmods.com/complementary-shaders/) + source read in `docs/research/aaa-look-techniques.md` |
| **SEUS PTGI** | path-traced global illumination, software ray tracing, ray-traced reflections, SVGF denoising, TAA, HRR variant | [texture-packs.com/seus-ptgi](https://texture-packs.com/shaders/seus-ptgi/) + [Sonic Ether](https://www.sonicether.com/seus/) |
| **Rethinking Voxels** | coloured lighting, voxel ray-traced occlusion, coloured flood-fill block light, volumetric lighting, shadows | [modrinth.com/shader/rethinking-voxels](https://modrinth.com/shader/rethinking-voxels) |

---

## 4. Top 5 — implementation in Bevy 0.19

All APIs below are confirmed present in Bevy 0.19 (`client/Cargo.toml:100`) by code read in `client/src/look.rs` and `docs/research/aaa-look-techniques.md`.

### 4.1 Volumetric cloud layer (impact H · effort medium)

**Goal:** break the flat dome and add sky gradient/hue span.

**Bevy 0.19 path:**
- Add a new render pass **next to** `atmosphere_sky()` / `sky_dome()` in `look.rs`.
- Use a `Mesh` shell or fullscreen quad at far depth; sample a 2D/3D noise texture in WGSL.
- Light the cloud by the existing `DirectionalLight` direction (`Hour::sun_dir()`).
- No native `VolumetricClouds` component in Bevy 0.19; this is a custom shader pass, not a built-in feature.
- Reference: `docs/research/aaa-look-techniques.md` §1.2 — even AAA shader packs use 2D-noise altitude-band marching, so a 2D-noise cloud layer is on-par with BSL/Complementary.

**Files to touch:** `look.rs`, new `assets/shaders/cloud_layer.wgsl`, `assets/textures/sky/` (noise already there per `atlas.json` comment).

### 4.2 Water planar reflection + SSR fallback (impact H · effort medium)

**Goal:** make water reflect sky/terrain; raise cool chroma and far detail.

**Bevy 0.19 path:**
- Planar reflection: add a second `Camera3d` with `RenderTarget::Image`, `order: -1`, `invert_culling: true`, mirrored transform + oblique near-plane clip. Copy pattern from Bevy `examples/3d/mirror.rs` (cited in `docs/research/aaa-look-techniques.md` §2.1).
- Feed the render target into `water.wgsl` and blend with Fresnel.
- SSR fallback is **not** `bevy_pbr::ScreenSpaceReflections` — that component is deferred-only (`bevy_pbr-0.19.0/src/ssr/mod.rs:54`). For our forward path, hand-roll a screen-space raymarch in WGSL using the depth prepass.
- Free refraction already exists: set `StandardMaterial::specular_transmission` + `ior: 1.33` + `thickness` on the water material (`bevy_pbr-0.19.0/src/pbr_material.rs:260/309/318`).

**Files to touch:** `water.rs`, `assets/shaders/water.wgsl`, possibly a new `WaterReflectionCamera` system.

### 4.3 Coloured / emissive block lights (impact H · effort small)

**Goal:** raise `emissive_blobs` from 0–10 toward REF 159.

**Bevy 0.19 path:**
- Spawn `PointLight`/`SpotLight` entities at lamp/glowstone/campfire blocks during chunk meshing.
- Use `StandardMaterial::emissive` on the block material (`StandardMaterial` supports emissive texture + colour).
- Gate by block id: `BlockId::LAMP` already exists (`sim/src/block.rs:32`); add similar ids for torch/lantern/glowstone.
- Keep count budgeted per chunk to avoid light explosion.

**Files to touch:** `voxel.rs` material build, `sim/src/block.rs` for new light ids, chunk spawn system.

### 4.4 TAA + PCSS tuning (impact M–H · effort tiny)

**Goal:** stabilise thin edges and widen soft shadows for the outdoor vista shots.

**Bevy 0.19 path:**
- Already inserted: `TemporalAntiAliasing::default()` in High/Ultra (`look.rs:2881`, `2902`, `2920`).
- PCSS width is already tunable via `VOXELFORGE_LOOK_PCSS` and `PCSS_WIDTH_V3 = 12.0` (`look.rs:686`).
- The work is **measurement**, not wiring: run `p0env_baseline.py` pairs while sweeping `VOXELFORGE_LOOK_PCSS` on `grade-vista` and `s1-vista`, then lock the value.

**Files to touch:** none — only env sweeps and docs.

### 4.5 Subsurface scattering + wind on leaves (impact M · effort small)

**Goal:** soften leaf response and add life; helps palette breadth and detail.

**Bevy 0.19 path:**
- `StandardMaterial::diffuse_transmission` controls thin translucent scattering (leaves).
- For cross-quad vegetation, `FoliageMaterial` already uses `ExtendedMaterial<StandardMaterial, FoliageWindExt>` (`foliage.rs:107`); extend the extension to set `diffuse_transmission` on the base material.
- Confirm `MeshMaterial3d<FoliageMaterial>` instances pick it up.

**Files to touch:** `foliage.rs`, `assets/shaders/foliage_wind.wgsl` (if base-material fields need override).

---

## 5. Asset variety — Minecraft block shapes vs Voxelforge

### 5.1 Minecraft Java block-shape categories (from `minecraft.wiki/w/Block`)

| Shape category | Examples | Visual purpose |
|---|---|---|
| Full cube | stone, dirt, planks, logs, wool, concrete | bulk terrain and walls |
| Slab | stone slab, wood slab | half-height floors/roofs |
| Stair | stone stairs, wood stairs | smooth vertical transitions |
| Fence / wall | oak fence, cobblestone wall | rails, boundaries |
| Door / trapdoor | oak door, iron trapdoor | entrances, shutters |
| Pane / bars | glass pane, iron bars | windows, railings |
| Button / pressure plate | stone button, heavy weighted plate | detail, interactive surfaces |
| Torch / lantern | torch, soul lantern | light sources (emissive + point light) |
| Sign / banner / item frame | oak sign, banner | narrative detail |
| Bed / carpet | red bed, white carpet | furniture, soft surfaces |
| Ladder / vine | ladder, vine | vertical detail |
| Anvil / cauldron / hopper / composter | anvil, cauldron | utility shapes |
| Partial-cell | snow layer, turtle egg, sea pickle | ground clutter |
| Cross / billboard | grass, flowers, crops | vegetation that reads from all angles |

### 5.2 What Voxelforge actually renders today

| Shape | Status | Evidence |
|---|---|---|
| Full cube | ✅ shipped | `greedy_mesh_chunk` / `greedy_mesh_chunk_split` in `voxel.rs:1188-1212` |
| Water (liquid surface lowered 2/16) | ✅ shipped | `WATER_TOP_OFFSET` in `voxel.rs:1384-1391` |
| Glass / transparent cube | ✅ shipped | `BlockId::GLASS` is solid but not opaque (`sim/src/block.rs:34`, `voxel.rs:941-943`) |
| Cross-quad vegetation | ✅ shipped | `FoliageMaterial` + `mode: "cross"` in `atlas.json` (`foliage.rs:28-107`) |
| Stair | ❌ declared, not meshed | `BlockId::STAIR_WOOD` exists (`sim/src/block.rs:62`) but `voxel.rs` emits only cubes; no `block_shapes.rs` file exists |
| Slab | ❌ declared, not meshed | `BlockId::SLAB_STONE` exists (`sim/src/block.rs:64`) but no slab geometry found |
| Fence | ❌ declared, not meshed | `BlockId::FENCE_WOOD` exists (`sim/src/block.rs:66`) but no fence geometry found |
| Pane | ❌ declared, not meshed | `BlockId::PANE_GLASS` exists (`sim/src/block.rs:68`) but pane is currently a transparent cube (`voxel.rs` only does full faces); atlas manifest has no `"mode": "pane"` entry |
| Door / trapdoor / button / pressure plate / torch / sign / bed / carpet / ladder / anvil / cauldron / etc. | ❌ not present | No ids, no geometry, no atlas entries |

### 5.3 Which shapes are actually necessary to make a scene look “built”

For outdoor/terrain shots the highest-value shapes are:

1. **Slab** — roofs, paths, ledges; removes the “everything is one metre thick” look.
2. **Stair** — natural ramps, porches, towers.
3. **Fence / wall** — boundaries, terraces, animal pens.
4. **Pane** — windows that read as windows instead of solid glass blocks.
5. **Torch / lantern** — the emissive points the CEO ref scores 159 vs our 11.
6. **Door / trapdoor** — buildings without them read as shells.
7. **Cross vegetation** — already shipped; keeps grass/flowers readable from all angles.

**Recommendation:** implement slab + stair + fence first (they share axis-aligned partial-cube geometry and the existing greedy mesher can be extended), then pane, then torch/lantern as emissive blocks with point lights.

---

## 6. Sources

All sources below were verified reachable at research time or are local primary code reads.

### Code / local primary
- `client/Cargo.toml` — Bevy 0.19, `experimental_pbr_pcss` feature.
- `client/src/look.rs` — render stack (verified imports and insertions).
- `client/src/voxel.rs` — mesher, `StandardMaterial` build, PBR maps, AO, water top offset.
- `client/src/water.rs` + `assets/shaders/water.wgsl` — custom water material.
- `client/src/foliage.rs` + `assets/shaders/foliage_wind.wgsl` — cross-quad vegetation wind.
- `client/src/block_atlas.rs` — shape-mode validation (`SHAPE_MODES` includes stair/slab/fence/pane/cross).
- `sim/src/block.rs` — block ids and shape declarations.
- `docs/assets/look/p0env-baseline-16plates-2026-08-20.json` — 16-plate measurements.
- `docs/VERDICT-p0env-controls-baseline-2026-08-20.md` — control/baseline methodology.
- `docs/aaa-scoreboard-live.md` — scoreboard with 6 GAP axes.
- `docs/research/aaa-look-techniques.md` — prior research on clouds, sun disc, water, god rays, shader packs.

### External sources
- BSL Shaders feature list — [shadersmods.com](https://shadersmods.com/capttatsus-bsl-shaders-mod/)
- Complementary Shaders feature list — [shadersmods.com](https://shadersmods.com/complementary-shaders/)
- SEUS PTGI overview — [texture-packs.com](https://texture-packs.com/shaders/seus-ptgi/) + [Sonic Ether](https://www.sonicether.com/seus/)
- Rethinking Voxels — [modrinth.com/shader/rethinking-voxels](https://modrinth.com/shader/rethinking-voxels)
- Minecraft block shapes/collision categories — [minecraft.wiki/w/Block](https://minecraft.wiki/w/Block)
- Bevy planar reflection example — `examples/3d/mirror.rs` (cited in `docs/research/aaa-look-techniques.md`)

---

## 7. One-line take-away

Shader packs win on **sky richness, water reflection, and emissive world detail**. Voxelforge already ships a strong Bevy 0.19 PBR/post stack; the cheapest big wins are **clouds, water planar reflection, and block lights**, followed by finishing the **slab/stair/fence/pane** shape set so the world reads as built rather than sculpted from full cubes.
