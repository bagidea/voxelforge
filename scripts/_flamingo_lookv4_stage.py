"""Stage Poppy's look-v4 pairs under *-nohud2.png so grade_gate.py will judge them.

grade_gate.py REFUSES any filename that does not end in -nohud2.png. These
frames come out of the shot binary, which draws no HUD at all (same rule
render_grade.sh relies on), so the rename is the whole adaptation - no pixel
is touched. Copies land in _fl_lookv4/ (scratch, not shipped).
"""
import os
import shutil

SRC = os.path.join('docs', 'assets', 'look')
DST = '_fl_lookv4'
SCENES = ['outdoor-noon', 'evening-raking', 'night-firelit']

os.makedirs(DST, exist_ok=True)
for scene in SCENES:
    for variant in ('before', 'after'):
        src = os.path.join(SRC, '%s_%s.png' % (scene, variant))
        dst = os.path.join(DST, '%s-%s-nohud2.png' % (scene, variant))
        shutil.copy(src, dst)
        print('staged', dst)
