# การออกแบบ Level ที่นำสายตาผู้เล่นโดยไม่พึ่งลูกศร UI

> **For:** Voxelforge — AAA voxel action-adventure สไตล์ soulslike กล้อง third-person  
> **Researched by:** Sahara  
> **Date:** 2026-07-31

---

## สรุปผล (Executive Summary)

ระบบนำทางแบบไม่ใช้ UI arrow อาศัย **การออกแบบสภาพแวดล้อม (environmental wayfinding)** ผู้เล่นจดจำทิศทางจากจุดสังเกต แสง สี รูปทรง และเสียง ไม่ใช่ mini-map waypoint บทความนี้รวบรวมหลักการ 8 ข้อ พร้อมตัวอย่างจากเกมจริง และวิธีประยุกต์ใช้กับโลก voxel + มุมมองบุคคลที่สาม

| # | หลักการ | เกมตัวอย่าง |
|---|---------|-------------|
| 1 | **Landmarks / Weenies** | *Ghost of Tsushima*, *Dark Souls*, *Elden Ring*, *Hytale* |
| 2 | **Triangles & Occlusion** | *The Legend of Zelda: Breath of the Wild* |
| 3 | **Framing / การจัดเฟรมกล้อง** | *Uncharted 4*, *The Last of Us* |
| 4 | **Leading Lines / เส้นนำสายตา** | *Half-Life 2*, *Dear Esther*, *Call of Juarez* |
| 5 | **Light, Color & Contrast** | *Uncharted 4*, *Dark Souls*, *Elden Ring* |
| 6 | **Negative Space & Pacing** | *Breath of the Wild*, *Journey*, *Silent Hill* |
| 7 | **Sound & Audio Landmarks** | *Ghost of Tsushima*, *Silent Hill*, *Breath of the Wild* |
| 8 | **Environmental Breadcrumbing** | *Uncharted 4*, *Dark Souls*, *The Last of Us* |

---

## 1. Landmarks / Weenies (จุดสังเกต / แรงดึงสายตา)

### หลักการ
"Weenie" เป็นคำจาก Disney Imagineering: สิ่งก่อสร้าง/รูปทรงสูงใหญ่ที่ดึงสายตาและดึงตัวผู้เล่นให้เดินไปหาได้โดยอัตโนมัติ ในเกม landmarks ทำหน้าที่:
- ให้ทิศทั้งระยะใกล้และระยะไกล
- ช่วยสร้าง mental map
- บ่งบอกเป้าหมายหลักหรือจุดเปลี่ยนเนื้อเรื่อง

### ตัวอย่างจากเกมจริง
- **Ghost of Tsushima (Sucker Punch, GDC 2021)** — Parker Hamilton อธิบายว่าทีมใช้ **Flags** (ควัน อาคาร ใบเมเปิ้ลแดง นก) และ **Breadcrumbs** (ประตูศาลเจ้า Torii Gate) โดยกฎคือ "ต้องมีอะไรดึงตาผู้เล่นทุก 30 วินาที" แลนด์มาร์กถูกเน้นด้วย particle (ใบไม้ แมลงเรืองแสง) และเสียง เพื่อให้การสำรวจรู้สึก organic ไม่พึ่ง mini-map [[1]](https://www.gamedeveloper.com/design/a-taxonomy-of-weenies-the-landmarks-that-define-i-ghost-of-tsushima-i-) [[2]](https://themouselets.com/what-is-a-disney-weenie)
- **Dark Souls** — กองไฟ (bonfire) ทำหน้าที่ "monument" ไม่ใช่แค่ checkpoint ตัวอย่างเช่น bonfire แรกใน Undead Asylum ตั้งอยู่ในบริเวณลานที่เป็นศูนย์กลางของดันเจี้ยน ผู้เล่นวนกลับมาเพื่อปลดล็อก shortcut ทำให้การนำทางเป็น "ปริศนาเชิงพื้นที่" มากกว่าการตาม marker [[3]](https://www.gamedeveloper.com/design/level-design-the-linear-labyrinth) [[4]](https://book.leveldesignbook.com/studies/sp/undead-burg)
- **Elden Ring** — **Erdtree** มองเห็นได้จากเกือบทุกจุดในโลก ทำหน้าที่เป็น "connective tissue" บอกทิศทางสู่เป้าหมายสูงสุด Divine Towers แต่ละภูมิภาค และลำแสงจาก Sites of Grace ก็ช่วยชี้ทิศโดยไม่ต้องใช้ลูกศร UI [[5]](https://www.gamedeveloper.com/game-platforms/narrative-design-in-elden-ring) [[6]](https://www.wayline.io/blog/invisible-walls-guiding-freedom-open-world-games)
- **Hytale** — Point-of-Interest (POI) ถูกสร้างเป็น prefab ที่มี silhouette และฟังก์ชันชัดเจน ผู้เล่นจะจำได้ว่า "เห็นแบบนี้คือจะเจอ encounter แบบไหน" ก่อนที่ world-gen จะวางลงไปใน terrain [[7]](https://hytale.com/news/2019/3/the-creation-of-a-new-point-of-interest-for-adventure-mode)

### ประยุกต์ใช้กับ Voxelforge
- สร้าง **วัตถุสูงเกิน skyline** ด้วยบล็อก เช่น หอคอยที่ถูกไฟไหม้ ต้นไม้ยักษ์ หรือเสาแก๊สพุ่งสูง
- ให้แลนด์มาร์กมี **silhouette ที่อ่านง่าย** แม้มองจากระยะไกลบนหน้าจอ third-person
- ใช้ particle รอบแลนด์มาร์ก เช่น ควัน ผีเสื้อไฟ นกบินวน เพื่อดึงตาแบบ diegetic
- ติดตั้ง **bonfire/shrine** เป็น monument ที่ผู้เล่นวนกลับมาเพื่อปลด shortcut

---

## 2. Triangles & Occlusion (กฎสามเหลี่ยม / บังสายตา)

### หลักการ
รูปทรงสามเหลี่ยม (เนินเขา ยอดเขา สันเขา) เป็นบล็อกตัวตัดของ open-world design เพราะ:
1. ให้ตัวเลือก: ปีนข้าม หรือ เดินอ้อม
2. บังวิสัยทัศน์: สร้างความอยากรู้ว่าด้านหลังมีอะไร

### ตัวอย่างจากเกมจริง
- **The Legend of Zelda: Breath of the Wild (CEDEC 2017)** — Hidemaro Fujibayashi และ Makoto Yonezu อธิบาย "Triangle Rule" ในเซสชัน *Field Level Design in The Legend of Zelda: Breath of the Wild* ทีมใช้สามเหลี่ยม 3 ขนาด:
  - **ใหญ่** = landmark มองเห็นไกล
  - **กลาง** = บัง POI เพื่อสร้างเซอร์ไพรส์
  - **เล็ก** = ควบคุม pacing/จังหวะการเคลื่อนที่  
  การออกแบบแบบนี้ลดการพึ่ง mini-map และ "detective vision" [[8]](https://www.gamedeveloper.com/design/5-design-lessons-learned-from-i-the-legend-of-zelda-breath-of-the-wild-i-) [[9]](https://www.blog.radiator.debacle.us/2017/10/open-world-level-design-spatial.html)

### ประยุกต์ใช้กับ Voxelforge
- ใช้ **เนิน/สันเขาแบบบล็อกขั้นบันได** บัง shrine/บ้านร้าง/ศัตรู camp
- หลีกเลี่ยงทางตรงยาว: ให้เส้นทางโค้งผ่านเนิน เพื่อค่อย ๆ reveal ฉาก
- ใช้ขนาดสามเหลี่ยมควบคุม pacing ของ combat encounter — เนินเล็กบัง line-of-sight ของศัตรู

---

## 3. Framing / การจัดเฟรมกล้อง

### หลักการ
Framing คือการใช้องค์ประกอบรอบ ๆ (ประตู หน้าต่าง กำแพงซาก) ล้อมเป้าหมายไว้ เหมือนกรอบรูป ทำให้สายตาผู้เล่นจดจ่อกับจุดที่ต้องการได้ทันที

### ตัวอย่างจากเกมจริง
- **Uncharted 4** — ตำแหน่งเริ่มต้นของฉากถูกออกแบบให้ผู้เล่นมองเห็นเส้นทางหลัก ช่องทางเลือก และเป้าหมายพร้อมกัน โดยใช้สภาพแวดล้อมเป็นกรอบนำสายตา [[10]](https://www.gamedeveloper.com/design/level-design-tips-and-tricks)
- **The Last of Us** — สะพานที่ Joel ต้องไปถูก "กรอบ" ด้วยตึกสูงสองข้างทาง ทำให้ผู้เล่นเดินไปตามทิศทางโดยอัตโนมัติ [[11]](https://www.gamedeveloper.com/design/my-insight-on-how-level-flow-is-applied-in-games-like-uncharted-4-the-last-of-us)
- **GTA V / Rockstar North (GDC 2019)** — Miriam Bellard อธิบายใน *Environment Design as Spatial Cinematography* ว่า composition แบบภาพยนตร์สามารถใช้กับ level design ได้ โดยต้อง balance ระหว่าง artistic framing และ gameplay affordance [[12]](https://book.leveldesignbook.com/process/blockout/massing/composition)

### ประยุกต์ใช้กับ Voxelforge
- สร้าง **ซากประตู/ช่องเขา/หน้าผา** ที่มองทะลุไปเห็นแลนด์มาร์ก
- ใน third-person camera ต้องคำนึงถึง **ตำแหน่งกล้องหลังตัวละคร**: อย่าให้กรอบสูงเกินไปจน遮挡 player model
- ใช้ **เปิดโล่ง (vista)** หลังจากผ่านทางแคบ เพื่อ reward ผู้เล่นด้วยภาพกว้าง

---

## 4. Leading Lines / เส้นนำสายตา

### หลักการ
Leading lines คือเส้นที่สร้างจากขอบของสิ่งของใน scene (ถนน ราวบันได ขอบตึก แม่น้ำ) นำสายตาไปสู่เป้าหมาย Mateusz Piaskiewicz แยกเป็น:
- **Practical lines** — เส้นจริงที่มองเห็น เช่น ขอบอาคาร ทางเดิน
- **Virtual lines** — เส้นที่สมองตีความจากทิศทางสายตาของ NPC/วัตถุ [[13]](https://www.gamedeveloper.com/design/composition-in-level-design)

### ตัวอย่างจากเกมจริง
- *Half-Life 2* — ขอบปล่องไฟ/ตึกนำสายตาขึ้นสู่จุดหมาย
- *Dear Esther* — เส้นขอบฟ้าของทะเลนำสายตาไปตามทิศทาง
- *Call of Juarez: Bound in Blood* — ทางเดินโค้งระหว่างหินนำสายตา
- *Fallout: New Vegas* — ซากอุโมงค์ที่เอียงสร้างเส้นนำสายตา [[13]](https://www.gamedeveloper.com/design/composition-in-level-design)

### ข้อโต้แย้ง (Caveat)
*The Level Design Book* วิจารณ์ leading lines ว่าเป็น "brain poison" ในบางกรณี เพราะภาพนิ่งไม่ใช่ประสบการณ์การเล่นจริง ผู้เล่นนำทางด้วย spatial understanding และ sightlines มากกว่าเส้นใน 2D ควรใช้ leading lines เป็นเครื่องมือเสริม ไม่ใช่หลัก [[12]](https://book.leveldesignbook.com/process/blockout/massing/composition)

### ประยุกต์ใช้กับ Voxelforge
- ใช้ **ทางเดิน รั้ว สะพาน แม่น้ำ หรือแถวต้นไม้** สร้างเส้นนำสายตา
- ในโลกบล็อก ขอบของ voxel terraces/retaining walls เป็น practical lines ที่อ่านง่าย
- รวมกับ landmark: เส้นทางโค้งนำตาไปหาแลนด์มาร์กระยะไกล

---

## 5. Light, Color & Contrast

### หลักการ
แสง สี และความคมชัดเป็นสัญญาณ visual ที่สมองประมวลผลเร็วที่สุด:
- พื้นที่สว่างดึงดูดสายตามากกว่าที่มืด
- สีอุ่น (แดง เหลือง ส้ม) ดึงดูดและบ่งบอกเป้าหมาย/ปฏิสัมพันธ์
- สีเย็น (น้ำเงิน เขียวอมฟ้า) ถอยหลังเป็นพื้นหลัง

### ตัวอย่างจากเกมจริง
- **Uncharted 4** — ขอบที่ปีนได้ถูกทาด้วยสีเหลืองอ่อน บอกผู้เล่นโดยไม่ต้องใช้ไอคอน [[11]](https://www.gamedeveloper.com/design/my-insight-on-how-level-flow-is-applied-in-games-like-uncharted-4-the-last-of-us)
- **Dark Souls / Elden Ring** — ใช้แสงจากกองไฟ, ลำแสง Sites of Grace, และบรรยากาศมืดสลัวแบ่งแยก safe zone กับ danger zone [[6]](https://www.wayline.io/blog/invisible-walls-guiding-freedom-open-world-games)
- **เกมทั่วไป** — แสงลำแสง (god rays) และ environmental color ชี้เส้นทางและเป้าหมายโดยไม่ต้องใช้ UI [[14]](https://generalistprogrammer.com/tutorials/game-level-design-complete-environment-guide-2025)

### ประยุกต์ใช้กับ Voxelforge
- วาง **คบเพลิง/โคมไฟ** ตามเส้นทางหลัก; ทางรองหรืออันตรายให้มืดกว่า
- ใช้ **ore/แร่เรืองแสง** หรือบล็อก emissive เป็นจุดดึงตา
- ตั้ง **biome color palette** ที่ตัดกัน: ป่าเขียวหม่น → ทุ่งดอกไม้สีส้ม → ถ้ำหินเหล็กสีน้ำเงิน
- ใน third-person ระวังว่าแสงสว่างจนเกินไปอาจทำให้ player model หายไปในฉาก

---

## 6. Negative Space & Pacing

### หลักการ
Negative space คือพื้นที่ว่างระหว่างสิ่งของ (positive space) มันไม่ใช่ "ที่ว่างเปล่า" แต่เป็นเครื่องมือควบคุม pacing, อารมณ์ และการนำสายตา

### ตัวอย่างจากเกมจริง
- **Breath of the Wild** — Hyrule กว้างใหญ่และค่อนข้างว่าง แต่ความว่างนั้นตั้งใจ ทำให้การค้นพบ shrine/vista รู้สึกคุ้มค่า ไม่ใช่ "theme park" ที่วัตถุเต็มไปหมด [[15]](https://www.wayline.io/blog/level-design-negative-space)
- **Journey** — ทะเลทรายกว้างทำหน้าที่กรอบแลนด์มาร์กระยะไกล
- **Silent Hill** — หมอกและความเงียบเปลี่ยนถนนว่างให้กลายเป็นความหวาดกลัว [[16]](https://dev.epicgames.com/community/learning/tutorials/3VKJ/unreal-engine-fortnite-level-design-fundamentals)
- Epic Developer Community สรุปว่า positive space รู้สึกปลอดภัยและกระชับ ส่วน negative space รู้สึกเปิดโล่งและผลักดันให้เคลื่อนที่เร็วขึ้น [[16]](https://dev.epicgames.com/community/learning/tutorials/3VKJ/unreal-engine-fortnite-level-design-fundamentals)

### ประยุกต์ใช้กับ Voxelforge
- สลับจังหวะ: ดันเจี้ยนแคบ/ชุมชน (positive) → ทุ่ง/ที่ราบกว้าง (negative) → ดันเจี้ยนถัดไป
- ในพื้นที่ว่าง ให้มี **แลนด์มาร์กเดียวที่เด่น** อย่าเติม prop จนลายตา
- หลีกเลี่ยง "prop dump" และ "texture overload" ที่ทำลาย negative space [[15]](https://www.wayline.io/blog/level-design-negative-space)

---

## 7. Sound & Audio Landmarks

### หลักการ
เสียงเป็นเครื่องมือนำทางที่ไม่ต้องใช้สายตา และทำงานได้ดีในทุกมุมกล้อง:
- **Ambient audio** — น้ำตก ลม พายุ นก แมลง
- **Audio landmarks / sonic landmarks** — เสียงเฉพาะบริเวณ เช่น ระฆัง คำภีร์ กลองศึก
- **Spatial audio** — เสียงดังขึ้นเมื่อใกล้เป้าหมาย

### ตัวอย่างจากเกมจริง
- **Ghost of Tsushima** — แลนด์มาร์กถูกให้ identity ด้วยเสียงประกอบ เช่น ใบไม้ร่วง นก แมลงเรืองแสง [[1]](https://www.gamedeveloper.com/design/a-taxonomy-of-weenies-the-landmarks-that-define-i-ghost-of-tsushima-i-)
- **Silent Hill** — Akira Yamaoka ใช้เสียงสร้างความหวาดกลัวและสภาพแวดล้อมที่ไร้ทิศทาง ทำให้ผู้เล่นพึ่งเสียงในการสำรวจ [[17]](https://designingsound.org/2010/03/22/akira-yamaokas-sound-design-lecture-at-gdc-2010/)
- งานวิจัยด้าน level design ระบุว่า sound เป็น factor สำคัญในการตัดสินใจและ wayfinding ของผู้เล่น ร่วมกับแสง สี สถาปัตยกรรม และ affordance [[18]](https://press-start.gla.ac.uk/press-start/article/download/129/92/846)

### ประยุกต์ใช้กับ Voxelforge
- ให้ **shrine/บ่อน้ำ/ถ้ำ** มีเสียงประจำตัว เช่น ระฆังกังวาน เสียงน้ำไหล เสียงคำรามของบอส
- ใช้ **spatial audio** ของลมหรือนกในทุ่งโล่ง เพื่อบ่งบอกขนาดของพื้นที่
- ระวัง mixing: อย่าให้ ambient ดังจนกลบเสียงศัตรู/telegraph

---

## 8. Environmental Breadcrumbing

### หลักการ
Breadcrumbing คือการทิ้ง "เบรดครัมบ์" หรือเบาะแสเล็ก ๆ ใน environment เพื่อชี้ทิศทาง โดยไม่ต้องใช้ marker

### ตัวอย่างจากเกมจริง
- **Uncharted 4** — ใช้ lines, pathways, รูปทรงภูเขา, หลังคาบ้าน และตัวละคร Nathan Drake ชี้มือไปยัง mega structure เป็นเบาะแสต่อเนื่อง [[11]](https://www.gamedeveloper.com/design/my-insight-on-how-level-flow-is-applied-in-games-like-uncharted-4-the-last-of-us)
- **Dark Souls** — ศัตรูและ fog gate ทำหน้าที่ breadcrumbs ดึงผู้เล่นไปยังพื้นที่ถัดไป [[4]](https://book.leveldesignbook.com/studies/sp/undead-burg)
- **The Last of Us** — ร่องรอยเลือด เศษซาก รอยเท้า นำผู้เล่นไปตามเส้นทาง

### ประยุกต์ใช้กับ Voxelforge
- วาง **ไอเท็มเรืองแสง** เช่น เหรียญ, ดอกไม้, สมุนไพร นำไปสู่ทางลับ
- ใช้ **ร่องรอยความเสียหาย**: รั้วหัก, รอยไฟไหม้, เศษบล็อกที่กระจัดกระจาย
- ศัตรู patrolling จาก camp หนึ่งไปอีก camp สามารถชี้ทางไปยังฐานใหญ่ได้

---

## สรุปการประยุกต์ใช้ใน Voxelforge (Voxel + Third-Person)

| ปัญหาของ Voxelforge | วิธีแก้ด้วยหลักการข้างต้น |
|----------------------|---------------------------|
| โลกบล็อกอาจดูซ้ำซากจำเจ | สร้าง **landmarks** ที่มี silhouette ชัดและ material ต่างจากพื้นที่รอบ |
| ผู้เล่นหลงในพื้นที่กว้าง | ใช้ **triangles/เนิน** บังทิศทาง + **negative space** ที่มี landmark เดียวบนขอบฟ้า |
| กล้อง third-person บังมุมมอง | ออกแบบ **framing** ด้วยประตู/ช่องเขาสูงพอที่มองทะลุได้ แต่ไม่บังตัวละคร |
| ไม่มี UI arrow | ใช้ **light/color contrast** กับ **leading lines** จากทางเดิน/แม่น้ำ |
| ต้องการบรรยากาศ | ใช้ **sound landmarks** และ ambient ที่เปลี่ยนตาม biome |
| ต้องการชี้ทางไปบอส/ดันเจี้ยน | ใช้ **breadcrumbing** จากศัตรู, ไอเท็ม, และร่องรอยสิ่งแวดล้อม |

---

## Caveats / ข้อควรระวัง

1. **Leading lines ไม่ใช่คำตอบเดียว** — *The Level Design Book* เตือนว่า leading lines อาจเป็นการอ่านภาพนิ่งเกินไป การนำทางที่ดีต้องมาจาก spatial wayfinding + playtest จริง [[12]](https://book.leveldesignbook.com/process/blockout/massing/composition)
2. **Third-person camera ซ่อนข้อมูลได้** — มุมกล้องหลังตัวละครอาจบัง landmark หรือเส้นทางด้านหลัง ควรมี camera cheat เล็กน้อยหรือออกแบบให้มองเห็นได้จากหลายมุม
3. **Voxel มีข้อจำกัดเรื่อง gradient** — แสงเงา/สีอาจดู blocky ควรใช้ material และ post-process ช่วยเพิ่ม readability
4. **อย่าใส่เบาะแสมากเกินไป** — particle, แสง, เสียง ที่เยอะเกินไปจะกลายเป็น noise ทำลาย negative space
5. **Accessibility** — ผู้เล่นบางคนอาจมีปัญหาการมองเห็นหรือได้ยิน ควรมี option เสริม เช่น subtitle สำหรับเสียงนำทาง หรือ high-contrast mode

---

## แหล่งอ้างอิง

1. Game Developer — "A taxonomy of Weenies: the landmarks that define *Ghost of Tsushima*"  
   https://www.gamedeveloper.com/design/a-taxonomy-of-weenies-the-landmarks-that-define-i-ghost-of-tsushima-i-
2. The Mouselets — "What Is A Disney Weenie?"  
   https://themouselets.com/what-is-a-disney-weenie
3. Game Developer — "Level Design: The Linear Labyrinth"  
   https://www.gamedeveloper.com/design/level-design-the-linear-labyrinth
4. The Level Design Book — "Undead Burg (Dark Souls 1)"  
   https://book.leveldesignbook.com/studies/sp/undead-burg
5. Game Developer — "Narrative Design in Elden Ring"  
   https://www.gamedeveloper.com/game-platforms/narrative-design-in-elden-ring
6. Wayline — "Invisible Walls: Guiding Freedom in Open-World Games"  
   https://www.wayline.io/blog/invisible-walls-guiding-freedom-open-world-games
7. Hytale Dev Blog — "The creation of a new point of interest for adventure mode"  
   https://hytale.com/news/2019/3/the-creation-of-a-new-point-of-interest-for-adventure-mode
8. Game Developer — "5 design lessons learned from *Breath of the Wild*"  
   https://www.gamedeveloper.com/design/5-design-lessons-learned-from-i-the-legend-of-zelda-breath-of-the-wild-i-
9. Radiator Blog — "Open world level design: spatial composition and flow in *Breath of the Wild*"  
   https://www.blog.radiator.debacle.us/2017/10/open-world-level-design-spatial.html
10. Game Developer — "Level Design Tips and Tricks"  
    https://www.gamedeveloper.com/design/level-design-tips-and-tricks
11. Game Developer — "My insight on how level flow is applied in games like *Uncharted 4* & *The Last of Us*"  
    https://www.gamedeveloper.com/design/my-insight-on-how-level-flow-is-applied-in-games-like-uncharted-4-the-last-of-us
12. The Level Design Book — "Composition"  
    https://book.leveldesignbook.com/process/blockout/massing/composition
13. Game Developer — "Composition in Level Design"  
    https://www.gamedeveloper.com/design/composition-in-level-design
14. Generalist Programmer — "Game Level Design: Complete Environment Creation Guide 2025"  
    https://generalistprogrammer.com/tutorials/game-level-design-complete-environment-guide-2025
15. Wayline — "The Power of Emptiness: Level Design and the Art of Negative Space"  
    https://www.wayline.io/blog/level-design-negative-space
16. Epic Developer Community — "Level Design Fundamentals"  
    https://dev.epicgames.com/community/learning/tutorials/3VKJ/unreal-engine-fortnite-level-design-fundamentals
17. Designing Sound — "Akira Yamaoka's Sound Design Lecture at GDC 2010"  
    https://designingsound.org/2010/03/22/akira-yamaokas-sound-design-lecture-at-gdc-2010/
18. Press Start — "Level Design: Resources and Techniques to Guide the Player"  
    https://press-start.gla.ac.uk/press-start/article/download/129/92/846

---

*สิ้นสุดเอกสาร — จัดทำโดย Sahara สำหรับ Voxelforge*
