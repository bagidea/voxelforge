# Look post-stack audit + perf — Rose (implementer), 2026-08-04

> เลน Look post stack (`client/src/look.rs`, owner **rose**). เป้า: ไล่ post stack เทียบ
> `look-bible.md`, ปิดช่องว่างกับ `golden-beauty-shot-ref.png`, ยืนยันเฟรมเรตเล่นได้บนเครื่องนี้.
> คู่กัน: [`look-audit-2026-08-03.md`](look-audit-2026-08-03.md) (Flamingo, มุม designer) ·
> [`look-contract.md`](look-contract.md) · [`look-tier-spec.md`](look-tier-spec.md).

## TL;DR

- **Perf (วัดจริง, GTX 1060 6GB):** post stack เอง **ไม่ใช่ปัญหาเฟรมเรต**. default High ถือ
  **119–135 fps**, แม้ Ultra (PCSS + Ultra SSAO + god ray 96-step) ยังถือ **76 fps** บนฉาก
  synthetic 64×64 @720p novsync. ด้านล่าง.
- **ช่องว่าง vs golden:** identity ตรงแล้วทุกแกน (อุ่น/เงานุ่ม/AO/bloom ไม่มี DOF) — เหลือ 2 อย่าง:
  (1) GI/bounce color-bleed เป็น ambient แบน (structural), (2) god ray หายที่ Low/Medium
  (trade-off ที่ tier-spec ยอมรับ). **ไม่ดันค่า grade บอด** (บทเรียน magenta).
- **streaming.rs B0002:** root cause ยืนยัน = `Res<World>` + `ResMut<World>` ซ้อนกัน → aliasing
  panic. **fix อยู่ใน working tree แล้ว** (streaming.rs `M`) + patch ที่ `docs/patches/`.
  เป็น P0 ของ Poppy — **ผมไม่แตะไฟล์นั้น**.
- **look.rs แก้ 1 จุดในเลน:** §7.3 — filter `With<Camera3d>` → `With<crate::OrbitCam>` ป้องกันชน
  VFX stage camera (vfx.rs:1477 จงใจไม่เอา TAA). **cargo check ผ่าน — `CHECK_EXIT=0`, 0 error** (ดู §5).

---

## 1. เฟรมเรตจริง — `voxelforge_perf` (novsync, GTX 1060 6GB)

Bin: `target/release/voxelforge_perf.exe` (built 2026-08-03 17:03 — **หลัง look.rs + หลัง fix B0002**,
จึง current). Standalone (ฉาก synthetic ของตัวเอง, ไม่ add StreamingPlugin, ไม่เข้า play) →
**วิ่งผ่าน panic ของ streaming.rs ได้**. ไม่แก้ look.rs ตามกติกา. log: `_rose_look_perf.log`.

การ์ดยืนยันตัวเองจาก log: `AdapterInfo { name: "NVIDIA GeForce GTX 1060 6GB", driver: "560.94",
backend: Vulkan }`, i5-12600K, 15.8 GiB.

| tier | median ms (2 รอบ) | fps (1000/median) | p95 ms | 1% low fps | stack vs off |
|---|---|---|---|---|---|
| **off** (baseline, look ดับ) | 5.16 | 194 | 6.3 | 131 | — |
| **Low** | 6.87 / 7.07 | 146 / 141 | 10.9 | 76–84 | +1.7 ms |
| **Medium** | 7.21 / 6.96 | 139 / 144 | 12.1 | 72–76 | +1.8 ms |
| **High** *(default ที่ผู้เล่นได้)* | 8.43 / 7.42 | 119 / 135 | 12.3 | 61–68 | +2.3–3.3 ms |
| **Ultra** | 13.11 / 13.11 | 76 / 76 | 19.0 | 37–40 | +7.9 ms |

**บทเรียนการวัด (สำคัญ):** รอบแรกของแต่ละ tier มัก**ตกค้าง** — Low ออก median 6.6 ms แต่ p95
46 ms, **Medium ออก 30.9 ms** (ช้ากว่า High ที่ทำงานมากกว่า — เป็นไปไม่ได้เชิงกล). ต้นเหตุ =
**cold shader cache** (driver คอมไพล์ shader ครั้งแรกของแต่ละโพรเซส). วัดซ้ำหลัง cache อุ่น =
กลับมา monotonic ถูกต้อง. ตัดสินด้วยกฎ look-perf-methodology §2: **mean/median > 15% = ทิ้งวัดใหม่**
(off/High/Ultra สะอาดตั้งแต่รอบแรก; Low/Medium ทิ้งรอบแรก ใช้รอบอุ่น). **สรุป: อย่าเชื่อ round-1
ของ tier ใด tier หนึ่งถ้า mean/median ห่างกัน.**

**คำตอบ "เล่นได้ไหมบน 1060":** ใช่ สบายมาก — post stack เองแม้ Ultra ยังไม่ใกล้กำแพง 16.7 ms.

⚠️ **ข้อจำกัด (look-perf-methodology §3):** ค่า absolute นี้คือราคา **post stack บนฉาก synthetic
64×64 @720p บนเครื่องนี้** — *ไม่ใช่* frame budget ของโลกจริง (draw-call/depth complexity/shadow
caster ต่างออกไป). เชื่อ **delta** ระหว่าง tier ได้; อย่าเอา median ของ `full` ไปเทียบเป้า 60fps
@1080p ตรงๆ (screen-space effect ขยาย ~2.25× เมื่อ 720p→1080p — แต่นั่นคือการประมาณ ไม่ใช่ของที่วัด).
frame budget ของโลกจริงต้องวัดจาก `--play` ซึ่งยัง **P0-gated** (streaming.rs).

---

## 2. Post stack audit vs `look-bible.md` §2 (14 ฟีเจอร์)

อ่านจาก `insert_stack` + `base_camera_look` + `apply_look_to_sun` ใน `look.rs` ตรงๆ.

| # | ฟีเจอร์ bible | สถานะในเกมจริง (look.rs) | หมายเหตุ |
|---|---|---|---|
| 1 | Directional sun + shadow map | ✅ ทุก tier | `apply_look_to_sun` + 4K atlas insert ใน look.rs |
| 2 | Soft shadow PCF/PCSS | ✅ Temporal (Gaussian ที่ Low) · PCSS Ultra เท่านั้น | High ตัด PCSS ก่อน vfog (§1) |
| 3 | Contact AO / SSAO | ✅ ทุก tier (Low→Ultra, thickness 1.45) | "ตัด=ตาย" มีครบทุก tier |
| 4 | GI / แสงอ้อม | ⚠️ **ambient แบน + vertex-AO bake** — ไม่ใช่ GI | โครงสร้างใหญ่ (ดู §3) |
| 5 | Volumetric god rays | ⚠️ High/Ultra เท่านั้น · **Low/Medium ไม่มี** | trade-off ที่ยอมรับ (§1) |
| 6 | Atmospheric fog | ✅ ทุก tier — `DistanceFog` Linear{112,320} | ซ่อน streaming radius |
| 7 | PBR material | ✅ ทุก tier — per-block material (split mesher) | `c69cc78` ต่อ render path แล้ว |
| 8 | Bloom | ✅ ทุก tier — threshold 1.0 (emissive-only) | จงใจไม่ฟุ้งทั้งเฟรม |
| 9 | Depth of Field | ❌ **ถอดถาวร** (look-contract §5) | CEO ตำหนิ "ไกลเบลอ"; เกมตั้งใจไม่มี |
| 10 | Tone-map filmic | ✅ ทุก tier — **TonyMcMapface** + ColorGrading | ไม่ใช่ AcesFitted (ดู §5) |
| 11 | Water reflection SSR | ❌ ไม่มีใน look.rs | ฉาก Edhari อาจยังไม่มีน้ำ |
| 12 | Emissive materials | ⚠️ bloom-only (ไม่ cast light) | ตรง intent Lite |
| 13 | Dust motes / particles | ❌ ไม่มี | ชั้นเสน่ห์ |
| 14 | Color grading LUT | ✅ `ColorGrading` (temp/sat/contrast/gain) | ไม่ใช่ LUT ไฟล์ แต่ทำหน้าที่เดียวกัน |

**แกน identity (bible "ห้ามตัด" #1,2,3,7,10):** ครบทุก tier ✅. ที่หายเป็นชั้นเสน่ห์ (#11,13) กับ
DOF (#9, ถอดโดยตั้งใจ) และ GI จริง (#4).

---

## 3. ช่องว่าง vs `golden-beauty-shot-ref.png`

Golden ref = **ครัวไม้ในร่ม golden-hour** (วัดจากภาพจริง: R≈255 > G≈180 > B≈100 ส้มลึก, god ray
ผ่านหน้าต่างซ้าย, shadow นุ่ม, contact AO, bloom ขอบหน้าต่าง, **ไม่มี DOF**). เกมจริง = **วิว
กลางแจ้ง Edhari** — **คนละฉาก**. จึงเปรียบเทียบ **identity** ไม่ใช่ scene-match:

| แกน identity (golden) | ในเกมจริง | ช่องว่าง |
|---|---|---|
| โทนอุ่น R>B | ✅ มาจาก **ไฟ** (key `[1.0,.84,.62]` + ambient `[.96,.84,.66]`) ไม่ใช่ grade matrix | ถูกทาง — กัน magenta บนท้องฟ้าแบน |
| เงานุ่ม + contact AO | ✅ Temporal/Gaussian + SSAO ทุก tier | — |
| bloom หน้าต่าง/โคม | ✅ threshold 1.0 | — |
| ไม่มี DOF (sharp ไกล) | ✅ ถอดถาวร | — |
| **GI/bounce color-bleed** | ⚠️ ambient แบน + vertex-AO | **เหลือ — structural** |
| god ray | ⚠️ High/Ultra เท่านั้น | Low/Medium เสียคะแนน pass 3 (ไม่ตก gate) |

**สรุป:** identity ตรงแล้ว. เหลือ 2 ช่องว่างจริง:
1. **GI color-bleed** (rubric pass 2, 18 คะแนน) — ambient uniform ทำ color bleed ไม่ได้เชิงกลไก.
   เป็นฟีเจอร์ใหญ่ (RT/DDGI หรือ baked lightmap) ไม่ใช่ one-line fix. bible เองยอม Lite ใช้
   "baked lightmap + AO probe".
2. **god ray ที่ Low/Medium** — bible อยากให้ Lite มีอย่างน้อย screen-space light shafts; ตอนนี้
   ไม่มีตัวทดแทน. tier-spec §1 จัดเป็น score-loss ไม่ใช่ gate-fail.

**⚠️ ไม่ดันค่า grade บอด:** office-memory บันทึก golden ref "R-B=+118 ส้มลึกกว่าที่เรนเดอร์".
การอยากได้ส้มขึ้นอาจไปดัน `grade::TEMPERATURE` — **ห้าม** (บทเรียน magenta 2026-08-01: grade matrix
ลากท้องฟ้าแบนไปด้วย). ความอุ่นต้องมาจากไฟ. การเซ็นค่า grade ที่ถูกต้อง **ต้องเรนเดอร์เฟรมเกมจริงแล้ววัด**
(look-tier-spec §4) — ซึ่งยังทำไม่ได้ (P0-gated). จนกว่าจะมีเฟรม ยึดค่าใน look.rs ปัจจุบัน.

---

## 4. streaming.rs — B0002 root cause (วิเคราะห์อย่างเดียว, ไม่แตะ)

**อาการ:** เกม panic ทุกครั้งที่เข้า ENTER_PLAY ตั้งแต่ commit `ca37297`.

**Root cause (ยืนยันแล้ว, ตรงกับ Flamingo audit §1):** `streaming_tick` เคยรับ resource
`crate::World` สองครั้ง — `Res<World>` + `ResMut<World>` ใน system เดียว. Bevy 0.19 ห้าม
aliasing → **B0002 panic ตอนประกอบ system** (ไม่ใช่ compile — `cargo check` เขียวสนิท, ตรงกฎ LANES.md).

**สถานะ fix:** **อยู่ใน working tree แล้ว** — `streaming.rs` ปัจจาบานมี `ResMut<World>` ตัวเดียว
(seed chunk อ่านจาก `world_res` ตรงๆ), ตรงกับ patch `docs/patches/streaming-b0002-fix.patch`
(1088 ไบต์). `M client/src/streaming.rs` ใน git status. **เป็น P0 ของ Poppy — ผมไม่แตะไฟล์นี้.**
สแกนทั้งไฟล์แล้ว: ไม่มีคู่ `Res`+`ResMut` ซ้อนเหลืออยู่. `.single()` ใน `streaming_tick`/`frustum_cull`
เป็นคนละ error (B0002 คือ aliasing resource ไม่ใช่ single-match).

**ผลกระทบต่องานผม:** block เฟรมพิสูจน์จาก `--play`. perf bin ไม่กระทบ (standalone). เมื่อ Shino
ประกาศ P0 เขียว → เรนเดอร์เฟรมเกมจริง (look-on/off/vista/ultra) แล้ววัด grade จริง + เติม §4 ของ
audit + ถ่าย before/after Gate 3.

---

## 5. look.rs — สิ่งที่แก้ในเลนวันนี้ (1 จุด)

**§7.3 `apply_look_to_cameras` filter `With<Camera3d>` → `With<crate::OrbitCam>`.**

ทำจริง ไม่ใช่สมมติฐาน: `vfx.rs:1477` spawn `Camera3d` stage camera ที่ **จงใจไม่เอา TAA**
("TAA is dropped ON PURPOSE... smears them into ghost trails") พร้อม grade/exposure/AcesFitted
ของตัวเอง. ถ้า filter เป็น `Camera3d` และรัน `--play`+`VOXELFORGE_VFX` → look lane จะ
`remove::<LookStack>()` ถอด grade ของ VFX + `insert_stack()` ยัด TAA กลับไป = พัง VFX lane เงียบๆ.
filter `OrbitCam` (กล้อง gameplay ตัวเดียว) แม่นยำกว่า. perf-neutral (กล้อง perf มี OrbitCam).

> ส่วน tier-spec §7 ข้ออื่น: §7.5 (4K atlas insert) **ทำแล้ว**; §7.1/§7.2 (grade 3 ค่า)
> **superceded** โดยการย้าย tonemapper เป็น TonyMcMapface — ค่าเดิม (0.02/1.00/1.0) จูนไว้สำหรับ
> AcesFitted จึงแรงเกินกับ Tony; look-contract §4 อธิบายไว้ ค่าใน look.rs ปัจจาบานถูกต้อง.

**compile — ผ่านแล้ว (`CHECK_EXIT=0`):** `cargo check --bin voxelforge --target-dir target`
(warm, `JOBS=1` เป็นเพื่อนบ้านที่ดีต่อ anim build ที่รันคู่ขนาน) — `Finished dev profile in 19.05s`,
**0 error** (24 warning ล้วนเป็น dead-code เดิมใน `quest.rs` ไม่ใช่ของ edit นี้), และ **anim build
ไม่ตายตาม** (วิ่งต่อจนจบของมัน). diff ของ `look.rs` = doc comment + เปลี่ยน filter 1 token
(13+/3-) ไม่มีแก้พวง. log: `_rose_check.log`.

> (บันทึกกระบวนการ: ครั้งแรกใช้ background `_rose_wait_check.sh` รอ 0-cargo — แต่มันถูก kill
> ตอน session ก่อนออกก่อน slot ว่าง เลยไม่มี `CHECK_EXIT`. ครั้งนี้รัน warm check ตรงๆ คู่ขนาน
> anim build ตามพอลิซี LANES.md ที่รับรอง lane `cargo check` คู่ขนาน integration build แบบ warm.)

---

## 6. Doc staleness (ให้เจ้าของเอกสารทราบ — ไม่ใช่ของผมครอง)

`look-tier-spec.md` §2 ตาราง + `look-perf-methodology.md` §5 ตาราง **ล้าหลัง** look.rs แล้ว:
ยังเขียนว่า High มี PCSS/DoF, tonemapper AcesFitted — ขัดกับ look.rs + look-contract.md +
look-audit ที่ใช้ **TonyMcMapface + High มี VolumetricFog(32) + ไม่มี PCSS + ถอด DoF**.
(lane owner: tier-spec = rose; methodology = poppy.)

---

## 7. ที่เหลือ

1. **Shino ประกาศ P0 เขียว** → เรนเดอร์เฟรมเกมจริง (`--play`): look-on/off/vista/ultra ผ่านกล้อง
   OrbitCam จริง → วัด grade จริง + ถ่าย before/after Gate 3 + เติม audit §4.
2. ครั้งต่อไปที่วัด perf ของ tier ใด: **ทิ้ง round-1** (cold cache), ใช้ round อุ่น, เช็ค mean/median.
3. พิจารณา screen-space light shaft แทน god ray ที่ Low/Medium (ถ้า pass 3 สำคัญต่อ score).
