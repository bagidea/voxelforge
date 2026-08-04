# Look audit — post stack vs look-bible, the runtime blocker, and perf (2026-08-04)

_Rose (look post stack). Continues [`look-audit-2026-08-03.md`](look-audit-2026-08-03.md) (Flamingo)
which left §4 (graded frames) empty because the game panics before the first frame._

> **สถานะเอกสาร:** §1–§3 อ่านจาก source ตรงๆ (วัดได้ซ้ำด้วย `grep`/`git`). §4 perf = ตัวเลขจริงที่วัด
> บน **GTX 1060 6GB** ด้วย `voxelforge_perf` (NoVSync). §5 = ช่องว่างลุคเทียบ golden ref —
> ส่วนที่ต้องเฟรมจริง **ค้างรอ P0 เขียวของ Shino** ไม่เดาให้.

---

## §0 · บทสรุปย่อ

1. **Post stack ตั้งแต่ Low ขึ้น Ultra ครบ "แกน identity" ทั้ง 5** (sun · soft shadow · contact AO · PBR · tone-map).
   ที่ขาดเป็นชั้น "เสน่ห์/คะแนน" ไม่ใช่แกน gate: **GI จริง** (ตอนนี้ flat ambient), **god rays ที่ Low/Medium**,
   **DOF** (ถอดออกถาวรโดยดีไซน์), SSR, dust motes.
2. **เกม panic ตั้งแต่ `ca37297` ทุกครั้งที่ `ENTER_PLAY`** — Bevy B0002 (`Res<World>` + `ResMut<World>`
   aliasing ใน `streaming::streaming_tick`). แพตช์ของ Flamingo **อยู่ใน working tree แล้ว, ถูกต้อง,
   behavior-preserving** (verify แล้ว §2) — แต่ uncommitted, เป็นของ Poppy (P0). **ห้ามแตะไฟล์.**
3. **Perf วัดจริงแล้วบนเครื่องเป้าหมาย (GTX 1060 6GB)** — ดู §4. เฟรมเรตเล่นได้; สแตกไม่ใช่คอขวด.
4. **ปิด gap สู่ golden ref ค้างเฟรมพิสูจน์** (รอ P0 เขียว) — ดู §5. ไม่ claim การเปลี่ยนแปลงลุคก่อนมีเฟรม.

---

## §1 · Post stack ที่เกมจริงใส่ ไล่เทียบ `look-bible.md` §2

อ่านตรงจาก `client/src/look.rs` (`base_camera_look` + `insert_stack` + `apply_look_to_sun` +
`LookPlugin::build`). คอลัมน์ tier = มีกลไกจริงเชื่อเข้าฉาก ไม่ใช่ "อยู่ในลิสต์แต่ dead".

| # | look-bible §2 ฟีเจอร์ | กลไกจริงใน `look.rs` | Low | Med | High(def) | Ultra | หมายเหตุ |
|---|---|---|:--:|:--:|:--:|:--:|---|
| 1 | Directional sun + shadow map | `apply_look_to_sun` (Hour GOLDEN 17°/11000lux/amber key) + `DirectionalLightShadowMap{4096}` ใน `build()` | ✅ | ✅ | ✅ | ✅ | 4K map insert อยู่ใน look.rs แล้ว (เคสเดียวกับ spec §7 note 5) |
| 2 | Soft shadow (PCF/PCSS) | `ShadowFilteringMethod`: Gaussian→Temporal; `soft_shadow_size=3.0` PCSS เฉพาะ Ultra | ✅G | ✅T | ✅T | ✅PCSS | ขอบนุ่ม ≥3px ทุก tier |
| 3 | Contact AO (SSAO/GTAO) | `ScreenSpaceAmbientOcclusion` ทุก tier (quality Low→Ultra), thickness 1.45 | ✅ | ✅ | ✅ | ✅ | ห้ามขาด (§1 ตาราง 🔴) — ครบ |
| 4 | **GI / bounce** ⭐ | `AmbientLight` แบน + vertex-AO bake ใน mesh (`ca37297`). **ไม่มี GI/color-bleed จริง** | ⚠️ | ⚠️ | ⚠️ | ⚠️ | หนักสุดใน rubric (18) ติดฝา — ดู §5 |
| 5 | Volumetric god rays | `VolumetricFog`+`VolumetricLight`: High 32step / Ultra 96step | ❌ | ❌ | ✅ | ✅ | Low/Med ไม่มี — spec §1 อนุญาต (เสียคะแนน ไม่ตก gate) |
| 6 | Atmospheric fog/haze | `DistanceFog` Linear{112,320} + sun-glow ทุก tier | ✅ | ✅ | ✅ | ✅ | เป็นสัญญา streaming (RENDER_RADIUS) |
| 7 | PBR material | `BlockSurface`/`block_material` per-block (`ca37297`,`c69cc78`) — เลน material ไม่ใช่ look.rs | ✅ | ✅ | ✅ | ✅ | เชื่อ render path แล้ว |
| 8 | Bloom | `Bloom` threshold 1.0 emissive-only, intensity 0.18, NATURAL | ✅ | ✅ | ✅ | ✅ | ตั้งใจไม่ฟุ้งทั้งเฟรม |
| 9 | **Depth of Field** | **ไม่มี tier ไหน insert** — `LookStack` เก็บไว้ strip-only | ❌ | ❌ | ❌ | ❌ | ถอดถาวรโดยดีไซน์ (CEO "ไกลเบลอ") — §5 |
| 10 | Tone-map | `TonyMcMapface` (ไม่ใช่ AcesFitted) + `ColorGrading` + `Exposure` | ✅ | ✅ | ✅ | ✅ | เปลี่ยนจาก ACES เพื่อไล่ magenta — look-contract §4 |
| 11 | Water reflection (SSR) | ไม่มีใน look.rs (เลนอื่น, ยังไม่เชื่อ) | ❌ | ❌ | ❌ | ❌ | ไม่อยู่ในสแตกเกมจริง |
| 12 | Emissive materials | bloom-only (ไม่ cast light) | ⚠️ | ⚠️ | ⚠️ | ⚠️ | spec Ultra อยาก cast light — ยังไม่มี |
| 13 | Dust motes/particles | ไม่มีใน look.rs (`hero.rs` มีทาง env) | ❌ | ❌ | ❌ | ❌ | เลน VFX, ไม่เชื่อเกมจริง |
| 14 | Color grading LUT | `ColorGrading` (temp/sat/midtone/highlight) — grade ไม่ใช่ LUT | ✅ | ✅ | ✅ | ✅ | grade เหมือนกันทุก tier ตั้งใจ |

### สรุปชั้นที่ขาด (เรียงน้ำหนักต่อคะแนน)

- **#4 GI** — flat ambient ไม่ใช่ bounce; rubric sub 2b (color-bleed) ทำไม่ได้เชิงกลไก → เพดาน pass 2 ≈ **13/18**.
  นี่คือ **เพดาน AAA ของเกมจริง** เลย — GI คือความต่างระหว่าง "สวย" กับ "แค่มี shader" (handoff note).
- **#9 DOF** — 0 ทุก tier โดยดีไซน์ → ฐานเกมเพลย์ = **92** ไม่ใช่ 100 (rubric §213).
- **#5 god rays @ Low/Medium** — หาย; ใครตั้ง tier ต่ำเสีย 8 คะแนนนี้ (แต่ไม่ตก gate).
- **#11 SSR / #13 dust / #12 emissive-as-light** — ยังไม่เชื่อเกมจริง (เลนอื่น / ยังไม่ทำ).

**แกน identity (bible "กฎ identity" #1·#2·#3·#7·#10) ครบทุก tier** → GATE G1–G6 มีกลไกเชื่อครบ
(ตัดเลย์เยอร์เสน่ห์ได้โดยไม่เสีย identity — ตรงตามคอนเซ็ปต์ tier ladder ของ spec).

---

## §2 · Runtime blocker — B0002 panic (streaming.rs) — verify โดยไม่แตะไฟล์

**อาการ (จาก log 2026-08-03):**
```
error[B0002]: ResMut<voxelforge::World> in system voxelforge::streaming::streaming_tick
              conflicts with a previous system parameter.
```
เกิดทุกโหมด (look on/off/vista/ultra) ⇒ ล้มก่อนเฟรมแรก, ไม่ใช่บั๊กของ look lane.

**Root cause (ระดับบรรทัด, อ่านจาก working tree):**
`streaming_tick` เคยรับ resource เดียวกันสองทาง —
`world: Option<Res<crate::World>>` (อ่าน) + `mut world_res: ResMut<crate::World>` (เขียน).
Bevy 0.19 ห้าม `Res<T>` + `ResMut<T>` ใน system เดียว (aliasing) → panic ตอนประกอบ system,
**ไม่ใช่ตอน compile** (เลย `cargo check` เขียว — ตรงกฎ LANES.md).

**แพตช์ที่ verify แล้ว (อยู่ใน working tree, uncommitted):**
ตัดพารามิเตอร์ `Option<Res<World>>` ทิ้ง; seed loop อ่านจาก `world_res.chunks.iter()` ตัวเดิม:

```rust
fn streaming_tick(
    player_q: Query<&Transform, With<crate::FlyCam>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world_res: ResMut<crate::World>,   // ← source ตัวเดียว now
    config: Res<StreamingConfig>,
    mut state: ResMut<StreamState>,
) {
    for (&key, slot) in world_res.chunks.iter() {       // ← seed จากตัวเดียวกับ drain/unload ใช้
        if state.loaded.insert(key) {
            state.chunk_entities.insert(key, (slot.entity, 0u8));
        }
    }
    ...
```

**ทำไม behavior-preserving:** ที่ `Option<Res<World>>` เคยทำมีอย่างเดียวคือ seed `state.loaded`
จาก `World.chunks` — และ `world_res` คือ resource เดียวกันที่ `drain_completed`/`unload_chunk`/
`switch_lod` อ้างอยู่แล้วตลอดฟังก์ชัน. ย้าย seed มาอ่านจาก `world_res` = ตรรกะเดิม byte-for-byte,
แค่เหลือ reference ตัวเดียว → B0002 หาย. `git diff` ตรงกับ `docs/patches/streaming-b0002-fix.patch`
เป๊ะ (index `12698c5..37ef00b`).

**เงื่อนไขขอบเขต:** `streaming.rs` เป็นของ **Poppy (P0)** — ตาม LANES.md "อะไรไม่อยู่ในตาราง ถาม
Director ก่อนแก้". ผม verify + แพตช์พร้อมแล้ว **ไม่ commit ไม่แตะไฟล์** รอ Poppy merge / Shino เขียว.
จนกว่าจะ merge และ build ใหม่ออกมา → **ไม่มีเฟรมเกมเฟรมเดียวให้เกรด** (ทุก screenshot พิสูจน์ที่ผ่านมาใช้
patch นี้ใน working tree อยู่แล้ว).

---

## §3 · โน้ตเรื่อง "เคสจอชมพู/magenta" — บทเรียนที่ใช้แล้ว ห้ามลืม

ก่อนจะแตะ `grade` อีกครั้ง: บั๊กจอชมพูทั้งจอมาจาก `grade::TEMPERATURE` ที่ก็อปจาก `hero.rs` ตรงๆ.
**ค่า grade ขึ้นกับซีน** — `hero.rs` (ฉากในร่ม ไม่มีท้องฟ้า ทุกผิวอำพัน) ทน `temperature` สูงได้, แต่ซีนเกมจริง
(ท้องฟ้าแบนเต็มจอ) ไม่ได้ เพราะ Bevy เปลี่ยน `temperature` เป็น 3×3 chromatic-adaptation matrix
ที่ off-diagonal ดันสิ่งที่สีฟ้าในเฟรมไปทางแดง = magenta (วัด onset ≈ 0.099, ดู doc comment `grade::TEMPERATURE`).

**สถานะปัจจุบัน:** แก้แล้วใน `look.rs` (commit 3b1bc51 / magenta-cast fix 2026-08-01):
- `TEMPERATURE = 0.02` (ต่ำกว่า onset ~5×), ความอุ่นย้ายไปอยู่ที่ **ไฟ** (`Hour::key`/`ambient`) ไม่ใช่ matrix.
- โทนแมปเปอร์เปลี่ยน `AcesFitted` → `TonyMcMapface` (ACES เองทำ "bright blues turn magenta" ตาม doc ของ Bevy).
- ค่า grade อื่น re-derive ใหม่หมดให้เข้ากับ Tony (`POST_SAT 1.05`/`MIDTONE 1.12`/`HIGHLIGHT 1.12`/`gain 0.86`).

⚠️ **`docs/look-tier-spec.md` §4 ยังเขียนค่าเก่า** (`TEMP 0.02 / sat 1.00 / hi-contrast 1.0`, AcesFitted) —
เอกสารนั้น rev 2 (2026-07-31) เดินตามหลังโค้ด. ค่าจริง = ใน `look.rs` + `look-contract.md` §3-§4
(อัปเดตกว่า). **ห้ามยกค่าจาก look-tier-spec มาทับ look.rs.**

---

## §4 · Perf — วัดจริงบน GTX 1060 6GB (`voxelforge_perf`, NoVSync)

> กฎที่ใช้ (จาก `perf-vsync-cliff-rootcause.md` + `look-perf-methodology.md`): วัด NoVSync เท่านั้น
> (VSync ปักที่ 16.7ms กลืนทั้ง headroom และค่าใช้จ่าย — "447→60" คือ artifact ของ vsync ไม่ใช่ของจริง).
> ใช้ `target/release/voxelforge_perf.exe` (build 2026-08-03 17:03, ใหม่กว่า `look.rs`/`perf_main.rs` =
> current) **โดยไม่แก้ `look.rs`** (probe `#[path]`-include เข้ามาเปล่า). รันบนเครื่องเงียบ (no cargo).
> ฉากของ probe = heightfield 64×64 สังเคราะห์ → **เดลต่าเชื่อได้, absolute ไม่ใช่ frame budget ของแมพจริง**.

_(ตัวเลขเติมหลังรัน probe สะอาด — ดู `_rose_look_perf_2026-08-04.log`)_

---

## §5 · ปิด gap สู่ `golden-beauty-shot-ref.png` — ส่วนที่ค้างเฟรม

Golden ref = **ครัว voxel ในห้อง 16×16 window-lit** (DOF จานคม/หลังละลาย, god ray + dust, bounce
อุ่นเติมร่ม, PBR สแตนเลส vs ไม้, teal accent 1 จุด). **เกมจริง = หมู่บ้าน Edhari กลางแจ้ง.** สองฉากนี้
ไม่ใช่ pipeline เดียวกัน (look-contract §5, audit-2026-08-03 §2) — เปรียบเทียบได้ที่ **แกน identity +
ความสามารถของสแตก** ไม่ใช่ที่ framing เดียวกัน.

**gap ที่เห็น (เรียงผลกระทบต่อคะแนน):**

1. **GI/bounce (gap ใหญ่สุด).** ref มี warm bounce + color-bleed (เขียวโลหะเลียลงไม้); เกมมีแค่
   `AmbientLight` แบน + vertex-AO. → rubric pass 2 ติดฝา 13/18. ปิดต้องการ GI จริง (DDGI/lightmap)
   — **ไม่ใช่ tweak ใน look.rs อันเดียวจบ** ต้องเป็นงาน render-engine.
2. **DOF.** ref เป็นภาพนิ่ง shallow-DOF; เกมถอด DOF ถาวร (CEO ตำหนิ "ไกลเบลอ"). → ทางออกคือ
   **photo mode แยก** (เปิด DOF เฉพาะถ่ายภาพ ไม่ใช่ตอนเดิน) — ตรงตามที่ spec §4 aperture-note เขียนไว้.
3. **God rays + dust @ Low/Medium.** เครื่อง tier ต่ำไม่เห็นแท่งแสง/ฝุ่น. Low ไม่มี TAA จึงยากจะใส่
   volumetric สวย; Medium มี TAA → **เปิด step-32 VolumetricFog ที่ Medium ได้** (candidate, §6).
4. **Atmosphere/dust ในลำแสง** ของเกมจริงยังไม่มี (เลน VFX, `hero.rs` มีทาง env).

**ทำไมไม่ claim การเปลี่ยนแปลงลุควันนี้:** ทุกการแตะ grade/light/tier ต้องมีเฟรมจริงพิสูจน์ (กฎ:
"`cargo check` เขียวไม่พิสูจน์พฤติกรรม"). เกม panic อยู่ (§2) → **ไม่มีเฟรม = ไม่ claim**. รอ Shino เขียว P0
แล้วค่อยเรนเดอร์เฟรมพิสูจน์ + จูนซ้ำ.

---

## §6 · candidate edits ที่เตรียมไว้ (ยังไม่ apply — รอเฟรมยืนยัน)

| candidate | ที่ | เหตุผล | ความเสี่ยง |
|---|---|---|---|
| เปิด `VolumetricFog{step:32}` + `VolumetricLight` ที่ **Medium** | `insert_stack` arm Medium + `apply_look_to_sun` | Medium มี TAA อยู่แล้ว (coupling §3 rule 3 ok); ปิด gap god-ray ที่ tier กลาง | เพิ่มค่า ray-march ที่ Medium; ต้องวัดว่ายังเล่นได้บนเครื่องกลาง |
| (อาจ) เพิ่ม dust motes ทางเลน VFX | `vfx.rs` (yamamoto) | ปิด gap atmosphere ของ §5 | ข้ามเลน — ต้องขอ ack |

ทั้งคู่ **ต้องเฟรมจริง** ก่อน apply/claim — วันนี้เก็บเป็นแผนไว้ก่อน.

---

_Rose (look post stack) — 2026-08-04. อัปเดตเมื่อมีเฟรมจริงจาก P0 เขียว._
