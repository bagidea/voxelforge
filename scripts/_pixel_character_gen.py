"""
Monanisa / art lane — hand-authored 16x16 player character texture set.

Companion to `scripts/_pixel_blocks_gen.py` (same technique: seeded HSV
jitter, no external deps beyond Pillow). These five swatches back the
palette in `docs/player-character-art.md` for the FlyCam avatar LOD body
(head/torso/arms/legs boxes sized off PLAYER_HALF_W/PLAYER_HEIGHT in
client/src/main.rs) — they are NOT yet read by any Rust code (that body is a
box array today with flat StandardMaterial colours, the same convention
`characters.rs`/`equipment.rs` already use for Auren/Maren/Toma/Garren), so
these tiles are shipped ready-to-wire the same way water.png/metal.png were
in the block pass: no gameplay consumer yet, costs nothing, blocks nothing.

Art-lane only — does not touch any Rust code or the build.
"""

import colorsys
import math
import random
from pathlib import Path

from PIL import Image

OUT_DIR = Path(__file__).resolve().parent.parent / "assets" / "textures" / "character"
OUT_DIR.mkdir(parents=True, exist_ok=True)

SIZE = 16


def hsv(h, s, v):
    r, g, b = colorsys.hsv_to_rgb((h % 360) / 360.0, max(0.0, min(1.0, s)), max(0.0, min(1.0, v)))
    return (round(r * 255), round(g * 255), round(b * 255), 255)


def new_img():
    return Image.new("RGBA", (SIZE, SIZE))


def save(img, name):
    path = OUT_DIR / f"{name}.png"
    img.save(path)
    px = list(img.getdata())
    avg = tuple(round(sum(c[i] for c in px) / len(px)) for i in range(3))
    print(f"{name:8s} -> {path.name:12s} avg #{avg[0]:02x}{avg[1]:02x}{avg[2]:02x}")
    return avg


def jitter(rng, base, amt):
    return base + (rng.random() * 2 - 1) * amt


# ---------------------------------------------------------------------- skin
# #D9B08C — reused from equipment.rs::Surf::Skin. Very low-amplitude
# mottling only: this is a face/hand swatch, not a rock texture, so grain
# has to stay almost subliminal or it reads as a skin disease, not skin.
def make_skin():
    rng = random.Random("player-skin")
    img = new_img()
    H, S, V = 28, 0.36, 0.85
    for y in range(SIZE):
        for x in range(SIZE):
            v = V + jitter(rng, 0, 0.025)
            s = S + jitter(rng, 0, 0.03)
            if rng.random() < 0.04:
                v -= 0.05  # faint freckle/pore
                s += 0.05
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 2), s, v))
    return img


# --------------------------------------------------------------------- shirt
# #2E6E78 tunic teal — the one cool-hued garment (see player-character-art.md
# "why this survives golden hour"). Diagonal weave, not vertical/horizontal,
# so it doesn't read as planks like the wood block textures.
def make_shirt():
    rng = random.Random("player-shirt")
    img = new_img()
    H, S, V = 188, 0.62, 0.47
    for y in range(SIZE):
        for x in range(SIZE):
            weave = math.sin((x + y) * 0.9) * 0.05
            v = V + weave + jitter(rng, 0, 0.03)
            s = S + jitter(rng, 0, 0.04)
            if (x + y) % 5 == 0:
                v -= 0.05  # seam line every 5th diagonal
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 3), s, v))
    return img


# --------------------------------------------------------------------- pants
# #4A3220 legwear — reused from equipment.rs::Surf::LeatherStrap ("leather has
# a broad sheen" per characters.rs module doc). Vertical grain streaks +
# a soft diagonal sheen band, not the brushed-metal pattern metal.png uses.
def make_pants():
    rng = random.Random("player-pants")
    img = new_img()
    H, S, V = 26, 0.57, 0.29
    for y in range(SIZE):
        for x in range(SIZE):
            grain = math.sin(x * 1.1 + y * 0.15) * 0.03
            sheen = max(0.0, 1.0 - abs((x - y) - 2) / 5.0) * 0.07  # soft sheen band
            v = V + grain + sheen + jitter(rng, 0, 0.03)
            s = S - sheen * 0.5 + jitter(rng, 0, 0.03)
            if rng.random() < 0.05:
                v -= 0.06  # crease/fold shadow
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 2), s, v))
    return img


# --------------------------------------------------------------------- boots
# #2A1E16 — darker than any existing Surf on purpose (ground anchor colour,
# see player-character-art.md). Worn/scuffed grain + a stitched sole line
# across the bottom two rows.
def make_boots():
    rng = random.Random("player-boots")
    img = new_img()
    H, S, V = 24, 0.48, 0.17
    for y in range(SIZE):
        for x in range(SIZE):
            v = V + jitter(rng, 0, 0.03)
            s = S + jitter(rng, 0, 0.03)
            if rng.random() < 0.06:
                v += 0.05  # scuff catching light
                s -= 0.10
            if y >= SIZE - 2:
                # stitched sole line
                stitch = (x % 3 == 1)
                v = (0.22 if stitch else 0.10) + jitter(rng, 0, 0.02)
                s = 0.30
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 2), s, v))
    return img


# --------------------------------------------------------------------- cloak
# #E8A23C trim/cloak amber — the warm counterweight to the teal shirt, and
# the highest-saturation swatch in the set on purpose (see palette table).
# Fabric weave like shirt.png but with a frayed lighter edge along one side,
# since a cloak's defining silhouette cue is its hem, not its field.
def make_cloak():
    rng = random.Random("player-cloak")
    img = new_img()
    H, S, V = 36, 0.74, 0.91
    for y in range(SIZE):
        for x in range(SIZE):
            weave = math.sin((x - y) * 0.8) * 0.04
            v = V + weave + jitter(rng, 0, 0.03)
            s = S + jitter(rng, 0, 0.03)
            if x <= 1 and rng.random() < 0.6:
                v -= 0.12  # frayed hem edge, uneven
                s += 0.04
            img.putpixel((x, y), hsv(H + jitter(rng, 0, 3), min(s, 0.95), min(v, 1.0)))
    return img


MATERIALS = [
    ("skin", make_skin),
    ("shirt", make_shirt),
    ("pants", make_pants),
    ("boots", make_boots),
    ("cloak", make_cloak),
]


def main():
    avgs = {}
    for name, fn in MATERIALS:
        img = fn()
        assert img.size == (SIZE, SIZE)
        avgs[name] = save(img, name)
    return avgs


if __name__ == "__main__":
    main()
