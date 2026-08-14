"""Cut the zoom crops I need to judge shadow/AO/edge claims at 100%+.

I do not grade a shadow claim from a 1280-wide thumbnail - the rubric's G4
asks for 400% on a shadow edge, so the crop has to exist before the verdict.
"""
import os
import sys

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _flamingo_lookv4_plates as plates  # noqa: E402

OUT = '_fl_lookv4'

CROPS = [
    # (scene, variant, x, y, w, h, tag)
    ('outdoor-noon', 'after', 380, 400, 420, 240, 'ground-under-arch'),
    ('outdoor-noon', 'after', 820, 60, 420, 240, 'far-terrain-sky'),
    ('evening-raking', 'after', 180, 280, 420, 240, 'raking-tower-base'),
    ('evening-raking', 'after', 700, 0, 460, 260, 'sky-void'),
    ('night-firelit', 'after', 420, 250, 420, 240, 'firelight-falloff'),
    ('night-firelit', 'after', 60, 250, 300, 300, 'silhouette-rim'),
]

print(plates.banner('crops'))
os.makedirs(OUT, exist_ok=True)
for scene, variant, x, y, w, h, tag in CROPS:
    im = Image.open(plates.plate(scene, variant)).convert('RGB')
    c = im.crop((x, y, x + w, y + h)).resize((w * 2, h * 2), Image.NEAREST)
    p = os.path.join(OUT, 'crop-%s-%s.png' % (scene, tag))
    c.save(p)
    print('wrote', p)
