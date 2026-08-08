#!/usr/bin/env python3
"""Strip the play-mode HUD out of a showcase frame — presentation-grade dehud.

Same idea as Flamingo's `_flamingo_dehud2.py` in `_flamingo_g7` (crop the top band,
detect the UI and paint it out), rebuilt for frames a person will look at instead of
frames a luminance gate will read. Three things had to change:

  * **Crosshair geometry.** `_flamingo_dehud2.py` crops the top band FIRST and then
    patches at `h // 2` of the *cropped* image. The crosshair sits at 50% of the
    **window** (`main.rs:912` — `top: Val::Percent(50.0)`), i.e. y=360 of 720, which
    after an 80 px crop is y=280, not 320. That patch lands 40 px low: it misses the
    crosshair and paints a square on clean ground. Here everything is located in
    ORIGINAL window coordinates and the crop happens last.
  * **Bright ≠ UI.** A "far brighter than the row median" test eats the campfire —
    on a real play frame it deleted the flame and left the fire's glow behind. The
    crosshair is white and *desaturated*; the fire is not. So the glyph test is
    brightness AND low saturation.
  * **The prompt is a plaque, not floating glyphs.** `quest.rs:1155` draws an 88%
    opaque panel (`PROMPT_PANEL_BG` 0x1A120D) behind "[E] Rest at campfire". No
    per-pixel brightness rule removes an opaque dark box; it is found by its own
    flat colour and inpainted (Telea) from the scene around it.

HUD inventory (all from the play-mode spawn sites), and how each one goes:
  main.rs:857   HudText            top 8 px,  16 px  ─┐
  combat.rs     HP/ST bars + text  top 30–62 px       ├─ inside the 80 px top crop
  main.rs:894   ObjectiveTracker   top 40 px, 14 px  ─┘
  main.rs:903   crosshair "+"      50% / 50%          ── masked + inpainted
  quest.rs:1143 "[E] …" plaque     bottom 100 px      ── masked + inpainted

Usage:
  python scripts/_poppy_showcase_dehud.py <frame.png> ...   ->  <frame>-nohud.png
"""
import sys

import cv2
import numpy as np
from PIL import Image

HUD_BAND = 80        # px cropped off the top: FPS line, HP/ST bars, objective tracker
CROSS_R = 16         # px half-window searched for the crosshair glyph, around centre
CROSS_OVER = 26      # L above the local median to be a candidate crosshair pixel
CROSS_SAT = 0.22     # ... and no more saturated than this (the fire is far above it)
PLAQUE_BAND = (0.74, 0.92)  # fraction of WINDOW height the prompt plaque lives in
PLAQUE_BG = np.array([0x1A, 0x12, 0x0D], dtype=np.float64)  # quest.rs:390
PLAQUE_TOL = 34      # max per-channel distance from PLAQUE_BG to count as panel
PLAQUE_W = (90, 560)  # px: plausible plaque width  ("[E] Rest at campfire" ≈ 180)
PLAQUE_H = (18, 70)   # px: plausible plaque height (16 px text + 6 px padding + border)
PLAQUE_FILL = 0.55    # the blob must actually FILL its bounding box — a panel does,
                      # scattered dark scenery pixels do not
# The prompts that are still bare text, not plaques: "[E] Talk to …" (quest.rs:641)
# and "[E] Read …" (quest.rs:1180). They are drawn in exactly these colours, so match
# on colour instead of on brightness — an amber "[E]" is more saturated than a
# campfire and no brightness rule separates the two.
TEXT_COLORS = np.array([
    [0xE8, 0xD8, 0xB8],   # PROMPT_TEXT_CREAM  (quest.rs:393)
    [0xF4, 0xB8, 0x60],   # PROMPT_ACCENT_AMBER(quest.rs:392)
    [0xFF, 0xFF, 0xFF],   # plain white prompt text
], dtype=np.float64)
TEXT_TOL = 26        # max per-channel distance from a UI text colour
TEXT_OVER = 18       # ... and it still has to out-shine its own row's background
TEXT_PALE_OVER = 30  # pale-glyph rule: L above the row background. 40 left a visible
                     # speckle trail on grass where the alpha-blended glyph edges of
                     # "[E] Read Scorched handprint" fell under the threshold.
TEXT_PALE_SAT = 0.30 # ... at no more than this saturation (keeps the fire out)
INPAINT_R = 4        # Telea radius


def _lum(a):
    return 0.2126 * a[:, :, 0] + 0.7152 * a[:, :, 1] + 0.0722 * a[:, :, 2]


def _sat(a):
    mx = a.max(axis=2)
    mn = a.min(axis=2)
    return np.divide(mx - mn, np.maximum(mx, 1e-6))


def _dilate(mask, r):
    k = np.ones((2 * r + 1, 2 * r + 1), np.uint8)
    return cv2.dilate(mask.astype(np.uint8), k).astype(bool)


def crosshair_mask(a):
    """White, desaturated glyph pixels in a small box at the window centre."""
    h, w = a.shape[:2]
    cy, cx = h // 2, w // 2
    box = a[cy - CROSS_R:cy + CROSS_R, cx - CROSS_R:cx + CROSS_R]
    lum, sat = _lum(box), _sat(box)
    hit = (lum > np.median(lum) + CROSS_OVER) & (sat < CROSS_SAT)

    m = np.zeros((h, w), bool)
    m[cy - CROSS_R:cy + CROSS_R, cx - CROSS_R:cx + CROSS_R] = hit
    return _dilate(m, 2)


def plaque_mask(a):
    """The interaction-prompt panel: a flat `PROMPT_PANEL_BG` box in the lower band.

    Found by colour rather than by geometry so a frame with no prompt (player out of
    range) is left completely untouched instead of being inpainted anyway.

    A bare colour match is not enough: dark voxel scenery hits the same tolerance, and
    taking the bounding box of every match masked most of the band on the first try.
    The panel is one solid connected rectangle, so the blob is required to be
    connected, plaque-sized, and to actually fill its own bounding box.
    """
    h, w = a.shape[:2]
    y0, y1 = int(h * PLAQUE_BAND[0]), int(h * PLAQUE_BAND[1])
    band = a[y0:y1]
    near = (np.abs(band - PLAQUE_BG).max(axis=2) < PLAQUE_TOL).astype(np.uint8)

    m = np.zeros((h, w), bool)
    n, _, stats, _ = cv2.connectedComponentsWithStats(near, connectivity=8)
    for i in range(1, n):
        x, y, bw, bh, area = stats[i]
        if not (PLAQUE_W[0] <= bw <= PLAQUE_W[1] and PLAQUE_H[0] <= bh <= PLAQUE_H[1]):
            continue
        if area < PLAQUE_FILL * bw * bh:
            continue
        # Mask the whole bounding box — the amber "[E]", the cream label and the
        # 1 px border all sit inside it, and a filled box is what Telea rebuilds
        # most cleanly.
        m[y0 + y:y0 + y + bh, x:x + bw] = True

    if not m.any():
        return m, 0
    return _dilate(m, 3), int(m.sum())


def prompt_text_mask(a):
    """Bare-text prompts in the lower band, matched on their literal UI colours."""
    h, w = a.shape[:2]
    y0, y1 = int(h * PLAQUE_BAND[0]), int(h * PLAQUE_BAND[1])
    band = a[y0:y1]

    lum, sat = _lum(band), _sat(band)
    row_med = np.median(lum, axis=1, keepdims=True)

    # White / cream glyphs: bright AND desaturated. Alpha-blended text never lands
    # exactly on its source colour, so brightness+neutrality catches it where an
    # exact colour match does not.
    pale = (lum > row_med + TEXT_PALE_OVER) & (sat < TEXT_PALE_SAT)
    # The amber "[E]" is too saturated for that rule — it gets a colour match.
    tinted = np.zeros(band.shape[:2], bool)
    for c in TEXT_COLORS:
        tinted |= np.abs(band - c).max(axis=2) < TEXT_TOL
    hit = pale | (tinted & (lum > row_med + TEXT_OVER))

    m = np.zeros((h, w), bool)
    m[y0:y1] = hit
    # r=4, not 3: Telea needs a margin of clean pixels around a glyph to rebuild from,
    # and at r=3 the faintest antialiased edge pixels sat right on the mask border and
    # were inpainted *with themselves*.
    return _dilate(m, 4), int(hit.sum())


def dehud(path: str) -> str:
    im = Image.open(path).convert("RGB")
    w, h = im.size
    a = np.asarray(im, dtype=np.float64)

    cm = crosshair_mask(a)
    pm, plaque_px = plaque_mask(a)
    tm, text_px = prompt_text_mask(a)
    mask = (cm | pm | tm).astype(np.uint8) * 255

    bgr = cv2.cvtColor(np.asarray(im), cv2.COLOR_RGB2BGR)
    fixed = cv2.inpaint(bgr, mask, INPAINT_R, cv2.INPAINT_TELEA)
    rgb = cv2.cvtColor(fixed, cv2.COLOR_BGR2RGB)

    # Crop last. 1280x720 - 80 => 1280x640, a clean 2:1; nothing else is cropped.
    out_im = Image.fromarray(rgb).crop((0, HUD_BAND, w, h))
    out = path.rsplit(".", 1)[0] + "-nohud.png"
    out_im.save(out)

    # A zero count is a silent failure (glyph never found), so say so out loud.
    warn = "" if cm.sum() else "  WARN: crosshair not detected"
    print(
        f"{out}  {out_im.size}  crosshair_px={int(cm.sum())}  "
        f"plaque_px={plaque_px}  prompt_text_px={text_px}{warn}"
    )
    return out


if __name__ == "__main__":
    for p in sys.argv[1:]:
        dehud(p)
