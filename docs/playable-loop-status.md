# Voxelforge — สถานะ "เล่นเป็นเกมได้จริงแค่ไหน"

อ่านจากซอร์สสาขา `poppy/native-only` อย่างเดียว (ไม่รัน cargo build/check — Poppy ถือเลนบิลด์)
วันที่ 2026-08-17

**กติกาการนับ** — "ใช้งานได้" ต้องมี system ถูก register จริงใน `App` จาก `client/src/main.rs`
(ผ่าน `add_plugins(...)` หรือ `.add_systems(...)` โดยตรง) ถ้าโค้ดมีแต่ไม่ได้ add_systems
(อยู่แค่ใน `[[bin]]` อื่น หรือเป็น library ที่ไม่มี `impl Plugin`) → นับเป็น **ไม่มี** ในตัวเกม

## สรุปบรรทัดเดียว

แกน voxel + เดิน/กระโดด/กล้อง/ต่อสู้/เซฟโหลด/เมนู **เล่นได้จริง** (closed loop:
เดิน → ต่อสู้ → ตาย → respawn) แต่**ยังไม่ใช่เกมที่สมบูรณ์** — ยังไม่มี inventory/ไอเทม,
ผู้เล่นยังเป็นแคปซูลกลม ๆ, ศัตรูในเกมมีตัวเดียว (Guard Husk), quest chain ค้างกลางทาง,
โลกเป็น procedural ไม่ใช่เวิลด์ที่ออกแบบ, และไม่มี win/lose end-state

## ตารางระบบ × สถานะ

| ระบบ | สถานะ | หลักฐาน (ไฟล์:บรรทัด ที่ register จริงใน App) |
|---|---|---|
| **Movement (เดิน/วิ่ง/บิน)** | ✅ REAL | `main.rs:1510` `fly_camera` — WASD เดินบนพื้น + บิน noclip (ปุ่ม F สลับ); gravity + AABB voxel collision + step-up ผ่าน `move_body`; ถูก add ตรง `main.rs:678` |
| **Jump (กระโดด)** | ✅ REAL | `main.rs:1595-1597` — Space กระโดดใน walk mode (`fly.vel.y = JUMP_SPEED`, gate ด้วย `fly.grounded`) |
| **Camera (กล้อง 3rd-person)** | ✅ REAL | `main.rs:1629-1636` orbit + camera boom ดึงกล้องหนีกำแพง; `main.rs:713` `combat::lock_on_camera` ล็อกเป้า; editor มี `editor_camera.rs:71` orbit/pan/zoom แยก |
| **ขุด/วางบล็อก** | ⚠️ PARTIAL | ตัว build loop จริงแต่อยู่ใน **Editor state เท่านั้น** — `editor::EditorPlugin` (add ที่ `main.rs:569`) + `main.rs:2121` `edit_voxels` ซึ่ง comment `main.rs:672-674` ระบุชัดว่า **gated OFF ตอน Play** ("the player fights, not builds"); ใน Play มีแค่ R-key วางบล็อกตาม objective ภารกิจ (`quest.rs:1706` `check_block_place_triggers`) |
| **Inventory / ไอเทม** | ❌ ไม่มี | `save_game.rs:9-10` เขียนตรง ๆ ว่า "There is no inventory in the codebase yet (combat exposes Health/Stamina, not items)"; `equipment.rs:1444` เป็นแค่ "Catalogue dump — the hook a future item/inventory system reads" — ไม่มี `impl Plugin`, ไม่ add_systems |
| **Terrain streaming** | ✅ REAL | `streaming.rs:44` `StreamingPlugin` — LOD 0/1 (`lod1_mesh` ที่ :148), frustum culling, async terrain-gen (`view_distance=12` → 25×25 chunks); add ที่ `main.rs:616` |
| **Day-night (กลางวัน/กลางคืน)** | ⚠️ PARTIAL | `look.rs:1374` struct `Hour` — เป็น **preset คงที่** (NIGHT/GOLDEN/DAY) เลือกจาก env/`night` flag (`look.rs:1762`), ไม่มีระบบเวลาต่อเนื่อง (grep `Res<Time>` ใน look.rs ไม่เจอระบบหมุนพระอาทิตย์); F7 หมุน **quality tier** ไม่ใช่เวลา |
| **Enemy AI (AI ศัตรู)** | ⚠️ PARTIAL | ในเกมจริงมี **แค่ Guard Husk ตัวเดียว** — `combat.rs:3856` `CombatFeelPlugin` → `husk_ai` (perception + tactics squad pass) add ที่ `main.rs:706`; ส่วน `enemy_ai.rs:672` `EnemyAiPlugin` (archetype Swarm/Bruiser/Pouncer) **ไม่ได้ declare เป็น mod ใน main.rs** — ถูก register แค่ใน `enemy_ai_proof_main.rs:440` (bin `voxelforge_enemyai_proof` แยก) → นับเป็นไม่มีในตัวเกม |
| **Combat (ต่อสู้)** | ✅ REAL | `main.rs:699-720` chain `gather_input → player_combat → husk_ai → husk_telegraph → hud_bars` (Play-gated) + `dodge_parry.rs:815` `DodgeParryPlugin`; attack light/heavy/charged, stamina, HP, poise, dodge i-frame, parry, hit-stop, kill→`EnemyDied`; มี headless proof "attack→hit→kill husk (HP=0)" (`combat.rs:3963`) |
| **Quest / ภารกิจ** | ⚠️ PARTIAL | `quest.rs:502` `QuestPlugin` จริง — journal, NPC interact, dialogue, kill/area/approach/lore/place triggers, world rewards (ประตู/sigil/แคมป์ไฟ) add ที่ `main.rs:583`; **แต่** chain ค้าง: scripted demo wedges ที่ map column (43,7) phase 2, q4 ไม่เคย Active (ดู memory `rkey-place-block-unreachable`) |
| **Cutscene** | ⚠️ PARTIAL | `cutscene.rs:221` `CutscenePlugin` — มีแค่ `stage_conversation_camera` (`cutscene.rs:254`) blend กล้องเป็น two-shot ตอนคุยบทสนทนา; **ไม่มี** timeline/keyframe/path กล้องแบบคัตซีนจริง |
| **Audio (เสียง)** | ✅ REAL | `audio.rs:361` `AudioPlugin` — spatial listener, SFX, footstep, ambient zone + crossfade, music; add ที่ `main.rs:581` |
| **Save/Load (เซฟ/โหลด)** | ✅ REAL | `save_game.rs:257` `SaveGamePlugin` — F6 quick-save (pos/yaw/HP/stamina) + quest journal; New Game / Continue จริง (`save_game.rs:169,201`); add ที่ `main.rs:614` + `main.rs:688` `egui_save_load` |
| **Menu / UI** | ✅ REAL | `main_menu.rs:120` `MainMenuPlugin` (egui, New Game/Continue) add ที่ `main.rs:613`; `settings_menu.rs:119`, `hud.rs:116` (HP/stamina bar), `dialogue_ui.rs:42` (กล่องบทสนทนา typewriter) |

## ขาดอะไรอีกถึงเรียกว่าเกม (เรียงตามความสำคัญ)

1. **Inventory + ระบบไอเทม** — ตอนนี้ไม่มีเลย (สู้แล้วได้แต่ HP/stamina เปลี่ยน ไม่มี loot/consumable/equipment ให้ผู้เล่นเก็บหรือตัดสินใจ)
   _แรงงาน: ~8-12 md_ (มี hook แล้วที่ `equipment.rs:1444` + `characters.rs` swap gear)

2. **ตัวละครผู้เล่นเป็นแคปซูล** — ผู้เล่นยังเป็น `Capsule3d` (`main.rs:975`) ไม่ใช่ตัวละคร rigged;
   rig ที่มีอยู่ (anim.rs box-humanoid) ต่อแค่กับ Husk ศัตรู — ต้องเอา rig มาใส่ผู้เล่น + animations เดิน/โจมตีจริง
   _แรงงาน: ~5-10 md_

3. **ต่อ AI ศัตรูหลาย archetype เข้าเกมจริง** — `enemy_ai.rs` (Swarm/Bruiser/Pouncer) เขียนเสร็จแล้วแต่เป็น proof bin
   แยก ไม่ได้อยู่ใน main binary; เกมจริงมีแค่ Guard Husk ตัวเดียว ต้อง wire `EnemyAiPlugin` + spawn squad ลงโลก
   _แรงงาน: ~5-8 md_

4. **ปิด quest chain ให้จบถึง end** — engine มีครบ แต่ demo ค้างที่ map column (43,7) และ q4 ไม่เคย Active;
   ต้องแก้ map/demo wedge + ทำให้ทั้ง chain เล่นจบได้เป็นเส้นเดียว
   _แรงงาน: ~5-10 md_ (ขึ้นกับว่าบั๊กอยู่ที่ map หรือ logic)

5. **เวิลด์/เนื้อหาที่ออกแบบ** — `play_map()` ยัง fallback เป็น procedural terrain; หมู่บ้านของ Shiba
   (`maps/edhari.json`) ยังไม่ลง — เกมที่เล่นจริงต้องมีเวิลด์ที่มีจุดหมาย ไม่ใช่ทุ่ง procedural เปล่า
   _แรงงาน: ~15-30 md_ (art + block placement)

6. **Day-night / เวลา / สภาพอากาศ** — มีแค่ Hour preset คงที่ (เลือกตอน boot) ไม่มีเวลาหมุนวนกลางคืน→กลางวัน
   ที่กระทบ gameplay (ศัตรู/เควสต์เปลี่ยนตามเวลา)
   _แรงงาน: ~3-5 md_

7. **Win/Lose end-state + คัตซีนเล่าเรื่อง** — ตอนนี้ death→respawn มี แต่ไม่มี "ชนะ/จบเกม" ไม่มีเงื่อนไขจบ
   และไม่มีคัตซีนจริง (มีแค่กล้อง two-shot บทสนทนา) — ไม่มี closure ให้ผู้เล่นรู้ว่า "จบแล้ว"
   _แรงงาน: ~6-10 md_
