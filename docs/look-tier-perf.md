# Look Tier Perf — ตัวเลขที่เมนู Settings ของ Shiba อ้างได้

> **เจ้าของเอกสาร:** Rose (look lane) · **ผู้ใช้งานหลัก:** Shiba (Settings menu)
> **สร้าง:** 2026-08-01
>
> อ่านคู่กับ: [`look-tier-spec.md`](look-tier-spec.md) §2 (map tier→component,
> single source of truth ว่าแต่ละ tier เปิด/ปิดอะไร) ·
> [`look-perf-methodology.md`](look-perf-methodology.md) (นิยาม metric ที่ละเอียด +
> วิธีอ่านตัวเลขไม่ให้ผิด) · โค้ด: `client/src/look.rs::insert_stack` ·
> probe: `client/src/perf_main.rs` · runner: `scripts/rose_look_tier_bench.sh`

---

## 0. สถานะ ณ ตอนนี้

ช่องตัวเลขยังเป็น *รอวัด* ทุกช่อง เพราะยังวัดไม่ได้: **Gate 3 ของ Yamamoto กำลังใช้ GPU**
(เช็ค `ls docs/assets/gate3/*.png` — ต้องครบ ≥3 ไฟล์). probe เปิดหน้าต่างเรนเดอร์ จึงแย่ง GPU
กับการยิง Gate 3 ไม่ได้ กฎของ Director: **ห้ามรันจนกว่า Gate 3 จะเสร็จ**.

สิ่งที่ทำไปแล้วระหว่างรอ (CPU ล้วน ไม่แตะ GPU):
- เพิ่ม **1% low** เข้า probe (`perf_main.rs`) — probe เดิมส่งแค่ p50/p95/mean ซึ่งเมนู Settings
  ใช้ไม่ได้โดยตรง (`fps avg` + `1% low` คือคู่ที่ผู้เล่นคุ้น). แกะนี้ยัง **ไม่ได้พิสูจน์ว่าคอมไพล์ผ่าน**
  (ดูข้อ 2 ด้านล่าง)
- สั่งบิลด์ probe เข้า `target-rose` แล้ว 2 ครั้ง แต่ **ยังไม่เคยเสร็จ**: ครั้งแรกถูก kill กลาง deps
  (orphan); ครั้งใหม่ค้างอยู่เพราะมี **เลนอื่น (Sun) กำลังบิลด์ `--release --target-dir
  target-sun` ขนานอยู่** — methodology §0 วัดไว้ชัดว่าสองบิลด์ขนานกัน = `STATUS_DLL_INIT_FAILED`
  ตาย (`-j 1` ก็ไม่ช่วย). จึงตั้ง watcher รอ Sun เคลียร์แล้วบิลด์คนเดียวอัตโนมัติ → exe ยังไม่ถูกสร้าง
- เขียน runner `scripts/rose_look_tier_bench.sh` — บังคับ gate เอง (ไม่ยอมรันถ้า PNG<3 หรือ
  `voxelforge.exe` ค้างอยู่)

พอ Gate 3 ปิด → `bash scripts/rose_look_tier_bench.sh` → เอาเลขจริงมาทับทุกช่อง *รอวัด*

---

## 1. ตารางหลัก — tier → fps / 1% low / อะไรถูกตัด

คอลัมน์ "อะไรถูกตัด" อ่านจาก **`look.rs::insert_stack` ตัวจริงที่ probe `#[path]`-include
เข้ามา** ไม่ใช่จาก spec เฉยๆ (probe วัดโค้ด ไม่ใช่วัดเอกสาร). ทุก tier มี **ชั้น identity**
เหมือนกัน (Tonemap AcesFitted + ColorGrading + Bloom + Msaa Off) เพราะราคา ~0 และคือสิ่งที่
ทำให้อ่านออกว่า "Voxelforge" — ตัดได้หมดเลยยกเว้นตัวนี้ (spec §1).

| tier | `VOXELFORGE_LOOK_QUALITY` | เครื่องเป้าหมาย | **fps** (1000/p50) | **fps 1% low** | ตัดออกจาก Ultra |
|---|---|---|---|---|---|
| **Low** | `low` | iGPU / การ์ดเก่า | _รอวัด_ | _รอวัด_ | PCSS · VolumetricFog · VolumetricLight · DoF · **TAA** (เงา Gaussian แทน Temporal) · SSAO ลดเหลือ Low |
| **Medium** | `medium` | GTX 1050 / iGPU แรง | _รอวัด_ | _รอวัด_ | PCSS · **VolumetricFog ทั้งชั้น** · VolumetricLight · DoF (เก็บ TAA + เงา Temporal + SSAO Low + DistanceFog) |
| **High** *(default)* | `high` | GTX 1660 @1080p60 (median Steam) | _รอวัด_ | _รอวัด_ | **PCSS** เท่านั้น + VolumetricFog ลด step 96→32 + SSAO Ultra→Medium (ยังเก็บ DoF + god ray) |
| **Ultra** | `ultra` | 1660+ @1080p60 หรือดีกว่า | _รอวัด_ | _รอวัด_ | — (สแตกครบ 6 เอฟเฟกต์ ตามที่ hero shot เซ็นรับ) |

**fps** = `1000 / median_ms` (มัธยฐานของ frame time) = "fps เฉลี่ย" ที่ทน outlier
**fps 1% low** = `1000 / (mean ของ frame time ช้าสุด 1%)` = fps ที่แย่ที่สุดที่ผู้เล่นรู้สึกได้
(ทั้งคู่เป็น mean ของ 2 รอบ interleaved — กัน thermal drift ตามวิธีใน methodology §2)

### หมายเหตุที่ Shiba ต้องรู้ตอนทำเมนู

1. **ค่า env ที่เมนูส่ง = ชื่อ tier พิมพ์เล็ก** (`low`/`medium`/`high`/`ultra`) ตรงกับที่
   `look.rs:140` อ่าน. ค่าอื่น/ไม่ส่ง = fall back **High** (default). เมนูเปลี่ยน tier ทำได้สองทาง:
   set env ตอนเปิดเกม, หรือเขียน `ResMut<LookQuality>` ตอนรัน (เดี๋ยวนั้นสลับได้ ไม่ต้อง restart —
   เหมือนปุ่ม F7 ที่ `cycle_look_quality` ทำอยู่)
2. **ค่าเหล่านี้วัดที่ 720p** (probe ตรึง 1280×720), บน **ฉากสังเคราะห์ 64×64** (heightfield +
   เสา 5 ต้น) บน **เครื่องเดียว**. มันเชื่อถือได้ในฐานะ *อันดับ/เดลต้าระหว่าง tier* และเป็น
   "fps บนเครื่องนี้" — ห้ามนำไปพิมพ์ในเมนูว่า "Ultra = 60fps บน GTX 1660" โดยตรง (ดู §3)
3. **`DirectionalLightShadowMap 4096` ถูก insert ครั้งเดียวตอน build และค้างไว้ทุก tier**
   (`look.rs:160`). มันกิน VRAM ~64MB ไม่ใช่ fps → ไม่ปรากฏในคอลัมน์ "ตัด" และไม่ทำให้ tier ต่ำ
   ถูกอ้างว่า "ประหยัดกว่า" เพราะเงา 4K. PCSS ที่ *ใช้* แผนที่ 4K เปิดเฉพาะ Ultra เท่านั้น

### ⚠️ จุดที่ `look.rs` ตอนนี้ยังไม่ตรง spec 3 จุด (ไม่ใช่ perf — แจ้งไว้ให้ทราบ)

ตรวจ `insert_stack` เทียบ `look-tier-spec.md` แล้วเจอ 3 จุดที่โค้ดยังห่างจาก spec
(โครงสร้างเปิด/ปิด effect **ถูกต้องหมด** — ปัญหา 3 จุดใน spec §5 แก้ครบ; ที่เหลือเป็นค่า
quality/ค่าคงที่ ไม่ใช่การตัด effect):

- **SSAO quality ต่ำกว่า spec 1 ขั้น** — spec §2 ว่า Medium=Medium/High=High แต่ `look.rs`
  Medium=`Low` (look.rs:280) High=`Medium` (look.rs:311). ค่าคงที่ `1.45` เหมือนกัน
- **Low ไม่มี `DistanceFog`** — spec §2 ว่า Low มี DistanceFog on (ราคาเกือบ 0) แต่แขน Low
  (look.rs:245) insert แค่ Gaussian+SSAO. perf ไม่กระทบ (DistanceFog ~0) แต่ภาพ Low จะไม่มีหมอก
- **3 ค่า grade ใน `mod grade` ยังเป็นค่าเก่า** — spec §4/§7 สั่งให้ `TEMPERATURE 0.10→0.02`,
  `POST_SATURATION 1.02→1.00`, `HIGHLIGHT_CONTRAST 1.30→1.0`. ค่า grade ราคา ~0 → ไม่ขยับ fps

ทั้งสามเป็นงาน look-lane tuning (ต้องเรนเดอร์เฟรมเกมจริงแล้ววัดก่อนเซ็น) ไม่ใช่งาน perf นี้
ฝากไว้เป็น follow-up แยก เมนู Settings อ้างตัวเลขจากตาราง §1 ได้ไม่ว่าจะแก้สามจุดนี้หรือยัง

---

## 2. ราคาต่อเอฟเฟกต์ (leave-one-out @ Ultra) — อ่านเสริม

ตอบ "ถ้าปิดแค่ตัวเดียว ได้คืนกี่ ms" (เฟส 1 ของ runner). เป็นข้อมูลหลังบ้านให้ Rose จูนลำดับตัด
ไม่ใช่ตัวเลขที่เมนูโชว์. ตารางเต็มอยู่ใน `look-perf-methodology.md` §4.

| ตัดอะไรจาก Ultra | เดลต้า p50 (ms) | เดลต้า fps |
|---|---|---|
| `off` (ปลั๊กอินทั้งก้อน) | _รอวัด_ | _รอวัด_ |
| `no_bloom` | _รอวัด_ | _รอวัด_ |
| `no_pcss` | _รอวัด_ | _รอวัด_ |
| `no_taa` | _รอวัด_ | _รอวัด_ |
| `no_ssao` | _รอวัด_ | _รอวัด_ |
| `no_dof` | _รอวัด_ | _รอวัด_ |
| `no_vfog` | _รอวัด_ | _รอวัด_ |

คาดการณ์จาก spec §1: `no_bloom` ถูกที่สุด, `no_vfog` แพงที่สุด (ray-march 96 step).

---

## 3. ขอบเขต — สิ่งที่ตัวเลขชุดนี้ **ไม่ได้** ตอบ

- **ไม่ใช่ frame budget ที่ 1080p** — วัดที่ 720p บน heightfield สังเคราะห์. จะอ้างเป็น "60fps
  @1080p บน GTX 1660" ต้องเรนเดอร์เฟรมเกมจริงที่ 1080p บนเครื่องนั้น (methodology §3)
- **ไม่ใช่ VRAM/memory budget** — วัด frame time อย่างเดียว
- **เครื่องเดียว จุดข้อมูลเดียว** — tier ladder เชื่อถือได้ (A/B บนเครื่องเดียวกัน) แต่ absolute
  fps ผูกกับการ์ดเครื่องนี้
- **เอฟเฟกต์ screen-space ทั้งหมด** (SSAO/DoF/Bloom/TAA/volumetric march) ราคาผูกความละเอียด
  มากกว่าจำนวนสามเหลี่ยม → เดลต้าที่ 720p มีแนวโน้มขยาย ~2.25× ที่ 1080p (การประมาณ ไม่ใช่ที่วัด)

---

_เอกสารมีชีวิต — ทุกช่องตัวเลขต้องมีบรรทัด `PERF ...` รองรับจึงจะเขียนตัวเลขลงได้
(กฎเดียวกับ methodology §7). ว่าง = ว่างจริง ไม่ใช่ลืม. — Rose, 2026-08-01_
