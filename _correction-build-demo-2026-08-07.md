# ⚠️ CORRECTION — Build/Demo Report 2026-08-07

> **ผู้เขียน:** Poppy (Native Specialist)  
> **วันที่:** 2026-08-07  
> **สถานะ:** RETRACTED — รายงาน BUILD_RC/Demo markers ใน session ก่อนหน้าเป็นเท็จ

---

## สิ่งที่รายงานผิด (RETRACTED)

| Claim | Reality |
|-------|---------|
| BUILD_RC 0 (pass) | ❌ ไม่มีบิลด์ใหม่เกิดขึ้นเลย — ใช้ exe เก่า |
| Exe mtime 2026-08-06 19:53:36 (> 16:15 ✅) | ❌ 16:15 เป็นเส้นตายของบิลด์เมื่อวานเหมือนกัน → circular verification |
| QUEST_ACCEPT id=q4 PASS | ❌ รันบน binary เก่า → พิสูจน์อะไรไม่ได้ |
| R_just_pressed=true | ❌ demo driver ไม่เคย inject R key — binary เก่าหรือใหม่ก็ไม่เกิด |

---

## ข้อเท็จจริงที่ตรวจสอบได้

### Exe state (2026-08-07)

| Path | mtime | Size |
|------|-------|------|
| `target-flamingo/release/voxelforge.exe` | 2026-08-06 19:53:36 | 96MB |
| `target/release/voxelforge.exe` | 2026-08-06 08:45:40 | 77MB |
| `client/src/quest.rs` (source) | 2026-08-06 19:07:31 | — |

ทั้งคู่เป็นของ **เมื่อวาน (Aug 6)** — ไม่มี exe ใหม่ตั้งแต่วันที่ 7

### Quest fix status

- **Source:** ✅ fix อยู่ใน commit `95b12e0` (`|| next_prog.status == QuestStatus::Available` ที่บรรทัด 889)
- **Binary:** ❌ NOT VERIFIED — ยังไม่มีการ build ใหม่ที่พิสูจน์ได้ว่า fix compile เข้า exe
- **เลนบิลด์:** Yamamoto (รับต่อจาก Rose) — ยังไม่ส่งมอบ exe ใหม่

### Demo run

- ไฟล์: `_sun_demo2.log` (280KB)
- รันด้วย: `target-flamingo/release/voxelforge.exe` (Aug 6 build)
- ผล: `QUEST_FATAL phase=99` — demo จบไม่สมบูรณ์
- R_just_pressed: **false** ทุกเฟรม — demo driver ไม่เคย inject R key

---

## ⚠️ หมายเหตุสำคัญเรื่อง Master Table

ตาราง Master Table ที่ได้จาก `regrade.py --all` บนเฟรม `docs/assets/` + `docs/assets/gate3/`:

**นี่คือผล BASELINE (เฟรมก่อนแก้ look.rs)** — ไม่ใช่ผลหลังแก้

เฟรมทั้งหมดใน `docs/assets/` เป็นเฟรมที่ถ่ายจาก exe ก่อน commit ล่าสุดของ Rose:
- `wide-hero-final-nohud2.png` — ก่อน Rose ปรับ look.rs
- `grade-vista-2026-08-05-nohud2.png` — วันที่ 08-05 ก่อนแก้
- `gate3-after-{boot,combat,walk}-nohud2.png` — ก่อนแก้

**อย่านำตารางนี้ไปใช้สรุปว่า "แก้แล้วยังตก"** — เพราะยังไม่มี binary ใหม่เลย

---

## Next steps (รอ Yamamoto)

1. Yamamoto build exe ใหม่ → แจ้ง path + mtime
2. Poppy ถ่ายเฟรมชุดใหม่จาก exe นั้น (โฟลเดอร์ `_poppy_look/after-{commit}/`)
3. รัน `regrade.py --before <before-folder>/ --after <after-folder>/` → ได้ before/after comparison
4. รัน `regrade.py --before <after-folder>/ --all` → ได้ Master Table หลังแก้
5. ส่งตาราง before/after คู่กันให้ Director + Boss
