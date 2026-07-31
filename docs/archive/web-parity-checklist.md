# Voxelforge — Web Look-Parity Checklist

> **คำถามเดียวที่เอกสารนี้ตอบ:** เฟรมจาก web build (wasm) ให้ "ลุคเดียวกัน" กับ
> golden beauty shot หรือยัง — และถ้ายัง ตกที่ชั้นไหน.
>
> เป้าหมายอ้างอิง: [`golden-beauty-shot.md`](golden-beauty-shot.md) · ตัวเลขทั้งหมดมาจาก
> `scripts/grade_axes.py` **ตัวเดียว** ห้ามสร้างเกณฑ์ชุดที่สอง.

---

## 🔒 BASELINE ที่ใช้เทียบ — TILT-DOWN (อัปเดต 2026-07-29)

**เฟรมที่ทุกอย่างในเอกสารนี้เทียบด้วย = `docs/assets/wide-hero-final.png`**
— establishing hero มุม **TILT-DOWN** ที่ CEO อนุมัติ (framing ไฟเขียว 2026-07-29,
กล้องล็อกไว้ตั้งแต่ 2026-07-27) · recipe เดียวที่เขียนไว้จริงคือ `scripts/render_wide_hero.sh`

| | ไฟล์ | บทบาท | ห้ามสับสน |
|---|---|---|---|
| 🎯 **parity baseline** | `docs/assets/wide-hero-final.png` (1280×720, tilt-down) | **native control + baseline ของรอบ v3** — เทียบ delta กับตัวนี้ | นี่คือตัวที่ web ต้องเลียน |
| 📐 calibration ref | `docs/assets/golden-beauty-shot-ref.png` (1024×1024, กล้องเกือบระดับ, tight) | ที่มาของ **ตัวเลข target** ใน `grade_axes.TARGETS` เท่านั้น | **ห้ามเอามาเทียบเฟรม v3 ตรงๆ** |

> **ทำไมต้องแยก:** `grade_axes.py` วัดบน resample 1024×1024 ด้วยกล่อง fg/bg ตำแหน่งตายตัว →
> **เปลี่ยนมุมกล้อง ตัวเลขขยับหมด**. ref เดิมเป็นภาพ 1:1 กล้องเกือบระดับ ระยะ tight (ชามเต็มหน้า)
> ส่วน v3 เป็น 16:9 tilt-down wide — เอา absolute target ของ ref มาตัดสิน v3 = วัดกล้อง ไม่ใช่วัดเว็บ.
> **ด่าน W0-D ในสคริปต์บังคับข้อนี้แล้ว** (native control ต้อง fingerprint ตรงกับ baseline).

**ค่าที่ baseline วัดได้จริง** (วัดสด 2026-07-29 · `grade_axes.py` + `measure_penumbra.py`):

| warmth R−B | blue B | sat | micro | p95 | DOF fg:bg | penumbra |
|---|---|---|---|---|---|---|
| 128.88 | 5.19 | 94.48 | 5.92 | 177.36 | 0.17 (wide tradeoff) | **4.83 px** |

> **พิสูจน์แล้วว่า re-shoot ได้:** build ใหม่จาก source ปัจจุบัน (หลัง refactor `env::var → Cfg`)
> แล้วเรนเดอร์ recipe เดิมซ้ำ → ไม่ byte-identical (dust motes สุ่ม) แต่ทุกแกนห่างจากเฟรมที่ล็อกไว้
> **≤ 0.15** (warmth +0.15 · blue −0.01 · sat +0.01 · p95 +0.00 · micro −0.00 · penumbra 4.92px)
> — หลักฐาน: `docs/assets/archive/tiltdown-baseline-reverify.png`. `BASELINE_TOL` ตั้งจากตัวเลขนี้

---

## 0. Gate ก่อนเริ่มเกรด (ถ้าไม่ผ่าน = ยังเกรดลุคไม่ได้)

เกรดลุคได้ต่อเมื่อเฟรมนั้น **เทียบได้จริง**. สี่ข้อนี้ต้องผ่านก่อน มิฉะนั้นตัวเลขที่วัดได้
ไม่ได้แปลว่า "ลุคเพี้ยน" แต่แปลว่า "วัดคนละของ":

| # | Gate | ผ่านเมื่อ | หลักฐาน (authoritative) |
|---|---|---|---|
| **W0-A** | **Backend ถูกตัว** | wgpu เปิด **WebGPU** ไม่ใช่ WebGL2 fallback | บรรทัด `AdapterInfo { … backend: BrowserWebGpu … }` ใน console log. ⚠️ **ห้ามใช้ HUD `WebGPU adapter OK` ตัดสิน** — HUD นั้นมาจาก `navigator.gpu.requestAdapter()` ซึ่งบอกแค่ว่า *เบราว์เซอร์* มี WebGPU ไม่ได้บอกว่า *wgpu เปิดอันไหน* (`scripts/web-verify.mjs` เขียนกับดักข้อนี้ไว้แล้ว และ gate จริงคือ `bevyBackend`) |
| **W0-B** | **ฉากถูกฉาก** | เป็นฉาก hero (ครัวไม้ + หน้าต่าง + ชาม + teal accent) ไม่ใช่ terrain/editor sandbox | `?hero` อยู่ใน URL ที่ log ไว้ (`client/src/main.rs:153`) + ตาคนดูเฟรม |
| **W0-C** | **รีซิพีถูกชุด** | query string ตรงกับ recipe tilt-down ที่ล็อกไว้ **ครบทุกตัว** | บรรทัด `[harness] VOXELFORGE_URL <url>` ในหัว console log → เทียบด้วย `python scripts/hero_recipe.py --check-url "<url>"` |
| **W0-D** | **native control ถูกเฟรม** | native ที่ส่งมาคู่กันเป็น **tilt-down baseline** จริง | fingerprint 5 แกนห่างจาก `wide-hero-final.png` ไม่เกิน tol (สคริปต์เช็คให้) |

> ### ✅ หนี้ W0-C ปลดแล้ว (2026-07-29) — web รับ recipe ได้จริงแล้ว
> เอกสารรอบก่อนบันทึกว่า wasm `read_cfg()` hard-code look knobs = `None` และ `hero.rs`
> อ่าน env ตรงๆ (บน wasm คืน `Err` เสมอ) → เว็บเรนเดอร์ recipe ที่อนุมัติไม่ได้เลย. **ตอนนี้แก้แล้วทั้งสองจุด:**
> - `client/src/main.rs` §`#[cfg(target_arch = "wasm32")] fn read_cfg` → parse query string
>   ครบทุก knob (`cam/sun/dof/exposure/grade/ambient/ambcolor/bluescale/bounce/bounce2/shoulder/dust/…`)
>   ผ่าน `qs_flag` / `qs_num` / `qs_floats` (all-or-nothing เหมือน `env_floats`)
> - `client/src/hero.rs` ไม่เหลือ `std::env::var` แล้วสักตัว — ทุก knob มาจาก `Cfg` ทางเดียว
>   (`wide` / `dust` / `bluescale` / `bounce` / `bounce2` / `shoulder` / `ambcolor` / `fg_apron`)
>
> **ผลต่อการเกรด:** ไม่ต้องเกรดกับ "baked default" อีกแล้ว — **บังคับได้ว่าทั้งสองฝั่งต้องวิ่ง
> recipe tilt-down ตัวเดียวกัน** และ W0-C กลายเป็นด่านที่เครื่องตรวจได้ ไม่ใช่ความเชื่อใจ

---

## 1. เกณฑ์ machine-checked (`scripts/grade_axes.py`)

รันตัวเดียวกับที่ใช้เกรด native — **ห้าม fork เกณฑ์**:

```bash
python scripts/grade_axes.py <web-frame.png> docs/assets/wide-hero-final.png
```

| axis | target (จาก ref 1:1) | REF | **tilt-down baseline** | หมายเหตุ parity |
|---|---|---|---|---|
| warmth R-B (mid) | ≥ 110 | 120.9 | **128.88** | ต้องตรง — grade/tonemap ปลอดภัยบน WebGPU |
| blue B (mid) | ≤ 10 | 4.3 | **5.19** | ต้องตรง |
| saturation (mid) | ≥ 90 | 96.1 | **94.48** | ต้องตรง |
| micro-contrast | ≥ 5 | 5.24 | **5.92** | ต้องตรง (dust + grade เป็นตัวหลัก) |
| highlight p95 | 150..185 | 165.8 | **177.36** | ต้องตรง (LUT shoulder) |
| DOF fg:bg | ≥ 3.0 | 3.50 | **0.17** | **ยกเว้นบน wide** — deep-focus ทำให้ fg≈bg โดยดีไซน์ เป็น tradeoff ที่รับไว้แล้วใน `golden-beauty-shot.md`; ถ้า DOF ถูก disable บน web ให้ลงเป็น deviation ไม่ใช่ fail |

> ⚠️ **อ่านคอลัมน์ให้ถูก:** คอลัมน์ `target`/`REF` มาจาก **ref 1:1 กล้องเกือบระดับ** — ใช้ตัดสิน
> *เฟรมที่ framing เดียวกับ ref* เท่านั้น. เฟรม v3 (16:9 tilt-down) ให้ดูคอลัมน์ **tilt-down baseline**
> แล้วเทียบเป็น **delta** ตาม §2.6 — ไม่ใช่เอา target มาทาบตรงๆ (เห็นชัดที่ DOF: baseline เอง 0.17 vs target 3.0)
>
> ⚠️ **กับดักการอ่านตัวเลข:** axis เหล่านี้ให้ค่าที่ "ผ่าน" ได้แบบไร้ความหมายถ้าเฟรมผิดฉาก —
> เช่น ฉาก editor ที่มีท้องฟ้าฟ้าจะได้ p95 ในแบนด์โดยบังเอิญ และ HUD text คมๆ จะดัน
> micro-contrast ขึ้นสูง. **PASS จะนับได้ก็ต่อเมื่อ W0-A/B/C/D ผ่านครบแล้วเท่านั้น.**

---

## 2. เกณฑ์ที่ต้องใช้ตาคน (ตัดสินบนเฟรม hero เท่านั้น)

เทียบกับ pass-list 9 ชั้นใน `golden-beauty-shot.md`:

| # | ชั้น | สถานะบน web ที่คาดไว้ | หมายเหตุ |
|---|---|---|---|
| 1 | Key light | ต้องตรง | |
| 2 | Bounce / GI (ห้ามเงาดำ) | ต้องตรง | **แกนห้ามพลาด** |
| 3 | God rays / volumetric | ⚠️ ตรวจ | `AtmospherePlugin` ต้องใช้ compute — ถ้า backend ตกไป GL จะไม่โหลด |
| 4 | Soft shadow + contact AO | ⚠️ **deviation ที่ประกาศไว้** | PCSS **ถูกตัดโดยตั้งใจ** รอบนี้ (Tint/Chrome reject shader). แต่ **contact AO ต้องยังอยู่** — ถ้า SSAO ก็หายด้วย ให้ลงเป็น *fail* ของแกน #4 ไม่ใช่ deviation |
| 5 | DOF | ⚠️ ตรวจ | ต้องการ depth texture |
| 6 | Tone-map filmic (ห้ามหน้าต่างขาวคลิป) | ต้องตรง | **แกนห้ามพลาด** |
| 7 | PBR 3 วัสดุ | ต้องตรง | |
| 8 | Bloom | ต้องตรง | |
| 9 | Palette lock | ต้องตรง | ครอบด้วย axis warmth/blue/sat |

**Deviation ที่ประกาศล่วงหน้า (ไม่นับเป็น fail):** PCSS penumbra เท่านั้น.
อย่างอื่นที่หายไปเพราะ backend ตกลง WebGL2 = **fail ของ gate W0-A** ไม่ใช่ deviation ของลุค.

---

## 2.5 PCSS-off — เท่าไหร่คือ "รับได้สำหรับ web" เท่าไหร่คือ "ตก"

> §2 ประกาศว่า PCSS เป็น deviation แต่ไม่มีเลข — ส่วนนี้เติมเลขให้ ตัดสินได้โดยไม่ต้องเถียง

### ตัดที่ไหน ทำไม
`index.html` ส่ง `data-cargo-no-default-features data-cargo-features="webgpu"` → drop
`experimental_pbr_pcss` **เฉพาะ web build** เพราะ shader ไม่คอมไพล์ใต้ Tint/Chrome แล้วเหลือ canvas ดำ.
`hero.rs:605-606` จึง `#[cfg(feature = …)]` ทิ้ง `soft_shadow_size` ไม่งั้น wasm32 พังด้วย E0560.
**native + hero-shot ยังได้ PCSS ครบ** — นี่คือความต่างที่ตั้งใจ ไม่ใช่ regression

### เกณฑ์ตัวเลข (วัดด้วย `scripts/measure_penumbra.py` — 20→80% luminance transition บนแถบพื้น)

| penumbra บนเฟรม web | คำตัดสิน | เหตุผล |
|---|---|---|
| **≥ 3.0 px** | ✅ **PASS เต็ม** ไม่ต้องใช้ waiver ด้วยซ้ำ | ผ่านบาร์ G4 ของ rubric เอง ("transition ≥ 3px") |
| **2.0 – 3.0 px** | 🟡 **รับได้สำหรับ web เท่านั้น** — ลงบันทึกเป็น known deviation | ต่ำกว่าบาร์ G4 แต่ยังอ่านว่า "นุ่ม" ไม่ใช่ขอบมีด. **ค่าเดียวกันบน native = FAIL** |
| **< 2.0 px** | ❌ **FAIL** ไม่มี waiver ไหนครอบ | อ่านเป็น hard 1px PCF → *"voxel ไม่มี soft shadow = แบนทันที"* ลุคตาย |

**native anchor (วัดจริง 2026-07-29):** `docs/assets/wide-hero-final.png` (**tilt-down baseline**) = **4.83 px**
— และเรนเดอร์ซ้ำจาก source ปัจจุบันได้ **4.92 px** → noise รอบต่อรอบ ~**0.1 px**, anchor ใช้ **4.83–4.92 px**

### 🔍 Discriminator — PCSS หาย vs TAA หาย (ข้อสำคัญที่สุดของส่วนนี้)

`hero.rs:591-599` บันทึกไว้ตรงๆ ว่า penumbra ที่เห็น **ไม่ได้มาจาก PCSS**:

> *"The visible penumbra actually comes from `ShadowFilteringMethod::Temporal` + TAA + the 4K shadow
> map — NOT from PCSS. In a room this small the blocker→receiver depth gap is tiny, so Bevy clamps
> `blur_size` to its 0.5 floor and the penumbra does NOT scale with occluder distance (measured flat
> from soft=0.02 to 400)."*

**ทำนายได้: ตัด PCSS ควรทำให้ penumbra หายไป ≤ ~1px เท่านั้น**

| สังเกต | แปลว่า | ทำอะไรต่อ |
|---|---|---|
| native − web ≤ 1.5 px | ตรงตามทำนาย = PCSS-off ล้วน | waive ตามตารางข้างบน |
| **native − web > 1.5 px** | **PCSS อธิบายไม่ได้** — น่าจะ `ShadowFilteringMethod::Temporal`/TAA ไม่รอดบน web | ❌ **ห้าม waive** ส่งกลับ lane wasm ตรวจ TAA (🟠 ใน `look-webgpu-risk.md`) |

> นี่คือกันไม่ให้ "PCSS ถูกตัดโดยตั้งใจ" กลายเป็นใบผ่านครอบจักรวาล.
> waiver ครอบแค่ **ความนุ่มของขอบเงาช่วง 2–3px** และ **pass-4c (penumbra กว้างตามระยะ)** — ซึ่ง 4c
> **native เองก็ทำไม่ได้อยู่แล้ว** (Bevy clamp blur floor) จึงเป็น **N/A ทั้งสองฝั่ง ไม่ใช่ข้อเสียของ web**

### waiver **ไม่** ครอบ
❌ contact AO / SSAO · ❌ Bokeh DoF · ❌ tone G3/G5/G6 · ❌ warmth/blue/sat/p95
· ❌ micro-contrast ที่ *ลดลง* · ❌ voxel hard edge (G1) · ❌ ทิศ key light (G2)

---

## 2.8 🔴 DEVIATION ตัวที่สอง — `DUST_INSTANCE_DROP` (**ฝั่ง NATIVE ผิด ไม่ใช่เว็บ**)

> เจอตอน sign-off รอบ v3 (Flamingo, 2026-07-29) จาก **ก้อนสว่างก้อนเดียวใน diff panel**
> ของ `web-parity-compare.png` — grader ให้ PASS ทุกแกนเพราะแกนทั้งหมดเป็นสถิติ *ทั้งภาพ*
> ส่วนนี่คือความต่าง **เฉพาะจุด** 0.6% ของเฟรม. **บทเรียน: axis delta ผ่าน ≠ เฟรมเหมือนกัน.**

**อาการ:** voxel หนึ่งลูกของ **teal accent tumbler** (ชั้นบน · world cube `(5,4,2)` ที่
`hero.rs` §4 WIDE dressing) **ไม่ถูกวาดบน native** — มองทะลุไปเห็น counter สีเหลืองข้างหลัง
ส่วน **web วาดครบ**. บนจอ = กล่อง **x 752–843, y 293–382** @1280×720 (91×89 px ≈ 0.88% ของเฟรม)

**ใครผิด = NATIVE.** `hero.rs:509-520` spawn ชั้นบน 3 ลูก (`notch` ข้ามแค่ `(gx+1, gz+1)` ลูกเดียว)
→ เฟรมที่ถูกต้องต้องมีลูกนี้ **เว็บตรงกับ source · native กับ golden baseline ขาด**

| หลักฐาน (วัดจริง 2026-07-29 · exe เดียว `target/release/voxelforge.exe` · recipe เดียว · เครื่อง idle) | ผล |
|---|---|
| native `DUST=0` | ✅ voxel **อยู่ครบ** |
| native `DUST=0.5 / 1.0 / 2.0 / 2.5 / **3.0 (recipe)** / 5.0` | ❌ **หายทุกค่า** (ไม่ใช่ threshold — แค่ dust ติดก็หาย) |
| เลื่อนกล้อง (eye x 7.6→7.9) | ❌ ยังหาย → **ไม่ใช่เรื่อง framing/culling ตามมุม** |
| เรนเดอร์ native control ซ้ำ | หายซ้ำ · W0-D vs baseline **±0.00 ทุกแกน** → ไม่ใช่เฟรมเสียครั้งเดียว |

**พิสูจน์ silhouette** (green-mask XOR บน crop x680–940, y250–500):

| คู่ | px ที่ต่างกัน |
|---|---|
| **WEB ↔ native `DUST=0`** | **29 px (0.04%)** ← เหมือนกันในระดับ AA noise |
| WEB ↔ native control (`DUST=3`) | **5,620 px (8.65%)** |
| WEB ↔ golden baseline | 5,620 px (8.65%) |
| native control ↔ golden baseline | 6 px (0.01%) ← ทั้งคู่ขาดลูกเดียวกัน |

→ **web = native ที่ปิด dust เป๊ะ**. dust คือตัวที่ทำ voxel หายบน native เท่านั้น

**กลไก (suspect ยังไม่ยืนยัน — งานของเลน render):** draw path คนละเส้น
`native log: "GPU preprocessing is fully supported on this device."` vs
`web log: "Some GPU preprocessing are limited on this device."`
`DUST>0` ใส่ **mesh ที่สอง + material ที่สอง** เข้าฉาก (`hero.rs:549-568`, `Cuboid 0.06` + mote mat)
→ indirect batch ที่ผ่าน GPU preprocessing เต็มรูปแบบบน Vulkan **ตกไป 1 instance**

**คำตัดสิน:** waiver PCSS **ไม่ครอบ** (ครอบแค่ความนุ่มขอบเงา 2–3px และตัดขาด G1 ไว้ชัดเจน)
แต่ **ไม่ใช่ความผิดของเว็บ** — เว็บคือฝั่งที่เรนเดอร์ถูก. จึง **ไม่ทำให้รอบ web parity ตก**
และเปิดเป็น **BLOCKER-N1 ของเลน native** แทน (ดู §8)

---

## 2.6 Native control frame — สิ่งที่ต้องมาคู่กับเฟรม web

เฟรม v3 เป็น framing เดียวกับ baseline แต่คนละ backend → **เกรดแบบ differential**: absolute target ของ
`grade_axes.py` calibrate มาจาก ref 1:1 กล้องเกือบระดับ (ดู §BASELINE) เอามาทาบ 16:9 tilt-down ตรงๆ ไม่ได้
ทั้งสองฝั่ง**ต้องวิ่ง recipe tilt-down ชุดเดียวกัน** — ตอนนี้ทำได้แล้วเพราะ wasm รับ query string ครบ:

| ต้องได้ **3 ไฟล์** (ขาดข้อไหน = เกรดไม่ได้) | ชื่อไฟล์รอบนี้ | วิธีได้มา | ทำไมต้องเป๊ะแบบนี้ |
|---|---|---|---|
| ① เฟรม web | `docs/assets/wasm-hero-v3.png` | เปิดด้วย **URL recipe ของ §2.6.1** แล้ว capture **เฉพาะ element `#bevy`** (`locator('#bevy').screenshot()`) | HUD ใน `index.html` เป็น `<div>` แยก — แคปทั้งหน้าแล้วตัวหนังสือจะดัน p95 / micro-contrast / warmth เพี้ยน (เฟรม v1 เป็นแบบนั้น) |
| ② **console log ของเฟรมนั้น** | `docs/assets/wasm-hero-v3.png.console.txt` | dump `console` ทั้งหมด — ต้องมี **ทั้ง** `AdapterInfo { … backend: … }` **และ** บรรทัดแรก `[harness] VOXELFORGE_URL <url>` (`web-verify.mjs` ใส่ให้เอง) | หลักฐาน authoritative ของ **W0-A (backend)** และ **W0-C (recipe)**. ไม่มี log = สคริปต์ **exit 2 NOT GRADEABLE** ไม่ใช่เตือนแล้วปล่อยผ่าน — "ไม่มี log" กับ "ตกไป Gl เงียบๆ" แยกกันไม่ออก |
| ③ native control | `docs/assets/wide-hero-final.png` (ใช้ตัวที่มีอยู่ได้เลย) หรือเรนเดอร์ใหม่เป็น `native-control-v3.png` | `bash scripts/render_wide_hero.sh` — **recipe tilt-down ชุดเดียวกับ URL ของ web เป๊ะ** | คือ baseline ที่ CEO อนุมัติ. ถ้าส่ง framing อื่นมา **W0-D จะเด้ง exit 2** เพราะ delta จะกลายเป็นการวัดกล้อง ไม่ใช่วัดเว็บ |

### 2.6.1 URL ที่ต้องใช้เปิดเว็บ (สร้างจาก recipe ตัวจริง ห้ามพิมพ์มือ)

```bash
python scripts/hero_recipe.py --url http://127.0.0.1:<port>/     # พิมพ์ URL เต็ม
python scripts/hero_recipe.py --query                            # เอาไปใส่ web-verify.mjs --query
python scripts/hero_recipe.py --check-url "<url ที่แคปจริง>"      # เช็คก่อนส่ง (exit 1 = ไม่ตรง)
```

query ที่ได้ตอนนี้ (สะท้อน `render_wide_hero.sh` ตรงๆ · `?hero` ถูกเติมให้เพราะ shot binary ไม่ต้องใช้แต่เว็บต้อง):

```
?hero&wide&cam=7.6,6.4,-6.0,7.6,2.7,8.0,60&sun=19,196,26000&dof=8,10&exposure=9.0
&grade=0.02,1.00,1.30&ambient=2800&ambcolor=0.70,0.60,0.44&bluescale=0.85
&bounce=1.0&bounce2=1.7&shoulder=0.64&dust=3.0
```

> `hero_recipe.py` **parse `render_wide_hero.sh` ตอนรัน** ไม่ได้ก็อปค่ามาเก็บไว้ — แก้ driver
> ที่เดียว URL กับ native control ขยับตามพร้อมกัน ไม่มีทางหลุดคนละสูตร.
> ถ้าเพิ่ม knob ใหม่ใน `main.rs::read_cfg` ต้องเพิ่มใน `ENV_TO_QS` ด้วย ไม่งั้นสคริปต์เตือนว่า
> knob นั้น **ส่งเข้าเว็บไม่ได้**

> **naming convention ที่สคริปต์รู้จักเอง:** วาง log ไว้ข้างๆ เฟรม web ในชื่อ **`<web>.console.txt`**
> แล้ว `grade_web_parity.py` จะ auto-detect ให้ ไม่ต้องพิมพ์ `--console` (จะพิมพ์ก็ได้ ชนะ auto-detect)
>
> ⚠️ **ห้ามใช้ชื่อ `wasm-first-frame-v2.png` ซ้ำ** — ชื่อนั้นถูกตัดสินไปแล้วใน §5 ว่า NOT GRADEABLE
> (Gl + terrain) และไฟล์ตัวนั้นถูกย้ายไป `docs/assets/archive/wasm-first-frame-v2-Gl-NOTGRADEABLE.png`
> แล้ว. เขียนทับ path เดิม = หนึ่งชื่อสองเฟรม บันทึกการตัดสินจะอ้างผิดตัวทันที **รอบใหม่ใช้ `-v3`**

- **resolution ต้องตรง** — native window 1280×720 (`main.rs:334`), เว็บ `fit_canvas_to_parent` → ตั้ง viewport 1280×720 เป๊ะ
- **TAA settle ต้องเท่ากัน** — hero shot ฝั่ง native รอ ~3.2s ก่อนแคป (`hero.rs:829-834`) เฟรม web ต้องรอเท่ากันหรือมากกว่า ไม่งั้นจะวัด "TAA ยังไม่นิ่ง" แล้วโทษ web ผิด
- ⏱ **ห้ามแคป native ตอนเครื่องกำลัง build** — 3.2s นั้นเป็น **เวลาจริง (wall-clock)** ไม่ใช่จำนวนเฟรม:
  ถ้า CPU/GPU ถูก build หรือ renderer ตัวที่สองแย่งไป TAA/dust/volumetric จะสะสมได้น้อยลงในหน้าต่างเดิม.
  วัดจริง 2026-07-29 (คำสั่ง/exe/recipe เดียวกัน ต่างแค่โหลดเครื่อง): **micro −0.94** — เกิน
  `BASELINE_TOL` (±0.4) ของ W0-D ตัวเดียวก็พอเด้งแล้ว เพราะ micro คือแกนที่วัด dust motes ตรงๆ
  (warmth ขยับแค่ ~−0.2). `render_native_control_v3.sh` เช็ค `tasklist` ก่อนยิง แล้ว exit 3 ถ้าเจอ

### ⚠️ 2.6.2 กับดัก CRLF — recipe เข้าไม่ครบแบบเงียบๆ (เจอจริง 2026-07-29)

`hero_recipe.py` เป็นตัวป้อน recipe ให้ทั้งสองเลน แต่ **Python บน Windows เขียน stdout เป็น
`\r\n`** ส่วน bash `$( )` / `mapfile -t` ตัดให้แค่ `\n` → `env` ได้ `VOXELFORGE_AMBIENT=2800\r`.

ฝั่ง Rust จัดการสองแบบไม่เหมือนกัน:

| knob | ทางที่ parse | เจอ `\r` แล้ว |
|---|---|---|
| `cam` `sun` `dof` `grade` `ambcolor` | `env_floats` → `s.trim().parse()` | **รอด** (trim กิน `\r`) |
| `exposure` `ambient` `bluescale` `bounce` `bounce2` `shoulder` `dust` | `v.parse()` เปล่าๆ | **ตกเงียบ → ใช้ baked default** |

ผลคือ "กล้องถูก แต่ไฟผิด" — เหมือนอาการของ recipe ผิดชุดเป๊ะ. วัดได้ **warmth −5.13 / blue −1.17**
จาก baseline ที่ driver ตัวเดียวกันเรนเดอร์ซ้ำได้ **+0.02** (พิสูจน์ว่า source ไม่ได้เพี้ยน).
ฝั่ง web ก็มีรูเดียวกัน (`\r` ไปเกาะ knob ตัวสุดท้ายของ query string).

**แก้ที่ต้นทาง:** `hero_recipe.py::main` เรียก `sys.stdout.reconfigure(newline="\n")` — ปิดทั้งสองเลนพร้อมกัน.
🔎 หมายเหตุที่ยังค้าง: `qs_num` ฝั่ง wasm `.trim()` อยู่แล้ว แต่ scalar ฝั่ง native ยังไม่ trim →
ถ้าจะรัดให้เท่ากันจริง ควรเติม `.trim()` ให้ scalar ใน `main.rs::read_cfg` (native) + `shot_main.rs` ด้วย

**tolerance ของ axis delta (|web − native|)** — **calibrated 2026-07-29** จาก drift ที่วัดได้จริงของคู่แรก
ที่ผ่าน W0 ครบ (§6.2) ≈ 10× ของ drift นั้น. **ตัวเลขในตารางนี้ = `AXIS_TOL` ใน `scripts/grade_web_parity.py`
เป๊ะ** — ถ้าแก้ตัวใดตัวหนึ่ง ต้องแก้อีกฝั่งพร้อมกันเสมอ (ห้ามมีเกณฑ์ชุดที่สอง):

| แกน | tol | ที่มา (drift ที่วัดได้ §6.2) | **tilt-down baseline** (`wide-hero-final.png`) |
|---|---|---|---|
| warmth R−B | **±2.0** | วัดได้ +0.17 → ~12× · ยังต่ำกว่า headroom ของ rubric 4 เท่า | 128.88 |
| blue B | **±1.0** | วัดได้ −0.01 · 1.0 คือขอบล่างที่เชื่อได้บน output 8-bit | 5.19 |
| saturation | **±1.0 pp** | วัดได้ +0.01 | 94.48 |
| highlight p95 | **±2.0** | วัดได้ +0.00 · 2.0 ยังจับ tonemap shift จริงในแบนด์กว้าง 35 ได้ | 177.36 |
| **micro-contrast** | **ห้ามต่ำกว่า native เกิน 0.4** (สูงขึ้นได้ถึง +1.0) | ตัด PCSS ทำให้ขอบเงา *คมขึ้น* → hi-freq *เพิ่ม*. ถ้ามัน **ลด** = เว็บทำ detail หายจริง (วัดได้ +0.03) | 5.92 |
| **DOF fg:bg** | web ÷ native ≥ 0.85 · **< 0.60 = FAIL** | suspect 🔴 DUAL_SOURCE_BLENDING (วัดได้ 0.99) | 0.17 (wide — ดู tradeoff §1) |

> ⚠️ micro-contrast ที่ **พุ่งขึ้นเกิน +1.0** ไม่ใช่ข่าวดี — นั่นคือ aliasing/TAA หาย
> ⚠️ **first-run rule (ยังใช้อยู่ แต่ตอนนี้แคบลง):** ค่าข้างบน calibrate บน **GTX 1060 + Chrome/Dawn vs Vulkan native
> เครื่องเดียว**. ถ้าเป็น **เครื่อง/เบราว์เซอร์/GPU ตัวใหม่ที่ยังไม่มีแถวใน §6.2** แล้วพลาดเฉพาะ axis delta
> ไม่เกิน **2 เท่าของ tol** → เติมแถว calibration ก่อน อย่าเพิ่งตัดสินว่า regression.
> **บนเครื่องที่ calibrate ไว้แล้ว (§6.2) ไม่มีผ่อนผัน — เกิน tol = FAIL**

**อย่าสับสนกับ `BASELINE_TOL`** — คนละด่าน คนละหน้าที่:

| | เทียบอะไรกับอะไร | ตั้งจาก | ตกแล้วได้อะไร |
|---|---|---|---|
| `AXIS_TOL` (ตารางบน) | web ↔ native control | **drift web↔native ที่วัดได้จริง** (§6.2) ×~10 | **FAIL** (exit 1) — เว็บทำลุคเพี้ยน |
| `BASELINE_TOL` (W0-D) | native control ↔ tilt-down baseline | **noise ที่วัดจริง** (≤0.15 จากการเรนเดอร์ซ้ำ) ×10–25 | **NOT GRADEABLE** (exit 2) — ส่ง control ผิดเฟรม ยังไม่ได้ตัดสินเว็บเลย |

---

## 2.7 คำสั่งเดียวที่ต้องรัน

```bash
python scripts/grade_web_parity.py \
  --web     docs/assets/wasm-hero-v3.png \
  --native  docs/assets/wide-hero-final.png \
  --console docs/assets/wasm-hero-v3.png.console.txt \
  --json    parity-v3.json
# --console ละได้ถ้าไฟล์ชื่อ <web>.console.txt วางอยู่ข้างๆ (auto-detect)
# --baseline ไม่ต้องใส่ — default = docs/assets/wide-hero-final.png (tilt-down ที่ CEO อนุมัติ)
#            ใส่ก็ต่อเมื่อ CEO re-lock framing ใหม่เท่านั้น
# exit 0 = PASS (รวม web-deviation) · 1 = FAIL · 2 = NOT GRADEABLE (capture ผิด/ไม่มี log/ผิด recipe/ผิด baseline)
```

สคริปต์ทำ **W0-A** (อ่าน `AdapterInfo` จาก log — **ไม่ใช่ HUD**, และ **ไม่มี log = exit 2** ไม่มี flag ไหน override ได้),
**W0-C** (เทียบ `VOXELFORGE_URL` ใน log กับ recipe tilt-down รายตัว), **W0-D** (fingerprint native control
กับ baseline), dead-frame guard, G3/G5/G6, axis delta, DOF ratio, penumbra + discriminator ให้ครบ.
**ยังต้องใช้คน:** W0-B (ฉากถูกไหม — สคริปต์เช็คได้แค่ว่า `?hero` อยู่ใน URL ไม่ได้ดูรูป),
SSAO contact-AO (zoom 400%), G1 voxel edge, G2 ทิศแดด

> สคริปต์ **import `measure()` จาก `grade_axes.py`** และ **shell out ไป `measure_penumbra.py`/`grade_gate.py`**
> โดยตั้งใจ — เลขทุกตัวที่มีเจ้าของอยู่แล้วห้ามมีสำเนาที่สอง
>
> ⚠️ ข้อจำกัดที่รู้ตัว: การจับ "pass ไหนไม่โหลด" ใช้ **string match กับ log ของ Bevy** ถ้า Bevy เปลี่ยน
> ข้อความ มันจะเงียบ (fail-open) → ต้องอ่าน output ของ `web-verify.mjs` ควบด้วยเสมอ ห้ามเชื่อสคริปต์ตัวเดียว

---

## 3. วิธีอ่านผล → verdict

| verdict | เงื่อนไข | หมายความว่า |
|---|---|---|
| ⛔ **NOT GRADEABLE** | ตก W0-A/B/C/**D** ข้อใดข้อหนึ่ง **หรือพิสูจน์ไม่ได้ (เช่น ไม่มี console log → W0-A/W0-C UNVERIFIED)** · เฟรมดำ · aspect ไม่ตรง · pass หลุดนอกจาก PCSS | **ยังตัดสินลุคไม่ได้** ต้องแคปใหม่ — ห้ามเขียนว่า FAIL (คนละความหมาย). **"ไม่มีหลักฐาน" = ไม่ผ่าน ไม่ใช่ "ผ่านแบบมีหมายเหตุ"** |
| ✅ **PASS** | W0 ครบ **(รวม D)** + G3/G5/G6 ผ่าน + axis delta ในกรอบ + **penumbra ≥ 3.0px** + ตาคนบอก "ลุคเดียวกัน" | web ปล่อยได้ ไม่มีหนี้ลุค |
| 🟡 **PASS w/ expected web deviation** | เหมือนบน แต่ **penumbra 2.0–3.0px และ drop ≤ 1.5px** | ปล่อยได้ **พร้อมบันทึกหนี้ 1 บรรทัด** ใน `look-webgpu-risk.md` ว่าเงา web นุ่มน้อยกว่า native เท่าไหร่ |
| ❌ **FAIL** | ตกแกนห้ามพลาด (#1/#2/#4/#6/voxel edge) · **penumbra < 2.0px** · **drop > 1.5px** · DOF ratio < 0.60 · micro ตกเกิน 0.4 | ส่งกลับ — และต้องระบุว่าตกเพราะ *อะไร* **ห้ามเขียนว่า "เพราะตัด PCSS" ถ้ามันไม่ใช่** |

### Scorecard (ก็อปไปติ๊กตอนส่ง)

```
web frame: ____________________  native control: ____________________
console log: __________________  res: ______x______  TAA settle: ____s (native 3.2s)

── W0 · ADMISSIBILITY ──────────────────────
W0-A backend = BrowserWebGpu (จาก AdapterInfo ไม่ใช่ HUD)  ☐P ☐F
W0-B ฉาก hero (?hero + ตาดูรูป)                            ☐P ☐F
W0-C recipe tilt-down ครบ (hero_recipe.py --check-url)     ☐P ☐F
W0-D native control = tilt-down baseline (fingerprint)     ☐P ☐F

── HARD PARITY (waiver ไม่ครอบ) ─────────────
G3 ☐P ☐F   G5 ☐P ☐F   G6 ☐P ☐F
warmth Δ____(±2.0)  blue Δ____(±1.0)  sat Δ____(±1.0)  p95 Δ____(±2.0)
micro Δ____(≥ −0.4, rise >+1.0 = ธง)   DOF web/native ____(≥0.85, <0.60=FAIL)
SSAO contact AO (ตา 400%) ☐ ยังเห็น ☐ วัตถุลอย = FAIL

── PCSS-OFF WAIVER ─────────────────────────
penumbra web ____px  native ____px  drop ____px
☐ ≥3.0 PASS   ☐ 2.0–3.0 web-waived   ☐ <2.0 FAIL
☐ drop ≤1.5 (PCSS ล้วน)   ☐ drop >1.5 → ส่งกลับตรวจ TAA ห้าม waive

VERDICT: ☐ NOT GRADEABLE  ☐ PASS  ☐ PASS w/ deviation  ☐ FAIL
เหตุผล: ______________________________________________
```

---

## 4. Handoff — สิ่งที่ต้องทำก่อน/ตอนแคปรอบ v3

1. ~~**บังคับ WebGPU ให้ติดจริง**~~ ✅ **เสร็จแล้ว 2026-07-29** — `web-verify.mjs` มี gate `backendFailure`
   ที่ตัดสินจาก `bevyBackend` (ไม่ใช่ HUD) แล้ว ได้ `Gl` = verdict FAIL ไม่ปล่อยผ่าน · เฟรม v3 เปิดได้
   `AdapterInfo.backend == BrowserWebGpu` จริง (§5 แถวล่าสุด, §7)
2. ~~**เปิด look knobs บน wasm**~~ ✅ **เสร็จแล้ว 2026-07-29** — wasm `read_cfg()` parse query string
   ครบทุก knob และ `hero.rs` ไม่เหลือ `std::env::var` แล้ว (ดูกล่องเขียวใน §0)
3. **แคปด้วย URL จาก `hero_recipe.py --url`** (§2.6.1) ไม่ใช่ `?hero` เปล่าๆ —
   `?hero` เดี่ยวจะได้ **baked default = narrow tight hero** ซึ่งคนละ framing กับ baseline → W0-C/W0-D เด้ง
4. **native control**: `bash scripts/render_wide_hero.sh` (หรือใช้ `wide-hero-final.png` ที่มีอยู่)
5. **วัด FPS บนเฟรมนั้น** — FPS จากฉาก editor ใช้เทียบ Lite target ไม่ได้ (คนละ workload).

---

## 5. บันทึกผลการตัดสิน

| วันที่ | เฟรม (path ที่เก็บถาวรแล้ว) | Backend | ฉาก / recipe | Verdict |
|---|---|---|---|---|
| 2026-07-29 | `assets/archive/wasm-first-frame-v2-Gl-NOTGRADEABLE.png` (เดิมชื่อ `assets/wasm-first-frame-v2.png`) | **Gl (WebGL2)** ❌ | terrain/editor ❌ | **NOT GRADEABLE** — ตก W0-A + W0-B + W0-C |
| 2026-07-29 | `assets/archive/native-control-v3-tiltB-recipe-REJECTED.png` (**native control ตัวแรกของรอบ v3**) | native | tilt-down cam ✅ แต่ recipe เก่า "Hero tilt-B" ❌ (ตก SHOULDER/DUST/BOUNCE/BOUNCE2, SUN 20000, AMBIENT 4400, EXPOSURE 8.82, GRADE 1.15) | **NOT GRADEABLE (W0-D)** — เทียบ baseline: warmth **+53.78** · p95 **+19.21** (196.57 หลุดแบนด์ 150–185) · micro **−1.30** · penumbra **2.92px** (ต่ำกว่าบาร์ G4 เอง) → ใช้เป็น anchor ไม่ได้ |
| **2026-07-29** | **`assets/wasm-hero-v3.png`** + `native-control-v3.png` (คู่แรกที่ผ่าน W0 ครบ) | **BrowserWebGpu** ✅ | hero ครัว tilt-down ✅ · recipe ตรงทุก knob ✅ | ✅ **PASS (full parity)** — G3/G5/G6 PASS · axis delta สูงสุด **+0.17** · DOF rel **0.99** · penumbra **web 4.92px = native 4.92px** (PCSS-off ไม่เสียอะไรเลย) · ไม่มี pass ไหนหลุด — *(machine pass เท่านั้น; ตาคนยังไม่ได้ตรวจตอนลงแถวนี้ ดูแถวล่าง)* |
| **2026-07-29 · SIGN-OFF (Flamingo)** | เฟรมชุดเดิม `wasm-hero-v3.png` ↔ `native-control-v3.png` ↔ golden `wide-hero-final.png` | **BrowserWebGpu** ✅ | hero ครัว tilt-down ✅ | ✅ **PASS — เซ็นรับ web parity v3** · G1/G2/G4 ตรวจด้วยตา 400% ผ่านครบบนเฟรม web · **แต่พบ deviation ตัวที่สองที่ตาคนเท่านั้นจับได้: `DUST_INSTANCE_DROP` — voxel ของ teal accent หาย 1 ลูก บน NATIVE (เว็บถูก)** → §2.8 + **BLOCKER-N1** (§8) |

> เฟรมที่ถูกตัดสินแล้วจะถูกย้ายเข้า `docs/assets/archive/` พร้อม `.console.txt`/`.browser.log` ของมัน
> ทันทีที่ลงบันทึก — บันทึกนี้ต้องชี้ไปที่ **ไฟล์ตัวที่ถูกตัดสินจริง** ตลอดไป ห้ามให้ชื่อถูกเขียนทับ
>
> 📌 **บทเรียนจากแถวที่ 2:** control ตัวนั้นพิมพ์ค่าจากโน้ต "Hero tilt-B" ด้วยมือ — กล้องถูก แต่ไฟเป็นชุดก่อน
> atmosphere pin. ถ้าไม่มี W0-D มันจะกลายเป็น anchor แล้วเว็บถูกตัดสินเทียบของผิด **โดยที่ทุกด่านอื่นผ่านหมด**.
> ตอนนี้ `render_native_control_v3.sh` / `verify_web_v3.sh` ไม่เก็บค่าเองแล้ว — ดึงจาก
> `hero_recipe.py` ซึ่ง parse `render_wide_hero.sh` ทั้งคู่ (แก้ที่เดียว ขยับพร้อมกัน)

---

## 6. Calibration log

### 6.1 native ↔ native (วัดแล้ว — เป็นที่มาของ `BASELINE_TOL`)

| วันที่ | เฟรม A | เฟรม B | warmth Δ | blue Δ | sat Δ | p95 Δ | micro Δ | penumbra | สรุป |
|---|---|---|---|---|---|---|---|---|---|
| 2026-07-29 | `wide-hero-final.png` (baseline ที่ล็อก) | `archive/tiltdown-baseline-reverify.png` (เรนเดอร์ซ้ำจาก source ปัจจุบัน, exe ใหม่) | **+0.15** | −0.01 | +0.01 | +0.00 | −0.00 | 4.83 → 4.92px | **noise floor ของการ re-shoot ≤ 0.15** (dust motes สุ่ม → ไม่ byte-identical แต่ตัวเลขนิ่ง) |
| 2026-07-29 | `wide-hero-final.png` | `archive/native-control-v3-tiltB-recipe-REJECTED.png` (recipe เก่า, กล้องเดียวกัน) | **+53.78** | +0.87 | +2.42 | **+19.21** | **−1.30** | 4.83 → 2.92px | **recipe ผิดชุดตรวจจับได้ชัด** แม้กล้องตรงกัน → W0-D มีของจริงให้จับ |

→ `BASELINE_TOL = warmth ±2.0 · blue ±1.0 · sat ±1.5 · p95 ±4.0 · micro ±0.4`
(≈ 10–25× noise ที่วัดได้ · ยังห่างจาก drift ของ recipe ผิดหลายเท่า → ไม่ false-positive ไม่ false-negative)

### 6.2 web ↔ native — **มีคู่แรกแล้ว** (2026-07-29) → `AXIS_TOL` รัดแล้ว

| วันที่ | web | native control | warmth Δ | blue Δ | sat Δ | p95 Δ | micro Δ | DOF rel | penumbra web/native | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| 2026-07-29 | `wasm-hero-v3.png` (Chrome+Dawn, BrowserWebGpu) | `native-control-v3.png` (Vulkan) | **+0.17** | −0.01 | +0.01 | +0.00 | +0.03 | **0.99** | **4.92 / 4.92 px** | ✅ **PASS (full parity)** |
| 2026-07-29 (re-verify หลังรัด tol) | `wasm-hero-v3.png` | `native-control-v3.png` | **−0.11** | +0.02 | −0.03 | +0.00 | +0.02 | **0.99** | 4.83 / 4.92 px | ✅ PASS — รันซ้ำด้วย `AXIS_TOL` ชุดใหม่ |
| 2026-07-29 (re-verify vs baseline) | `wasm-hero-v3.png` | `wide-hero-final.png` (baseline) | **+0.08** | +0.00 | −0.01 | +0.00 | +0.01 | **1.00** | 4.83 / 4.83 px | ✅ PASS |

> สองแถวล่างคือการรันซ้ำบนไฟล์ชุดเดิมหลังรัด tol — ทุกแกน **≤ 0.11** ทั้งคู่ (ต่ำกว่าแถวแรกด้วยซ้ำ)
> → drift ที่ใช้ตั้ง `AXIS_TOL` (0.17) ยังเป็นตัวเลขที่ conservative ที่สุดที่วัดได้ ค่า tol ใหม่จึงไม่ต้องขยับ

**รัดค่าแล้วตามที่สัญญาไว้** — `AXIS_TOL` เดิม (provisional จาก headroom ของ rubric) หลวมเกินจริงมาก:
warmth ±8 จะปล่อย regression ขนาด 5 หน่วยผ่านได้สบาย. ค่าใหม่ ≈ **10× ของ drift ที่วัดได้จริง**

| แกน | เดิม (provisional) | **ใหม่ (calibrated)** | drift ที่วัดได้ |
|---|---|---|---|
| warmth R−B | ±8 | **±2.0** | +0.17 |
| blue B | ±3 | **±1.0** | −0.01 |
| saturation | ±5 | **±1.0** | +0.01 |
| highlight p95 | ±10 | **±2.0** | +0.00 |
| micro drop / rise-flag | −0.6 / +1.5 | **−0.4 / +1.0** | +0.03 |

> ⚠️ **ขอบเขตของ calibration นี้:** วัดบน **GTX 1060 + Chrome/Dawn** เทียบ **Vulkan native** เครื่องเดียว.
> เครื่อง/เบราว์เซอร์อื่น (โดยเฉพาะ integrated GPU ที่เป็นเป้า Lite) อาจ drift มากกว่านี้โดยไม่ใช่ regression —
> **first-run rule ยังอยู่:** พลาดไม่เกิน 2× tol บนเครื่องใหม่ = เติมแถว calibration ก่อน อย่าเพิ่งตัดสิน
>
> 💡 **สิ่งที่คู่นี้พิสูจน์ให้เห็นเลย:** penumbra web = native เป๊ะ (4.92px) → **การตัด PCSS ไม่ได้ทำให้เงาแข็งขึ้นเลยแม้แต่นิด**
> ตรงกับที่ `hero.rs:591-599` เขียนไว้ว่าความนุ่มมาจาก `ShadowFilteringMethod::Temporal` + TAA ไม่ใช่ PCSS.
> **waiver 2–3px ยังคงไว้เป็นตาข่ายกันเครื่องอื่น แต่รอบนี้ไม่ได้ใช้ — ไม่มีหนี้ลุคไปลง `look-webgpu-risk.md`**

---

## 7. สถานะรอบ v3 — ✅ ปิดจ็อบ (2026-07-29)

| ไฟล์ | สถานะ |
|---|---|
| ① `docs/assets/wasm-hero-v3.png` + `.console.txt` + `.browser.log` | ✅ ได้แล้ว (`bash scripts/verify_web_v3.sh`) — backend `BrowserWebGpu`, URL ตรง recipe |
| ③ `docs/assets/native-control-v3.png` | ✅ `bash scripts/render_native_control_v3.sh` — ผ่าน W0-D (warmth −0.05 จาก baseline) |
| baseline | ✅ `docs/assets/wide-hero-final.png` — ยืนยัน re-shoot ได้ (§6.1) |
| **verdict** | ✅ **PASS — เซ็นรับแล้ว (Flamingo, 2026-07-29)** · `parity-v3.json` · ภาพเทียบ: `docs/assets/web-parity-v3-compare.png` · หลักฐาน sign-off: `docs/assets/web-parity-v3-signoff-evidence.png` · ⚠️ ติด **BLOCKER-N1** ที่เลน native (§2.8/§8) |

### ⚠️ กับดัก 2 ตัวที่เจอระหว่างรอบนี้ — ต้องอ่านก่อนถ่ายเฟรมรอบหน้า

ระหว่างรอบนี้เจอเฟรม native ที่วัดได้ **warmth −5.1 จาก baseline** แล้วเกือบสรุปผิดว่าเป็น "โหลดเครื่อง".
แยกออกมาได้ 2 สาเหตุ **คนละตัว คนละแกน** (วัดจริงทั้ง 4 เคส · exe เดียวกัน recipe เดียวกัน):

| เคส | warmth | micro | สาเหตุจริง |
|---|---|---|---|
| idle + env **LF** | −0.05 | −0.02 | ✅ ถูกต้อง = baseline |
| idle + env **CRLF** | **−5.13** | −0.01 | 🐞 **บั๊ก CRLF** |
| wasm build รันอยู่ + CRLF | −5.34 | **−0.95** | บั๊ก + โหลด |
| (แยกส่วนโหลดออกมา) | ~−0.2 | **−0.94** | 🐌 **โหลดเครื่อง** |

1. 🐞 **CRLF ฆ่า recipe ทั้งชุดแบบเงียบ** — `hero_recipe.py --env` พิมพ์ออก stdout บน Windows แล้วได้ `\r\n`
   → `\r` ติดไปกับค่า → `v.parse::<f32>()` ฝั่ง Rust reject **ทุก knob ที่เป็นตัวเลข** → ตกกลับไปใช้ baked default
   เงียบๆ **ไม่มี error สักบรรทัด**. แก้แล้วด้วย `sys.stdout.reconfigure(newline="\n")`.
   ฝั่ง web ก็มีรูเดียวกัน (`\r` ไปเกาะ knob ตัวสุดท้ายของ query string)
2. 🐌 **แคปตอนเครื่องมีงานหนัก = เฟรมยังไม่ converge** — จุดแคปคือ **wall-clock t>3.2s**
   (`screenshot_once`) ไม่ใช่ "ครบ N เฟรม" → มี build/renderer อื่นแย่ง GPU-CPU แล้วเฟรมที่สะสมได้น้อยลง.
   **แกนที่โดนคือ micro-contrast (−0.94)** เพราะ dust motes + TAA คือของที่หยุดสะสม — เกิน `BASELINE_TOL` (0.4) ชัดๆ

> **บทเรียนที่ขอให้จำ:** อาการเดียวกัน (เฟรมเพี้ยนจาก baseline) มาจากคนละสาเหตุได้ และ **ลายเซ็นต่างกันตามแกน** —
> ค่าเลื่อนทั้ง recipe = ตรวจ input pipeline ก่อน · micro ตกอย่างเดียว = เรื่อง convergence/โหลด.
> ทั้งสองเคสถ้าไม่มี W0-D จะไหลเข้าไปเป็น "หลักฐาน" ว่าเว็บทำลุคพัง โดยที่เว็บไม่ได้ผิดอะไรเลย.
> `render_native_control_v3.sh` ตอนนี้กันทั้งสองชั้น: ปฏิเสธถ่ายเมื่อมี build/renderer อื่นรันอยู่ + รัน W0-D ให้เองก่อนจบสคริปต์

---

## 8. ✅ OFFICIAL SIGN-OFF — web parity v3 (Flamingo, Designer · 2026-07-29)

```
web frame: docs/assets/wasm-hero-v3.png     native control: docs/assets/native-control-v3.png
console log: wasm-hero-v3.png.console.txt   res: 1280x720   TAA settle: 3.2s+ (native 3.2s)
golden tilt-down ที่เทียบ: docs/assets/wide-hero-final.png

── W0 · ADMISSIBILITY ──────────────────────
W0-A backend = BrowserWebGpu (จาก AdapterInfo ไม่ใช่ HUD)  [P] — log บรรทัด bevy_render/mod.rs:288
W0-B ฉาก hero (?hero + ตาดูรูป)                            [P] — ครัวไม้ + หน้าต่าง + ชาม + teal accent ครบ
W0-C recipe tilt-down ครบ (hero_recipe.py --check-url)     [P] — knob ครบ shoulder/dust/bounce/bounce2
W0-D native control = tilt-down baseline (fingerprint)     [P] — เรนเดอร์ซ้ำเองได้ +-0.00 ทุกแกน

── HARD PARITY (waiver ไม่ครอบ) ─────────────
G3 [P]   G5 [P]   G6 [P]
warmth D-0.11 (+-2.0)  blue D+0.02 (+-1.0)  sat D-0.03 (+-1.0)  p95 D+0.00 (+-2.0)
micro D+0.02 (>= -0.4)                       DOF web/native 0.99 (>=0.85)
SSAO contact AO (ตา 400%) [ยังเห็น] — แถบเข้มที่ฐานเคาน์เตอร์/ฐาน accent ตรงกับ golden
G1 voxel edge  [P] ขอบ 90 องศา คมทุกวัตถุหลัก (เทียบ 400% กับ golden = เหมือนกัน)
G2 ทิศ key light [P] แถบแสงกรอบหน้าต่าง + เงาทอดทิศเดียว ตรงกับ golden

── PCSS-OFF WAIVER ─────────────────────────
penumbra web 4.83px  native 4.83-4.92px  drop 0.00-0.09px
[X] >=3.0 PASS  -> waiver ไม่ถูกใช้เลยรอบนี้ ไม่มีหนี้ลุคจาก PCSS

── DEVIATION ตัวที่สอง (เจอตอน sign-off) ────
DUST_INSTANCE_DROP — voxel teal accent หาย 1 ลูกบน NATIVE (เว็บวาดถูก) — ดู §2.8
[ ] web ผิด    [X] native/golden ผิด  -> ไม่หัก verdict ของเว็บ

VERDICT: [X] PASS   (NOT GRADEABLE / PASS w-deviation / FAIL = ไม่ใช่)
เหตุผล: W0 ครบ 4 ข้อ · G1-G6 ผ่านครบ (3 ข้อสุดท้ายตรวจด้วยตา 400% เอง) · axis delta สูงสุด 0.11
        · penumbra เท่ากับ native · ก้อนสว่างใน diff panel พิสูจน์แล้วว่าเป็นข้อบกพร่องฝั่ง native
```

### 🔴 BLOCKER-N1 — เลน native + golden beauty shot (ไม่ใช่เลนเว็บ)

`docs/assets/wide-hero-final.png` ที่ CEO อนุมัติและล็อกไว้ **มีรูอยู่ในตัว hero accent block**
(voxel หายไป 1 ลูก · 5,620 px ของ silhouette · §2.8). golden ตัวนี้ยังใช้เป็น parity baseline ได้
เพราะทั้งสองฝั่งเทียบ *ลุค* ไม่ใช่ geometry — แต่ **ใช้เป็นภาพโชว์/beauty shot ไม่ได้จนกว่าจะแก้**

ลำดับที่ควรทำ (ต้องให้ CEO อนุมัติก่อน re-lock golden ใหม่ — framing/golden ล็อกโดย CEO):
1. เลน render หา root cause ของ instance ที่หายบน GPU-preprocessing path (`DUST>0` = ตัวจุด)
2. แก้แล้วเรนเดอร์ golden ใหม่ด้วย `scripts/render_wide_hero.sh` ตัวเดิม (recipe ไม่ต้องแตะ)
3. CEO ตรวจ + re-lock → แล้วค่อยอัปเดต `BASELINE_TOL` ถ้าตัวเลข baseline ขยับ
4. ระหว่างนี้ **ห้ามใช้ `wide-hero-final.png` เป็นภาพโปรโมต**

หลักฐาน 4 ช่อง (golden / native DUST=3 / native DUST=0 / web): `docs/assets/web-parity-v3-signoff-evidence.png`

---

_ตัวเลข PCSS waiver + native-control protocol + `grade_web_parity.py` + baseline tilt-down เตรียมไว้ก่อนเฟรม web มาถึง
(ไม่ได้เปิดเบราว์เซอร์ — เลนนั้นของ Poppy) — Flamingo (Designer), 2026-07-29_

_Sign-off §8 + deviation §2.8: ตรวจเอง เรนเดอร์ native ซ้ำเอง 8 เฟรมเพื่อ isolate ตัวแปร — Flamingo (Designer), 2026-07-29_
