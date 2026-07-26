# Contact sheet: hero-golden-b (honey target) + 3 WIDE angles at the tone4 honey grade.
# Every wide tile rendered from voxelforge_shot.exe (fresh) and passing G3/G5/G6.
from PIL import Image, ImageDraw, ImageFont
CELL_W, CELL_H, BAR = 640, 360, 30
def load_fit(path):
    im = Image.open(path).convert("RGB")
    im.thumbnail((CELL_W, CELL_H), Image.LANCZOS)
    canvas = Image.new("RGB", (CELL_W, CELL_H), (18, 12, 10))
    canvas.paste(im, ((CELL_W-im.width)//2, (CELL_H-im.height)//2))
    return canvas
try: font = ImageFont.truetype("arial.ttf", 18)
except: font = ImageFont.load_default()
cells = [
    ("hero-golden-b.png", "GOLDEN-B  (honey target)  R-G 56 · sat 0.89"),
    ("wide-A.png",        "WIDE-A  3/4 to window  ·  R-G 54 · G3/G5/G6 PASS"),
    ("wide-B.png",        "WIDE-B  straight-on    ·  R-G 53 · G3/G5/G6 PASS"),
    ("wide-C.png",        "WIDE-C  3/4 to fridge  ·  R-G 53 · G3/G5/G6 PASS"),
]
cols = 2
rows = (len(cells)+cols-1)//cols
sheet = Image.new("RGB", (cols*CELL_W, rows*(CELL_H+BAR)), (10,7,6))
d = ImageDraw.Draw(sheet)
for i,(p,label) in enumerate(cells):
    r,c = divmod(i, cols)
    x,y = c*CELL_W, r*(CELL_H+BAR)
    d.rectangle([x,y,x+CELL_W,y+BAR], fill=(30,20,16))
    d.text((x+10,y+6), label, fill=(240,220,180), font=font)
    sheet.paste(load_fit(p), (x, y+BAR))
sheet.save("wide-compare-b.png")
print("saved wide-compare-b.png", sheet.size)
