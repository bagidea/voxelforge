"""Cast shadow on HORIZONTAL ground only -- the verdict tool for blocker #1.

The first tool in this set (_flamingo_abelev_measure.py) is kept on disk and is
WRONG for this question: it counted a grass block's own side face as open ground,
because a side face is grass-hued and therefore sits inside the ground mask far
from any mask boundary. The overlays under docs/assets/look/ab-elev/overlay/
show that plainly. Nothing in this file uses its numbers.

THE SEPARATION. A horizontal face's direct sun term is `illuminance*sin(elev)` --
independent of the sun's compass bearing. A vertical face's is
`cos(elev)*cos(delta-azimuth)` -- entirely dependent on it. So each cell of the
grid was shot twice, azimuth 205 and azimuth 25 (180 deg apart):

  TOP mask  = ground pixels that reach the top-lit level in EITHER azimuth at
              66 deg.  A side face cannot: at 180 deg apart, whichever azimuth
              lights it, the other leaves it on ambient, and at 66 deg its lit
              level is cos(66)/sin(66) = 0.45x a top's anyway.
  the mask is taken at 66 deg and reused verbatim at 22 deg -- same camera, same
  world, same pixels, so it is the same set of horizontal faces.

  SHADOW % = of that fixed TOP mask, the fraction below `CUT` x the top-lit
             level OF THAT FRAME (p90 over the mask), so lowering the sun
             darkening everything by sin(22)/sin(66) cancels out.

Only a cast shadow can darken a horizontal face. There is no face-shading
explanation left in this number.

ONE MORE THING THE FIRST DRAFT GOT WRONG, and the overlay caught it too: the
grass texture is high-frequency speckle, and thresholding raw pixels counted its
own dark specks as shadow (11.99% of scene 1's top faces at 66 deg, and the
overlay showed it scattered through the texture, not lying in a region). A cast
shadow is a LARGE-SCALE feature, so every luminance here is Gaussian-blurred by
[`SMOOTH`] px first. That is what makes the difference between the two rows a
shadow number rather than a texture number.
"""
import os
import sys

import numpy as np
from PIL import Image
from scipy import ndimage

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _flamingo_abelev_measure as m  # noqa: E402

CUT = 0.55          # below this x the frame's own top-lit level = shadowed
TOP_LIT = 0.80      # at/above this x p90 = a face standing up to the full sun
SMOOTH = 4.0        # px, kills grass-texture speckle; a shadow survives it
ROWS = [('s1cam', 'the wide vista camera (scene 1)'),
        ('s2cam', 'the close raking camera (scene 2)')]
ELEVS = [66, 22]


def load(name):
    p = os.path.join(m.AB, name + '.png')
    a = np.asarray(Image.open(p).convert('RGB')).astype(np.float32)
    return a, ndimage.gaussian_filter(m.lum(a), SMOOTH), m.md5(p)


def main():
    print('| camera | elev | plate (az205) | md5 | top-face px | top-lit L | '
          'shadowed % of top faces |')
    print('|---|---|---|---|---|---|---|')
    out = {}
    for cam, _label in ROWS:
        # --- the mask, built at 66 deg where a top out-reads a side 2.25:1 ---
        _, l66a, _ = load('%s-elev66' % cam)
        _, l66b, _ = load('%s-elev66-az25' % cam)
        a66, _, _ = load('%s-elev66' % cam)
        g = m.ground_mask(a66)
        p90 = float(np.percentile(np.maximum(l66a, l66b)[g], 90))
        top = g & (np.maximum(l66a, l66b) >= TOP_LIT * p90)
        for e in ELEVS:
            name = '%s-elev%d' % (cam, e)
            _, L, h = load(name)
            lit = float(np.percentile(L[top], 90))
            sh = 100.0 * float((L[top] < CUT * lit).sum()) / int(top.sum())
            out[name] = sh
            print('| %s | %d | %s.png | %s | %d | %.1f | **%.2f%%** |'
                  % (cam, e, name, h, int(top.sum()), lit, sh))
    print()
    print('elevation effect, camera held: '
          's1cam %.2f -> %.2f, s2cam %.2f -> %.2f  (66 -> 22 deg)'
          % (out['s1cam-elev66'], out['s1cam-elev22'],
             out['s2cam-elev66'], out['s2cam-elev22']))
    print('camera effect, elevation held: '
          '66 deg %.2f -> %.2f, 22 deg %.2f -> %.2f  (s1cam -> s2cam)'
          % (out['s1cam-elev66'], out['s2cam-elev66'],
             out['s1cam-elev22'], out['s2cam-elev22']))
    return out


if __name__ == '__main__':
    main()
