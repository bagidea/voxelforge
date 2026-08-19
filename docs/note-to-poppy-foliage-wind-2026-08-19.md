# note to Poppy — foliage wind ready to build (97ed070)

พร้อมให้คุณ build แล้วครับ — ผมไม่รัน cargo build เอง (คุณถือ lock)

```
cargo build --release --bin voxelforge_foliage_proof
```

แล้ว render (รันจาก repo root):

```
VOXELFORGE_SHOT=<path>.png  ./target-rose/release/voxelforge_foliage_proof.exe
```

## มันคืออะไร

custom vertex shader **ตัวแรก** ของ client — ทุ่ง grass แบบ cross-quad ไหวลม
ทั้ง 4 ข้ออยู่ใน `assets/shaders/foliage_wind.wgsl`:

| ข้อ | ฟังก์ชัน | บรรทัด |
|---|---|---|
| (1) wind field (noise เลื่อนตามเวลา, ทิศเดียวทั้งฉาก) | `wind_field()` | 70 |
| (2) gust (คลื่นวิ่งผ่านทุ่ง) | `gust()` | 80 |
| (3) phase/amp สุ่มรายต้นจาก world cell | `plant_phase()` / `plant_amp()` | 88 / 94 |
| (4) bend ramp 0@โคน → สุด@ปลาย (ต้นไม่หลุดพื้น) | `bend = uv.y * uv.y` ใน `vertex()` | 118 |

Rust glue: `client/src/foliage.rs` (`FoliageMaterial = ExtendedMaterial<StandardMaterial, FoliageWindExt>`, uniform `#[uniform(100)]` → `@group(2) @binding(100)`).

## ⚠ จุดที่ต้อง verify ตอน build (ผม compile ไม่ได้ — lock เป็นของคุณ)

- **plumbing bevy_pbr ใน WGSL คือส่วนเสี่ยง**: import `mesh_vertex_output::{Vertex, MeshVertexOutput}`
  + `mesh_functions::{mesh_position_local_to_world, mesh_position_local_to_clip}`.
  ถ้า `MeshVertexOutput` ใน Bevy 0.19 มี field เพิ่ม (เช่น `world_tangent`/`uv_b`/`color`)
  WGSL จะ fail "field not assigned" — แก้บรรทัดเดียว (assign field นั้นจาก `in`).
- **wind-core (`hash22` → `sway_metres`) เป็น portable — อย่าแตะ** ถ้าเลขการไหวแปลก ดูที่
  `FoliageWindUniform::default()` ใน foliage.rs:67 (amp/ความเร็ว/ทิศ)

## ไฟล์

- `assets/shaders/foliage_wind.wgsl` (ใหม่)
- `client/src/foliage.rs` (ใหม่)
- `client/src/foliage_proof_main.rs` (ใหม่ — proof bin, ไม่แตะ main.rs/look.rs)
- `client/Cargo.toml` (+ `[[bin]] voxelforge_foliage_proof`)
