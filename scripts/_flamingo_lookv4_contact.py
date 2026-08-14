"""Flamingo · look-v4 — population test for contact AO on the ground plane.

Claim under test: "blocks sit on the grass with no darkening at the seam".
Rather than eyedropping one corner (which scores whatever corner I picked),
this walks the whole frame:

  1. mask the ground material (grass: G clearly dominant)
  2. distance-transform to the nearest NON-ground pixel = distance from the
     seam where something sits on the ground
  3. mean ground luminance as a function of that distance

Real contact AO makes the d=1-3 band measurably darker than the d>=12 band.
Flat-lit ground makes the curve flat. The golden ref runs through the same
code as the positive control - a metric with no control is not a verdict.
"""
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _flamingo_lookv4_plates as plates  # noqa: E402

try:
    from scipy.ndimage import distance_transform_edt as edt
except ImportError:  # pragma: no cover - fall back to a coarse ring walk
    edt = None

BANDS = [(1, 2), (2, 4), (4, 7), (7, 12), (12, 20), (20, 40)]


def lum(a):
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def ground_mask(a):
    L = lum(a)
    return (a[..., 1] > a[..., 2] + 12) & (a[..., 1] > a[..., 0] * 0.75) & (L > 6)


def profile(a, tag):
    m = ground_mask(a)
    if m.sum() < 5000 or edt is None:
        print('%-28s ground px=%d  (edt=%s) - SKIP' % (tag, m.sum(), bool(edt)))
        return
    d = edt(m)
    L = lum(a)
    far = L[m & (d >= 12)]
    if len(far) < 200:
        print('%-28s no far band - SKIP' % tag)
        return
    ref = far.mean()
    out = []
    for lo, hi in BANDS:
        b = m & (d >= lo) & (d < hi)
        out.append('%2d-%-2d %+6.2f' % (lo, hi, (L[b].mean() - ref) if b.sum() > 200 else float('nan')))
    print('%-28s ground=%6d  farL=%6.2f   dL vs far:  %s' % (tag, m.sum(), ref, '  '.join(out)))


if __name__ == '__main__':
    print(plates.banner('contact'))
    for s in ['outdoor-noon', 'evening-raking', 'night-firelit']:
        for v in ('before', 'after'):
            a = np.asarray(Image.open(
                plates.plate(s, v)).convert('RGB')).astype(np.float32)
            profile(a, '%s %s' % (s, v))
    ref = np.asarray(Image.open(os.path.join(
        'docs', 'assets', 'golden-beauty-shot-ref.png')).convert('RGB')).astype(np.float32)
    profile(ref, 'golden-ref (control)')
