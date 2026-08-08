#!/usr/bin/env python3
"""Scan the raw capture and report, per frame, which HUD/UI states are on screen.

The cinematic crop (y 52..554) clears every fixed HUD element except the two
that sit dead centre by construction (crosshair, lock reticle) -- those get
inpainted. It does NOT clear the dialogue plaque, which is ~936px wide and
opaque; there is no honest way to paint that out, so those frames are cut.

This probe is what decides the cut points, instead of me eyeballing a contact
sheet: it prints the frame ranges where the plaque / the lock reticle / the
still-loading sky are live.

Usage: python scripts/_poppy_cine_probe.py _poppy_cine/raw_<ts>.mkv
"""
import sys

import cv2
import numpy as np

# Dialogue plaque: amber 1px border around a dark panel, spawned by
# dialogue_ui.rs at 70% viewport width, centred, BOTTOM-anchored -- its height
# grows with the number of choice lines, so its top edge MOVES between frames.
# A fixed probe row found 26 of ~400 plaque frames; the rule has to be searched
# for over the whole lower half instead.
PLAQUE_ROWS = (380, 706)
PLAQUE_X = (260, 1020)

# hud.rs spawn_lock_reticle: 40x40 box centred on the viewport, four L brackets
# in ACCENT_AMBER (#F4B860), tick 12x2 / 2x12.
RETICLE_PROBE = [(622, 341), (655, 341), (622, 378), (655, 378)]


def amber_mask(bgr):
    """#F4B860-ish: hot red channel, mid green, cold blue."""
    b = bgr[..., 0].astype(np.int16)
    g = bgr[..., 1].astype(np.int16)
    r = bgr[..., 2].astype(np.int16)
    return (r > 170) & (g > 120) & (g < 215) & (b < 140) & (r - b > 60)


def main(path):
    cap = cv2.VideoCapture(path)
    n = 0
    plaque, reticle, sky = [], [], []
    while True:
        ok, fr = cap.read()
        if not ok:
            break
        band = amber_mask(fr[PLAQUE_ROWS[0]:PLAQUE_ROWS[1], PLAQUE_X[0]:PLAQUE_X[1]])
        # a plaque frame has one row that is amber essentially all the way across
        if band.mean(axis=1).max() > 0.85:
            plaque.append(n)

        m = amber_mask(fr)
        if sum(int(m[y, x]) for x, y in RETICLE_PROBE) >= 3:
            reticle.append(n)

        if fr.std() < 45 and fr[:, :, 0].mean() > fr[:, :, 2].mean():
            sky.append(n)
        n += 1
    cap.release()

    def runs(xs):
        out = []
        for x in xs:
            if out and x == out[-1][1] + 1:
                out[-1][1] = x
            else:
                out.append([x, x])
        return [tuple(r) for r in out]

    print(f"frames={n}")
    print(f"sky/loading   : {runs(sky)}")
    print(f"dialogue      : {runs(plaque)}")
    print(f"lock reticle  : {runs(reticle)}")


if __name__ == "__main__":
    main(sys.argv[1])
