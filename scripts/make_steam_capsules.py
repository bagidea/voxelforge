"""Crop the two Steam key-art masters into the capsule set. Run once, ad hoc — not part of the build."""
from PIL import Image

SRC_DIR = r"E:\Projects\bagidea-ai-agents-office\workspace\uploads"
OUT_DIR = r"E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\docs\assets\steam"

landscape = Image.open(f"{SRC_DIR}\\gen_1785505599301.png").convert("RGB")
portrait = Image.open(f"{SRC_DIR}\\gen_1785505667008.png").convert("RGB")

landscape.save(f"{OUT_DIR}\\key-art-landscape-master.png")
portrait.save(f"{OUT_DIR}\\key-art-portrait-master.png")

W, H = landscape.size  # 1024x1024

def crop_band(img, top, bottom, target_w, target_h, name):
    w, h = img.size
    band = img.crop((0, top, w, bottom))
    band = band.resize((target_w, target_h), Image.LANCZOS)
    band.save(f"{OUT_DIR}\\{name}")
    print(name, band.size)

# Header capsule 460x215 (~2.14:1) — head-to-knee band, god-ray gap visible left-of-center
crop_band(landscape, 140, 618, 460, 215, "header-capsule-460x215.png")

# Main capsule 616x353 (~1.75:1) — a bit taller, more environment top/bottom
crop_band(landscape, 130, 717, 616, 353, "main-capsule-616x353.png")

# Library hero — ultrawide band, upscaled straight to the shipped 3840x1240 in one Lanczos
# pass (the crop band itself is ~1024x330 since 3840px wide exceeds this master's native
# 1024x1024 resolution — this IS the upscale step, not a placeholder; re-run this file if it
# ever ships soft at full zoom, this time rendering the master at native 3840px instead)
crop_band(landscape, 330, 660, 3840, 1240, "library-hero-3840x1240.png")

# Library capsule 600x900 (2:3 portrait) — full height, centered crop on the hero
pw, ph = portrait.size
target_w = int(ph * (600 / 900))
x0 = (pw - target_w) // 2
lib = portrait.crop((x0, 0, x0 + target_w, ph)).resize((600, 900), Image.LANCZOS)
lib.save(f"{OUT_DIR}\\library-capsule-600x900.png")
print("library-capsule-600x900.png", lib.size)
