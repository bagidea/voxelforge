#!/usr/bin/env python3
"""Cut + de-HUD the raw gameplay capture into the cinematic MP4.

Every pixel here comes out of _poppy_cine.exe via gdigrab. Nothing is
re-rendered, re-lit or painted in; the only edits are (1) a cinematic crop,
(2) two cuts, (3) inpainting the centre crosshair, (4) fades.

WHY A CROP AND NOT scripts/_flamingo_dehud2.py
    dehud2 is a *grading* tool: it pastes a flat grey square over the crosshair
    and row-median-fills the prompt band. On a still that is a conservative way
    to stop the [E] glyphs from owning the p95; on 1285 moving frames it is a
    grey box welded to the middle of the screen and a band that boils. So the
    HUD is removed the way a film would remove it -- by framing.

    The crop below (y 52..554, 2.55:1) is derived from the actual layout
    constants, not from taste:
      main.rs   HudText        top 8,  font 16  -> two lines, ends y~46
      hud.rs    quest banner   top 16, font ~16 -> ends y~44
      quest.rs  ObjectiveTracker top 16 right 16
      quest.rs  prompts        bottom 60 / 100 / 140 -> highest glyph top y~560
      hud.rs    HP/ST bars     bottom 28/46, left 28
    Everything above lives outside [52, 554). What does NOT is the pair that is
    centred by construction -- main.rs's "+" crosshair and hud.rs's 40x40 lock
    reticle -- so those are masked and inpainted.

WHY THE DIALOGUE FRAMES ARE CUT
    dialogue_ui.rs draws an opaque 70%-width plaque anchored to the bottom; at
    its tallest it reaches y=488, well inside any watchable frame. There is no
    honest way to paint that out, so the 79 frames it is live for are cut and
    the seam is crossfaded. _poppy_cine_probe.py is what found the range.

Usage:
  python scripts/_poppy_cine_render.py _poppy_cine/raw_<ts>.mkv docs/assets/cinematic/out.mp4
"""
import os
import subprocess
import sys

import cv2
import numpy as np

FPS = 60
CROP_Y0, CROP_Y1 = 52, 554          # 502 rows -> 1280x502, 2.55:1

# Cut list in RAW capture frame numbers (see module docstring + the probe).
SEGMENTS = [(8, 682), (761, 1381)]  # [start, end)
XFADE = 8                           # frames of dissolve at the seam
FADE_IN = 24
FADE_OUT = 40

CX, CY = 640, 360                   # viewport centre = both centred HUD widgets

# main.rs crosshair: Text("+"), font 22, margin (-6, -13) off the 50%/50% node.
# Measured on the capture at 8x: glyph occupies x 636..645, y 355..365.
CROSSHAIR_BOX = (633, 352, 648, 368)  # x0, y0, x1, y1 (exclusive), 3px pad

# hud.rs spawn_lock_reticle: 40x40 centred box, four L brackets in #F4B860,
# ticks 12x2 and 2x12. Only drawn while LockOn.target is Some.
_R = 20
RETICLE_TICKS = []
for _sx in (0, 1):
    for _sy in (0, 1):
        _x = CX - _R + (28 if _sx else 0)
        _y = CY - _R + (38 if _sy else 0)
        RETICLE_TICKS.append((_x, _y, _x + 12, _y + 2))            # horizontal
        _vx = CX - _R + (38 if _sx else 0)
        _vy = CY - _R + (28 if _sy else 0)
        RETICLE_TICKS.append((_vx, _vy, _vx + 2, _vy + 12))        # vertical


def amber_mask(bgr):
    b = bgr[..., 0].astype(np.int16)
    g = bgr[..., 1].astype(np.int16)
    r = bgr[..., 2].astype(np.int16)
    return (r > 170) & (g > 120) & (g < 215) & (b < 140) & (r - b > 60)


def static_mask():
    m = np.zeros((CROP_Y1 - CROP_Y0, 1280), np.uint8)
    x0, y0, x1, y1 = CROSSHAIR_BOX
    m[y0 - CROP_Y0:y1 - CROP_Y0, x0:x1] = 255
    return m


def reticle_mask_if_live(frame):
    """Reticle brackets, but only on frames where they are actually drawn."""
    lit = 0
    for x0, y0, x1, y1 in RETICLE_TICKS:
        if amber_mask(frame[y0:y1, x0:x1]).mean() > 0.6:
            lit += 1
    if lit < 6:                      # 8 ticks; allow two to sit on amber scenery
        return None
    m = np.zeros((CROP_Y1 - CROP_Y0, 1280), np.uint8)
    for x0, y0, x1, y1 in RETICLE_TICKS:
        m[y0 - CROP_Y0 - 2:y1 - CROP_Y0 + 2, x0 - 2:x1 + 2] = 255
    return m


class Stats:
    decoded = 0
    reticle = 0


def timeline(path):
    """Yield finished output frames in order, one sequential decode, holding at
    most XFADE frames in RAM. (Buffering the whole 21s at 1280x502 BGR would be
    ~2.5 GB, which is not a thing to do to this box while Flamingo is building.)"""
    cap = cv2.VideoCapture(path)
    base = static_mask()
    cursor = [0]

    def take(a, b):
        while cursor[0] < b:
            ok, fr = cap.read()
            if not ok:
                return
            i, cursor[0] = cursor[0], cursor[0] + 1
            Stats.decoded += 1
            if i < a:
                continue
            rm = reticle_mask_if_live(fr)
            mask = base
            if rm is not None:
                Stats.reticle += 1
                mask = cv2.max(base, rm)
            yield cv2.inpaint(fr[CROP_Y0:CROP_Y1], mask, 4, cv2.INPAINT_TELEA)

    prev_tail = []
    last = len(SEGMENTS) - 1
    for si, (a, b) in enumerate(SEGMENTS):
        n = b - a
        tail_from = n - XFADE if si < last else n
        tail = []
        for c, fr in enumerate(take(a, b)):
            if si > 0 and c < XFADE:
                w = (c + 1) / (XFADE + 1)
                yield cv2.addWeighted(prev_tail[c], 1 - w, fr, w, 0)
            elif c >= tail_from:
                tail.append(fr)
            else:
                yield fr
        prev_tail = tail
    cap.release()


def render(src, dst):
    total = sum(b - a for a, b in SEGMENTS) - XFADE * (len(SEGMENTS) - 1)
    h, w = CROP_Y1 - CROP_Y0, 1280
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    cmd = [
        "ffmpeg", "-hide_banner", "-loglevel", "error",
        "-f", "rawvideo", "-pix_fmt", "bgr24", "-s", f"{w}x{h}", "-r", str(FPS),
        "-i", "-", "-an",
        "-c:v", "libx264", "-preset", "slow", "-crf", "18",
        "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-r", str(FPS),
        "-y", dst,
    ]
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE)
    written = 0
    for k, fr in enumerate(timeline(src)):
        if k < FADE_IN:
            fr = (fr * (k / FADE_IN)).astype(np.uint8)
        elif k >= total - FADE_OUT:
            fr = (fr * ((total - 1 - k) / FADE_OUT)).astype(np.uint8)
        p.stdin.write(fr.tobytes())
        written += 1
    p.stdin.close()
    rc = p.wait()
    print(f"decoded={Stats.decoded} reticle_frames={Stats.reticle}")
    print(f"expected={total} written={written} size={w}x{h} fps={FPS}")
    print(f"ffmpeg exit={rc} -> {dst}")
    return 0 if (rc == 0 and written == total) else 1


if __name__ == "__main__":
    sys.exit(render(sys.argv[1], sys.argv[2]))
