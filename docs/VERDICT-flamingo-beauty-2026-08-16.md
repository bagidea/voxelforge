# VERDICT — golden beauty shot vs current source (2026-08-16 · Flamingo / pixel lane)

> **สรุปหนึ่งบรรทัด: เฟรมใหม่ยิงได้แล้ว, ไม่มี regression เทียบ baseline ที่ CEO อนุมัติ (ตรงกันทุกแกนถึงทศนิยมที่สอง) — แต่ห่างจาก golden ref มาก และค่า default ที่ baked ในโค้ดเรนเดอร์ออกมาเป็นภาพแดงตันใช้ไม่ได้.**
>
> เกรดจากพิกเซลจริง ไม่ใช่ความจำ. ทุกคำสั่ง + stdout + exit code อยู่บนดิสก์ที่
> `_fl_beauty_20260816/logs/` (17 ไฟล์) — รันซ้ำได้ด้วย `bash scripts/_pixel_beauty_chain.sh`.

---

## 0. Provenance — เฟรมนี้มาจากบิลด์ไหน

`_fl_beauty_20260816/logs/11-provenance.log`

| | |
|---|---|
| exe | `target-pixel/release/voxelforge_shot.exe` · 71,400,448 b · mtime **2026-08-16 07:53:47 +07** |
| sha256 | `38d03bceb310fddc459ad8c060150532246145d4cb1b75d6b11cd4eb763caff5` |
| git | `4242f21` · branch `poppy/native-only` · working tree dirty 131 paths |
| ⚠️ source ที่ใหม่กว่า exe | **6 ไฟล์** (`anim.rs` `vfx_bridge.rs` `enemy_ai_proof_main.rs` `main.rs` `vfx.rs` `audio.rs`) — เป็นการแก้ของเลนอื่นที่ **ไม่ได้อยู่ในไบนารีตัวนี้**. ไม่มีไฟล์ของเลน look/hero อยู่ในลิสต์ ⇒ ไม่กระทบคำตัดสินด้านล่าง |

| plate | ไฟล์ | md5 | ที่มา |
|---|---|---|---|
| **A · beauty-wide** | `_fl_beauty_20260816/beauty-wide-nohud2.png` 1280×720 | `f56da73833ed…` | recipe verbatim จาก `render_wide_hero.sh` (framing ที่ CEO อนุมัติ) |
| **B · beauty-tight** | `_fl_beauty_20260816/beauty-tight-nohud2.png` 1280×720 | `d8b1dca1d91d…` | **ไม่ใส่ env เลย** = ค่า default ที่ baked ใน `hero.rs` |

> ยิงลง scratch dir ตามวันที่ **ไม่แตะ** `docs/assets/wide-hero-final.png` (มันคือ parity baseline ของทุกเลน).
> md5 ของทั้งสองใบ**ไม่คงที่ข้าม shoot** (ยิงซ้ำได้ค่าใหม่) แต่ตัวเลขที่เกรด**นิ่ง** — เทียบ run-1 กับ run-2
> ได้ 125.90/5.50/2.47/93.87/0.17/5.90/177.36 เท่ากันทุกหลัก ⇒ noise อยู่ในระดับที่ไม่ขยับคำตัดสิน.

---

## 1. Controls — รันแล้ว บันทึกแล้ว ทุกใบมี exit code

รอบก่อนตัวเลขอยู่ในแชทอย่างเดียว รอบนี้อยู่บนดิสก์ทั้งหมด.

| # | control | คำสั่ง | ผล | log |
|---|---|---|---|---|
| 30 | **positive · axes** | `grade_axes.py` บน golden ref | **ALL AXES PASS · exit 0** (clip 18.79 · DOF 3.50 · micro 5.24 · p95 165.83) | `30-ctl-axes-ref.log` |
| 31 | **positive · gate** | `grade_gate.py` บน golden ref | G3=P G5=P G6=P · **exit 0** | `31-ctl-gate-ref.log` |
| 32 | **positive · look** | `grade_look.py` บน golden ref | 5/5 measured PASS · **penumbra 8px** · exit 0 | `32-ctl-look-ref.log` |
| 33 | **positive · rubric probe** | `_pixel_rubric_probe.py` บน golden ref | accent 1.11% · median hue 63.9° (moss) · exit 0 | `33-ctl-rubric-ref.log` |
| 34 | **known-shape · axes** | `grade_axes.py` บน wide baseline | exit 1 — **ตกเฉพาะ DOF 0.18** ตามที่ `golden-beauty-shot.md` จดไว้ว่าเป็น intrinsic ของ wide | `34-ctl-axes-base.log` |
| 35 | **known-shape · look** | `grade_look.py` บน wide baseline | exit 1 — **ตกเฉพาะ G4a 4px** | `35-ctl-look-base.log` |
| 36 | **known-shape · rubric** | `_pixel_rubric_probe.py` บน wide baseline | accent 2.87% · exit 0 | `36-ctl-rubric-base.log` |

**ขอบเขตของ control ชุด resample (แก้ตามรีวิว):** เพลต 720p/640p ที่ผมยิงไว้รอบก่อน
**คุมได้เฉพาะแกน penumbra เท่านั้น** — ทั้งคู่ **ตก G6** ⇒ ห้ามยกไปอ้างว่าเป็น "control ผ่าน" แบบกว้าง.
สิ่งที่มันพิสูจน์คือ: ลดความละเอียด ref ลง penumbra **ไม่ลด** (8px→8px→9px) ⇒ 4px ของเพลต wide
ไม่ได้เกิดจากความละเอียด แต่เป็นเงาที่คมจริง.

**ข้อผิดรอบก่อนที่ผมแก้แล้ว:** ผมเคยสรุปว่า "`grade_look.py` ตั้งเส้น ≥5px เกิน ให้ยึด ≥3px ของรูบริค"
— **ผิด**. `grade_look.py:159-163` calibrate เส้นนี้กับสองจุดอ้างอิงจริง (ref=8px นุ่ม / hard-PCF เก่า=3px
เพราะ AA พองขอบ 1px) ส่วน `≥3px` ในรูบริคเป็นเกณฑ์ **ตาเปล่า zoom 400%** คนละเครื่องมือ.
ยึด ≥3px กับ median ของเครื่อง = ปล่อยเฟรม hard-shadow ผ่าน = ลดความเข้มของด่าน ซึ่ง `LANES.md` ห้าม.
รูบริค (ไฟล์ในเลนผมเอง) แก้แล้ว → กล่อง 📏 ใน `docs/look-acceptance-rubric.md` ชั้น A + แถว 4a.

---

## 2. Plate A · beauty-wide — ตารางคะแนนต่อข้อ

### ชั้น A — GATE (ตก 1 = FAIL)

| G# | ด่าน | ชนิด | ค่าที่วัดได้ | ผล |
|---|---|---|---|---|
| **G1** | voxel hard-edge geometry | 👁 visual | ทุกวัตถุหลักเป็นลูกบาศก์ขอบ 90° ไม่มี bevel/round | **PASS** |
| **G2** | key light มีทิศชัด **+ แถบแสงกรอบหน้าต่างทาบพื้น/ผนัง** | 📏+👁 | เครื่อง: top5% ΔL **48.0** (need ≥18) → ผ่านครึ่งทิศ · **แต่ beam x-centroid bias 0.05** (ref 0.67) = ลำแสงอยู่กลางจอ ไม่ราก · **ไม่มีแถบ mullion ทาบพื้น/ผนังเลยสักเส้น** | **FAIL** (clause แถบแสง) |
| **G3** | ร่มไม่ดำ ไม่ฟ้า | 📏 | interior p05-L **14.3%** (≥8) · darkest (45,22,7) R−B +38 warm | **PASS** |
| **G4** | soft shadow + contact AO | 📏+👁 | G4a median **4px** (เส้นเครื่อง ≥5 @1280w · ref 8px) · 👁 บล็อกมิ้นต์/ชามไม่มีเงาสัมผัส "นั่งลอย" | **FAIL** |
| **G5** | tone-map ไม่ blow-out | 📏 | สว่างสุด (246,205,124) min(G,B)=124 ≤245 · 3-pt spread 43.8 · near-clip px **0.00%** | **PASS** |
| **G6** | warm golden tone | 📏 | patch (232,211,181) R−B **+51** (band 40–210) L 83.7 ≥55 | **PASS** (แต่ +51 อยู่ก้นแบนด์ · ref +133/+184) |

**GATE ผล: ❌ FAIL — ตก G2, G4**

### ชั้น B — AAA SCORE

> รูบริคบอกว่าเฟรมที่ตก gate **ไม่ได้เกรด**. ตารางนี้จึงเป็น **diagnostic ว่าจะจ่ายงานตรงไหน** ไม่ใช่คะแนนที่ใช้ตัดสิน.

| Pass | เต็ม | ได้ | เหตุผลจากค่าที่วัด |
|---|---|---|---|
| 1 Key light | 12 | **0** | 1a (8) แถบ mullion ทาบพื้น = **ไม่มี** → ตกข้อหลัก ⇒ 0 ทั้ง pass · (1b: patch แดด R=232 ต่ำกว่าแบนด์ 235–255 อยู่นิด) |
| 2 Bounce/GI ⭐ | 18 | **10** | 2a ✅ open shade L **15.67%** (band 12–35) R−B +66.5 · 2b color bleed ❌ ไม่เห็นสีวัตถุเลียผิวข้างเคียงเลย · 2c ❌ พื้นมีแถบสว่าง/มืดขอบตัดแข็ง |
| 3 God rays | 8 | **0** | 3a ❌ ไม่เห็นแท่งแสงในอากาศ · 3b ✅ dust motes เห็นชัด **แต่ ladder ให้ 0 เมื่อตกข้อหลัก** |
| 4 Shadow+AO ⭐ | 16 | **0** | 4a ❌ 4px < 5 ⇒ ตกข้อหลัก ⇒ 0 ทั้ง pass |
| 5 DOF | 8 | **N/A** | 0.17 = deep-focus โดยดีไซน์ของ wide (`VOXELFORGE_DOF=8,10` เพื่อให้พื้นคมตาม G1) — จดเป็น tradeoff ที่รับแล้วใน `golden-beauty-shot.md` |
| 6 Tone-map ⭐ | 12 | **12** | 6a ✅ window ladder 86.3/13.7/12.6 spread 73.7 สว่างสุด 240 ≤252 · 6b ✅ near-clip 0.00%, pure-black 0.00% |
| 7 PBR material | 10 | **0** | 7a ❌ ทุกผิวเป็นสีแบนล้วน — ไม้ไม่มี grain, ตู้ไม่มี specular streak, เซรามิกกับไม้แยกไม่ออก |
| 8 Bloom | 6 | **4** | 8a ✅ ฟุ้งนุ่มเฉพาะหน้าต่าง/motes · 8b ❌ shadow floor p01/p05 = **12.30/14.56%** เทียบ ref **7.21/8.57%** ⇒ เงาถูกดันสว่าง |
| 9 Palette lock | 6 | **6** | 9a ✅ warm share 97.87% · 9b ✅ accent 2.87% ≤15% และ non-zero |
| Gr Geometry read | 4 | **4** | ✅ อ่านออกว่าเป็นเกม voxel ชัดเจน |

**รวม 36 / ฐาน 92 (Pass 5 = N/A) = 39% → ต่ำกว่าเกรด B**
คะแนนที่หายไปกระจุกอยู่ที่ **แสง–เงา–วัสดุ** ทั้งหมด ไม่ใช่ที่สี/โทน (6, 9, Gr เต็ม).

### ⚖️ ไม่มี regression

`beauty-wide` (ยิงใหม่จาก source วันนี้) เทียบ `wide-hero-final.png` (CEO อนุมัติ):

| | warmth | blue | clip | sat | DOF | micro | p95 | G4a |
|---|---|---|---|---|---|---|---|---|
| baseline (approved) | 125.89 | 5.50 | 2.47 | 93.87 | 0.18 | 5.89 | 177.36 | 4px |
| **beauty-wide (new)** | **125.90** | **5.50** | **2.47** | **93.87** | **0.17** | **5.90** | **177.36** | **4px** |

⇒ source ปัจจุบัน **เรนเดอร์ลุคที่อนุมัติไว้ได้เหมือนเดิมทุกหลัก**. ทุกช่องว่างในตารางข้างบนเป็น
**หนี้เก่าที่มีมาตั้งแต่ baseline** ไม่ใช่ของที่เพิ่งพัง.

---

## 3. Plate B · beauty-tight — ค่า default ที่ baked ในโค้ด **ใช้ไม่ได้**

ยิงโดยไม่ใส่ env เลยสักตัว = "ต้นไม้เรนเดอร์ลุคที่อนุมัติออกมาเองได้ไหม".

| แกน | ค่า | ผล |
|---|---|---|
| mid clip % | **44.36** | ❌ FAIL (เพดาน 35 · ref 18.79) |
| mean R/G/B ทั้งเฟรม | **147.9 / 39.6 / 9.4** | เขียวถูกบด ฟ้าเกือบศูนย์ |
| cool accent share | **0.00% (0 px)** | ❌ บล็อก moss หายไปทั้งก้อน — ถูกกลืนเป็นแดง |
| saturation | **N/A** | grader ปฏิเสธวัดเอง: "midtone clipped 44.4% — honest sat over <55.6% survivors is noise" |
| G4a penumbra | **0px** — "no shadow edges detected" | ❌ (นี่คือ "หาไม่เจอ" ไม่ใช่ "วัดแล้วแคบ" — ต่างกัน) |
| G3/G5/G6 | machine **PASS ทั้งสามด่าน** | 🚨 **PASS ปลอม** |

> 🚨 **นี่คือกับดัก "gamut clip fakes colour gates" เต็มรูปแบบ.** G6 ถาม "R>G>B ไหม" — เฟรมที่ถูกบด
> เป็นแดงตันตอบ "ใช่" ได้สบาย. G3 ถาม "ร่มอุ่นไหม" — จุดมืดสุดคือ **(79,0,0)** คือ G กับ B ตันที่ศูนย์
> ซึ่งอ่านว่า "อุ่นมาก R−B +79". **ทั้งสามด่านเขียวบนภาพที่พังชัดด้วยตา** ⇒ ห้ามรายงาน gate เพลตนี้ว่า PASS.
> ตัวเดียวที่จับได้คือ `clip ≤35` (44.36 ตก) กับ `9b accent non-zero` (0 px ตก) ตรงตามที่รูบริคออกแบบไว้.

---

## 4. 🎯 3 จุดที่ยังไม่สวยที่สุด + เจ้าของเลน

เรียงตามน้ำหนักในรูบริค (เจ้าของอ้างอิง `docs/LANES.md`).

### ① ไม่มีแถบแสงหน้าต่างทาบพื้น และเงาทอดไม่นุ่ม — **28 คะแนน + ตก 2 gate**
**เจ้าของ: rose** → `client/src/look.rs` (`LookPlugin` — PCSS/shadow post stack)

| หลักฐาน | ค่า | ref |
|---|---|---|
| G4a penumbra median | **4px** | 8px |
| beam x-centroid bias | **0.05** (กลางจอ) | 0.67 (รากจากข้าง) |
| แถบ mullion บนพื้น/ผนัง | **0 เส้น** | เห็นชัดทั้งผนัง |
| grooves (AO indicator) | 1,583 · dip 68.7 | 3,205 · dip 78.2 |

หน้าต่างในฉาก **มี mullion เป็นกากบาทอยู่แล้ว** (เห็นในเฟรม) แต่แสงที่ลอดผ่านไม่ได้พารูปทรงนั้น
ลงไปบนพื้น ⇒ ปัญหาอยู่ที่ shadow/light transport ไม่ใช่ที่ geometry. นี่คือรายการเดียวที่ปลดล็อก
ทั้ง Pass 1 (12) + Pass 4 (16) และปิด gate G2/G4 พร้อมกัน — **จ่ายก่อนข้ออื่นเสมอ**.
มุมแดดของช็อต (`VOXELFORGE_SUN=19,196,26000`) อยู่ใน recipe ของผม (pixel) — ถ้า rose ต้องการ
มุมอื่นเพื่อให้ราก บอกมาได้เลย ผมขยับให้ในเลนผม.

### ② ทุกผิวเป็นสีแบนล้วน ไม่มีวัสดุ — **15 คะแนน** (Pass 7 = 10 · Pass 2b = 5)
**เจ้าของ: poppy** → `client/src/voxel.rs` (voxel core / material lane, ยืมจาก shiba)

ref แยกไม้/สแตนเลส/เซรามิกออกจากกันได้ด้วยตาจาก specular response. เฟรมใหม่แยกไม่ออกเลย —
ไม้ไม่มี grain, ตู้ไม่มีแถบสะท้อน, และไม่มี color bleed ระหว่างผิวที่ติดกัน.
`hi-pass std 6.12` ผ่านเกณฑ์ micro-contrast จริง **แต่มาจากขอบบล็อก ไม่ใช่พื้นผิว** — เลขนี้จึง
ไม่ขัดกับข้อสรุปนี้. ⚠️ หมายเหตุจาก memory ของเลนนี้: Bevy **ทิ้ง normal map เงียบๆ ถ้า mesh
ไม่มี TANGENT** — เป็นจุดแรกที่ควรเช็คก่อนไปไล่ material asset.

### ③ ค่า default ที่ baked เรนเดอร์เป็นภาพแดงตัน — **ไม่ใช่คะแนน แต่เป็น ship-blocker**
**เจ้าของ: pixel (ผมเอง)** → `client/src/hero.rs`

ลุคที่ CEO อนุมัติมีอยู่ **เฉพาะหลัง env recipe 12 ตัวใน `render_wide_hero.sh`** เท่านั้น.
ใครก็ตามที่ build แล้วรัน `voxelforge_shot` ตรงๆ จะได้ plate B (clip 44%, moss หาย, เงาไม่มี).
ผมรับผิดชอบข้อนี้เอง — งานคือ bake exposure/grade/ambient ที่พิสูจน์แล้วเข้า `hero.rs` ให้ค่า
default ออกมาเท่ากับ recipe แล้ว **พิสูจน์ด้วยการยิงแบบไม่ใส่ env เลยแล้วได้เลขชุดเดียวกัน**
(ห้ามพิสูจน์ด้วย env — นั่นคือการฟอก regression ให้ผ่าน).

---

## 5. รันซ้ำเองได้

```bash
cd "E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge"
bash scripts/_pixel_beauty_chain.sh      # ยิง 2 เพลต + เกรดครบ + control ครบ + เขียน log ทุกใบ
```

ผลลัพธ์: `_fl_beauty_20260816/` — เฟรม, `sheet-ref-vs-new.png`, และ `logs/*.log` (แต่ละไฟล์มี
คำสั่ง + cwd + เวลา + stdout/stderr + `--- exit=N ---`).
exit code ที่ควรได้: control 30–33 = **0** · 34/35 = **1** (ตกตามที่จดไว้) · plate 20/22 = **1**.
`exit 2` ที่ไหนก็ตาม = **grader ปฏิเสธที่จะวัด** (nohud2 guard) ไม่ใช่ FAIL — อย่านับเป็นคำตัดสิน.

---

_Flamingo (Designer · pixel lane) — 2026-08-16_
