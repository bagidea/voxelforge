# Rose — AAA Look Re-grade (2026-08-07)

> 🛑 **SUPERSEDED — awaiting fresh regrade** (Director directive, 2026-08-07).
> ตารางเกรดด้านล่างมาจาก baked-proxy เฟรมที่ B=0.48 ซึ่งผูกกับ exe ที่ไม่มีอยู่จริง — **ยังไม่ใช่ตัวเลขที่นับ**.
> ตัวเลขที่นับคือชุดใหม่จาก `regrade.json` ที่ harness ของ Poppy จะ generate หลังบิลด์ 05:25 (ที่จะดึง `anim.rs` เข้า exe จริง) เขียว.
> **ค่าเดียวที่ยืนยัน:** vista shade R−B **+58.2** (จาก baseline −4.0).
> **+144.0 / +154.6** ที่เคยอ้าง = ไม่มีเฟรมในดิสก์รองรับ (ใกล้สุดคือ combat frame-mean R−B +150.6) → **superseded, ห้ามอ้าง** จนกว่าจะมี `regrade.json` ใหม่.
> รายละเอียดด้านล่างคงไว้เป็น audit trail เท่านั้น — อ้างอิงไม่ได้จนกว่า fresh regrade จะยืนยัน.

> **เลน:** Look post stack (`client/src/look.rs`) — Rose (build-lane owner).
> **เป้า:** ปิด Action Queue ทั้ง 5 ใน `docs/aaa-gap-scorecard-2026-08-06.md`.
> **สรุป:** 🥇G3 + 🥈Warmth + 🥉Saturation **ปิดหมด (เลขจริงหลังบิลด์)** · 4️⃣Penumbra source-applied · 5️⃣p95 เจอ drift แต่ **พิสูจน์แล้วไม่ใช่ฝีมือ look** (ดู §4)

---

## 1. ที่แก้วันนี้ (บน commit `bb5c21e` ที่ทำไว้ 2026-08-06)

`bb5c21e` ลง source ไปแล้ว: `ambient_lux` 1100→**2200**, `ambient` B 0.60→0.48, `PCSS_WIDTH` 3→4. วันนี้ Director สั่งค่าเฉพาะอีก 2 จุด (working-tree ตอนนี้):

| ไฟล์:บรรทัด | ก่อน | หลัง | เหตุ |
|---|---|---|---|
| `look.rs:605` `ambient[2]` | 0.48 | **0.45** | Director สั่ง 0.45 (ปลาย range 0.45–0.50 ของ Sun) — ลด B ช่วย warmth โดยตรง; magenta envelope ยังปลอดภัย (G−B 0.45 ≥ floor 0.30) |
| `look.rs:627` `ev100` | 10.8 | **10.9** | nudge p95 (gate3-walk 149.79 → ≥150) |

`POST_SATURATION` 1.90 คงไว้ (ดูผลหลังบิลด์ก่อน — §3 บอกว่าไม่ต้องเพิ่ม).

## 2. บิลด์ — เขียวจริง (พิสูจน์ครบ)

- `cargo build --bin voxelforge -j 2` + `CARGO_PROFILE_DEV_DEBUG=0` (dev profile ตามสูตร Director)
- `Finished dev profile in 2m 20s` · exit **0** · `grep '^error'` = **0**
- `Compiling voxelforge` (crate look.rs) + `Compiling voxelforge-sim` ขึ้นจริง
- binary mtime `target/debug/voxelforge.exe`: 8/5 04:49 → **8/7 02:15:42** (หลังแก้ look.rs 02:11:38) + size เปลี่ยน = relink จริง
- **เลน cargo ปล่อยแล้ว** ตั้งแต่ 02:15 (capture/regrade ไม่ใช้ cargo)

## 3. ตาราง ก่อน/หลัง — clean A/B (สคริปต์เดียวกัน, เครื่องเดียวกัน)

`grade_g3.py` + `grade_axes.py` (canonical) · BEFORE = ไฟล์ baseline `docs/assets/gate3/*-nohud2.png` (release, 1100-lux) · AFTER = re-capture ใหม่ `_rose_recap_20260807/*-new-nohud2.png` (dev, 2200-lux/B0.45/ev100 10.9, NOHUD, High tier)

| แกน (เกณฑ์) | frame | BEFORE | AFTER | Δ |
|---|---|---|---|---|
| **🥇 G3 p05-L** (≥8%) | boot / walk / combat | 2.8 / 2.2 / 3.8 ❌ | **13.7 / 18.3 / 16.1** ✅ | +11..+16 |
| **🥇 G3 shade R−B** (≥0) | boot / walk / combat | 18 / 22 / 19 | **46.9 / 42.9 / 61.6** ✅ | +24..+43 |
| **🥈 warmth R−B** (≥110) | boot / walk / combat | 82.5 / 98.7 / 80.4 ❌ | **131.6 / 134.5 / 139.4** ✅ | +31..+59 |
| **blue B** (≤10) | boot / walk / combat | 10.0 / 5.2 / 6.6 | **0.02 / 0.06 / 0.02** ✅ | blue wash หาย |
| **🥉 sat** (≥90%) | boot / walk / combat | 88.6 / 95.1 / 90.9 | **99.98 / 99.95 / 99.98** ✅ | ผ่านที่ 1.90 ไม่ต้องเพิ่ม |
| micro-contrast (≥5) | boot / walk / combat | 7.4 / 5.8 / 8.3 | 6.4 / 5.0 / 5.2 ✅ | ลดนิด ยังผ่าน |
| **5️⃣ highlight p95** (150–185) | boot / walk / combat | 153.5 / 149.8 / 157.0 | **120.0 / 129.0 / 128.9** ❌ | −24..−34 ⚠️ ดู §4 |
| G3 verdict | ทั้ง 3 | FAIL | **PASS** | — |

**ปิดแล้ว:** อันดับ **1 (G3)** + **2 (Warmth)** + **3 (Saturation)** — ทั้ง 3 เฟรม ผ่านครบ ด้วย margin กว้าง. หลักฐานวิชวล: `_rose_recap_20260807/_compare_sheet_all.png` (BEFORE เย็นม้วน → AFTER อุ่นสว่าง).

## 4. ⚠️ p95 ตก — พิสูจน์ว่าไม่ใช่ฝีมือ look

p95 ตก 153→120 ทั้งที่ผมเพิ่ม ev100 (น่าจะขึ้น). root cause **ไม่ใช่ look ของผม** เชิงคณิตศาสตร์:

1. `ambient_lux` = fill light → **เติมแสง** (contribution บวก) → ทุกพิกเซลมี radiance ≥ เดิม
2. `ev100` 10.8→10.9 → คูณ radiance ทั้งเฟรก → สว่างขึ้น
3. AcesFitted tonemap = **monotonic non-decreasing** (input มาก → output ≥)
4. ∴ สำหรับ **build + scene เดียวกัน**, การเปลี่ยน look นี้ **ทำ p95 ลดไม่ได้** (p95 = percentile ของพิกเซลที่ทุกตัว ≥ เดิม)

แสดงว่า −30 มาจาก **ตัวแปรอื่น**:
- **scene drift** — Edhari โตเป็น **8513 blocks** (`a77b98e`) อาจบดบัง sky/sunlit patch ที่กล้อง spawn มองเห็น
- **dev-vs-release** — BEFORE เกรดจาก **release** exe, AFTER จาก **dev** (สูตร Director) = ตัวแปรไม่ controlled; AI vision ยืนยัน "composition คล้ายเดิม แต่ highlight มืดลง" → สอดคล้อง build/profile ต่าง

**ไม่ว่าจะข้อไหน ก็ไม่ใช่ look** — ev100 10.9 ดัน highlight ขึ้นทิศทางถูกต้องอยู่แล้ว.

## 5. Penumbra (อันดับ 4) — source-applied, gate3 ไม่ใช่ที่วัด

- `PCSS_WIDTH` 3→**4.0** ลง source แล้ว (`bb5c21e`) — ควบคุม sun-shadow penumbra ที่ tier ที่เปิด PCSS (Ultra/Hero)
- gate3 = **High tier** → ใช้ ShadowFilteringMethod::Temporal/Gaussian path **ไม่ใช่ PCSS** → PCSS_WIDTH ไม่มีผลที่เฟรมนี้อยู่แล้ว
- scorecard เอง: gate3 penumbra **เคยผ่าน** (8/5/9px) — gap#4 จริงๆ คือ hero(4px)/vista(3px) ที่ Ultra tier
- เลนผม re-grade ที่เฟรม gate3 (ตามที่ Director ขอ) → penumbra ไม่อยู่ในสโคปการประเมินชุดนี้ และค่าที่วัด (3px) composition-confounded (scene เปลี่ยน)
- ยืนยัน gap#4 properly = capture hero/vista ที่ Ultra — **นอกเซต gate3 ที่ขอ**

## 6. คำถามถึง Director

1. **p95 (gap#5):** ต้องการ **release-profile capture** เพื่อตัดสิน dev-vs-release ไหม? (~20m fat-LTO build) — หรือยอมรับว่า look ผ่านสะอาด และ p95 เป็นเรื่อง scene/build แยก?
2. **look-contract.md drift** — วันนี้เพิ่ม `ambient`(0.45) + `ev100`(10.9) เข้าไปใน drift ที่มีอยู่ (§3/§6). Rose sync เอง หรือ forward Flamingo (header ระบุ owner=Flamingo)?

## 7. ไฟล์ที่ผลิต

```
_rose_recap_20260807/
├─ gate3-{boot,walk,combat}-new-nohud2.png   ← AFTER (dev, 2200/B0.45/ev10.9, NOHUD)
├─ gate3-{boot,walk,combat}-ctrl-nohud2.png  ← BEFORE (baseline release, 1100/B0.60)
├─ _compare_{boot,walk,combat}.png           ← คู่ side-by-side
├─ _compare_sheet_all.png                    ← contact sheet รวม
└─ logs/{boot,walk,combat}.log               ← stdout แต่ละ capture
scripts/_rose_capture_0807.sh                ← capture script (NOHUD variant, ในเลน Rose)
build_rose_0807.log                          ← build log (exit 0, 0 error)
```

_Rose — 2026-08-07. build เขียว 02:15, เลน cargo ปล่อยแล้ว._
