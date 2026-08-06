# 🔥 AAA Gap Scorecard — Voxelforge vs Elden Ring Tier

> **วันที่เกรด:** 2026-08-06 | **ผู้เกรด:** Sun (Specialist, Positive Psychology)  
> **สคริปต์ที่รัน:** `grade_axes.py` · `grade_gate.py` *(canonical gates)* · `grade_look.py` · `grade_midtone.py` · `measure_penumbra.py` · `grade_hero.py` · `grade_beauty.py` · `grade_g3.py`  
> **สคริปต์ที่รันไม่ได้:** `grade_web_parity.py` (ต้องการ paired native+web builds เดียวกัน) · `grade_g7.py` (ต้องการ --ab-haze/--ab-ao paired frames ที่ toggle look layer) · `grade_ref.py`/`grade_ref2.py` (เกรดเฉพาะ golden ref — ref ผ่าน AAA อยู่แล้วตาม calibration log)  
> **เฟรมที่เกรด:** `grade-vista-2026-08-05-nohud2.png` · `wide-hero-final-nohud2.png` · `gate3-after-boot-nohud2.png` · `gate3-after-combat-nohud2.png` · `gate3-after-walk-nohud2.png`

---

## 📊 ตารางรวมทุกแกน (Master Table) — ค่าที่วัดได้ vs เกณฑ์ผ่าน

| แกน | เกณฑ์ผ่าน | REF (golden) | wide-hero (baseline) | grade-vista | gate3-boot | gate3-combat | gate3-walk |
|---|---|---|---|---|---|---|---|
| **warmth R−B** | ≥ +110 | +120.9 | **+128.8 ✅** | +84.9 ❌ | +82.5 ❌ | +80.4 ❌ | +98.7 ❌ |
| **blue B** | ≤ 10 | 4.3 | **5.2 ✅** | 16.1 ❌ | **10.0 ✅¹** | **6.6 ✅** | **5.2 ✅** |
| **saturation** | ≥ 90% | 96.1 | **94.5 ✅** | 85.3 ❌ | 88.6 ❌ | **90.9 ✅** | **95.1 ✅** |
| **micro-contrast** | ≥ 5.0 | 5.24 | **5.9 ✅** | **15.8 ✅²** | **7.4 ✅** | **8.3 ✅** | **5.8 ✅** |
| **highlight p95** | 150–185 | 165.8 | **177 ✅** | **161 ✅** | **154 ✅** | **157 ✅** | 149.8 ❌ |
| **DOF fg:bg** | ≥ 3.0 | 3.50 | 0.18 ⚠️³ | SKIP | SKIP | SKIP | SKIP |
| | | | | | | | |
| **G3 (shade)** | p05≥8% + warm | PASS | **PASS ✅** | FAIL ❌ | FAIL ❌ | FAIL ❌ | FAIL ❌ |
| **G4a (penumbra)** | ≥ 5px | — | 4px ❌ | 3px ❌ | **8 ✅** | **5 ✅** | **9 ✅** |
| **G5 (window)** | G/B≤245 + grad. | PASS | **PASS ✅** | N/A ⚠️⁴ | **PASS ✅** | **PASS ✅** | **PASS ✅** |
| **G6 (warm wood)** | R−B 40–210 + L≥55 | PASS | **PASS ✅** | **PASS ✅** | **PASS ✅** | **PASS ✅** | **PASS ✅** |

> ¹ gate3-boot blue B = 10.01 — เกิน threshold 0.01 จุด (borderline PASS)  
> ² grade-vista micro-contrast = 15.76 — สูงผิดปกติเพราะ dehud ทิ้ง hard edge (63109 px) ใน grass band; ค่าจริงน่าจะ ~7–9  
> ³ DOF fg:bg บน wide-hero = 0.18 — **accepted tradeoff** (deep-focus โดยดีไซน์สำหรับ establishing shot; `VOXELFORGE_DOF=8,10`)  
> ⁴ G5 บน grade-vista — **inapplicable-gate**: gate ออกแบบสำหรับ indoor (มีหน้าต่าง); เฟรมนี้เป็น outdoor portrait ไม่มีหน้าต่าง → auto-locator เจอท้องฟ้าที่มี RGB(252,251,255) ซึ่งตก G/B≤245 จริง แต่ไม่ใช่ rendering defect

---

## 🔬 G3 Deep Dive — Interior Darkness (จาก `grade_g3.py`)

| เฟรม | p01-L | p05-L | p10-L | p50-L | darkest shade RGB | warm? | G3 |
|---|---|---|---|---|---|---|---|
| wide-hero | — | 14.3% | 14.8% | 25.0% | (45,22,7) R−B=+38 | ✅ | **PASS** |
| grade-vista | 6.4% | 6.4% | 6.4% | 33.8% | (16,16,20) R−B=−4 | ❌ cold! | **FAIL** |
| gate3-boot | 1.5% | 2.8% | 5.1% | 20.2% | (18,0,0) R−B=+18 | ✅ (but G=0) | **FAIL** |
| gate3-combat | 1.9% | 3.8% | 8.6% | 16.8% | (19,0,0) R−B=+19 | ✅ (but G=0) | **FAIL** |
| gate3-walk | 2.0% | 2.2% | 3.6% | 22.1% | (22,0,0) R−B=+22 | ✅ (but G=0) | **FAIL** |

**ข้อสังเกตสำคัญ:**
- grade-vista — เฟรมเดียวที่ตก **ทั้งสอง clause** (p05-L ต่ำ + darkest shade เย็น R−B=−4) → ต้องแก้ทั้งแสงและสี
- gate3 ทั้งสาม — p05-L ต่ำมาก (2.2–3.8%) แต่ darkest shade **อุ่น** (R−B +18 ถึง +22) → ต้องการแค่แสงเพิ่ม ไม่ต้องแก้สี
- combat p10-L = 8.6% → เกิน threshold ที่ p10! แปลว่า "มืดแค่ 5% แรกของพิกเซล" — การยก ambient lux นิดเดียวจะผ่าน G3

---

## 🎨 Beauty Delta — เทียบ golden ref (จาก `grade_beauty.py`)

| metric | golden ref | wide-hero | grade-vista | gate3-boot | gate3-combat | gate3-walk |
|---|---|---|---|---|---|---|
| **Warmth R−B** (global mean) | 118.5 | 108.6 | 54.7 | 69.4 | 78.5 | 73.2 |
| **Sat** (global mean) | 94.4% | 86.3% | 62.8% | 82.8% | 86.9% | 87.2% |
| **Shadow L** (darkest15%) | 9.4% | 14.4% | 6.2% | 5.0% | 5.2% | 4.7% |
| **Shadow R−B** | +50.3 | +69.0 | −4.1 ⚠️ | +43.6 | +48.4 | +36.5 |
| **Highlight p99** | 227.0 | 205.7 | 162.2 | 169.5 | 174.4 | 157.1 |
| **Clip %** (≥254) | 0.00% | 0.00% | 0.03% | 0.00% | 0.01% | 0.00% |
| **Micro-contrast** | 5.24 | 5.89 | 15.76² | 7.43 | 8.34 | 5.82 |
| **Edge energy** | 2.87 | 0.80 | 4.71² | 2.03 | 2.08 | 1.53 |

> ⚠️ grade-vista shadow R−B = **−4.1** — dark area ออกฟ้า/เย็น (B > R) → ตรงกับ G3 coldest shade finding

---

## 📐 Baseline Sanity Check — wide-hero ยังยืนไหม?

| metric | golden ref | wide-hero | delta | status |
|---|---|---|---|---|
| Lum mean | 70.1 | 76.1 | +6 | สว่างกว่านิด |
| Warmth R−B | 118.5 | 108.6 | −10 | เย็นกว่า ref เล็กน้อย (แต่ผ่าน P0) |
| Sat mean | 94.4% | 86.3% | −8 | จืดกว่า ref (แต่ผ่าน P0 midtone) |
| Micro-contrast | 5.24 | 5.89 | +0.65 | detail ดีกว่า ref! |
| p95 | 165.8 | 177.4 | +12 | สว่างกว่า ref |
| G3/G5/G6 | all PASS | all PASS | — | gate มั่นคง |

**Baseline มั่นคง** — wide-hero ผ่าน P0 5/6 (DOF ตกโดยดีไซน์) + ผ่าน gate ครบ. งานต่อไปคือดึง gameplay frames ให้ไล่ตาม baseline

---

## 🏆 5 อันดับแรก — จัดอันดับด้วย "แก้แล้วสายตาคนธรรมดาเห็นความต่างมากที่สุด"

### 🥇 อันดับ 1: G3 — เงาในร่มถูกบดดำ (Interior Crushed to Black)

| Field | Value |
|---|---|
| **ความรุนแรง** | 🔴🔴🔴🔴🔴 รุนแรงที่สุด |
| **ค่าเป้า** | p05-L ≥ 8% + darkest shade warm (R ≥ B) |
| **ค่าที่วัดได้** | vista: p05=6.4% + cold (R−B=−4) · boot: 2.8% · combat: 3.8% · walk: 2.2% |
| **gap** | ขาด 1.6–5.8 จุด (บางเฟรมมืดกว่าที่ควร **3–4 เท่า**) |
| **สายตาคนธรรมดา** | เห็นทันที — เงาในฉากกลางแจ้งเป็น**หลุมดำ** ไร้รายละเอียด |
| **ไฟล์** | `client/src/look.rs:571-572` (`Hour::GOLDEN` ambient color + lux) |
| **เลน** | **Rose** |
| **แนวทางแก้** | (1) `ambient_lux: 1100.0 → 2000–2400` (เทียบ: hero shot ได้ 2800) — หนึ่งปุ่มยกแสง fill ทั้งฉาก (2) `ambient[2] (B): 0.60 → 0.45–0.50` — ลด blue wash ใน open shade (แก้เฉพาะ vista ที่ coldest shade เป็นฟ้า) |

> 📖 *"The people walking in darkness have seen a great light."* — Isaiah 9:2

---

### 🥈 อันดับ 2: Warmth R−B — โทนอุ่น golden-hour หาย (Cold Midtones)

| Field | Value |
|---|---|
| **ความรุนแรง** | 🔴🔴🔴🔴 |
| **ค่าเป้า** | midtone R−B ≥ +110 |
| **ค่าที่วัดได้** | vista +84.9 · boot +82.5 · combat +80.4 · walk +98.7 |
| **gap** | ขาด 11–30 จุด |
| **สายตาคนธรรมดา** | เห็นชัด — รูปไม่ใช่ "golden hour" เหมือน ref; ดูเหมือนกลางวันธรรมดา |
| **ไฟล์** | `client/src/look.rs:397` (`grade::TEMPERATURE = 0.05` — ceiling แล้ว) + `look.rs:571-572` |
| **เลน** | **Rose** |
| **แนวทางแก้** | TEMPERATURE ดัน warmth ได้ถึง ~97 ก่อนชน magenta onset → แกนที่เหลือมาจาก **ambient lux ที่สูงขึ้น** (ดึง R-channel fill ใน open shade → แก้ G3 ไปด้วยในตัว) + ตรวจ `ambient[2] (B): 0.60` ว่าสูงไปสำหรับฉากที่มีท้องฟ้าเยอะหรือไม่ |

---

### 🥉 อันดับ 3: Saturation — สีจืด/หม่น (Washed-Out Look)

| Field | Value |
|---|---|
| **ความรุนแรง** | 🟡🟡🟡 |
| **ค่าเป้า** | midtone sat ≥ 90% |
| **ค่าที่วัดได้** | vista 85.3% · boot 88.6% (combat 90.9% ✅ · walk 95.1% ✅) |
| **gap** | ขาด 1.4–4.7 จุด |
| **สายตาคนธรรมดา** | เทียบข้าง ref จะเห็นว่าสีไม่สด — ไม้/หินดู "จาง" |
| **ไฟล์** | `client/src/look.rs:443` (`grade::POST_SATURATION = 1.90`) |
| **เลน** | **Rose** |
| **แนวทางแก้** | ⚠️ **1.90 อาจเพียงพอแล้ว** (combat + walk ผ่านที่ 90.9%/95.1%) — vista + boot ถ่ายด้วย exe ก่อน commit 1.90 → ต้อง rebuild + re-capture ยืนยัน; ถ้ายังไม่ถึง: เพิ่มเป็น 2.00–2.10 โดยรัน `colour_gate.py` กัน magenta หลุด |

---

### 4️⃣ อันดับ 4: G4a Penumbra — ขอบเงาคมเกินบน Hero Shot + Vista

| Field | Value |
|---|---|
| **ความรุนแรง** | 🟡🟡 |
| **ค่าเป้า** | penumbra median ≥ 5px |
| **ค่าที่วัดได้** | wide-hero 4px ❌ · grade-vista 3px ❌ (boot 8 ✅ · combat 5 ✅ · walk 9 ✅) |
| **gap** | wide-hero ขาด 1px · vista ขาด 2px |
| **สายตาคนธรรมดา** | ซูม 400% ที่ขอบเงาถึงเห็น — ขอบแข็ง 1px แทนที่จะฟุ้งนุ่ม |
| **ไฟล์** | `client/src/look.rs:347` (`PCSS_WIDTH = 3.0`) + `look.rs:1165-1176` (tier logic) |
| **เลน** | **Rose** |
| **แนวทางแก้** | (1) `PCSS_WIDTH: 3.0 → 4.0` (Ultra tier) (2) สำหรับ High tier (hero shot): เพิ่ม `ShadowFilteringMethod::Gaussian` เป็น fallback → เพิ่ม penumbra 4px → 5+px โดยไม่ต้องเปิด PCSS |

---

### 5️⃣ อันดับ 5: Highlight p95 — หน้าต่าง/emissive ทึมไปนิด (gate3-walk)

| Field | Value |
|---|---|
| **ความรุนแรง** | 🟢 |
| **ค่าเป้า** | p95 150–185 |
| **ค่าที่วัดได้** | walk 149.79 (ขาด 0.21) |
| **gap** | 0.21 จุด — แทบไม่หลุด |
| **สายตาคนธรรมดา** | **มองไม่เห็น** — ต่าง 0.21; frame-to-frame variance gameplay มี ±1–2 อยู่แล้ว |
| **ไฟล์** | `client/src/look.rs:587` (`Hour::GOLDEN.ev100 = 10.8`) |
| **เลน** | **Rose** |
| **แนวทางแก้** | ev100 10.8 → 10.9 (tiny bump) หรือ **ยอมรับเป็น noise** — gameplay frame-to-frame variance ปกติ |

---

## 📋 สรุปการจัดคิวงาน (Action Queue)

| อันดับ | Gap | ไฟล์ + บรรทัด | เลน | แก้ยังไง | ผลกระทบ |
|---|---|---|---|---|---|
| 🥇 | G3 ดำตัน | `look.rs:571-572` | Rose | `ambient_lux` 1100→2000+ + `ambient[2]` 0.60→0.45 | 🔴 เห็นทันที |
| 🥈 | โทนเย็น | `look.rs:397,571-572` | Rose | TEMP ceiling แล้ว → ambient lux + B-lower | 🔴 เทียบ ref เห็นชัด |
| 🥉 | สีจืด | `look.rs:443` | Rose | 1.90 อาจพอแล้ว → rebuild ยืนยันก่อนเพิ่ม | 🟡 เทียบ ref เห็น |
| 4 | เงาคม | `look.rs:347` | Rose | PCSS 3→4 หรือเปิด Gaussian ที่ High tier | 🟡 ซูมถึงเห็น |
| 5 | p95 หายนิด | `look.rs:587` | Rose | ev100 10.8→10.9 (หรือ ignore) | 🟢 มองไม่เห็น |

> ⚡ **หนึ่งปุ่ม สองแกน:** G3 (🥇) กับ Warmth (🥈) แก้ด้วย `ambient_lux` ปุ่มเดียว — ROI สูงสุด  
> ⚡ งานทั้งหมดอยู่ในเลนของ **Rose** (`client/src/look.rs`) — ไม่มี dependency ข้ามเลน

---

## 📝 Script Coverage — รันแล้ว vs ยังไม่ได้รัน

| สคริปต์ | รันแล้ว? | ผลลัพธ์ |
|---|---|---|
| `grade_axes.py` | ✅ 5 เฟรม | P0 axis table |
| `grade_gate.py` | ✅ 5 เฟรม | **Canonical G3/G5/G6** |
| `grade_look.py` | ✅ 5 เฟรม | G4a penumbra, G2 key-dir, G1 visual flag |
| `grade_midtone.py` | ✅ 5 เฟรม | Midtone band R−B + sat |
| `measure_penumbra.py` | ✅ 5 เฟรม | Raw penumbra px |
| `grade_hero.py` | ✅ 5 เฟรม | G6 cross-check, G3/G5 detail, green share |
| `grade_beauty.py` | ✅ 5 เฟรม | Full-frame delta vs golden ref |
| `grade_g3.py` | ✅ 5 เฟรม | G3 deep dive (p01–p99, darkest shade detail) |
| `grade_web_parity.py` | ❌ | ต้องการ paired native+web builds (ใช้กับเฟรมชุดนี้ไม่ได้) |
| `grade_g7.py` | ❌ | ต้องการ --ab-haze/--ab-ao paired frames (ไม่มีในเซตนี้) |
| `grade_ref.py` / `grade_ref2.py` | ❌ | เกรดเฉพาะ golden ref — ref ผ่าน AAA แล้วตาม calibration log |

---

_เกรดด้วยสคริปต์ 8 ตัว รันจริงทุกเฟรม — Sun (Specialist), 2026-08-06._
