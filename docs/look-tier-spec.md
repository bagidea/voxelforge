# Look Tier Spec — อะไรตัดแล้วตาย อะไรตัดแล้วรอด

> **เจ้าของเอกสาร:** Flamingo (look lane) · **ผู้ใช้งานหลัก:** Rose (implement tier ใน `client/src/look.rs`)
> **สร้าง:** 2026-07-31 · **สถานะ:** ทิศทางที่ตัดสินแล้ว — ตัวเลข GPU cost เป็นการประเมิน รอวัดจริงบนเครื่องเป้าหมาย
>
> อ่านคู่กับ: [`look-bible.md`](look-bible.md) §2–§3 · [`look-acceptance-rubric.md`](look-acceptance-rubric.md) §GATE
> · [`golden-beauty-shot.md`](golden-beauty-shot.md) §go/no-go

---

## 0. ทำไมเอกสารนี้ต้องเขียนใหม่ (ไม่ใช่แค่ก็อป look-bible §3)

`look-bible.md` §3 นิยาม tier ไว้เป็น **Ultra = native / Lite = web-wasm** — คือแบ่งตาม **แพลตฟอร์ม**.
วันที่ 2026-07-31 CEO ยกเลิก web target ถาวร (`65b1d12`, `GAME-VISION.md` — "No wasm / web / browser
export"). นิยามเดิมจึงตายไปครึ่งหนึ่ง: ไม่มี Lite platform อีกแล้ว.

**นิยามใหม่: tier = งบ GPU บนเครื่องเดสก์ท็อป ไม่ใช่แพลตฟอร์ม.** เป้า Ultra ยังเป็นตัวเดิมจาก
`GAME-VISION.md` — 60fps @1080p บน GTX 1660+ — และ tier ที่ต่ำลงมามีไว้รับสองกรณี:
การ์ดต่ำกว่า 1660 / iGPU, และคนที่อยากได้ 1440p–4K หรือ 120fps บนการ์ดกลาง.

สิ่งที่ **ไม่เปลี่ยน** จาก look-bible: กฎ identity. "เอาภาพนิ่ง tier ต่ำไปวางข้าง Ultra —
ถ้าคนดูบอกว่า *เกมเดียวกัน แค่เครื่องเบากว่า* = ผ่าน. ถ้าดูเหมือน *Minecraft vanilla ไม่มี shader*
= ตัดลึกไป." เกณฑ์นี้ยังเป็นคำตัดสินสุดท้ายของทุก tier.

---

## 1. คำตอบสั้นที่สุด — ตัดแล้วตาย vs ตัดแล้วรอด

### 🔴 ตัดแล้ว "ตาย" — ห้ามแตะทุก tier รวมถึง Low

| ชั้น | component ใน `look.rs` | ตัดแล้วเกิดอะไร | GPU cost |
|---|---|---|---|
| **Filmic tone-map** | `Tonemapping::AcesFitted` | หน้าต่างคลิปขาวตัน → **G5 ตกทันที** และ golden hour กลายเป็นส้มไหม้ | ~0 |
| **Color grade** | `ColorGrading` (temp/sat/midtone contrast/highlight shoulder) | นี่**คือ**ตัวล็อกโทน golden-hour (look-bible ฟีเจอร์ #14) ถอดออกแล้วเฟรมออกเทา-เย็นทันที → G6 ตก | ~0 |
| **Contact AO** | `ScreenSpaceAmbientOcclusion` (คุณภาพลดได้ แต่ชั้นต้องอยู่) | บล็อกลอย ไม่ติดพื้น → **G4 ตกครึ่งหนึ่ง** + เป็นหน้าตา "Minecraft ไม่มี shader" ตรงตัว | ปรับได้ |
| **Soft shadow ≥3px** | `ShadowFilteringMethod` (Temporal หรือ Gaussian) | ขอบเงาคม 1px → **G4 ตกอีกครึ่ง** | ต่ำ |
| **Window bloom** | `Bloom` | look-bible §107 ระบุ "bloom หน้าต่าง+โคม" อยู่ในรายการห้ามตัด | ต่ำมาก |
| **Warm bounce ในร่ม** | *(ไม่ได้อยู่ใน look.rs — เป็นไฟของฝั่งเกม)* | ร่มดำตัน → **G3 ตก** ซึ่ง rubric ระบุว่าเป็นด่านที่พลาดบ่อยที่สุด | ~0 |
| **`Msaa::Off`** | `Msaa::Off` | ไม่ใช่เรื่องสวยงาม — SSAO ของ Bevy **บังคับ** ให้ MSAA ปิด. MSAA จึงไม่ใช่ knob ของ tier เลย | — |

> 4 ใน 7 บรรทัดนี้ราคาเกือบ 0 (tone-map, grade, bloom, ambient). **ข้อสรุปเชิงออกแบบ: สิ่งที่ทำให้
> ภาพ "เป็น Voxelforge" แทบไม่กินเฟรมเรตเลย** — ที่กินคือชั้นเสน่ห์. เพราะงั้นแม้ tier Low ที่สุด
> ก็ยังหน้าตาเดียวกับ Ultra ได้ ถ้าตัดถูกที่.

### 🟢 ตัดแล้ว "รอด" — เรียงตามลำดับที่ควรตัดก่อน-หลัง

| # | ตัดอะไร | ทำไมตัดได้ | เสียอะไร |
|---|---|---|---|
| 1 | **PCSS** (`soft_shadow_size`) | **มีหลักฐานวัดในรีโปเราเอง** — `hero.rs:558-570`: penumbra ~6px ที่เห็นจริงมาจาก `ShadowFilteringMethod::Temporal` + TAA + shadow map 4K **ไม่ใช่ PCSS**; ห้องเล็กทำให้ Bevy clamp `blur_size` ไปที่ floor 0.5 และวัดแล้วนิ่งตั้งแต่ soft=0.02 ถึง 400 | **แทบไม่เสียอะไรเลย** — นี่คือของฟรีที่ควรตัดเป็นอันดับแรก |
| 2 | **Depth of Field** | rubric §213 อนุญาตให้ pass 5 = N/A แล้วปรับฐาน 100→92 ตรง ๆ | ความ cinematic; ยังผ่าน gate ครบ 6 |
| 3 | **SSAO คุณภาพ** Ultra→High→Medium→Low | ชั้นยังอยู่ = G4 ยังผ่าน แค่ crease นุ่มลง | มิติ contact ลดลงเล็กน้อย |
| 4 | **VolumetricFog `step_count`** 96→64→32→16 | ลด*คุณภาพ*ลำแสง ไม่ใช่ลบชั้น | ลำแสงมี banding ขึ้นเมื่อ step ต่ำ |
| 5 | **TAA** | ตัดได้ **แต่มีเงื่อนไขผูก — ดู §3** | อ่านที่ §3 ก่อนตัด |
| 6 | **Shadow map** 4096→2048→1024 | ยังผ่าน G4 ที่ 2048 สบาย ๆ | เงาไกลหยาบขึ้น |
| 7 | **VolumetricLight บนดวงอาทิตย์** | ถ้าปิด VolumetricFog แล้ว ตัวนี้เป็นค่าใช้จ่ายตายซาก ต้องปิดตาม | — |

> ⚠️ **VolumetricFog ห้ามปิดสนิท** ตราบใดที่ยังไม่มี screen-space light shaft มาแทน — rubric §216
> ระบุชัด: "volumetric god ray→screen-space **ยังต้องเห็นแท่ง**" คือเปลี่ยนเทคนิคได้ แต่ผลชั้นนั้น
> ต้องเห็น. ปิดสนิท = เสีย pass 3 ทั้งดุ้น. ให้ลด `step_count` แทน.

---

## 2. ตาราง tier — map ตรงกับ component ใน `look.rs`

| knob | **Ultra** (reference) | **High** | **Medium** | **Low** |
|---|---|---|---|---|
| เครื่องเป้าหมาย | GTX 1660+ @1080p60 | 1660 @1440p / 1650 @1080p | GTX 1050 / iGPU แรง | iGPU / การ์ดเก่า |
| `Tonemapping` | AcesFitted | ← เหมือนกัน | ← | ← |
| `ColorGrading` (ทุกค่า) | signed-off (§4) | ← **เหมือนกันเป๊ะ** | ← | ← |
| `Bloom` | NATURAL @ 0.26 | ← | ← | ← |
| `DistanceFog` | on | ← | ← | ← |
| `Msaa` | Off | Off | Off | Off |
| `ShadowFilteringMethod` | Temporal | Temporal | Temporal | **Gaussian** |
| `soft_shadow_size` (PCSS) | 3.0 | 3.0 | **ปิด** | ปิด |
| `DirectionalLightShadowMap` | 4096 | 2048 | 2048 | 1024 |
| `TemporalAntiAliasing` | on | on | on | **ปิด** |
| `ScreenSpaceAmbientOcclusion` | **Ultra**, thickness 1.45 | High, 1.45 | Medium, 1.45 | **Low**, 1.45 |
| `DepthOfField` | Bokeh (§4) | Bokeh | **ปิด** | ปิด |
| `VolumetricFog.step_count` | 96 | 64 | 32 | **16** |
| `VolumetricFog.jitter` | 0.6 | 0.6 | 0.6 | **0.0** |
| `VolumetricLight` (sun) | on | on | on | on |

**สังเกต 3 อย่างในตารางนี้:**

1. **แถวสีของภาพ (tone-map / grade / bloom / distance fog) เหมือนกันหมดทั้ง 4 tier.** ตั้งใจ —
   นี่คือเหตุผลเดียวที่ screenshot ของ Low ยังอ่านออกว่าเป็น Voxelforge. และมันฟรี.
2. **`constant_object_thickness: 1.45` ไม่ลดตาม tier.** มันไม่ใช่ค่าที่แพง — เป็นค่า "ความหนาระดับ
   voxel" ที่ทำให้ของนั่งบนพื้น. ลด *sample count* (quality_level) ได้ แต่ลดความหนาแล้วของจะลอย.
3. **`jitter: 0.0` ที่ Low** เพราะ jitter มีไว้ให้ TAA เกลี่ย — ไม่มี TAA แล้ว jitter = noise ล้วน.

---

## 3. กฎการผูกกัน (coupling) — ห้ามตัดข้ามกฎนี้

ตัดผิดลำดับแล้วจะได้ภาพ *แย่กว่า* ตอนยังไม่ตัด ทั้งที่เฟรมเรตไม่ได้ดีขึ้นเท่าไหร่:

1. **ปิด TAA ⇒ ต้องปิด PCSS ด้วย และ SSAO ต้องไม่ใช่ Ultra.**
   PCSS กับ SSAO Ultra เป็น stochastic ทั้งคู่ — เฟรมเดียวคือ noise. TAA คือตัวสะสมให้เนียน
   (`hero.rs:794-797` เขียนเหตุผลนี้ไว้ตรง ๆ). ปิด TAA อย่างเดียว = ส่ง noise ให้ผู้เล่นดู.
2. **SSAO เปิด ⇒ `Msaa::Off` เสมอ.** ข้อบังคับของ engine ไม่ใช่ทางเลือก.
3. **ปิด VolumetricFog ⇒ ปิด `VolumetricLight` บนดวงอาทิตย์ด้วย** ไม่งั้นจ่ายค่า in-scattering ฟรี ๆ
   โดยไม่มีอะไรมารับ.
4. **DoF เปิด ⇒ `focal_distance` ต้องตามกล้องจริง** (ดู §4 ข้อ DoF) ไม่ใช่ค่าคงที่.

---

## 4. Ultra = ค่าที่เซ็นรับแล้ว (single source of truth)

Tier Ultra **ไม่ใช่ tier ที่ตั้งค่าใหม่** — มันคือชุดค่าที่ผ่าน gate มาแล้วบนเฟรม
`docs/assets/wide-hero-final.png` (CEO-approved tilt-down). Tier อื่นทั้งหมดนิยามเป็น
**"Ultra ลบอะไรออก"** เท่านั้น ห้ามมีชุดตัวเลขคู่ขนานอีกชุด.

| knob | ค่า | ที่มา |
|---|---|---|
| `temperature` | 0.10 | baked default `hero.rs:698` |
| `post_saturation` | 1.02 | baked default `hero.rs:698` |
| midtone `contrast` | 1.30 | baked default + `golden-beauty-shot.md` GRADE |
| shadows `contrast` | **1.0** | ถือ neutral โดยตั้งใจ — ใส่ contrast ที่ shadows แล้ว p05-L ร่วง 13.7%→3.3% (วัดแล้ว) |
| highlights `contrast` | **1.0** ← *แก้* | `hero.rs:711` — path `wide` ใช้ `(1.0, shoulder)` |
| highlights `gain` | 0.64 | `VOXELFORGE_SHOULDER=0.64` ใน recipe ที่ล็อก |
| `Bloom.intensity` | 0.26 บน `NATURAL` | `hero.rs:800-811` |
| SSAO | Ultra, thickness 1.45 | `hero.rs:823-834` |
| `soft_shadow_size` | 3.0 | `hero.rs:574` |
| `sensor_height` | 0.35 | จำเป็น — sensor default ~18.6mm ทำให้ CoC เกือบศูนย์ที่สเกลนี้ |
| DoF `aperture_f_stops` | **f/8** ← *แก้* | ดูเหตุผลข้างล่าง |
| DoF `focal_distance` | ตาม orbit boom (dynamic) | ✅ เห็นด้วยกับ Rose |

### สองจุดที่ต้องแก้จากที่ implement ไว้

**(ก) `highlights.contrast` ต้องเป็น 1.0 ไม่ใช่ 1.30.**
`hero.rs:711` เขียนไว้ว่า `let (hi_contrast, hi_gain) = if wide { (1.0, shoulder) } else { (g_contrast, 1.0) };`
— คือ contrast กับ gain บน highlight เป็น **either/or ตามดีไซน์** ไม่เคยเปิดพร้อมกัน. เฟรมที่ CEO
อนุมัติเดินทาง `wide` = highlight contrast **1.0** + gain 0.64. การใส่ 1.30 คู่กับ 0.64 คือ
**บีบสองชั้น**: contrast ดันปลายสว่างออกจาก mid ก่อน แล้ว gain ดึงกลับลง 36% — ผลคือหน้าต่างทึม
ไล่เกรนหาย ซึ่งคือแกน G5 ที่ look-bible ระบุว่า "ตกข้อนี้ = FAIL ทันที". แก้บรรทัดเดียว.

**(ข) `aperture_f_stops` ควรเป็น f/8 ไม่ใช่ f/4.0.**
เห็นด้วยกับ *เหตุผล* ของ Rose (กล้องที่คนเล่นอยู่หลัง ไม่ใช่ภาพนิ่ง) แต่เลขยังตื้นไปหนึ่งช่วง:

- `note-to-poppy-dof-fix.md` วัดไว้ว่า f/2.8→f/4.5 ทำให้ subject คมขึ้น +60% โดย **bg bokeh ไม่ขยับ
  (wall LapVar นิ่งที่ ~6.0) และ "narrower ไม่ล้าง bg จนถึง f/8"** → f/8 คือจุดที่ subject คมสุด
  *และ* พื้นหลังเริ่มกลับมาอ่านออก
- เฟรม establishing ที่ล็อกไว้ใช้ **f/10** โดยให้เหตุผลตรง ๆ ว่า "deep so the establishing floor
  stays crisp voxel geometry (**G1**)" — และเกมคือมุม establishing ไม่ใช่ hero tabletop
- อันตรายจริงของ f/4: `focal_distance` ผูกกับ boom ซึ่ง `camera_boom` หดเหลือ ~2m เวลาชิดกำแพง →
  ที่ f/4 + sensor 0.35 ทุกอย่างเลย ~5m ละลายหมด = **voxel grid อ่านไม่ออก = G1 ตก** ในจังหวะที่
  ผู้เล่นเจอบ่อยที่สุด

f/8 = ยังแยกระยะได้ (ตอบโจทย์ที่บรีฟขอ) แต่ภูมิประเทศ voxel ยังคม. ถ้าอยากได้ bokeh มากกว่านี้
ค่อยเปิด f/4 เป็น "cinematic mode" ตอน cutscene/photo mode แยกต่างหาก.

---

## 5. เกณฑ์ตรวจรับต่อ tier

| tier | GATE G1–G6 | AAA Score (ชั้น B) | ฐาน |
|---|---|---|---|
| **Ultra** | ผ่านครบ 6/6 | ≥ 83 (AA ขึ้นไป) | 100 |
| **High** | ผ่านครบ 6/6 | ≥ 78 | 100 |
| **Medium** | ผ่านครบ 6/6 | ≥ 72 | **92** (DoF = N/A ตาม rubric §213) |
| **Low** | ผ่านครบ 6/6 | ≥ 60 (A) | 92 |

**กฎเหล็กจาก rubric §216 ที่ห้ามผ่อน: แกน gate ทั้ง 6 ห้าม N/A ทุก tier.** tier ต่ำได้สิทธิ์
"คะแนนเสน่ห์ต่ำลง" เท่านั้น — ไม่ได้สิทธิ์ "ข้ามด่าน identity". Low ที่ทำให้ G3 ร่มดำ หรือ G4 ของลอย
= FAIL ไม่ใช่ tier

**วิธีตรวจที่ถูก:** เรนเดอร์ทั้ง 4 tier จาก **กล้อง/ฉาก/เวลาเดียวกัน** แล้ววางเรียงกันใบเดียว
(เหมือนที่ทำ `wide-final-compare.png`). ถ้าไล่จาก Ultra→Low แล้ว "เห็นว่าเบาลง" ได้ แต่ไม่มีจุดไหน
ที่ "เปลี่ยนเกม" = ผ่าน

---

## 6. โน้ตถึง Rose (implement)

1. **หนึ่งแหล่งความจริง.** `LookTier` เป็น `Resource` ตัวเดียว ตั้งครั้งเดียวจาก config/CLI แล้ว
   `look.rs` อ่านฝ่ายเดียว. ค่าของ Ultra = const ที่มีอยู่แล้วในไฟล์; tier อื่น **return เฉพาะ
   delta** (quality level, step_count, มี/ไม่มี DoF, มี/ไม่มี TAA) — อย่าก็อปตาราง grade ไปไว้อีกที่
   เด็ดขาด ไม่งั้นวันหนึ่งจะมีค่าความจริงสองชุดแล้วหาไม่เจอว่าอันไหนจริง
2. **`DirectionalLightShadowMap` อยู่ที่ `main.rs:373`** ซึ่งเป็นไฟล์เลนอื่น. ถ้า tier จะคุม shadow
   map ต้องขอ ack ตาม `LANES.md` ก่อน — หรือ (ทางที่ผมแนะนำ) **เฟสแรกอย่าเพิ่งแตะ** ปล่อย 4096 ไว้
   ทุก tier แล้วค่อยเก็บทีหลังเป็น PR แยก
3. **`apply_look_to_cameras` ควร filter `With<crate::OrbitCam>` ไม่ใช่ `With<Camera3d>` เฉย ๆ.**
   เหตุผลไม่ใช่แค่ป้องกัน: `focus_dof` ต้องการ `OrbitCam` อยู่แล้ว — กล้องที่ไม่มี OrbitCam จะได้
   DoF ที่ `focal_distance` **ค้างที่ `BOOM_DIST` ตลอดกาล** ซึ่งไม่มีใครต้องการ. และมันกันชนกับ
   กล้อง VFX stage (`vfx.rs:1478`) ที่จงใจ **ไม่เอา TAA** เพราะ TAA ลากอนุภาคเป็นผี — ถ้าวันไหนมีคน
   รัน `--play` พร้อม `VOXELFORGE_VFX` เข้า เลนนั้นพังโดยที่โค้ดเขาไม่ได้เปลี่ยน
4. **`apply_look_to_sun` จับ `DirectionalLight` ทุกดวง.** ตอนนี้เกมมีดวงเดียว (`main.rs:686`) เลยยัง
   ไม่พัง แต่ถ้าวันหนึ่งพอร์ต bounce card ของ `hero.rs` เข้ามา (มี 2 ดวง shadowless) มันจะได้
   `VolumetricLight` ไปด้วยแล้วหมอกจะสว่างผิด. เติม marker หรือ filter ไว้ตอนนี้ถูกกว่าตามแก้ทีหลัง
5. **`DistanceFog` density 0.008 จูนมาจากห้อง 16×16.** ในโลกเปิด `exp(-0.008 × 200m) ≈ 0.20` = ที่
   200m หมอกกลืนไป 80%. ยังไม่ต้องแก้ตอนนี้ แต่ตอนวัดเฟรมแรกให้ดูแกนนี้ด้วย — เดาว่าจะอยากได้ราว
   0.003–0.004 สำหรับ view distance ของเกมจริง

---

_เอกสารมีชีวิต — ตัวเลข GPU cost ทั้งหมดยังเป็นการประเมิน. พอ Yamamoto ปิด Gate 3 แล้วเรนเดอร์
4 tier จากกล้องเดียวกันได้ ให้เอาเลขจริงมาทับตารางนี้ทันที. — Flamingo (Designer), 2026-07-31_
