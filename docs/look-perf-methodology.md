# Look Perf Methodology — วิธีวัดราคาเฟรมของ post stack (และวิธีอ่านตัวเลขไม่ให้ผิด)

> **เจ้าของเอกสาร:** Poppy (perf lane) · **ผู้ใช้งานหลัก:** Rose (จูน tier ใน `client/src/look.rs`)
> **สร้าง:** 2026-07-31
>
> อ่านคู่กับ: [`look-tier-spec.md`](look-tier-spec.md) §1 (อะไรตัดแล้วตาย) ·
> [`perf-vsync-cliff-rootcause.md`](perf-vsync-cliff-rootcause.md) (ทำไมต้องปิด VSync)
> · โค้ด: `client/src/perf_main.rs` · runner: `scripts/perf_look_probe.sh`

---

## 0. สถานะ ณ ตอนนี้ — **ยังไม่มีตัวเลขสักตัว**

ตารางในเอกสารนี้ยังว่าง และมันว่างเพราะโปรบ **ยังคอมไพล์ไม่ผ่านสักครั้ง** ไม่ใช่เพราะยังไม่ได้รัน
หลักฐานบนดิสก์:

```
$ tail -4 _poppy_perf_build.log       # mtime 20:50:44
error: failed to run custom build command for `slotmap v1.1.1`
  process didn't exit successfully: ...build-script-build (exit code: 0xc0000142, STATUS_DLL_INIT_FAILED)
BUILD_EXIT=101

$ ls target-poppy/perf/voxelforge_perf.exe
ls: cannot access '...': No such file or directory
```

`0xc0000142 STATUS_DLL_INIT_FAILED` **ไม่ใช่บั๊กในโค้ด** — เป็นอาการเครื่องหมด commit headroom ตอน
โหลด DLL ของ build-script. ตอนที่ตายมี release build ของ Gate 3 (`build_safe.sh build --release
-j 2`) กินอยู่แล้ว และ `-j 1` ไม่ช่วย เพราะ peak มาจากการมี **สองบิลด์พร้อมกัน** ไม่ใช่จำนวน job.
👉 **โปรบต้องบิลด์คนเดียวบนเครื่อง** ด้วย `-j 2` ไม่ใช่แทรกคิวคนอื่น

**อย่าเชื่อค่าใดๆ ในเอกสารนี้จนกว่าช่องผลจะมีตัวเลขพร้อมบรรทัด log กำกับ** — ช่องว่างคือช่องว่างจริง

### 0.1 วิธีบอกว่าบิลด์ "ยังมีชีวิต" — ต้องครบ 3 อย่างพร้อมกัน

ฉันเคยรายงานผิดว่าบิลด์กำลังเดิน ทั้งที่มันตายไปแล้ว เพราะเห็น `rustc.exe` ใน `tasklist`
แล้วนับเป็นของตัวเอง — **แต่มันเป็นลูกของ release build เลนอื่น**. `tasklist` บอกแค่ว่า
"มี cargo.exe อยู่ในเครื่อง" ไม่ได้บอกว่าเป็นของใคร เช็คแบบนั้นให้ false green เสมอ

| # | เช็คอะไร | คำสั่ง |
|---|---|---|
| 1 | cmdline ของ cargo **ตรงกับคำสั่งที่เราสั่งจริง** | `Get-CimInstance Win32_Process -Filter "Name='cargo.exe'" \| Select CommandLine` |
| 2 | mtime ของ `target-poppy/` **ขยับใน 2 นาทีล่าสุด** | `ls -lt target-poppy/perf/deps \| head` |
| 3 | log **ยังไม่มี**บรรทัด `BUILD_EXIT=` | `grep BUILD_EXIT _poppy_perf_build.log` |

ครบ 3 = มีชีวิต · ขาดข้อใดข้อหนึ่ง = ตายแล้ว หรือเป็นบิลด์ของเลนอื่น

---

## 1. ทำไมตัวเลข "ชุดก่อนหน้า" ใช้ไม่ได้ (บั๊กที่เจอตอนรีวิวตัวเอง)

โปรบรุ่นแรกจะให้ตัวเลขที่ *ดูสมเหตุสมผล* แต่ผิด 3 ทาง — บันทึกไว้เพราะถ้า Rose เห็นตัวเลขจาก
ไบนารีเก่าที่ไหน ต้องทิ้งทั้งชุด:

| # | บั๊ก | ผลต่อตัวเลข | แก้แล้วที่ |
|---|---|---|---|
| 1 | **baseline `off` วิ่ง MSAA 4×** — `bevy_render` ประกาศ `register_required_components::<Camera, Msaa>()` และ `Msaa` default = `Sample4`; `look.rs:190` ใส่ `Msaa::Off` **เฉพาะตอนปลั๊กอินทำงาน** โหมด `off` จึงเป็นโหมดเดียวที่จ่ายค่า raster 4× + resolve | เดลตาพาดหัว "full − off" **ปนราคา MSAA เข้าไปทั้งก้อน** และปนแบบติดลบ (baseline แพงเกินจริง → post stack ดูถูกเกินจริง) | `perf_main.rs` spawn กล้องด้วย `Msaa::Off` ตั้งแต่ `setup()` → MSAA เป็นค่าคงที่ทุกโหมด |
| 2 | **`no_pcss` churn ทุกเฟรม** — query ใช้ `With<LookApplied>` เฉยๆ เลย insert `ShadowFilteringMethod` ใหม่ + `&mut DirectionalLight` (mark Changed) ซ้ำทั้ง 840 เฟรมที่วัด | ยัด CPU/extract churn เข้าไปในโหมดที่กำลังวัดพอดี → PCSS ดู "ประหยัดน้อยกว่าจริง" | เพิ่ม marker `PcssStripped` + กรอง `Without<PcssStripped>` ทั้งกล้องและดวงอาทิตย์ → ทำครั้งเดียวจบ |
| 3 | **วัด leave-one-out บน tier default (High)** — `look.rs:94` `#[default] High` และ High **ไม่มี `VolumetricFog`** | `no_vfog` จะไม่ถอดอะไรเลย → รายงานว่า **ray-march ฟรี** ซึ่งเป็นข้อสรุปที่ผิดที่สุดเท่าที่ tier ladder จะรับได้ | runner ตรึง `VOXELFORGE_LOOK_QUALITY=ultra` ในเฟส leave-one-out (Ultra เป็น tier เดียวที่มีครบ 6 เอฟเฟกต์) |

---

## 2. นิยาม metric — **p50 / p95 ไม่ใช่ average**

โปรบเก็บ `Time<Real>::delta_secs()` ดิบทีละเฟรม แล้วรายงาน 3 ค่า:

| ค่า | คืออะไร | ใช้ตัดสินอะไร |
|---|---|---|
| **`median_ms` (p50)** | มัธยฐานของ frame time | **ค่าหลักที่ใช้เทียบ** — ทนต่อ outlier จาก OS scheduler / shader cache miss ที่หลุด warm-up มา |
| **`p95_ms`** | เฟรมที่แย่กว่า 95% ของกลุ่ม | **ค่าที่ตัดสินว่า "กระตุกไหม"** — เอฟเฟกต์ที่ p50 ถูกแต่ p95 พุ่ง = hitching ผู้เล่นรู้สึกได้ ต่อให้ fps เฉลี่ยสวย |
| `mean_ms` | ค่าเฉลี่ย | **รายงานไว้เทียบกับ p50 เท่านั้น** ห้ามใช้ตัดสิน — เฟรมค้างเฟรมเดียว 200 ms ดึงค่าเฉลี่ยของ 600 เฟรมขึ้นทั้งชุด |

ถ้า `mean` กับ `median` ห่างกันเกิน ~15% แปลว่ารันนั้นมี outlier หนัก → **ทิ้งแล้ววัดใหม่** อย่าเอาไปลงตาราง

**กันตัวแปรอื่น** ที่ฝังไว้ในโปรบแล้ว:

- `PresentMode::AutoNoVsync` — ถ้าไม่ปิด ทุกค่าจะพิงกำแพง 16.7 ms และ A/B จะอ่านว่า "ฟรี" ทั้งกอง
- **warm-up 240 เฟรมทิ้ง** ก่อนเริ่มเก็บ — เฟรมแรกๆ จ่ายค่า pipeline compile, shadow atlas alloc และ TAA history ยังไม่เต็ม เก็บไปคือชาร์จ startup ให้เอฟเฟกต์ที่กำลังขึ้นเขียง
- **เก็บ 600 เฟรม** ต่อรัน
- **2 รอบ interleaved** ทุกเฟส — ไล่ลิสต์รอบเดียว GPU จะร้อนขึ้นเรื่อยๆ แล้วชาร์จ thermal drift ให้ตัวท้ายลิสต์ รายงานสองรอบทำให้เห็น ไม่ใช่เฉลี่ยกลบ
- ฉากและกล้อง**ตรึงตายตัว** (heightfield จาก integer hash ไม่ใช่ `rand`, กล้องเป็น transform ฮาร์ดโค้ด ไม่มี controller) — A/B ที่ปล่อยมุมกล้องลอยไม่ใช่ A/B

---

## 3. ⚠️ วิธีอ่านตัวเลข — **เดลตา ไม่ใช่ frame budget**

ฉากที่โปรบใช้คือ **heightfield สังเคราะห์ 64×64** (สองชั้น voxel + เสา 5 ต้น จาก mesh/material ที่แชร์กัน)
**ไม่ใช่แมพจริงของเกม**

- ✅ **เดลตาเชื่อได้** — ทุกโหมดวิ่งบนฉากเดียวกัน กล้องเดียวกัน ความละเอียดเดียวกัน (1280×720)
  ต่างกันแค่ตัวแปรที่ตั้งใจเปลี่ยน "SSAO Ultra กิน X ms" คือคำตอบที่เอาไปจัด tier ได้
- ❌ **ค่า absolute เชื่อไม่ได้ในฐานะ frame budget** — มันคือราคาเฟรมของ *ฉากนี้ บนเครื่องนี้*
  ไม่ใช่ของโลกจริงที่ draw-call/depth complexity/จำนวน shadow caster ต่างออกไป
  ห้ามเอา `median_ms` ของ `full` ไปเทียบกับเป้า "60fps @1080p บน GTX 1660+" ตรงๆ

เอฟเฟกต์ในสแตกนี้เกือบทั้งหมดเป็น **screen-space** (SSAO, DoF, Bloom, TAA, volumetric march) →
ราคาผูกกับ **ความละเอียด** มากกว่าจำนวนโพลีกอน ดังนั้นเดลตาที่วัดที่ 720p มีแนวโน้มขยายตามพื้นที่พิกเซล
เมื่อขึ้น 1080p (~2.25×) — แต่นั่นคือ *การประมาณ* ไม่ใช่สิ่งที่วัดมา ถ้า Rose ต้องการเลข 1080p จริง บอกได้ ฉันเพิ่มเฟสให้

---

## 4. เฟส 1 — ราคาต่อเอฟเฟกต์ (leave-one-out ที่ Ultra)

**ทำไม leave-one-out ไม่ใช่ add-one-in:** เอฟเฟกต์เหล่านี้แชร์ pass กัน (SSAO และ VolumetricFog
อ่าน depth prepass ตัวเดียวกัน; TAA คือสิ่งที่ทำให้ PCSS กับ SSAO ใช้งานได้จริง) ถ้าวัดทีละตัวบนกล้องเปล่า
ค่า setup ที่แชร์กันจะถูกชาร์จให้ตัวที่รันก่อนทั้งหมด — `full − full_without_X` ตอบคำถามที่ tier สนใจจริง:
*ถ้าปิด X ตอนที่ทุกอย่างเปิดอยู่ ได้คืนเท่าไร*

**ตรึง `VOXELFORGE_LOOK_QUALITY=ultra`** — เหตุผลในข้อ 1 บั๊ก #3

| mode | ถอดอะไรออกจาก Ultra | p50 (ms) | p95 (ms) | เดลตา vs `full` (ms) | หมายเหตุ |
|---|---|---|---|---|---|
| `off` | ปลั๊กอินไม่ทำงานเลย (`Cfg.play=false`) | _รอวัด_ | _รอวัด_ | — (baseline) | MSAA ถูกตรึง `Off` เท่ากับโหมดอื่นแล้ว |
| `full` | — (สแตกครบตามที่ `look.rs` ส่ง) | _รอวัด_ | _รอวัด_ | 0 | ตัวตั้ง |
| `no_ssao` | `ScreenSpaceAmbientOcclusion` **+ `NormalPrepass`** | _รอวัด_ | _รอวัด_ | _รอวัด_ | ต้องถอด NormalPrepass ด้วย — Bevy ผูกมันมาทาง `#[require]` และ required component **ไม่ถูกลบตามตัวที่ require** ถ้าปล่อยไว้จะยัง render normal buffer ที่ไม่มีใครอ่าน |
| `no_vfog` | `VolumetricFog` (ray-march ฝั่งกล้อง) | _รอวัด_ | _รอวัด_ | _รอวัด_ | `DistanceFog` + `VolumetricLight` ยังอยู่ → อ่านเป็นราคา march ล้วน |
| `no_taa` | `TemporalAntiAliasing` | _รอวัด_ | _รอวัด_ | _รอวัด_ | **MSAA ยังปิดอยู่โดยตั้งใจ** — `extract_ssao_settings` hard-return เมื่อ `Msaa != Off` ถ้าสลับ MSAA กลับมา SSAO จะดับเงียบๆ แล้วรันนี้จะชาร์จเงินออมของ SSAO ให้ TAA |
| `no_dof` | `DepthOfField` (Bokeh) | _รอวัด_ | _รอวัด_ | _รอวัด_ | ดูเฟส 3 ประกอบ — ราคาขึ้นกับ `sensor_height` |
| `no_bloom` | `Bloom` | _รอวัด_ | _รอวัด_ | _รอวัด_ | คาดว่าถูกที่สุด (`look-tier-spec` จัดเป็น "ห้ามตัด, ต่ำมาก") — วัดเพื่อ**ยืนยัน**ข้อสมมตินั้น |
| `no_pcss` | `soft_shadow_size=None` (ดวงอาทิตย์) + `ShadowFilteringMethod::Hardware2x2` (กล้อง) | _รอวัด_ | _รอวัด_ | _รอวัด_ | ถอดทั้งสองฝั่ง — ถอดครึ่งเดียวได้ตัวเลขที่เอาไปทำอะไรไม่ได้ |

> `no_*` แต่ละตัวคือ **สแตกครบลบตัวนั้นตัวเดียว** — บวกเดลตาทั้งหมดแล้วไม่เท่ากับ `full − off`
> (pass ที่แชร์กันถูกนับซ้ำ) นั่นเป็นเรื่องปกติของ leave-one-out ไม่ใช่ความผิดพลาดของการวัด

---

## 5. เฟส 2 — ราคาต่อ tier (ตามที่ ship จริง)

เฟส 1 ตอบว่า *เอฟเฟกต์หนึ่งตัวราคาเท่าไร* เฟสนี้ตอบว่า *ขั้นบันไดหนึ่งขั้นราคาเท่าไร* — คนละคำถาม
เพราะ tier ต่างกันที่ **quality level ของ SSAO** ด้วย (Low → Medium → Ultra) ไม่ใช่แค่มี/ไม่มีเอฟเฟกต์
ดังนั้นเอาเดลตาจากเฟส 1 มาบวกกันเองจะพลาดขั้น quality ทั้งหมด

| tier | สแตกตาม `insert_stack()` | p50 (ms) | p95 (ms) | เดลตา vs Ultra | fps |
|---|---|---|---|---|---|
| _(off)_ | ปลั๊กอินไม่ทำงาน | _รอวัด_ | _รอวัด_ | _รอวัด_ | _รอวัด_ |
| **Low** | Tonemap + Grade + Bloom | _รอวัด_ | _รอวัด_ | _รอวัด_ | _รอวัด_ |
| **Medium** | + TAA, `Hardware2x2`, SSAO **Low**, DistanceFog | _รอวัด_ | _รอวัด_ | _รอวัด_ | _รอวัด_ |
| **High** *(default)* | + `ShadowFilteringMethod::Temporal`+PCSS, DoF, SSAO **Medium** | _รอวัด_ | _รอวัด_ | _รอวัด_ | _รอวัด_ |
| **Ultra** | + VolumetricFog (step_count 96), SSAO **Ultra** | _รอวัด_ | _รอวัด_ | 0 | _รอวัด_ |

---

## 6. เฟส 3 — `sensor_height` เป็น knob ของ frame time ไม่ใช่แค่ของภาพ

นี่ไม่ใช่คำถามด้านภาพที่แต่งตัวมาเป็นคำถาม perf. Bevy คำนวณ

```
focal_length   = 0.5 · sensor_height / tan(fov/2)
coc_scale      = focal_length² / (sensor_height · N)
               = 0.25 · sensor_height / (tan²(fov/2) · N)      ← LINEAR ใน sensor_height
```

แล้ว blur วิ่ง `support = round(coc/2)` taps ต่อทิศ (`dof.wgsl` `box_blur_a`/`_b`) →
**`sensor_height` สเกลจำนวน tap ตรงๆ** จนกว่าจะชน clamp `max_circle_of_confusion_diameter` ที่ 64 px

`look.rs:280,303` ตั้งไว้ **0.35** ทั้ง High และ Ultra ซึ่งคือ **~19× ของ default เชิงฟิสิกส์ (0.0186)**

| `sensor_height` | คือ | p50 (ms) | เดลตา vs 0.35 | ชนกำแพง 64px หรือยัง |
|---|---|---|---|---|
| 0.0186 | default เชิงฟิสิกส์ของ Bevy (~18.6mm) | _รอวัด_ | _รอวัด_ | _รอวัด_ |
| 0.10 | | _รอวัด_ | _รอวัด_ | _รอวัด_ |
| 0.20 | | _รอวัด_ | _รอวัด_ | _รอวัด_ |
| **0.35** | **ค่าที่ `look.rs` ใช้อยู่ตอนนี้** | _รอวัด_ | 0 | _รอวัด_ |
| 0.50 | | _รอวัด_ | _รอวัด_ | _รอวัด_ |

**สิ่งที่ Rose จะได้จากเฟสนี้:** ถ้าเส้นโค้งแบนหลัง ~0.2 (= ชน clamp แล้ว) แปลว่า 0.35 จ่ายแพงโดย
ไม่ได้ภาพเพิ่ม → ลดได้ฟรี. ถ้ายังชันที่ 0.35 แปลว่ามันคือ knob จริงของ tier ต่ำ —
แต่ **ข้อเสนอตัวเลขต้องมาคู่กับการตรวจภาพ** ฉันวัดได้แค่ ms; ว่าเบลอน้อยลงแล้วยัง "อ่านเป็น
Voxelforge" ไหม เป็นการตัดสินของ look lane ตาม `look-acceptance-rubric.md`

> ⚠️ **ฉันจะไม่แก้ `client/src/look.rs`** ค่า 0.35 จะเปลี่ยนหรือไม่ เป็นการตัดสินใจของ Rose
> โปรบ override ค่าผ่าน `VOXELFORGE_PERF_SENSOR` ตอน runtime — ค่าคงที่ในไฟล์ยังเดิมทุกตัวอักษร

---

## 7. รันซ้ำเองยังไง

```bash
# 1) บิลด์ — สคริปต์จะ "ปฏิเสธไม่ยอมเริ่ม" (exit 3) ถ้ามี cargo ของเลนอื่นรันอยู่
#    และตัดสินผลจาก log ไม่ใช่จาก exit code ผ่าน pipe (PIPESTATUS หลอก)
bash scripts/perf_look_build.sh

# 3) รันทั้งสามเฟส (~34 รัน)
scripts/perf_look_probe.sh _poppy_look_perf.log
```

รูปแบบบรรทัดผล — self-describing ทุกบรรทัด ไม่ต้องเดาว่ามาจาก tier ไหน:

```
round=1 phase=effect PERF mode=no_vfog tier=Ultra median_ms=... mean_ms=... p95_ms=... fps=... frames=600
```

**กฎการลงตาราง:** ทุกช่องในเอกสารนี้ต้องมีบรรทัด `PERF ...` จริงรองรับ ถ้าแปะ log ไม่ได้ = ห้ามเขียนตัวเลข
ให้เขียนว่า *รอวัด*

---

## 8. ขอบเขต — สิ่งที่เอกสารนี้ **ไม่ได้** ตอบ

- **ไม่ใช่ VRAM / memory budget** — วัด frame time อย่างเดียว shadow atlas 4096 กินหน่วยความจำเท่าไรยังไม่ได้ตอบ
- **ไม่ใช่ผลบนการ์ดหลายรุ่น** — เครื่องเดียว จุดข้อมูลเดียว การจัด tier สำหรับ GTX 1660 ต้องมีเครื่องนั้นจริง
- **ไม่ใช่ 1080p** — วัดที่ 720p (ดูข้อ 3)
- **ไม่แตะ `look.rs`** — โปรบ `#[path]`-include ไฟล์ของ Rose เข้ามาแบบไม่แก้ ตามกติกา `docs/LANES.md`
