<!-- provenance:begin -->
# RECOVERED 2026-08-17, not re-measured. Every number below is the ORIGINAL 2026-08-16 measurement,
# replayed out of docs/assets/look/beauty/_grid.json (which survived the git clean -fd) through the
# shipped _flamingo_beauty_table.py by scripts/_pixel_grid_md_from_json.py. Binary: 97cb283,
# _flamingo_beauty_wt exe md5 f8e3a848..., 100,570,112 B, 2026-08-16 03:23. All-pixel mask.
# 44 plates (4 poses x 11 rungs) -- fill-up/fill-down were shot after this table was written and
# are not in the JSON, so they are absent here too. The re-shot 08-17 numbers are in _grid_reshoot.md.
<!-- provenance:end -->

REF = docs/assets/moodboard.png crop (514, 0, 1024, 1024)   poses=4  rungs=11  plates=44

### #2 value span L(p95-p5)   REF 209.5   TARGET >= 150
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                       108.6 f               108.7 f               106.7 f               114.7 f        fail
fog8-45          136.3 (+27.7) f       136.2 (+27.5) f       134.3 (+27.6) f       137.5 (+22.8) f        fail
fog6-55          129.4 (+20.8) f       128.6 (+19.9) f       127.3 (+20.6) f       132.5 (+17.8) f        fail
fog5-70          122.6 (+14.0) f       121.6 (+12.9) f       120.6 (+13.9) f       126.5 (+11.8) f        fail
amb1700           113.0 (+4.4) f        112.5 (+3.8) f        111.0 (+4.3) f        119.2 (+4.5) f        fail
amb1400           111.9 (+3.3) f        111.4 (+2.7) f        109.6 (+2.9) f        118.1 (+3.4) f        fail
amb1100           110.6 (+2.0) f        110.4 (+1.7) f        108.4 (+1.7) f        116.7 (+2.0) f        fail
ev11.3            95.2 (-13.4) f        96.9 (-11.8) f        94.1 (-12.6) f        97.1 (-17.6) f        fail
ev12.0            90.5 (-18.1) f        91.7 (-17.0) f        90.2 (-16.5) f        88.9 (-25.8) f        fail
atmos-off        127.7 (+19.1) f       126.0 (+17.3) f       125.5 (+18.8) f       133.3 (+18.6) f        fail
win-atmosfog       140.8 (+32.2) f       139.9 (+31.2) f       138.5 (+31.8) f       142.9 (+28.2) f        fail

### #3 shadow L p5   REF 23.7   TARGET 20-28
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                         5.0 f                 5.9 f                 6.9 f                 5.8 f        fail
fog8-45             5.1 (+0.1) f          5.9 (+0.0) f          6.9 (+0.0) f          5.9 (+0.1) f        fail
fog6-55             5.1 (+0.1) f          5.9 (+0.0) f          6.9 (+0.0) f          5.8 (+0.0) f        fail
fog5-70             5.1 (+0.1) f          5.9 (+0.0) f          6.9 (+0.0) f          5.8 (+0.0) f        fail
amb1700             5.1 (+0.1) f          5.9 (+0.0) f          6.9 (+0.0) f          5.9 (+0.1) f        fail
amb1400             5.1 (+0.1) f          5.9 (+0.0) f          6.9 (+0.0) f          5.9 (+0.1) f        fail
amb1100             5.1 (+0.1) f          5.9 (+0.0) f          6.9 (+0.0) f          5.9 (+0.1) f        fail
ev11.3              3.3 (-1.7) f          3.4 (-2.5) f          4.3 (-2.6) f          3.4 (-2.4) f        fail
ev12.0              3.0 (-2.0) f          3.0 (-2.9) f          3.1 (-3.8) f          2.9 (-2.9) f        fail
atmos-off           6.0 (+1.0) f          6.9 (+1.0) f          7.8 (+0.9) f          7.1 (+1.3) f        fail
win-atmosfog          6.0 (+1.0) f          6.8 (+0.9) f          7.8 (+0.9) f          7.1 (+1.3) f        fail

### #3 shadow % of white   REF 9.3%   TARGET 8-11%
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                        2.0% f                2.3% f                2.7% f                2.3% f        fail
fog8-45            2.0% (+0.0) f         2.3% (+0.0) f         2.7% (+0.0) f         2.3% (+0.0) f        fail
fog6-55            2.0% (+0.0) f         2.3% (+0.0) f         2.7% (+0.0) f         2.3% (+0.0) f        fail
fog5-70            2.0% (+0.0) f         2.3% (+0.0) f         2.7% (+0.0) f         2.3% (+0.0) f        fail
amb1700            2.0% (+0.0) f         2.3% (+0.0) f         2.7% (+0.0) f         2.3% (+0.0) f        fail
amb1400            2.0% (+0.0) f         2.3% (+0.0) f         2.7% (+0.0) f         2.3% (+0.0) f        fail
amb1100            2.0% (+0.0) f         2.3% (+0.0) f         2.7% (+0.0) f         2.3% (+0.0) f        fail
ev11.3             1.3% (-0.7) f         1.3% (-1.0) f         1.7% (-1.0) f         1.3% (-0.9) f        fail
ev12.0             1.2% (-0.8) f         1.2% (-1.1) f         1.2% (-1.5) f         1.1% (-1.1) f        fail
atmos-off          2.4% (+0.4) f         2.7% (+0.4) f         3.1% (+0.4) f         2.8% (+0.5) f        fail
win-atmosfog         2.4% (+0.4) f         2.7% (+0.4) f         3.1% (+0.4) f         2.8% (+0.5) f        fail

### #5 chroma span sat(p95-p5)   REF 0.776   TARGET >= 0.62
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                       0.957 P               0.952 P               0.846 P               0.947 P        PASS
fog8-45         0.578 (-0.379) f      0.577 (-0.375) f      0.525 (-0.321) f      0.621 (-0.326) P        fail
fog6-55         0.582 (-0.375) f      0.581 (-0.371) f      0.549 (-0.297) f      0.621 (-0.326) P        fail
fog5-70         0.641 (-0.316) P      0.619 (-0.333) f      0.648 (-0.198) P      0.655 (-0.292) P        fail
amb1700         0.957 (+0.000) P      0.945 (-0.007) P      0.846 (+0.000) P      0.947 (+0.000) P        PASS
amb1400         0.957 (+0.000) P      0.945 (-0.007) P      0.846 (+0.000) P      0.947 (+0.000) P        PASS
amb1100         0.957 (+0.000) P      0.952 (+0.000) P      0.846 (+0.000) P      0.947 (+0.000) P        PASS
ev11.3          0.941 (-0.016) P      0.933 (-0.019) P      0.846 (+0.000) P      0.938 (-0.009) P        PASS
ev12.0          0.955 (-0.002) P      0.947 (-0.005) P      0.875 (+0.029) P      0.947 (+0.000) P        PASS
atmos-off       0.724 (-0.233) P      0.754 (-0.198) P      0.696 (-0.150) P      0.723 (-0.224) P        PASS
win-atmosfog      0.253 (-0.704) f      0.299 (-0.653) f      0.272 (-0.574) f      0.279 (-0.668) f        fail

### #6 sat p5   REF 0.224   TARGET <= 0.32
rung               p1-east-level            p2-east-up          p3-east-down p8-abelev-eye-horizon   all poses
--------------------------------------------------------------------------------------------------------------
r0                       0.043 P               0.048 P               0.154 P               0.045 P        PASS
fog8-45         0.043 (+0.000) P      0.048 (+0.000) P      0.154 (+0.000) P      0.045 (+0.000) P        PASS
fog6-55         0.043 (+0.000) P      0.048 (+0.000) P      0.154 (+0.000) P      0.045 (+0.000) P        PASS
fog5-70         0.043 (+0.000) P      0.048 (+0.000) P      0.154 (+0.000) P      0.045 (+0.000) P        PASS
amb1700         0.043 (+0.000) P      0.048 (+0.000) P      0.154 (+0.000) P      0.045 (+0.000) P        PASS
amb1400         0.043 (+0.000) P      0.048 (+0.000) P      0.154 (+0.000) P      0.045 (+0.000) P        PASS
amb1100         0.043 (+0.000) P      0.048 (+0.000) P      0.154 (+0.000) P      0.045 (+0.000) P        PASS
ev11.3          0.059 (+0.016) P      0.067 (+0.019) P      0.154 (+0.000) P      0.062 (+0.017) P        PASS
ev12.0          0.045 (+0.002) P      0.053 (+0.005) P      0.125 (-0.029) P      0.053 (+0.008) P        PASS
atmos-off       0.276 (+0.233) P      0.231 (+0.183) P      0.304 (+0.150) P      0.250 (+0.205) P        PASS
win-atmosfog      0.280 (+0.237) P      0.231 (+0.183) P      0.312 (+0.158) P      0.250 (+0.205) P        PASS

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

REFUSED = mask below the 3% floor; axes #7-#9 are not scored on that plate at all
