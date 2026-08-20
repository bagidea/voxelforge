# Reference Shader Gap — 2026-08-21

## 1. แสง + AO (Light & Ambient Occlusion)

> ปัญหาปัจจุบัน: ภาพแบน ไม่มี AO ทั้งภาพเป็นสีส้มเดียว → หัวข้อนี้เป็น low-hanging fruit เพราะ Bevy มี SSAO built-in อยู่แล้ว

| เทคนิค | ต้นทุนประมาณ | ทำใน Bevy ได้ไหม / ติดตรงไหน |
|---|---|---|
| **SSAO** (Screen-Space Ambient Occlusion) | ~0.3–0.8 ms ที่ 1080p บน GPU รุ่นใหม่ | ทำได้ทันที — Bevy 0.11+ มี SSAO/GTAO built-in แค่เพิ่ม `ScreenSpaceAmbientOcclusion` component บน camera |
| **GTAO** (Ground-Truth Ambient Occlusion) | ~0.5–2.0 ms; รายงานต้นฉบับ Activision ได้ ~0.5 ms บน PS4 ที่ 1080p | Bevy 0.11–0.14 ใช้ GTAO ภายใน แต่ Bevy 0.15 เปลี่ยนเป็น **VBAO** แทน ถ้าอยากได้ GTAO ต้องเขียน custom post-process |
| **HBAO+** (Horizon-Based Ambient Occlusion Plus) | ~0.8–1.5 ms ที่ 1080p | ไม่มี built-in ใน Bevy; implement ผ่าน custom WGSL post-process ได้ แต่ต้องทำ bilateral blur / view-space position reconstruction เอง |
| **VXAO / Voxel Cone Tracing AO** | voxelization ~3–5 ms + cone tracing ~2.5–3.5 ms บน GTX 980 (1080p) | Bevy ไม่มี built-in voxel cone tracing ต้องสร้าง voxelization pass + 3D texture เอง หรือหา crate ภายนอกที่ support Bevy 0.15+ |
| **Bent Normals** | ~0.1–0.3 ms เพิ่มจาก AO pass | ทำได้ใน WGSL แต่ต้องปรับ lighting pass ให้รับ bent-normal เพื่อให้แสงอ้อมเข้ามุมถูกทิศ |

**สิ่งที่ควรทำก่อน** (เร็วสุด): เปิด `ScreenSpaceAmbientOcclusion` + `TemporalAntiAliasing` บน camera ใน Bevy 0.15 — VBAO แทน GTAO ได้คุณภาพดีขึ้นบน thin geometry (voxel edge) โดยไม่ต้องเขียน shader เอง แต่ยังไม่ support WebGL2/WebGPU

**อ้างอิง**
- [Ambient Occlusion Explained: SSAO vs HBAO vs GTAO 2026](https://superrendersfarm.com/article/ambient-occlusion-explained-ssao-hbao-gtao-2026)
- [Bevy 0.15 release notes — VBAO replaces GTAO](https://bevy.org/news/bevy-0-15/)
- [Activision — Practical Realtime Strategies for Accurate Indirect Occlusion (GTAO paper)](https://www.iryoku.com/downloads/Practical-Realtime-Strategies-for-Accurate-Indirect-Occlusion.pdf)
- [NVIDIA — HBAO+](https://developer.nvidia.com/rendering-technologies/horizon-based-ambient-occlusion-plus)
- [NVIDIA GTC 2015 — VXGI Dynamic Global Illumination for Games](https://docs.huihoo.com/gputechconf/gtc2015/S5670-NVIDIA-VXGI-Dynamic-Global-Illumination-for-Games.pdf)
- [NVIDIA GDC 2016 — Advanced Ambient Occlusion (VXAO overview)](https://developer.download.nvidia.com/gameworks/events/GDC2016/atatarinov_alpanteleev_advanced_ao.pdf)


## 2. วัสดุ PBR (PBR Materials)

> ปัญหาปัจจุบัน: เท็กซ์เจอร์ไม่มี normal/roughness ที่ทำงานจริง → ภาพจึงไม่มี micro-detail และทุกพื้นผิวมี specular เหมือนกันหมด

| เทคนิค | ต้นทุนประมาณ | ทำใน Bevy ได้ไหม / ติดตรงไหน |
|---|---|---|
| **Normal mapping** | ถูกมาก (~0.05–0.2 ms) ถ้ามี tangents | ทำได้ natively ใน `StandardMaterial` ต้องสร้าง mesh tangents (`Mesh::generate_tangents()`) และโหลด normal map เป็น linear (`is_srgb = false`) |
| **Metallic-Roughness texture** (glTF packed) | ตัว texture lookup ถูก; ผลกระทบหลักคือ BRDF/IBL ที่แพงขึ้นนิดหน่อย | `StandardMaterial` รองรับ glTF packing: **green=roughness, blue=metallic**; ต้องตั้ง scalar `metallic=1.0`/`perceptual_roughness=1.0` เพื่อให้ texture มีผล |
| **LabPBR specular texture** (Minecraft PBR standard) | lookup ถูกเหมือนกัน แต่ channel layout ต่างจาก glTF | LabPBR ใช้ **red=smoothness, green=metalness/F0** และช่องอื่นเป็นผิว/SSS/emission; ถ้าจะเอา LabPBR texture มาใช้ใน Bevy ต้อง swizzle หรือ re-pack เป็น glTF |
| **Parallax Occlusion Mapping (POM)** | ~0.1–0.3 ms บน GPU รุ่นใหม่ @ 8–32 samples; ~0.5–3 ms บน iGPU | Bevy 0.11+ มี parallax mapping built-in ใน `StandardMaterial` ผ่าน `depth_map`/`parallax_depth_scale` |
| **Triplanar mapping** | ~3× texture samples ต่อ material layer; ถ้า blend 4 materials อาจถึง 12 samples แค่ albedo | Bevy ไม่มี built-in แต่มี crate ใช้ได้: [`plumesplat`](https://github.com/DrewRidley/plumesplat), [`bevy_triplanar_splatting`](https://github.com/bonsairobo/bevy_triplanar_splatting) หรือเขียน custom WGSL |
| **Anisotropy / Clearcoat / Specular textures** | anisotropy +10–30% shader ALU; clearcoat เพิ่ม ~1 extra sample | รองรับผ่าน `StandardMaterial` แต่บาง feature ต้องเปิด feature gate (`pbr_specular_textures`, `pbr_anisotropy_textures`) |
| **IBL / Environment map lighting** | prefiltered map ถูก (lookup เดียว) แต่ irradiance volume แพง | Bevy มี `EnvironmentMapLight` และ `AmbientLight`; ถ้าจะทำ runtime reflection probe ต้องทำเอง |

**สิ่งที่ควรทำก่อน** (เร็วสุด): ใช้ `StandardMaterial` ให้ครบ — normal map, metallic-roughness packed texture, AO map, emissive — แล้ว generate tangents ให้ voxel mesh ทุกตัว (Bevy ตัด normal map เงียบๆ ถ้าไม่มี TANGENT) ส่วน POM เปิดได้ทันทีถ้ามี height/displacement map

**อ้างอิง**
- [Bevy `StandardMaterial` docs](https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html)
- [Bevy PBR example](https://bevy.org/examples/3d-rendering/pbr/)
- [Bevy 0.11 — Parallax mapping announcement](https://bevy.org/news/bevy-0-11/)
- [OptimumRealism — What Is PBR in Minecraft? LabPBR Explained](https://optimumrealism.com/info/what-is-pbr-in-minecraft)
- [mfagerlund/parallax-mapping-demo — measured POM costs](https://github.com/mfagerlund/parallax-mapping-demo)
- [Tatarchuk — Parallax Occlusion Mapping (SIGGRAPH 2006)](https://www.realtimerendering.com/advances/s2006/Tatarchuk-POM.pdf)
- [`plumesplat` — triplanar PBR splatting for Bevy](https://github.com/DrewRidley/plumesplat)
- [`bevy_triplanar_splatting` — triplanar material blending for Bevy](https://github.com/bonsairobo/bevy_triplanar_splatting)
- [N-hance — Triplanar Mapping overview](https://nhance-school.com/tools/glossary/triplanar-mapping)


## 3. น้ำ + ผิวเปียก (Water + Wetness)

| เทคนิค | ต้นทุนประมาณ | ทำใน Bevy ได้ไหม / ติดตรงไหน |
|---|---|---|
| **Screen-space refraction** (distorted transmission texture sample) | ต่ำ ~0.1–0.3 ms; แค่ texture lookup + normal distortion | ทำได้ — Bevy 0.12+ มี `view_transmission_texture` ให้ custom material อ่านผ่าน `reads_view_transmission_texture()` หรือใช้ `specular_transmission` บน `StandardMaterial` |
| **Screen-space reflections (SSR) บนน้ำ** | ~0.5–1.5 ms ที่ half-res (ขึ้นกับ ray march step และ roughness cutoff) | Bevy ไม่มี built-in SSR สำหรับ water ต้อง implement ray-march เอง หรือรอ Bevy มี SSR สำหรับ PBR |
| **Planar reflection** (render scene ซ้ำจากกระจก/น้ำ) | เท่ากับ re-render ฉากอีกครั้ง (vertex/poly count dependent) | ไม่มี built-in; ต้อง spawn secondary camera ให้ render ลง `RenderTarget/Image` แล้ว sample ใน shader |
| **FFT ocean simulation** | ~0.5–2.0 ms ที่ 256×256 บน mid/high-end GPU; ~2× ถ้า 3 cascades | Bevy ไม่มี built-in FFT ocean ต้องเขียน compute shader เองหรือใช้ vertex displacement แบบ Gerstner ก่อน |
| **Gerstner waves / vertex displacement** | ถูก ~0.1–0.5 ms สำหรับ mesh ปานกลาง | ทำได้ด้วย custom material/vertex shader ใน Bevy; เหมาะกับทะเล/แม่น้ำขนาดเล็ก |
| **Wetness / puddles** (albedo darken + roughness drop + mask) | ถูก ~0.05–0.2 ms ถ้าเป็น parameter blend; เพิ่ม ~0.5–2 ms ถ้าเปิด SSR/ripple animation | ทำได้ด้วย custom `Material` ปรับ albedo/roughness/normal ตาม puddle mask; ถ้าต้องการ reflection ในพื้นน้ำท่วมต้องทำ SSR/planar reflection เพิ่ม |
| **Caustics** | ~0.2–0.8 ms ขึ้นกับวิธี (projected texture / ray-marched) | Bevy ไม่มี built-in caustics ต้องเขียน projector/decal หรือ light function เอง |

**สิ่งที่ควรทำก่อน** (เร็วสุด): ทำ screen-space refraction ผ่าน `view_transmission_texture` + normal distortion สำหรับผิวน้ำ แล้ว blend กับ deep/shallow water color ตาม depth prepass เปิด wetness แบบง่ายด้วย puddle mask ที่ darkens albedo และลด roughness ก่อน ถึงจะเพิ่ม reflection ทีหลัง

**อ้างอิง**
- [Bevy GitHub Discussion — Performant water refraction shader example](https://github.com/bevyengine/bevy/discussions/22643)
- [Bevy 0.12 — PBR transmission / screen-space refraction](https://bevy.org/news/bevy-0-12/)
- [Aalborg University thesis — Reflection/refraction performance in Unity](https://projekter.aau.dk/projekter/files/213101783/Thesis_final_writing.pdf)
- [80.lv — Real-Time Physically Accurate Ocean Surface Simulation in Unity](https://80.lv/articles/real-time-physically-accurate-ocean-surface-simulation-in-unity)
- [NVIDIA Ocean SDK — Ocean Surface Simulation slides](https://developer.download.nvidia.com/assets/gamedev/files/sdk/11/OceanCS_Slides.pdf)
- [GodotShaders — Rain Puddles V3 (wetness/puddle approach)](https://godotshaders.com/shader/rain-puddles-v1/)
- [OpenSceneGraph users — Shader composition / wetness LOD cost](https://osg-users.openscenegraph.narkive.com/iTu0dVTp/shader-composition-opengl-modes-and-custom-modes)


## 4. ฟ้า + เมฆ (Sky + Clouds)

> บริบท Voxelforge: ยังไม่มี approved outdoor look; god rays ไม่ render ใน gameplay เพราะไม่มี `FogVolume`; ฟ้าในตอนนี้มีแนวโน้ม flat / แบน

| เทคนิค | ต้นทุนประมาณ | ทำใน Bevy ได้ไหม / ติดตรงไหน |
|---|---|---|
| **Procedural atmospheric scattering** (Rayleigh/Mie + precomputed LUT) | ~0.03–0.1 ms/frame ถ้า precompute แล้ว; ~3–300 ms สำหรับ precompute แต่กระจายได้หลาย frame | **ทำได้ natively** ตั้งแต่ Bevy 0.16 — แค่เพิ่ม `Atmosphere` component บน camera; support dynamic day/night และ mobile/WebGPU |
| **Skybox / cube map** | ถูกมาก ~0.01–0.05 ms (single texture lookup) | ทำได้ทันทีด้วย `Cubemap`/`EnvironmentMapLight` หรือ skybox mesh ธรรมดา แต่เป็นภาพ static |
| **Volumetric clouds (ray-marched 3D noise)** | ~2–3 ms/frame ที่ full-res; ~0.5–1 ms ที่ half-res | ไม่มี built-in แต่มี crate [`bevy-volumetric-clouds`](https://github.com/evroon/bevy-volumetric-clouds) ใช้ method จาก Horizon Zero Dawn รองรับ Bevy 0.17/0.18 แต่ยังไม่มี depth integration |
| **God rays / light shafts** (screen-space radial blur) | ~0.2–0.8 ms ที่ half-res | Bevy ไม่มี built-in god rays ต้อง implement เองด้วย post-process radial blur จาก sun position หรือรอ unified volumetrics |
| **Volumetric fog / FogVolume** | ~0.5–2 ms ขึ้นกับ density และ ray march steps | Bevy ยังไม่มี built-in `FogVolume` (issue [#18151](https://github.com/bevyengine/bevy/issues/18151) กำลัง proposal unified volumetrics) ต้องเขียน custom ray-march fog เอง |
| **Cloud impostors / billboard clouds** | ถูก ~0.1–0.3 ms | ทำได้ด้วย mesh + alpha-blended sprites; ไม่ต้อง ray march แต่ quality ต่ำ |

**สิ่งที่ควรทำก่อน** (เร็วสุด): เปิด `Atmosphere` บน camera เพื่อให้ได้ physically-based sky + sunset/sunrise ฟรี (~0.1 ms) จากนั้นถ้าต้องการเมฆให้ทดลอง `bevy-volumetric-clouds` หรือทำ cloud impostors ก่อน ส่วน god rays/fog volume ต้อง custom หรือรอ engine

**อ้างอิง**
- [Bevy 0.16 — Procedural atmospheric scattering](https://bevy.org/news/bevy-0-16/)
- [`bevy-volumetric-clouds` — Horizon Zero Dawn-style clouds for Bevy](https://github.com/evroon/bevy-volumetric-clouds)
- [Unreal Engine 4.27 — Volumetric Clouds documentation](https://dev.epicgames.com/documentation/unreal-engine/volumetric-clouds?application_version=4.27)
- [TU Wien — Real-Time Volumetric Rendering of Meteorological Cloud Data (2026)](https://www.cg.tuwien.ac.at/research/publications/2026/muth-2026-clouds/)
- [Chalmers — Efficient and Dynamic Atmospheric Scattering (thesis)](https://publications.lib.chalmers.se/records/fulltext/203057/203057.pdf)
- [Moonjump — Volumetric Lighting (God Rays) performance](https://moonjump.com/game-dev-mechanics-volumetric-lighting-god-rays-how-it-works/)
- [Chetan Jags — Volumetric Lighting: SunShafts](https://chetanjags.wordpress.com/2016/02/02/volumetric-lighting-sunshafts/)
- [Bevy issue #18151 — Physically based unified volumetrics system](https://github.com/bevyengine/bevy/issues/18151)


## 5. ใบไม้ไหว (Foliage Wind)

> บริบท Voxelforge: ฟีเจอร์ wind สำหรับพืชในเกมเพิ่ง closed ใน source (8a830b8) แต่ยังเป็นระดับ vertex displacement ง่าย — ยังไม่มี hierarchical wind หรือ pivot-based branch bending ที่ shader MOD Minecraft ใหม่ๆ ใช้

| เทคนิค | ต้นทุนประมาณ | ทำใน Bevy ได้ไหม / ติดตรงไหน |
|---|---|---|
| **Simple sine vertex displacement** (grass/leaves) | ~0.1–0.5 ms/scene ถ้า instanced + LOD | ทำได้ทันทีด้วย custom vertex shader ใน `Material`; ต้องส่ง `Time`, wind direction, amplitude เป็น uniform |
| **Hierarchical wind** (trunk sway → branch flutter → leaf shimmer) | ~0.3–1 ms ขึ้นกับ vertex count และ layer count | ทำได้ด้วย custom WGSL แต่ต้องเก็บ phase/weight ใน vertex color หรือ UV channel |
| **Pivot-based branch bending** | ~0.5–2 ms; ~71–95 shader instructions ตาม UDK | Bevy ไม่มี built-in ต้อง encode pivot ต่อ vertex/branch ใน mesh data แล้ว rotate ใน vertex shader |
| **Noise texture / gradient-based wind** | 0.1–0.4 ms เพิ่ม (texture lookup ใน vertex shader) | ทำได้แต่ texture lookup ใน vertex shader อาจช้ากว่า pure math บน mobile; แนะนำ compute ลง buffer ก่อน |
| **Interactive wind** (player/object collision) | CPU-GPU sync เป็นต้นทุนหลัก; shader เอง ~0.1 ms | Bevy ต้อง push influence radius/direction เข้า uniform/storage buffer เอง ยังไม่มี built-in interactive foliage |
| **Vertex Animation Texture (VAT)** | decode ถูก ~0.05–0.2 ms แต่ pre-bake กิน memory | มี crate [`bevy_open_vat`](https://github.com/HK416/bevy_open_vat) ช่วยเล่น VAT บน `StandardMaterial` |
| **Instancing + LOD + culling** | ลด draw calls ~90% ไม่ใช่ cost ต่อ pixel แต่ช่วย budget รวม | `GpuMeshInstancing` / `bevy_feronia` ช่วย scatter grass ได้ แต่ต้องจัด culling เอง |

**สิ่งที่ควรทำก่อน** (เร็วสุด): ใช้ custom vertex shader แบบ sine/gerstner บน foliage mesh ที่มี vertex color เป็น wind weight แล้ว instancing + frustum culling ถ้าต้องการความสมจริงขึ้นให้ทำ hierarchical wind (trunk/branch/leaf layers) ก่อน pivot-based bending เพราะราคาถูกกว่าและดูดีขึ้นมาก

**อ้างอิง**
- [Cyanilux — Soft Foliage Shader Breakdown](https://www.cyanilux.com/tutorials/soft-foliage-shader-breakdown/)
- [NVIDIA GPU Gems — Rendering Countless Blades of Waving Grass](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-7-rendering-countless-blades-of-waving-grass)
- [NVIDIA GPU Gems 3 — GPU-Generated Procedural Wind Animations for Trees](https://developer.nvidia.com/gpugems/gpugems3/part-i-geometry/chapter-6-gpu-generated-procedural-wind-animations-trees)
- [TRUETECH — Vegetation Shaders and Wind Animation performance](https://truetech.dev/games-development/services/vfx/vegetation-and-wind-shader-development.html)
- [`bevy_feronia` — foliage/grass scattering crate for Bevy](https://lib.rs/crates/bevy_feronia)
- [`bevy_open_vat` — Vertex Animation Texture plugin for Bevy](https://github.com/HK416/bevy_open_vat)
- [Francesco's GameDev Corner — UDK Wind Vertex Shader (71–95 instructions)](https://minifloppy.it/posts/2014/udk-wind-vertex-shader/)


## 6. 5 อย่างที่ให้ผลเยอะสุดต่อแรงที่ลง

อันดับเรียงจาก “ผลต่อภาพ” ต่อ “แรงที่ลง” สำหรับทีมอีก 7 เลนหยิบไปใช้ทันที:

1. **เปิด Bevy SSAO/VBAO + TAA บน camera** — ต้นทุน ~0.3–0.8 ms แต่แก้ปัญหา “ภาพแบน ไม่มี AO” ได้ทันที; เป็น built-in แค่เพิ่ม component ไม่ต้องเขียน shader
2. **ใช้ `StandardMaterial` ให้ครบ: normal map + metallic-roughness packed texture + generate tangents** — ต้นทุนต่ำ (~0.05–0.2 ms) แก้ “สีส้มเดียว / ทุกพื้นผิวเหมือนกัน” ได้ชัดเจน; งานส่วนใหญ่อยู่ที่ art pipeline ไม่ใช่ shader
3. **เปิด `Atmosphere` procedural sky บน camera** — ต้นทุน ~0.03–0.1 ms เปลี่ยนฟ้า flat เป็นฟ้า physically-based มี sunrise/sunset; built-in ตั้งแต่ Bevy 0.16
4. **Screen-space refraction สำหรับผิวน้ำผ่าน `view_transmission_texture`** — ต้นทุน ~0.1–0.3 ms ทำให้น้ำดูแยกจาก solid block ได้ทันที; ใช้ custom `Material` + WGSL สั้นๆ
5. **Simple foliage wind vertex shader + instancing/culling** — ต้นทุน ~0.1–0.5 ms ทำให้โลกมีชีวิต; implement ด้วย custom vertex shader + vertex-color wind weight ง่ายสุด

**สิ่งที่ *ไม่* ควรจับก่อน** เพราะแรงลงเยอะกว่าผล: VXAO, planar reflection, FFT ocean, volumetric clouds, god rays — เหล่านี้ต้อง custom render pass หรือ compute shader ในยุคปัจจุบันของ Bevy ยังไม่คุ้มกับเวลา

