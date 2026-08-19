<!-- provenance:begin -->
# RE-SHOT 2026-08-17 on a NEW binary. These are NOT the numbers of the plates git clean -fd destroyed.
# Binary: HEAD 2262f71 + client/src/anim.rs from the main working tree (HEAD alone does not compile --
# see docs/pixel-plate-recovery-2026-08-17.md section 2), _pixel_recover_wt exe md5 e850f2c0...,
# 100,991,488 B, 2026-08-17 14:22. All-pixel mask, 52 plates (4 poses x 13 rungs).
# Direct comparator: _grid.md (same mask, same grader, the original 08-16 numbers, 44 of these plates).
<!-- provenance:end -->

REF = docs/assets/moodboard.png crop (514, 0, 1024, 1024)   poses=4  rungs=13  plates=52

### #2 value span L(p95-p5)   REF 209.5   TARGET >= 150
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                       112.9 f               113.3 f               111.1 f               118.3 f        fail
fog8-45          137.0 (+24.1) f       137.0 (+23.7) f       135.2 (+24.1) f       138.2 (+19.9) f        fail
fog6-55          130.9 (+18.0) f       130.4 (+17.1) f       129.1 (+18.0) f       133.7 (+15.4) f        fail
fog5-70          125.3 (+12.4) f       124.4 (+11.1) f       123.3 (+12.2) f       128.4 (+10.1) f        fail
amb1700           116.8 (+3.9) f        116.9 (+3.6) f        115.0 (+3.9) f        122.0 (+3.7) f        fail
amb1400           115.9 (+3.0) f        116.0 (+2.7) f        114.0 (+2.9) f        121.0 (+2.7) f        fail
amb1100           114.7 (+1.8) f        115.0 (+1.7) f        112.9 (+1.8) f        120.2 (+1.9) f        fail
ev11.3           100.7 (-12.2) f       101.7 (-11.6) f        99.1 (-12.0) f       102.6 (-15.7) f        fail
ev12.0            96.1 (-16.8) f        97.1 (-16.2) f        95.2 (-15.9) f        94.7 (-23.6) f        fail
atmos-off        129.7 (+16.8) f       128.4 (+15.1) f       127.9 (+16.8) f       134.5 (+16.2) f        fail
win-atmosfog       141.1 (+28.2) f       140.5 (+27.2) f       139.2 (+28.1) f       142.8 (+24.5) f        fail
fill-up           113.0 (+0.1) f        112.8 (-0.5) f        110.9 (-0.2) f        117.9 (-0.4) f        fail
fill-down         112.9 (+0.0) f        113.5 (+0.2) f        111.3 (+0.2) f        118.5 (+0.2) f        fail

### #3 shadow L p5   REF 23.7   TARGET 20-28
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                         5.2 f                 5.7 f                 6.8 f                 5.7 f        fail
fog8-45             5.2 (+0.0) f          5.7 (+0.0) f          6.8 (+0.0) f          5.8 (+0.1) f        fail
fog6-55             5.2 (+0.0) f          5.7 (+0.0) f          6.8 (+0.0) f          5.7 (+0.0) f        fail
fog5-70             5.2 (+0.0) f          5.7 (+0.0) f          6.8 (+0.0) f          5.7 (+0.0) f        fail
amb1700             5.2 (+0.0) f          5.7 (+0.0) f          6.8 (+0.0) f          5.8 (+0.1) f        fail
amb1400             5.2 (+0.0) f          5.7 (+0.0) f          6.8 (+0.0) f          5.8 (+0.1) f        fail
amb1100             5.2 (+0.0) f          5.7 (+0.0) f          6.8 (+0.0) f          5.7 (+0.0) f        fail
ev11.3              3.3 (-1.9) f          3.6 (-2.1) f          4.4 (-2.4) f          3.4 (-2.3) f        fail
ev12.0              3.0 (-2.2) f          3.1 (-2.6) f          3.2 (-3.6) f          2.6 (-3.1) f        fail
atmos-off           6.1 (+0.9) f          6.6 (+0.9) f          7.6 (+0.8) f          7.1 (+1.4) f        fail
win-atmosfog          6.1 (+0.9) f          6.4 (+0.7) f          7.5 (+0.7) f          7.2 (+1.5) f        fail
fill-up             5.2 (+0.0) f          6.2 (+0.5) f          7.1 (+0.3) f          6.1 (+0.4) f        fail
fill-down           5.2 (+0.0) f          5.5 (-0.2) f          6.6 (-0.2) f          5.5 (-0.2) f        fail

### #3 shadow % of white   REF 9.3%   TARGET 8-11%
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                        2.0% f                2.2% f                2.7% f                2.2% f        fail
fog8-45            2.0% (+0.0) f         2.2% (+0.0) f         2.7% (+0.0) f         2.3% (+0.0) f        fail
fog6-55            2.0% (+0.0) f         2.2% (+0.0) f         2.7% (+0.0) f         2.2% (+0.0) f        fail
fog5-70            2.0% (+0.0) f         2.2% (+0.0) f         2.7% (+0.0) f         2.2% (+0.0) f        fail
amb1700            2.0% (+0.0) f         2.2% (+0.0) f         2.7% (+0.0) f         2.3% (+0.0) f        fail
amb1400            2.0% (+0.0) f         2.2% (+0.0) f         2.7% (+0.0) f         2.3% (+0.0) f        fail
amb1100            2.0% (+0.0) f         2.2% (+0.0) f         2.7% (+0.0) f         2.2% (+0.0) f        fail
ev11.3             1.3% (-0.7) f         1.4% (-0.8) f         1.7% (-0.9) f         1.3% (-0.9) f        fail
ev12.0             1.2% (-0.9) f         1.2% (-1.0) f         1.3% (-1.4) f         1.0% (-1.2) f        fail
atmos-off          2.4% (+0.4) f         2.6% (+0.4) f         3.0% (+0.3) f         2.8% (+0.5) f        fail
win-atmosfog         2.4% (+0.4) f         2.5% (+0.3) f         2.9% (+0.3) f         2.8% (+0.6) f        fail
fill-up            2.0% (+0.0) f         2.4% (+0.2) f         2.8% (+0.1) f         2.4% (+0.2) f        fail
fill-down          2.0% (+0.0) f         2.2% (-0.1) f         2.6% (-0.1) f         2.2% (-0.1) f        fail

### #5 chroma span sat(p95-p5)   REF 0.776   TARGET >= 0.62
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                       0.833 P               0.875 P               0.727 P               0.854 P        PASS
fog8-45         0.500 (-0.333) f      0.560 (-0.315) f      0.442 (-0.285) f      0.575 (-0.279) f        fail
fog6-55         0.512 (-0.321) f      0.567 (-0.308) f      0.452 (-0.275) f      0.584 (-0.270) f        fail
fog5-70         0.531 (-0.302) f      0.575 (-0.300) f      0.505 (-0.222) f      0.585 (-0.269) f        fail
amb1700         0.833 (+0.000) P      0.874 (-0.001) P      0.727 (+0.000) P      0.842 (-0.012) P        PASS
amb1400         0.833 (+0.000) P      0.868 (-0.007) P      0.727 (+0.000) P      0.843 (-0.011) P        PASS
amb1100         0.833 (+0.000) P      0.875 (+0.000) P      0.727 (+0.000) P      0.849 (-0.005) P        PASS
ev11.3          0.833 (+0.000) P      0.871 (-0.004) P      0.737 (+0.010) P      0.870 (+0.016) P        PASS
ev12.0          0.850 (+0.017) P      0.867 (-0.008) P      0.750 (+0.023) P      0.870 (+0.016) P        PASS
atmos-off       0.658 (-0.175) P      0.668 (-0.207) P      0.634 (-0.093) P      0.627 (-0.227) P        PASS
win-atmosfog      0.224 (-0.609) f      0.267 (-0.608) f      0.224 (-0.503) f      0.250 (-0.604) f        fail
fill-up         0.844 (+0.011) P      0.871 (-0.004) P      0.762 (+0.035) P      0.850 (-0.004) P        PASS
fill-down       0.818 (-0.015) P      0.875 (+0.000) P      0.694 (-0.033) P      0.852 (-0.002) P        PASS

### #6 sat p5   REF 0.224   TARGET <= 0.32
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                       0.167 P               0.118 P               0.273 P               0.130 P        PASS
fog8-45         0.167 (+0.000) P      0.120 (+0.002) P      0.273 (+0.000) P      0.130 (+0.000) P        PASS
fog6-55         0.167 (+0.000) P      0.118 (+0.000) P      0.273 (+0.000) P      0.130 (+0.000) P        PASS
fog5-70         0.167 (+0.000) P      0.118 (+0.000) P      0.273 (+0.000) P      0.130 (+0.000) P        PASS
amb1700         0.167 (+0.000) P      0.118 (+0.000) P      0.273 (+0.000) P      0.130 (+0.000) P        PASS
amb1400         0.167 (+0.000) P      0.118 (+0.000) P      0.273 (+0.000) P      0.130 (+0.000) P        PASS
amb1100         0.167 (+0.000) P      0.118 (+0.000) P      0.273 (+0.000) P      0.130 (+0.000) P        PASS
ev11.3          0.167 (+0.000) P      0.129 (+0.011) P      0.263 (-0.010) P      0.130 (+0.000) P        PASS
ev12.0          0.150 (-0.017) P      0.133 (+0.015) P      0.250 (-0.023) P      0.130 (+0.000) P        PASS
atmos-off       0.342 (+0.175) f      0.304 (+0.186) P      0.366 (+0.093) f      0.316 (+0.186) P        fail
win-atmosfog      0.348 (+0.181) f      0.304 (+0.186) P      0.376 (+0.103) f      0.321 (+0.191) f        fail
fill-up         0.156 (-0.011) P      0.121 (+0.003) P      0.238 (-0.035) P      0.129 (-0.001) P        PASS
fill-down       0.182 (+0.015) P      0.118 (+0.000) P      0.306 (+0.033) P      0.133 (+0.003) P        PASS

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
