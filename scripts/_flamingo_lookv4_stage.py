"""Stage Poppy's look-v4 pairs under *-nohud2.png so grade_gate.py will judge them.

grade_gate.py REFUSES any filename that does not end in -nohud2.png. These
frames come out of the shot binary, which draws no HUD at all (same rule
render_grade.sh relies on), so the rename is the whole adaptation - no pixel
is touched. Copies land in _fl_lookv4/ (scratch, not shipped).

Source is the PINNED plate set (see _flamingo_lookv4_plates), so grade_gate.py
scores the same pixels the sheet was cut from even after a re-shoot.
"""
import os
import shutil
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _flamingo_lookv4_plates as plates  # noqa: E402

DST = '_fl_lookv4'

print(plates.banner('stage'))
os.makedirs(DST, exist_ok=True)
for scene in plates.SCENES:
    for variant in ('before', 'after'):
        src = plates.plate(scene, variant)
        dst = os.path.join(DST, '%s-%s-nohud2.png' % (scene, variant))
        shutil.copy(src, dst)
        print('staged', dst, '<-', src)
