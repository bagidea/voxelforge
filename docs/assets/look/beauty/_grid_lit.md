<!-- provenance:begin -->
# RE-SHOT 2026-08-17 on a NEW binary. The 2026-08-16 lit-only table CANNOT be recovered: it was never
# written to _grid.json (the JSON carries all-pixel structure only) and no transcript holds the file,
# so its numbers exist today only as the quoted table in docs/flamingo-beauty-ladder-2026-08-16.md section 2.
# Binary: HEAD 2262f71 + main-tree client/src/anim.rs, exe md5 e850f2c0..., 2026-08-17 14:22.
# Lit-only mask (L >= 12), 52 plates (4 poses x 13 rungs) -- the decisive table, same rule as 08-16.
<!-- provenance:end -->

REF = docs/assets/moodboard.png crop (514, 0, 1024, 1024)   poses=4  rungs=13  plates=52

### #2 value span L(p95-p5)   REF 209.5   TARGET >= 150
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                       105.1 f               105.9 f               103.1 f               111.3 f        fail
fog8-45          128.2 (+23.1) f       128.6 (+22.7) f       126.7 (+23.6) f       130.4 (+19.1) f        fail
fog6-55          122.7 (+17.6) f       123.2 (+17.3) f       120.8 (+17.7) f       125.7 (+14.4) f        fail
fog5-70          117.3 (+12.2) f       117.2 (+11.3) f       115.3 (+12.2) f        120.8 (+9.5) f        fail
amb1700           109.0 (+3.9) f        109.4 (+3.5) f        107.0 (+3.9) f        115.0 (+3.7) f        fail
amb1400           108.0 (+2.9) f        108.4 (+2.5) f        106.0 (+2.9) f        114.1 (+2.8) f        fail
amb1100           106.8 (+1.7) f        107.4 (+1.5) f        105.0 (+1.9) f        112.9 (+1.6) f        fail
ev11.3            91.4 (-13.7) f        93.3 (-12.6) f        89.5 (-13.6) f        94.9 (-16.4) f        fail
ev12.0            85.9 (-19.2) f        88.3 (-17.6) f        85.1 (-18.0) f        86.7 (-24.6) f        fail
atmos-off        122.7 (+17.6) f       121.8 (+15.9) f       119.9 (+16.8) f       129.0 (+17.7) f        fail
win-atmosfog       133.3 (+28.2) f       133.2 (+27.3) f       130.6 (+27.5) f       136.6 (+25.3) f        fail
fill-up           105.0 (-0.1) f        105.8 (-0.1) f        103.1 (+0.0) f        111.2 (-0.1) f        fail
fill-down         105.1 (+0.0) f        105.9 (+0.0) f        103.2 (+0.1) f        111.3 (+0.0) f        fail

### #3 shadow L p5   REF 23.7   TARGET 20-28
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                        14.7 f                14.5 f                15.8 f                14.6 f        fail
fog8-45            14.6 (-0.1) f         14.5 (+0.0) f         15.7 (-0.1) f         14.6 (+0.0) f        fail
fog6-55            14.6 (-0.1) f         14.6 (+0.1) f         15.7 (-0.1) f         14.6 (+0.0) f        fail
fog5-70            14.7 (+0.0) f         14.6 (+0.1) f         15.8 (+0.0) f         14.6 (+0.0) f        fail
amb1700            14.7 (+0.0) f         14.6 (+0.1) f         15.8 (+0.0) f         14.6 (+0.0) f        fail
amb1400            14.7 (+0.0) f         14.6 (+0.1) f         15.8 (+0.0) f         14.6 (+0.0) f        fail
amb1100            14.7 (+0.0) f         14.6 (+0.1) f         15.8 (+0.0) f         14.6 (+0.0) f        fail
ev11.3             14.5 (-0.2) f         13.8 (-0.7) f         15.5 (-0.3) f         13.9 (-0.7) f        fail
ev12.0             14.4 (-0.3) f         13.7 (-0.8) f         15.0 (-0.8) f         13.6 (-1.0) f        fail
atmos-off          15.2 (+0.5) f         15.0 (+0.5) f         16.9 (+1.1) f         14.6 (+0.0) f        fail
win-atmosfog         15.3 (+0.6) f         15.0 (+0.5) f         17.0 (+1.2) f         14.6 (+0.0) f        fail
fill-up            14.8 (+0.1) f         14.7 (+0.2) f         15.9 (+0.1) f         14.7 (+0.1) f        fail
fill-down          14.6 (-0.1) f         14.4 (-0.1) f         15.7 (-0.1) f         14.6 (+0.0) f        fail

### #3 shadow % of white   REF 9.3%   TARGET 8-11%
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                        5.8% f                5.7% f                6.2% f                5.7% f        fail
fog8-45            5.7% (-0.0) f         5.7% (+0.0) f         6.2% (-0.0) f         5.7% (+0.0) f        fail
fog6-55            5.7% (-0.0) f         5.7% (+0.0) f         6.2% (-0.0) f         5.7% (+0.0) f        fail
fog5-70            5.8% (+0.0) f         5.7% (+0.0) f         6.2% (+0.0) f         5.7% (+0.0) f        fail
amb1700            5.8% (+0.0) f         5.7% (+0.0) f         6.2% (+0.0) f         5.7% (+0.0) f        fail
amb1400            5.8% (+0.0) f         5.7% (+0.0) f         6.2% (+0.0) f         5.7% (+0.0) f        fail
amb1100            5.8% (+0.0) f         5.7% (+0.0) f         6.2% (+0.0) f         5.7% (+0.0) f        fail
ev11.3             5.7% (-0.1) f         5.4% (-0.3) f         6.1% (-0.1) f         5.5% (-0.3) f        fail
ev12.0             5.6% (-0.1) f         5.4% (-0.3) f         5.9% (-0.3) f         5.3% (-0.4) f        fail
atmos-off          6.0% (+0.2) f         5.9% (+0.2) f         6.6% (+0.4) f         5.7% (+0.0) f        fail
win-atmosfog         6.0% (+0.2) f         5.9% (+0.2) f         6.7% (+0.5) f         5.7% (+0.0) f        fail
fill-up            5.8% (+0.0) f         5.8% (+0.1) f         6.2% (+0.0) f         5.8% (+0.0) f        fail
fill-down          5.7% (-0.0) f         5.6% (-0.0) f         6.2% (-0.0) f         5.7% (+0.0) f        fail

### #5 chroma span sat(p95-p5)   REF 0.776   TARGET >= 0.62
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                       0.824 P               0.877 P               0.714 P               0.867 P        PASS
fog8-45         0.395 (-0.429) f      0.456 (-0.421) f      0.353 (-0.361) f      0.436 (-0.431) f        fail
fog6-55         0.404 (-0.420) f      0.462 (-0.415) f      0.392 (-0.322) f      0.453 (-0.414) f        fail
fog5-70         0.464 (-0.360) f      0.499 (-0.378) f      0.489 (-0.225) f      0.488 (-0.379) f        fail
amb1700         0.824 (+0.000) P      0.877 (+0.000) P      0.714 (+0.000) P      0.860 (-0.007) P        PASS
amb1400         0.824 (+0.000) P      0.877 (+0.000) P      0.714 (+0.000) P      0.861 (-0.006) P        PASS
amb1100         0.824 (+0.000) P      0.877 (+0.000) P      0.714 (+0.000) P      0.867 (+0.000) P        PASS
ev11.3          0.826 (+0.002) P      0.875 (-0.002) P      0.692 (-0.022) P      0.875 (+0.008) P        PASS
ev12.0          0.826 (+0.002) P      0.864 (-0.013) P      0.667 (-0.047) P      0.867 (+0.000) P        PASS
atmos-off       0.665 (-0.159) P      0.684 (-0.193) P      0.639 (-0.075) P      0.652 (-0.215) P        PASS
win-atmosfog      0.160 (-0.664) f      0.200 (-0.677) f      0.188 (-0.526) f      0.204 (-0.663) f        fail
fill-up         0.842 (+0.018) P      0.873 (-0.004) P      0.750 (+0.036) P      0.861 (-0.006) P        PASS
fill-down       0.810 (-0.014) P      0.877 (+0.000) P      0.688 (-0.026) P      0.863 (-0.004) P        PASS

### #6 sat p5   REF 0.224   TARGET <= 0.32
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                       0.176 P               0.115 P               0.286 P               0.125 P        PASS
fog8-45         0.176 (+0.000) P      0.115 (+0.000) P      0.282 (-0.004) P      0.125 (+0.000) P        PASS
fog6-55         0.176 (+0.000) P      0.115 (+0.000) P      0.282 (-0.004) P      0.125 (+0.000) P        PASS
fog5-70         0.176 (+0.000) P      0.115 (+0.000) P      0.286 (+0.000) P      0.125 (+0.000) P        PASS
amb1700         0.176 (+0.000) P      0.115 (+0.000) P      0.286 (+0.000) P      0.125 (+0.000) P        PASS
amb1400         0.176 (+0.000) P      0.115 (+0.000) P      0.286 (+0.000) P      0.125 (+0.000) P        PASS
amb1100         0.176 (+0.000) P      0.115 (+0.000) P      0.286 (+0.000) P      0.125 (+0.000) P        PASS
ev11.3          0.174 (-0.002) P      0.125 (+0.010) P      0.308 (+0.022) P      0.125 (+0.000) P        PASS
ev12.0          0.174 (-0.002) P      0.136 (+0.021) P      0.333 (+0.047) f      0.133 (+0.008) P        fail
atmos-off       0.335 (+0.159) f      0.300 (+0.185) P      0.361 (+0.075) f      0.314 (+0.189) P        fail
win-atmosfog      0.343 (+0.167) f      0.300 (+0.185) P      0.375 (+0.089) f      0.316 (+0.191) P        fail
fill-up         0.158 (-0.018) P      0.120 (+0.005) P      0.250 (-0.036) P      0.125 (+0.000) P        PASS
fill-down       0.190 (+0.014) P      0.115 (+0.000) P      0.312 (+0.026) P      0.129 (+0.004) P        PASS

### sky mask (gate for axes #7-#9; brief floor = 3.00% of frame)
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon
--------------------------------------------------------------------------------------------------
r0                 0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
fog8-45            0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
fog6-55            0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
fog5-70            0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
amb1700            0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
amb1400            0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
amb1100            0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
ev11.3             0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
ev12.0             0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
atmos-off          0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
win-atmosfog         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
fill-up            0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED
fill-down          0.00% REFUSED         0.00% REFUSED         0.00% REFUSED         0.00% REFUSED

REFUSED = mask below the 3% floor; axes #7-#9 are not scored on that plate at all
