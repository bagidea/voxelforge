# Look Tier Spec — อะไรตัดแล้วตาย อะไรตัดแล้วรอด

> **เจ้าของเอกสาร:** Flamingo (look lane) · **ผู้ใช้งานหลัก:** Rose (implement tier ใน `client/src/look.rs`)
> **สร้าง:** 2026-07-31 · **แก้ครั้งที่ 2:** 2026-07-31 (หลัง review — แก้ค่า Ultra ที่อ้างผิดแหล่ง,
> ลำดับตัดที่ขัดกันเอง, และเพิ่มผลตรวจรับ tier ladder ที่ implement ไปแล้ว)
>
> อ่านคู่กับ: [`look-bible.md`](look-bible.md) §2–§3 · [`look-acceptance-rubric.md`](look-acceptance-rubric.md) §GATE
> · [`golden-beauty-shot.md`](golden-beauty-shot.md) §go/no-go · `scripts/render_wide_hero.sh`

---

## 0. ทำไมเอกสารนี้ต้องเขียนใหม่ (ไม่ใช่แค่ก็อป look-bible §3)

`look-bible.md` §3 นิยาม tier ไว้เป็น **Ultra = native / Lite = web-wasm** — คือแบ่งตาม **แพลตฟอร์ม**.
วันที่ 2026-07-31 CEO ยกเลิก web target ถาวร (`65b1d12`, `GAME-VISION.md` — "No wasm / web / browser
export"). นิยามเดิมจึงตายไปครึ่งหนึ่ง.

**นิยามใหม่: tier = งบ GPU บนเครื่องเดสก์ท็อป ไม่ใช่แพลตฟอร์ม.** เป้า Ultra ยังเป็นตัวเดิมจาก
`GAME-VISION.md` — 60fps @1080p บน GTX 1660+ — tier ที่ต่ำลงมารับการ์ดต่ำกว่านั้น / iGPU และคนที่อยาก
ได้ 1440p–4K หรือ 120fps บนการ์ดกลาง.

สิ่งที่ **ไม่เปลี่ยน**: กฎ identity ของ look-bible — "เอาภาพนิ่ง tier ต่ำไปวางข้าง Ultra ถ้าคนดูบอกว่า
*เกมเดียวกัน แค่เครื่องเบากว่า* = ผ่าน. ถ้าดูเหมือน *Minecraft vanilla ไม่มี shader* = ตัดลึกไป."

---

## 1. คำตอบสั้นที่สุด — ตัดแล้วตาย vs ตัดแล้วรอด

### 🔴 ตัดแล้ว "ตาย" — ห้ามแตะทุก tier รวมถึง Low

| ชั้น | component ใน `look.rs` | ตัดแล้วเกิดอะไร | GPU cost |
|---|---|---|---|
| **Filmic tone-map** | `Tonemapping::AcesFitted` | หน้าต่าง/ท้องฟ้าคลิปขาวตัน → **G5 ตก** และ golden hour กลายเป็นส้มไหม้ | ~0 |
| **Color grade** | `ColorGrading` (temp/sat/midtone contrast/highlight shoulder) | นี่**คือ**ตัวล็อกโทน golden-hour (look-bible ฟีเจอร์ #14) ถอดแล้วเฟรมออกเทา-เย็น → G6 ตก | ~0 |
| **Contact AO** | `ScreenSpaceAmbientOcclusion` — **ลดคุณภาพได้ ลบชั้นไม่ได้** | บล็อกลอย ไม่ติดพื้น → **G4 ตกครึ่งหนึ่ง** + เป็นหน้าตา "Minecraft ไม่มี shader" ตรงตัว | ปรับได้ |
| **Soft shadow ≥3px** | `ShadowFilteringMethod` (Temporal หรือ Gaussian) — **ต้องมีเสมอ ไม่ปล่อย default** | ขอบเงาคม 1px → **G4 ตกอีกครึ่ง** | ต่ำ |
| **Window/emissive bloom** | `Bloom` | look-bible §107 ระบุ "bloom หน้าต่าง+โคม" อยู่ในรายการห้ามตัด | ต่ำมาก |
| **Warm bounce ในร่ม** | *(ไม่ได้อยู่ใน look.rs — เป็นไฟของฝั่งเกม)* | ร่มดำตัน → **G3 ตก** ซึ่ง rubric ระบุว่าเป็นด่านที่พลาดบ่อยที่สุด | ~0 |
| **`Msaa::Off`** | `Msaa::Off` | ไม่ใช่เรื่องสวยงาม — SSAO ของ Bevy **บังคับ** ให้ MSAA ปิด. MSAA จึงไม่ใช่ knob ของ tier | — |

> 4 ใน 7 บรรทัดนี้ราคาเกือบ 0 (tone-map, grade, bloom, ambient). **สิ่งที่ทำให้ภาพ "เป็น Voxelforge"
> แทบไม่กินเฟรมเรตเลย** ที่กินคือชั้นเสน่ห์ล้วน ๆ. แต่ระวังกับดัก: **grade เป็นเงื่อนไข *จำเป็น*
> ไม่ใช่ *เพียงพอ*** — เฟรมที่มีแค่ grade สวยแต่ไม่มี AO/soft shadow ยังตก G4 อยู่ดี (ดู §5).

### 🟢 ตัดแล้ว "รอด" — เรียงตามลำดับที่ควรตัดก่อน-หลัง

ลำดับนี้คือลำดับเดียวกับตาราง §2 (ตัดข้อ 1–2 ที่ High, ข้อ 3–4 ที่ Medium, ข้อ 5–6 ที่ Low):

| # | ตัดอะไร | ตัดที่ tier | ทำไมตัดได้ | เสียอะไร |
|---|---|---|---|---|
| 1 | **PCSS** (`soft_shadow_size`) | High | `hero.rs:558-570` วัดไว้ว่า penumbra ~6px ที่เห็นจริงมาจาก `ShadowFilteringMethod::Temporal` + TAA + shadow map 4K **ไม่ใช่ PCSS** — Bevy clamp `blur_size` ไปที่ floor 0.5 แล้ววัดนิ่งตั้งแต่ soft=0.02 ถึง 400 | ⚠️ **ดู caveat ข้างล่าง** |
| 2 | **VolumetricFog `step_count`** 96→32 | High | ลด*คุณภาพ*ลำแสง ไม่ใช่ลบชั้น — ray-march เป็น pass ที่แพงที่สุดในสแต็ก | ลำแสงมี banding ขึ้น |
| 3 | **Depth of Field** | Medium | rubric §213 อนุญาตให้ pass 5 = N/A แล้วปรับฐาน 100→92 ตรง ๆ | ความ cinematic; gate ยังผ่านครบ 6 |
| 4 | **VolumetricFog ทั้งชั้น** | Medium | god ray = **pass 3 ของชั้น B (คะแนน) ไม่ใช่แกน gate** — G1–G6 ไม่มีข้อไหนวัดลำแสง | เสียคะแนน pass 3; **ไม่ตก gate** |
| 5 | **TAA** + `Temporal`→`Gaussian` | Low | Gaussian เป็น multi-tap blur ไม่ต้องใช้ history → ยังได้ขอบนุ่ม ≥3px โดยไม่มี TAA | ขอบ aliasing กลับมา |
| 6 | **SSAO คุณภาพ** Ultra→High→Medium→Low | ทุกชั้น | ชั้นยังอยู่ = G4 ยังผ่าน แค่ crease นุ่มลง | มิติ contact ลดลง |

> ⚠️ **Caveat ของข้อ 1 ที่ต้องพกไปด้วยทุกครั้งที่อ้าง:** หลักฐาน "PCSS ตัดได้เกือบฟรี" ผูกกับ
> **สเกลห้อง**. `hero.rs:558-570` เขียนเหตุผลไว้ตรง ๆ ว่า *"in a room this small the directional
> blocker→receiver depth gap is tiny, so Bevy clamps blur_size to its 0.5 floor"* — วัดในห้อง 16×16.
> `look.rs` วิ่งในโลกเปิดที่ระยะ occluder→receiver ใหญ่กว่านั้นมาก (ยอดเขาทอดเงาลงหุบ) ซึ่ง**เป็น
> เงื่อนไขที่ PCSS ออกแบบมาให้ทำงานพอดี**. ข้อสรุปนี้จึงยัง **ไม่ได้พิสูจน์ว่า transfer** — ให้ถือเป็น
> สมมติฐานที่มีน้ำหนัก และ **วัดซ้ำในโลกเปิดก่อนล็อก**. ถ้าในโลกเปิด PCSS สร้าง penumbra ที่ไล่ตาม
> ระยะจริง มันจะเลื่อนจาก "ตัดก่อนอันดับ 1" ไปเป็น "ตัดทีหลัง" ทันที
>
> วิธีวัด: เรนเดอร์เฟรมเดียวกันที่ `soft_shadow_size = Some(3.0)` vs `None` โดยให้มีเงาที่ occluder
> อยู่สูงจากพื้น ≥10m ในเฟรม แล้ววัดความกว้าง transition ของขอบเงาเป็นพิกเซล. ต่างกัน <1px = ตัดได้ฟรีจริง

---

## 2. ตาราง tier — map ตรงกับ component ใน `look.rs`

| knob | **Ultra** | **High** (default) | **Medium** | **Low** |
|---|---|---|---|---|
| เครื่องเป้าหมาย | 1660+ @1080p60 หรือดีกว่า | 1660 @1080p60 (median Steam) | GTX 1050 / iGPU แรง | iGPU / การ์ดเก่า |
| `Tonemapping` | AcesFitted | ← เหมือนกัน | ← | ← |
| `ColorGrading` (ทุกค่า §4) | signed-off | ← **เหมือนกันเป๊ะ** | ← | ← |
| `Bloom` | NATURAL @ 0.26 | ← | ← | ← |
| `Msaa` | Off | Off | Off | Off |
| `DistanceFog` | on | on | on | **on** (ราคาเกือบ 0 ไม่มีเหตุผลให้ตัด) |
| `ShadowFilteringMethod` | Temporal | Temporal | Temporal | **Gaussian** |
| `soft_shadow_size` (PCSS) | 3.0 | **ปิด** | ปิด | ปิด |
| `TemporalAntiAliasing` | on | on | on | **ปิด** |
| `ScreenSpaceAmbientOcclusion` | **Ultra**, 1.45 | High, 1.45 | Medium, 1.45 | **Low, 1.45** (ห้ามไม่มี) |
| `DepthOfField` | Bokeh, f/8 (§4) | Bokeh, f/8 | **ปิด** | ปิด |
| `VolumetricFog` | step 96, jitter 0.6 | **step 32**, jitter 0.6 | **ปิด** | ปิด |
| `VolumetricLight` (sun) | on | on | **ปิด** | ปิด |
| `DirectionalLightShadowMap` | **4096 — ต้อง insert เอง** | ไม่แตะ (Bevy default 2048) | ไม่แตะ | ไม่แตะ |

**สังเกต 4 อย่าง:**

1. **แถวสีของภาพ (tone-map / grade / bloom / distance fog) เหมือนกันหมดทั้ง 4 tier** ตั้งใจ —
   นี่คือเหตุผลเดียวที่ screenshot ของ Low ยังอ่านออกว่าเป็น Voxelforge และมันฟรี
2. **`constant_object_thickness: 1.45` ไม่ลดตาม tier** มันไม่ใช่ค่าที่แพง — เป็นค่า "ความหนาระดับ
   voxel" ที่ทำให้ของนั่งบนพื้น. ลด *sample count* ได้ ลดความหนาแล้วของจะลอย
3. **`DirectionalLightShadowMap` ไม่ได้อยู่ใน play path เลย** — `main.rs:373` ที่ insert 4096 อยู่ใน
   `if cfg.hero {}` (เลน hero-shot) เกมจริงเดินทาง `else` แล้ววิ่งที่ Bevy default 2048.
   คอมเมนต์บรรทัดนั้นบอกเองว่า 4K มีไว้ให้ "PCSS penumbra มี texel พอไม่ stair-step" — แปลว่า
   **PCSS 3.0 ที่เซ็นรับไว้ ถูกเซ็นบน 4K map** เพราะฉะนั้น Ultra ต้อง insert `DirectionalLightShadowMap
   { size: 4096 }` **เองใน `look.rs`** ไม่งั้นได้ PCSS ที่ไม่ใช่คอนฟิกที่ผ่าน gate. ตัวนี้อยู่ในเลนเรา
   (insert ใน look.rs) ไม่ต้องขอ ack ใคร
4. **`jitter` ไม่ต้องปิดที่ tier ไหน** เพราะ VolumetricFog มีเฉพาะ Ultra/High ซึ่งทั้งคู่มี TAA อยู่แล้ว

---

## 3. กฎการผูกกัน (coupling) — ห้ามตัดข้ามกฎนี้

1. **ปิด TAA ⇒ ต้องปิด PCSS ด้วย และ SSAO ต้องไม่ใช่ Ultra.** PCSS กับ SSAO Ultra เป็น stochastic
   ทั้งคู่ — เฟรมเดียวคือ noise. TAA คือตัวสะสมให้เนียน (`hero.rs:794-797`). ปิด TAA อย่างเดียว =
   ส่ง noise ให้ผู้เล่นดูโดยเฟรมเรตแทบไม่ขึ้น
2. **SSAO เปิด ⇒ `Msaa::Off` เสมอ** ข้อบังคับของ engine ไม่ใช่ทางเลือก
3. **ปิด VolumetricFog ⇒ ปิด `VolumetricLight` บนดวงอาทิตย์ด้วย** ไม่งั้นจ่ายค่า in-scattering ฟรี ๆ
4. **PCSS เปิด ⇒ shadow map ต้อง 4096** (ข้อ 3 ของ §2)
5. **DoF เปิด ⇒ `focal_distance` ต้องตามกล้องจริง** ไม่ใช่ค่าคงที่ (§4)

---

## 4. Ultra = ค่าที่เซ็นรับแล้ว (single source of truth)

Tier Ultra **ไม่ใช่ tier ที่ตั้งค่าใหม่** — คือชุดค่าที่ผ่าน gate บนเฟรม `docs/assets/wide-hero-final.png`
(CEO-approved tilt-down). Tier อื่นนิยามเป็น **"Ultra ลบอะไรออก"** เท่านั้น ห้ามมีชุดตัวเลขคู่ขนาน.

> ⚠️ **"เซ็นรับแล้ว" = ค่าที่เฟรมนั้นเรนเดอร์ออกมาจริง ซึ่งคือค่าใน `scripts/render_wide_hero.sh`
> ไม่ใช่ `unwrap_or(...)` ใน `hero.rs`** — recipe override baked default ทับไปแล้ว. นี่คือกับดักที่
> ผมเองเหยียบในฉบับแรก (อ่าน baked default ของ path *narrow* มาเซ็นรับ ทั้งที่เฟรมเดินทาง *wide*).

| knob | ค่า | ที่มา |
|---|---|---|
| `temperature` | **0.02** | `render_wide_hero.sh:55` `VOXELFORGE_GRADE=0.02,1.00,1.30` |
| `post_saturation` | **1.00** | เดียวกัน |
| midtone `contrast` | 1.30 | เดียวกัน |
| shadows `contrast` | **1.0** | ถือ neutral โดยตั้งใจ — ใส่ contrast ที่ shadows แล้ว p05-L ร่วง 13.7%→3.3% (วัดแล้ว) |
| highlights `contrast` | **1.0** | `hero.rs:711` — path `wide` ใช้ `(1.0, shoulder)` |
| highlights `gain` | **0.64** | `render_wide_hero.sh:56` `VOXELFORGE_SHOULDER=0.64` |
| `Bloom.intensity` | 0.26 บน `NATURAL` | baked `hero.rs:800-811` (ไม่มี env override) |
| SSAO | Ultra, thickness 1.45 | baked `hero.rs:823-834` |
| `soft_shadow_size` | 3.0 **+ shadow map 4096** | baked `hero.rs:574` + `main.rs:373` |
| `sensor_height` | 0.35 | จำเป็น — sensor default ~18.6mm ทำให้ CoC เกือบศูนย์ที่สเกลนี้ |
| DoF `aperture_f_stops` | **f/8** *(เป็นการตัดสินใหม่ ไม่ใช่ค่าที่เซ็นรับ — ดูข้างล่าง)* | |
| DoF `focal_distance` | ตาม orbit boom (dynamic) | ✅ ตัดสินใจของ Rose ถูก |

### จุดที่ `look.rs` ตอนนี้ยังไม่ตรง (3 ค่า)

`grade::TEMPERATURE = 0.10` → ต้องเป็น **0.02** · `grade::POST_SATURATION = 1.02` → ต้องเป็น **1.00**
· `grade::HIGHLIGHT_CONTRAST = 1.30` → ต้องเป็น **1.0**

สองตัวแรกคือ baked default ของ path *narrow* (`hero.rs:698` `unwrap_or([0.10, 1.02, 1.30])`) ซึ่ง
recipe ทับไปแล้ว. ตัวที่สามหนักกว่า: `hero.rs:711` เขียนว่า
`let (hi_contrast, hi_gain) = if wide { (1.0, shoulder) } else { (g_contrast, 1.0) };` — contrast
กับ gain บน highlight เป็น **either/or ตามดีไซน์ ไม่เคยเปิดพร้อมกัน**. ใส่ 1.30 คู่กับ 0.64 คือ
**บีบสองชั้น** — contrast ดันปลายสว่างออกจาก mid ก่อน แล้ว gain ดึงกลับลง 36% ผลคือหน้าต่างทึม
gradient หาย = แกน **G5**.

> **แต่ระวังอย่าเชื่อค่าพวกนี้เกินตัว:** เฟรมที่ล็อกไว้เป็นห้องครัว 16×16 ที่มี bounce card 2 ดวง +
> `AMBCOLOR` อุ่น + `BLUESCALE=0.85` ซึ่งเกมจริงไม่มี. ค่า grade ที่ *ถูกต้อง* สำหรับเกมอาจไม่ใช่
> 0.02/1.00 — **แต่ทางที่ถูกคือเรนเดอร์เฟรมเกม แล้ววัด แล้วเซ็นค่าใหม่** ไม่ใช่หยิบ baked default
> อีกชุดมาใช้เงียบ ๆ. จนกว่าจะมีเฟรมเกมที่วัดแล้ว ให้ยึด 0.02/1.00/1.0 เพราะนั่นคือชุดเดียวที่มี
> หลักฐานว่าผ่าน gate

### เรื่อง aperture — f/8 (ทบทวนน้ำหนักหลักฐานใหม่)

เห็นด้วยกับ *เหตุผล* ของ Rose (กล้องที่คนเล่นอยู่หลัง ไม่ใช่ภาพนิ่ง) แต่ f/4.0 ยังตื้นไปหนึ่งช่วง.
แยกหลักฐานตามน้ำหนักจริง:

- **วัดจริง:** `note-to-poppy-dof-fix.md` sweep ช่วง **f/2.8–f/4.5** เท่านั้น พบว่า subject คมขึ้น
  +60% โดย wall LapVar นิ่งที่ ~6.0 ตลอดช่วง. ประโยค *"narrower doesn't wash the bg until f/8"*
  ในโน้ตนั้น **เป็นการคาดการณ์ ไม่ใช่แถวที่วัด** — ฉบับแรกของเอกสารนี้เขียนว่า "วัดไว้ว่านิ่งจนถึง
  f/8" ซึ่งเกินหลักฐาน แก้แล้ว
- **หลักฐานที่แข็งกว่า (และเป็นตัวตัดสินจริง):** เฟรม establishing ที่ล็อกไว้ใช้ **f/10** พร้อม
  เหตุผลเขียนกำกับว่า "deep so the establishing floor stays crisp voxel geometry (**G1**)"
  (`golden-beauty-shot.md` แถว DOF) — และเกมคือมุม establishing ไม่ใช่ hero tabletop
- **เหตุผลเชิงกลไก:** `focal_distance` ผูกกับ boom ที่ `camera_boom` หดเหลือ ~2m เวลาชิดกำแพง →
  ที่ f/4 + sensor 0.35 ทุกอย่างเลย ~5m ละลาย = **voxel grid อ่านไม่ออก = G1 ตก** ในจังหวะที่ผู้เล่น
  เจอบ่อยที่สุด

f/8 = อยู่ระหว่าง sweet spot ที่วัดได้ (4.5) กับค่าที่เฟรม establishing ใช้จริง (10) — **เป็นการ
ตัดสินโดยเจ้าของ look ไม่ใช่ค่าที่ผ่าน gate มาแล้ว** ถ้าเรนเดอร์แล้ววัดได้ว่า f/6 หรือ f/10 ดีกว่า
ให้ทับตัวนี้ได้เลย. อยากได้ bokeh จัดกว่านี้ค่อยแยกเป็น photo mode

---

## 5. ตรวจรับ tier ladder ที่ implement ไปแล้ว (`look.rs` working tree, 430 บรรทัด, ยังไม่ commit)

โครงสร้างดีมาก 4 อย่าง: `LookQuality` เป็น resource ตัวเดียว · `LookStack` เป็น tuple เดียวสำหรับ
`remove` ทำให้สลับ tier ตอนรันได้จริง · `apply_look_to_sun` **ถอด `VolumetricLight` ออกเมื่อไม่ใช่
Ultra** (ตรงกับกฎ coupling ข้อ 3 พอดี) · F7 + `VOXELFORGE_LOOK_QUALITY` ทำให้ Poppy โปรไฟล์ทีละ tier ได้

**แต่ ladder ที่ลงไปชนตาราง 🔴 ของ §1 อยู่ 3 จุด — และ 2 ใน 3 อยู่บน tier ที่ผู้เล่นได้เป็น default:**

| # | ปัญหา | อยู่ที่ | ผล |
|---|---|---|---|
| **1** | **`Low` ไม่มี `ScreenSpaceAmbientOcclusion` และไม่มี `ShadowFilteringMethod` เลย** | `look.rs` แขน `LookQuality::Low` | **G4 ตกทั้งสองครึ่ง** — ไม่มี contact AO และตกไปใช้ Bevy default (`Hardware2x2`) ซึ่งไม่ใช่ขอบนุ่ม ≥3px. ต้องเติม `SSAO { Low, 1.45 }` + `ShadowFilteringMethod::Gaussian` |
| **2** | **`Medium` ใช้ `Hardware2x2`** | แขน `Medium` | G4 ครึ่งขอบเงา **สุ่มเสี่ยง** — 2×2 PCF อาจไม่ถึง 3px. ให้เปลี่ยนเป็น `Temporal` (มี TAA อยู่แล้วในแขนนี้ ไม่มีค่าใช้จ่ายเพิ่ม) |
| **3** | **`High` (default) ไม่มี `VolumetricFog` แต่ยังเปิด PCSS** | แขน `High` + `apply_look_to_sun` | ขัดลำดับตัด: ตัด god ray (แพงจริง แต่เป็นชั้นเสน่ห์ที่เห็นชัด) ทิ้งทั้งชั้น ขณะที่ยังจ่ายค่า PCSS ที่ `hero.rs` วัดว่าแทบไม่ให้อะไร. ให้สลับ — **ปิด PCSS, เปิด VolumetricFog ที่ `step_count: 32`** |
| — | `apply_look_to_cameras` filter `With<Camera3d>` | | ควรเป็น `With<crate::OrbitCam>` — ดู §6 ข้อ 3 |

**ข้อ 3 นี้เป็นความผิดของเอกสาร ไม่ใช่ของ Rose** — ฉบับแรกของ §1 จัด PCSS เป็น "ตัดก่อนอันดับ 1"
แต่ตาราง §2 กลับคง PCSS ไว้ที่ High แล้วไปตัดอย่างอื่นก่อน. อ่านแล้วขัดกันเอง แก้ในฉบับนี้แล้ว
(§1 กับ §2 ตรงกันบรรทัดต่อบรรทัด)

**และแก้จุดยืนตัวเองอีกข้อ:** ฉบับแรกเขียนว่า "VolumetricFog ห้ามปิดสนิท" โดยอ้าง rubric §216 —
แรงเกินไป. §216 เขียนไว้สำหรับการ **เกรด beauty shot**, และ god ray เป็น **pass 3 ของชั้น B (คะแนน)
ไม่ใช่แกน gate** — G1–G6 ไม่มีข้อไหนวัดลำแสงเลย. ปิดสนิทที่ Medium/Low จึง **เสียคะแนน ไม่ตก gate**
ซึ่งเป็นสิ่งที่ tier ต่ำมีสิทธิ์ทำ. ที่ยังยืน: **ห้ามปิดที่ High** เพราะ High คือเฟรมที่คนส่วนใหญ่เห็น
และ atmosphere pin คือสิ่งที่ดัน micro-contrast จาก 4.9 near-miss เป็น 5.92 PASS

---

## 6. เกณฑ์ตรวจรับต่อ tier

| tier | GATE G1–G6 | AAA Score (ชั้น B) | ฐาน |
|---|---|---|---|
| **Ultra** | ผ่านครบ 6/6 | ≥ 83 (AA ขึ้นไป) | 100 |
| **High** | ผ่านครบ 6/6 | ≥ 78 | 100 |
| **Medium** | ผ่านครบ 6/6 | ≥ 72 | **92** (DoF = N/A ตาม rubric §213) |
| **Low** | ผ่านครบ 6/6 | ≥ 60 (A) | 92 |

**กฎเหล็กจาก rubric §216: แกน gate ทั้ง 6 ห้าม N/A ทุก tier.** tier ต่ำได้สิทธิ์ "คะแนนเสน่ห์ต่ำลง"
เท่านั้น ไม่ได้สิทธิ์ "ข้ามด่าน identity". Low ที่ทำให้ G4 ของลอย = FAIL ไม่ใช่ tier

**วิธีตรวจที่ถูก:** เรนเดอร์ทั้ง 4 tier จาก **กล้อง/ฉาก/เวลาเดียวกัน** (`VOXELFORGE_LOOK_QUALITY`
ทำให้ทำได้แล้ว) แล้ววางเรียงกันใบเดียวเหมือน `wide-final-compare.png`. ถ้าไล่จาก Ultra→Low แล้ว
"เห็นว่าเบาลง" ได้ แต่ไม่มีจุดไหนที่ "เปลี่ยนเกม" = ผ่าน

---

## 7. โน้ตถึง Rose (implement)

1. **หนึ่งแหล่งความจริง.** `LookQuality` เป็น resource ตัวเดียว ✅ ทำแล้ว. ค่าของ Ultra = const ใน
   `mod grade`; tier อื่น return เฉพาะ delta — **อย่าก็อปตาราง grade ไปไว้อีกที่**
2. **แก้ 3 ค่าใน `mod grade`** ตาม §4: `TEMPERATURE 0.10→0.02`, `POST_SATURATION 1.02→1.00`,
   `HIGHLIGHT_CONTRAST 1.30→1.0` และแก้คอมเมนต์ที่อ้างว่า "carried over from the signed-off hero
   shot" ให้ชี้ `render_wide_hero.sh` ไม่ใช่ `hero.rs` baked default
3. **`apply_look_to_cameras` ควร filter `With<crate::OrbitCam>` ไม่ใช่ `With<Camera3d>` เฉย ๆ.**
   ไม่ใช่แค่ป้องกัน: `focus_dof` ต้องการ `OrbitCam` อยู่แล้ว — กล้องที่ไม่มีจะได้ DoF ที่
   `focal_distance` **ค้างที่ `BOOM_DIST` ตลอดกาล**. และมันกันชนกับกล้อง VFX stage
   (`vfx.rs:1478`) ที่จงใจ **ไม่เอา TAA** เพราะ TAA ลากอนุภาคเป็นผี — ถ้าวันไหนมีคนรัน `--play`
   พร้อม `VOXELFORGE_VFX` เลนนั้นพังโดยที่โค้ดเขาไม่ได้เปลี่ยน
4. **`apply_look_to_sun` จับ `DirectionalLight` ทุกดวง.** ตอนนี้เกมมีดวงเดียว (`main.rs:686`) เลยยัง
   ไม่พัง แต่ถ้าพอร์ต bounce card ของ `hero.rs` เข้ามา (2 ดวง shadowless) มันจะได้ `VolumetricLight`
   ไปด้วยแล้วหมอกสว่างผิด. เติม marker หรือ filter ตอนนี้ถูกกว่าตามแก้ทีหลัง
5. **Ultra ต้อง `insert_resource(DirectionalLightShadowMap { size: 4096 })` เอง** — play path ไม่มี
   บรรทัดนี้ (ของที่ `main.rs:373` อยู่ใน `if cfg.hero`). insert ใน `look.rs` ได้เลย ไม่ต้องขอ ack
   ⚠️ เป็น resource ระดับ app ไม่ใช่ component จึง**สลับตอนรันไม่ได้แบบ F7** — ตั้งครั้งเดียวตอน
   `build()` จากค่า tier เริ่มต้น แล้วบอกผู้เล่นว่าสลับ Ultra ต้องรีสตาร์ต (หรือปล่อย 4096 ค้างไว้
   ทุก tier ก็ได้ — VRAM ~64MB ไม่ใช่ FPS)
6. **`DistanceFog` density 0.008 จูนมาจากห้อง 16×16.** ในโลกเปิด `exp(-0.008 × 200m) ≈ 0.20` = ที่
   200m หมอกกลืน 80%. ตอนวัดเฟรมแรกให้ดูแกนนี้ด้วย — เดาว่าจะอยากได้ราว 0.003–0.004

---

_เอกสารมีชีวิต — ตัวเลข GPU cost ทั้งหมดยังเป็นการประเมิน และ caveat PCSS (§1) ยังไม่ได้พิสูจน์ใน
โลกเปิด. พอ Yamamoto ปิด Gate 3 แล้วเรนเดอร์ 4 tier จากกล้องเดียวกันได้ ให้เอาเลขจริงมาทับทันที.
— Flamingo (Designer), 2026-07-31 (rev 2)_
