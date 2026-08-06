# Rose — `place_block` proof checklist

> เกณฑ์ว่า exe ใหม่ (จาก sun) "พิสูจน์แล้ว" จริงหรือไม่ — อ้างอิง marker string จริงจาก
> `client/src/quest.rs` + `scene.rs`, ไม่ใช่การเดา phase. เอาไว้กันเถียงทีหลัง.
> owner: Rose (prover lane). baseline ห้ามลบ.

## ข้อ 0 — STALENESS GATE (เช็คก่อนทุกอย่าง, ก่อนข้อ 1) ⚠️
> **ก่อนนับผลรันใดๆ** (PASS/FAIL/phase อะไรก็ตาม) ต้องพิสูจน์ก่อนว่า exe ที่จะรัน ไม่ใช่ binary ค้าง
> — ไม่งั้นผลลัพธ์เป็นขยะ (เคยเห็น "ล้ม phase 2 ติดเสา" ทั้งที่ source แก้เสาออกไปแล้ว)
> และห้ามนำไปสรุปว่า nav/phase พัง.

**เงื่อนไข:** `mtime(exe)` ต้อง **ใหม่กว่า** `mtime` ล่าสุดของ **ทุกไฟล์** ใน `client/src/*.rs`.
ถ้า `mtime(exe) < mtime(newest client/src/*.rs)` ⇒ ตัดสิน **INVALID (stale binary)** ทันที —
**ห้ามรายงานเป็น PASS หรือ FAIL เด็ดขาด** (รายงานว่า INVALID อย่างเดียว + รอ exe ใหม่).

**ต้องบันทึกลงในผลรันทุกครั้ง:**
- `git rev-parse --short HEAD` (commit short hash ตอนรัน)
- dirty flag = จำนวนไฟล์ใน `git status --porcelain` + รายชื่อไฟล์ `client/src/*.rs` ที่ `M`

**คำสั่งตรวจ (reproducible):**
```powershell
$vf="E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
$exe=Get-Item "$vf\_rose_rkey_proof.exe"
$src=Get-ChildItem "$vf\client\src\*.rs" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Push-Location $vf; $head=git rev-parse --short HEAD; $dirty=git status --porcelain; Pop-Location
"STALE=$($exe.LastWriteTime -lt $src.LastWriteTime) EXE=$($exe.LastWriteTime.ToString('MM-dd_HH:mm')) SRC=$($src.Name)@$($src.LastWriteTime.ToString('MM-dd_HH:mm')) HEAD=$head DIRTY=$($dirty.Count)"
```

**ตัวอย่างผล (รอบ baseline ปัจจุบัน, verify ดิสก์แล้ว 2026-08-06):**
```
STALE=True  EXE=08-06_06:10  SRC=look.rs@08-06_06:48  HEAD=f358809  DIRTY=4
```
exe 06:10 เก่ากว่า source (quest.rs 06:47, look.rs 06:48) ⇒ **INVALID** — อย่าใช้ผลรันนี้สรุปอะไร
(Source ตอนนี้ `POST_LANE_Z=8.5` quest.rs:1493 + Shiba ลบเสา (43,7)+ขอบ(41,3,9) ไปแล้ว —
exe ยังเดิน z=8.0 แบบเก่าจึง "ติด column (43,7)" ที่จริงไม่มีแล้ว).

## หลักการ (กันโกง — หัวใจของการพิสูจน์)
1. ทุกบรรทัด `=> PASS` ต้องมาจาก **stdout ของตัวเกม exe จริง** (เกมพิมพ์เองผ่าน
   `println!` ใน `quest.rs`/`scene.rs`) — **ห้าม** driver/script พิมพ์ PASS แทนเกม
   (นี่คือจุดที่ baseline proof ของ Rose จับได้)
2. ไฟล์ log ต้องเป็น "รันจริง" = มี bevy boot log คละอยู่ด้วย
   (`SystemInfo ...` / `AdapterInfo ...` / `Creating new window Voxelforge`) — ไม่ใช่มีแค่บรรทัด PASS โผล่มาเฉยๆ
3. marker ต้องเรียงตาม phase **0→1→2→3→4** (หมายเลขบรรทัดเพิ่มขึ้นเรื่อยๆ) — ห้ามกระโดดข้าม phase
4. **ห้ามมี** `QUEST_FATAL phase=99` และห้ามมี `=> FAIL` ใดๆ ในไฟล์

## baseline = known-bad (ห้ามลบ / ห้าม build ทับ)
> ⚠️ **ผล PHASE 2 รอบ baseline = INVALID (stale binary)** — exe mtime 08-06 06:10 เก่ากว่า
> source (quest.rs 06:47 `POST_LANE_Z=8.5` + Shiba ลบเสา (43,7)+ขอบ(41,3,9) ไปแล้ว; HEAD=f358809, dirty=4).
> จับได้ค่าจริงว่า **build เก่า** ติด z=8.0 ใช่ — แต่ **ห้ามใช้สรุปว่า nav ยังพัง / column(43,7) ยังขวาง**
> เพราะ lane นั้น source แก้แล้ว. baseline เก็บไว้เป็น known-bad เฉยๆ รอ exe ใหม่จาก sun.
- exe: `_rose_rkey_proof.exe` — source label **f3f7b17b** (commit short hash),
  on-disk sha256 `90ab89b30fc509e5…`, **80,492,032 bytes**
- รันด้วย: `--quest-demo` (gate `cfg.quest_demo`, quest.rs:1590; flag main.rs:154)
- capture ไว้แล้ว: `_rose_placeblock.out.log` + `_rose_placeblock2.out.log`
- **ล้มที่ phase 2 ไม่ใช่ phase 4** — ทั้งสองรอบเหมือนกัน **[⚠️ INVALID: stale binary — ไม่ใช่หลักฐานมัดตัว column(43,7), ดู ข้อ 0]**:
  ```
  QUEST_WALK_GUARD_POST timeout x=42.7 z=8.0 hp=30 => FAIL
  QUEST_FATAL phase=99
  ```
  x≈42.7,z≈8.0 = ติด **column (43,7)** (quest.rs:1745 เตือนไว้) → baseline **ไม่เคยถึง phase 4 เลย**
- ฉะนั้น exe ใหม่ต้องไข chain phase 0→1→2→3→4 ให้ครบ จึงจะเห็น `=> PASS (place_block)`

## แผนที่ phase → marker (จาก quest.rs จริง)
| phase | ทำ | marker ที่ต้องเห็น (ตัวอย่าง) |
|---|---|---|
| spawn | เกิดบนพื้น | `SPAWN_GROUND col=(32,32) top=0 feet=1 => PASS` (scene.rs:307) |
| 0 | เดินไป gate | `QUEST_WALK_TO_GATE z=.. => reached gate_square` → `QUEST_CHECK q1_embers status=..` |
| 1 | คุย Maren (E/Enter/5) | `QUEST_ACCEPT id=q3_gatekeeper => PASS` |
| 2 | เดินไป guard_post | `QUEST_WALK_GUARD_POST reached x=.. z=..` — **ผ่าน column (43,7) ได้** |
| 3 | ฆ่า Garren | `QUEST_STAGE_COMPLETE .. o3_defeat => PASS (defeat)` → `QUEST_COMPLETE id=q3_gatekeeper => PASS` |
| **4** | **q4: place_block + interact×2** | **`QUEST_STAGE_COMPLETE qid=q4_what_walls_remember oid=o1_build => PASS (place_block)`** (quest.rs:1364) |
|   |   | `.. oid=o2_ledger => PASS (interact ..)` → `.. oid=o3_offering => PASS (interact ..)` |
|   |   | `QUEST_COMPLETE q4_what_walls_remember => PASS (all objectives done)` (quest.rs:1838) |
| 5 | q5 (optional ยืนยันจบ act1) | `QUEST_COMPLETE q5_sigil_that_knew_you => PASS` + `QUEST_FLAG act1_complete => PASS` |
| 99 | ล้ม | `QUEST_FATAL phase=99` (quest.rs:1944) — **ห้ามปรากฏ** |

## exe ใหม่ต้องผ่าน — ข้อ 0 (STALENESS GATE) + ข้อ 1–5 จึงนับ "พิสูจน์แล้ว"
**ข้อ 0 ผ่านก่อน** (`mtime(exe) > newest client/src/*.rs`, STALE=False; ดูด้านบน) — ไม่งั้นที่เหลือไม่นับ (INVALID). แล้วจึงเช็ค 1–5:
1. **binary ต่างจาก baseline** — sha256 ≠ `90ab89b3…` (เหมือนเป๊ะ = build เดิมที่ล้ม)
2. **รันจริง** — `_rose_rkey_proof.exe` (ตัวใหม่) `--quest-demo`, redirect stdout ลงไฟล์ใหม่
   (เช่น `_rose_placeblock_NEW.out.log`); Rose รันเอง capture เอง ไม่ใช้ log ที่คนอื่นส่งมา
3. **เห็น sequence phase 0→4** ครบตามตาราง โดยเฉพาะบรรทัด:
   `QUEST_STAGE_COMPLETE qid=q4_what_walls_remember oid=o1_build => PASS (place_block)`
   และต้องมี marker ของ phase 0/1/2/3 มาก่อนหน้า (เลขบรรทัดน้อยกว่า)
4. **ไม่มี FAIL/FATAL** เลย — ทั้ง `QUEST_FATAL phase=99`, `QUEST_WALK_GUARD_POST timeout … => FAIL`,
   `QUEST_STAGE_COMPLETE q4 o1_build => FAIL (timeout)`, `QUEST_KILL … => FAIL`
5. (ยืนยันจบ act1) phase 5 PASS ด้วย — ถ้าได้ถึงนี่ = chain เต็ม

## คำสั่งตรวจ (reproducible — ใครรันก็ได้คำตอบเดียวกัน)
**(ก่อน) STALENESS GATE — ต้อง STALE=False ก่อนจึงรันต่อ; STALE=True ⇒ หยุด รายงาน INVALID:**
```powershell
$vf="E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
$exe=Get-Item "$vf\_rose_rkey_proof.exe"
$src=Get-ChildItem "$vf\client\src\*.rs" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Push-Location $vf; $head=git rev-parse --short HEAD; $dirty=git status --porcelain; Pop-Location
"STALE=$($exe.LastWriteTime -lt $src.LastWriteTime) EXE=$($exe.LastWriteTime.ToString('MM-dd_HH:mm')) SRC=$($src.Name)@$($src.LastWriteTime.ToString('MM-dd_HH:mm')) HEAD=$head DIRTY=$($dirty.Count)"
```
```bash
cd "<Voxelforge>"
LOG=_rose_placeblock_NEW.out.log   # หรือชื่อ log ที่ Rose capture ตอนถูกเรียกกลับ

# (ก) ดู sequence phase เรียบ (กรอง VFX noise ออก) — ต้องเห็น 0→1→2→3→4 ไล่ลงมา
grep -nE '^(SPAWN_GROUND|QUEST_(WALK_TO_GATE|CHECK|ACCEPT|WALK_GUARD_POST|KILL|STAGE_COMPLETE|COMPLETE|FLAG|FATAL))' "$LOG"

# (ข) บรรทัดเป้าหมาย present จริง จาก stdout เกม — ต้อง = 1
grep -c 'QUEST_STAGE_COMPLETE qid=q4_what_walls_remember oid=o1_build => PASS (place_block)' "$LOG"

# (ง) ห้ามมี FAIL/FATAL เลย — ต้อง = 0
grep -cE 'QUEST_FATAL phase=99|=> FAIL' "$LOG"

# (จ) ยืนยันเป็นรันจริง (มี bevy boot) — ต้อง > 0
grep -c 'Creating new window Voxelforge' "$LOG"
```

## คำตัดสิน
- **INVALID (stale binary)** = `mtime(exe) < mtime(newest client/src/*.rs)` — exe ค้าง (ข้อ 0 ตก).
  **ห้าม** รายงานเป็น PASS หรือ FAIL; บันทึก HEAD + dirty flag แล้วรอ exe ใหม่.
  *(นี่คือสถานะ baseline รอบนี้ — exe 06:10 < source 06:47–06:48, HEAD f358809)*
- **PASS (พิสูจน์แล้ว)** = ข้อ 0 ผ่าน + ข้อ 1–4 ครบ (เห็น `=> PASS (place_block)` จากเกมจริง, เรียง phase ถูก,
  ไม่มี FATAL/FAIL, binary ต่างจาก baseline). ข้อ 5 = bonus ยืนยันจบ act1.
- **ไม่นับ PASS** เมื่อ: บรรทัด PASS มาจาก driver ไม่ใช่ stdout เกม / ขาด marker phase ก่อนหน้า /
  ยังมี FATAL หรือ FAIL / binary เหมือน baseline เป๊ะ / log ไม่มี bevy boot (สงสัยปลอม)
