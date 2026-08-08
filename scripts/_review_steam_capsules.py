"""Flamingo's review pass over docs/assets/steam/ — measures only, writes proof sheets. Read-only on the art."""
import os, math
import numpy as np
from PIL import Image, ImageFilter, ImageDraw

D = r"E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\docs\assets\steam"
OUT = os.path.join(D, "review")
os.makedirs(OUT, exist_ok=True)

VALVE = {  # Valve Steamworks graphical-asset spec (current, post-Aug-2024 store sizes)
    "header-capsule-920x430.png": (920, 430),
    "small-capsule-462x174.png": (462, 174),
    "main-capsule-1232x706.png": (1232, 706),
    "vertical-capsule-748x896.png": (748, 896),
    "page-background-1438x810.png": (1438, 810),
    "library-capsule-600x900.png": (600, 900),
    "library-hero-3840x1240.png": (3840, 1240),
}
# required slots that must EXIST in the deliverable set (page background is optional)
REQUIRED = {
    "header-capsule-920x430.png": (920, 430, "store: header capsule"),
    "small-capsule-462x174.png": (462, 174, "store: small capsule (search/wishlist)"),
    "main-capsule-1232x706.png": (1232, 706, "store: main capsule"),
    "vertical-capsule-748x896.png": (748, 896, "store: vertical capsule"),
    "library-capsule-600x900.png": (600, 900, "library: capsule"),
    "library-header-920x430.png": (920, 430, "library: header (separate upload slot)"),
    "library-hero-3840x1240.png": (3840, 1240, "library: hero"),
    "library-logo-1280x720.png": (1280, 720, "library: logo — TRANSPARENT png"),
}
MASTER_W = 1024  # both key-art masters are 1024x1024 (see F7)

def lum(a):  # Rec.709 luma on 0..255 float
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]

def hue_sat(a):
    r, g, b = a[..., 0] / 255, a[..., 1] / 255, a[..., 2] / 255
    mx, mn = np.max(a, axis=-1) / 255, np.min(a, axis=-1) / 255
    d = mx - mn
    h = np.zeros_like(mx)
    m = d > 1e-6
    idx = m & (mx == r); h[idx] = ((g - b)[idx] / d[idx]) % 6
    idx = m & (mx == g); h[idx] = ((b - r)[idx] / d[idx]) + 2
    idx = m & (mx == b); h[idx] = ((r - g)[idx] / d[idx]) + 4
    h = h * 60
    s = np.where(mx > 1e-6, d / np.maximum(mx, 1e-6), 0)
    return h, s, mx

print("=" * 88)
print("1. SPEC / GEOMETRY")
print("=" * 88)
for f in sorted(os.listdir(D)):
    if not f.endswith(".png"): continue
    raw = Image.open(os.path.join(D, f))
    mode, w, h = raw.mode, raw.size[0], raw.size[1]
    mb = os.path.getsize(os.path.join(D, f)) / 1e6
    tag = "(not a Valve slot — master)"
    if f in VALVE:
        ew, eh = VALVE[f]
        tag = "EXACT-MATCH" if (w, h) == (ew, eh) else f"MISMATCH exp {ew}x{eh}"
    up = w / MASTER_W
    print(f"  {f:32} {w:>5}x{h:<5} ar={w/h:6.4f} mode={mode:5} {mb:5.2f}MB "
          f"src-scale={up:5.2f}x  {tag}")

print("\n  -- required-slot roll call --")
for f, (ew, eh, why) in sorted(REQUIRED.items()):
    p = os.path.join(D, f)
    if os.path.exists(p):
        w, h = Image.open(p).size
        print(f"  {'PRESENT' if (w,h)==(ew,eh) else 'WRONG-SIZE':10} {f:32} {w}x{h}   {why}")
    else:
        print(f"  {'MISSING':10} {f:32} need {ew}x{eh}   {why}")

print("\n  -- aspect distortion: SOURCE crop-band ar vs the Valve-slot ar it was resized into --")
# NOT shipped-ar vs spec-ar: for a file that already EXACT-MATCHes its slot that delta is 0 by
# definition and measures nothing. Real squash/stretch happens in the Lanczos step of
# make_steam_capsules.py, which takes a band of the 1024x1024 master and forces it into the slot —
# so the honest number is the band's own ar vs the target ar. Bands mirror make_steam_capsules.py;
# _review_steam_confirm.py proves they are the bands actually used (maxdiff = 0 re-derivation).
SRC = {  # file -> ("band", top, bottom) horizontal band, or ("vert",) full-height centred crop
    "header-capsule-920x430.png":   ("band", 140, 619),
    "small-capsule-462x174.png":    ("band", 187, 573),
    "main-capsule-1232x706.png":    ("band", 130, 717),
    "page-background-1438x810.png": ("band", 135, 712),
    "library-hero-3840x1240.png":   ("band", 330, 660),
    "vertical-capsule-748x896.png": ("vert",),
    "library-capsule-600x900.png":  ("vert",),
}
MASTER_H = 1024
worst = 0.0
for f, (ew, eh) in sorted(VALVE.items()):
    p = os.path.join(D, f)
    if not os.path.exists(p) or f not in SRC: continue
    w, h = Image.open(p).size
    s = SRC[f]
    if s[0] == "band":
        bw, bh = MASTER_W, s[2] - s[1]
        band_txt = f"{bw}x{bh} (y{s[1]}-{s[2]})"
    else:  # crop_vertical(): band_w = int(H * tw/th), integer-truncated -> tiny real distortion
        bw, bh = int(MASTER_H * (ew / eh)), MASTER_H
        band_txt = f"{bw}x{bh} (centred)"
    d = 100 * abs(bw / bh - ew / eh) / (ew / eh)
    worst = max(worst, d)
    print(f"  {f:32} band {band_txt:20} ar={bw/bh:7.5f} -> slot ar={ew/eh:7.5f}  "
          f"squash={d:6.3f}%  {'ok' if d < 0.5 else 'VISIBLE'}")
print(f"  worst-case source->slot distortion = {worst:.3f}% "
      f"(threshold 0.5% ~ half a pixel of drift over a 100px feature)")

print()
print("=" * 88)
print("2. TONE / LEGIBILITY AT REAL STOREFRONT DISPLAY SIZE")
print("=" * 88)
# what the shopper actually sees, in CSS px, on the live store
DISPLAY = {
    "header-capsule-920x430.png":   [("store shelf / wishlist row", 292, 136), ("hover tooltip", 184, 86)],
    "small-capsule-462x174.png":    [("search result", 231, 87), ("dense list", 154, 58)],
    "main-capsule-1232x706.png":    [("front-page feature", 616, 353), ("carousel small", 308, 176)],
    "vertical-capsule-748x896.png": [("front-page vertical", 374, 448)],
    "library-capsule-600x900.png":  [("library grid tile", 150, 225), ("dense grid", 100, 150)],
    "library-hero-3840x1240.png":   [("library banner @1080p", 1280, 413)],
}
for f, modes in DISPLAY.items():
    im = Image.open(os.path.join(D, f)).convert("RGB")
    print(f"\n  {f}")
    for name, dw, dh in modes:
        t = im.resize((dw, dh), Image.LANCZOS)
        a = np.asarray(t).astype(np.float32)
        L = lum(a)
        p1, p50, p99 = np.percentile(L, [1, 50, 99])
        # squint test: blur hard, measure how much structure survives
        sq = np.asarray(t.convert("L").filter(ImageFilter.GaussianBlur(max(1, dw // 40)))).astype(np.float32)
        edge = np.asarray(t.convert("L").filter(ImageFilter.FIND_EDGES)).astype(np.float32).mean()
        print(f"    {name:26} {dw:>4}x{dh:<4} L: p1={p1:5.1f} med={p50:5.1f} p99={p99:5.1f} "
              f"range={p99-p1:5.1f} std={L.std():5.1f} | squint-std={sq.std():5.1f} edge={edge:5.2f}")

print()
print("=" * 88)
print("3. PALETTE LAW  (look-bible: ~85% warm, teal <=10-15%, shadows never pure black)")
print("=" * 88)
for f in sorted(os.listdir(D)):
    if not f.endswith(".png"): continue
    im = Image.open(os.path.join(D, f)).convert("RGB")
    if max(im.size) > 1400: im = im.resize((im.width // 3, im.height // 3), Image.LANCZOS)
    a = np.asarray(im).astype(np.float32)
    h, s, v = hue_sat(a)
    sat = s > 0.12
    warm = ((h >= 10) & (h <= 60)) & sat
    teal = ((h >= 160) & (h <= 210)) & sat
    L = lum(a)
    n = L.size
    print(f"  {f:34} warm={100*warm.mean():5.1f}%  teal={100*teal.mean():5.3f}%  "
          f"crushed(L<12)={100*(L<12).mean():5.2f}%  blown(L>250)={100*(L>250).mean():5.2f}%  "
          f"medL={np.median(L):5.1f}  satMed={np.median(s[sat]) if sat.any() else 0:4.2f}")

print()
print("=" * 88)
print("4. RESERVED LOGO ZONE — is it actually clean enough to drop a lockup on?")
print("=" * 88)
def zone_report(path, box, label):
    im = Image.open(path).convert("RGB")
    z = im.crop(box)
    g = z.convert("L")
    a = np.asarray(g).astype(np.float32)
    edge = np.asarray(g.filter(ImageFilter.FIND_EDGES)).astype(np.float32)
    # local contrast a white/gold logo would have to fight
    print(f"  {label:44} L med={np.median(a):5.1f} std={a.std():5.1f} "
          f"p5-p95={np.percentile(a,5):5.1f}-{np.percentile(a,95):5.1f} edgeE={edge.mean():5.2f} "
          f"-> {'CLEAN' if a.std() < 18 and edge.mean() < 6 else 'BUSY'}")

lp = os.path.join(D, "key-art-landscape-master.png")
pp = os.path.join(D, "key-art-portrait-master.png")
zone_report(lp, (0, 0, 341, 1024), "landscape master: left THIRD (doc 4 promise)")
zone_report(lp, (0, 0, 512, 1024), "landscape master: left HALF (doc 4 promise)")
zone_report(lp, (0, 0, 512, 340), "landscape master: left-half UPPER band only")
zone_report(pp, (0, 0, 1024, 256), "portrait master: TOP 25% (doc 4 promise)")
zone_report(os.path.join(D, "header-capsule-920x430.png"), (0, 0, 460, 430), "header capsule: left half")
zone_report(os.path.join(D, "small-capsule-462x174.png"), (0, 0, 462, 174), "small capsule: WHOLE frame (logo must fill it)")
zone_report(os.path.join(D, "main-capsule-1232x706.png"), (0, 0, 616, 706), "main capsule: left half")
zone_report(os.path.join(D, "vertical-capsule-748x896.png"), (0, 0, 748, 224), "vertical capsule: top 25%")
zone_report(os.path.join(D, "library-capsule-600x900.png"), (0, 0, 600, 225), "library capsule: top 25%")

print()
print("=" * 88)
print("5. EDGE-CLIP CHECK — does the hero touch/leave the frame?")
print("=" * 88)
def clip_report(path, label, dark_thresh=None):
    im = Image.open(path).convert("RGB")
    a = np.asarray(im).astype(np.float32)
    L = lum(a)
    t = dark_thresh if dark_thresh else np.percentile(L, 22)  # hero reads as the dark mass
    dark = L < t
    H, W = dark.shape
    band = max(2, H // 200)
    top = 100 * dark[:band, :].mean(); bot = 100 * dark[-band:, :].mean()
    lef = 100 * dark[:, :band].mean(); rig = 100 * dark[:, -band:].mean()
    print(f"  {label:34} dark-mass touching edge %:  top={top:5.1f} bottom={bot:5.1f} "
          f"left={lef:5.1f} right={rig:5.1f}")
for f in ["header-capsule-920x430.png", "small-capsule-462x174.png", "main-capsule-1232x706.png",
          "vertical-capsule-748x896.png", "library-capsule-600x900.png",
          "library-hero-3840x1240.png", "page-background-1438x810.png"]:
    clip_report(os.path.join(D, f), f)

print()
print("=" * 88)
print("6. RESOLUTION HONESTY — is the hero a real render or an upscale?")
print("=" * 88)
for f in ["library-hero-3840x1240.png", "main-capsule-1232x706.png", "page-background-1438x810.png",
          "header-capsule-920x430.png", "small-capsule-462x174.png", "vertical-capsule-748x896.png",
          "library-capsule-600x900.png"]:
    im = Image.open(os.path.join(D, f)).convert("L")
    a = np.asarray(im).astype(np.float32)
    lap = np.abs(np.asarray(im.filter(ImageFilter.FIND_EDGES)).astype(np.float32))
    # high-frequency energy per pixel: a Lanczos upscale has almost none
    fx = np.abs(np.diff(a, axis=1)).mean()
    fy = np.abs(np.diff(a, axis=0)).mean()
    print(f"  {f:34} per-px gradient x={fx:5.3f} y={fy:5.3f}  edgeE={lap.mean():5.2f}")
print("  (native 1024 master for reference:)")
im = Image.open(lp).convert("L"); a = np.asarray(im).astype(np.float32)
print(f"  {'key-art-landscape-master.png':34} per-px gradient x={np.abs(np.diff(a,axis=1)).mean():5.3f} "
      f"y={np.abs(np.diff(a,axis=0)).mean():5.3f}")

# ---------------- proof sheets ----------------
BG = (27, 40, 56)  # steam store slate

def sheet(items, path, cols_pad=24, label_h=22):
    from PIL import ImageFont
    try: font = ImageFont.truetype("arial.ttf", 13)
    except Exception: font = ImageFont.load_default()
    tw = sum(i[1].width for i in items) + cols_pad * (len(items) + 1)
    th = max(i[1].height for i in items) + cols_pad * 2 + label_h
    c = Image.new("RGB", (tw, th), BG)
    d = ImageDraw.Draw(c)
    x = cols_pad
    for label, img in items:
        c.paste(img, (x, cols_pad + label_h))
        d.text((x, cols_pad - 4), label, fill=(200, 210, 220), font=font)
        x += img.width + cols_pad
    c.save(path)
    print("  wrote", path)

print()
print("=" * 88)
print("7. PROOF SHEETS")
print("=" * 88)
hdr = Image.open(os.path.join(D, "header-capsule-920x430.png")).convert("RGB")
lib = Image.open(os.path.join(D, "library-capsule-600x900.png")).convert("RGB")
main = Image.open(os.path.join(D, "main-capsule-1232x706.png")).convert("RGB")
small = Image.open(os.path.join(D, "small-capsule-462x174.png")).convert("RGB")

sheet([("header 920x430 shown at display 460x215", hdr.resize((460, 215), Image.LANCZOS)),
       ("small capsule 462x174 at display 231x87", small.resize((231, 87), Image.LANCZOS)),
       ("hover 184x86", hdr.resize((184, 86), Image.LANCZOS))],
      os.path.join(OUT, "sim-header-at-real-sizes.png"))

sheet([("library tile 150x225", lib.resize((150, 225), Image.LANCZOS)),
       ("dense grid 100x150", lib.resize((100, 150), Image.LANCZOS)),
       ("wishlist thumb 64x96", lib.resize((64, 96), Image.LANCZOS))],
      os.path.join(OUT, "sim-library-tiny.png"))

# squint / value-structure test: does anything read when you kill colour + detail?
def squint(img, w):
    s = img.resize((w, int(img.height * w / img.width)), Image.LANCZOS).convert("L")
    s = s.filter(ImageFilter.GaussianBlur(1.2))
    return s.resize((img.width // 2, img.height // 2), Image.NEAREST).convert("RGB")
sheet([("main capsule - value only, squinted", squint(main, 40)),
       ("header - value only, squinted", squint(hdr, 40))],
      os.path.join(OUT, "squint-value-test.png"))

# logo-zone overlay on the landscape master
ov = Image.open(lp).convert("RGB").copy()
d = ImageDraw.Draw(ov, "RGBA")
d.rectangle([0, 0, 341, 1024], fill=(0, 200, 255, 46), outline=(0, 220, 255, 255), width=3)
d.rectangle([341, 0, 512, 1024], fill=(255, 0, 120, 40), outline=(255, 60, 140, 255), width=3)
d.text((14, 12), "promised CLEAN: left third", fill=(180, 245, 255, 255))
d.text((352, 12), "promised clean to half", fill=(255, 190, 215, 255))
ov.save(os.path.join(OUT, "logo-zone-overlay-landscape.png"))
print("  wrote", os.path.join(OUT, "logo-zone-overlay-landscape.png"))

ov2 = Image.open(os.path.join(D, "library-capsule-600x900.png")).convert("RGB").copy()
d2 = ImageDraw.Draw(ov2, "RGBA")
d2.rectangle([0, 0, 600, 225], fill=(0, 200, 255, 46), outline=(0, 220, 255, 255), width=3)
d2.text((12, 8), "promised CLEAN: top 25% for title lockup", fill=(180, 245, 255, 255))
ov2.save(os.path.join(OUT, "logo-zone-overlay-library.png"))
print("  wrote", os.path.join(OUT, "logo-zone-overlay-library.png"))
