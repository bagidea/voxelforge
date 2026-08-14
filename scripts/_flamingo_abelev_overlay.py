"""Paint what the A/B numbers are actually counting, so they can be looked at.

A number that says "22.8% of open ground is dark" is only worth as much as the
pixels it drew that from -- C2 in look-gap-v4 was retired for exactly this. So
this writes one overlay per plate:

  magenta = dark ground OUT in the open (>= OPEN_PX from any non-ground pixel)
            -- the thing the verdict rests on
  blue    = dark ground hugging geometry (<= NEAR_PX) -- face shading lives here
  grey    = lit ground
  black   = not ground at all

If the magenta in a plate lands on distant haze or on a slope instead of lying
across open grass as a band with an edge, the number for that plate does not
mean what it says, and the overlay is where that shows.
"""
import os
import sys

import numpy as np
from PIL import Image
from scipy import ndimage

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _flamingo_abelev_measure as m  # noqa: E402

CUT = 0.55
OUT = os.path.join(m.AB, 'overlay')


def main(names=None):
    os.makedirs(OUT, exist_ok=True)
    for name in (names or m.PLATES):
        a = np.asarray(Image.open(os.path.join(m.AB, name + '.png')
                                  ).convert('RGB')).astype(np.float32)
        L = m.lum(a)
        g = m.ground_mask(a)
        n = L / max(float(np.percentile(L[g], 90)), 1e-6)
        dist = ndimage.distance_transform_edt(g)
        dark = g & (n < CUT)
        out = np.zeros(a.shape, np.uint8)
        out[g] = (120, 120, 120)
        out[dark & (dist <= m.NEAR_PX)] = (60, 90, 220)
        out[dark & (dist >= m.OPEN_PX)] = (245, 40, 200)
        p = os.path.join(OUT, name + '_overlay.png')
        Image.fromarray(out).save(p)
        print('wrote', p)


if __name__ == '__main__':
    main(sys.argv[1:] or None)
