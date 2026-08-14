#!/usr/bin/env python3
"""Beauty gap sheet v4 (2026-08-15) — Poppy's look pass, graded.

Same job as _flamingo_beauty_sheet.py, wider scope: three times of day, each
with its before/after pair, every callout carrying the number that earned it.

Layout, top to bottom:
  1. one column per scene: the AFTER frame with arrows on the losses, the
     before|after pair underneath so the lever is visible, then a 200% crop of
     whatever the arrows point at, then the rubric line for that scene.
  2. a time-of-day swatch bar - the three scenes' mean colour next to the
     golden ref's, because "noon looks like evening" is a claim about hue that
     a reader should be able to check by eye in one second.
  3. the two approved reference frames' caveat + the modern-shader-mod
     checklist, and the blocker.

Rule: every arrow label quotes a measured value from
scripts/_flamingo_lookv4_axes.py / _claims.py / _contact.py / _shadowmask.py
or from scripts/grade_gate.py. No adjective ships without its number.

Every column is laid out on the SAME fixed grid (same crop aspect, same block
heights) so the three verdict blocks line up and the eye can compare scenes by
scanning one row - a sheet whose rows drift is a sheet nobody reads across.

Usage:
    python scripts/_flamingo_lookv4_sheet.py docs/assets/flamingo-beauty-gap-v4-2026-08-15.png
"""
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _flamingo_lookv4_plates as plates  # noqa: E402

ASSETS = os.path.join('docs', 'assets')

BG = (16, 16, 21)
PANEL = (26, 26, 33)
FG = (232, 232, 240)
DIM = (150, 150, 162)
BAD = (255, 104, 104)
GOOD = (118, 224, 150)
WARN = (255, 196, 96)
EDGE = (70, 70, 84)

FDIR = 'C:/Windows/Fonts/'


def font(name, size):
    return ImageFont.truetype(FDIR + name, size)


F_TITLE = font('segoeuib.ttf', 36)
F_SUB = font('segoeui.ttf', 19)
F_H = font('segoeuib.ttf', 24)
F_L = font('segoeui.ttf', 18)
F_S = font('segoeui.ttf', 16)
F_M = font('consola.ttf', 15)
F_MB = font('consolab.ttf', 16)

CROP_W, CROP_H = 420, 240  # every zoom crop shares this aspect -> aligned rows

SCENES = [
    dict(
        key='outdoor-noon',
        title='1 \u00b7 OUTDOOR NOON',
        cfg='sun 66\u00b0/205\u00b0 \u00b7 20 000 lx \u00b7 PCSS 12 \u00b7 ambient 1150\u2192380 \u00b7 IBL V3 260 nits',
        gate=['G1 P', 'G2 F', 'G3 P', 'G4 F', 'G5 P', 'G6 P'],
        verdict='GATE FAIL  (G2, G4)',
        score=28,
        crop=(380, 400, CROP_W, CROP_H),
        croplabel='200% \u00b7 wall foot on grass \u2014 the ground beside it is the same value as open ground',
        calls=[
            ((600, 592), (30, 632), BAD,
             'no cast shadow lands on the ground', 'every dark pixel = a cube SIDE face'),
            ((1074, 96), (600, 40), BAD,
             'emissive block, zero bloom halo', 'L r0-6 222 \u2192 r10-14 176 \u2192 r28-34 139'),
            ((1215, 26), (940, 152), BAD,
             'sky = black hole, L < 12', '3.31% of the top band'),
            # NOT "the haze eats the distance" - I measured that and it is
            # false: saturation falls 0.77 near -> 0.50 far, which is aerial
            # perspective working. The real loss here is the dead blue channel.
            ((300, 300), (26, 196), WARN,
             'B channel clamped to 0 on 9.43% of px', '(before 7.92%) — no cool light exists'),
        ],
    ),
    dict(
        key='evening-raking',
        title='2 \u00b7 EVENING RAKING',
        cfg='sun 22\u00b0/205\u00b0 \u00b7 22 000 lx \u00b7 PCSS 12 \u00b7 ambient 1150\u2192380 \u00b7 IBL V3 260 nits',
        gate=['G1 P', 'G2 P', 'G3 P', 'G4 F', 'G5 P', 'G6 F'],
        verdict='GATE FAIL  (G4, G6)',
        score=44,
        crop=(180, 280, CROP_W, CROP_H),
        croplabel='200% \u00b7 the soft shadow edge lands (G2 win) \u2014 but the block foot has no AO crease',
        calls=[
            ((1010, 300), (690, 398), GOOD,
             'real cast shadow, soft edge', 'the only scene that takes G2'),
            ((1058, 196), (760, 56), BAD,
             'solid-black blocks floating in sky', '9.02% of the top band < L12'),
            ((494, 690), (26, 626), BAD,
             'G6 FAIL: brightest golden patch', 'L 53.4, the floor is 55'),
            ((250, 400), (26, 286), BAD,
             'block sits on grass with', 'no contact crease'),
        ],
    ),
    dict(
        key='night-firelit',
        title='3 \u00b7 NIGHT FIRELIT',
        cfg='sun \u22128\u00b0 \u00b7 260 lx \u00b7 PCSS 12 \u00b7 ambient 42\u219214 \u00b7 IBL V3 9 nits \u00b7 rim 0\u219230 lx',
        gate=['G1 P', 'G2 P', 'G3 F', 'G4 F', 'G5 P', 'G6 P*'],
        verdict='GATE FAIL  (G3 regression, G4)',
        score=23,
        crop=(60, 250, CROP_W, CROP_H),
        croplabel='200% \u00b7 the hero stands in a warm bounce field and picks up none of it',
        calls=[
            ((205, 400), (26, 566), BAD,
             'hero L 10.5% \u00b7 wall 100 px away L 44%',
             'no bounce reaches him, no shadow under him'),
            ((612, 380), (560, 606), BAD,
             'fire core clips 255, no halo',
             'r18-22 (192) > r10-14 (163) = lit floor'),
            ((672, 122), (700, 40), BAD,
             'G3 REGRESSION  p05-L 10.1% \u2192 7.3%', 'floor is 8 \u2014 caused by ambient 42\u219214'),
            ((1060, 400), (900, 196), WARN,
             'a third of the frame is a', 'featureless dark slab'),
        ],
    ),
]

YARDSTICK = [
    ('Sky: atmosphere + sun/moon disc + horizon ramp', 'flat wash, holes at L<12', BAD),
    ('Sun/moon cast shadow at every time of day', 'evening only (1 of 3)', BAD),
    # The ref runs through the same code but is a different plate scale, so
    # this is a direction, not a ratio to quote on its own.
    ('SSAO / contact crease on every seam', 'seam only -9.6 L (ref -52.4, other plate)', BAD),
    ('Coloured GI \u2014 block light bleeding onto neighbours', 'none; hero takes 0 bounce', BAD),
    ('Bloom halo on emissive blocks', 'none on any of the 3', BAD),
    ('Volumetric light shafts', 'FogVolume spawns, no shaft renders', BAD),
    ('Specular / SSR on water + metal', 'every surface reads matte', BAD),
    ('Time-of-day colour arc (cool noon \u2192 gold dusk)', 'R-B +76 / +65 / +58 = no arc', BAD),
    ('Filmic tone-map, highlights not clipped', 'G5 PASS on all three', GOOD),
    ('Voxel identity survives the shader', 'G1 PASS on all three', GOOD),
]

BLOCKER = """Scene 1 and scene 3 have no cast shadow anywhere on the ground plane. The ground-plane luminance
map (scripts/_flamingo_lookv4_shadowmask.py) shows every dark pixel in scene 1 landing on a cube SIDE
face - there is not one coherent shadow region with an edge to it. That single defect takes G2 and G4
down and costs 28 of the 100 rubric points in scene 1 alone (Pass 1 = 12 and Pass 4 = 16, the two
heaviest lighting passes in the rubric).

It is NOT a missing feature. Same binary, same map, same shadow_map=4096 / PCSS 12 / contact=true -
scene 2 at sun elevation 22 deg casts correct soft shadows onto the same grass. Only the sun elevation
and the camera differ between them. So the hypothesis is cascade coverage or depth bias at a high sun
angle on the wide vista camera, not "we have no shadows".

The shoot that settles it: re-shoot the SCENE 1 camera at sun elevation 22 deg, and the SCENE 2 camera
at 66 deg. If the shadow follows the elevation it is bias/cascade; if it follows the camera it is the
vista frustum. Either answer is a one-line fix; guessing between them is not."""

DRIFT = """SINCE THIS SHEET WAS CUT - the plates are being re-shot on a loop. At least THREE different sets have sat at docs/assets/look/ since b361a9d: 03:38, 03:43, and the one
on disk now. This sheet is pinned to b361a9d and grades none of them. What follows is a STAMPED SNAPSHOT of the 03:50:38-03:51:07 set - AFTER md5 outdoor bb18a84f0b / evening
672bbe78fd / night 64a6cca33d. If your md5s differ you are on a LATER shoot: re-measure, do not reconcile (LOOKV4_LIVE=1 points every script on this sheet at the working tree).
FIXED: night G3 is back over the line, p05-L 7.3% -> 8.1% (floor 8) - the "must fix" to the left is already done, do not re-raise it. UNCHANGED: the blocker - the re-shot noon
ground map still puts every dark pixel on a cube side face. Evening G6 still FAILS, L 53.4 -> 53.7 against a floor of 55. WORSE: the new grade buys warmth by crushing blue -
noon R-B +75.6 -> +87.5 with B clamped to 0 on 18.70% of the frame (was 9.43%), evening 4.77% -> 8.67%. Warmth out of a dead channel cannot be graded back out later."""

NEGATIVE = """WHAT DOES NOT SUPPORT THE BLOCKER - printed because a negative result that gets deleted is a negative result somebody re-derives in a week. _claims.py C2 was written
as the numeric test of exactly the blocker above; its own docstring reads "no split = the sun casts nothing onto the ground". It came back unable to tell the scenes apart. Grass
luminance bimodality (otsu-sep) on the pinned plates: outdoor-noon 0.718 (claimed: NO cast shadow) vs evening-raking 0.730 (claimed: casts correctly) - the two scenes on opposite
sides of the claim are 0.012 apart - and night-firelit, which has no sun at all, scores HIGHEST at 0.983, because fire falloff is bimodal too. So C2 is a broken instrument for
this question rather than evidence either way, and no number on this sheet is sourced from it. The blocker stands on the two things that did measure it: the ground-plane
luminance maps (_shadowmask.py) and the 200% crops in the columns above."""

CAVEAT = """CAVEAT ON THESE SCORES - read this before quoting them anywhere. Every threshold in look-acceptance-rubric.md was calibrated on an INDOOR window-lit kitchen
(golden-beauty-shot-ref.png), and BOTH signed-off references are interiors. Two of the three frames graded here are open air. G2 literally asks for "window mullion bars on the
floor" and G5 for "the window is not blown out"; outdoors I read those as "a readable cast-shadow pattern on the ground" and "sky and emissives are not blown out". That mapping
is my call and it is load-bearing. It is also why a NIGHT frame can PASS G6 ("warm sunlit wood") on a patch that is actually the fire block itself (255,239,214, R-B +41 against a
floor of 40) - scene 3's G6 is marked P* and I do not count it as evidence either way. The thing still missing underneath all of this is an approved OUTDOOR reference frame:
right now open-air gameplay is being graded against a kitchen. That is a 30-minute art call from the boss and it unblocks every outdoor number in this sheet."""


def lum(a):
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def load(scene, variant):
    """Always the PINNED plate - see _flamingo_lookv4_plates on why not the path."""
    return Image.open(plates.path(scene, variant)).convert('RGB')


def arrow(d, p0, p1, col, w=3):
    d.line([p0, p1], fill=col, width=w)
    vx, vy = p1[0] - p0[0], p1[1] - p0[1]
    n = max((vx * vx + vy * vy) ** 0.5, 1e-3)
    vx, vy = vx / n, vy / n
    px, py = -vy, vx
    b = (p1[0] - vx * 16, p1[1] - vy * 16)
    d.polygon([p1, (b[0] + px * 7, b[1] + py * 7), (b[0] - px * 7, b[1] - py * 7)], fill=col)
    d.ellipse([p1[0] - 6, p1[1] - 6, p1[0] + 6, p1[1] + 6], outline=col, width=2)


def draw_column(sheet, d, sc, x, y, cw):
    src = load(sc['key'], 'after')
    k = cw / src.size[0]
    big_h = int(src.size[1] * k)
    sheet.paste(src.resize((cw, big_h), Image.LANCZOS), (x, y))
    d.rectangle([x, y, x + cw - 1, y + big_h - 1], outline=EDGE)

    for (anchor, label, col, l1, l2) in sc['calls']:
        ax, ay = x + int(anchor[0] * k), y + int(anchor[1] * k)
        lx, ly = x + int(label[0] * k), y + int(label[1] * k)
        tw = int(max(d.textlength(l1, font=F_S), d.textlength(l2, font=F_S))) + 14
        bh = 44
        lx = min(max(lx, x + 4), x + cw - tw - 5)
        ly = min(max(ly, y + 4), y + big_h - bh - 5)
        # leave from the box edge nearest the anchor so the line never crosses text
        sx = lx + tw // 2
        sy = ly + bh + 2 if ay > ly + bh else ly - 2
        arrow(d, (sx, sy), (ax, ay), col)
        d.rectangle([lx, ly, lx + tw, ly + bh], fill=(9, 9, 13), outline=col)
        d.text((lx + 7, ly + 2), l1, font=F_S, fill=col)
        d.text((lx + 7, ly + 22), l2, font=F_S, fill=col)

    yy = y + big_h + 8
    d.text((x, yy), sc['cfg'], font=F_S, fill=DIM)
    yy += 28

    hw = (cw - 12) // 2
    strip_h = int(720 * hw / 1280)
    for i, v in enumerate(('before', 'after')):
        im = load(sc['key'], v).resize((hw, strip_h), Image.LANCZOS)
        bx = x + i * (hw + 12)
        d.text((bx, yy), v.upper(), font=F_S, fill=DIM if v == 'before' else FG)
        sheet.paste(im, (bx, yy + 22))
        d.rectangle([bx, yy + 22, bx + hw - 1, yy + 22 + strip_h - 1], outline=EDGE)
    yy += 22 + strip_h + 16

    cx, cy, ccw, cch = sc['crop']
    crop_h = int(cch * cw / ccw)
    cr = src.crop((cx, cy, cx + ccw, cy + cch)).resize((cw, crop_h), Image.LANCZOS)
    d.text((x, yy), sc['croplabel'], font=F_S, fill=DIM)
    sheet.paste(cr, (x, yy + 22))
    d.rectangle([x, yy + 22, x + cw - 1, yy + 22 + crop_h - 1], outline=EDGE)
    yy += 22 + crop_h + 18

    d.rectangle([x, yy, x + cw - 1, yy + 104], fill=PANEL)
    gx = x + 12
    for g in sc['gate']:
        col = GOOD if g.endswith('P') else (WARN if g.endswith('*') else BAD)
        d.text((gx, yy + 10), g, font=F_MB, fill=col)
        gx += int(d.textlength(g, font=F_MB)) + 22
    d.text((x + 12, yy + 38), sc['verdict'], font=F_H, fill=BAD)
    d.text((x + 12, yy + 72),
           'AAA score  %d / 100   \u2192  grade B  (\u201cjust has a shader\u201d)' % sc['score'],
           font=F_L, fill=WARN)
    return yy + 104


def swatch_bar(d, x, y, w):
    d.text((x, y), 'TIME-OF-DAY COLOUR ARC \u2014 mean frame colour of all three AFTER frames, '
                   'next to the approved golden reference', font=F_H, fill=FG)
    items = [(s['key'], load(s['key'], 'after')) for s in SCENES]
    items.append(('golden-beauty-shot-ref (approved)',
                  Image.open(os.path.join(ASSETS, 'golden-beauty-shot-ref.png')).convert('RGB')))
    bw = (w - 3 * 14) // 4
    for i, (nm, im) in enumerate(items):
        a = np.asarray(im).astype(np.float32)
        c = tuple(int(v) for v in a.reshape(-1, 3).mean(0))
        rb = a[..., 0].mean() - a[..., 2].mean()
        bx = x + i * (bw + 14)
        d.rectangle([bx, y + 36, bx + bw, y + 36 + 86], fill=c, outline=(100, 100, 114))
        ink = (10, 10, 14) if sum(c) > 210 else (245, 245, 250)
        d.text((bx + 10, y + 42), nm, font=F_S, fill=ink)
        d.text((bx + 10, y + 66), 'R-B %+.1f' % rb, font=F_MB, fill=ink)
        d.text((bx + 10, y + 90), 'RGB %d,%d,%d' % c, font=F_M, fill=ink)
    d.text((x, y + 132),
           'Noon is the WARMEST frame in the set (R-B +75.6) and night the coolest (+57.6). The whole '
           'day cycle moves 18 units of hue, so every hour reads as the same orange.', font=F_L, fill=DIM)
    d.text((x, y + 156),
           'The approved reference sits at +118.5. So the gap is not that we are too warm \u2014 it is '
           'that we are warm FLATLY. A cool noon is what makes a golden dusk read as golden.',
           font=F_L, fill=DIM)
    return y + 188


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else '_fl_lookv4/gap-v4.png'
    W, pad, gap = 2560, 34, 26
    cw = (W - pad * 2 - gap * 2) // 3

    sheet = Image.new('RGB', (W, 3000), BG)  # generous; cropped to content at the end
    d = ImageDraw.Draw(sheet)

    d.text((pad, 20), 'VOXELFORGE \u2014 BEAUTY GAP v4 \u00b7 2026-08-15 \u00b7 Flamingo',
           font=F_TITLE, fill=FG)
    d.text((pad, 66),
           'Grading Poppy\u2019s look pass (docs/assets/look/, shot 2026-08-14 21:45) against '
           'docs/look-acceptance-rubric.md, the approved references, and what a modern Minecraft '
           'shader pack ships.', font=F_SUB, fill=DIM)
    d.text((pad, 90),
           'The AFTER lever, read out of _shoot.log: IBL V2\u2192V3 \u00b7 ambient cut ~3\u00d7 \u00b7 '
           'rim light 0\u2192600 lx \u00b7 PCSS 8\u219212. Camera, map and sun are identical inside '
           'each pair.', font=F_SUB, fill=DIM)

    y0 = 154
    yend = y0
    for i, sc in enumerate(SCENES):
        x = pad + i * (cw + gap)
        d.text((x, y0 - 32), sc['title'], font=F_H, fill=FG)
        yend = max(yend, draw_column(sheet, d, sc, x, y0, cw))

    y = yend + 40
    d.line([pad, y - 18, W - pad, y - 18], fill=(60, 60, 74), width=2)
    y = swatch_bar(d, pad, y, W - pad * 2)

    d.line([pad, y, W - pad, y], fill=(60, 60, 74), width=2)
    y += 24

    colw = 1130
    d.text((pad, y), 'YARDSTICK \u2014 what a modern Minecraft shader pack ships',
           font=F_H, fill=FG)
    ty = y + 36
    for i, (feat, got, col) in enumerate(YARDSTICK):
        ry = ty + i * 30
        d.rectangle([pad, ry, pad + colw, ry + 27], fill=PANEL if i % 2 == 0 else (21, 21, 27))
        d.text((pad + 10, ry + 3), feat, font=F_L, fill=FG if col is GOOD else DIM)
        d.text((pad + 640, ry + 4), got, font=F_M, fill=col)
    ty2 = ty + len(YARDSTICK) * 30 + 20

    bx = pad + colw + 42
    bw = W - pad - bx
    blines = BLOCKER.split('\n')
    bh = 56 + len(blines) * 21
    d.rectangle([bx, y - 8, bx + bw, y - 8 + bh], fill=(46, 20, 20), outline=BAD, width=2)
    d.text((bx + 18, y + 6), 'BLOCKER #1 \u2014 the sun casts nothing onto the ground',
           font=F_H, fill=BAD)
    for i, ln in enumerate(blines):
        d.text((bx + 18, y + 42 + i * 21), ln, font=F_S, fill=(255, 210, 210))
    by2 = y - 8 + bh + 18

    d.rectangle([pad, ty2, pad + colw, ty2 + 118], fill=(40, 30, 22), outline=WARN)
    d.text((pad + 12, ty2 + 8), 'MUST FIX BEFORE THIS PASS MERGES', font=F_H, fill=WARN)
    d.text((pad + 366, ty2 + 14), '— item 1 is already done in the working tree, '
                                  'see the green band', font=F_S, fill=GOOD)
    d.text((pad + 12, ty2 + 42),
           'Scene 3 G3 went PASS \u2192 FAIL because of this pass: p05-L 10.1% \u2192 7.3% against a '
           'floor of 8. The ambient cut 42\u219214 is the cause,', font=F_S, fill=WARN)
    d.text((pad + 12, ty2 + 64),
           'and the rim light added alongside it does not reach the hero (see column 3). Also: PCSS '
           '8\u219212 walks toward the width already measured', font=F_S, fill=WARN)
    d.text((pad + 12, ty2 + 86),
           'as a net loss on a vista camera \u2014 it wants its own A/B before it stays in.',
           font=F_S, fill=WARN)

    # The plates moved under this sheet while it was being cut. Reporting the
    # delta beats silently re-grading a target that is still moving.
    dy = max(ty2 + 138, by2)
    dlines = DRIFT.split('\n')
    d.rectangle([pad, dy, W - pad, dy + 30 + len(dlines) * 21], fill=(20, 38, 26),
                outline=GOOD)
    for i, ln in enumerate(dlines):
        d.text((pad + 14, dy + 12 + i * 21), ln, font=F_S,
               fill=(190, 245, 205) if i else GOOD)

    # A measurement that came back against me still has to be on the sheet.
    ny = dy + 30 + len(dlines) * 21 + 16
    nlines = NEGATIVE.split('\n')
    d.rectangle([pad, ny, W - pad, ny + 30 + len(nlines) * 21], fill=(34, 30, 20),
                outline=(150, 132, 78))
    for i, ln in enumerate(nlines):
        d.text((pad + 14, ny + 12 + i * 21), ln, font=F_S,
               fill=(226, 214, 176) if i else (236, 206, 120))

    cy = ny + 30 + len(nlines) * 21 + 16
    clines = CAVEAT.split('\n')
    d.rectangle([pad, cy, W - pad, cy + 30 + len(clines) * 21], fill=(24, 28, 40),
                outline=(96, 122, 176))
    for i, ln in enumerate(clines):
        d.text((pad + 14, cy + 12 + i * 21), ln, font=F_S,
               fill=(180, 205, 245) if i else (150, 190, 255))
    fy = cy + 30 + len(clines) * 21 + 16

    d.text((pad, fy),
           'Frames graded: docs/assets/look/*_{before,after}.png PINNED AT %s '
           '(git show %s:<path> reproduces them) \u00b7 AFTER md5 outdoor %s / evening %s / night %s \u00b7 '
           'lighting config read from _shoot.log, not from the filenames.'
           % (plates.PLATE_REV, plates.PLATE_REV, plates.md5('outdoor-noon', 'after'),
              plates.md5('evening-raking', 'after'), plates.md5('night-firelit', 'after')),
           font=F_S, fill=(112, 112, 126))
    d.text((pad, fy + 22),
           'Measured by scripts/_flamingo_lookv4_axes.py, _claims.py, _contact.py, _shadowmask.py, '
           '_stage.py \u2192 scripts/grade_gate.py \u00b7 sheet by _sheet.py \u00b7 EVERY one of them resolves '
           'its input through _flamingo_lookv4_plates.plate(), so they all read the pinned set by '
           'default and the working tree only under LOOKV4_LIVE=1',
           font=F_S, fill=(112, 112, 126))

    sheet = sheet.crop((0, 0, W, fy + 56))
    os.makedirs(os.path.dirname(out) or '.', exist_ok=True)
    sheet.save(out)
    print('wrote %s (%dx%d)' % (out, sheet.size[0], sheet.size[1]))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
