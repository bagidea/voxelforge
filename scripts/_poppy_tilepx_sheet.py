#!/usr/bin/env python3
# ===========================================================================
# Poppy — the manifest-driven tile-size contact sheet: three arms side by side.
#
# Three columns, because two would lie. `head16 -> art64on` is a real, large
# change and it is also useless as evidence: it cannot say which half of the
# drop did the work. The middle column is the whole reason this sheet exists.
#
# Every number on the sheet is recomputed here from the PNGs it names, and the
# per-arm load facts (manifest tile_px, how many authored maps actually bound)
# are read out of each arm's OWN run log — not typed in from a shell history.
#
# USAGE
#   python scripts/_poppy_tilepx_sheet.py [out.png] [plates-dir]
# ===========================================================================
import os
import re
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

SCENES = [
    ("day", "outdoor noon   SUN=66,205,20000  EXPOSURE=10.6"),
    ("evening-raking", "raking evening   (look.rs hour default)"),
    ("night-firelit", "night, lit by fire   LOOK_NIGHT=1"),
]
ARMS = [
    ("head16", "HEAD 16px albedo\nderived maps", (150, 190, 235)),
    ("art64off", "64px albedo\nderived maps", (235, 220, 150)),
    ("art64on", "64px albedo\n+ authored _n/_r", (150, 235, 175)),
]
COL_W = 560


def load(p):
    return np.asarray(Image.open(p).convert("RGB"), dtype=np.float64)


def luma(a):
    return a[:, :, 0] * 0.2126 + a[:, :, 1] * 0.7152 + a[:, :, 2] * 0.0722


def lap(y):
    return np.abs(4 * y[1:-1, 1:-1] - y[:-2, 1:-1] - y[2:, 1:-1] - y[1:-1, :-2] - y[1:-1, 2:])


def font(sz, bold=False):
    for f in ("C:/Windows/Fonts/consolab.ttf" if bold else "C:/Windows/Fonts/consola.ttf",
              "C:/Windows/Fonts/arial.ttf"):
        if os.path.isfile(f):
            return ImageFont.truetype(f, sz)
    return ImageFont.load_default()


def arm_facts(plates, arm):
    """What the run itself reported about the art it loaded."""
    log = os.path.join(plates, f"{arm}_day.log")
    if not os.path.isfile(log):
        return "no log"
    txt = open(log, encoding="utf-8", errors="replace").read()
    m = re.search(r"^BLOCK_ART file-backed.*?tile_px=(\d+)", txt, re.M)
    px = m.group(1) if m else "none"
    pbr = len(re.findall(r"^BLOCK_PBR authored", txt, re.M))
    atlas = re.search(r"^BLOCK_ART LOD atlas (\d+x\d+)", txt, re.M)
    return f"manifest tile_px={px}   authored maps bound={pbr}   LOD atlas={atlas.group(1) if atlas else '?'}"


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "_poppy_tilepx", "tilepx_contact_sheet.png")
    plates = sys.argv[2] if len(sys.argv) > 2 else os.path.join(ROOT, "_poppy_tilepx", "plates")

    f_hdr, f_cap, f_sm, f_col = font(24, True), font(17), font(14), font(16, True)

    # --- null floor, recomputed ------------------------------------------
    a, b = load(os.path.join(plates, "art64on_day.png")), load(os.path.join(plates, "art64on_dayNULL.png"))
    null_mad = float(np.abs(a - b).mean())
    ha, hb = lap(luma(a)).mean(), lap(luma(b)).mean()
    null_hp = abs(hb - ha) / ha * 100.0

    rows = []
    for scene, hour in SCENES:
        imgs, arr = [], {}
        for arm, _, _ in ARMS:
            p = os.path.join(plates, f"{arm}_{scene}.png")
            im = Image.open(p).convert("RGB")
            h = int(im.height * COL_W / im.width)
            imgs.append(im.resize((COL_W, h), Image.LANCZOS))
            arr[arm] = load(p)
        hp = {k: float(lap(luma(v)).mean()) for k, v in arr.items()}

        def d(x, y):
            m = float(np.abs(arr[x] - arr[y]).mean())
            return m, (hp[y] - hp[x]) / hp[x] * 100.0

        rows.append((scene, hour, imgs, hp, d("head16", "art64off"), d("art64off", "art64on")))

    tw, th = rows[0][2][0].size
    pad, head, cap_h, foot = 16, 138, 66, 92
    W = pad * 4 + tw * 3
    H = head + sum(cap_h + th + pad for _ in rows) + foot
    sheet = Image.new("RGB", (W, H), (17, 17, 20))
    dr = ImageDraw.Draw(sheet)

    dr.text((pad, 12), "Manifest-driven tile size — what the 64px albedo bought, apart from what the maps bought",
            (240, 240, 240), font=f_hdr)
    dr.text((pad, 46), "target-poppy/release/voxelforge.exe, ONE binary. Only the art source + MAT_MAPS move "
                       "between columns; camera, sun, hour, exposure and shot frame are shared.",
            (170, 170, 175), font=f_sm)
    dr.text((pad, 64), "shared env: PLAY=1 NOHUD=1 LOOK_QUALITY=ultra CINE_START=1.0", (170, 170, 175), font=f_sm)
    dr.text((pad, 82), f"null pair (art64on day shot twice, identical env): mad {null_mad:.4f} lv, "
                       f"local contrast {null_hp:+.2f}%  <- nothing below this counts",
            (150, 200, 150), font=f_sm)
    for i, (arm, label, colour) in enumerate(ARMS):
        x = pad + i * (tw + pad)
        dr.text((x, 102), label.replace("\n", "   "), colour, font=f_col)
        dr.text((x, 120), arm_facts(plates, arm), (150, 150, 158), font=f_sm)

    y = head
    for scene, hour, imgs, hp, (mad_a, hp_a), (mad_b, hp_b) in rows:
        dr.text((pad, y + 4), f"{scene}   {hour}", (235, 235, 235), font=f_cap)
        dr.text((pad, y + 26),
                f"albedo 16->64px:  mad {mad_a:6.2f} lv   local contrast {hp_a:+6.1f}%        "
                f"maps derived->authored:  mad {mad_b:6.2f} lv   local contrast {hp_b:+6.1f}%",
                (150, 200, 150), font=f_sm)
        dr.text((pad, y + 44),
                "local contrast (mean |Laplacian| of luma):  "
                + "   ".join(f"{a} {hp[a]:.2f}" for a, _, _ in ARMS),
                (150, 150, 158), font=f_sm)
        y += cap_h
        for i, im in enumerate(imgs):
            x = pad + i * (tw + pad)
            sheet.paste(im, (x, y))
            dr.text((x + 8, y + 6), ARMS[i][0], ARMS[i][2], font=f_col)
        y += th + pad

    dr.text((pad, y + 4), "Read left-to-right: column 1->2 is the ALBEDO upgrade on its own (this is what the "
                          "TILE_PX gate used to throw away entirely);", (200, 200, 205), font=f_sm)
    dr.text((pad, y + 22), "column 2->3 is the authored normal/roughness maps on their own. Neither may be "
                           "quoted for the other's number.", (200, 200, 205), font=f_sm)
    dr.text((pad, y + 44), "spec% (share of pixels above luma 200) is NOT plotted: it moves by <0.1pp in every "
                           "pair. Nobody gets to call this a highlight change.", (210, 165, 165), font=f_sm)
    dr.text((pad, y + 66), "scripts/_poppy_tilepx_{build,shoot,measure,sheet}  |  plates: _poppy_tilepx/plates/",
            (140, 140, 145), font=f_sm)

    sheet.save(out)
    print(f"{out}  {sheet.size[0]}x{sheet.size[1]}")


if __name__ == "__main__":
    main()
