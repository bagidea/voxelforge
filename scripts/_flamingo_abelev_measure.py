"""Blocker #1 A/B: does the cast shadow follow the SUN ELEVATION or the CAMERA?

Reads the four pinned plates in docs/assets/look/ab-elev/ (2 cameras x 2
elevations, everything else identical -- see _flamingo_abelev_shoot.cmd) and
answers one question per plate: *is there dark ground that a block's own side
face cannot account for?*

WHY NOT A BRIGHTNESS NUMBER. Lowering the sun from 66 to 22 deg drops ground
irradiance by sin(22)/sin(66) = 0.41 all by itself, so "the 22 frames have more
dark pixels" is true whether or not a single shadow is being drawn. Every
luminance here is therefore normalised by that plate's OWN sunlit ground level
(p90 of the ground mask), so the numbers are ratios inside one frame and a
global exposure change cancels out. This is the same trap that retired C2 in
look-gap-v4: a population statistic cannot tell a shadow edge from a falloff.

WHAT ACTUALLY DISCRIMINATES: distance to the nearest NON-ground pixel.

  * A cube's own shaded side face is, by construction, within a few px of the
    silhouette where ground stops being ground.
  * A cast shadow lies ACROSS open grass, tens of px from any geometry.

So the ground mask is split by that distance and the dark fraction reported
separately for each band:

  NEAR (<= 6 px of non-ground)  -- face shading lives here, and so do the short
                                  shadows a high sun throws (h/tan(66) = 0.45
                                  block).  Dark here proves the shadow pass runs.
  OPEN (>= 14 px of non-ground) -- nothing but a cast shadow can darken this.
                                  Dark here proves the shadow REACHES the ground.

Reported at two thresholds (0.55 and 0.65 of the plate's own sunlit level) so no
verdict rests on one cut, plus the largest connected dark-open blob and its
thickness (max inscribed radius) -- a shadow is thick, a face strip is not.
"""
import hashlib
import os
import sys

import numpy as np
from PIL import Image
from scipy import ndimage

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
AB = os.path.join(ROOT, 'docs', 'assets', 'look', 'ab-elev')
PLATES = ['s1cam-elev66', 's1cam-elev22', 's2cam-elev66', 's2cam-elev22']
NEAR_PX = 6
OPEN_PX = 14
CUTS = (0.55, 0.65)


def md5(p):
    with open(p, 'rb') as f:
        return hashlib.md5(f.read()).hexdigest()[:10]


def ground_mask(a):
    """Grass hue, verbatim from _flamingo_lookv4_shadowmask.py so the two tools
    agree on what 'ground' is."""
    L = lum(a)
    return (a[..., 1] > a[..., 2] + 12) & (a[..., 1] > a[..., 0] * 0.75) & (L > 6)


def lum(a):
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def measure(name):
    p = os.path.join(AB, name + '.png')
    a = np.asarray(Image.open(p).convert('RGB')).astype(np.float32)
    L = lum(a)
    g = ground_mask(a)
    # Distance, in px, from every ground pixel to the nearest NON-ground pixel.
    dist = ndimage.distance_transform_edt(g)
    near = g & (dist <= NEAR_PX)
    open_ = g & (dist >= OPEN_PX)
    sunlit = float(np.percentile(L[g], 90))       # this plate's own sunlit level
    n = L / max(sunlit, 1e-6)

    row = {
        'name': name, 'md5': md5(p), 'sunlit_L': round(sunlit, 1),
        'ground_px': int(g.sum()), 'near_px': int(near.sum()),
        'open_px': int(open_.sum()),
    }
    for c in CUTS:
        dark = g & (n < c)
        row['near@%.2f' % c] = pct(dark & near, near)
        row['open@%.2f' % c] = pct(dark & open_, open_)
    # Blob geometry on the strict cut, open ground only.
    dark_open = open_ & (n < CUTS[0])
    lab, k = ndimage.label(dark_open)
    if k:
        sizes = ndimage.sum(dark_open, lab, range(1, k + 1))
        big = int(np.argmax(sizes)) + 1
        blob = lab == big
        row['blobs'] = k
        row['big_blob_px'] = int(sizes.max())
        row['big_blob_pct_open'] = pct(blob, open_)
        row['big_blob_thick_px'] = round(
            float(ndimage.distance_transform_edt(blob).max()), 1)
    else:
        row.update(blobs=0, big_blob_px=0, big_blob_pct_open=0.0,
                   big_blob_thick_px=0.0)
    row['_masks'] = (g, near, open_, dark_open)
    return row


def pct(sub, base):
    b = int(base.sum())
    return round(100.0 * int((sub & base).sum()) / b, 2) if b else 0.0


def main():
    rows = [measure(n) for n in PLATES]
    cols = ['name', 'md5', 'sunlit_L', 'ground_px', 'near_px', 'open_px',
            'near@0.55', 'open@0.55', 'near@0.65', 'open@0.65',
            'blobs', 'big_blob_px', 'big_blob_pct_open', 'big_blob_thick_px']
    print('| ' + ' | '.join(cols) + ' |')
    print('|' + '---|' * len(cols))
    for r in rows:
        print('| ' + ' | '.join(str(r[c]) for c in cols) + ' |')
    return rows


if __name__ == '__main__':
    main()
    sys.exit(0)
