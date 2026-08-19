# Character Design Review — 2026-08-18

> เตรียมให้ CEO ทบทวน character design รอบหน้า เขียนตรงๆ ไม่อวย ตรวจจากไฟล์จริงในโปรเจค
> ไม่ใช่จากเอกสารเก่า ขอบเขต: `docs/character-*.md` เท่านั้น ไม่แตะโค้ด/asset/map
> อ่านจบใน 5 นาที · ไฟล์ที่อ้าง: `character-bible.md`, `character-design.md`,
> `character-slots.md`, `player-character-art.md`, `auren-extra-parts-spec.md`, `character-sculpt-pass.md`

---

## TL;DR

- เอกสาร character **เก่งเรื่องระบบ** (teal = lore, emissive = narrative, silhouette-first) แต่
  **คนเล่นยังไม่เห็นระบบพวกนั้น** — ตัวจริงในเกมยังเป็น rig ง่ายๆ ไม่ใช่ตัวที่อนุมัติไว้
- **จุดอ่อนแรงที่สุด: ฮีโร่อ่านไม่ออกในระยะไกล** — Auren เกือบทั้งตัวเป็นโทน walnut/espresso
  โทนเดียวกับ NPC ทุกตัวและพื้นโลกโทนอุ่น จนทีมต้องแปะ teal gem clasp หนึ่งเม็ดเพื่อให้ hue แยก
  (วัดจริง hero 30.2° vs พื้นหลัง 33.4° = ห่างแค่ 3.3°) — หนึ่งเม็ดไม่ใช่ silhouette tell
  พระเอกจึงยังเป็นตัวละครที่จำยากที่สุดในชุด
- **Divergence ใหญ่ที่ยังไม่มีใครปิด:** เอกสารเล่า ladder ว่า Guard Husk → Warden → Architect
  แต่โค้ดจริง spawn ศัตรูอีกชุด (Sentinel + Reaver) และ **Warden/Architect ไม่มีโค้ดเลย** —
  มีแค่ concept art PNG

---

## Q1 — จุดอ่อนเทียบเกม AAA

### 1.1 Silhouette อ่านออกไหมในระยะไกล → ยังอ่านไม่ออก

เอกสารตั้งสเปคไว้ว่า "อ่านได้จาก black silhouette ที่ 12–16 blocks ใน <1 วินาที" แต่ยังไม่มีใคร
**gate** ข้อนี้จริง — มันเป็นคำสัญญา ไม่ใช่ตัววัด และเมื่อดูจากสิ่งที่สร้างจริง:

- **ตัวละครสูง ~2.5 blocks** ที่ระยะ lock-on 12–16 blocks = เล็กมาก พื้นที่ให้ silhouette ทำงาน
  เหลือน้อยกว่าเกม TPV ระดับ AAA (ที่ตัวละครมักสูง 1.8–2m บนเฟรมกว้างกว่า)
- **Auren มี read-point จริงแค่ 1 อัน** = half-cloak เส้นทแยงอันเดียวที่ขยับ ทรงผมอ่านไม่ออกเพราะ
  `#2A1B12` (ผม) บน `#3A2716` (cloak) = มืดบนมืด; กระเป๋า ember ตั้งใจให้เล็ก = มองไม่เห็น; teal gem
  clasp อันเดียว (เพิ่มใน A6 fix) ก็เล็กเกินจะอ่านที่ 12–16 blocks — เป็นตัวแยก hue ไม่ใช่ตัวแยกรูปทรง
- **จุดจำได้ของทั้งชุดส่วนใหญ่เป็น "ความสูง/ความหนา" ไม่ใช่ "รูปทรง"** — Toma = เตี้ย, Maren = กลม +
  ไม้เท้า, Husk = หนา สิ่งเหล่านี้ต่างกันด้วย *ขนาด* ซึ่งอ่อนกว่าการต่างด้วย *geometry* (หัว/อาวุธ/เงา
  เฉพาะตัว) มาตรฐาน AAA ใช้ 2–3 stacked tells (เช่น Geralt: ผมขาว+ดาบสองเล่ม+ตาแมว; Aloy: ผมแดง+
  headdress) ของเรามี ~1 tells ต่อตัว

### 1.2 สัดส่วนสื่ออารมณ์ไหม → sculpt pass ดีจริง แต่ยังไม่ถึงคนเล่น

`character-sculpt-pass.md` (5.5 heads, shoulder:waist 1.97, เพิ่ม diagonal/taper/หน้า 24 boxes)
**ทำได้จริงและถูกต้อง** — แต่ทั้งหมดอยู่ใน `AUREN_BODY` (characters.rs) ซึ่ง render เฉพาะ charshot
กับ scene NPC ตัวที่ผู้เล่นเล่นจริงยังเป็น rig ของ `anim.rs` (11 ชิ้น core + `extra_parts`: ผม 4
กล่อง, กระเป๋า ember, teal gem clasp, cloak) — "alert, forward-leaning survivor" ที่ CEO ให้แก้
มีอยู่ใน concept art กับ sculpt body แต่**ยังไม่ใช่ตัวที่อยู่ในมือผู้เล่น** — งานอารมณ์/ท่าทางยัง
ค้างอยู่ครึ่งทางระหว่างสาม representation

### 1.3 จุดเด่นให้จำได้ → มีระบบดี แต่ฮีโร่ได้น้อยที่สุด

- ระบบ **teal = corruption** (สี teal ยิ่งเข้ม = ยิ่งหลุดจากความเป็นมนุษย์) เป็นไอเดียที่จำได้และ
  เป็นระบบ ไม่ใช่แค่ palette — ดีที่สุดในเอกสาร
- **teal ในเกมจริงมีอยู่จุดเดียว = gem clasp บนไหล่ฮีโร่** (เพิ่มใน A6 fix เพราะวัด hue ห่างพื้นหลังแค่
  3.3°) — ไม่ใช่ ladder "teal = corruption" ที่เอกสารเล่า: teal กลายเป็น accent ของฮีโร่เอง ไม่ใช่ภาษา
  ของตัวร้าย และ Warden/Architect ที่จะใช้ ladder นั้น = 0 โค้ด
- ตัวร้าย: blank-visor Husk "ไม่มีใครอยู่บ้าน" = ไอเดีย shape ที่แข็งที่สุดในเอกสาร และ**ถูกแทนที่ในโค้ด**
  ด้วย Sentinel (eye-slits เรืองแสงฟ้าเย็น `#8FD6FF`) — ดู Q2

**สรุป Q1:** ระบบคิดดี งาน sculpt ดี แต่ (ก) ฮีโร่กลืนกับโลกโทนอุ่น (ข) silhouette ต่างกันด้วย
ขนาด ไม่ใช่รูปทรง (ค) ไอเดียที่จำได้ที่สุดสองอัน (blank-visor, teal ladder) ยังไม่ถึงโค้ด

---

## Q2 — มีจริงในเกม vs ยังเป็นแค่เอกสาร

ตรวจจากไฟล์จริง (`client/src/`, `docs/assets/characters/`, `assets/textures/character/`)

### ✅ มีจริง (มีโค้ด / รันได้ / มีไฟล์)

| ของ | อยู่ไหน | หมายเหตุ |
|---|---|---|
| Auren sculpt body (113 boxes, หน้า+rot+taper) | `characters.rs:208` `AUREN_BODY` | จริง แต่ render เฉพาะ charshot + scene NPC |
| ระบบ equipment (20 parts / 6 slots / 4 presets) | `equipment.rs` | จริง แต่**ผูกกับ charshot เท่านั้น** ผู้เล่นในเกมไม่ได้ใช้ |
| ผม + กระเป๋า ember + teal gem clasp บนผู้เล่น | `anim.rs:611` `extra_parts(RigLook::Player)` → wire ผ่าน `attach_rigs` (`anim.rs:1397`) | **มีจริงในเกมแล้ว** — ผม 4 กล่อง, กระเป๋า 2 กล่อง, gem clasp 1 เม็ด |
| Guard Husk (Garren) | `characters.rs` `GARREN` + `combat.rs` HuskState + `enemies.rs:614` legacy | มี body + AI จริง |
| ศัตรูชุดใหม่: Sentinel / Reaver / Stalker | `enemies.rs:194` `EnemyKind` | **Sentinel + Reaver ถูก spawn จริง** ใน encounter (`main.rs:863`) |
| Maren + Toma (body จริง) | `characters.rs` `MAREN`/`TOMA`; spawn ที่ `scene.rs:810/835` | มีในเกมแล้ว (ตรงข้ามกับ lane note เก่า) |
| Concept art ทั้ง 6 ตัว (Auren/Husk/Maren/Toma/Warden/Architect) | `docs/assets/characters/*.png` | ไฟล์ครบ + before/after + charmask |
| Texture swatch 5 ชิ้น (boots/cloak/pants/shirt/skin) | `assets/textures/character/` | มีไฟล์ แต่**ไม่มีโค้ดอ้างถึงเลย** (orphan จาก LOD body ที่ยกเลิกไป) |

### ❌ ยังเป็นแค่เอกสาร (ไม่มีโค้ด / ยังไม่ต่อ)

| ของ | หลักฐาน |
|---|---|
| **The Warden (4–5 blk)** | concept art อย่างเดียว — grep ทั้ง client/sim/server ไม่เจอ identifier เลย |
| **The Architect (8–10 blk)** | concept art อย่างเดียว — เหมือนกัน = **teal ladder ทั้งสายไม่มีในเกม** |
| กระเป๋า ember **เรืองแสง (bloom)** | mesh มีแล้วในเกม แต่ `PartSpec` ไม่มี field emissive — coal `#FFD98A` สว่างแต่ไม่ bloom (`auren-extra-parts-spec.md` open note #1) |
| Cloak notch asymmetry | deferred ใน spec doc |
| `ClothTeal` / `TrimGold` 2 Surf variants | spec พร้อม แต่ยังไม่เติมใน `equipment.rs` |
| AUREN_BODY เป็น player mesh | ตัดสินใจแล้ว (2026-08-18) แต่**ยังไม่ wire** — ผู้เล่นยังเป็น `Capsule3d` (`main.rs:1028`) + rig ของ `anim.rs` |

### ⚠️ Divergence ที่สำคัญที่สุด (ต้องให้ CEO รู้ก่อนประชุม)

`character-bible.md` §5 เขียนว่า Warden/Architect **"already built, already validated"** — นั่นคือ
concept art เท่านั้น ไม่ใช่โค้ด และที่หนักกว่า: เอกสารล็อก horror beat ของ Guard Husk ว่า
**blank visor ไม่มีตาไม่มีปาก** แต่ `enemies.rs` สร้าง `Sentinel` ตั้งชื่อว่า **"Guard Husk replacement"**
แล้ว encounter จริง spawn Sentinel+Reaver — แปลว่า **มีภาษาการออกแบบศัตรูสองชุดวิ่งคู่กันอยู่**
(blank-void ของ Monanisa vs eye/claw horror ของ enemies.rs) และ **ภาษาสีก็ไม่ตรงกัน**: ศัตรูในเกมใช้
glow เขียว (`GhoulGlow #7CFF6E`) / ฟ้าเย็น (`SentinelGlow #8FD6FF`) / แดง (`StalkerGlow #FF3B1E`) —
ไม่มีตัวไหนใช้ ladder amber→teal ที่เป็นแกนของ character-bible เลย (teal ในเกมอยู่ที่ gem clasp ฮีโร่
จุดเดียว)

---

## Q3 — 3 ทิศทางดีไซน์ให้ CEO เลือก

### ทิศทาง A — "ขัดให้คมก่อน ต่อสิ่งที่ตัดสินไปแล้ว"
เก็บ roster ปัจจุบัน แต่แก้จุดอ่อนที่แรงสุด: เพิ่ม read-point ให้ฮีโร่ (ทรงผม/ผิวแยกค่าแสงให้อ่านไกล,
อาวุธ signature ที่อ่านจาก black silhouette, ตัวเน้นสีอุ่นที่ไม่ใช่ teal), และ wire `AUREN_BODY` เข้า
FlyCam ตามที่ตัดสินไว้แล้ว (ยังค้างอยู่) — ผม/กระเป๋า/gem clasp อยู่ในเกมแล้ว ไม่ต้อง ship ซ้ำ
- ✅ ถูกสุด เร็วสุด ต่อยอดงานที่มีแล้ว 90% ไม่เสี่ยง
- ❌ ไม่แก้ราก "ฮีโร่กลืนกับโลกโทนอุ่น" (gem clasp หนึ่งเม็ดไม่พอ) และ teal-dormant ยังจำกัดความเด่นของพระเอก

### ทิศทาง B — "ลงทุนกับ teal-escalation ให้เป็นกระดูกสันหลังจริง"
เลือกภาษา roster ภาษาเดียว ปิด divergence แล้ว**สร้าง Warden/Architect ให้เป็นของจริง** ให้ teal =
ตัวร้ายเป็นแกนที่เล่นเห็นตั้งแต่ Husk ตัวแรก และให้ Auren ได้ teal กลับหลังจุดเผยตัว (dormant→ignite)
เป็น color arc ของพระเอก
- ✅ ระบบ teal = ไอเดียที่จำได้ที่สุดในเอกสาร และได้ payoff ที่ตั้งใจไว้; ปิด divergence ที่ค้างอยู่
- ❌ งานใหม่หนักสุด (สร้าง boss 2 ตัว + ตัดสินใจ merge roster ที่ขัดกัน)

### ทิศทาง C — "รีบูทด้วย silhouette-first: รูปทรงเหนือสี"
เลิกพึ่ง palette (ซึ่งใกล้ monochrome โดยตั้งใจ) แล้วให้ทุกตัวมี **shape-tell ที่รอด black-cutout**:
Auren ได้ head/weapon silhouette เฉพาะตัว, Husk เอา blank-void กลับมาเป็นหลุมในเงาจริง,
Warden/Architect ใช้ orbiting-fragment geometry (ไอเดีย geometry ที่ใหม่ที่สุดในเอกสาร) — และตั้ง
black-silhouette test เป็น **gate** ที่วัดจริง ไม่ใช่คำสัญญา
- ✅ โจมตีรากจุดอ่อนจริง (silhouette ต่างด้วยขนาด ไม่ใช่รูปทรง) และไอเดีย orbiting/void มีอยู่ในมือแล้ว
- ❌ งานมาก และ golden-hour โทนอุ่น + ค่าแสงต่ำต่อสู้กับ "shape first" — อาจต้องยอมแหกกฎ 85%-warm
  เพื่อให้เงาแยกตัวได้

**ฉันโน้มไปทาง B + A** — เอา B เป็นทิศทาง (ปิด divergence + สร้าง teal ladder ที่ตั้งใจไว้จริง)
แล้วคว้า quick win จาก A ระหว่างทาง (wire AUREN_BODY + เปิด emissive กระเป๋า) เพราะปัญหาใหญ่สุดตอนนี้
ไม่ใช่ "งานไม่พอ" แต่เป็น "เอกสารกับเกมเดินคนละทาง" — ปิดตรงนั้นก่อน งาน sculpt ที่ทำดีแล้วจะได้
ขึ้นเฟรมจริง ไม่ใช่ค้างอยู่ใน charshot

---

*ผู้เขียน: Sun · ตรวจกับไฟล์จริง 2026-08-18 · ยังไม่ push/ไม่แตะโค้ด — ขอให้ CEO ตัดสินใจทิศทางก่อน*
