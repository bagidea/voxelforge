"""Is the A/B verdict an artefact of where I drew the OPEN band?

_flamingo_abelev_measure.py splits the ground mask by distance-to-geometry in
PIXELS, and pixels are camera-dependent: if the vista camera renders a block
smaller on screen, a 14 px band eats more of a block there than on the close
camera, and 'no shadow on open ground' could be nothing but that. So:

  1. the two cameras are 22.06 and 21.22 world units from their look-at point
     (computed from the CINE triples in _flamingo_abelev_shoot.cmd) -- 4% apart,
     so px-per-block is within 4% between the rows to begin with; and
  2. this sweeps the OPEN radius 4 -> 32 px anyway, plus a band-free number
     (dark fraction over the WHOLE ground mask), so a verdict that only holds at
     one radius cannot survive here.

Same normalisation as the main tool: every luminance is divided by that plate's
own p90 ground level, so the exposure drop from lowering the sun cancels.
"""
import os
import sys

import numpy as np
from PIL import Image
from scipy import ndimage

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _flamingo_abelev_measure as m  # noqa: E402

RADII = (4, 8, 14, 20, 26, 32)
CUT = 0.55


def main():
    print('dark ground (normalised L < %.2f x that plate\'s own sunlit p90)' % CUT)
    print()
    head = ['plate', 'ALL ground'] + ['open>=%dpx' % r for r in RADII]
    print('| ' + ' | '.join(head) + ' |')
    print('|' + '---|' * len(head))
    for name in m.PLATES:
        a = np.asarray(Image.open(os.path.join(m.AB, name + '.png')
                                  ).convert('RGB')).astype(np.float32)
        L = m.lum(a)
        g = m.ground_mask(a)
        n = L / max(float(np.percentile(L[g], 90)), 1e-6)
        dark = g & (n < CUT)
        dist = ndimage.distance_transform_edt(g)
        cells = ['%.2f' % (100.0 * dark.sum() / g.sum())]
        for r in RADII:
            band = g & (dist >= r)
            cells.append('%.2f' % (100.0 * (dark & band).sum() / band.sum())
                         if band.sum() else 'n/a')
        print('| ' + name + ' | ' + ' | '.join(cells) + ' |')


if __name__ == '__main__':
    main()
