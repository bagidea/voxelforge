# Build a labelled contact sheet: golden ref + shipped cramped + 3 wide angles.
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
    ("docs/assets/golden-beauty-shot-ref.png", "GOLDEN REF  (target)  ·  G3/G5/G6 PASS"),
    ("hero-look-final.png",                    "SHIPPED hero  (cramped, dark void at top)"),
    ("wide-A.png",                             "WIDE-A  3/4 toward window  ·  G3/G5/G6 PASS"),
    ("wide-B.png",                             "WIDE-B  straight-on  ·  G3/G5/G6 PASS"),
    ("wide-C.png",                             "WIDE-C  3/4 toward fridge  ·  G3/G5/G6 PASS"),
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
sheet.save("wide-compare-sheet.png")
print("saved wide-compare-sheet.png", sheet.size)
