"""Is there a cast shadow on the ground, or only face shading?

Before I write "no cast shadow" on a sheet I want to SEE the ground-plane
luminance on its own. This paints every ground (grass) pixel by luminance
quartile and greys everything else out. A real cast shadow shows up as a
coherent dark region with a straight-ish edge offset from an occluder; face
shading alone shows up as per-block speckle that follows cube tops/sides.
"""
import os
import sys

import numpy as np
from PIL import Image

LOOK = os.path.join('docs', 'assets', 'look')
COLS = np.array([(20, 20, 34), (60, 60, 130), (110, 170, 110), (250, 240, 170)],
                dtype=np.uint8)


def main(scene, variant='after'):
    a = np.asarray(Image.open(os.path.join(LOOK, '%s_%s.png' % (scene, variant))
                              ).convert('RGB')).astype(np.float32)
    L = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
    m = (a[..., 1] > a[..., 2] + 12) & (a[..., 1] > a[..., 0] * 0.75) & (L > 6)
    q = np.percentile(L[m], [25, 50, 75])
    idx = np.digitize(L, q)
    out = np.full(a.shape, 40, dtype=np.uint8)
    out[m] = COLS[idx[m]]
    p = '_fl_lookv4/shadowmask-%s-%s.png' % (scene, variant)
    Image.fromarray(out).save(p)
    print('wrote', p, ' ground px=', int(m.sum()), ' quartiles=', np.round(q, 1))


if __name__ == '__main__':
    main(*sys.argv[1:] or ['outdoor-noon'])
