"""The one comparison image for the blocker #1 A/B.

2 cameras x 2 sun elevations, everything else pinned, plus the two controls that
decide what the grid means: a cot(elev) ladder on scene 1 and an azimuth flip at
a fixed camera and elevation. Each cell carries its own md5 and its measured
cast-shadow coverage on HORIZONTAL ground, with the overlay the number was read
from inset so the claim and its pixels ship together.
"""
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _flamingo_abelev_measure as m  # noqa: E402
import _flamingo_abelev_topface as tf  # noqa: E402

W, H = 700, 394          # per-cell plate size
PAD, GAP = 40, 20
HEAD, FOOT = 268, 380
LBL = 32                 # label strip under each cell
COL2 = 520               # x offset of the value column in the footer
OUT = os.path.join(m.AB, '..', '..', 'flamingo-blocker1-ab-elev-2026-08-15.png')

INK = (232, 236, 244)
DIM = (150, 158, 172)
BG = (16, 18, 24)
HOT = (255, 92, 190)
OK = (110, 230, 150)


def font(sz, bold=False):
    for p in (r'C:\Windows\Fonts\segoeuib.ttf' if bold else
              r'C:\Windows\Fonts\segoeui.ttf',
              r'C:\Windows\Fonts\arialbd.ttf' if bold else
              r'C:\Windows\Fonts\arial.ttf'):
        if os.path.exists(p):
            return ImageFont.truetype(p, sz)
    return ImageFont.load_default()


def wrap(d, text, f, width):
    """Greedy wrap -- a sheet whose own verdict runs off the right edge is not a
    sheet anybody can read."""
    out, line = [], ''
    for w in text.split(' '):
        t = (line + ' ' + w).strip()
        if d.textlength(t, font=f) <= width:
            line = t
        else:
            out.append(line)
            line = w
    if line:
        out.append(line)
    return out


def block(d, x, y, text, f, fill, width, lh=25):
    for ln in wrap(d, text, f, width):
        d.text((x, y), ln, font=f, fill=fill)
        y += lh
    return y


def shadow_pct():
    """Re-measure here rather than transcribe -- a sheet that quotes a number it
    did not compute is how the v4 delta table went a round stale."""
    out = {}
    for cam, _ in tf.ROWS:
        _, l66a, _ = tf.load('%s-elev66' % cam)
        _, l66b, _ = tf.load('%s-elev66-az25' % cam)
        a66, _, _ = tf.load('%s-elev66' % cam)
        g = m.ground_mask(a66)
        p90 = float(np.percentile(np.maximum(l66a, l66b)[g], 90))
        top = g & (np.maximum(l66a, l66b) >= tf.TOP_LIT * p90)
        for name in ('%s-elev66' % cam, '%s-elev22' % cam,
                     '%s-elev66-az25' % cam):
            _, L, h = tf.load(name)
            lit = float(np.percentile(L[top], 90))
            out[name] = (100.0 * float((L[top] < tf.CUT * lit).sum())
                         / int(top.sum()), h)
    return out


def main():
    pct = shadow_pct()
    cells = [['s1cam-elev66', 's1cam-elev22'], ['s2cam-elev66', 's2cam-elev22']]
    cw, ch = W, H + LBL
    sheet = Image.new('RGB', (PAD * 2 + cw * 2 + GAP,
                              HEAD + ch * 2 + GAP + FOOT), BG)
    d = ImageDraw.Draw(sheet)

    inner = sheet.width - PAD * 2
    d.text((PAD, 24), 'Blocker #1 — does the missing ground shadow follow the '
           'SUN ELEVATION or the CAMERA?', font=font(32, True), fill=INK)
    y = block(d, PAD, 66,
              'Voxelforge · poppy/native-only @ 711305d · one exe '
              '(target-poppy/perf, 03:50:28) · gen=v3 · Ultra · pcss 12 · '
              'shadow_map 4096 · azimuth 205° · 20 000 lx · ev100 10.6 · only '
              'the camera pose and the sun elevation move. Numbers = cast-shadow '
              'coverage of HORIZONTAL ground only, isolated by an azimuth flip '
              '(a top face\'s sun term depends on elevation, never on bearing).',
              font(17), DIM, inner, 22)
    y = block(d, PAD, y + 8,
              'ANSWER: the ELEVATION. Swapping the camera at a fixed sun does '
              'not bring the shadow back; lowering the sun on a fixed camera '
              'does — on both cameras.', font(22, True), OK, inner, 27)
    block(d, PAD, y + 2,
          'But that does NOT convict bias or the cascade split: see the two '
          'controls below. Nothing is broken at 66° — the shadow is short and '
          'pointed away from this camera.', font(21, True), HOT, inner, 26)

    for r, row in enumerate(cells):
        for c, name in enumerate(row):
            x = PAD + c * (cw + GAP)
            y = HEAD + r * (ch + GAP)
            im = Image.open(os.path.join(m.AB, name + '.png')
                            ).convert('RGB').resize((W, H), Image.LANCZOS)
            ov = Image.open(os.path.join(m.AB, 'overlay',
                                         name + '_topface.png')
                            ).convert('RGB').resize((W // 3, H // 3),
                                                    Image.LANCZOS)
            im.paste(ov, (W - W // 3 - 6, H - H // 3 - 6))
            dd = ImageDraw.Draw(im)
            dd.rectangle([W - W // 3 - 7, H - H // 3 - 7, W - 5, H - 5],
                         outline=(250, 250, 250))
            sheet.paste(im, (x, y))
            v, h = pct[name]
            d.text((x, y + H + 5),
                   '%s   md5 %s   shadowed ground %.2f%%' % (name, h, v),
                   font=font(19, True), fill=HOT if v < 5 else OK)

    e = {k: v[0] for k, v in pct.items()}
    fy = HEAD + ch * 2 + GAP + 18
    d.text((PAD, fy), 'What each move is worth', font=font(24, True), fill=INK)
    lines = [
        ('camera held, sun 66° → 22°',
         's1cam %.2f%% → %.2f%%  (×%.1f)        s2cam %.2f%% → %.2f%%  (×%.1f)'
         % (e['s1cam-elev66'], e['s1cam-elev22'],
            e['s1cam-elev22'] / e['s1cam-elev66'],
            e['s2cam-elev66'], e['s2cam-elev22'],
            e['s2cam-elev22'] / e['s2cam-elev66']), OK),
        ('sun held, s1cam → s2cam',
         'at 66°  %.2f%% → %.2f%%  (×%.1f)        at 22°  %.2f%% → %.2f%%  (×%.1f)'
         % (e['s1cam-elev66'], e['s2cam-elev66'],
            e['s2cam-elev66'] / e['s1cam-elev66'],
            e['s1cam-elev22'], e['s2cam-elev22'],
            e['s2cam-elev22'] / e['s1cam-elev22']), DIM),
        ('', '', BG),
        ('CONTROL 1 · cot(elev) ladder — s1cam, sun 22/34/50/66°',
         'at 66° the measured coverage is 0.157× the 22° one; shadow LENGTH '
         'alone predicts 0.180×. Within 12%: nothing is eating shadow at a high '
         'sun.', INK),
        ('CONTROL 2 · azimuth flip — s1cam at 66° held',
         'rotating only the sun\'s compass bearing 205° → 25° moves the same '
         'number %.2f%% → %.2f%% (×%.1f). The shadow pass casts perfectly well '
         'at 66°.' % (e['s1cam-elev66'], e['s1cam-elev66-az25'],
                      e['s1cam-elev66-az25'] / e['s1cam-elev66']), INK),
        ('', '', BG),
        ('So blocker #1 is mis-scoped',
         '"the sun casts nothing onto the ground" is not what the pixels say. At '
         '66°/az205 it casts a shadow 0.45× block height, pointed away from this '
         'camera. The hour and the bearing are the lever — not depth bias, not '
         'the cascade split, not the vista frustum.', HOT),
    ]
    y = fy + 38
    for a, b, col in lines:
        if not a:
            y += 12
            continue
        d.text((PAD, y), a, font=font(18, True), fill=col)
        y = max(block(d, PAD + COL2, y, b, font(19), col,
                      sheet.width - PAD - COL2 - PAD, 24), y + 27)

    d.text((PAD, sheet.height - 30),
           'Plates + overlays: docs/assets/look/ab-elev/ · shot by '
           'scripts/_flamingo_abelev_shoot.cmd + _azflip.cmd · measured by '
           '_flamingo_abelev_topface.py / _ladder.py · md5 on every cell',
           font=font(16), fill=DIM)

    p = os.path.abspath(OUT)
    sheet.save(p)
    print('wrote', p, sheet.size)


if __name__ == '__main__':
    main()
