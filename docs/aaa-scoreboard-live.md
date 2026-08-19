# 🏁 AAA Scoreboard — LIVE

> **ไฟล์นี้ถูกเครื่องสร้างขึ้นใหม่ทุกครั้งที่รัน grader — ห้ามแก้ด้วยมือ.**  
> สร้างโดย `scripts/_pixel_artgap_grade.py --oneshot` · เกณฑ์ทั้งหมดอยู่ใน [`docs/look-acceptance-rubric.md` §P0-ENV](look-acceptance-rubric.md#-p0-env--ท้องฟ้า--น้ำ--environment-scripts_pixel_artgap_gradepy--เพิ่ม-2026-08-18)

**รอบล่าสุด:** 2026-08-19 19:48 +0700 · **poppy/renderer: first PLAY binary with the painted dome (target/release/voxelforge.exe relinked 19:45, HEAD gain 8.0 via _SKY_GAIN)**

## 15 / 21 แกนผ่าน = **71% ของภาพอ้างอิง CEO**  ·  6 GAP  ·  0 ยังวัดไม่ได้  ·  1 advisory

เฟรมที่ตัดสิน: **`_poppy_sky_g80_HEAD.png`** (1333x750 normalised) เทียบ **`docs/refs/ceo_ref_sunset_valley.jpg`** (747x1339)  
เครื่องวัดผ่าน control แล้ว (`_pixel_artgap_controls.py` **exit 0**) ⇒ เลขข้างล่างเชื่อได้ตามกติกาข้อ 7

> **ตัวหาร 21 = แกนที่วัดได้จริงรอบนี้** จากด่านทั้งหมด 21 แกน — อีก 0 แกนเครื่อง**ปฏิเสธที่จะวัด** จึงไม่ถูกนับเป็นทั้งผ่านและตก (นับเป็นตก = ให้คะแนนการปฏิเสธเป็นความผิดของเฟรม) · advisory อีก 1 แกนวัดแล้วแต่ไม่ตัดสิน

## แกนทั้งหมด — เทียบภาพอ้างอิงของ CEO เป็น %

| แกน | เจ้าของเลน | REF | เฟรมเรา | % ของ REF | สถานะ | Δ จากรอบก่อน |
|---|---|---:|---:|---:|---|---:|
| cool/water chroma | world | 2.96 | 0.15 | 5% | ❌ GAP | – |
| emissive light points | world | 159 | 11 | 7% | ❌ GAP | – |
| sky tonal gradient | renderer | 172.58 | 34.30 | 20% | ❌ GAP | – |
| sky hue range | renderer | 60.00 | 20.00 | 33% | ❌ GAP | – |
| sky brighter than ground | renderer | 1.80 | 0.84 | 47% | ❌ GAP | – |
| palette breadth | art | 15 | 8 | 53% | ❌ GAP | – |
| atmospheric perspective (detail) | renderer | 0.81 | 1.00 | 123% | 📎 advisory | – |
| dynamic range | renderer | 187.71 | 112.74 | 60% | ✅ ok | – |
| hue diversity | art | 3.90 | 2.68 | 69% | ✅ ok | – |
| flat/featureless area ↓ | art | 15.82 | 23.23 | 147% | ✅ ok | – |
| detail surviving at distance | art | 15.61 | 11.54 | 74% | ✅ ok | – |
| tonal spread | renderer | 32 | 24 | 75% | ✅ ok | – |
| atmospheric perspective (sat) | renderer | 1.13 | 0.87 | 77% | ✅ ok | – |
| detail per area | art | 58.63 | 47.57 | 81% | ✅ ok | – |
| local contrast r3 | art | 18.74 | 15.45 | 82% | ✅ ok | – |
| distant silhouette | renderer | 22.33 | 19.09 | 85% | ✅ ok | – |
| sky presence | world | 21.85 | 19.25 | 88% | ✅ ok | – |
| sky sitting at black (L<10) ↓ | renderer | 0.00 | 0.00 | – | ✅ ok | – |
| saturation | renderer | 57.56 | 60.59 | 105% | ✅ ok | – |
| crushed blacks ↓ | renderer | 0.55 | 0.00 | 0% | ✅ ok | – |
| sky blown to white (L>245) ↓ | renderer | 5.09 | 0.00 | 0% | ✅ ok | – |
| clipped highlights ↓ | renderer | 10.30 | 0.27 | 3% | ✅ ok | – |

**`↓` ต่อท้ายชื่อแกน = แกนที่ "น้อยกว่าดีกว่า"** — คอลัมน์ `% ของ REF` เป็น *ค่าดิบเทียบค่าดิบ* ไม่ใช่คะแนน ⇒ แกน ↓ ที่ได้ 2% คือ **ดี** (เราเหลือ 2% ของสิ่งที่ REF มี)  
คอลัมน์ Δ: **▲ = ขยับเข้าหาภาพอ้างอิง · ▼ = ถอยห่าง** (ตัวเลขคือส่วนต่างดิบ จึงเป็น `▲ -36.44` ได้ ถ้าแกนนั้นน้อยกว่าดีกว่า) · `–` = รอบก่อนไม่มีตัวเลขให้เทียบ

`ok` = ≥ 60% ของ REF (ตามทิศของแกน) · `GAP` = ต่ำกว่านั้น · `over` = **ไกลจาก REF เกิน 167%** (advisory เฉย ๆ ไม่ตัด FAIL — มีไว้กันคนอ่านตัวเลขไกลจาก REF แล้วนึกว่า "ดีกว่า ref") · `advisory` = **วัดแล้วแต่ไม่ตัดสิน** (ภาพอ้างอิงเองไม่รองรับเส้นตัดของแกนนั้น) · `ยังวัดไม่ได้` = เครื่องมือ **ปฏิเสธที่จะวัด** ไม่ใช่วัดแล้วได้ศูนย์

<details><summary>ทำไม **atmospheric perspective (detail)** ถึงเป็น advisory ไม่ใช่ด่าน</summary>

`depth_micro_ratio` = `micro(near) / micro(far)`. บนภาพอ้างอิงของ CEO ค่านี้ = **0.81** (near 12.67 / far 15.61) — คือ **แถบไกลของภาพอ้างอิงมีรายละเอียดมากกว่าแถบใกล้**. แปลว่า *ภาพอ้างอิงเองไม่ได้แสดงสิ่งที่แกนนี้ตั้งชื่อไว้* การเอาเฟรมเราไปเทียบกับ 0.81 จึงวัดว่า "ฉากวางของรกไว้ตรงไหน" ไม่ได้วัดว่าหมอกทำให้ระยะไกลนุ่มลงไหม — และ "ทำให้ได้ 60% ของ 0.81" เป็นบาร์ที่เฟรมไหนก็ข้ามได้ฟรี. กติกาข้อ 7: **เส้นตัดที่ภาพอ้างอิงเองไม่รองรับ = phantom target** เอกสารนี้จ่ายค่ามันมาแล้ว 3 ครั้ง.

ฝาแฝดของมัน `depth_sat_ratio` **ยังเป็นด่านอยู่** เพราะ REF ได้ 1.13 > 1 (ใกล้อิ่มสีกว่าไกล) = รองรับชื่อแกนตัวเอง. จะเลื่อน `depth_micro_ratio` กลับมาเป็นด่านได้ ต้องมี ref กลางแจ้งที่ค่านี้ > 1 แล้วบันทึก control ไว้ก่อน

</details>

## ที่ยังวัดไม่ได้ — ปฏิเสธเพราะอะไร และใครปลดล็อก

**ไม่มี** — ทุกแกนวัดได้หมดในรอบนี้

## สุขภาพเครื่องวัด (ต้องดูก่อนเถียงเรื่องเลข)

| | REF | เฟรมเรา |
|---|---:|---:|
| sky mask กินพื้นที่ | 21.85% | 19.25% |
| คอลัมน์ที่มีเส้นขอบฟ้า | 100.00% | 91.45% |
| ฟ้าดำสนิท (L<10) | 0.00% | 0.00% |
| ฟ้าไหม้ขาว (L>245) | 5.09% | 0.00% |

mask overlay เขียนออกมาทุกครั้ง (ฟ้า=น้ำเงิน · far=ชมพู · near=เขียว · เส้นขอบฟ้า=เหลือง) — **ดู mask ก่อนเถียงเรื่องตัวเลข**

## เฟรมอื่นในรอบเดียวกัน

| เฟรม | ผ่าน | GAP | ยังวัดไม่ได้ |
|---|---:|---:|---:|
| `_pixel_world_AFTER.png` | 13 | 7 | 1 |

## รันซ้ำ (คำสั่งเดียว ต่อ build ใหม่ 1 ตัว)

```bash
python scripts/_pixel_artgap_grade.py _poppy_sky_g80_HEAD.png \
       _pixel_world_AFTER.png \
       --oneshot --label "<ใครส่ง build / commit>"
```

`--oneshot` = รัน control ก่อนเสมอ (control ตก ⇒ **exit 3** และ **ไม่พิมพ์เลขให้เชื่อ**) → เกรด → เขียน JSON + mask + history → สร้างหน้านี้ใหม่  
exit: `0` ผ่านหมด · `1` มี GAP · `2` วัดครบแต่มีแกนที่ปฏิเสธจะวัด · `3` control ตก

| ไฟล์ | sha256 (12 ตัวแรก) |
|---|---|
| `docs/refs/ceo_ref_sunset_valley.jpg` (REF) | `d4491a12e14e` |
| `_poppy_sky_g80_HEAD.png` | `1be25dbc32e2` |
| `_pixel_world_AFTER.png` | `3f2d6001dc10` |
