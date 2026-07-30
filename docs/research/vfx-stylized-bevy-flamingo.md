# VFX Research: Stylized / Voxel Combat VFX ใน Bevy สำหรับ Voxelforge

> ร่าง: Sahara (Researcher)  
> วันที่: 2026-07-31  
> เป้าหมาย: รวบรวมเทคนิค VFX ระดับ AAA สไตล์ stylized / voxel ที่สามารถใช้งานได้ใน Bevy 0.19 + wgpu สำหรับ Voxelforge

---

## สรุปจำนวนเทคนิค

- **หมวดหลัก 4 หมวด**: Hit-stop, Impact / hit-spark / blood alternative, Dissolve, Trail / motion trail
- **เทคนิคย่อยที่สามารถนำไป implement ได้**: 10 เทคนิค
- **แหล่งอ้างอิงที่ตรวจสอบได้**: GDC Vault, Dustloop, Tekken Docs, SmashWiki, Kotaku, Critpoints, Socratopia, Bevy examples/docs/crates, RealtimeVFX, 80.lv, ฯลฯ

---

## 1. Hit-stop / Hit-pause Timing

### หลักการแปลงหน่วย (verified)
- **1 frame @ 60 fps = 16.67 ms**
- **4 frames @ 60 fps = 67 ms**
- แนะนำให้กำหนดค่าเป็น **ms ไม่ใช่ frame** เพื่อความเสถียรข้าม framerate (Socratopia)

### ตัวเลขจริงจากเกม / แหล่งข้อมูล

| เกม / บริบท | Hit-stop | คำนวณเป็น ms (@60fps) | หมายเหตุ | แหล่งอ้างอิง |
|---|---:|---:|---|---|
| **Guilty Gear Strive** — Attack Lv.0 | 11F | ~183 ms | frame data จริง | [Dustloop GGST Frame Data](https://www.dustloop.com/w/GGST/Frame_Data) |
| **Guilty Gear Strive** — Attack Lv.4 | 15F | ~250 ms | frame data จริง | [Dustloop GGST Frame Data](https://www.dustloop.com/w/GGST/Frame_Data) |
| **Guilty Gear Strive** — CH slowdown (large) | 31F | ~517 ms | counter hit | [Dustloop GGST Frame Data](https://www.dustloop.com/w/GGST/Frame_Data) |
| **Guilty Gear Strive** — Psych Burst | 15–16F | ~250–267 ms | ขึ้นกับ burst type | [Dustloop GGST Frame Data](https://www.dustloop.com/w/GGST/Frame_Data) |
| **Tekken 8** — Jun `f,F+1+2` | ~20F | ~333 ms | ท่าเดียวที่มีเลขติด | [Tekken Docs Jun](https://tekkendocs.com/jun) |
| **God of War (2018)** | ~2F | **32.5 ms** | ค่าประมาณจากบทวิเคราะห์ | [Kotaku — In Praise of Sticky Friction](https://kotaku.com/in-praise-of-sticky-friction-5558166) |
| **Street Fighter II** normals | ~10F | ~167 ms | ประมาณจากบทวิเคราะห์ | [Critpoints](https://critpoints.net/2017/05/17/hitstophitfreezehitlaghitpausehitshit/) |
| **Super Smash Bros. Melee** | cap 20F | ~333 ms | hitlag cap | [SmashWiki](https://www.ssbwiki.com/Hitlag) |
| **Super Smash Bros. (Brawl+)** | cap 30F | ~500 ms | hitlag cap | [SmashWiki](https://www.ssbwiki.com/Hitlag) |
| **Monster Hunter World: Iceborne** — HBG melee (mod) | 20F → 1F | 333 ms → 17 ms | ค่าดั้งเดิม 20F, mod ลดเหลือ 1F | [Ice-Stable](https://github.com/AsteriskAmpersand/Ice-Stable) |
| **ULTRAKILL** — major hitstop | 15F | **0.25 s / 250 ms** | ระยะหลัก | [ULTRAKILL Wiki](https://ultrakill.fandom.com/wiki/Hitstop) |
| **แนวทาง pixel-art combat** — light | 2–4F | 33–67 ms | guideline | [GamineAI Pixel Art Combat FX Guide](https://gamineai.com/blog/pixel-art-combat-fx-hitstop-smear-frames-impact-timing-2026) |
| **แนวทาง pixel-art combat** — heavy | 6–10F | 100–167 ms | guideline | [GamineAI Pixel Art Combat FX Guide](https://gamineai.com/blog/pixel-art-combat-fx-hitstop-smear-frames-impact-timing-2026) |
| **แนวทาง pixel-art combat** — boss super | 12–18F | 200–300 ms | guideline | [GamineAI Pixel Art Combat FX Guide](https://gamineai.com/blog/pixel-art-combat-fx-hitstop-smear-frames-impact-timing-2026) |
| **Indie design doc (ref)** — normal / crit / player hit / boss defeat | 2F / 4F / 3F / 10F | 33 / 67 / 50 / 167 ms | ตัวอย่างการกำหนดค่า | [infinite-dungeon-game hitstop.md](https://github.com/balbonits/infinite-dungeon-game/blob/main/docs/ui/hitstop.md) |
| **Socratopia recommendation** — first playable baseline | ~5F | **80 ms** | 40 ms เบาไป, 160 ms ช้าไป | [Socratopia](https://www.socratopia.app/library/game-design-compelling-en/chapter-13) |

### วิธี implement ใน Bevy
1. **Time<Virtual> pause** — หยุดเวลา gameplay ชั่วคราว แต่ยังให้ render / audio ทำงาน
2. **Animation speed = 0** — บน `AnimationPlayer` ของ attacker และ target ในระยะเวลาที่กำหนด (Socratopia)
3. **Input queueing** — เก็บ input ระหว่าง pause เพื่อรักษา flow ของ combo
4. **เลือก scope ของ pause** — หยุดเฉพาะตัวละครที่เกี่ยวข้อง หรือทั้ง scene? (Dark Souls / Monster Hunter หยุดเฉพาะที่เกี่ยวข้อง)

---

## 2. Impact / Hit-spark / Blood Alternative

### 2.1 Particle burst (sparks, embers, debris, voxel chunks)
- **Bevy approach:**
  - **[bevy_hanabi](https://github.com/djeedai/bevy_hanabi)** — GPU particle system หลักของ Bevy, เหมาะกับ 3D burst จำนวนมาก
  - **[bevy_enoki](https://github.com/lommix/bevy_enoki)** — 2D particles, รองรับ WebGL2 / mobile, มี visual editor
  - **[Sprinkles](https://doce.sh/blog/bevy-sprinkles)** — GPU particle system + editor เน้น 3D combat VFX
  - **[bevy_magic_fx](https://github.com/ethereumdegen/bevy_magic_fx)** — กำหนด VFX ผ่านไฟล์ RON สำหรับ mesh-based effect
- **สำหรับ voxel game:** สร้าง particle เป็น voxel cube เล็กๆ หรือ instanced cube ที่กระเด็นออกจากจุด impact แทนการใช้ sprite

### 2.2 Stylized impact flash / hit-flash / "sakuga" impact frames
- **ตัวเลข:** hit-flash ควรอยู่ที่ **1–2 frames** (GamineAI)
- **Technique จากอุตสาหกรรม:**
  - "Impact frames" ใน anime ใช้ภาพขาวดำ / สีตัดกันฉับพลัน 1–3 frames เพื่อเน้นจังหวะ impact ([Sakuga Blog](https://blog.sakugabooru.com/glossary/impact-frames/))
  - UE5/My Hero Academia-style ใช้ 7 post-process materials ต่อ 1 frame รวม ~12 frames ([80.lv](https://80.lv/articles/my-hero-academia-style-attack-vfx-in-unreal-engine-5))
  - Easy Impact Frames ใช้ radial shockwave shader + LUT + contrast boost ([RealtimeVFX](https://realtimevfx.com/t/easy-impact-frames-for-unreal-engine-and-unity/30252))
- **Bevy approach:**
  - **Fullscreen post-process flash** ผ่าน [Custom Post-Processing example](https://bevy.org/examples/shaders/custom-post-processing/) — invert/flash screen 1–2 frames
  - **Quad billboard shader** ด้วย additive radial gradient ([AIBodh tutorial](https://aibodh.com/posts/bevy-rust-game-development-chapter-6/))
  - **Sprite sheet animation** 2–4 frames บน quad ที่หันหน้าเข้ากล้อง (billboard)
  - **GDC reference:** "Shader Sauce: How to use shaders to create Stylized VFX" — V. Sharma, GDC Visual Effects Summit 2021 ([GDC Vault](https://gdcvault.com/play/1027332/Visual-Effects-Summit-Shader-Sauce))

### 2.3 Blood alternative สำหรับเกม voxel ที่ไม่มีเลือด
- ใช้ **voxel debris / cube chunks** สีของ enemy (stone, wood, rust, crystal, dust)
- **Dust / smoke / spark / ember particles**
- **Hit marker / crosshair feedback** + **screen shake**
- **Flinch / wince animation** บน enemy
- **Confetti / paint / ink / oil** ตามประเภทศัตรู (robot = oil/sparks, ghost = ectoplasm, plant = sap/leaves)
- แหล่งอ้างอิง: [Roblox DevForum — Alternative to blood?](https://devforum.roblox.com/t/alternative-to-blood/2081600)

---

## 3. Dissolve / Disintegration

### 3.1 Noise-based dissolve (เหมาะกับตัวละคร / enemy despawn)
- **Bevy approach:**
  - ใช้ `MaterialExtension` ขยาย `StandardMaterial` ([Rust Adventure](https://www.rustadventure.dev/extending-materials-in-bevy-0-12-with-materialextension))
  - WGSL: ใช้ `simplex_noise_3d` + `step` + `discard`
  - ต้อง implement ทั้ง `fragment_shader` และ `prepass_fragment_shader` เพื่อให้ shadow ถูกต้อง ([Hexbee](https://blog.hexbee.net/37-fragment-discard-and-transparency))
  - ใช้ `AlphaMode::Mask` สำหรับ hard edge cutout หรือ `AlphaMode::Blend` สำหรับ soft fade
  - เพิ่ม emissive edge ตามขอบ dissolve threshold เพื่อให้ดู stylized

### 3.2 Voxel block dissolve / crumbling
- **Bevy approach:**
  - แยก mesh เป็น voxel/block ย่อย แล้ว scale down / pop out ทีละ block ตาม noise pattern
  - ใช้ **GPU instancing** + compute shader หรือ update transform ของแต่ละ instance
  - Bevy 0.19 ยังไม่มี geometry shader สำเร็จ ต้องจัดการ instance buffer / compute เอง
  - สามารถ combine กับ particle debris เพื่อให้ดูเป็นธรรมชาติ

---

## 4. Trail / Weapon Trail / Motion Trail

### 4.1 Particle trail / ribbon (projectile trail, magic trail)
- **Bevy approach:**
  - **[bevy_hanabi](https://github.com/djeedai/bevy_hanabi)** 0.15+ มี ribbon/trail system ใหม่ ใช้ `Attribute::RIBBON_ID`
  - 0.16 มี open issue เรื่อง **indirect buffer overrun กับ trail effects** ([#493](https://github.com/djeedai/bevy_hanabi/issues/493)) — ต้องระวังถ้ามี entity จำนวนมากพร้อม trail
  - เหมาะกับ projectile trail หรือ magic swipe มากกว่า weapon melee trail ที่ต้อง precision สูง

### 4.2 Mesh trail / ribbon trail (weapon swing arc)
- **Bevy approach:**
  - สร้าง **procedural mesh `TriangleStrip`** จาก historical positions ของขอบอาวุธ (sample ทุก frame)
  - Custom material ด้วย gradient alpha fade (UV.x ตามความยาว trail)
  - Update mesh ทุก frame ตาม arc ของอาวุธ แล้ว fade alpha ภายใน 6–12 frames
  - ใช้ `AlphaMode::Blend` และ `double_sided` เพื่อให้มองเห็นจากทั้งสองด้าน

### 4.3 Afterimage / motion trail (character dash / dodge)
- **Bevy approach:**
  - Spawn ghost mesh copies ของตัวละคร/อาวุธ ทุก 2–4 frames
  - ลด alpha และ fade out ภายใน 3–6 frames
  - ใช้ `Visibility` และ `AlphaMode::Blend`
  - สำหรับ stylized look อาจใช้โทนสีต่างจากตัวจริง (เช่น ขาวดำ, สีสะท้อน)

---

## 5. Post-processing / Screen-space Feedback

### Built-in Bevy 0.19
- `Vignette` — ดำขอบจอเวลา low HP หรือ impact
- `LensDistortion` — บิดเลนส์เบาๆ ตอน heavy hit
- รายละเอียด: [Bevy 0.19 Release Notes](https://bevy.org/news/bevy-0-19/)

### Custom post-processing
- ใช้ตัวอย่าง [Custom Render Pass](https://bevy.org/examples/shaders/custom-post-processing/) ของ Bevy
- ใช้ได้กับ: fullscreen hit-flash, chromatic aberration ตอน impact, radial blur, color inversion
- แนวทางเพิ่มเติม: [bevy-vfx-bag](https://github.com/torsteingrindvik/bevy-vfx-bag) สำหรับ chromatic aberration, vignette, wave, pixelation

---

## 6. ข้อแนะนำสำหรับ Voxelforge

1. **Hit-stop tuning:** เริ่มที่ **80 ms** (~5F @ 60fps) สำหรับ light attack, **150–200 ms** สำหรับ heavy attack, ปรับทีละ 20–40 ms
2. **Impact VFX:** ใช้ `bevy_hanabi` หรือ `bevy_enoki` สำหรับ sparks/debris + quad/sprite hit-flash 1–2 frames + screen shake 4–8 frames
3. **Blood alternative:** ใช้ voxel cube particles สีของ enemy + dust/spark + flinch animation
4. **Dissolve:** ใช้ `MaterialExtension` + noise-based `discard` สำหรับ enemy despawn
5. **Trail:** สำหรับ weapon trail แนะนำ procedural mesh ribbon มากกว่า Hanabi trail (precision สูงกว่า); สำหรับ projectile ใช้ Hanabi ribbon/trail ได้
6. **Post-processing:** custom fullscreen flash + `Vignette` สำหรับ combat feedback

---

## 7. Caveats / ข้อจำกัด

- ตัวเลขบางค่า (เช่น God of War 32.5 ms, Street Fighter II ~10F) มาจากบทความวิเคราะห์ ไม่ใช่ official developer quote
- **Monster Hunter / Dark Souls** ไม่มีตาราง hit-stop/hitlag สาธารณะละเอียด — มีเพียงบริบททั่วไปจาก modding community
- **bevy_hanabi 0.16** มี open bug **#493** (indirect buffer overrun กับ trail) — ต้องทดสอบกับ load จริง
- **Version compatibility:** โปรเจกต์ใช้ `bevy = "0.19"` (verified จาก `client/Cargo.toml`) แต่ผลการค้นหาสาธารณะเกี่ยวกับ `bevy_hanabi` สำหรับ Bevy 0.19 ไม่ชัดเจน — ต้องตรวจสอบ `Cargo.toml` / `Cargo.lock` ของ crate ที่เลือกใช้ก่อน integrate
- ยังไม่มี VFX crate ใดใน repo ปัจจุบัน (verified จากการค้นหาใน workspace)

---

## 8. แหล่งอ้างอิง

### Hit-stop / Frame data
- [Dustloop — GGST Frame Data](https://www.dustloop.com/w/GGST/Frame_Data)
- [Tekken Docs — Jun](https://tekkendocs.com/jun)
- [Kotaku — In Praise of Sticky Friction](https://kotaku.com/in-praise-of-sticky-friction-5558166)
- [Critpoints — Hitstop/Hitfreeze/Hitlag/Hitpause](https://critpoints.net/2017/05/17/hitstophitfreezehitlaghitpausehitshit/)
- [Socratopia — Animation and Hit-Pause as Feedback](https://www.socratopia.app/library/game-design-compelling-en/chapter-13)
- [SmashWiki — Hitlag](https://www.ssbwiki.com/Hitlag)
- [GamineAI — Pixel Art Combat FX Guide](https://gamineai.com/blog/pixel-art-combat-fx-hitstop-smear-frames-impact-timing-2026)
- [ULTRAKILL Wiki — Hitstop](https://ultrakill.fandom.com/wiki/Hitstop)
- [Ice-Stable (MHW: Iceborne mod)](https://github.com/AsteriskAmpersand/Ice-Stable)
- [infinite-dungeon-game hitstop.md](https://github.com/balbonits/infinite-dungeon-game/blob/main/docs/ui/hitstop.md)

### Bevy VFX crates / docs
- [bevy_hanabi](https://github.com/djeedai/bevy_hanabi)
- [bevy_hanabi CHANGELOG](https://github.com/djeedai/bevy_hanabi/blob/main/CHANGELOG.md)
- [bevy_hanabi issue #493 — Indirect buffer overrun](https://github.com/djeedai/bevy_hanabi/issues/493)
- [bevy_enoki](https://github.com/lommix/bevy_enoki)
- [Sprinkles — GPU particle system & editor for Bevy](https://doce.sh/blog/bevy-sprinkles)
- [bevy_magic_fx](https://github.com/ethereumdegen/bevy_magic_fx)
- [bevy-vfx-bag](https://github.com/torsteingrindvik/bevy-vfx-bag)
- [Bevy — Custom Post-Processing example](https://bevy.org/examples/shaders/custom-post-processing/)
- [Bevy 0.19 Release Notes](https://bevy.org/news/bevy-0-19/)
- [Bevy Examples README](https://github.com/bevyengine/bevy/blob/main/examples/README.md)

### Shaders / Dissolve
- [Rust Adventure — Extending Materials in Bevy 0.12 with MaterialExtension](https://www.rustadventure.dev/extending-materials-in-bevy-0-12-with-materialextension)
- [Hexbee — Bevy & WGSL: Alpha, Discard & Transparency](https://blog.hexbee.net/37-fragment-discard-and-transparency)
- [AIBodh — Custom glow particle shader tutorial](https://aibodh.com/posts/bevy-rust-game-development-chapter-6/)

### Stylized VFX / Impact frames
- [GDC Vault — Shader Sauce: How to use shaders to create Stylized VFX (2021)](https://gdcvault.com/play/1027332/Visual-Effects-Summit-Shader-Sauce)
- [Sakuga Blog — Impact Frames](https://blog.sakugabooru.com/glossary/impact-frames/)
- [80.lv — My Hero Academia-Style Attack VFX in Unreal Engine 5](https://80.lv/articles/my-hero-academia-style-attack-vfx-in-unreal-engine-5)
- [RealtimeVFX — Easy Impact Frames for Unreal Engine and Unity](https://realtimevfx.com/t/easy-impact-frames-for-unreal-engine-and-unity/30252)
- [Roblox DevForum — Alternative to blood?](https://devforum.roblox.com/t/alternative-to-blood/2081600)
