#!/usr/bin/env python3
"""Contact sheet of the hero-rig film's named beats, BEFORE over AFTER.

A video is the deliverable, but a video does not paste into a doc or a review
thread, and half the places this gets read will not autoplay one. This lays the
same beats out as stills so the pair can be argued about in text.

    python scripts/_poppy_herofilm_sheet.py _poppy_hero

Beat times are in SIM seconds and are converted to a frame index with the SAME
two constants the engine filmed with (`FEEL_FILM_FROM`, `FEEL_FILM_DT` in
main.rs). They are checked against the frames actually on disk rather than
assumed: a sheet whose captions have drifted off its own pixels is worse than no
sheet.

The beats themselves are not invented for this sheet either -- with one addition
they ARE `FEEL_DEMO_SHOT_BEATS`, the table main.rs already samples its nine
stills at, so the columns here and the stills lane name the same instants.
"""
import sys
import pathlib
from PIL import Image, ImageDraw, ImageFont

# --- must match client/src/main.rs -----------------------------------------
FILM_FROM = 0.30    # FEEL_FILM_FROM
DT = 1.0 / 30.0     # FEEL_FILM_DT

BEATS = [
    (0.40, "idle"),           # before the stick engages at t=0.5
    (0.90, "walk_a"),         # the nine below are FEEL_DEMO_SHOT_BEATS verbatim
    (1.40, "walk_b"),
    (1.90, "walk_c"),
    (2.30, "walk_d"),
    (3.20, "run"),
    (4.05, "jump_rise"),
    (4.30, "jump_apex"),
    (4.50, "jump_fall"),
    (5.20, "walk_resumed"),
]

THUMB_W = 320
PAD = 8
CAPTION_H = 22
ROW_LABEL_W = 92


def frame_index(t):
    return int(round((t - FILM_FROM) / DT))


def font(size):
    for name in ("DejaVuSans.ttf", "arial.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def load_row(d, idxs):
    out = []
    for i in idxs:
        p = d / f"f{i:05d}.png"
        out.append(Image.open(p).convert("RGB") if p.exists() else None)
    return out


def main():
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "_poppy_hero")
    after_d, before_d = out / "film_after", out / "film_before"
    if not after_d.is_dir():
        print(f"SHEET FAIL: no {after_d}")
        return 2

    idxs = [frame_index(t) for t, _ in BEATS]
    have = sorted(int(p.stem[1:]) for p in after_d.glob("f*.png"))
    if not have:
        print(f"SHEET FAIL: {after_d} has no frames")
        return 2
    missing = [i for i in idxs if i not in have]
    if missing:
        print(f"SHEET FAIL: film is f{have[0]:05d}..f{have[-1]:05d}, beats want {missing} -- "
              "the timeline constants and the frames on disk disagree")
        return 3

    rows = [("AFTER\n(rig)", load_row(after_d, idxs))]
    if before_d.is_dir() and any(before_d.glob("f*.png")):
        rows.insert(0, ("BEFORE\n(capsule)", load_row(before_d, idxs)))

    src = next(im for _, r in rows for im in r if im)
    tw = THUMB_W
    th = int(round(src.height * tw / src.width))

    W = ROW_LABEL_W + len(idxs) * (tw + PAD) + PAD
    H = CAPTION_H + len(rows) * (th + CAPTION_H + PAD) + PAD
    sheet = Image.new("RGB", (W, H), (18, 18, 20))
    dr = ImageDraw.Draw(sheet)
    f_cap, f_row = font(14), font(13)

    for c, (t, label) in enumerate(BEATS):
        x = ROW_LABEL_W + c * (tw + PAD)
        dr.text((x + 2, 4), f"{label}  t={t:.2f}s  f{idxs[c]:05d}",
                font=f_cap, fill=(235, 235, 225))

    y = CAPTION_H
    for name, imgs in rows:
        for ln, line in enumerate(name.split("\n")):
            dr.text((6, y + 6 + ln * 16), line, font=f_row, fill=(255, 210, 130))
        for c, im in enumerate(imgs):
            x = ROW_LABEL_W + c * (tw + PAD)
            if im is None:
                dr.rectangle([x, y, x + tw, y + th], outline=(120, 40, 40))
                dr.text((x + 8, y + th // 2), "missing", font=f_cap, fill=(200, 90, 90))
            else:
                sheet.paste(im.resize((tw, th), Image.LANCZOS), (x, y))
        y += th + CAPTION_H + PAD

    dst = out / "hero-rig-beats.png"
    sheet.save(dst)
    print(f"SHEET OK {dst} {sheet.size[0]}x{sheet.size[1]} rows={len(rows)} beats={len(idxs)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
