# Reference — Colour Fix for Voxelforge 2026-08-21

> ปัญหาที่วัดได้จริง: เรนเดอร์ของเราทั้งฉากเป็นส้ม/แดงล้วน เงาก็ส้ม — 12/16 เพลตมีพิกเซล 98–99.7% ที่ค่า Blue = 0 พอดี (mean B 0.4–2.2 เทียบ reference 68.7)
>
> ไฟล์นี้รวบรวมค่าอ้างอิงที่เอาไปตั้งค่าได้จริง 3 หัวข้อ: สี ambient ให้เงาเย็น, สี/ความเข้ม sun ตามช่วงเวลา, และ tonemapping ใน Bevy 0.19 ที่ไม่บีบช่องน้ำเงินตาย

---

## สรุปค่าที่แนะนำสำหรับ Voxelforge (ลองก่อน แล้ววัด)

| จุด | ค่าปัจจุบัน (working tree) | ค่าที่แนะนำทดสอบ | หมายเหตุ |
|---|---|---|---|
| `Hour::GOLDEN.ambient` | `[0.96, 0.90, 0.48]` (R-B +0.48) | `[0.45, 0.55, 0.90]` (R-B -0.45) หรือ `[0.55, 0.65, 0.95]` | เปลี่ยนจาก warm fill เป็น cool skylight fill |
| `Hour::GOLDEN.ambient_lux` | `2200.0` | ลอง `1500–2200` แล้ววัด warmth/sat | ถ้า ambient สูงเกินไปจะฆ่า contrast |
| `Tonemapping` ใน main/editor/bench lanes | `AcesFitted` | `TonyMcMapface` หรือ `AgX` | AcesFitted เปลี่ยน bright blue → magenta, ซึ่งหลอมกับ warm ambient แล้วเป็นส้ม |
| `DirectionalLight.color` (sun) | ต้องตรวจสอบ | ใช้ตาราง §2 เช่น 5500K → `(1.000, 0.931, 0.872)` | อย่าปล่อยให้ sun ส้มเกินไป |

คำเตือนสำคัญ: **ต้อง A/B วัดกับ golden reference ทุกครั้ง** — การเปลี่ยน ambient เป็นน้ำเงินอาจทำให้ warmth/saturation ตกต่ำเกินไป

---

## 1. Sky-ambient / Ambient Color ให้เงาเป็นน้ำเงินเย็น

### หลักการ

เงาที่ถูกบล็อกแสงแดดโดยตรงจะถูกส่องสว่างโดย **sky light / ambient light** แทน ในธรรมชาติ sky light มี color temperature สูง (6500–12000K) ดังนั้นเงาจึงออกน้ำเงิน-ฟ้า ถ้า ambient ของเราอุ่นเกินไป ทั้งฉากจะกลายเป็นสีเดียวกันหมด

### ค่าอ้างอิงจาก engine และเกม

| แหล่ง | สี ambient / sky | ค่า RGB | บริบท |
|---|---|---|---|
| **Unity research scene (day)** | ambient light | `(0.75, 0.75, 0.75)` หรือ `191, 191, 191` | ambient กลางวัน neutral |
| **Unity research scene (day tint)** | environment tint | `(0.78, 0.78, 0.86)` หรือ `200, 200, 220` | cool/bluish-white tint |
| **Unity research scene (night tint)** | environment tint | `(0.18, 0.22, 0.25)` หรือ `45, 55, 65` | dark blue-grey night tint |
| **57 Studios survival lighting guide** | default cool shadows | `(0.15, 0.15, 0.18)` หรือ `38, 38, 46` | ค่า default สำหรับ cool shadow |
| **Unity hemisphere sky** | hemisphere sky color | `(0.73, 0.89, 1.00)` หรือ `186, 226, 255` | บทเรียน Unity-style hemisphere light |
| **Godot WorldEnvironment** | soft blue ambient | `(70, 110, 160)` หรือ `#466EA0` | คู่มือ photorealistic 3D |
| **Godot cinematic blue** | cinematic blue shadow | `(50, 80, 120)` | เงาฟ้าเข้ม |
| **Unreal Engine 5** | skylight temperature | **~10000K** cool blue vs sun ~5500K | สร้าง warm sun + cool shadow contrast |
| **BSL Shaders preset** | sky color | `(0, 100, 255)` | Minecraft BSL — ฟ้าสด น้ำเงินเข้ม |
| **Bedrock lighting JSON** | ambient color default | `"#ffffff"` + `illuminance: 0.02` | สามารถ override เป็น cool blue ได้ |

### ค่าที่นำไปลองใน Bevy 0.19 ได้ทันที

```rust
// แทนที่ warm ambient ด้วย cool skylight fill
commands.spawn((
    Camera3d::default(),
    AmbientLight {
        color: Color::srgb(0.45, 0.55, 0.90), // cool blue, R < B
        brightness: 1500.0,                    // ลดลงจาก 2200 แล้ววัด
        ..default()
    },
));
```

หรือถ้าต้องการให้ยังคง warmth บ้างในพื้นที่ open shade:

```rust
// น้ำเงินอ่อนกว่า เอาไว้ดู natural
AmbientLight {
    color: Color::srgb(0.55, 0.65, 0.95),
    brightness: 1800.0,
    ..default()
}
```

### หลักการผสมสี ambient

| ส่วน | สี | บทบาท |
|---|---|---|
| Sky/top hemisphere | cool blue `(~0.5, ~0.6, ~0.95)` | ให้เงาเย็น |
| Ground/bottom hemisphere | warm brown `(~0.5, ~0.4, ~0.2)` | bounce จากพื้น ให้ warmth ธรรมชาติ |
| Sun key | warm white `(~1.0, ~0.9, ~0.8)` หรือ golden | แสงตรง |

> หมายเหตุ: Bevy 0.19 ยังไม่มี hemisphere ambient แบบ sky/ground split built-in แต่สามารถใช้ combination ของ `AmbientLight` + `EnvironmentMapLight` หรือ custom directional fill light เพื่อจำลองได้

### อ้างอิง
- [57 Studios — Environmental Lighting and Weather](https://docs.57studios.net/mapping/environmental-lighting-and-weather)
- [Unity-style hemisphere light paper](https://projekter.aau.dk/projekter/files/718135942/Med10_Rapport.pdf)
- [Hexaquo — Environment and Light in Godot](https://hexaquo.at/pages/environment-and-light-in-godot-setting-up-for-photorealistic-3d-graphics/)
- [Godot Proposals #348 — Improve Default 3D Lighting](https://github.com/godotengine/godot-proposals/issues/348)
- [Unreal Engine 5 Photorealistic Lighting Guide](https://yelzkizi.org/photorealistic-lighting-in-unreal-engine-5-guide/)
- [Minecraft Bedrock — Light Sources](https://learn.microsoft.com/en-us/minecraft/creator/documents/vibrantvisuals/lightingcustomization?view=minecraft-bedrock-stable)

---

## 2. Sun Color Temperature + Intensity ตามช่วงเวลาของวัน

### ตาราง Kelvin → RGB → Intensity

ใช้ **Tanner Helland algorithm** (อ้างอิง §2.2) แปลง Kelvin เป็น sRGB ตารางนี้รวมค่าที่ใช้บ่อยในเกมและ cinematography:

| ช่วงเวลา | Kelvin (K) | RGB (0–255) | RGB (0–1) | Illuminance (lux) |
|---|---|---|---|---|
| Deep sunrise/sunset | 2,000 | `(255, 137, 14)` | `(1.000, 0.537, 0.055)` | 10–400 |
| Warm sunrise/sunset | 2,500 | `(255, 159, 70)` | `(1.000, 0.624, 0.275)` | 400–1,000 |
| Golden hour (warm) | 3,000 | `(255, 177, 110)` | `(1.000, 0.695, 0.431)` | 1,000–10,000 |
| Golden hour (soft) | 3,500 | `(255, 193, 141)` | `(1.000, 0.755, 0.552)` | 10,000–25,000 |
| Early morning/late afternoon | 4,000 | `(255, 206, 166)` | `(1.000, 0.807, 0.651)` | 25,000–50,000 |
| Midday sun (clear) | 5,500 | `(255, 237, 222)` | `(1.000, 0.931, 0.872)` | 60,000–100,000 |
| Hazy/overcast noon | 6,000 | `(255, 246, 237)` | `(1.000, 0.965, 0.929)` | 30,000–60,000 |
| D65 average noon | 6,500 | `(255, 254, 250)` | `(1.000, 0.997, 0.981)` | reference white |
| Overcast sky | 7,000 | `(243, 242, 255)` | `(0.951, 0.950, 1.000)` | ~1,000 |
| Heavy overcast/open shade | 7,500 | `(230, 235, 255)` | `(0.901, 0.921, 1.000)` | 1,000–3,000 |
| Blue hour (deep twilight) | 9,000 | `(210, 223, 255)` | `(0.822, 0.874, 1.000)` | 0.1–10 |
| Blue hour (pale blue) | 10,000 | `(202, 218, 255)` | `(0.791, 0.855, 1.000)` | 0.1–10 |
| Violet-blue twilight | 12,000 | `(191, 211, 255)` | `(0.749, 0.829, 1.000)` | ~0.05 |

### สูตรแปลง Kelvin → RGB (Tanner Helland)

```rust
use std::f32;

fn kelvin_to_rgb(k: f32) -> (f32, f32, f32) {
    let t = k.clamp(1000.0, 40000.0) / 100.0;

    let r = if t <= 66.0 {
        255.0
    } else {
        (329.698727446 * (t - 60.0).powf(-0.1332047592)).clamp(0.0, 255.0)
    };

    let g = if t <= 66.0 {
        (99.4708025861 * t.ln() - 161.1195681661).clamp(0.0, 255.0)
    } else {
        (288.1221695283 * (t - 60.0).powf(-0.0755148492)).clamp(0.0, 255.0)
    };

    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        (138.5177312231 * (t - 10.0).ln() - 305.0447927307).clamp(0.0, 255.0)
    };

    (r / 255.0, g / 255.0, b / 255.0)
}
```

### Presets สำหรับ Voxelforge

```rust
// Bevy 0.19 DirectionalLight
commands.spawn((
    DirectionalLight {
        color: Color::srgb(1.000, 0.931, 0.872), // 5500K midday
        illuminance: 80_000.0,                    // lux
        shadows_enabled: true,
        ..default()
    },
    Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
));
```

| Phase | Preset K | `color` (srgb 0–1) | `illuminance` |
|---|---|---|---|
| Sunrise | 3,000 K | `(1.000, 0.695, 0.431)` | 3,000–5,000 lux |
| Morning | 4,000 K | `(1.000, 0.807, 0.651)` | 40,000–60,000 lux |
| Noon | 5,500 K | `(1.000, 0.931, 0.872)` | 80,000–100,000 lux |
| Overcast | 7,000 K | `(0.951, 0.950, 1.000)` | 5,000–15,000 lux |
| Golden hour | 3,500 K | `(1.000, 0.755, 0.552)` | 10,000–25,000 lux |
| Blue hour | 10,000 K | `(0.791, 0.855, 1.000)` | 1.0–5.0 lux |
| Night/moon | — | `(0.25, 0.28, 0.45)` | 0.1–0.5 lux |

### หลักการใช้ค่า
- **อย่าให้ sun อุ่นเกินไปในช่วงกลางวัน**: 5500K เป็นสีขาวอุ่น ถ้าใช้ 3000K ทั้งวันโลกจะดูเหมือนตะวันลับตลอดเวลา
- ** ambient ต้องเย็นกว่า sun**: ถ้า ambient อุ่นเท่า sun หรืออุ่นกว่า เงาจะไม่มี cool fill
- **ใช้ lux แทน multiplier**: Bevy `DirectionalLight.illuminance` รองรับหน่วย lux อยู่แล้ว

### อ้างอิง
- [Tanner Helland — Convert Temperature to RGB](https://tannerhelland.com/2012/09/18/convert-temperature-rgb-algorithm-code.html)
- [paulkaplan — Kelvin to RGB gist](https://gist.github.com/paulkaplan/5184275)
- [petrklus — Kelvin to RGB Python](https://gist.github.com/petrklus/b1f427accdf7438606a6)
- [Pixflow — Color Temperature of Daylight](https://pixflow.net/blog/white-balance-video-color-temperature/)
- [FromLux — What is the Color Temperature of Daylight](https://fromlux.com/what-is-the-color-temperature-of-daylight-a-complete-guide/)
- [Shotkit — Color Temperature Photography](https://shotkit.com/color-temperature-photography/)
- [RP Photonics — CIE Standard Illuminants](https://www.rp-photonics.com/cie_standard_illuminants.html)

---

## 3. Bevy 0.19 Tonemapping / Color Grading — ตัวไหนไม่บีบช่องน้ำเงิน

### สถานการณ์ Voxelforge

จากการค้นหาใน codebase:
- `client/src/main.rs`, `client/src/atlas_shot_main.rs`, `client/src/characters.rs`, `client/src/enemies.rs` ใช้ `Tonemapping::AcesFitted`
- `client/src/look.rs` ใช้ `Tonemapping::TonyMcMapface` (line 2599) พร้อมเหตุผลว่า AcesFitted เปลี่ยน bright blue → magenta

นี่คือปัญหา: เมื่อ scene มี warm ambient อยู่แล้ว AcesFitted จะดัน bright blue ไป magenta แล้ว magenta หลอมกับ warm orange กลายเป็นสีส้ม/แดงทั้งภาพ

### เปรียบเทียบ Tonemapping variants ใน Bevy 0.19

| Variant | Hue shifting | พฤติกรรมช่องน้ำเงิน | แนะนำสำหรับ Voxelforge |
|---|---|---|---|
| `None` | ไม่มี | ไม่ compress แต่จะ clip ถ้า HDR สูง | ไม่เหมาะกับ HDR scene |
| `Reinhard` | **เยอะมาก** | brights ไม่ desaturate ตามธรรมชาติ | หลีกเลี่ยง |
| `ReinhardLuminance` | มี | brights ไม่ค่อย desaturate | หลีกเลี่ยง |
| `AcesFitted` | **dramatic** | **bright blues → magenta**, greens/reds → orange | **นี่คือตัวผิด** ในภาพ warm ของเรา |
| `AgX` | เกือบไม่มี | neutral, ภาพอาจดูจางกว่า | **ตัวเลือกที่ดี** |
| `SomewhatBoringDisplayTransform` | น้อยใน darks/mids, เยอะใน brights | ระหว่าง Reinhard กับ ReinhardLuminance | ทดสอบได้ |
| `TonyMcMapface` | น้อยและตั้งใจ | preserve hues, ใกล้ input stimulus | **แนะนำหลัก** |
| `BlenderFilmic` | มี | suffers hue shifting | ไม่เหมาะเท่า Tony/AgX |
| `KhronosPbrNeutral` | significant Abney | crush grays/desaturated colors | ไม่เหมาะ |

### คำอธิบายจาก Bevy docs (verbatim)

> **AcesFitted**: "Not neutral, has a very specific aesthetic, intentional and dramatic hue shifting. Bright greens and reds turn orange. **Bright blues turn magenta.**"

> **TonyMcMapface**: "Very neutral... Color hues are preserved during compression, except for a deliberate Bezold–Brücke shift... stays close to the input stimulus where compression isn't necessary."

> **AgX**: "Very neutral. Image is somewhat desaturated when compared to other tonemappers. Little to no hue shifting."

### Code สำหรับ Bevy 0.19

```rust
use bevy::prelude::*;
use bevy::core_pipeline::tonemapping::Tonemapping;

// ตัวเลือกที่แนะนำ
commands.spawn((
    Camera3d::default(),
    Camera {
        hdr: true,
        ..default()
    },
    Tonemapping::TonyMcMapface, // หรือ Tonemapping::AgX
));
```

### Color Grading ประกอบ

```rust
use bevy::prelude::*;

commands.spawn((
    Camera3d::default(),
    ColorGrading {
        global: ColorGradingGlobal {
            post_saturation: 1.1,    // ค่อยๆ ปรับ อย่าดันเกินไป
            ..default()
        },
        shadows: ColorGradingSection {
            lift: Vec3::new(0.02, 0.03, 0.08), // ยกช่องน้ำเงินในเงา
            ..default()
        },
        ..default()
    },
));
```

> คำเตือน: จาก `look.rs` ระบุว่า shadow contrast บีบ open shade ลงไปดำได้ ดังนั้นใช้ `shadow lift` แทน `shadow contrast`

### อ้างอิง
- [docs.rs — Bevy `Tonemapping` enum](https://docs.rs/bevy/latest/bevy/core_pipeline/tonemapping/enum.Tonemapping.html)
- [Bevy — Tonemapping example](https://bevy.org/examples/3d-rendering/tonemapping/)
- [Bevy Cheatbook — HDR and Tonemapping](https://bevy-cheatbook.github.io/graphics/hdr-tonemap.html)
- [Godot proposal — Add TonyMcMapface](https://github.com/godotengine/godot-proposals/issues/7263)

---

## 4. แผนการทดสอบใน Voxelforge

### ขั้นตอนที่แนะนำ

1. **เปลี่ยน tonemapping ทุก camera lane ที่ยังใช้ AcesFitted** เป็น `TonyMcMapface`
   - `client/src/main.rs:371`
   - `client/src/atlas_shot_main.rs:371`
   - `client/src/characters.rs:1036`
   - `client/src/enemies.rs:881`
   - คง `look.rs:2599` ไว้ (TonyMcMapface อยู่แล้ว)

2. **ปรับ `Hour::GOLDEN.ambient` จาก warm เป็น cool blue**
   - เริ่มจาก `[0.55, 0.65, 0.95]` แล้ว A/B
   - ถ้า warm reference ยังไม่ผ่าน ลอง `[0.45, 0.55, 0.90]`

3. **ลด `ambient_lux` ลงมาเล็กน้อย** ถ้า contrast หาย
   - จาก `2200` ลอง `1800` หรือ `1500`

4. **ตรวจสอบ sun color**
   - ถ้า sun ยังคงอุ่นเกินไป ให้ใช้ 5500K preset

5. **วัดผล**
   - รัน `scripts/grade_gate.py`
   - รัน `scripts/colour_gate.py` (magenta fraction)
   - เปรียบเทียบกับ `golden-beauty-shot-ref.png`

### สิ่งที่ต้องระวัง

- การเปลี่ยน ambient เป็น cool blue อาจทำให้ **interior/golden-hour warmth** ตก ต้อง A/B กับ reference
- การเปลี่ยน tonemapping อาจทำให้ **bloom** เปลี่ยนลักษณะ (TonyMcMapface เป็น default ที่ Bevy แนะนำให้ใช้คู่กับ Bloom)
- ต้องตรวจสอบว่า `tonemapping_luts` cargo feature เปิดอยู่ (default เปิดอยู่แล้ว)

---

## 5. บทสรุป

| หัวข้อ | คำตอบสั้น |
|---|---|
| **Ambient ให้เงาเย็น** | ใช้ cool blue ambient เช่น `[0.45–0.55, 0.55–0.65, 0.90–0.95]` แทน `[0.96, 0.90, 0.48]` |
| **Sun ตามช่วงเวลา** | Midday 5500K `(1.0, 0.931, 0.872)` ที่ 80,000 lux; ambient ต้องเย็นกว่า sun |
| **Tonemapping** | หยุดใช้ `AcesFitted` ใน warm scene — ใช้ `TonyMcMapface` หรือ `AgX` |

> เป้าหมาย: Blue channel ในเงาต้องไม่เป็น 0, ภาพต้องมี cool/warm contrast, และ warmth/saturation ต้องยังผ่าน gate

---

## Sources

- [57 Studios — Environmental Lighting and Weather](https://docs.57studios.net/mapping/environmental-lighting-and-weather)
- [Unity-style hemisphere light paper](https://projekter.aau.dk/projekter/files/718135942/Med10_Rapport.pdf)
- [Hexaquo — Environment and Light in Godot](https://hexaquo.at/pages/environment-and-light-in-godot-setting-up-for-photorealistic-3d-graphics/)
- [Godot Proposals #348 — Improve Default 3D Lighting](https://github.com/godotengine/godot-proposals/issues/348)
- [Unreal Engine 5 Photorealistic Lighting Guide](https://yelzkizi.org/photorealistic-lighting-in-unreal-engine-5-guide/)
- [Minecraft Bedrock — Light Sources](https://learn.microsoft.com/en-us/minecraft/creator/documents/vibrantvisuals/lightingcustomization?view=minecraft-bedrock-stable)
- [Tanner Helland — Convert Temperature to RGB](https://tannerhelland.com/2012/09/18/convert-temperature-rgb-algorithm-code.html)
- [paulkaplan — Kelvin to RGB gist](https://gist.github.com/paulkaplan/5184275)
- [Pixflow — Color Temperature of Daylight](https://pixflow.net/blog/white-balance-video-color-temperature/)
- [FromLux — What is the Color Temperature of Daylight](https://fromlux.com/what-is-the-color-temperature-of-daylight-a-complete-guide/)
- [RP Photonics — CIE Standard Illuminants](https://www.rp-photonics.com/cie_standard_illuminants.html)
- [docs.rs — Bevy `Tonemapping` enum](https://docs.rs/bevy/latest/bevy/core_pipeline/tonemapping/enum.Tonemapping.html)
- [Bevy — Tonemapping example](https://bevy.org/examples/3d-rendering/tonemapping/)
- [Bevy Cheatbook — HDR and Tonemapping](https://bevy-cheatbook.github.io/graphics/hdr-tonemap.html)
- [Godot proposal — Add TonyMcMapface](https://github.com/godotengine/godot-proposals/issues/7263)
- [Bevy `DirectionalLight` docs](https://docs.rs/bevy/latest/bevy/prelude/struct.DirectionalLight.html)
- [Bevy `AmbientLight` docs](https://docs.rs/bevy/latest/bevy/prelude/struct.AmbientLight.html)
