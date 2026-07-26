# Voxelforge — Look Bible

> **Art-direction target** สำหรับทีม graphics engine.
> ถอดจาก reference 4 รูป (Minecraft-style voxel + realistic shader stack, โทน golden-hour).
> เอกสารนี้ = "ภาพปลายทาง" ที่วิศวกร render เล็งได้ชัด + checklist ฟีเจอร์ที่ต้องมี + สเปกแยก 2 tier.

Mood board ประกอบ: `docs/assets/moodboard.png`

> **🔒 Engine locked = Rust + Bevy 0.19 + wgpu.** ทั้งสอง tier คือ **โค้ดเบสเดียวกัน คนละ render preset** —
> ไม่ใช่คนละ engine. Ultra = wgpu native (Vulkan/Metal/DX12). Lite = wgpu เดิม compile → WASM
> (WebGPU เป็นหลัก, WebGL2 เป็น fallback backend ของ wgpu). **ห้ามหยิบลุคนี้ไปสร้างบน Godot / WebGL-engine อื่น.**
> ทุกอย่างในเอกสารนี้เป็น **engine-agnostic art target** — mapping ลง Bevy feature จริงว่าอะไรทำไหว/ไม่ไหว
> เป็นหน้าที่ของ Bevy spike (Poppy). เกณฑ์รับ visual อยู่ที่ `docs/golden-beauty-shot.md`.

---

## 0. TL;DR (หนึ่งย่อหน้า)

โลกเป็น **บล็อกลูกบาศก์คมชัด** (voxel, hard 90° edges, low-poly geometry) แต่ถูก "แต่งแสง" ด้วย
**shader realistic เต็มสแตก** — global illumination, soft shadow, god rays, atmospheric fog, PBR material,
SSAO, bloom, depth-of-field, tone-mapping และน้ำสะท้อน. หัวใจของ identity คือ **ความขัดแย้งที่ลงตัว**:
รูปทรงดิบเรียบง่ายเหมือนของเล่น × แสงนุ่มสมจริงเหมือนภาพถ่าย golden-hour. ตัด geometry ให้เรียบไม่ได้
(เสีย identity) แต่ตัดชั้นแสงบางตัวใน Lite tier ได้ถ้ายังรักษา "แสงอุ่น + เงานุ่ม + บล็อกคม" ไว้ครบ.

---

## 1. จุดร่วมของลุค (สิ่งที่ทั้ง 4 รูปมีเหมือนกัน)

ทุกรูปยืนอยู่บนสมการเดียว:

> **เรขาคณิตบล็อกคม (voxel)  ×  ชั้นแสง realistic (shader)  =  Voxelforge look**

รายละเอียดจุดร่วมที่เห็นซ้ำทั้ง 4 รูป:

| จุดร่วม | สังเกตจากรูป |
|---|---|
| **Voxel geometry คมชัด** | ทุกอย่างเป็นลูกบาศก์ ขอบ 90° ไม่มีการ bevel/round — บ้าน, เฟอร์นิเจอร์, ต้นไม้, ตัวละคร (แกะ) ล้วนเป็นบล็อก |
| **แสงอ้อม (GI) อุ่น** | เงาในร่มไม่ดำสนิท — มีแสงสะท้อนสีอุ่นจากพื้นไม้/ผนังเติมเข้าไป (bounce light) เห็นชัดในรูปครัว |
| **Directional sun แรง + god rays** | ลำแสงพุ่งผ่านกรอบหน้าต่าง เป็นแท่งแสงมองเห็นได้ (volumetric) — รูปครัวญี่ปุ่นและห้องนอน |
| **เงานุ่ม + contact shadow** | เงามีขอบฟุ้ง (soft/PCF) และมีเงาเข้มเล็กๆ ตรงจุดที่วัตถุแตะพื้น (AO/contact) |
| **Atmospheric fog / haze** | ระยะไกลจางเป็นหมอกทอง (รูป aerial village); ในร่มมีฝุ่นแสงลอย (dust motes) |
| **PBR material** | ไม้มี grain + specular นุ่ม, สแตนเลสตู้เย็นสะท้อนแบบ metallic, กระจกโปร่ง — วัสดุตอบสนองแสงต่างกันจริง |
| **Bloom อบอุ่น** | หน้าต่างและโคมไฟ (emissive) เรืองฟุ้งนุ่มๆ ไม่โอเวอร์ |
| **Golden-hour palette** | ทุกรูปอยู่ในโทนทอง/อำพัน/น้ำตาลอุ่น — ไม่มีรูปไหนโทนเย็น/กลางวันจัด/กลางคืน |
| **Tone-mapping filmic** | ไฮไลต์ไม่ไหม้ (หน้าต่างสว่างแต่ยังเห็นเกรน), เงายังมีรายละเอียด — dynamic range แบบฟิล์ม |

**Reference breakdown:**

- **รูป 1 — Aerial village (golden autumn):** โชว์ระยะไกล → godrays กว้าง, fog ทอง, **น้ำสะท้อน** (แสงอาทิตย์เป็นเส้นระยิบบนแม่น้ำ = specular sun glint), soft shadow ของบ้านทอดยาว, ใบไม้ autumn palette.
- **รูป 2 — ครัวญี่ปุ่น (window-lit):** โชว์ **DOF** ชัด (ฉากหลังละลาย), **volumetric god rays** ผ่านหน้าต่างบานเลื่อน, dust motes ลอย, contact shadow ใต้ถ้วยชาม, bloom ขอบหน้าต่าง.
- **รูป 3 — ครัวไม้ warm (sunset flood):** โชว์ **GI/bounce** จัดเต็ม (แสงอุ่นเติมทั้งห้อง), atmospheric haze ในร่ม, โคมไฟ emissive, PBR สแตนเลส vs ไม้, ceiling beam ทอดเงา.
- **รูป 4 — ห้องนอน close-up (crisp shadow):** โชว์ **hard-edge window shadow** ทาบพื้น (เงาลายกรอบหน้าต่างคมชัด), emissive block (บล็อกเพชรเรืองฟ้า), contact shadow, teal×wood palette — พิสูจน์ว่าลุคเดียวกันทำได้ทั้งเงานุ่มและเงาคม.

---

## 2. Render Feature Checklist (ฟีเจอร์ที่ทำให้ได้ลุคนี้)

เรียงจาก "ขาดไม่ได้" → "ปรุงรส". คอลัมน์ tier ดูข้อ 3.

| # | ฟีเจอร์ | ทำหน้าที่อะไรในลุคนี้ | Ultra | Lite |
|---|---|---|---|---|
| 1 | **Directional sun + shadow map** | แหล่งแสงหลัก golden-hour; ทิศ ~15–25° เหนือขอบฟ้า | ✅ | ✅ |
| 2 | **Soft shadow (PCF / PCSS)** | ขอบเงาฟุ้ง ยิ่งไกลจาก occluder ยิ่งนุ่ม (PCSS = penumbra จริง) | PCSS | PCF 3×3 |
| 3 | **Contact shadow / AO (SSAO/GTAO)** | เงาเข้มตรงรอยต่อบล็อก + ใต้วัตถุ = ให้มิติกับ voxel | GTAO | SSAO (ลด sample) |
| 4 | **Global Illumination (แสงอ้อม)** | bounce สีอุ่นเข้าที่ร่ม = หัวใจความ "อยู่สบาย" | RT/DDGI/VXGI | baked lightmap + AO probe |
| 5 | **Volumetric god rays** | ลำแสงมองเห็นได้ผ่านหน้าต่าง/ใบไม้ | true volumetric (froxel) | screen-space light shafts |
| 6 | **Atmospheric fog / haze** | ระยะไกลจางเป็นหมอกทอง + haze ในร่ม | height + volumetric fog | distance fog (exp²) |
| 7 | **PBR material (albedo/normal/specular/roughness)** | ไม้/หิน/โลหะ/กระจกตอบแสงต่างกันจริง | full metallic-roughness | albedo + roughness (normal ออปชัน) |
| 8 | **Bloom** | หน้าต่าง+โคมไฟ (emissive) เรืองนุ่ม | HDR threshold bloom | cheap kawase bloom |
| 9 | **Depth of Field** | ฉากหลังละลาย = ความ cinematic (รูปครัว) | bokeh DOF | ปิด หรือ gaussian เบาๆ เฉพาะ hero shot |
| 10 | **Tone-mapping (ACES / filmic)** | กันไฮไลต์ไหม้ + คุมโทนอุ่น | ACES + exposure | Reinhard/filmic เบา |
| 11 | **Water reflection (SSR / planar)** | แม่น้ำสะท้อนฟ้า + sun glint (รูป aerial) | SSR + planar hybrid | screen-space reflection ง่าย / cubemap |
| 12 | **Emissive materials** | บล็อกเรืองแสง (โคม, เพชร) เป็นแหล่งแสงจุด | emissive → light + bloom | emissive → bloom เท่านั้น (ไม่ cast light) |
| 13 | **Dust motes / particles** | ฝุ่นลอยในลำแสง = ชีวิตชีวา (รูปครัว) | GPU particles ใน light shaft | sprite particle เบาๆ (hero shot) |
| 14 | **Color grading LUT** | ล็อกโทน golden-hour ให้คงเส้น | per-scene LUT | global warm LUT |

> **กฎ identity:** ฟีเจอร์ #1, #2, #3, #7, #10 คือ "แกน" — ตัดออกเมื่อไหร่ลุคหายทันที.
> #4 (GI) ตัดเป็น baked ได้แต่ห้ามตัดหมด (ร่มจะดำ = เสียความอุ่น). ที่เหลือเป็นชั้น "เสน่ห์" ปรับได้.

---

## 3. Two-Tier Spec — Ultra vs Lite

เป้า: **ลุคเดียวกันคนละงบ**. ทั้งสอง tier ต้องอ่านออกว่าเป็น Voxelforge จากภาพนิ่งเดียว.

### 🌟 Ultra — Native / Desktop (Bevy + wgpu native → Vulkan / Metal / DX12 desktop GPU)

- **เป้า FPS:** 60fps @ 1080p บน GPU กลาง (GTX 1660 / RX 5600 ขึ้นไป), 30fps @ 4K บนการ์ดแรง.
- **แสง:** Directional sun + **PCSS soft shadow** + **GTAO** + **GI จริง** (RT ถ้ามี ไม่งั้น DDGI/VXGI) + **true volumetric fog & god rays** (froxel).
- **Material:** PBR เต็ม metallic-roughness + normal map + parallax เบาๆ บนหิน/ไม้.
- **Post:** bokeh DOF, HDR bloom, **ACES tone-map** + per-scene LUT, SSR น้ำ + planar สำหรับ hero, GPU dust particles.
- **Emissive:** cast แสงจริง (โคม/เพชร = point light + bloom).
- **Texture:** 512² ต่อบล็อก (option HD 1024² pack), normal + roughness ครบ.

### 🪶 Lite — Web / Plugin (same Bevy + wgpu → WASM · WebGPU primary, WebGL2 fallback backend · panel webview, มือถือ)

- **เป้า FPS:** 60fps บน integrated GPU / mobile mid-range @ 720–1080p.
- **แสง:** Directional sun + **PCF 3×3 soft shadow** (1–2 cascade) + **SSAO ลด sample** + **baked lightmap/AO probe** แทน GI + **screen-space light shafts** (fake god rays) + distance/height fog แบบ exp².
- **Material:** albedo + roughness (normal เป็น option ต่อ device), ไม่มี parallax.
- **Post:** ปิด DOF (เปิดเฉพาะ hero screenshot), **kawase bloom** ถูกๆ, filmic tone-map เบา, SSR ง่ายหรือ cubemap สำหรับน้ำ, global warm LUT.
- **Emissive:** bloom only (ไม่ cast แสง — ประหยัด light loop).
- **Texture:** 128–256² ต่อบล็อก, atlas รวม, ไม่มี normal บน low-end.

### สิ่งที่ **ตัดได้** ใน Lite โดยไม่เสีย identity
DOF, RT/volumetric GI, true volumetric fog, bokeh, dust particles, planar reflection, emissive-as-light, parallax, HD texture.

### สิ่งที่ **ห้ามตัด** (ทั้งสอง tier ต้องมี)
Voxel hard-edge geometry · directional golden sun · soft shadow (อย่างน้อย PCF) · contact shadow/AO · แสงอ้อมในร่ม (จริงหรือ baked) · PBR ขั้นต่ำ (roughness) · warm tone-map/LUT · bloom หน้าต่าง+โคม.

> **เกณฑ์ตัดสิน "ยังเป็น Voxelforge ไหม":** เอาภาพนิ่ง Lite ไปวางข้าง Ultra — ถ้าคนดูบอกได้ว่า
> "เกมเดียวกัน แค่เครื่องเบากว่า" = ผ่าน. ถ้าดูเหมือน "Minecraft vanilla ไม่มี shader" = ตัดลึกไป.

---

## 4. Palette · Mood · Texture

### Palette — Golden Hour (แกนหลักทุกฉาก)

| บทบาท | สี | Hex (แนะนำ) |
|---|---|---|
| Key light / sun | Amber-gold | `#F4B860` → `#FFD98A` |
| Warm bounce (GI) | Honey / caramel | `#C88A4A` |
| Wood mid | Walnut brown | `#6B4A2E` |
| Wood dark | Espresso | `#3A2716` |
| Stone / wall | Warm grey-beige | `#B9A98C` |
| Sky (day) | Pale warm blue | `#Bcd3e0` |
| Fog / haze | Warm cream | `#E8D8B8` (จางลงตามระยะ) |
| **Accent 1** | Foliage gold-green | `#8A8A3C` (autumn) |
| **Accent 2 (cool)** | Teal / diamond glow | `#4FC9D6` (ตัดกับอุ่น — โคม/พืช/พรม) |
| Shadow (ไม่ดำสนิท) | Warm brown-violet | `#2A2030` |

> **หลัก:** ~85% ของเฟรมเป็นโทนอุ่น (ทอง→น้ำตาล), ปล่อย **teal accent** จุดเล็กๆ ตัดให้ภาพมีชีวิต
> (เห็นชัดในรูปห้องนอน: ผนัง teal + บล็อกเพชร กับพื้นไม้อุ่น). อย่าให้ teal เกิน ~10–15% ของเฟรม.

### Mood
อบอุ่น · เงียบสงบ · น่าอยู่ (cozy) · "แสงเย็นวันหยุด/เช้าตรู่" · nostalgic. ไม่ใช่ dramatic/horror/neon.
เป้าอารมณ์: เห็นแล้วอยากเข้าไป "นั่งจิบชา". ทุกฉากควรรู้สึกว่ามี "เวลาของวัน" ชัด (แดดเฉียง).

### Texture Resolution (แนะนำ)
- **Ultra:** 512² base (HD pack 1024²) — albedo + normal + roughness (+ AO/height ตามวัสดุ).
- **Lite:** 128–256² atlas — albedo + roughness; normal เป็น optional per-device.
- **สไตล์ texture:** ยัง "pixel-ish" อ่านออกว่าเป็นบล็อก (ไม่ใช่ photo-real texture) — grain ไม้/หินชัด
  แต่ pixel density พอให้ shader เล่นแสงได้. อย่า smooth จน geometry ดูไม่ใช่ voxel.
- **Filtering:** nearest หรือ nearest+mip (คง pixel edge) — ไม่ใช้ bilinear เต็มจนเบลอบล็อก.

---

## 5. Handoff Notes (สำหรับ graphics engineer)

1. **Golden Beauty-Shot Target = ด่านรับ visual ของ Bevy spike** — ครัวไม้ window-lit 1 ฉาก
   เป็น golden test เดียว. Poppy เรนเดอร์ใน Bevy spike ให้ผ่าน pass-list (แสง key/bounce/godray/DOF/tone-map)
   แล้วเทียบกับ reference frame — เกณฑ์เต็มอยู่ที่ `docs/golden-beauty-shot.md`. ผ่านฉากนี้ก่อน แล้วค่อย generalize.
   **ไม่ต้อง prototype ลุคนี้ในเครื่องมืออื่น** — visual validation เกิดใน Bevy spike ที่เดียว เทียบ FPS + ลุค apples-to-apples.
2. **Shadow เป็นด่านแรก** — voxel ที่ไม่มี soft shadow + AO จะดู "แบน" ทันที. ลงทุนตรงนี้ก่อน bloom/DOF.
3. **GI คือความต่างระหว่าง "สวย" กับ "แค่มี shader"** — ถ้า Lite ทำ RT ไม่ไหว ให้ลงแรงกับ baked lightmap
   คุณภาพดี + AO probe แทน อย่าปล่อยร่มดำ.
4. **แยก LUT/tone-map ออกเป็น config** — ให้ art ปรับโทน golden-hour ได้โดยไม่แตะ shader.
5. **Emissive → light เป็นสวิตช์ tier** (Ultra cast จริง / Lite bloom-only) เพื่อคุมงบ light loop.
6. Mood board + reference 4 รูปแนบไว้ที่ `docs/assets/` และ `workspace/uploads/1784911379*` — ใช้เป็น visual truth.

---

_เอกสารมีชีวิต — ปรับได้เมื่อ engine พิสูจน์ว่าอะไรทำไหว/ไม่ไหวจริงบนเครื่องเป้าหมาย. โยน feedback กลับมาได้เลยครับ. — Flamingo (Designer)_
