# Commit plan — `poppy/third-person-controller` (2026-07-30)

Audit ของ working tree ที่ยัง uncommitted บน branch `poppy/third-person-controller`.
HEAD ปัจจุบัน = `2861818` · staging ว่าง · **ยังไม่มีอะไรถูก commit**

เอกสารนี้คือ *แผน* ไม่ใช่การกระทำ — ไม่มี `git add` / `git commit` ถูกรันตอนเขียนไฟล์นี้.
**ห้าม `git add` จนกว่า CEO สั่ง** (คำสั่งตรงจาก CEO รอบ 15:07)

## รอบ re-sync — 15:15 (ไฟล์จริงเดินหน้าไปกว่าแผนรอบ 15:10)

แผนรอบ 15:10:47 ตกยุคทันทีเพราะ **yamamoto ลงทั้ง 3 รายการค้างเสร็จตอน 15:11** — ตรวจใหม่
แบบ read-only เมื่อ 15:15:06 แล้ว (`scene.rs` mtime 15:13:56 = ยังขยับอยู่, `combat.rs` 15:11:11):

| รายการ | สถานะจริง 15:15 | หลักฐาน |
|---|---|---|
| ถอด `info!("PROBE …")` ใน `combat.rs` | ✅ **เสร็จ** | `grep -c PROBE client/src/combat.rs` = **0** |
| `AppExit::Success` → `from_code(1)` ที่ `COMBAT_FATAL` | ✅ **เสร็จ** | `scene.rs:1176` = `exit.write(AppExit::from_code(1)); // nonzero: this path is a failure, not a clean exit` |
| คอมเมนต์ขัดกันเองเหนือบล็อก husk | ✅ **เสร็จ** | `scene.rs:283-291` เหลือย่อหน้าเดียว บรรยายของใหม่ (gate ด้วยระยะจริง + D-strafe window สั้น) ตรงกับ phase 1 |

→ **ของค้างของ yamamoto เหลือแค่ build + prove** ไม่มีงานแก้โค้ดค้างอีก. แผนนี้จะถูกใช้เป็น
runbook ตอน commit → ตัวเลข/เลขบรรทัด/สูตร `git add` ในเอกสารนี้ sync กับ working tree ณ 15:15 แล้ว

## รอบแก้ที่ 2 — ตาม verdict ของ CEO (15:07)

CEO เปิด `docs/LANES.md` ตรวจเองแล้วพบว่าแผนรอบแรกอ้างเลนผิด 3 จุด. แก้แล้วทั้งหมด:

| # | ที่ CEO ชี้ | ผลต่อแผน |
|---|---|---|
| ① | `scripts/verify_edhari_village.py` → `LANES.md:34` = **shiba** (ไม่ใช่ yamamoto) | ไม่มีการข้ามเลน → **ยุบ C1b กลับเข้า C1** |
| ② | `scripts/_shino_*.py` → `LANES.md:33` = **shino (Director)** = เลนของ CEO เอง | **ยกเลิก HOLD ของ C1c — commit ได้เลย** |
| ③ | C0 เขียนว่า "เพิ่ม 2 แถวให้ yamamoto" แต่ diff จริงเพิ่ม **3 แถว** (kevin / shino / shiba) | แก้ข้อความ C0 ให้ตรง diff |

verdict อื่นในรอบเดียวกัน: **C6 อนุมัติ** (shino เคาะเอง) · **C7 อนุมัติ** ·
**`AppExit::from_code(1)` อนุมัติ** · **C5 ไม่ผ่าน** (เหตุผล + สถานะจริงอยู่ในหัวข้อ C5) ·
งานแก้โค้ด + build + prove ของ C5 = **yamamoto** → poppy **ถอยจาก `scene.rs` / `combat.rs`
ห้ามแก้ซ้อน** แล้วสแตนด์บาย พอ proof เขียว poppy เป็นคนรัน commit sequence **C2 → C4 → C5 → C6**

## ไฟล์ทั้งหมดที่ต้องหาบ้าน (14 ไฟล์)

| # | ไฟล์ | สถานะ | +/- | ปลายทาง |
|---|---|---|---|---|
| 1 | `.gitignore` | M | 7/0 | C0 |
| 2 | `docs/LANES.md` | M | 3/0 | C0 |
| 3 | `maps/edhari.json` | M | 1/1 | C1 |
| 4 | `scripts/gen_edhari.py` | M | 150/29 | C1 |
| 5 | `scripts/verify_edhari_village.py` | ?? | new | **C1** *(ยุบจาก C1b — เลนเดียวกัน)* |
| 6 | `scripts/_shino_spawn_sim.py` | ?? | new | **C1c** *(HOLD ยกเลิกแล้ว)* |
| 7 | `client/src/combat.rs` | M | **6/3** (PROBE ถอดแล้ว) | C2 (**ทั้งไฟล์ 3 hunk**) |
| 8 | `client/src/main.rs` | M | 5/5 | C2 (1 hunk) + C3 (1 hunk) |
| 9 | `client/src/editor_ui.rs` | M | 3/1 | C3 |
| 10 | `client/src/scene.rs` | M | **317/44** (ยังขยับ — yamamoto ถือไฟล์อยู่) | C4 + C5 (แยก hunk) |
| 11 | `scripts/prove_combat.sh` | M | 1/1 | **C6 ✅ shino อนุมัติแล้ว** |
| 12 | `docs/first-five-minutes.md` | ?? | new | C7 |
| 13 | `docs/assets/edhari-village-topdown.png` | ?? | new (2.4 KB) | C7 |
| 14 | `docs/commit-plan-2026-07-30.md` | ?? | ไฟล์นี้ | C7 |

---

## C0 — housekeeping (ไม่กระทบ build)

```
git add .gitignore docs/LANES.md
git commit -m "chore: ignore _combat_proof evidence dir + three missing lane rows"
```

- `.gitignore:71-77` → `/_combat_proof/` (พิสูจน์แล้ว: `git check-ignore -v _combat_proof/` คืน `.gitignore:77`) — โฟลเดอร์ยังอยู่บนดิสก์ ไม่ได้ลบหลักฐาน
- `docs/LANES.md` **เพิ่ม 3 แถว ไม่ได้ลบของเดิม** (ตรวจกับ `git diff` แล้ว):
  - `:30` `client/src/editor_ui.rs` → **kevin**
  - `:33` `scripts/_shino_*.py` (Independent cross-checks) → **shino (Director)**
  - `:34` `scripts/verify_edhari_village.py` (Map verification) → **shiba**
- ปลอดภัยที่จะลงเป็นตัวแรก: ไม่แตะ `.rs` เลย

**sign-off ของไฟล์นอกตาราง lane** (`LANES.md:36` — "Anything not listed: ask the Director
before the first edit"). C0 แก้ไฟล์นอกตาราง 2 ไฟล์ → ต้องมีบรรทัดนี้ ไม่ใช่แค่ C7:

- `docs/LANES.md` — ✅ **ผ่าน**: 3 แถวนี้เขียนตามที่ Director สั่ง และ Director เปิด diff
  ตรวจเองรอบ 15:07 แล้วยืนยันเนื้อแถวทั้งสาม (สั่งแก้แค่ *คำบรรยาย* ใน C0 ให้ตรง diff — verdict ③)
- `.gitignore` — ⏸ **ยังไม่มี sign-off ตรง ๆ**. เป็น housekeeping ล้วน (`/_combat_proof/`,
  ไม่ลบไฟล์บนดิสก์) แต่กฎคือกฎ → ขอ ack หนึ่งคำจาก Director ก่อนรัน C0. ถ้าไม่ทันให้แยกลง
  `docs/LANES.md` ก่อนแล้วเก็บ `.gitignore` ไว้รอ ack — ห้ามลงพ่วงเงียบ ๆ

---

## C1 — Edhari map + generator + verifier · lane **shiba**

```
git add maps/edhari.json scripts/gen_edhari.py scripts/verify_edhari_village.py
git commit -m "maps(edhari): village layout — road, shelter, campfire ring, dungeon door + schema/spawn verifier"
```

**หลักฐาน reproducibility (รันจริงแล้ว):** rerun `gen_edhari.py` ไปที่ temp แล้วได้ไฟล์ **byte-identical** กับ `maps/edhari.json` — seed `20260730`, 4,592 บล็อก, 64×64. generator กับ artifact ตรงกัน ไม่ต้องกลัว drift

**ทำไม verifier มาอยู่ที่นี่ (เดิมแยกเป็น C1b):** `LANES.md:34` ระบุ `scripts/verify_edhari_village.py` = **shiba** เหตุผลตามที่แถวนั้นเขียนไว้เอง — "grades `maps/edhari.json` against the loader schema, so it moves with the map lane above". lane เดียวกับ `maps/` + `gen_edhari.py` → **ไม่ใช่ commit ข้ามเลน** และควรอยู่ commit เดียวกับแมพที่มันเกรด. C1b ในแผนรอบแรกอ้าง `LANES.md:33` ผิดแถว — ยกเลิกแล้ว

---

## C1c — spawn cross-check sim · lane **shino (Director)** · ✅ HOLD ยกเลิก

```
git add scripts/_shino_spawn_sim.py
git commit -m "scripts(cross-check): independent re-implementation of map_spawn/boot_scene, written from the Rust source"
```

**ทำไมเลิก HOLD:** ไฟล์ match `scripts/_shino_*.py` ที่ `LANES.md:33` = **shino (Director)** = เจ้าของคือ CEO เอง ตรงกับ docstring บรรทัดแรกของไฟล์เป๊ะ:

> `"""Shino's independent re-implementation of scene.rs::map_spawn / boot_scene.`
> `Written from the Rust source, NOT from Yamamoto's validator, so it can disagree."""`

ไม่มีข้อขัดแย้งให้ชี้ขาด — แผนรอบแรกอ่านแถวผิดเป็น yamamoto เอง. เจ้าของในประวัติจะถูกต้องตั้งแต่ commit แรก

**และห้ามยุบทิ้ง:** `LANES.md:33` เขียนกฎไว้เองว่า *"Don't delete one because another checker 'already covers it'"* — ไฟล์นี้ตั้งใจให้ **ไม่เหมือน** `verify_edhari_village.py` (เขียนจาก Rust source ไม่ใช่จาก validator ตัวอื่น) เพื่อให้มันสามารถ *ไม่เห็นด้วย* ได้. คนละ commit คนละเลนกับ C1 ถูกต้องแล้ว

---

## C2 — `spawn_guard_husk(surface_y)` · lane **kevin** · ⚠️ **ต้องมาก่อน C4**

```
git add client/src/combat.rs         # ทั้งไฟล์ — PROBE ถอดแล้ว ไม่มี hunk ที่ต้องกัน
git add -p client/src/main.rs        # เอาเฉพาะ hunk `None,` ที่ spawn_encounter
git commit -m "combat: let the caller pass the husk's ground Y (map terrain != noise terrain)"
```

⚠️ **สูตรเปลี่ยนจากรอบแรก** — เดิมเขียนว่า "`git add -p` เอาเฉพาะ hunk `surface_y` ไม่เอา PROBE".
ใช้ไม่ได้แล้วและถ้าทำตามตรงตัวจะ **เหลือของค้างใน working tree**: PROBE หายไปแล้ว แต่การถอดมันทิ้ง
เศษ refactor ไว้ 2 ก้อน → `combat.rs` ตอนนี้มี **3 hunk** และทั้งสามควรอยู่ commit นี้ (ไฟล์เดียว
lane เดียว = kevin, ทั้งหมด behaviour-neutral):

| hunk | ที่ | คืออะไร |
|---|---|---|
| 1 | `spawn_guard_husk:590` | เพิ่มพารามิเตอร์ `surface_y: Option<f32>`; `None` = `terrain_height()` เดิม → พฤติกรรมเดิมไม่เปลี่ยน ← **แกนของ C2** |
| 2 | `gather_input:746` | ดึง `let light_pressed = mouse.just_pressed(Left) \|\| keys.just_pressed(KeyX)` ออกมาเป็น local (ตัวที่ PROBE เคยพิมพ์) แล้วใส่ใน `CombatIntent { light: light_pressed, … }` — ค่าเท่าเดิมเป๊ะ |
| 3 | `player_combat:863` | ดึง `let in_cone = in_cone(facing, to, MELEE_CONE);` ออกมาเป็น local แล้ว `if !in_cone { continue }` |

**ตรวจ hunk 3 แล้ว ไม่ใช่บั๊ก:** มัน *ยก* การเรียก `in_cone()` ขึ้นมาเหนือ early-continue ของ
ระยะ → ตอนนี้คำนวณ cone แม้ตัวที่ไกลเกินระยะ. `in_cone()` เป็น pure fn (`combat.rs:1361`)
ลำดับ `continue` ทั้งสองยังเดิม → **ผลลัพธ์ไม่เปลี่ยน** แค่เสียการคำนวณเปล่าเล็กน้อยต่อ entity ที่ไกล.
และการ shadow ชื่อ `in_cone` ด้วย local `bool` **คอมไพล์ผ่าน** — ในบล็อกนั้นไม่มีการเรียก `in_cone()`
ซ้ำอีก (การเรียกอีกที่เดียวอยู่ `nearest_target:1388` คนละฟังก์ชัน). *ถ้า* yamamoto ว่างจะย้ายกลับไป
ใต้เช็คระยะก็ดีขึ้นเล็กน้อย แต่ **ไม่ block C2**
- `main.rs:683` เติม `None,` ที่ call site เดิม

**เหตุผลที่ต้องบังคับลำดับ:** เปลี่ยน signature ในไฟล์เดียวแล้ว commit → `main.rs` และ `scene.rs` เรียกด้วยจำนวน arg เก่า = **build แตกกลางประวัติ**. call site ใน `main.rs` จึงต้องอยู่ commit เดียวกับ signature แม้จะทำให้ `main.rs` ถูกแบ่งเป็น 2 commit (ทั้งคู่ lane kevin — ไม่ข้ามเลน)

✅ **ของค้างข้อนี้ปิดแล้ว:** ถอด `info!("PROBE …")` ออกจาก `combat.rs` ครบ — yamamoto ลงตอน
15:11:11, ยืนยัน 15:15 ด้วย `grep -c PROBE client/src/combat.rs` = **0**. ไม่ต้องกัน hunk PROBE
ตอน stage อีกแล้ว (นั่นคือเหตุผลที่สูตร `git add` ข้างบนเปลี่ยนเป็นทั้งไฟล์)

---

## C3 — ปิดโหมดสร้างตอน Play · lane **kevin**

```
git add -p client/src/main.rs        # hunk edit_voxels ที่เหลือ
git add client/src/editor_ui.rs
git commit -m "play: L/R-click no longer breaks the village, and the editor panels stay hidden"
```

- `main.rs:583` ถอด `edit_voxels.run_if(in_state(AppState::Play))` → ผู้เล่นทุบหมู่บ้านไม่ได้แล้ว
- `editor_ui.rs:85` egui panel ทั้ง 5 ตัว `.run_if(in_state(AppState::Editor))` → HUD dev ไม่โผล่ทับจอเกม
- ตรงกับ P0 ข้อ 3 ใน `docs/first-five-minutes.md`

---

## C4 — husk บนแมพ + respawn ที่กองไฟ · lane **poppy** (ไฟล์อยู่กับ yamamoto ชั่วคราว)

```
git add -p client/src/scene.rs       # hunks: boot_scene + respawn_at_campfire
git commit -m "play(edhari): one Guard Husk 7 blocks from spawn; respawn wakes you at the fire"
```

- `boot_scene` รับ `ResMut<Encounter>`, ถ้า `from_map` → spawn husk ที่ `(sx, sz-7)` ด้วย `Some(ground_y)` (พื้นเดียวกับผู้เล่น) + `spawn_combat_hud` + ตั้ง `enc.spawned = true`
- `respawn_at_campfire` วางผู้เล่นที่ `camp.fire` ไม่ใช่ `camp.eye` + guard log ถ้า `place_player` ไม่ติด
- ปลดล็อก P0 ข้อ 1: ศัตรู + HUD + ตาย/เกิดใหม่ พร้อมกัน

⚠️ **เลขบรรทัดของ `scene.rs` ในแผนนี้เลิกอ้างอิงแบบ pin แล้ว** — ไฟล์ถูกแก้สดตอน 15:05 (yamamoto) และจะขยับอีก. อ้างอิงด้วยชื่อ symbol เท่านั้น เวลา `git add -p` ให้ดู hunk ตามชื่อฟังก์ชัน

✅ **ของค้างข้อ 1 ปิดแล้ว:** คอมเมนต์ขัดกันเองเหนือบล็อก husk ใน `boot_scene` — yamamoto ยุบเหลือ
ย่อหน้าเดียว (`scene.rs:283-291`) และเนื้อความตรงกับโค้ดจริงแล้ว: "walks W until within melee range
(gated on live distance, not a wall-clock guess), with a brief D-strafe on a short wall-clock window"
ตรงกับ phase 1 ที่เกท `husk_dist <= 3.0` + D-strafe หน้าต่างสั้น. ไม่มี "t≈3.5" ค้างอีก

**ต้องยืนยันก่อน stage — เจ้าของงาน: poppy (ตอน commit):**
2. **`from_map` ไม่ได้ gate ด้วย `combat_demo`** — ทุกครั้งที่โหลดแมพจะมี husk โผล่ รวมถึงตอนถ่าย screenshot. golden shot ถ่ายผ่าน `voxelforge_shot` / `hero.rs` ซึ่งไม่ผ่าน `boot_scene` → *น่าจะ* ไม่กระทบ แต่ **ยังไม่ได้รันพิสูจน์**. วิธีพิสูจน์: ถ่าย shot 1 รูปหลัง build ของ yamamoto เขียว แล้วเทียบว่าไม่มี husk ในเฟรม — ต้องไม่ชน exe lock ของ pixel/yamamoto (ใช้ target dir แยก); ถ้าชน ให้เลื่อนแล้วรายงาน ไม่แอบ commit โดยไม่พิสูจน์

---

## C5 — combat proof harness · lane **poppy** · 🛑 **CEO ไม่อนุมัติ (15:07) — งานแก้อยู่กับ yamamoto**

```
git add client/src/scene.rs          # ส่วนที่เหลือ: CombatProof fields, ScenePlugin ordering, combat_proof
git commit -m "proof(combat): close the distance before swinging, and stop grading the gate backwards"
```

**บั๊กจริงที่แผนนี้แก้** — ใน `combat_proof`

```rust
proof.husk_dead = husk_alive;   // เดิม: gate กลับด้าน — husk ที่ยัง "เป็น" = ผ่าน
proof.husk_dead = !husk_alive;  // แก้แล้ว
```

นี่คือ "gate ที่ผ่านตอนควรตก" ตรงตามที่ `LANES.md:40` เตือนไว้เป๊ะ — proof เดิมรายงาน PASS ได้ทั้งที่ husk ไม่เคยตาย

การเปลี่ยนแปลงอื่นในกลุ่มนี้:
- `CombatProof` เพิ่ม `last_log`, `walk_start`, `stamp` (per-phase relative time)
- `combat_proof` เพิ่ม `.before(combat::gather_input)` — คีย์ที่ inject ต้องลงใน intent เฟรมเดียวกัน
- phase 1 เดินจน `husk_dist <= 3.0` แทน hard-code `t < 3.5` + timeout guard 20 s
- `keys.reset()` ก่อนทุก `keys.press(KeyCode::KeyX)` (ไม่งั้น `just_pressed` ไม่ยิงซ้ำ)
- ~~phase 6 เปลี่ยน heavy(C) → light(X) ที่ 4 = 80 dmg พอดี~~ **← CEO ปฏิเสธ: ลด coverage**
- log ใหม่: `CHASE_DIST` (ทุก 0.5 s), `COMBAT_WALK_DONE`, `COMBAT_FATAL`
- ระยะ respawn วัดแบบ XZ จาก `camp.fire` แทนระยะ 3D จาก `camp.eye`

### 🛑 CEO verdict — phase 6 ต้องคงเป็น heavy(C)

คำตัดสินเดิมของ CEO ยืนยันอีกครั้ง: เปลี่ยน phase 6 จาก heavy(C) → light(X)×4 = **ลด coverage
ของ proof** ไม่อนุมัติ. ผลข้างเคียงที่ CEO ชี้: `husk_hp_pre_heavy` / `stam_pre_heavy` /
`heavy_hit` / `COMBAT_HEAVY` จะกลายเป็น **โค้ดตาย** เพราะไม่มี `KeyC` ถูกกดเลย

**สถานะจริงในไฟล์ ณ 15:07 (ตรวจแบบ read-only, ไม่แก้อะไร):** heavy(C) **กลับมาแล้ว** —
yamamoto แก้ `scene.rs` ไปตอน 15:05:22:

| สิ่งที่ต้องมี | พบใน working tree |
|---|---|
| กด `KeyC` จริง | phase 4 `keys.press(KeyCode::KeyC)` · phase 5 hold แล้ว `keys.reset` เพื่อ commit Heavy |
| `husk_hp_pre_heavy` / `stam_pre_heavy` ถูกเขียน | phase 4 ทั้งคู่ |
| เกรด `COMBAT_HEAVY` | phase 6 — `dmg > 30.0` **และ** `(stam_cost - 35.0).abs() < 5.0` (กันกรณีตกไปเป็น light เงียบ ๆ) |
| `heavy_hit` ถูกอ่าน | phase 3 — พิมพ์ `COMBAT_HEAVY skipped` ถ้า husk ตายตั้งแต่ light แรก |

→ **ไม่มีโค้ดตายเหลือแล้ว** และ phase 7-9 ยัง insurance light ต่อจาก heavy (คอมเมนต์คิดเลข
80 hp − ~20 light − ~45 heavy ≈ 15). ข้อนี้ปิดได้เมื่อ proof รันเขียว

### 🛑 ที่ยังค้างจริง — เหลือข้อเดียว (ณ 15:15)

1. ✅ **`AppExit::from_code(1)` ลงแล้ว** — `scene.rs:1176` ในบล็อก `proof.phase == 99`:
   `exit.write(AppExit::from_code(1)); // nonzero: this path is a failure, not a clean exit`
   (yamamoto, 15:11–15:13). สำคัญเพราะ `prove_combat.sh:50` เกรด `[ "$rc" -ne 0 ]` อยู่แล้ว
   → ตอนนี้เส้นทาง fatal ทำให้ gate **ตกจริง** แทนที่จะคืน 0 แล้วเนียนผ่าน. ทางออกปกติสองจุด
   (`:805` จบ walk-proof, `:1169` จบหลัง `COMBAT_RESPAWN`) ยังเป็น `AppExit::Success` — ถูกต้องตามเจตนา
2. 🛑 **ยังไม่มีใคร build หรือรัน `prove_combat.sh` หลังชุดแก้นี้** — ข้อนี้ยังค้างอยู่และเป็นตัวเดียว
   ที่ block. `cargo check` เขียวไม่ได้แปลว่าพฤติกรรมถูก (`LANES.md:46`). commit ที่บอกว่า "แก้ proof"
   โดยไม่เคยเห็น proof ผ่าน คือสิ่งเดียวกับที่ LANES ห้าม. **เจ้าของงาน build + prove: yamamoto**
3. C5 พึ่ง C2 ผ่าน C4 → ลำดับ **C2 → C4 → C5** บังคับ

**gate `husk_dead` ตรวจแล้วครบ 3 ทาง** (`scene.rs:991`, `:1055`, `:1089`): สองทางแรก set `true`
ตอนรู้แน่ว่า husk ตาย, ทางที่สามคือ `!husk_alive` ที่แก้ด้านกลับแล้ว — ไม่มีทางไหนเหลือ `= husk_alive`

**Definition of done ของ C5:** `bash scripts/build_safe.sh build --bin voxelforge` ผ่าน (เช็คด้วย
`grep '^error'` ไม่ใช่ `tail`) แล้ว `bash scripts/prove_combat.sh` แสดง `COMBAT_HIT … PASS` →
`COMBAT_HEAVY … PASS` → `COMBAT_KILL` → `COMBAT_DEATH … PASS` → `COMBAT_RESPAWN … PASS` ครบสาย
และปิดท้าย `PROVE_COMBAT: PASS`. ถ้ายังไม่ได้ log นี้ **ห้าม commit**

**ขอบเขตของ poppy ในข้อนี้:** ไม่แก้ `scene.rs` / `combat.rs` เลย (คำสั่ง CEO — กันแก้ซ้อนกับ
yamamoto). poppy รอ proof เขียว แล้วเป็นคนรัน commit sequence **C2 → C4 → C5 → C6**

---

## C6 — proof script grep · lane **shino (Director)** · ✅ **shino อนุมัติแล้ว (15:07)**

```
git add scripts/prove_combat.sh
git commit -m "proof(combat): surface CHASE_DIST / WALK_DONE / FATAL in the graded log"
```

diff จริงคือบรรทัดเดียว (`prove_combat.sh:44`) — เติม 3 pattern เข้า `grep -E`:

```
… |PLAYER_DIED|RESPAWN|CHASE_DIST|COMBAT_WALK_DONE|COMBAT_FATAL
```

**เหตุผลที่ผ่าน:** เป็นการ *เพิ่ม* บรรทัดที่ถูกพิมพ์ออกมาให้เห็นในล็อก ไม่ได้ลบ pattern เดิมสักตัว
และไม่ได้แตะเงื่อนไข pass/fail — gate **เข้มขึ้น** (มองเห็น `COMBAT_FATAL` ได้) ไม่ได้หลวมลง
ตรงตามกฎ "gates get stricter, never looser"
**ผูกกับ C5:** 3 pattern นี้จะไม่มีอะไรให้ grep เลยถ้า C5 ยังไม่ลง → **C5 ก่อน C6**

### 🔍 ข้อเสนอเพิ่มถึง shino (ไม่รวมใน C6 — เลนของ shino, poppy ไม่แก้เอง)

`COMBAT_HEAVY` **ไม่อยู่ทั้งสองที่** ใน `prove_combat.sh`:

- `:44` grep -E → ไม่มี `COMBAT_HEAVY` → PASS/FAIL ของ heavy ไม่โผล่ในล็อกที่ self-documenting
- `:55` `for line in COMBAT_HIT COMBAT_KILL COMBAT_DEATH COMBAT_RESPAWN` → **ไม่มี `COMBAT_HEAVY`**
  → ถ้า heavy พิมพ์ `=> FAIL` **gate ยังผ่าน**

ในเมื่อ verdict ของ CEO คือ "ต้องคง heavy coverage" การเกรด heavy ก็ควรอยู่ในเกตด้วย ไม่ใช่แค่
ในโค้ด. เสนอเติม `COMBAT_HEAVY` ทั้งสองจุด (เข้มขึ้น ไม่หลวมลง) — **รอ shino เคาะ**; ถ้าเคาะ
ให้เป็น commit แยก (C6b) เพราะเป็นการเปลี่ยนเงื่อนไข pass/fail ไม่ใช่แค่บรรทัดที่พิมพ์

---

## C7 — docs · ✅ **shino ack แล้ว (15:07)**

```
git add docs/first-five-minutes.md docs/assets/edhari-village-topdown.png docs/commit-plan-2026-07-30.md
git commit -m "docs: first-five-minutes punch list + village topdown + this branch's commit plan"
```

- `docs/first-five-minutes.md` — punch list "กด Play แล้ว 5 นาทีแรกเจออะไร" พร้อมตาราง P0/P1 ที่ C3/C4 ในแผนนี้ไปปิดให้ 2 ข้อ
- `docs/assets/edhari-village-topdown.png` (2.4 KB) — หลักฐาน topdown ที่เอกสารอ้าง
- ไฟล์นี้เอง — ต้องเป็นเวอร์ชันหลังรอบแก้ที่ 2 (รวม verdict ของ CEO) ตอน commit
- ทั้งสามไฟล์ไม่อยู่ในตาราง lane → `LANES.md:36` ให้ถาม Director ก่อน; **ถามแล้ว ผ่านแล้ว**

---

## ลำดับที่บังคับ

```
C0 ─ C1 ─ C1c        (อิสระ ลงเมื่อไหร่ก็ได้ — C1c ไม่ HOLD อีกแล้ว)
C2 ──► C4 ──► C5 ──► C6        ← ห้ามสลับ · poppy รันเมื่อ proof ของ yamamoto เขียว
C3                   (อิสระจากสายบน แต่ต้องหลัง C2 เพราะแชร์ main.rs)
                     C7        (ลงท้ายสุด — แผนนี้ต้องเป็นเวอร์ชันสุดท้าย)
                     C6b       ⏸ รอ shino เคาะ (COMBAT_HEAVY เข้า gate)
```

*(C1b ไม่มีอยู่แล้ว — ยุบเข้า C1 ตาม verdict ① ของ CEO)*

## เช็คลิสต์ก่อนเริ่ม commit จริง

**yamamoto (งานแก้โค้ด + build + prove)** — ยืนยันสถานะเมื่อ 15:15:06:
- [x] ถอด `info!("PROBE …")` ใน `combat.rs` — `grep -c PROBE` = **0** (15:11:11)
- [x] ลบคอมเมนต์ย่อหน้าเก่าที่ขัดกันเองเหนือบล็อก husk ใน `boot_scene` — เหลือย่อหน้าเดียว `scene.rs:283-291`
- [x] `AppExit::Success` → `AppExit::from_code(1)` ที่เส้นทาง `COMBAT_FATAL` — `scene.rs:1176`
- [x] phase 6 คง heavy(C) ไว้ — `KeyC` ถูกกดจริง, `COMBAT_HEAVY` ถูกเกรด, ไม่มีโค้ดตาย
- [ ] `build_safe.sh build --bin voxelforge` ผ่าน (เกรดด้วย `grep '^error'` ห้ามใช้ `tail`) ← **เหลือข้อนี้**
- [ ] `prove_combat.sh` ได้ `COMBAT_HIT PASS → COMBAT_HEAVY PASS → COMBAT_KILL → COMBAT_DEATH PASS → COMBAT_RESPAWN PASS → PROVE_COMBAT: PASS` ← **เหลือข้อนี้**

**poppy (ตอน commit — ไม่แก้ `.rs` เลย):**
- [ ] re-sync ตัวเลข/เลขบรรทัดในเอกสารนี้อีกครั้งก่อน stage ถ้า `scene.rs` ยังขยับ (mtime ล่าสุด 15:13:56)
- [ ] ยืนยัน husk ใน `boot_scene` ไม่โผล่ใน golden shot (ถ่าย 1 รูปหลัง build เขียว, target dir แยก, ไม่ชน exe lock)
- [ ] รัน `C2 → C4 → C5 → C6` ตามลำดับ + `C0` `C1` `C1c` `C3` `C7`
- [ ] `git add` เฉพาะเมื่อ CEO สั่ง

**อนุมัติที่ปิดแล้ว:**
- [x] shino ack C6 (lane ตัวเอง) + C7 (ไฟล์นอกตาราง)
- [x] CEO ชี้ขาดเจ้าของ `scripts/_shino_*.py` = shino → C1c ปลด HOLD
- [x] CEO อนุมัติ `AppExit::from_code(1)`
- [x] CEO ยืนยัน: C5 phase 6 ต้องคง heavy(C)

**ยังรอ:**
- [ ] shino เคาะ C6b (`COMBAT_HEAVY` เข้า `:44` grep + `:55` gate loop)
- [ ] Director ack `.gitignore` (ไฟล์นอกตาราง lane — ดูหัวข้อ C0)
