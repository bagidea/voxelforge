# 🔬 VERIFY — Baseline, 2026-08-19

> เลนอิสระ ไม่เชื่อรายงานใคร ตรวจซ้ำด้วยมือตัวเอง ก่อนเลนอื่น (Poppy/Flamingo/Kevin/Sun) ส่งงานวันนี้ครบ
> ขอบเขต: ตรวจอย่างเดียว ไม่แก้โค้ด ไม่รัน `cargo build`

## 1) สุขภาพเครื่องวัด — `_pixel_artgap_controls.py`

```
python scripts/_pixel_artgap_controls.py
```

**exit code = 0** — control ทั้งหมด PASS (9 กลุ่ม, C0–C10, รวม positive control C0 ที่ REF เทียบตัวเอง 22/22 แกน ที่เกณฑ์ 60%)

⇒ ตัวเลขทั้งกระดานด้านล่างเชื่อได้ตามกติกาข้อ 7 — เครื่องไม่ได้พัง ไม่มี collapse หลุดผ่าน ไม่มี survive หลุดตก

## 2) Baseline 20 แกน — ก่อนงานวันนี้ (รัน `--oneshot` ด้วยตัวเอง)

```
python scripts/_pixel_artgap_grade.py _matmaps_after.png docs/assets/look/outdoor-noon_after.png --oneshot --label "VERIFY baseline 2026-08-19"
```

**exit code = 1** (มี GAP — ตรงตามสเปก: `0`=ผ่านหมด, `1`=มี GAP, `2`=วัดครบแต่มีแกนปฏิเสธ, `3`=control ตก)

### ผลตัดสิน — **9 / 20 = 45% ของภาพอ้างอิง CEO** (แกนหลัก `_matmaps_after.png`)

ตรงกับตัวเลขอ้างอิงเดิมที่ Director ให้มา (`docs/aaa-scoreboard-live.md` รอบ 16:28 = 9/20 = 45%) — **ยืนยันอิสระด้วยการรันเองซ้ำ ไม่ใช่แค่อ่านไฟล์เดิม**

| แกน | เจ้าของเลน | REF | `_matmaps_after` | %REF | สถานะ | `outdoor-noon_after` | %REF | สถานะ |
|---|---|---:|---:|---:|---|---:|---:|---|
| dynamic range | renderer | 187.71 | 129.05 | 69% | ✅ ok | 106.13 | 57% | ❌ GAP |
| crushed blacks ↓ | renderer | 0.55 | 4.80 | 873% | ❌ GAP | 0.16 | 29% | ✅ ok |
| clipped highlights ↓ | renderer | 10.30 | 0.16 | 2% | ✅ ok | 0.80 | 8% | ✅ ok |
| tonal spread | renderer | 32 | 21 | 66% | ✅ ok | 22 | 69% | ✅ ok |
| saturation | renderer | 57.56 | 76.61 | 133% | ✅ ok | 71.12 | 124% | ✅ ok |
| palette breadth | art | 15 | 6 | 40% | ❌ GAP | 7 | 47% | ❌ GAP |
| hue diversity | art | 3.90 | 2.15 | 55% | ❌ GAP | 2.14 | 55% | ❌ GAP |
| cool/water chroma | world | 2.96 | 0.00 | 0% | ❌ GAP | 0.00 | 0% | ❌ GAP |
| local contrast r3 | art | 18.74 | 11.61 | 62% | ✅ ok | 11.44 | 61% | ✅ ok |
| detail per area | art | 58.63 | 31.03 | 53% | ❌ GAP | 38.84 | 66% | ✅ ok |
| flat/featureless area ↓ | art | 15.82 | 25.37 | 160% | ✅ ok | 11.07 | 141% | ✅ ok |
| sky presence | world | 21.85 | 18.24 | 83% | ✅ ok | 11.12 | 51% | ❌ GAP |
| sky brighter than ground | renderer | 1.80 | 0.16 | 9% | ❌ GAP | 1.05 | 59% | ❌ GAP |
| sky sitting at black (L<10) ↓ | renderer | 0.00 | 36.38 | – | ❌ GAP | 3.84 | – | ❌ GAP |
| sky blown to white (L>245) ↓ | renderer | 5.09 | 0.00 | 0% | ✅ ok | 0.00 | 0% | ✅ ok |
| sky tonal gradient | renderer | 172.58 | 17.76 | 10% | ❌ GAP | 103.83 | 60% | ✅ ok |
| sky hue range | renderer | 60.00 | 20.00 | 33% | ❌ GAP | 10.00 | 17% | ❌ GAP |
| distant silhouette | renderer | 22.33 | **–** | **SKIP** | ⏸ ยังวัดไม่ได้ | 28.76 | 129% | ✅ ok |
| detail surviving at distance | art | 15.61 | 8.05 | 52% | ❌ GAP | 6.34 | 41% | ❌ GAP |
| atmospheric perspective (sat) | renderer | 1.13 | 1.02 | 91% | ✅ ok | 1.46 | 129% | ✅ ok |
| atmospheric perspective (detail) [adv] | renderer | 0.81 | 1.09 | 135% | 📎 advisory (ไม่ตัดสิน) | 1.23 | 151% | 📎 advisory |
| emissive light points | world | 159 | 2 | 1% | ❌ GAP | 8 | 5% | ❌ GAP |

`_matmaps_after.png`: **9 ok / 11 GAP / 1 SKIP / 1 advisory** (22 แกนวัดจริง, ตัวหารกระดาน = 20 = ok+GAP)
`outdoor-noon_after.png`: **11 ok / 10 GAP / 0 SKIP / 1 advisory** (เฟรมรอง ไม่ถูกนับใน 9/20 หลัก — ตรงกับกติกาเดิมของกระดาน)

**สังเกตสำคัญ:** สกอร์รวม 9/20 **เท่าเดิมเป๊ะ** กับรอบ 16:28 ของ Director แต่ **ไม่ใช่ Δ0 หลอกแบบรอบก่อน** — ดูข้อ 3 ด้านล่าง ไฟล์ `_matmaps_after.png` เปลี่ยนไบต์จริงตั้งแต่เมื่อวาน แต่การขยับต่อแกน (ดู mtime/history) เล็กเกินกว่าจะข้ามเกณฑ์ 60% ในทิศทางไหนเพิ่มเติม — สุทธิแล้วยังล็อกอยู่ที่ 9 แกนเดิม

## 3) sha256 ของทุกเพลตตัดสิน (พิสูจน์ render ใหม่จริง ไม่ใช่ไบต์เดิม)

| ไฟล์ | sha256 (เต็ม) | mtime (local) |
|---|---|---|
| `_matmaps_after.png` | `eca2d49358aa76cf8ea5012d09b0e70618b19c5da63ba7d026663cd045c678e5` | 2026-08-19 15:47:41 +0700 |
| `docs/assets/look/outdoor-noon_after.png` | `41c0190c4564d296836f0783a1eb7e092564eb2e615004a7527d6e4a10966a4c` | 2026-08-15 04:37:59 +0700 |
| `docs/assets/artgap/art-gap-vs-ceo-ref-2026-08-18.png` | `243866ffcef4e5adb1e4a4b7f61314599a42aeea1a69fdfc70e786df2b57af7c` | 2026-08-18 16:27:45 +0700 |
| `docs/refs/ceo_ref_sunset_valley.jpg` (REF, ไม่ควรขยับ) | `d4491a12e14e5188d875b5933bb6d6358aedb53451533d1b4d906ad40e9528a1` | 2026-08-18 14:58:38 +0700 |

**อ่านค่า:**
- `_matmaps_after.png` **sha256 เปลี่ยนจากรอบ 16:28** (`00e29783d438…` → `eca2d49358aa…`) และ **mtime = วันนี้ 15:47** — มีคนแล้ว render ทับไฟล์นี้ *ก่อน* ที่ VERIFY จะเริ่มงานรอบนี้เสียอีก (ไม่ทราบว่าเป็นใคร ต้องถาม Director/เลนที่เกี่ยวข้อง — คนละประเด็นกับ "ตัวเลขน่าเชื่อไหม" เพราะ control ผ่านและ sha256 ยืนยันว่าเป็นเรนเดอร์ใหม่จริง ไม่ใช่ก็อปปี้ไบต์เดิม)
- `docs/assets/look/outdoor-noon_after.png` sha256 **ตรงกับรอบก่อนทุกตัว** และ mtime ค้างที่ 2026-08-15 — เฟรมนี้ **ยังไม่มีใครแตะตั้งแต่ 4 วันก่อน** ถ้ารอบ after อ้างว่าแก้ไฟล์นี้แล้วต้องเช็ค sha256 ใหม่ก่อนเชื่อ
- REF ไม่ขยับ (ต้องไม่ขยับ) — ยืนยันว่าไม่มีใครแก้ภาพอ้างอิงกลางทาง

## 4) แกน "ผ่านแบบ phantom" — ref ไม่รองรับ หรือเครื่องปฏิเสธวัด

| แกน | ประเภท | เหตุผลคำต่อคำ | ผลต่อสกอร์ |
|---|---|---|---|
| `distant silhouette` (renderer) | **เครื่องปฏิเสธวัด (SKIP)** | `sky unlit (36% below L=10) - contrast against a black void is degenerate` — grader เองปฏิเสธเพราะฟ้าเราดำเกิน 20% (36.38%) ให้ contrast กับฟ้าดำ = ค่าไร้ความหมาย | **ไม่ถูกนับเป็นทั้งผ่านและตก** — ตัวหารกระดานคือ 20 ไม่ใช่ 21 (นับเป็นตกไปแล้วในทางปฏิบัติ เพราะไม่ได้ให้เครดิตแกนที่ปฏิเสธ) |
| `atmospheric perspective (detail)` (renderer, `depth_micro_ratio`) | **ref ไม่รองรับเส้นตัด (advisory)** | REF เอง = 0.81 (near micro 12.67 < far micro 15.61) คือ **ภาพอ้างอิงของ CEO เองก็มีรายละเอียดไกลมากกว่าใกล้** — ตั้งเกณฑ์ "ต้อง ≥60% ของ 0.81" จะวัดแค่ "จัดฉากรกตรงไหน" ไม่ใช่ atmospheric perspective จริง เอกสาร rubric เองบันทึกไว้แล้วว่า "จ่ายค่าแกน phantom นี้มา 3 ครั้ง" | **วัดแล้ว (1.09/1.23) แต่ไม่ตัดสิน** — ไม่กระทบ 9/20 ไม่ว่าค่าจะสูงต่ำแค่ไหน |

ไม่พบแกนอื่นที่เข้าเกณฑ์ phantom (ref ไม่รองรับ / เครื่องปฏิเสธ) ใน 22 แกนที่เหลือ — ทุกแกน ok/GAP อีก 20 ตัว มีทั้งเกณฑ์และเลข REF ที่สมเหตุสมผล (ไม่มีตัวไหนได้ 0/0 หรือ REF=0 ที่ทำให้ % ไร้ความหมายนอกจาก `sky sitting at black` ซึ่งมี REF=0 โดยธรรมชาติของแกน "น้อยกว่าดีกว่า" — ไม่ใช่ bug เครื่องวัด)

**ไม่ได้ตรวจเชิงลึกเรื่อง gamut-clip-fakes-colour-gates** (บทเรียนเก่าของออฟฟิศ: sat/warmth PASS อาจมาจาก channel clamp) — `saturation` (133% ok) คู่กับ `crushed blacks` (873% GAP, เราครัชมากกว่า ref เยอะ) เป็นคอมโบที่ **ควรสงสัย** ว่าความอิ่มสีที่วัดได้อาจถูกดันขึ้นจากพิกเซลที่ crush/clip บางส่วน ไม่ใช่สีจริงที่ artist ตั้งใจ — ทิ้งไว้เป็นข้อสังเกตสำหรับรอบ after ถ้ามีเวลา ไม่ใช่ finding ที่ยืนยันแล้ว (ต้องนับ clipped-channel count แยก ไม่ใช่แค่ mean ถึงจะฟันธงได้)

## สรุปสำหรับ Director/CEO

- ✅ เครื่องวัดสุขภาพดี (control exit 0) — เชื่อเลขในกระดานได้
- ✅ Baseline ยืนยันอิสระ = **9/20 = 45%** ตรงกับตัวเลขเดิมที่ Director ให้มา
- ⚠️ `_matmaps_after.png` **ถูกเรนเดอร์ใหม่แล้ว** (sha256 เปลี่ยน, mtime วันนี้ 15:47) **ก่อน** VERIFY จะเริ่มงาน — สกอร์รวมไม่ขยับ (ล็อกที่ 9/20) แม้ตัวเลขต่อแกนขยับจริงเล็กน้อย → ใครก็ตามที่ render ไฟล์นี้ยังไม่ทำให้แกนไหนข้ามเกณฑ์
- ⚠️ `docs/assets/look/outdoor-noon_after.png` **ยังไม่มีใครแตะ** (sha256/mtime เดิมตั้งแต่ 08-15) — ถ้ารอบ after อ้างว่าแก้เฟรมนี้ ต้องเช็ค sha256 ก่อนเชื่อ
- 2 แกน (`distant silhouette` SKIP, `atmospheric perspective (detail)` advisory) เป็น phantom โดยเจตนาของกระดานเอง ไม่ใช่บั๊กใหม่ — ไม่กระทบ 9/20
- สิ่งที่ VERIFY **ยังไม่ได้ทำ** (นอกขอบเขตรอบนี้ตามบรีฟ): ไม่ได้สืบว่าใคร render `_matmaps_after.png` ทับก่อนเวลา, ไม่ได้ฟันธงเรื่อง gamut-clip บนแกน saturation

รอเลนอื่นส่งงานครบ แล้วเรียก VERIFY กลับมาทำรอบ **after** — เทียบ sha256 ใหม่ vs baseline นี้ทุกไฟล์ (ทั้ง 4 ไฟล์ในข้อ 3), รันเกรดซ้ำ, สร้างตาราง before→after 20 แกนพร้อมชื่อคนทำ, list phantom เพิ่มถ้ามี
