# 🏁 AAA Scoreboard — LIVE

> **ไฟล์นี้ถูกเครื่องสร้างขึ้นใหม่ทุกครั้งที่รัน grader — ห้ามแก้ด้วยมือ.**  
> สร้างโดย `scripts/_pixel_artgap_grade.py --oneshot` · เกณฑ์ทั้งหมดอยู่ใน [`docs/look-acceptance-rubric.md` §P0-ENV](look-acceptance-rubric.md#-p0-env--ท้องฟ้า--น้ำ--environment-scripts_pixel_artgap_gradepy--เพิ่ม-2026-08-18)

**รอบล่าสุด:** 2026-08-19 16:05 +0700 · **world lane: water+lanterns in beach_dusk (pixel)**

## 10 / 20 แกนผ่าน = **50% ของภาพอ้างอิง CEO**  ·  10 GAP  ·  1 ยังวัดไม่ได้  ·  1 advisory

เฟรมที่ตัดสิน: **`_pixel_world_AFTER.png`** (1333x750 normalised) เทียบ **`docs/refs/ceo_ref_sunset_valley.jpg`** (747x1339)  
เครื่องวัดผ่าน control แล้ว (`_pixel_artgap_controls.py` **exit 0**) ⇒ เลขข้างล่างเชื่อได้ตามกติกาข้อ 7

> **ตัวหาร 20 = แกนที่วัดได้จริงรอบนี้** จากด่านทั้งหมด 21 แกน — อีก 1 แกนเครื่อง**ปฏิเสธที่จะวัด** จึงไม่ถูกนับเป็นทั้งผ่านและตก (นับเป็นตก = ให้คะแนนการปฏิเสธเป็นความผิดของเฟรม) · advisory อีก 1 แกนวัดแล้วแต่ไม่ตัดสิน

## แกนทั้งหมด — เทียบภาพอ้างอิงของ CEO เป็น %

| แกน | เจ้าของเลน | REF | เฟรมเรา | % ของ REF | สถานะ | Δ จากรอบก่อน |
|---|---|---:|---:|---:|---|---:|
| cool/water chroma | world | 2.96 | 0.00 | 0% | ❌ GAP | 0 |
| sky sitting at black (L<10) ↓ | renderer | 0.00 | 36.58 | – | ❌ GAP | ▲ -0.03 |
| emissive light points | world | 159 | 6 | 4% | ❌ GAP | 0 |
| sky brighter than ground | renderer | 1.80 | 0.16 | 9% | ❌ GAP | 0 |
| sky tonal gradient | renderer | 172.58 | 17.55 | 10% | ❌ GAP | 0 |
| crushed blacks ↓ | renderer | 0.55 | 4.81 | 875% | ❌ GAP | ▲ -0.03 |
| sky hue range | renderer | 60.00 | 20.00 | 33% | ❌ GAP | 0 |
| palette breadth | art | 15 | 6 | 40% | ❌ GAP | 0 |
| detail surviving at distance | art | 15.61 | 8.70 | 56% | ❌ GAP | ▼ -0.00 |
| hue diversity | art | 3.90 | 2.18 | 56% | ❌ GAP | ▼ -0.00 |
| distant silhouette | renderer | 22.33 | - | – | ⏸ ยังวัดไม่ได้ | – |
| atmospheric perspective (detail) | renderer | 0.81 | 1.20 | 147% | 📎 advisory | ▼ -0.01 |
| detail per area | art | 58.63 | 37.30 | 64% | ✅ ok | ▼ -0.28 |
| flat/featureless area ↓ | art | 15.82 | 24.71 | 156% | ✅ ok | ▲ -0.34 |
| tonal spread | renderer | 32 | 21 | 66% | ✅ ok | ▼ -1.00 |
| dynamic range | renderer | 187.71 | 127.66 | 68% | ✅ ok | ▼ -0.90 |
| local contrast r3 | art | 18.74 | 12.85 | 69% | ✅ ok | ▼ -0.16 |
| sky presence | world | 21.85 | 18.16 | 83% | ✅ ok | ▼ -0.03 |
| atmospheric perspective (sat) | renderer | 1.13 | 1.00 | 89% | ✅ ok | ▼ -0.01 |
| saturation | renderer | 57.56 | 75.20 | 131% | ✅ ok | ▼ -0.10 |
| sky blown to white (L>245) ↓ | renderer | 5.09 | 0.00 | 0% | ✅ ok | 0 |
| clipped highlights ↓ | renderer | 10.30 | 0.20 | 2% | ✅ ok | ▲ -0.57 |

**`↓` ต่อท้ายชื่อแกน = แกนที่ "น้อยกว่าดีกว่า"** — คอลัมน์ `% ของ REF` เป็น *ค่าดิบเทียบค่าดิบ* ไม่ใช่คะแนน ⇒ แกน ↓ ที่ได้ 2% คือ **ดี** (เราเหลือ 2% ของสิ่งที่ REF มี)  
คอลัมน์ Δ: **▲ = ขยับเข้าหาภาพอ้างอิง · ▼ = ถอยห่าง** (ตัวเลขคือส่วนต่างดิบ จึงเป็น `▲ -36.44` ได้ ถ้าแกนนั้นน้อยกว่าดีกว่า) · `–` = รอบก่อนไม่มีตัวเลขให้เทียบ

`ok` = ≥ 60% ของ REF (ตามทิศของแกน) · `GAP` = ต่ำกว่านั้น · `over` = **ไกลจาก REF เกิน 167%** (advisory เฉย ๆ ไม่ตัด FAIL — มีไว้กันคนอ่านตัวเลขไกลจาก REF แล้วนึกว่า "ดีกว่า ref") · `advisory` = **วัดแล้วแต่ไม่ตัดสิน** (ภาพอ้างอิงเองไม่รองรับเส้นตัดของแกนนั้น) · `ยังวัดไม่ได้` = เครื่องมือ **ปฏิเสธที่จะวัด** ไม่ใช่วัดแล้วได้ศูนย์

<details><summary>ทำไม **atmospheric perspective (detail)** ถึงเป็น advisory ไม่ใช่ด่าน</summary>

`depth_micro_ratio` = `micro(near) / micro(far)`. บนภาพอ้างอิงของ CEO ค่านี้ = **0.81** (near 12.67 / far 15.61) — คือ **แถบไกลของภาพอ้างอิงมีรายละเอียดมากกว่าแถบใกล้**. แปลว่า *ภาพอ้างอิงเองไม่ได้แสดงสิ่งที่แกนนี้ตั้งชื่อไว้* การเอาเฟรมเราไปเทียบกับ 0.81 จึงวัดว่า "ฉากวางของรกไว้ตรงไหน" ไม่ได้วัดว่าหมอกทำให้ระยะไกลนุ่มลงไหม — และ "ทำให้ได้ 60% ของ 0.81" เป็นบาร์ที่เฟรมไหนก็ข้ามได้ฟรี. กติกาข้อ 7: **เส้นตัดที่ภาพอ้างอิงเองไม่รองรับ = phantom target** เอกสารนี้จ่ายค่ามันมาแล้ว 3 ครั้ง.

ฝาแฝดของมัน `depth_sat_ratio` **ยังเป็นด่านอยู่** เพราะ REF ได้ 1.13 > 1 (ใกล้อิ่มสีกว่าไกล) = รองรับชื่อแกนตัวเอง. จะเลื่อน `depth_micro_ratio` กลับมาเป็นด่านได้ ต้องมี ref กลางแจ้งที่ค่านี้ > 1 แล้วบันทึก control ไว้ก่อน

</details>

## ที่ยังวัดไม่ได้ — ปฏิเสธเพราะอะไร และใครปลดล็อก

| แกน | เหตุผลที่เครื่องปฏิเสธ (คำต่อคำจาก grader) |
|---|---|
| distant silhouette (renderer) | `sky unlit (37% below L=10) - contrast against a black void is degenerate` |

**ปลดล็อกยังไง:** แกนพวกนี้จะกลับมาวัดได้ **เอง** ทันทีที่เงื่อนไขข้างบนหาย — ไม่มีใครต้องแก้เครื่องวัด (พิสูจน์แล้ว: เฟรมที่ฟ้ามีโทนวัด `distant silhouette` ได้ตามปกติ)

`distant silhouette` ต้องการฟ้าที่ *มีโทน*: ตอนนี้ `sky_void_pct` = **36.58%** (ต้อง ≤ 20) และ `sky_blown_pct` = **0.00%** (ต้อง ≤ 20)

## สุขภาพเครื่องวัด (ต้องดูก่อนเถียงเรื่องเลข)

| | REF | เฟรมเรา |
|---|---:|---:|
| sky mask กินพื้นที่ | 21.85% | 18.16% |
| คอลัมน์ที่มีเส้นขอบฟ้า | 100.00% | 95.35% |
| ฟ้าดำสนิท (L<10) | 0.00% | 36.58% |
| ฟ้าไหม้ขาว (L>245) | 5.09% | 0.00% |

mask overlay เขียนออกมาทุกครั้ง (ฟ้า=น้ำเงิน · far=ชมพู · near=เขียว · เส้นขอบฟ้า=เหลือง) — **ดู mask ก่อนเถียงเรื่องตัวเลข**

## เฟรมอื่นในรอบเดียวกัน

| เฟรม | ผ่าน | GAP | ยังวัดไม่ได้ |
|---|---:|---:|---:|
| `_pixel_world_BEFORE.png` | 10 | 10 | 1 |

## รันซ้ำ (คำสั่งเดียว ต่อ build ใหม่ 1 ตัว)

```bash
python scripts/_pixel_artgap_grade.py _pixel_world_AFTER.png \
       _pixel_world_BEFORE.png \
       --oneshot --label "<ใครส่ง build / commit>"
```

`--oneshot` = รัน control ก่อนเสมอ (control ตก ⇒ **exit 3** และ **ไม่พิมพ์เลขให้เชื่อ**) → เกรด → เขียน JSON + mask + history → สร้างหน้านี้ใหม่  
exit: `0` ผ่านหมด · `1` มี GAP · `2` วัดครบแต่มีแกนที่ปฏิเสธจะวัด · `3` control ตก

| ไฟล์ | sha256 (12 ตัวแรก) |
|---|---|
| `docs/refs/ceo_ref_sunset_valley.jpg` (REF) | `d4491a12e14e` |
| `_pixel_world_AFTER.png` | `7251e646ee27` |
| `_pixel_world_BEFORE.png` | `a3d9b250952a` |
