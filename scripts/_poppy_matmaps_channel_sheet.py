#!/usr/bin/env python3
# ===========================================================================
# Poppy — WHICH map ate the value span: `_n` alone vs `_r` alone, on one row.
#
# M4 on atlas16/night-firelit failed at lum_spread -2.570, with the normals and
# the roughness both live. That number convicts a PAIR of files. Here each one
# runs alone out of a split atlas dir (scripts/_poppy_matmaps_split.py: same
# albedo, same manifest, only which map the loader can find differs), against
# ITS OWN `off` plate and ITS OWN null pair. One lever per column, one floor per
# column — no borrowed anything.
#
# The numbers are NOT re-derived here. They come from
# scripts/_fl_matmaps_judge.py's own `measure()`, imported and called — one
# definition of lum_spread in this repo, not a second one on a contact sheet
# that could quietly disagree with the gate.
#
# The three `off` plates are also cross-checked against each other, because the
# whole row rests on them being the same picture: MAT_MAPS=off reads no authored
# map at all, so the atlas dir should not matter, and if it did this comparison
# would be measuring the dirs instead of the maps.
#
# USAGE
#   python scripts/_poppy_matmaps_channel_sheet.py [out.png]
# ===========================================================================
import importlib.util
import os
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PUB = os.path.join(ROOT, "docs", "assets", "look")
SCENE = "night-firelit"

spec = importlib.util.spec_from_file_location(
    "_fl_matmaps_judge", os.path.join(ROOT, "scripts", "_fl_matmaps_judge.py")
)
JUDGE = importlib.util.module_from_spec(spec)
spec.loader.exec_module(JUDGE)

# label, arm prefix ("" = the full atlas16 set), sub-caption
PANELS = [
    ("off — derived maps only", None, "MAT_MAPS=off; neither authored map is read"),
    ("_n only — authored normals", "nonly_", "atlas16_nonly: no *_r, so roughness stays derived"),
    ("_r only — authored roughness", "ronly_", "atlas16_ronly: no *_n, so normals stay derived"),
    ("both — the shipped drop", "", "atlas16: 72 maps bound, the pair the verdict judged"),
]
AXES = [
    ("lum_spread", "%+.3f", "M4 value span"),
    ("micro_rms", "%+.3f", "M3 local contrast"),
    ("spec_cov", "%+.3f", "A6 highlight cover"),
    ("chroma_std", "%+.5f", "M2 albedo held"),
]
COL_W = 470


def font(sz):
    for f in ("C:/Windows/Fonts/consola.ttf", "C:/Windows/Fonts/arial.ttf"):
        if os.path.isfile(f):
            return ImageFont.truetype(f, sz)
    return ImageFont.load_default()


def plate(tag):
    return os.path.join(PUB, f"matmaps_{SCENE}_{tag}.png")


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(PUB, f"matmaps-channels-{SCENE}.png")

    need = [plate("off")]
    for _, arm, _ in PANELS[1:]:
        need += [plate(f"{arm}on"), plate(f"{arm}off"), plate(f"{arm}onNULLA"), plate(f"{arm}onNULLB")]
    missing = [p for p in need if not os.path.isfile(p)]
    if missing:
        raise SystemExit("REFUSED  missing plates:\n  " + "\n  ".join(missing))

    print("measuring with _fl_matmaps_judge.measure() ...")
    cache = {}

    def M(tag):
        if tag not in cache:
            cache[tag] = JUDGE.measure(plate(tag))
        return cache[tag]

    # Each arm reads against its OWN off and its OWN null pair.
    stats = {}
    for label, arm, _ in PANELS[1:]:
        a, b = M(f"{arm}onNULLA"), M(f"{arm}onNULLB")
        stats[arm] = {
            "on": M(f"{arm}on"),
            "off": M(f"{arm}off"),
            "floor": {k: abs(a[k] - b[k]) for k, _, _ in AXES},
        }

    # Control: the three off plates must be the same picture.
    print("\n--- control: MAT_MAPS=off should not care which atlas dir ---")
    ref = M("off")
    for label, arm, _ in PANELS[1:]:
        if arm == "":
            continue
        d = {k: stats[arm]["off"][k] - ref[k] for k, _, _ in AXES}
        fl = stats[arm]["floor"]
        worst = max(abs(d[k]) / fl[k] if fl[k] else float("inf") for k, _, _ in AXES)
        print(f"  {arm or 'atlas16':10s} off vs atlas16 off: "
              + "  ".join(f"{k} {d[k]:+.4f}" for k, _, _ in AXES)
              + f"   worst {worst:.1f}x that arm's floor")

    thumbs = []
    for _, arm, _ in PANELS:
        tag = "off" if arm is None else f"{arm}on"
        im = Image.open(plate(tag)).convert("RGB")
        thumbs.append(im.resize((COL_W, int(im.height * COL_W / im.width)), Image.LANCZOS))

    tw, th = thumbs[0].size
    pad, head, cap_h = 12, 104, 132
    W = pad * (len(PANELS) + 1) + tw * len(PANELS)
    H = head + th + cap_h + 26
    sheet = Image.new("RGB", (W, H), (17, 17, 20))
    dr = ImageDraw.Draw(sheet)
    f_hdr, f_cap, f_sm = font(22), font(16), font(13)

    dr.text((pad, 12), f"MAT_MAPS on {SCENE}: one authored map at a time — which one costs the value span?",
            (240, 240, 240), font=f_hdr)
    dr.text((pad, 42), "target/release/voxelforge.exe  built 2026-08-18 08:01:34  sha256 C46A7015…  "
                       "— the binary that shot the on/off plates, because a floor belongs to one build",
            (170, 170, 175), font=f_sm)
    dr.text((pad, 60), "shared env: PLAY=1 NOHUD=1 LOOK_QUALITY=ultra CINE_START=1.0 LOOK_NIGHT=1  "
                       "CINE=36,3.6,34, 36,3.6,34, 32.5,1.8,29.5, 1", (170, 170, 175), font=f_sm)
    dr.text((pad, 78), "numbers from scripts/_fl_matmaps_judge.py measure().  d = vs THIS arm's own off plate; "
                       "×floor = vs THIS arm's own null pair (onNULLA vs onNULLB).",
            (170, 170, 175), font=f_sm)

    for i, ((label, arm, how), im) in enumerate(zip(PANELS, thumbs)):
        x = pad + i * (tw + pad)
        sheet.paste(im, (x, head))
        y = head + th + 8
        dr.text((x, y), label, (240, 240, 240), font=f_cap)
        dr.text((x, y + 20), how, (150, 150, 158), font=f_sm)
        y += 42
        for k, fmt, axis in AXES:
            if arm is None:
                dr.text((x, y), f"{axis:18s} {ref[k]:9.4f}", (200, 200, 206), font=f_sm)
            else:
                s = stats[arm]
                d = s["on"][k] - s["off"][k]
                fl = s["floor"][k]
                over = abs(d) / fl if fl else float("inf")
                col = (200, 200, 206)
                if k == "lum_spread":
                    col = (235, 120, 110) if d < 0 and over > 3 else (140, 210, 150)
                dr.text((x, y), f"{axis:18s} {fmt % d}  {over:6.1f}x floor", col, font=f_sm)
            y += 19
        if arm is not None:
            fl = stats[arm]["floor"]["lum_spread"]
            share = 100.0 * (stats[arm]["on"]["lum_spread"] - stats[arm]["off"]["lum_spread"])
            dr.text((x, y + 4), f"(own floor lum_spread {fl:.4f})", (130, 130, 138), font=f_sm)

    sheet.save(out)
    print(f"\nwrote {out}  {sheet.size[0]}x{sheet.size[1]}")

    print("\n--- M4 lum_spread: who ate the span ---")
    both = stats[""]["on"]["lum_spread"] - stats[""]["off"]["lum_spread"]
    for label, arm, _ in PANELS[1:]:
        s = stats[arm]
        d = s["on"]["lum_spread"] - s["off"]["lum_spread"]
        print(f"  {label:30s} d {d:+.4f}   {abs(d)/s['floor']['lum_spread']:7.1f}x its own floor"
              f"   = {100*d/both:5.1f}% of the both-on loss")


if __name__ == "__main__":
    main()
