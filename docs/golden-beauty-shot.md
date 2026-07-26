# Voxelforge — Golden Beauty-Shot Target

> **หนึ่งฉาก หนึ่งเกณฑ์รับ.** นี่คือด่านตัดสิน visual ของ **Bevy spike (Poppy)** —
> เรนเดอร์ครัวไม้ฉากนี้ใน Rust + Bevy 0.19 + wgpu ให้ผ่าน pass-list ด้านล่าง แล้ววัด FPS **บนเฟรมเดียวกัน**
> → ได้ทั้งลุคและตัวเลข apples-to-apples ในนัดเดียว. ไม่ต้องทำ spec sheet ต่อทุกฟีเจอร์ก่อน go/no-go —
> เอาฉากนี้ให้ผ่านก่อน แล้วค่อย generalize. ที่มาของลุค: [look-bible.md](look-bible.md).

## Reference frame (เป้าที่ต้องเรนเดอร์ให้ถึง)

![Golden beauty-shot reference](assets/golden-beauty-shot-ref.png)

`docs/assets/golden-beauty-shot-ref.png` — ครัวไม้ voxel, แดด golden-hour เฉียงผ่านหน้าต่างซ้าย,
god rays + dust motes, bounce อุ่นเติมร่ม (ไม่มีเงาดำสนิท), DOF ชามหน้า-คม/หลัง-ละลาย,
ตู้เย็นสแตนเลส specular, teal accent (บล็อกเขียว) 1 จุด. **นี่คือลุค ไม่ใช่ geometry ตายตัว** —
Poppy จัดบล็อกเองได้ ขอให้ครบ element + ค่าแสงตาม pass-list.

---

## Scene setup (ค่าตั้งฉาก — คงที่ทุกครั้งที่เทียบ)

| พารามิเตอร์ | ค่าเป้า |
|---|---|
| **กล้อง** | FOV ~50° · สูงตา ~1.4m · เล็งข้ามเคาน์เตอร์ไปทางหน้าต่าง · ก้มเล็กน้อย ~5° |
| **ตัวแบบ hero** | ชามไม้/เซรามิก 1 ใบวางหน้าเคาน์เตอร์ที่ระยะ ~1.5m (จุด focus ของ DOF) |
| **หน้าต่าง** | บานเลื่อนกรอบไม้ซ้ายมือ มี mullion แบ่งช่อง (ให้ god ray เป็นแท่งตามช่อง) |
| **แดด (key)** | directional, azimuth เข้าทางหน้าต่างซ้าย, elevation **15–20°** เหนือขอบฟ้า (golden hour) |
| **teal accent** | วัตถุเย็น 1 ชิ้น (บล็อกเพชร/ของเขียว) — ต้องกิน ≤15% ของเฟรม |

---

## Pass-list — ค่าที่ Poppy ต้องเรนเดอร์ให้ถึง

เรียงตาม "แกน identity ก่อน → เสน่ห์". เทียบกับ reference frame ข้างบน. เกณฑ์ผ่าน = **สังเกตเห็นได้ชัดในเฟรม** ตามคำอธิบาย.

| # | ชั้น | ค่าเป้า / สิ่งที่ต้องเห็น | ผ่านเมื่อ |
|---|---|---|---|
| 1 | **Key light (แดด)** | directional golden `#F4B860`→`#FFD98A`, elevation 15–20°, ทาบเงากรอบหน้าต่างลงพื้น/ผนัง | เห็นทิศแดดชัด + แถบแสงหน้าต่างบนพื้น |
| 2 | **Bounce / GI (แสงอ้อม)** | fill ร่มด้วย honey `#C88A4A`; **shadow floor ≥ ~10% luminance, โทนอุ่น** ห้ามดำสนิท/ห้ามฟ้า | เอา eyedrop จุดมืดสุด → ยัง "เห็นเนื้อไม้" + สีอุ่น |
| 3 | **God rays (volumetric)** | แท่งแสงตาม mullion หน้าต่าง + dust motes ลอยในลำแสง, ความหนาปานกลาง (ไม่ fog ทึบ) | เห็นลำแสงเป็นแท่งแยกช่องได้ |
| 4 | **Soft shadow + contact AO** | PCSS penumbra กว้างขึ้นตามระยะ occluder; AO เข้มที่รอยต่อบล็อก + ใต้ชาม | ขอบเงาฟุ้ง ไม่คม 1px + มีเงาเข้มจุดสัมผัส |
| 5 | **DOF** | focus ~1.5m (ชาม hero คม), ตู้/ฟริดจ์หลังละลายเป็น bokeh — feel ~f/2.8 | fg คม + bg เบลอชัด แยกระยะได้ |
| 6 | **Tone-map (ACES filmic)** | exposure คุมให้ **หน้าต่างสว่างแต่ยังไล่เกรน ไม่ขาวคลิป**, เงายังมีรายละเอียด | ซูมหน้าต่าง → ยังเห็น gradient ไม่ใช่ขาว 255 ล้วน |
| 7 | **PBR material** | ไม้ = albedo+roughness grain; ฟริดจ์ = metallic specular streak; เซรามิก = matte | 3 วัสดุตอบแสงต่างกันเห็นชัด |
| 8 | **Bloom** | ฟุ้งนุ่มเฉพาะหน้าต่าง/emissive (threshold สูง) — ไม่ฟุ้งทั้งเฟรม | เรืองที่หน้าต่างเท่านั้น ไม่ล้าง contrast |
| 9 | **Palette lock** | ~85% เฟรมโทนอุ่น (ทอง→น้ำตาล), teal accent ≤15% | histogram/สายตาเอนอุ่น, teal เป็นจุดตัด |

> **แกนห้ามพลาด (ถ้าตกข้อใดข้อหนึ่ง = FAIL ทันที):** #1 key · #2 bounce (ห้ามเงาดำ) · #4 soft shadow+AO ·
> #6 filmic (ห้ามหน้าต่างขาวคลิป) · voxel hard-edge geometry ยังอ่านออกว่าเป็นบล็อก.

---

## เกณฑ์ตัดสิน go / no-go (วัดพร้อม FPS เฟรมเดียว)

1. **Blind side-by-side:** วางเฟรมที่ spike เรนเดอร์ ข้าง `golden-beauty-shot-ref.png` → คนดูบอกได้ว่า
   "ลุคเดียวกัน" = ผ่าน visual. ถ้าดูเหมือน "Minecraft ไม่มี shader" = FAIL.
2. **วัด FPS บนเฟรม/มุมกล้องนี้เป๊ะ** พร้อมกัน:
   - **Ultra** (wgpu native): เป้า 60fps @1080p บน GPU กลาง (GTX 1660 / RX 5600 ขึ้นไป).
   - **Lite** (wgpu→WASM, WebGPU): เป้า 60fps บน integrated/mobile mid @720–1080p.
3. **ถ้าลุคผ่านแต่ FPS ตก** → ไล่ตัดชั้น "เสน่ห์" ตามลำดับใน look-bible §2/§3 (DOF → volumetric → RT GI → …)
   โดย **แกน #1/#2/#4/#6 ต้องอยู่ครบ** แล้วเรนเดอร์เทียบใหม่. ตัวเลขที่ตัดได้/ไม่ได้ = ผลลัพธ์จริงของ spike ที่ป้อนกลับ look-bible.

> จุดสำคัญ: **ลุคกับ FPS ต้องมาจากเฟรมเดียวกัน** — จะได้ตัดสิน go/no-go แบบ apples-to-apples
> ไม่ใช่ลุคจากฉากสวยแล้ว FPS จากฉากเปล่า.

---

_ส่งต่อ Bevy spike (Poppy). ค่าไหน Bevy ทำไม่ไหวจริงบนเครื่องเป้าหมาย โยนกลับมา — เอกสารนี้ปรับตามผลจริง. — Flamingo (Designer)_
