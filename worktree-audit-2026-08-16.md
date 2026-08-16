# Voxelforge worktree audit — 2026-08-16

> อ่านอย่างเดียว ไม่มี commit/push/stash/ลบ worktree ใด ๆ ในการตรวจนี้

## สรุปภาพรวม

ตรวจ `git worktree list` พบ **9 worktrees** ทั้งหมด มีงานค้าง (uncommitted หรือ unpushed) อยู่ **7 อัน** ในนั้นมี **3 อันที่เสี่ยงหายสูง** ที่ Director ควรรีบจัดการก่อน 6 เลนจะเขียนทับกัน

| # | Worktree | Branch / HEAD | Ahead of `poppy/native-only` | Unpushed commits | ไฟล์ค้าง | สิ่งที่ดูเป็นงานจริงยังไม่ถูกเก็บ |
|---|----------|---------------|-----------------------------|------------------|---------|-----------------------------------|
| 1 | `Voxelforge` (main) | `poppy/native-only` | 0 (เป็นตัวเอง) | **11** | **123** | สคริปต์ lane build (`lane-build.ps1`), AI proof docs/assets, art-order fixes, audio proof runlog/reel, look A/B evidence, source Rust 15 ไฟล์ |
| 2 | `_sun-quest-wt` | detached `b5dea47` | 0 | N/A | 2 | `client/src/quest.rs`, `client/src/quest_chaos.rs` แก้ไขค้าง |
| 3 | `_vf_poppy_8ba5ede` | detached `8ba5ede` | 0 | N/A | 1 | `client/src/main.rs` แก้ไข +49/-1 บรรทัด |
| 4 | `_vf_poppy_dome` | detached `bbd6624` | 0 | N/A | 1 | `client/src/hud.rs` แก้ไขค้าง |
| 5 | `_vf_poppy_head` | detached `eff6c7b` | 0 | N/A | 5 | `client/build.rs`, `client/src/anim.rs`, `client/src/main.rs`, `client/src/quest.rs` แก้ไข + **untracked textures 3 ไฟล์** ใน `assets/textures/` |
| 6 | `_yamamoto_combat_proof` | `yamamoto/audio-close-out` | **2** | N/A (ไม่มี upstream) | 1 | 2 commits งาน audio proof ยังไม่ merge + untracked `sim/examples/probe_surface.rs` |
| 7 | `Voxelforge/_fl_char_wt` | detached `be8333a` | 0 | N/A | 1 | `client/src/scene.rs` แก้ไขค้าง |
| 8 | `Voxelforge/_flamingo_beauty_wt` | `flamingo/beauty-axes` | 0 | N/A (ไม่มี upstream) | 0 | สะอาด (ไม่มี pending) |
| 9 | `Voxelforge/_poppy_head_wt` | detached `370c4a3` | 0 | N/A | 2 | `client/src/voxel.rs`, `sim/src/block.rs` แก้ไขค้าง |

หมายเหตุ: ค่า “Ahead of `poppy/native-only`” คือจำนวน commit บน worktree นั้นที่ไม่อยู่ใน `poppy/native-only` (merge-base → HEAD) ค่า “Unpushed commits” นับ `@{upstream}..HEAD` ใช้ได้เฉพาะ branch ที่มี upstream tracking

## อันดับความเสี่ยงที่งานจะหาย

### 1. 🔴 `Voxelforge` main worktree — `poppy/native-only`

- **Unpushed commits: 11 commits** — รวมงานสำคัญ:
  - `b90db33 fix(build): lane-build.ps1 busy gate is dynamic...`
  - `2d252e3 docs(ai): land the enemy-AI proof evidence...`
  - `43d01e5 fix(art-order): missed 2nd consumer of hero_hue_sep...`
  - `1bc06f1 fix(A4): stop auto-detecting frame class...`
  - `36d6648 feat(build): lane-build.ps1 - per-lane build tool...`
  - `97cb283 fix(build): unbreak HEAD — dialogue_ui.rs imports 5 hud tokens...`
  - `bbd6624 docs(look): ship the two ab-elev proof artifacts gitignore ate...`
  - `5f0d5b6 feat(audio): route audio proof through grass + wood surfaces...`
  - `5c7989b docs(look): blocker #1 A/B — shadow follows elevation...`
  - `b25b15e feat(look): v5 fill/grade pass...`
- **Pending files: 123 ไฟล์** แบ่งเป็นประเภทหลัก:
  - `.py` 27, `.sh` 25, `.cmd` 24 (สคริปต์ lane tooling / grade / proof)
  - `.rs` 15 (source Rust)
  - `.md` 12, `.png` 6, `.sha256` 3, `.ps1` 3
- **ทำไมเสี่ยง:** นี่คือ branch หลักที่ทุกเลนต้อง merge เข้า ถ้ามีคน push ทับหรือ rebase ผิดที่ 11 commits + 123 files นี้จะชนกันหรือหายได้ทันที โดยเฉพาะ `.png`/`.runlog`/`.trace` ที่เป็น evidence ลบแล้ว rebuild ยาก

### 2. 🟠 `_yamamoto_combat_proof` — `yamamoto/audio-close-out`

- **Ahead of `poppy/native-only`: 2 commits**
  - `197e5e5 fix(audio): make gen_audio_proof_reel.py actually parse the runlog`
  - `50b1451 fix(audio): ship the real 23:32 4-surface runlog, regen reel+timeline to match`
- **ไม่มี upstream tracking** (`git branch -vv` ไม่ขึ้น `[origin/...]`) แปลว่ายังไม่ได้ push ไป remote
- **Pending files: 1** — untracked `sim/examples/probe_surface.rs` เป็น scratch probe สำหรับ audio lane ที่มีคอมเมนต์ว่า “Not part of the shipped crate” แต่เป็นงานที่ยังไม่ถูกเก็บ
- **ทำไมเสี่ยง:** memory ระบุว่า “Voxelforge audio proof (2026-08-14, agent yamamoto): audio-proof.log ยืนยัน cue เล่นจริงครบ แต่ไม่มี commit ใหม่เลย — ไฟล์เสียง/audio.rs/proof log ทั้งหมดยังเป็น untracked; ต้องสั่ง commit จริงก่อนน” งานนี้ยังไม่ merge เข้า poppy/native-only และไม่มี remote backup ถ้า worktree นี้ถูกลบหรือ checkout ทับ งาน audio proof จะหาย

### 3. 🟡 `_vf_poppy_head` — detached `eff6c7b`

- **Pending files: 5**
  - Modified: `client/build.rs`, `client/src/anim.rs`, `client/src/main.rs`, `client/src/quest.rs`
  - **Untracked directory `assets/textures/`** มีไฟล์ PNG 3 ไฟล์:
    - `block_atlas_v1_procedural_BACKUP.png`
    - `block_atlas_v2.png`
    - `block_atlas_v2_bonus_glass_lamp.png`
- **ทำไมเสี่ยง:** detached HEAD ไม่มี branch สำรอง ถ้า worktree ถูกลบหรือมีคน `git checkout` ทับ ไฟล์แก้ไขทั้งหมดและ texture assets จะหายโดยไม่มีวิธีกู้คืนง่าย ๆ โดยเฉพาะ texture assets ที่อาจเป็น source-of-truth ของ art pass

## หมายเหตุ / คำแนะนำ (ไม่ได้ทำ เพราะ lane นี้อ่านอย่างเดียว)

1. **รีบ push `poppy/native-only` หลัก** ก่อน lane อื่นจะแตก branch ใหม่ เพราะ 11 commits เป็นจุดรวมของหลายเลน
2. **สร้าง PR/merge `yamamoto/audio-close-out` เข้า poppy/native-only** พร้อม commit `sim/examples/probe_surface.rs` หรือตัดสินใจว่าจะเก็บหรือทิ้ง
3. **แปลงงานใน detached HEAD worktrees เป็น branch หรือ commit** โดยเฉพาะ `_vf_poppy_head` และ `_vf_poppy_8ba5ede` ก่อนจะเกิด “ผิด worktree” อีกครั้ง
4. **ตรวจ `assets/textures/` ใน `_vf_poppy_head`** ว่าเป็น source หรือ backup — ถ้าเป็น source ควร commit ทันที

---
*ตรวจโดย Shiba — read-only audit, ไม่มีการเปลี่ยนแปลง repo ใด ๆ*
