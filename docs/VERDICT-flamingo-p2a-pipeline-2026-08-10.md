# VERDICT — C10 = 66.20 (A6 hero) เชื่อได้หรือไม่?
Flamingo (Designer) · 2026-08-10 · P2a pipeline audit
สถานะ: ✅ CLOSED (rev.2 — แก้หลังรีวิว 5 ข้อ; ส่วน rev.1 ด้านล่างเก็บไว้เป็นบันทึก แต่ **บทสรุปที่ใช้จริงคือ VERDICT rev.2 ท้ายไฟล์**)

## โจทย์ (จาก Director)
1. เลข 66.20 ที่ `docs/a6-idle-silhouette-spec-2026-08-10.md:19` วัดด้วย pipeline ไหน — raw mask หรือ gaussian-smoothed mask?
2. รัน grader บน golden reference / approved beauty shot เป็น **control** — ถ้า golden เองไม่ผ่าน ≥40 บน pipeline เดียวกัน = **มาตรวัดพัง ไม่ใช่งานศิลป์พัง**
3. เกณฑ์ **52.15** — grep แล้วไม่พบในรีโปเลย (docs/ + scripts/). สเปกจริงเขียน ≥ 40. ต้องยืนยันว่ามีจริงหรือไม่
4. ห้ามรัน cargo build/check (RAM ต่ำ) · ห้ามแก้ client/src/
5. เป้า 15 นาที

## สิ่งที่รู้แล้วตอนเริ่ม (ยังไม่พิสูจน์ — เป็น input ที่ Director ให้มา)
- ตัวเลขคู่ที่ flag ไว้: **53.5 vs 27.4** → **ตอบแล้วใน §C-3 ท้ายไฟล์** (ไม่ใช่ raw-vs-smoothed อย่างที่เดาไว้ตรงนี้ แต่เป็น *คนละเครื่องมือ* บน charmask)
- `scripts/grade_character.py` line 408-409 มีคอมเมนต์ระบุว่าใช้ **gaussian-smoothed mask**
- ถ้า 66.20 มาจาก raw pipeline แต่ threshold/สเปกอ้างอิงจาก smoothed (หรือกลับกัน) → เทียบข้ามหน่วยกัน = เลขไม่มีความหมาย

## LOG (append-only)
- [t0] สร้างไฟล์นี้ก่อนเริ่มอ่านโค้ด

---
## 🔴 FINDING 1 — ที่มาของ "52.15" เจอแล้ว มันไม่ใช่เกณฑ์ในสเปก มันเป็นเกณฑ์ "สัมพัทธ์" ที่ grader คำนวณเอง

`scripts/grade_character.py:128`
```
("iso",    "C10 silhouette complexity", ("relmin", 0.70),  "ref x0.70"),
```
เกณฑ์จริงของ C10 ใน grader = **relmin 0.70 → ต้องได้ ≥ 70% ของค่า reference** ไม่ใช่ตัวเลขคงที่
→ 0.70 × **74.50** (iso ของ concept ref) = **52.15**

ยืนยันจาก `docs/VERDICT-a6-composition-2026-08-10.md:62`:
```
| **boot / idle** | **66.20** (89% of concept's 74.50, PASS ≥52.1) | 17.9 (box=16) | ≥40 |
```
**สรุปข้อ (3): 52.15 "มีจริง" แต่ไม่เคยถูกเขียนเป็น literal ในไฟล์ไหนเลย** — จึง grep ไม่เจอ
มันคือผลคูณ `0.70 × ref` ที่เกิดขึ้นตอน runtime. เลขที่ปรากฏในเอกสารคือ `≥52.1` (ปัดแล้ว) ไม่ใช่ `52.15`
→ ทั้งออฟฟิศ **เลิกอ้าง "เกณฑ์ 52.15" แบบตัวเลขตายตัวได้เลย** มันลอยตาม ref ที่เลือกใช้.
มี **สองเกณฑ์คนละตัว** อยู่พร้อมกัน: สเปก A6 เขียน `≥ 40` (absolute, art-order:360) แต่ grader บังคับ `≥ 0.70×ref`.

## 🔴 FINDING 2 — 66.20 วัดด้วย pipeline "smoothed"
`scripts/grade_character.py:406-410`
```
# C10 silhouette complexity: isoperimetric ratio P^2/A (a circle = 12.57,
# ...
# Measured on a gaussian-smoothed mask: raw single-pixel jaggies on an
# antialiased matte add perimeter that no eye reads as silhouette shape.
ms = ndimage.gaussian_filter(mf, 2.0) > 0.5
```
และ `docs/assets/gate3-a6-2026-08-10/boot-char.json:18` → `"iso": 66.20441912257301`
= ผลผลิตของ grade_character.py ตัวเดียวกัน → **66.20 = SMOOTHED pipeline** (ยืนยันต่อด้วยการรันซ้ำ ดูข้างล่าง)

## ✅ CONTROL RUN (ข้อ 2) — รัน grader บน golden จริง
เครื่องมือ: `scripts/_flamingo_p2a_isoprobe.py` (เขียนใหม่, read-only, `import` mask extractor
ของ `grade_character.py` เอง ไม่ได้เขียนสูตรใหม่ — ตัวเลขจึงมาจาก pipeline ที่ ship จริง)

```
GOLDEN auren-hero-concept   mask_from_ref   px= 119020 scale=0.547
   iso raw      =    50.56
   iso smoothed =    43.57
   iso SHIPPED  =    43.52     <- grade_character.py

A6 boot/idle render         mask_from_bbox  px= 165689 scale=1.082
   iso raw      =    79.22
   iso smoothed =    66.27
   iso SHIPPED  =    66.20     <- ตรงกับ boot-char.json 66.20441 เป๊ะ
```

### ตอบข้อ (1) ชัดเจน: **66.20 = SMOOTHED pipeline** — reproduce ได้เป๊ะ
รันวันนี้ได้ 66.20 จากสูตร `P(smoothed)² / A(raw)` ตรงกับ `boot-char.json` ทุกหลัก
ถ้าเป็น raw ล้วนจะได้ **79.22** ไม่ใช่ 66.20 → ปิดประเด็น. เลข 66.20 ไม่ใช่เลข raw.

## 🔴🔴 FINDING 3 (ตัวใหญ่ที่สุด) — golden reference JSON ที่ใช้ gate **ทุก axis** วัดจากคนละภาพ
`boot-char.json` ใช้ `ref_json: _fl_char/ref-auren-hero.json` เป็นตัวตั้งเกณฑ์
ผมรัน `measure()` ตัวเดิม บนไฟล์ concept ตัวเดิมที่ json นั้นอ้าง
(`docs/assets/characters/auren-hero-concept.png`) วันนี้ — **ไม่ตรงเลยสักแกน**:

| axis | รันวันนี้ | ที่เก็บไว้ใน ref json | delta |
|---|---:|---:|---:|
| warmth | 48.71 | 87.04 | **−38.32** |
| sat | 79.34 | 90.24 | −10.90 |
| p05L | 4.45 | 7.30 | −2.85 |
| darkRB | 8.02 | 21.18 | **−13.16** |
| form | 81.31 | 105.09 | **−23.78** |
| **iso (C10)** | **43.52** | **74.50** | **−30.97** |
| micro | 8.97 | 8.98 | −0.01 |
| mats | 6.00 | 6.00 | 0.00 |

micro/mats ตรง แต่ที่เหลือเพี้ยนหมด → **ref json นี้ไม่ได้วัดจากภาพ concept ที่อยู่บนดิสก์ตอนนี้**
(มี `auren-hero-concept-before-after-2026-08-06-hairfix.png` + โฟลเดอร์
`docs/assets/characters/archive-before-2026-08-06/` = ภาพ concept ถูกแก้/ทับหลังจากวัด ref ไปแล้ว)
`_fl_char/` ไม่ได้อยู่ใน git และ `scripts/grade_character.py` ก็ไม่มี git log →
**ทั้ง grader และ golden ref ไม่มี provenance ตรวจย้อนได้**

## 🔴 FINDING 4 — 66.20 (render) > 43.52 (concept art) = ตัวเลขไม่ได้อยู่มาตราเดียวกัน
โมเดล voxel ตัวจริง "ซับซ้อนกว่า" ภาพ concept art ที่วาดมือถึง **+52%** — เป็นไปไม่ได้ทางศิลป์
สาเหตุ: สอง mask คนละตัวสกัด
- golden → `mask_from_ref()` = gradient matte จากพื้นหลังสตูดิโอเรียบ → ขอบสะอาด
- render → `mask_from_bbox()` + `key_tol=26` = chroma key ในฉากจริง → ขอบรุ่ย/มี speckle
หลักฐานเชิงปริมาณ: การ smooth กินคะแนนไปไม่เท่ากัน
**golden −7.03 (50.56→43.52) แต่ render −13.02 (79.22→66.20)** — render เสียมากกว่าเกือบเท่าตัว
= mask ของ render มี high-frequency noise มากกว่าจริง แม้ smooth แล้วก็ยังเหลือ
→ **iso ของ render กับ iso ของ concept เทียบกันตรงๆ ไม่ได้** และ `relmin 0.70` ที่คูณข้ามสอง mask นี้
คือการเทียบข้ามหน่วย

## 🟠 FINDING 5 — bug จริงในสูตร: perimeter จาก mask ที่ smooth แล้ว หาร area จาก mask ดิบ
`grade_character.py:410-413`
```
ms = ndimage.gaussian_filter(mf, 2.0) > 0.5   # smoothed
per = ndimage.binary_dilation(ms, ...) & ~ms   # P จาก smoothed
P, A = float(per.sum()), float(m.sum())        # A จาก RAW mask (m) <-- ผสมกัน
```
ผลกระทบเล็ก (golden 43.57→43.52, render 66.27→66.20) แต่มันคือ **สูตรที่ผสมสอง mask** ควรเป็น `ms.sum()`

## 🎯 FINDING 6 — เจอตัวจริงแล้ว: ref json วัดจากภาพที่ถูก **archive ทิ้งไปแล้ว**
รัน `measure()` บนทุกภาพ concept auren ในโฟลเดอร์ archive:

| ไฟล์ | iso | warmth | form |
|---|---:|---:|---:|
| `characters/auren-hero-concept.png` (ตัวที่ ref json อ้าง, ปัจจุบัน) | 43.52 | 48.71 | 81.31 |
| `archive-before-2026-08-06/auren-hero-concept-BEFORE.png` | 65.67 | 69.60 | 104.25 |
| **`archive-before-2026-08-06/auren-hero-concept-PASS1-flathair-2026-08-06.png`** | **74.50** | **87.04** | **105.09** |
| ค่าที่เก็บใน `_fl_char/ref-auren-hero.json` | **74.50** | **87.04** | **105.09** |

ตรงเป๊ะทุกหลัก → **golden reference ที่ใช้ตัดสิน A6 ทุกแกน คือภาพ concept เวอร์ชัน PASS1 (flathair)
ที่ถูกย้ายเข้า archive ไปแล้วตั้งแต่ 2026-08-06** ไม่ใช่ concept ที่ทีมอนุมัติใช้อยู่ตอนนี้
`ref_json` ชี้ไปที่ `docs/assets/characters/auren-hero-concept.png` ในเมทาดาทา แต่ตัวเลขข้างในไม่ใช่ของไฟล์นั้น

---
# 🏁 VERDICT (rev.1 — ⚠️ SUPERSEDED โดย rev.2 ท้ายไฟล์ ข้อ (2)(3)(4) มีที่ผิด)

## (1) 66.20 วัดด้วย pipeline ไหน → **SMOOTHED**
`grade_character.py:410-413`, `ms = gaussian_filter(mf, 2.0) > 0.5`
reproduce วันนี้ได้ **66.20** ตรงกับ `boot-char.json` (66.20441912257301) ทุกหลัก
ถ้าเป็น raw mask จะได้ **79.22**. → เลข 66.20 ถูกคำนวณถูกต้องตามสูตรของมันเอง **ไม่ใช่เลขปลอม**

## (2) golden ได้กี่คะแนนบน pipeline เดียวกัน → **43.52 (PASS ≥40 แต่เฉียดฉิว +8.8%)**
มาตรวัด **ไม่พัง** ในความหมายที่ว่า golden ยังผ่าน ≥40 ได้ — เลยประกาศ "grader พัง" ไม่ได้
**แต่ที่พังคือ "ตัวตั้งเกณฑ์" ไม่ใช่ตัวสูตร**: ref json ที่ใช้ gate ทุกแกนของ A6
มาจากภาพ concept ที่ถูก archive ไปแล้ว (74.50) ส่วน concept จริงวันนี้ = 43.52

## (3) เกณฑ์ 52.15 → **ไม่มีอยู่จริงในฐานะตัวเลขเกณฑ์ ให้เลิกอ้างทั้งออฟฟิศ**
- ~~ไม่มี literal `52.15` ในไฟล์ไหนเลย~~ ⛔ **ผิด — ดู §C-4**: มีอยู่จริงบนดิสก์ 3 ไฟล์ (`"bound": 52.148206281819256`)
- มันคือ `0.70 × 74.50` ที่ grader คูณตอน runtime จาก rule `("relmin", 0.70)` (line 128)
- **74.50 คือค่าของภาพที่ archive ไปแล้ว** → 52.15 จึงเป็นเกณฑ์ที่ derive จาก reference ที่ตายแล้ว
- ถ้าใช้ concept ปัจจุบัน เกณฑ์เดียวกันจะกลายเป็น `0.70 × 43.52` = **30.47**
- เกณฑ์ที่เขียนไว้ในสเปกจริง (`art-order-2026-08-09-composition.md:360`) คือ **≥ 40** absolute — คนละตัวกัน
→ **มีสองเกณฑ์วิ่งขนานกันโดยไม่มีใครประกาศว่าอันไหนคือของจริง**

## (4) A6 รูปทรง ผ่านจริงหรือต้องวัดใหม่ → **ผ่านเกณฑ์ ≥40 จริง แต่ประโยคเคลมต้องถอน**
✅ **ผ่าน**: 66.20 ≥ 40 บน pipeline ของตัวเอง และ golden control ก็ผ่านบาร์เดียวกัน
→ ไม่ใช่เคส "มาตรวัดพัง" ที่ต้องล้มผล และ **ไม่ต้องสั่ง Yamamoto แก้รูปทรงใหม่**

❌ **สิ่งที่ต้องถอน** — 3 ประโยคใน `docs/VERDICT-a6-composition-2026-08-10.md:62`
~~และ `docs/a6-idle-silhouette-spec-2026-08-10.md:19,78,87`~~ ⛔ **line ref ผิด — ตำแหน่งจริงอยู่ใน §C-5**:
1. **"89% of concept's 74.50"** — 74.50 เป็นของภาพ archive. เทียบกับ concept ปัจจุบัน (43.52)
   render จะกลายเป็น **152% ของ concept** ซึ่งแปลว่าตัวเลขไม่ได้อยู่มาตราเดียวกัน ไม่ใช่ว่างานดีเกิน
2. **"PASS ≥52.1"** — เกณฑ์นี้ derive จาก reference ที่ตายแล้ว
3. **"clears the bar by 65%"** — margin นี้พองจากการที่ render ใช้ `mask_from_bbox` (ขอบรุ่ย)
   ส่วน concept ใช้ `mask_from_ref` (ขอบสะอาด). พิสูจน์: smoothing กิน render −13.02
   แต่กิน golden แค่ −7.03 → mask ของ render มี noise มากกว่าเกือบเท่าตัว
   **ตัวเลข 66.20 จึงบอกได้แค่ว่า "ไม่ใช่กล่อง" (box≈16-20) บอกไม่ได้ว่า "ดีกว่า concept"**

## สิ่งที่ต้องทำต่อ (ไม่ได้ทำในรอบนี้ — นอกขอบเขตที่ Director ล็อกไว้)
1. รีเจน `_fl_char/ref-auren-hero.json` จาก `docs/assets/characters/auren-hero-concept.png` ตัวปัจจุบัน
   (`grade_character.py --ref <sheet>.png --json <out>.json`) แล้ว re-grade A6 ทั้งชุด
2. ตัดสินให้ขาดว่า C10 ใช้เกณฑ์ absolute `≥40` หรือ relative `0.70×ref` — เลือกอันเดียว
3. แก้ line 413 `A = float(m.sum())` → `ms.sum()` (mixed-mask)
4. เอา `_fl_char/` + `scripts/grade_character.py` เข้า git — ตอนนี้ทั้งคู่ไม่มี provenance

**หลักฐาน/สคริปต์**: `scripts/_flamingo_p2a_isoprobe.py` (read-only, import mask extractor ของ grader เอง)
ไม่ได้แตะ `client/src/` และไม่ได้รัน cargo ใดๆ ตลอดรอบนี้

---
---
# 🔧 CORRECTIONS (rev.2 — หลังรีวิว, 2026-08-10)

รีวิวจับได้ 5 จุด ผมตรวจซ้ำเองทุกจุดแล้ว **รีวิวถูกทั้งหมด** ด้านล่างคือของที่แก้
พร้อมตัวเลขที่ผมรันเองใหม่ ไม่ใช่รับมาจากรีวิว

## ❌ C-1 (ร้ายแรงสุด) — บทสรุปเดิมทำ "cross-quote" ที่รีโปห้ามไว้เป็นตัวใหญ่
บทสรุปเดิมเขียนว่า *"66.20 ≥ 40 บน pipeline ของตัวเอง"* — **ผิด**
**เส้น ≥40 ไม่ใช่ของ C10** มันคาลิเบรตอยู่ใน `scripts/art_order_grade.py` คนละไฟล์ คนละสูตร:

`scripts/art_order_grade.py:306-320` (docstring ของ `shape_complexity`) เขียนไว้ตัวพิมพ์ใหญ่:
```
DO NOT CROSS-QUOTE THIS NUMBER WITH `scripts/grade_character.py` C10. Both are
"isoperimetric P^2/A", but C10 first smooths the mask (gaussian 2.0 > 0.5) ...
on a real antialiased matte they diverge by ~2x (auren concept: 53.5 here, 27.4
under C10). The ratio to a reference measured by the SAME code is stable; the
absolute is not. A6's bar of 40 is calibrated in this pipeline ...
```
คำเตือนเดียวกันเป็นภาษาไทยอยู่ที่ `docs/art-order-2026-08-09-composition.md:237-241`
(เข้ามาใน commit `fce442d` **ก่อน** ไฟล์ VERDICT-a6 ถูกเขียน) — คือมันประกาศไว้ก่อนแล้ว
แต่ทั้งไฟล์ VERDICT-a6 และ rev.1 ของผม **ไม่ได้อ้างถึงเลย** ทั้งที่ FINDING 4 ของผมเอง
ก็คือเรื่อง "ผสมมาตรา" เป๊ะๆ — ผมชี้นิ้วเรื่องนี้แล้วดันทำเองในย่อหน้าถัดไป

ความต่างของสองสูตร:

| | perimeter | mask | ค่าที่ได้บน mask เดียวกัน |
|---|---|---|---|
| `grade_character.py` C10 | outer dilation ring **หลัง** gaussian 2.0 | smoothed | ต่ำกว่า |
| `art_order_grade.py` `shape_complexity` | inner erosion boundary **ดิบ** | ไม่ smooth | สูงกว่า ~1.1–2.9× |

## ✅ C-2 — control run ที่ "ครบ" (เดิมรันแค่ครึ่งเดียว)
เดิมผมรัน golden แค่บน C10 (43.52) ซึ่ง **ไม่ใช่ pipeline ที่เป็นเจ้าของเส้น 40**
รันใหม่ทั้งสองเครื่องมือ **บน mask ชุดเดียวกัน** (`scripts/_flamingo_p2a_isoprobe.py` PART 2):

```
                                   art_order    C10     ratio
GOLDEN concept (current)             49.25     43.52    1.13x
GOLDEN concept (ARCHIVED PASS1)      80.02     74.50    1.07x
A6 boot/idle render                  77.09     66.20    1.16x

bar >=40 (art_order pipeline): golden PASS (49.25) | A6 PASS (77.09)
```
→ **ในไปป์ไลน์ที่เป็นเจ้าของเกณฑ์จริง A6 ได้ 77.09 ผ่าน ≥40 และ golden control ก็ผ่าน (49.25)**
บทสรุป "ผ่าน" รอด **แต่รอบที่แล้วผมไม่เคยพิสูจน์** — ปล่อยให้แขวนอยู่บนการเทียบผิดมาตรา

## ✅ C-3 — ตอบ 53.5 vs 27.4 (โจทย์ข้อที่ Director สั่งจับ แต่ rev.1 ทิ้งไว้เฉยๆ)
**ไม่ใช่ "raw vs smoothed" ของภาพเดียวกันแบบที่ผมเดาไว้บรรทัด 13** — มันคือ
**ภาพเดียวกัน วัดด้วยสองเครื่องมือ** และมาจาก **mask ตัวที่สาม** ที่ผมไม่เคยแตะใน rev.1:
`art_order_grade.py:494-502` ไม่ได้ใช้ `mask_from_ref` แต่อ่าน **`*-concept-charmask.png`** ตรงๆ
ผมรันเองบนไฟล์ charmask ทั้งสามใบ:

| charmask (ชุดที่คาลิเบรตเส้น 40) | art_order | C10 | ratio |
|---|---:|---:|---:|
| `auren-hero-concept-charmask.png` | **53.51** | **27.41** | **1.95×** |
| `elder-maren-concept-charmask.png` | 136.67 | 63.80 | 2.14× |
| `guard-husk-concept-charmask.png` | 298.10 | 104.01 | 2.87× |

**53.51 / 27.41 reproduce ตรงกับ docstring ทุกหลัก** → ปิดประเด็นด้วยการวัดเอง
และนี่คือของสำคัญ: **hero concept ที่อนุมัติแล้ว ได้ C10 = 27.41 ซึ่ง "ตก" ≥40 ยับ**
ถ้าใครเอา C10 ไปเทียบเส้น 40 ตรงๆ → **golden จะสอบตกเอง = มาตรวัดพัง**
เส้น 40 ถูกคาลิเบรตให้ = 0.75 × 53.5 ในไปป์ไลน์ `art_order_grade` เท่านั้น

⚠️ ข้อสังเกตที่ผมต้องพูดตรงๆ: **มี mask 3 เส้นทาง ให้ 3 มาตรา** บน concept ตัวเดียวกัน —
charmask (53.51/27.41) · `mask_from_ref` ปัจจุบัน (49.25/43.52) · `mask_from_ref` archived (80.02/74.50)
และ ratio ระหว่างสองเครื่องมือก็ไม่คงที่ (1.07× → 1.95×) → **แม้แต่ "ต่างกัน ~2 เท่า" ก็ยังขึ้นกับ mask**

## ❌ C-4 — "ไม่มี literal 52.15 ในไฟล์ไหนเลย" **ผมพูดผิด ขอถอน**
มันถูก persist ลงดิสก์ทุกครั้งที่ gate รัน — grep `52\.1` เจอ 3 ไฟล์:
```
docs/assets/gate3-a6-2026-08-10/boot-char.json:126:   "bound": 52.148206281819256,
docs/assets/gate3-a6-2026-08-10/combat-char.json:126: "bound": 52.148206281819256,
docs/assets/gate3-a6-2026-08-10/walk-char.json:126:   "bound": 52.148206281819256,
```
ที่มา `0.70 × 74.50` ยังถูก แต่ประโยค "ไม่มีในรีโป" **ผิด** — grep ไม่เจอเพราะ
เอกสารเขียนแบบปัดเป็น `52.1` ส่วนบนดิสก์เป็น `52.148206281819256` เต็มความละเอียด
**คำแนะนำที่แก้แล้ว:** ไม่ใช่ "เลิกอ้างเพราะไม่มีอยู่จริง" แต่คือ
**"เลิกอ้างเพราะมันเป็น bound ที่ grader คำนวณสดจาก ref ที่ตายแล้ว แล้วเขียนทับลง gate artifact ทุกรอบ"**
→ artifact 3 ไฟล์นี้ต้องถูกสร้างใหม่หลังรีเจน ref ไม่งั้นเลขผิดจะถูกอ่านซ้ำเรื่อยๆ

## ❌ C-5 — line ref ของ "3 ประโยคที่ต้องถอน" ผมใส่ผิด ขอแก้
ของเดิมเขียน `a6-idle-silhouette-spec-2026-08-10.md:19,78,87` ซึ่งผิด (line 19 มีแค่ `66.20 | ≥ 40`)
**ตำแหน่งจริง ตรวจด้วย grep แล้ว:**

| ประโยคที่ต้องถอน/แก้ | ตำแหน่งจริง |
|---|---|
| `89% of concept's 74.50` | `docs/VERDICT-a6-composition-2026-08-10.md:62` (ที่เดียว) |
| `PASS ≥52.1` | `docs/VERDICT-a6-composition-2026-08-10.md:62` (ที่เดียว) |
| `clears the ≥40 bar by 65%` | `docs/VERDICT-a6-composition-2026-08-10.md:67` |
| `clears the bar by 65%` | `docs/a6-idle-silhouette-spec-2026-08-10.md:23` |
| `66.20 ✅ (C10, ดู VERDICT)` วางในคอลัมน์เกณฑ์ ≥40 | `docs/art-order-2026-08-09-composition.md:360` ← **cross-quote ตัวเป็นๆ อยู่ในตารางเกณฑ์เอง** |

---
# 🏁 VERDICT (rev.2 — ฉบับที่ใช้จริง แทนที่ของเดิมทั้งหมด)

## (1) 66.20 วัดด้วย pipeline ไหน → **C10 SMOOTHED** (ไม่เปลี่ยน, reproduce ได้เป๊ะ)
`grade_character.py:410-413` · รันวันนี้ได้ **66.20** ตรงกับ `boot-char.json` (66.20441912257301) ทุกหลัก
raw ล้วนจะได้ 79.22 · `art_order_grade` บน mask เดียวกันได้ 77.09 → **สามเลข สามสูตร ภาพเดียว**

## (2) golden ได้เท่าไหร่บน pipeline เดียวกัน → **ต้องแยกตอบสองไปป์ไลน์**

| | golden (concept ปัจจุบัน) | A6 render | เส้น |
|---|---:|---:|---|
| `art_order_grade` (**เจ้าของเส้น 40**) | **49.25 PASS** | **77.09 PASS** | ≥40 |
| `grade_character` C10 | 43.52 | 66.20 | ≥0.70×ref |
| C10 บน charmask ของ concept ที่อนุมัติ | **27.41** | — | (ถ้าเอาไปเทียบ 40 = golden ตก) |

→ **มาตรวัดไม่พัง ถ้าใช้ให้ถูกไปป์ไลน์** golden ผ่านเส้นของตัวเองสบายๆ (49.25 ≥ 40)
→ **แต่จะพังทันทีที่เอา C10 ไปเทียบเส้น 40** เพราะ golden เองได้ 27.41 = ตก
   **นี่คือกับดักที่ VERDICT-a6 เดินเข้าไป และ rev.1 ของผมก็เดินตาม**

## (3) เกณฑ์ 52.15 → **มีอยู่จริงบนดิสก์ 3 ไฟล์ แต่เป็น bound ที่ derive จาก ref ที่ตายแล้ว**
- `"bound": 52.148206281819256` ใน `{boot,walk,combat}-char.json:126` (ไม่ใช่ "ไม่มี" อย่างที่ผมบอก rev.1)
- = `0.70 × 74.50` จาก rule `("relmin", 0.70)` (`grade_character.py:128`)
- **74.50 = ค่าของ `archive-before-2026-08-06/auren-hero-concept-PASS1-flathair-2026-08-06.png`**
  ซึ่งถูก archive ไปแล้ว (ยืนยันด้วยการวัดซ้ำ ตรงทุกหลัก 74.50/87.04/105.09 — FINDING 6)
- ใช้ concept ปัจจุบัน เกณฑ์เดียวกันจะเป็น `0.70 × 43.52` = **30.47**
- **52.15 ไม่ใช่ และไม่เคยเป็น เกณฑ์ของ A6** เกณฑ์ A6 คือ `≥40` ใน `art_order_grade` เท่านั้น

## (4) A6 รูปทรง ผ่านจริงหรือต้องวัดใหม่ → **ผ่านจริง และตอนนี้พิสูจน์ในไปป์ไลน์ที่ถูกต้องแล้ว**
✅ `art_order_grade.shape_complexity` = **77.09 ≥ 40** และ golden control ในไปป์ไลน์เดียวกัน = 49.25 ผ่าน
→ **ไม่ต้องสั่ง Yamamoto แก้รูปทรง** ผลลัพธ์ "ผ่าน" ยืนได้
→ **แต่ผ่านด้วยเหตุผลคนละอันกับที่ VERDICT-a6 เขียนไว้** — ที่นั่นผ่านเพราะ cross-quote ที่รีโปห้าม
   รอบนี้ผ่านเพราะวัดในไปป์ไลน์ของเส้นจริง

❌ **สิ่งที่ต้องแก้ในเอกสาร** (ตำแหน่งตาม C-5 ข้างบน):
1. `VERDICT-a6:62` — `89% of concept's 74.50` และ `PASS ≥52.1` → 74.50 เป็นของภาพ archive; bound derive จากของตาย
2. `VERDICT-a6:67` + `spec:23` — `clears the bar by 65%` → margin นี้คิดจาก C10 เทียบเส้นของ art_order
   ตัวเลขที่ถูกคือ **77.09 vs 40 = +93%** ในไปป์ไลน์เดียวกัน (บังเอิญดีกว่าเดิม แต่ต้องแก้เพราะที่มาผิด)
3. `art-order:360` — ช่อง "เฟรมใหม่ 08-10 = 66.20 ✅" อยู่ในตารางเกณฑ์ ≥40 = cross-quote
   ต้องเปลี่ยนเป็น **77.09 (art_order_grade)** ให้ตรงกับคำเตือนที่ตัวเองเขียนไว้ที่บรรทัด 237-241

## TODO (ไม่ได้ทำ — นอกขอบเขตที่ Director ล็อก)
1. รีเจน `_fl_char/ref-auren-hero.json` จาก concept ปัจจุบัน แล้ว re-grade + **สร้าง gate artifact 3 ไฟล์ใหม่**
2. ตัดสินให้ขาด: A6 ใช้ `≥40 (art_order)` เป็นเกณฑ์เดียว — C10 เป็น axis ภายในของ character lane เท่านั้น
   ห้ามเอามาตอบ A6 (ตามที่ docstring สั่งไว้แล้ว)
3. แก้ `grade_character.py:413` `A = float(m.sum())` → `ms.sum()` (mixed-mask, ผลเล็ก 0.05–0.07 แต่ผิดหลัก)
4. เอา `_fl_char/` + `scripts/grade_character.py` เข้า git — ตอนนี้ไม่มี provenance

**หลักฐาน**: `scripts/_flamingo_p2a_isoprobe.py` (read-only, import ทั้ง `grade_character.py`
และ `art_order_grade.py` ของจริง ไม่ได้เขียนสูตรเอง) · ไม่แตะ `client/src/` · ไม่รัน cargo
