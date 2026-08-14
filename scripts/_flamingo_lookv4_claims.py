"""Flamingo · look-v4 — one check per claim I intend to print on the sheet.

Rule I work by: a label with an arrow on it has to have a number behind it,
measured on the file the caption names. Claims checked here:

  C1 void-black   the floating squares in the sky band really are ~0,0,0
                  (a rendering hole, not a dark colour choice)
  C2 cast-shadow  does the grass population split into lit + shadowed?
                  no split = the sun casts nothing onto the ground
  C3 bloom        radial profile out from an emissive lamp cube
  C4 char-bounce  the night silhouette vs the lit wall 40px away
"""
import os

import numpy as np
from PIL import Image

LOOK = os.path.join('docs', 'assets', 'look')


def load(scene, variant='after'):
    return np.asarray(Image.open(
        os.path.join(LOOK, '%s_%s.png' % (scene, variant))).convert('RGB')
    ).astype(np.float32)


def lum(a):
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def c1_void(scene):
    a = load(scene)
    band = a[: int(a.shape[0] * 0.22)]
    L = lum(band)
    m = L < 12
    print('C1 %-15s upper-band px<L12: %5.2f%%   their mean RGB=(%.1f,%.1f,%.1f) '
          'max=%.0f' % (scene, m.mean() * 100,
                        band[..., 0][m].mean() if m.any() else -1,
                        band[..., 1][m].mean() if m.any() else -1,
                        band[..., 2][m].mean() if m.any() else -1,
                        band.max(axis=2)[m].max() if m.any() else -1))


def c2_cast_shadow(scene):
    """Grass = pixels where G is the dominant channel by a clear margin.
    If the sun casts onto the ground the grass luminance splits in two."""
    a = load(scene)
    L = lum(a)
    g = (a[..., 1] > a[..., 2] + 12) & (a[..., 1] > a[..., 0] * 0.75) & (L > 8)
    if g.sum() < 500:
        print('C2 %-15s grass px too few (%d) - skip' % (scene, g.sum()))
        return
    v = L[g]
    lo, hi = np.percentile(v, 10), np.percentile(v, 90)
    # bimodality: Otsu-style between-class variance ratio
    ts = np.linspace(v.min() + 1, v.max() - 1, 64)
    best, bt = 0, 0
    for t in ts:
        a1, a2 = v[v <= t], v[v > t]
        if len(a1) < 10 or len(a2) < 10:
            continue
        w1, w2 = len(a1) / len(v), len(a2) / len(v)
        bc = w1 * w2 * (a1.mean() - a2.mean()) ** 2
        if bc > best:
            best, bt = bc, t
    sep = np.sqrt(best) / (v.std() + 1e-6)
    print('C2 %-15s grass n=%7d  L p10=%5.1f p90=%5.1f  spread=%5.1f  '
          'otsu-sep=%.3f  split@%.0f' % (scene, g.sum(), lo, hi, hi - lo, sep, bt))


def c3_bloom(scene, cx, cy):
    a = load(scene)
    L = lum(a)
    yy, xx = np.mgrid[0:L.shape[0], 0:L.shape[1]]
    d = np.hypot(yy - cy, xx - cx)
    print('C3 %-15s emissive @(%d,%d) radial L:' % (scene, cx, cy), end=' ')
    for r0, r1 in [(0, 6), (10, 14), (18, 22), (28, 34), (44, 52), (70, 80)]:
        m = (d >= r0) & (d < r1)
        print('r%02d-%02d=%.0f' % (r0, r1, L[m].mean()), end='  ')
    print()


def c4_char_bounce():
    a = load('night-firelit')
    # silhouette body block and the lit wall to its right, same scanline band
    sil = a[300:420, 150:250]
    wall = a[300:420, 300:400]
    for nm, p in (('silhouette', sil), ('lit wall', wall)):
        L = lum(p)
        print('C4 night %-11s meanRGB=(%.1f,%.1f,%.1f) L=%.1f  R-B=%+.1f'
              % (nm, p[..., 0].mean(), p[..., 1].mean(), p[..., 2].mean(),
                 L.mean(), p[..., 0].mean() - p[..., 2].mean()))


if __name__ == '__main__':
    for s in ['outdoor-noon', 'evening-raking', 'night-firelit']:
        c1_void(s)
    print()
    for s in ['outdoor-noon', 'evening-raking', 'night-firelit']:
        c2_cast_shadow(s)
    print()
    c3_bloom('outdoor-noon', 940, 180)
    c3_bloom('night-firelit', 603, 315)
    print()
    c4_char_bounce()
