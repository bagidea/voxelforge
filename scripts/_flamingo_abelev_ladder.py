"""Is the elevation effect a BUG, or just the shadow getting shorter?

The 2x2 says the missing ground shadow follows the sun's elevation and not the
camera. That kills the vista-frustum branch, but it does not by itself convict
bias or cascades -- because a shadow's LENGTH is h/tan(elev) all on its own, so
some of the drop from 22 to 66 deg is plain geometry and would be correct.

This separates them. On one camera (scene 1, held fixed) the sun is walked
22 / 34 / 50 / 66 deg and the top-face shadow coverage is compared against
cot(elev), normalised at 22 deg:

  measured tracks cot(elev)      -> the short shadow at 66 deg is CORRECT and the
                                    blocker is a framing/hour call, not a bug
  measured falls FASTER than cot -> something is eating shadow at high sun, and a
                                    bias/cascade fix has a target

Same top-face mask as _flamingo_abelev_topface.py (built from the 66 deg azimuth
pair, valid at every elevation because the camera and the world do not move).
"""
import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _flamingo_abelev_measure as m  # noqa: E402
import _flamingo_abelev_topface as tf  # noqa: E402

CAM = 's1cam'
ELEVS = [22, 34, 50, 66]
REF = 22


def main():
    _, l66a, _ = tf.load('%s-elev66' % CAM)
    _, l66b, _ = tf.load('%s-elev66-az25' % CAM)
    a66, _, _ = tf.load('%s-elev66' % CAM)
    g = m.ground_mask(a66)
    p90 = float(np.percentile(np.maximum(l66a, l66b)[g], 90))
    top = g & (np.maximum(l66a, l66b) >= tf.TOP_LIT * p90)

    sh = {}
    for e in ELEVS:
        _, L, h = tf.load('%s-elev%d' % (CAM, e))
        lit = float(np.percentile(L[top], 90))
        sh[e] = (100.0 * float((L[top] < tf.CUT * lit).sum()) / int(top.sum()), h)

    base = sh[REF][0]
    cot_ref = 1.0 / math.tan(math.radians(REF))
    print('| elev | md5 | shadowed %% of top faces | measured / %d deg | '
          'cot(elev) / cot(%d) | measured vs geometry |'
          % (REF, REF))
    print('|---|---|---|---|---|---|')
    for e in ELEVS:
        v, h = sh[e]
        pred = (1.0 / math.tan(math.radians(e))) / cot_ref
        got = v / base
        print('| %d | %s | %.2f%% | %.3f | %.3f | %s |'
              % (e, h, v, got, pred,
                 '%+.0f%%' % (100.0 * (got / pred - 1.0))))


if __name__ == '__main__':
    main()
