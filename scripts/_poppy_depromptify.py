"""Erase the residual "[E] ..." quest prompt from a NOHUD capture frame.

Why this exists at all: `VOXELFORGE_NOHUD=1` blanks every HUD widget that is
spawned once (FPS readout, crosshair, objective tracker, HP/stamina bars) — those
are gone from these frames at the source. It does NOT win against the quest
prompts, because `quest.rs` despawns and re-spawns them EVERY frame and the only
NOHUD-capable binary on disk (08:45) still runs `nohud_sweep` before that respawn
has flushed. The spawn-site fix landed later (commit 31f1d71) and the build lane
is held, so on this binary the prompt is in the frame no matter the camera.

The avatar also cannot be walked out of `CAMPFIRE_REST_RANGE` (4.0) before the
grab: `VOXELFORGE_SHOT` fires at a hardcoded 3.2 s and spawn sits 3.0 blocks from
the fire, so the prompt is live in every reachable frame.

So the text is removed in post. Scope is deliberately narrow — it repaints ONLY
pixels that are part of a detected glyph row, never a rectangle of scenery.
"""
import sys
import cv2
import numpy as np


def prompt_mask(bgr):
    """Mask of the prompt glyphs, or None if the frame carries no prompt row.

    Glyphs are thin bright strokes composited on top of the render, so they sit
    far above their own local background regardless of how hazed the scene is —
    absolute-brightness thresholds miss a prompt drawn over pale limestone. What
    makes them unmistakable is BASELINE ALIGNMENT: many same-height blobs whose
    tops share one scanline across hundreds of px. Voxel texture never does that.
    """
    h, w = bgr.shape[:2]
    y0, y1 = int(h * 0.83), int(h * 0.99)
    x0, x1 = int(w * 0.20), int(w * 0.80)
    band = cv2.cvtColor(bgr[y0:y1, x0:x1], cv2.COLOR_BGR2GRAY).astype(np.int16)

    opened = cv2.morphologyEx(band.astype(np.uint8), cv2.MORPH_OPEN,
                              cv2.getStructuringElement(cv2.MORPH_RECT, (9, 9)))
    strokes = ((band - opened) > 16).astype(np.uint8)

    n, lab, stats, _ = cv2.connectedComponentsWithStats(strokes, 8)
    rows = {}
    for i in range(1, n):
        x, y, bw, bh, area = stats[i]
        if 4 <= bh <= 26 and 1 <= bw <= 30 and area >= 3:
            rows.setdefault(y // 4, []).append((i, x))

    mask = np.zeros((h, w), np.uint8)
    found = False
    for _, comps in rows.items():
        xs = [x for _, x in comps]
        if len(comps) < 8 or (max(xs) - min(xs)) < int(w * 0.05):
            continue  # not a text line — a few aligned block edges
        found = True
        for i, _ in comps:
            mask[y0:y1, x0:x1][lab == i] = 255
    if not found:
        return None
    # Grow past the glyph's antialiased fringe so no halo survives the repaint.
    return cv2.dilate(mask, np.ones((7, 7), np.uint8), iterations=1)


def clean(src, dst):
    bgr = cv2.imread(src, cv2.IMREAD_COLOR)
    if bgr is None:
        raise SystemExit(f"cannot read {src}")
    m = prompt_mask(bgr)
    if m is None:
        cv2.imwrite(dst, bgr)
        return 0
    out = cv2.inpaint(bgr, m, 4, cv2.INPAINT_TELEA)
    cv2.imwrite(dst, out)
    return int((m > 0).sum())


if __name__ == "__main__":
    for src, dst in zip(sys.argv[1::2], sys.argv[2::2]):
        px = clean(src, dst)
        print(f"{src} -> {dst}  repainted {px} px"
              f"{' (no prompt found)' if px == 0 else ''}")
