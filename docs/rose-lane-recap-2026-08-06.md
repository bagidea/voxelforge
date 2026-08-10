# Rose Lane Recap — 2026-08-06 (19:2x ICT)

> Owner: **Rose** (Look post stack + place_block prover).
> เขียนระหว่าง **build lock ของ Sun** (Director: "ห้ามรัน cargo จนกว่าจะบอก") —
> ทุกข้อนี้เป็นงาน read-only / non-compile. เป้าคือให้ทีมเห็นตำแหน่งงานตอนนี้จากดิสก์จริง
> ไม่ใช่จาก scorecard ที่เกรดตอนเช้า.

## 0. Snapshot (verify ดิสก์เอง, ไม่ใช่ความจำ)

| อะไร | ค่าจริง |
|---|---|
| branch | `poppy/native-only` |
| HEAD | `31f1d71` (combat/anim/vfx/edhari — เลนคนอื่น) |
| dirty | 26 ไฟล์ (ส่วนใหญ่ = character art + docs ของเพื่อน; `client/src` ที่ dirty มีแค่ `quest.rs`) |
| **cargo/rustc/link รันอยู่** | **0 process** (observe `Get-Process`, ไม่ได้รัน cargo) |
| exe baseline `_rose_rkey_proof.exe` mtime | **2026-08-06 06:10:09**, 80,492,032 bytes |
| source ใหม่สุดใน `client/src` | `quest.rs` @ **19:07:31** (Sun แก้วันนี้บ่าย) |
| **STALENESS GATE** | **STALE = True** (exe 06:10 < source ทุกตัวที่แก้วันนี้) ⇒ exe baseline **INVALID**, ห้ามใช้สรุปอะไร |

> ⚠️ exe baseline 06:10 เก่ากว่าแม้แต่ look fix commit `bb5c21e` (08:03) —
> ไม่ได้รวมการแก้ G3/Warmth/PCSS เลย. exe ใหม่ที่ Sun build อยู่จะรวมทั้งหมดนี้
> (look fix + haze refit + quest patches + scene/anim/vfx ของเลนอื่น).

---

## 1. Lane: Look post stack (`client/src/look.rs`) — เจ้าของ Rose

**AAA Gap Scorecard (`docs/aaa-gap-scorecard-2026-08-06.md`, Sun เกรดเช้า) ชี้ gap 5 ตัวเข้าเลนนี้ทั้งหมด.**
แต่ Sun เกรดจาก frame/exe ก่อน commit `bb5c21e` — ดิสก์ตอนนี้เป็นดังนี้:

| Gap (อันดับ Sun) | ค่าที่ Sun อ้าง (เก่า) | ดิสก์จริงตอนนี้ | commit | สถานะ |
|---|---|---|---|---|
| 🥇 G3 (interior crushed) | `ambient_lux 1100` → แนะนำ 2000+ | **`ambient_lux: 2200.0`** (look.rs:602) | `bb5c21e` | ✅ แก้ใน source |
| 🥇 G3 hue | `ambient B 0.60` → แนะนำ 0.45–0.50 | **`ambient: [0.96,0.90,0.48]`** (look.rs:601) | `bb5c21e` | ✅ แก้ใน source (B 0.60→0.48) |
| 🥈 Warmth R−B cold | `TEMPERATURE 0.05` (ceiling) → ambient lux+lower-B | `TEMPERATURE: 0.05` (look.rs:404) + ambient lift ข้างบน | `bb5c21e` | ✅ แก้ผ่าน ambient (ปุ่มเดียวกับ G3, ROI สูงสุดตามที่ Sun เล็ง) |
| 🥉 Saturation washed | `POST_SATURATION 1.90` → rebuild ยืนยันก่อนเพิ่ม | `POST_SATURATION: 1.90` (look.rs:450) | — | ⏳ คงค่า (Sun เองทัก "อาจพอแล้ว" — รอ regrade หลัง ambient lift ก่อนตัดสินใจ) |
| 4 G4a penumbra | `PCSS_WIDTH 3.0` → แนะนำ 3→4 | **`PCSS_WIDTH: 4.0`** (look.rs:354) | `bb5c21e` | ✅ แก้ใน source |
| 5 Highlight p95 | `ev100 10.8` → 10.9 หรือ ignore | `ev100: 10.8` (look.rs:617, Hour::GOLDEN) | — | ⏳ คงค่า (Sun เองทัก "gap 0.21 มองไม่เห็น — หรือ ignore") |

**บทสรุปเลน look:** gap อันดับ 1, 2, 4 **ลง source + committed แล้ว** (commit เดียว `bb5c21e`).
อันดับ 3, 5 จงใจคงค่าไว้รอดูผลจาก ambient lift ก่อน. **สิ่งเดียวที่ขาด = rebuild + re-capture + re-grade** ยืนยันว่า ambient_lux 2200/B 0.48 กวาด G3+Warmth จริง.

### ⚠️ Drift ระหว่าง `look.rs` (source of truth) กับ `look-contract.md` (ต้อง sync)
ตรวจ params ทั้งหมดแล้ว — **ทุกค่าที่ไม่ตรงเป็นการแก้ที่ตั้งใจ** (committed + commented ที่ตัว),
ไม่มีค่าแปลก/accident. ปัญหาคือ doc ล้าสมัย. look-contract กฎเดียวคือ "ตัวเลขเป็นสำเนาของ look.rs"

| § | param | contract เขียน | look.rs จริง | commit (ตั้งใจ?) |
|---|---|---|---|---|
| §1 | `FOG_COLOR_DAY` | `(0.60,0.72,0.88)` ฟ้าเทา | **`(0.94,0.66,0.26)`** ส้มทอง | `bd3cde5` "warmth was never the saturation knob's job" — เอา warmth จากสีหมอก ไม่ใช่ sat ✅ |
| §3/§6 | `ambient_lux` Golden | `1100` | **`2200`** | `bb5c21e` fix G3 crushed-shade ✅ |
| §3/§6 | `ambient` Golden | `(0.96,0.84,0.66)` | **`(0.96,0.90,0.48)`** | `bb5c21e` (G lifted, B drained 0.60→0.48) ✅ |
| §3 | `PCSS_WIDTH` | ไม่ระบุตัวเลข | `4.0` (เคย 3.0) | `bb5c21e` ✅ |

**ค่าที่ตรง contract ทุกตัว (verify ผ่าน):** `FOG_START 112` / `FOG_END 320` / `FOG_SUN_GLOW (1.00,0.85,0.60)` / `FOG_SUN_EXPONENT 30` / `Bloom{intensity 0.18, prefilter threshold 1.0, softness 0.4}` / `ev100 10.8` / `TEMPERATURE 0.05` / `POST_SATURATION 1.90` / tonemap `TonyMcMapface` / DoF stripped.
`HAZE_START 20` / `HAZE_FULL 150` = refit `f358809` (contract ไม่ระบุ HAZE ตัวเลข พูดแค่ FOG).

**ถาม Director:** look-contract header ระบุ owner = **Flaminggo** — Rose sync เอง (เป็นสำเนาของเลนตัวเอง + คนแก้ look.rs ควร sync ตามกฎ)
หรือ forward ให้ Flamingo? ยังไม่แตะ doc จนกว่าจะได้คำตอบ.

---

## 2. Lane: place_block proof (`q4_what_walls_remember o1_build`) — prover = Rose

- เป้า marker (เกมพิมพ์เอง, quest.rs:1364):
  `QUEST_STAGE_COMPLETE qid=q4_what_walls_remember oid=o1_build => PASS (place_block)`
- **baseline ปัจจุบัน INVALID** (ข้อ 0 STALENESS GATE: exe 06:10 < source 06:47–19:07).
  baseline ล้มที่ phase 2 (`QUEST_WALK_GUARD_POST timeout x=42.7 z=8.0 => FAIL` → `QUEST_FATAL phase=99`)
  — แต่นั่นคือ binary เก่า (z=8.0) ที่ติด "column (43,7)" ที่ **Shiba ลบไปแล้ว** + Sun แก้ `POST_LANE_Z=8.5` (quest.rs:1493) แล้ว.
  ⇒ **ห้ามใช้สรุปว่า nav/column(43,7) ยังพัง** — เป็น known-bad เฉยๆ.
- **พร้อมทำงานทันทีที่ได้ exe ใหม่**: checklist `_rose_placeblock_proof_checklist.md` (ข้อ 0 STALENESS GATE + ข้อ 1–5),
  Rose รัน `_rose_rkey_proof.exe --quest-demo` เอง capture เอง, ตรวจ sequence phase 0→4 + ไม่มี FAIL/FATAL.
- blocker = exe ใหม่จาก Sun (Director จะเรียก Rose กลับเมื่อ Sun ปล่อยเลน).

---

## 3. Lane: Distance haze — ปิดแล้ว

- G7b Linear ramp (`HAZE_START=20` / `HAZE_FULL=150`, refit commit `f358809` จาก 250→150) = shipped + แข็งแรง:
  whole-frame net 3.32 / %≥5 28.3, near-band 0.10 (dead zone ทำงาน) → A2 PASS.
- haze เป็น differential A/B gate ชน baseline ตัวเอง (metric เป้า 3.5/15% = PHANTOM, autopsy แล้ว).
- **quality axis ของ distance haze = FLAT ทุก tier** (look.rs:880 "One constructor for every tier")
  ⇒ อย่า re-shoot quality sweep เพื่อ haze. ไม่มีงานค้างในเลนนี้.

---

## 4. Plan เมื่อ build lock ปล่อย (Sun ส่ง exe ใหม่ → Director เรียก Rose)

รอบเดียวกัน (exe ใหม่รวมทั้ง look fix + quest fix) ทำสองงานพร้อมกัน:

1. **place_block proof** — ข้อ 0 STALENESS GATE ต้อง STALE=False ก่อน; รัน `--quest-demo` capture ใหม่;
   ดู `=> PASS (place_block)` + sequence phase + ไม่มี FAIL/FATAL. (ใช้ `-j 2` ตามที่ Director สั่ง.)
2. **Look re-grade** — re-capture vista + gate3 frames ด้วย exe ใหม่ (รวม ambient_lux 2200/B 0.48);
   รัน `grade_g3.py` + `grade_axes.py` + `grade_beauty.py` เทียบเดิม → ดูว่า G3 p05-L ≥8% และ Warmth R−B ≥+110 ผ่านหรือไม่.
3. ถ้า G3+Warmth ผ่าน → ตัดสิน Saturation (อาจไม่ต้องเพิ่มจาก 1.90).

**สคริปต์เปรียบเทียบภาพเตรียมไว้แล้ว** (`_rose_haze_delta.py`, `_rose_compare_sheet.py`) — refine ใน task คู่ขนานนี้.

---

## 5. คำถามถึง Director

1. **look-contract.md drift** (§3/§6 ambient_lux 1100→2200, ambient color): Rose sync เองหรือ forward Flamingo?
2. exe ใหม่จาก Sun มาแล้วแจ้ง Rose — ผมจะรันทั้ง place_block proof + look re-grade ในรอบเดียว (เลนผม, `-j 2`).

_Rose — 2026-08-06, build lock ของ Sun อยู่, non-compile work._
