"""Look at what the top-face verdict is counting, before believing it.

Paints, over the plate itself:
  green   = the TOP-face mask (horizontal ground the tool is allowed to grade)
  magenta = top-face pixels the tool calls SHADOWED

If the magenta forms coherent regions with an edge, offset from an occluder, it
is a cast shadow. If it is speckle scattered through the grass texture, the
number is texture noise and must not be quoted. Same rule the first tool in this
set failed.
"""
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _flamingo_abelev_measure as m  # noqa: E402
import _flamingo_abelev_topface as tf  # noqa: E402

OUT = os.path.join(m.AB, 'overlay')


def main():
    os.makedirs(OUT, exist_ok=True)
    for cam, _ in tf.ROWS:
        _, l66a, _ = tf.load('%s-elev66' % cam)
        _, l66b, _ = tf.load('%s-elev66-az25' % cam)
        a66, _, _ = tf.load('%s-elev66' % cam)
        g = m.ground_mask(a66)
        p90 = float(np.percentile(np.maximum(l66a, l66b)[g], 90))
        top = g & (np.maximum(l66a, l66b) >= tf.TOP_LIT * p90)
        for e in tf.ELEVS:
            name = '%s-elev%d' % (cam, e)
            a, L, _ = tf.load(name)
            lit = float(np.percentile(L[top], 90))
            sh = top & (L < tf.CUT * lit)
            out = (a * 0.45).astype(np.uint8)
            out[top] = (out[top] * 0.5 + np.array([40, 200, 90]) * 0.5)
            out[sh] = (245, 40, 200)
            p = os.path.join(OUT, name + '_topface.png')
            Image.fromarray(out).save(p)
            print('wrote', p, ' shadowed px=', int(sh.sum()))


if __name__ == '__main__':
    main()
