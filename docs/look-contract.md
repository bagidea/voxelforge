# Voxelforge — LOOK CONTRACT (gameplay camera)

> **สัญญาค่าลุคของ "กล้องที่เล่นจริง"** — ไม่ใช่ hero shot, ไม่ใช่ editor, ไม่ใช่ bench.
> ใครก็ตามที่งานของตัวเองต้อง "รู้ค่าเดียวกันกับที่เรนเดอร์ใช้" ให้อ่านไฟล์นี้ **แล้ว
> import ค่าจริงจาก `client/src/look.rs`** อย่าคัดตัวเลขไปฝังซ้ำ.
>
> **Owner:** Flamingo (Look Lead) · **ไฟล์ที่เป็น source of truth:** `client/src/look.rs`
> **Consumers:** Kevin (chunk streaming / render distance), Poppy (perf tiers), Rose, Sun.

---

## 0. กฎเหล็กข้อเดียวของเอกสารนี้

**ตัวเลขในหน้านี้เป็น "สำเนาให้คนอ่าน" ของ `pub const` ใน `client/src/look.rs` เท่านั้น.**
โค้ดที่ต้องใช้ค่าพวกนี้ **ต้อง `use crate::look::…`** — ห้ามพิมพ์เลขซ้ำในไฟล์ตัวเอง.
เหตุผล: บทเรียนของออฟฟิศเราคือ "อย่ามีค่าความจริงสองชุด" — วันที่ผมขยับหมอก
streaming ของ Kevin ต้องขยับตามเองโดยไม่ต้องมีใครจำได้ว่าต้องไปแก้ที่ไหนอีก.

```rust
use crate::look::{FOG_START, FOG_END, RENDER_RADIUS};
```

---

## 1. DistanceFog — ค่าที่ประกาศ (Golden Hour = ค่า default ที่ชิป)

| อะไร | ค่า | const ใน look.rs |
|---|---|---|
| สีหมอก | `Color::srgb(0.60, 0.72, 0.88)` (ฟ้าเทาขอบฟ้า) | `FOG_COLOR_DAY` |
| falloff | **`FogFalloff::Linear`** | — |
| start (ใสสนิท) | **112.0 บล็อก** | `FOG_START` |
| end (ทึบเต็ม) | **320.0 บล็อก** | `FOG_END` |
| สีเรืองรอบดวงอาทิตย์ | `Color::srgb(1.00, 0.85, 0.60)` | `FOG_SUN_GLOW` |
| exponent ของ glow | `30.0` | `FOG_SUN_EXPONENT` |

**ทำไม Linear ไม่ใช่ Exponential:** ค่าเดิมในลาน look เป็น `Exponential { density: 0.008 }`
ซึ่ง "เริ่มขุ่นตั้งแต่ก้าวแรก" — ที่ 40 บล็อกก็ขุ่นไปแล้ว ~27% ทำให้ของกลางระยะซีดลงทั้งที่ยังไม่ไกล.
ภาพอ้างอิงของ CEO (บ้านไม้กลางป่าสน) ต้นสนหลังบ้าน **ยังเขียวอิ่มและคมทุกใบ** — หมอกไปโผล่
เฉพาะ "ไกลจริงๆ" เท่านั้น. `Linear{112,320}` ให้ **0% ที่ ≤112 บล็อก** แล้วค่อยไต่เชิงเส้น
→ ทุกอย่างในระยะที่ผู้เล่นสนใจใสสนิท, ขอบโลกละลายเข้าฟ้าเงียบๆ.

**Night preset** (`VOXELFORGE_LOOK_NIGHT=1`): start/end **เท่าเดิมทุกตัว** เปลี่ยนแค่สี
→ `FOG_COLOR_NIGHT = Color::srgb(0.05, 0.08, 0.17)`. ระยะเป็นสัญญากับ streaming
จึงห้ามผูกกับเวลากลางวัน/กลางคืน.

---

## 2. สัญญากับ Kevin (chunk streaming) — ข้อผูกพันจริง

```
RENDER_RADIUS = FOG_END = 320.0 บล็อก
```

**invariant ที่ streaming ต้องรักษา:**

> chunk ทุกก้อนที่ศูนย์กลางอยู่ในรัศมี `FOG_END` จากกล้อง **ต้องมี mesh อยู่บนจอ**.

ทำไมถึงเป็น "ต้อง" ไม่ใช่ "ควร": หมอก Linear ทึบเต็ม 100% พอดีที่ `FOG_END`.
chunk ที่หายก่อนถึงระยะนั้นจะโผล่เป็น "ขอบโลก/ช่องว่างเห็นฟ้า" **ในบริเวณที่หมอกยังไม่ทึบพอจะกลบ**
— ซึ่งคือ pop-in ที่ตาจับได้ทันที. ถ้ากลบด้วยหมอกได้พอดี pop-in จะมองไม่เห็นเลย.

**ถ้ารัศมี streaming จริงทำได้ไม่ถึง 320** (เช่นงบเฟรมไม่พอ) — **อย่าปล่อยให้ค่าไม่ตรงกันเงียบๆ**
ให้บอกผมมาแล้วผมขยับ `FOG_END` ลงมาเท่ารัศมีจริง `R` แล้ว `FOG_START` จะไล่ตามสูตรเดียว:

```
FOG_END   = R
FOG_START = 0.35 × R
```

(320 / 112 คือสูตรนี้ที่ R = 320.) แก้ที่ `look.rs` ที่เดียว ทั้งเกมขยับตาม.

**ที่ Kevin ไม่ต้องแคร์:** สีหมอก, สีฟ้า, มุมแดด, exposure — เปลี่ยนได้ตลอดโดยไม่กระทบ streaming.
**ที่ Kevin ต้องแคร์:** `FOG_END` ตัวเดียว.

---

## 3. ท่อ post ของกล้อง gameplay (สรุปว่ามีอะไรบ้าง)

ทั้งหมดอยู่ใน `client/src/look.rs` และถูกใส่ให้กล้องที่ `client/src/main.rs` spawn.

| ชั้น | ค่า | เหตุผลสั้นๆ |
|---|---|---|
| Tonemapping | **`TonyMcMapface`** | ดูข้อ 4 |
| ColorGrading | temp 0.02 · sat 1.05 · midtone contrast 1.12 · highlight gain 0.86 · shadows neutral | ความอุ่นมาจาก "ไฟ" ไม่ใช่ white-balance matrix (บทเรียน magenta 2026-08-01) |
| Exposure | **ev100 = 10.8** | แดด 11,000 lux golden-hour. 11.0 เดิมมืดไป 1.3 stop จาก `Exposure::BLENDER` (9.7) — วัดบนเฟรม vista: patch แดดสว่างสุด L=53.9 ต่ำกว่า floor G6 ที่ 55. 9.7 ดันขึ้น L=69.9 ก็จริงแต่ซีด (R−B 133 จาก 155, G5 spread 56.7→35.8); **10.8** ผ่าน floor ที่ L=57.1 โดยยังเหลือความอุ่น (R−B 151, spread 53.6) |
| Bloom | intensity 0.18 · **prefilter threshold 1.0 / softness 0.4** | ฟุ้งเฉพาะสิ่งที่สว่างเกิน 1.0 ใน HDR = โคมไฟ/ไฟ/emissive/แดดในกระจก เท่านั้น |
| MSAA | **Off** | SSAO บังคับ + ขอบ voxel เป็น 90° ไม่มี jaggy ให้ลบ |
| TAA | on ตั้งแต่ Medium ขึ้นไป | SSAO/เงา temporal เป็น stochastic ต้องมีตัวสะสม |
| ShadowFilteringMethod | Temporal (Gaussian ที่ tier Low) | ขอบเงานุ่ม ≥3px ตาม gate G4 |
| SSAO | Low→Ultra ตาม tier · thickness 1.45 | ของสัมผัสพื้น ไม่ลอย |
| VolumetricFog | High/Ultra (step 32 / 96) | ลำแสงลอดหน้าต่าง/ยอดไม้ |
| **DepthOfField** | **ไม่มี — ถูกถอดออกถาวร** | ดูข้อ 5 |

---

## 4. ทำไมเปลี่ยน tonemapper เป็น TonyMcMapface (จาก AcesFitted)

ไม่ใช่เรื่องรสนิยม — เป็นเรื่องที่ Bevy เขียนไว้เองในซอร์ส (`bevy_core_pipeline::tonemapping`):

> `AcesFitted` — *"Bright greens and reds turn orange. **Bright blues turn magenta.**"*

**ฟ้าที่สว่างที่สุดในเฟรมของเราคือ "ท้องฟ้า"** และ magenta cast คือบั๊กที่เพิ่งถูกไล่ไปเมื่อ
2026-08-01 (commit 26b2ae6) — การเก็บ ACES ไว้บนกล้อง gameplay คือการเลี้ยงสาเหตุเดิมไว้
อีกทางหนึ่ง. `TonyMcMapface` (default ของ Bevy เอง) เป็น *"very neutral… color hues are
preserved during compression"* → ฟ้าอยู่ฟ้า, ใบไม้อยู่เขียว, แดดอยู่ส้ม, ไม่มีตัวไหนไหลไปหากัน.
และ Bevy ระบุตรงๆ ในเอกสารของ `Bloom` ว่าให้ใช้คู่กับ `TonyMcMapface` โดยเฉพาะ.

ผลข้างเคียงที่ต้องชดเชย: Tony ไม่เพิ่ม contrast/saturation ให้ฟรีแบบ ACES → grade เดิม
(sat 1.02 / midtone 1.30 / hi-gain 0.64) ถูกจูนไว้ "แก้ของที่ ACES ทำเสีย" จึงแรงเกินไปกับ Tony.
ค่าใหม่ในข้อ 3 คือค่าที่จูนกับ Tony.

**hero.rs ไม่ถูกแตะ** — golden beauty shot ยังเป็น AcesFitted ตามที่เซ็นรับไว้.
สองไฟล์นี้คนละฉาก คนละสัญญา และ hero.rs ไม่ได้ป้อนค่าให้ streaming.

---

## 5. DepthOfField — ถอดออกถาวรจากกล้อง gameplay

CEO ตำหนิว่า "ไกลๆ เบลอละลาย". ต้นเหตุคือ DOF ที่ `hero.rs:803` ซึ่ง look.rs ยกมาใส่ tier
High/Ultra พร้อมระบบ `focus_dof` ที่ **ล็อกโฟกัสไว้ที่ระยะบูมกล้อง (~6.5 บล็อก)**.
ผลคือทุกอย่างที่ไกลกว่าตัวละครไม่กี่บล็อกจะเบลอทันที — ตรงข้ามกับภาพอ้างอิงที่
**ต้นสนหลังบ้าน/ยอดเขาไกลคมทุกพิกเซล**.

DOF โฟกัสใกล้เป็นภาษาของ "ภาพนิ่งโชว์ของ" ไม่ใช่ของ "กล้องที่คนเล่นมองโลกผ่าน".
จึงถอด `DepthOfField` ออกจากทุก tier และลบระบบ `focus_dof` ทิ้ง.
`DepthOfField` **ยังอยู่ในลิสต์ `LookStack`** เพื่อให้การสลับ tier ยัง "ถอด" DOF เก่าออกได้
ถ้ามีใครเผลอใส่มา — คือ strip แต่ไม่เคย insert.

> ⚠️ ผลข้างเคียงที่ยอมรับ: `look-acceptance-rubric.md` Pass 5 (DOF, 8 คะแนน) และแกน
> `DOF fg:bg ≥ 3.0` ใน `grade_axes.py` **จะตกโดยตั้งใจ** สำหรับเฟรม gameplay ทุกใบ
> ตั้งแต่นี้ไป — เหมือนที่ `wide-hero-final.png` (baseline ที่ CEO อนุมัติ) ตกอยู่แล้วที่ 0.17.
> นี่คือ trade-off ที่ตัดสินแล้ว ไม่ใช่ regression. เกรดเฟรม gameplay ให้ mark Pass 5 = N/A
> แล้วปรับฐานเป็น 92 ตามหัวข้อ "การปรับฐาน Lite" ของ rubric.

---

## 6. ดวงอาทิตย์ + ฟ้า + fill (ลานลุคเป็นคนตัดสิน "ชั่วโมงของวัน")

| อะไร | Golden Hour (default) | Night (`VOXELFORGE_LOOK_NIGHT=1`) |
|---|---|---|
| elevation | **17°** (แดดเฉียง) | −8° (ลับขอบฟ้า) |
| azimuth | 205° | 205° |
| illuminance | 11,000 lux | 260 lux (แสงจันทร์) |
| สีแดด | `srgb(1.00, 0.84, 0.62)` | `srgb(0.55, 0.66, 0.95)` |
| ClearColor (ฟ้า) | hue `srgb(0.36, 0.60, 0.90)` × **sky_gain 2.4** (linear) | hue `srgb(0.03, 0.05, 0.12)` × **sky_gain 1.0** |
| AmbientLight สี | `srgb(0.96, 0.84, 0.66)` | `srgb(0.42, 0.52, 0.78)` |
| AmbientLight brightness | 1100 lux | 90 lux |
| Exposure ev100 | 10.8 | 7.5 |

`illuminance` / มุมแดด / ClearColor เดิมเป็นของ `main.rs` (9000 lux, ดวงอาทิตย์สูง 59°,
ฟ้าซีด 0.53/0.72/0.92). ตอนนี้ **ลานลุคเป็นคนเซ็ต** เพราะ "แดดเฉียง golden-hour" คือ
เนื้อของโจทย์ ไม่ใช่ค่าเสริม — และมันต้องขยับพร้อมกันทั้งชุด (แดดต่ำ + ฟ้าเข้ม + fill อุ่น +
exposure) ไม่งั้นได้ภาพที่ขัดกันเอง. ค่าพวกนี้ถูกเซ็ต **เฉพาะตอน look เปิด** (`--play`)
เท่านั้น — bench / editor / hero shot ยังเห็นค่าที่ main.rs ตั้งไว้เดิมทุกตัว.

---

## 7. env hooks (สำหรับ sweep โดยไม่ต้อง recompile)

| env | ทำอะไร |
|---|---|
| `VOXELFORGE_LOOK_QUALITY=low\|medium\|high\|ultra` | เลือก tier ตอนบูต (F7 สลับสดตอนเล่น) |
| `VOXELFORGE_LOOK_NIGHT=1` | สลับไป Night preset ทั้งชุด |
| `VOXELFORGE_LOOK_EXPOSURE=<ev100>` | ทับ exposure |
| `VOXELFORGE_LOOK_SUN=elev,azim,illum` | ทับมุม/ความแรงแดด |
| `VOXELFORGE_LOOK_FOG=start,end` | ทับระยะหมอก (**sweep เท่านั้น — ค่าที่ชิปคือ const**) |
| `VOXELFORGE_LOOK_GRADE=temp,sat,mid,hi_gain` | ทับ grade |
| `VOXELFORGE_LOOK_LIGHT=kr,kg,kb,ar,ag,ab` | ทับสีแดด/สี fill |
| `VOXELFORGE_LOOK_DISABLE=1` | ปิดลานลุคทั้งลาน (ใช้ทำ before/after ของ Gate 3) |
| `VOXELFORGE_LOOK_FORCE=1` | เปิดลุคแม้ไม่ได้ `--play` (ใช้ถ่ายมุมจาก map ที่โหลด) |
| `VOXELFORGE_LOOK_CAM=yaw_deg,pitch_deg,dist` | จัดมุมกล้องตอน spawn (ถ่ายรูปพิสูจน์งาน) |

ทุกตัว "ไม่ตั้ง = ค่าที่ชิป byte-for-byte" — ไม่มี env ตัวไหนจำเป็นต่อการเล่นปกติ.

---

_Flamingo (Look Lead) — 2026-08-02. แก้ตัวเลขที่ `client/src/look.rs` แล้ว sync สำเนามาที่นี่ เสมอ._
