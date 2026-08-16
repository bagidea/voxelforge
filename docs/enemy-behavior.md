# Enemy Behaviour — the AI lane

**Owner:** rose · **Created:** 2026-08-14 · **Code:** `client/src/enemy_ai.rs` (self-contained, no `crate::` refs) · **Proof bin:** `client/src/enemy_ai_proof_main.rs` (bin `voxelforge_enemyai_proof`)

The bodies are Monanisa's (`enemies.rs`, see `docs/enemy-design.md`). The hit→die→respawn loop is Kevin's (`combat.rs`). This document specifies the **third leg**: how an enemy *behaves* — the state machine that will drive any body it is attached to.

---

## 1. Design goals

The shipped Guard Husk AI (`combat.rs` §4.1) is a single archetype: patrol → aggro → attack, one enemy at a time. The CEO brief ("enemies should look scary") and the three new bodies need behaviour that differentiates them at a glance, in motion, not just in silhouette:

1. **Three readable pursuit styles.** A player watching 5 seconds of gameplay, with the HUD hidden, should be able to name which enemy is which by *how it moves*.
2. **The alert beat stays.** The "it noticed me" freeze-then-face moment that `combat.rs` established is kept for every archetype — it is the single best tension beat in the current AI.
3. **Honest telegraphs, always.** Every attack locks its strike direction at the END of its windup and cannot turn afterwards. A dodge that leaves the locked line is a real dodge — no input-reading fake-outs. (Same contract the husk's swing already honors.)
4. **Motion only.** This module moves bodies and prints transitions. Damage, health, knockback, i-frames — all `combat.rs`'s. The lane fence is: *behaviour proposes, combat disposes.*

## 2. Archetypes (behaviour) ↔ kinds (visual)

Behaviour is tuned against `Archetype`, deliberately **not** against `enemies::EnemyKind` — so the AI module compiles standalone in the proof bin, and so a future kind can reuse an existing archetype without touching this file.

| `enemies::EnemyKind` | `Archetype` | The read |
|---|---|---|
| Ghoul Reaver | **Swarm** | many · fast · erratic |
| Bone Sentinel | **Bruiser** | slow · relentless · never forgets |
| Thornclaw Stalker | **Pouncer** | patient · circles · dashes through |

The mapping itself is one match (`archetype_for` in the proof bin; the combat lane's spawn path in the real game).

## 3. State machine

All archetypes share this skeleton; the tuning table decides what each state means.

```text
                 ┌──────────── deaggro ─────────────┐
                 ▼                                   │
 Patrol ──aggro──▶ Alert ──beat──▶ { Chase | Advance | Stalk }
 (wander 'round    (freeze,      │                      │
  home)             face player)  │                      │
                                  ▼                      ▼
                    Swarm: Chase ◀─zigzag seek   Pouncer: Stalk (orbit r=6.5)
                              │ strike_range          │ dist<5.5 or stalk timer
                              ▼                       ▼
                    Bruiser: Advance ◀─slow,     Crouch (freeze 0.45s)
                        keep_dist 2.1                │ aims at the LAST instant
                              │ strike_range          ▼
                              ▼                   Pounce (dash 9 u/s,
                          Windup (0.25 / 0.65s)    overshoot, re-orbit)
                              │ lock dir at end        │
                              ▼                       ▼
                          Strike (dash, committed) ▶ Recover (0.7–1.2s opening)
                              │                       │
                              └────── back to pursuit ┘
```

**The two contract rules:**

- **Alert beat** (all): on aggro, the enemy freezes `alert_beat` seconds and turns to face the player. Nothing moves. The stillness is the tell.
- **Lock-at-end** (all attacks): during windup/crouch the enemy still tracks the player with its facing; the strike **direction** is only sampled when the windup expires. Dodge after the lock = free. This is what makes the telegraph honest.

## 4. Tuning table (v1 — the numbers the clip was graded against)

| | Swarm | Bruiser | Pouncer |
|---|---|---|---|
| patrol speed | 1.2 | 0.8 | 1.4 |
| aggro / deaggro | 12 / 22 | 9 / **∞** | 14 / 24 |
| alert beat | 0.40s | 0.55s | 0.45s |
| pursuit | seek **4.4** + zigzag weave | advance **1.7**, hold at 2.1 | orbit **2.7** at r 6.5 |
| telegraph | windup 0.25s | windup **0.65s** (longest) | crouch 0.45s |
| strike | 6.0 u/s × 0.22s | 5.0 u/s × 0.28s | **9.0 u/s × 0.35s** |
| recover | 0.7s | 1.1s | 1.2s |

Rationale for the load-bearing numbers:

- **Swarm chase 4.4 > player flee 3.8.** A chase that starts inside aggro *must* end in contact — the archetype's threat model is "you cannot outrun many." The clip's flee phase proves it closes.
- **Bruiser deaggro = ∞.** Once it has seen you, the slow advance never stops. Slow (1.7) is scary only if it is also inevitable.
- **Pouncer dash 9.0 × 0.35s = ~3.2u of travel**, deliberately overshooting past the player: it lands *behind* you, then re-orbits. Standing still is what it punishes.
- **Separation 1.3u** between minds, pushed at 2× — a swarm converges into a rough ring, never one overlapping blob. O(n²) over single-digit counts; revisit only when a map fields dozens.

## 5. Module contract (for the combat lane's wiring)

`enemy_ai.rs` exposes exactly this, nothing else is needed:

```rust
app.add_plugins(EnemyAiPlugin);                      // Arena + seeded AiRng + the system
commands.entity(player).insert(AiPlayer);            // what the minds hunt
enemy_ai::attach_mind(&mut commands, body_root, id, // drive any body root
                      Archetype::Bruiser, spawn_pos);
```

- `AiPlayer` is a marker: put it on the player transform entity. No other read of the game world happens — no health, no input, no lock-on.
- Ground is `y=0` (flat-arena contract, clamped to `Arena.half`). When wiring into the real map, replace the clamp with `combat.rs`'s ground query — that edit belongs to the wiring, not this file.
- `AiRng` is seeded (`0x52505345`, via `Default`): proof clips are reproducible run-to-run.
- The system is one single pass over `iter_mut()` — peer positions for the O(n²) separation are snapshotted before the first write each frame, so no borrow-splitting two-pass collection exists anymore.
- Transition prints (`AI id=… state -> state dist=…`, `TELEGRAPH->COMMIT locked=…`) are load-bearing: the proof greps them.

## 6. The proof (bin `voxelforge_enemyai_proof`)

`enemy_ai_proof_main.rs` links `enemies.rs` + `enemy_ai.rs` ONLY (same `#[path]` isolation pattern as `char_shot_main.rs`) — an enemy-AI proof cannot be blocked by, or block, another lane's mid-edit file.

The scenario (fixed, scripted, 4 phases over ~22s):

1. **stand** (0–2s) — player idles far off; all four patrol.
2. **approach** (2–7s) — player walks into the pack; alert beats fire per-enemy.
3. **flee** (7.8–14.5s) — player runs across the arena at 3.8 u/s; the swarm *must* close (4.4 > 3.8), the Bruiser never stops coming, the Pouncer orbits and dashes.
4. **cornered** (14.5s+) — player stops; gets surrounded, telegraphed, struck.

Cast: Reaver ×2 (Swarm), Sentinel ×1 (Bruiser), Stalker ×1 (Pouncer), spread wide so phase 1 reads as patrol, not a wall.

Outputs per run:

- `<dir>/f%04d.png` — one frame every other app frame (≈30fps clip material);
- `<dir>/trace.csv` — `frame,enemy,state,archetype,x,z,dist` every frame — the numeric truth the clip renders (state coverage + distances closed can be asserted from it without watching pixels);
- stdout — `AI …` transition lines.

Assemble the clip:

```bash
ffmpeg -framerate 30 -i _enemyai_frames/f%04d.png -c:v libx264 -pix_fmt yuv420p docs/assets/enemy-chase.mp4
```

## 7. การพิสูจน์ (proof, 2026-08-14)

สิ่งที่ถูกพิสูจน์ และหลักฐานที่ระบบจริงพิมพ์เอง (ไม่มี test driver พิมพ์ PASS แทน):

| ขั้น | คำสั่ง | ตัวตัดสิน |
|---|---|---|
| build | `cargo build --manifest-path client/Cargo.toml --bin voxelforge_enemyai_proof --target-dir target-rose` | `(Select-String '^error' rose-build.log).Count == 0` |
| รันจริง | `VOXELFORGE_AIFRAMES=docs/assets/ai/chase-frames VOXELFORGE_AILOG=docs/assets/ai/trace.csv target-rose/debug/voxelforge_enemyai_proof.exe` | โปรแกรมพิมพ์เอง `AIPROOF DONE frames=… captured=…` |
| คลิป | `ffmpeg -framerate 30 -i docs/assets/ai/chase-frames/f%04d.png … docs/assets/ai/enemy-chase.mp4` | ไฟล์ mp4 มีจริง + ขนาดไบต์ |
| contact sheet | เลือกเฟรมจาก `trace.csv` คู่ละ state จริง ห่างเวลาจริง (คู่ archetype ×3 + คู่ detect/chase/strike/retreat) แล้ว gamma-lift 0.32 เหมือนคลิป | `docs/assets/ai/ai-*.png` 14 แผ่น + `_paircheck_*.png` 7 แผ่น — แผนที่เฟรม↔trace↔diff% อยู่ที่ `docs/ai-evidence-stills-2026-08-16.md` (v2 หลังรีวิวตีกลับคู่ v1 ที่ห่างแค่ 2–4 เฟรม + md5 ซ้ำ) |

**เกณฑ์ผ่าน:** `trace.csv` ต้องมีทั้ง 4 สถานะหลักปรากฏ (grep จาก CSV ที่ sim เขียนเอง ไม่ใช่ที่ script เขียน) และบรรทัด `AI id=… TELEGRAPH->COMMIT` ต้องมีอย่างน้อยหนึ่งต่อ archetype ที่โจมตี — นั่นคือหลักฐานว่าทิศทางฟันถูกล็อคตอนท้าย windup จริง

## 7.5 Pose channel — ท่าขู่-ท่าเข้าตี (2026-08-16)

ปัญหา: telegraph เดิมอ่านไม่ออก — windup ดูเหมือน "ศัตรูหยุด" เฉยๆ (มีแค่ position+yaw)

กลไก (`enemy_ai.rs`, pure fn `pose_for(state, t, tuning)` — ไม่แตะ translation, ไม่มี RNG):

| state | ท่า | curve |
|---|---|---|
| Alert | สะดุ้ (หุบ 5% หนึ่งเฟรม แล้วคลายออก) | ease-out |
| Windup | ขมิบ: เอียงหลัง + หุบลึก-กว้าง | p² ease-in — กระชับถึงจุด commit |
| Crouch | หุบแรงและค้าง (freeze คือ tell) | ease-out แล้ว hold |
| Strike/Pounce | whip: จากท่าขมิบ → เอียงหน้าเต็ม + ยืดตัว | ease-out หนักหน้า (75% ในครึ่งแรก), ต่อเนื่องจาก coil ที่จุดต่อ |
| Recover | สปริงดับ สั่นผ่านตำแหน่งกลาง | e^(−4.5k)·cos(9k) เริ่มจากท่า strike พอดี |

Amplitude ต่อ archetype (`POSE` table ขนาน `TUNING`): Swarm เบา (lean_back 0.10 rad), Bruiser หนักสุด (0.22, coil sy 0.86), Pouncer เป็นลูกศร (lean_in 0.34, dash sy 1.16)

### การพิสูจน์ (2026-08-16, หลักฐานใน `docs/assets/ai/pose/`)

- **build:** `scripts/lane-build.ps1 -Lane rose -Bin voxelforge_enemyai_proof` → `VERDICT PASS` (gate ใหม่ post-93b9876): `BUILD_DONE exit=0 errors=0`, `GATE errors=0:True exit=0:True mtime_fresh:True`; `grep -c '^error' _rose_build.log` = 0
- **captures:** `VOXELFORGE_AIEND=2400 VOXELFORGE_AICAM=actor` ทั้งสองฝั่ง (before = pre-pose binary 117941c, after = 9809338) — รันละ 2401 เฟรม / 9604 แถว trace
- **contact sheets (ภาพคู่ก่อน/หลัง ต่อ archetype):** `pose-sheet_bruiser.png` / `pose-sheet_pouncer.png` / `pose-sheet_swarm.png` — หน้าต่างเฟรมถูกเลือกจาก trace ของแต่ละรันเอง (`sheet_pick.txt`) กรอบส้ม = commit (whip) Bruiser: coil เอียงหลัง-หุบ → whip เอียงหน้า → คลาย; Pouncer: crouch แบนลงชัด → ยืดพุ่ง; Swarm อ่านบางตามดีไซน์ท่าเบา (หลักฐานหลักอยู่สองตัวใหญ่)
- **purity (pose ไม่แตะพฤติกรรม):** ตัวตัดสิน = transition sequence ต้องตรงกันทุก index ใน common prefix — **ผ่านทั้ง A/B และ control** (ศัตรู 1-4: 98/106/44/58 transitions identical, ไม่มี divergence กลางเทป; `purity_gate.txt`, `purity_control.txt`) raw row-diff: A/B 9579/9604 **ต่ำกว่า** control (binary เดียวกันรันสองรอบ) 9597/9604 — wall-clock dt (vsync) ทำให้ทุกคู่รันต่างกันระดับนี้อยู่แล้ว (`rowdiff_numbers.txt`)

## 7.5 Pose channel — ท่าขู่-ท่าเข้าตี (2026-08-16)

ปัญหา: telegraph เดิมอ่านไม่ออก — windup ดูเหมือน "ศัตรูหยุด" เฉยๆ (มีแต่ position+yaw) `pose_for(state, t, tuning)` เพิ่มช่องทางท่าทาง: pure function ของ state+เวลาใน state ไม่แตะ translation ไม่มี RNG ขับที่ root เป็น pitch เฉพาะที่ (หลัง yaw) + squash/stretch scale:

| state | ท่า | curve |
|---|---|---|
| Alert | สะดุ้ 1 เฟรม (หุบ 5%+เงยหัวเล็กน้อย) แล้วคลาย | ease-out |
| Windup | ขมิบ: เอียงหลัง + หุบลึก-กว้าง | p² (กระชับถึงจุด commit) |
| Crouch | หุบแรงแล้วค้าง (freeze คือ tell ของ pouncer) | ease-out + hold |
| Strike/Pounce | whip: coil → เอียงหน้าเต็ม + ยืด | ease-out (75% ในครึ่งแรก) |
| Recover | สปริงดับ สั่นผ่านเป็นกลาง ต่อเนื่องจากท่า strike พอดี | e^(−4.5k)·cos(9k) |

Amplitude ต่อ archetype (`POSE` ตารางขนาน `TUNING`): Swarm เบา (lean_back 0.10 rad, coil 0.92) · Bruiser หนักสุด (0.22, coil 0.86) · Pouncer ลูกศร (lean_in 0.34, dash ยืด 1.16) — ค่าต่อเนื่องกันทุกจุดสลับ state (ไม่มีกระโดดค่า นอกจากความเร็วเปลี่ยนที่ whip ซึ่งคือจุดขาย)

### การพิสูจน์ (ทั้งหมดใน `docs/assets/ai/pose/`)

build: `lane-build.ps1 -Lane rose -Bin voxelforge_enemyai_proof` → `grep -c '^error'` = **0** (log `_rose_build_after.log`), cargo exit 0, exe relink จริง (mtime ขยับ, size เปลี่ยน)

captures: binary ก่อน/หลัง (source ที่ `9809338^` vs `9809338`) รันด้วย env เดียวกัน `VOXELFORGE_AIEND=2400 VOXELFORGE_AICAM=actor` (seed เดียวกัน RPSE, route เดียวกัน) ได้ trace ฝั่งละ 9,605 แถว (2,401 เฟรม × 4 minds)

**ภาพ (contact sheet 8 ช่องต่อ cycle เดียวกัน ก่อน/หลัง ตัดจาก trace ของแต่ละรันเอง):**
`pose-sheet_swarm.png` · `pose-sheet_bruiser.png` · `pose-sheet_pouncer.png` — แถวล่าง (posed) เห็น coil→whip→settle แถวบน (ก่อนแก้) ยืนนิ่งตลอด

**สัญญา purity (pose ไม่แตะพฤติกรรม) — วัด 3 ชั้นเทียบ control รันซ้ำ binary เดิม:**

| มิติ | control (binary เดิม ×2) | A/B (ก่อน vs หลัง) |
|---|---|---|
| transition common-prefix ต่อ enemy | 98/106/44/58 เหมือนเป๊ะ | 98/106/44/58 **เหมือนเป๊ะ** |
| max phase-aligned \|Δdist\| | 14.13 | **12.92** (≤ control) |
| row-diff ดิบ | ~100% ของแถว (17,486/8,749) | ~100% (19,158/9,605) |

คำอธิบาย row-diff: เครื่องมือ (wall-clock dt จาก vsync) ไม่ deterministic — **binary เดียวกันรันสองรอบก็ต่างกันเกือบทุกแถว** (ตำแหน่งทศนิยมที่ 2 ขยับตาม jitter ของ dt) ดังนั้น "bit-identical" พิสูจน์ไม่ได้แม้กับตัวเครื่องมือเอง หลักฐานที่มีความหมายคือ prefix identity + \|Δdist\| ที่**ไม่เกิน** noise floor ของ control — ผ่านทั้งคู่ (`purity_gate.txt`, `purity_control.txt`; ความยาว tape ต่างกัน ±4-6 transition ทั้งสองคู่ ทิศสุ่ม = ผลของ Σdt ต่างกัน ไม่ใช่พฤติกรรม)

scripts: `_rose_ai_pose_sheet.py` (ตัดหน้าต่างจาก trace, ป้ายทุกช่อง, กรอบส้ม=commit) · `_rose_ai_pose_purity.py` (prefix gate, ทดสอบบน control บวก/ลบแล้ว)

## 8. Open / next (combat lane's call)

- Damage windows: what a Strike/Pounce connect does (damage, poise, knockback) — `combat.rs`.
- Parry interplay: the Bruiser's 0.65s windup is sized to be parryable; hook it to `dodge_parry`.
- Real-map grounding and pathing around blocks (flat-arena clamp stands in for now).
- Pack tactics beyond separation (flank bias for Swarm) — deliberately not v1.
