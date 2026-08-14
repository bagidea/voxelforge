"""Flamingo · look-v4 axis probe — the numbers behind the v4 gap sheet.

Every axis here is one I am about to write a claim about on the sheet, so it
gets measured on the exact file the caption names. Axes:

  void        near-black pixels in the upper sky band (dome missing / void)
  clip        B-channel clamped to 0 (gamut crush - a warm PASS bought by a
              dead channel is not a warm frame; see the colour-gate scar)
  tod         R-B separation between the three times of day (does noon read
              different from evening?)
  depth       near-half vs far-half contrast (aerial perspective doing work?)
  shadowdir   directional-shadow evidence: contrast of top-facing vs
              side-facing block faces
  bloom       halo falloff around the brightest emissive cluster
  micro       high-frequency energy (texture/normal detail survival)
"""
import os

import numpy as np
from PIL import Image

LOOK = os.path.join('docs', 'assets', 'look')
SCENES = ['outdoor-noon', 'evening-raking', 'night-firelit']


def load(scene, variant):
    p = os.path.join(LOOK, '%s_%s.png' % (scene, variant))
    return np.asarray(Image.open(p).convert('RGB')).astype(np.float32)


def lum(a):
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def void_frac(a):
    """Near-black pixels in the top 22% of frame = sky that never rendered."""
    band = a[: int(a.shape[0] * 0.22)]
    return float((lum(band) < 12).mean())


def clip_frac(a):
    return float((a[..., 2] < 1).mean())


def depth_split(a):
    """Contrast in the far half (upper) vs near half (lower) of the frame."""
    L = lum(a)
    h = L.shape[0] // 2
    return float(L[:h].std()), float(L[h:].std())


def face_split(a):
    """Voxel top faces are the brightest 30% of local-max pixels; side faces
    the rest. A lit scene separates them; a flat-lit one does not."""
    L = lum(a)
    hi = np.percentile(L, 80)
    lo = np.percentile(L, 35)
    return float(L[L >= hi].mean() - L[L <= lo].mean())


def bloom_falloff(a):
    """Ring means at r=8 and r=40 px around the brightest blob centroid.
    A real bloom leaves a soft halo: ring8 clearly above ring40 but both
    above the frame floor. No bloom = ring8 collapses to local background."""
    L = lum(a)
    idx = np.unravel_index(np.argmax(L), L.shape)
    yy, xx = np.mgrid[0:L.shape[0], 0:L.shape[1]]
    d = np.hypot(yy - idx[0], xx - idx[1])
    r8 = L[(d > 6) & (d <= 12)].mean()
    r40 = L[(d > 34) & (d <= 46)].mean()
    return float(L[idx]), float(r8), float(r40)


def micro(a):
    L = lum(a)
    gx = np.abs(np.diff(L, axis=1)).mean()
    gy = np.abs(np.diff(L, axis=0)).mean()
    return float((gx + gy) / 2)


def main():
    print('%-16s %-7s %7s %7s %8s %8s %8s %8s %8s' % (
        'scene', 'var', 'void%', 'B=0%', 'stdFar', 'stdNear', 'faceD', 'micro', 'R-B'))
    for scene in SCENES:
        for variant in ('before', 'after'):
            a = load(scene, variant)
            f, n = depth_split(a)
            print('%-16s %-7s %6.2f%% %6.2f%% %8.2f %8.2f %8.2f %8.3f %+8.1f' % (
                scene, variant, void_frac(a) * 100, clip_frac(a) * 100,
                f, n, face_split(a), micro(a),
                a[..., 0].mean() - a[..., 2].mean()))
    print()
    print('-- bloom falloff around brightest blob (peakL, ring8, ring40)')
    for scene in SCENES:
        for variant in ('before', 'after'):
            print('   %-16s %-7s %6.1f %6.1f %6.1f' % (
                (scene, variant) + bloom_falloff(load(scene, variant))[0:3]))
    print()
    ref = np.asarray(Image.open(
        os.path.join('docs', 'assets', 'golden-beauty-shot-ref.png')).convert('RGB')
    ).astype(np.float32)
    f, n = depth_split(ref)
    print('golden-beauty-shot-ref  void=%.2f%% B=0 %.2f%% stdFar=%.2f stdNear=%.2f '
          'faceD=%.2f micro=%.3f R-B=%+.1f' % (
              void_frac(ref) * 100, clip_frac(ref) * 100, f, n,
              face_split(ref), micro(ref), ref[..., 0].mean() - ref[..., 2].mean()))


if __name__ == '__main__':
    main()
