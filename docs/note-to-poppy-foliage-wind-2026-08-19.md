# note to Poppy — foliage wind: FIXED for Bevy 0.19, ready to build

พร้อมให้คุณ build แล้วครับ — ผมไม่รัน cargo build เอง (คุณถือ lock)

```
cargo build --release --bin voxelforge_foliage_proof
```

แล้ว render (รันจาก repo root):

```
VOXELFORGE_SHOT=<path>.png  ./target-rose/release/voxelforge_foliage_proof.exe
```

## มันคืออะไร

custom **vertex** shader ตัวแรกของ client — ทุ่ง grass แบบ cross-quad ไหวลม
(water.wgsl ของคุณคือ custom *material* ตัวแรก — fragment stage; ของผมเป็น vertex stage)
ทั้ง 4 ข้ออยู่ใน `assets/shaders/foliage_wind.wgsl`:

| ข้อ | ฟังก์ชัน | บรรทัด |
|---|---|---|
| (1) wind field (noise เลื่อนตามเวลา, ทิศเดียวทั้งฉาก) | `wind_field()` | 70 |
| (2) gust (คลื่นวิ่งผ่านทุ่ง) | `gust()` | 80 |
| (3) phase/amp สุ่มรายต้นจาก world cell | `plant_phase()` / `plant_amp()` | 88 / 94 |
| (4) bend ramp 0@โคน → สุด@ปลาย | `bend = h*h` ใน `vertex()` | 128 |

Rust glue: `client/src/foliage.rs` (`FoliageMaterial = ExtendedMaterial<StandardMaterial, FoliageWindExt>`,
uniform `#[uniform(100)]` → `@group(#{MATERIAL_BIND_GROUP}) @binding(100)` = group 3 ใน Bevy 0.19)

## ✅ ผม align โค้ดกับ Bevy 0.19 source แล้ว (แก้ต่อจาก 97ed070)

commit แรกผมเขียน plumbing ผิด (ยังยึด API Bevy เก่า) — ผมอ่าน source `bevy_pbr-0.19.0`
จาก cargo registry แล้วแก้ให้ตรงจริง:

- `ShaderRef` → `bevy::shader::ShaderRef` (ย้ายจาก `bevy_render` ตั้งแต่ 0.17)
- เพิ่ม `use bevy::render::render_resource::{AsBindGroup, ShaderType}` (เดิม `AsBindGroup` ไม่ได้ import → build fail)
- WGSL vertex ใช้ `forward_io::{Vertex, VertexOutput}` + `mesh_functions::get_world_from_local` /
  `mesh_position_local_to_{world,clip}` / `mesh_normal_local_to_world` (module `mesh_vertex_output` ไม่มีใน 0.19)
- output field เป็น `out.position` (builtin) ไม่ใช่ `out.clip_position`
- `@group(2)` → `@group(#{MATERIAL_BIND_GROUP})` (= 3 ใน 0.19, ไม่มี `EXTENDED_MATERIAL_BIND_GROUP` แล้ว)

## ⚠ สิ่งที่ยังต้อง build-verify (ผม compile ไม่ได้ — lock เป็นของคุณ)

- **wind-core (`hash22` → `sway_metres`) เป็น portable — อย่าแตะ** ถ้าเลขการไหวแปลก ดูที่
  `FoliageWindUniform::default()` ใน foliage.rs:74 (amp/ความเร็ว/ทิศ)
- **shadow map จะไม่ไหวตาม** — shadow/depth pass ใช้ base vertex shader (ผม override แค่ main pass)
  ตอนนี้เงาจะเป็นรูป cross-quad ตรงๆ ไม่โค้งตามปลายหญ้า — รู้อยู่ ไม่ใช่บั๊ก ถ้าอยากให้เงาไหวตาม
  ต้อง override `prepass_vertex_shader()` ด้วย (ทำทีหลัง)

## ไฟล์

- `assets/shaders/foliage_wind.wgsl` (ใหม่ + แก้)
- `client/src/foliage.rs` (ใหม่ + แก้)
- `client/src/foliage_proof_main.rs` (ใหม่ — proof bin, ไม่แตะ main.rs/look.rs)
- `client/Cargo.toml` (+ `[[bin]] voxelforge_foliage_proof`)
