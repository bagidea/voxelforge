#!/usr/bin/env python3
"""Per-CHANNEL shade budget of the v3 day rig, straight from look.rs's constants.

The existing table in look.rs (on IBL_NITS_DAY) budgets the shade terms in LUX
and shows every surface landing within 3% of its v2 total. That table is correct
and it is colour-blind: it never multiplies a term by the colour of the light
delivering it, which is the only place a B channel can go missing.

This does the same table again with the sRGB->linear channel weights in, so
"which term is spending the blue" is a number instead of an argument. Then it
knocks each 2026-08-15 constant back to its predecessor, one at a time, and
reports how much of the frame's B/R each one is worth.

Read-only, no engine, no frame. Every input is a literal from client/src/look.rs
with its line number next to it, so a second person can diff it against source.
"""
from __future__ import annotations

import math

# ── constants, quoted with their file:line in client/src/look.rs ────────────
KEY = (1.00, 0.92, 0.62)          # Hour::GOLDEN.key            :1713
AMBIENT = (0.96, 0.90, 0.48)      # Hour::GOLDEN.ambient        :1749
SKY_FILL = (0.62, 0.76, 1.00)     # Hour::GOLDEN.sky_fill       :1769
BOUNCE = (1.00, 0.78, 0.50)       # Hour::GOLDEN.bounce         :1774
SKY_FILL_LUX = 1400.0             # Hour::GOLDEN.sky_fill_lux   :1770
BOUNCE_LUX = 1100.0               # Hour::GOLDEN.bounce_lux     :1775
SKY_FILL_ELEV = 76.0              # SKY_FILL_ELEV               :1021
BOUNCE_ELEV = 28.0                # BOUNCE_ELEV                 :1032

AMBIENT_LUX_V3 = 620.0            # AMBIENT_LUX_V3              :624   (was 380)
IBL_NITS_DAY = 330.0              # IBL_NITS_DAY                :530   (was 440)
IBL_HORIZON_MIX = 0.68            # IBL_HORIZON_MIX             :593   (was 0.55)

PREV = {"AMBIENT_LUX_V3": 380.0, "IBL_NITS_DAY": 440.0, "IBL_HORIZON_MIX": 0.55}


def to_lin(c: float) -> float:
    """sRGB -> linear, the transfer Color::srgb(..).to_linear() applies."""
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def lin3(c) -> tuple:
    return tuple(to_lin(x) for x in c)


def scale(c, k: float) -> tuple:
    return tuple(x * k for x in c)


def add(*cs) -> tuple:
    return tuple(sum(v) for v in zip(*cs))


def lerp(a, b, t: float) -> tuple:
    return tuple(x + (y - x) * t for x, y in zip(a, b))


def budget(amb_lux: float, ibl_nits: float, mix: float) -> dict:
    """Per-channel linear irradiance on the four surfaces look.rs tabulates."""
    a = scale(lin3(AMBIENT), amb_lux)
    top, bot = lin3(SKY_FILL), lin3(BOUNCE)
    mid = lerp(top, bot, mix)

    e_sky_h = SKY_FILL_LUX * math.sin(math.radians(SKY_FILL_ELEV))   # 1358
    e_sky_v = SKY_FILL_LUX * math.cos(math.radians(SKY_FILL_ELEV))   #  339
    e_bnc_v = BOUNCE_LUX * math.cos(math.radians(BOUNCE_ELEV))       #  971
    e_bnc_h = BOUNCE_LUX * math.sin(math.radians(BOUNCE_ELEV))       #  516

    # E = pi * L * (map colour). A face looking UP integrates the top half, a
    # face looking SIDEWAYS the horizon band, a face looking DOWN the bottom.
    ibl_up = scale(top, math.pi * ibl_nits)
    ibl_side = scale(mid, math.pi * ibl_nits)
    ibl_down = scale(bot, math.pi * ibl_nits)

    return {
        "shaded ground (up)": add(a, scale(top, e_sky_h), ibl_up),
        "wall, shadow side": add(a, scale(top, e_sky_v), ibl_side),
        "wall, sun side (shade)": add(a, scale(bot, e_bnc_v), ibl_side),
        "underside": add(a, scale(bot, e_bnc_h), ibl_down),
    }


def show(label: str, b: dict) -> None:
    print(f"\n{label}")
    print(f"  {'surface':<24} {'R':>9} {'G':>9} {'B':>9}   {'B/R':>6}  {'R-B':>9}")
    for k, (r, g, bl) in b.items():
        print(f"  {k:<24} {r:9.1f} {g:9.1f} {bl:9.1f}   {bl / r:6.3f}  {r - bl:9.1f}")
    tot = add(*b.values())
    print(f"  {'-- all four summed':<24} {tot[0]:9.1f} {tot[1]:9.1f} {tot[2]:9.1f}"
          f"   {tot[2] / tot[0]:6.3f}  {tot[0] - tot[2]:9.1f}")
    return tot[2] / tot[0]


def main() -> int:
    print("Per-channel LINEAR irradiance of the v3 day shade rig")
    print("(the sun key is excluded on purpose: these are the SHADED surfaces,")
    print(" which is where the reference has its cool pixels and this frame has none)")

    ship = budget(AMBIENT_LUX_V3, IBL_NITS_DAY, IBL_HORIZON_MIX)
    base = show("SHIPPED  (ambient 620 · ibl 330 · mix 0.68)", ship)

    print("\n\nKNOCK-BACK: each 2026-08-15 constant returned to its predecessor,")
    print("one at a time, everything else shipped. Delta is on the summed B/R.\n")
    rows = []
    for name, prev in PREV.items():
        kw = {"amb_lux": AMBIENT_LUX_V3, "ibl_nits": IBL_NITS_DAY, "mix": IBL_HORIZON_MIX}
        kw[{"AMBIENT_LUX_V3": "amb_lux",
            "IBL_NITS_DAY": "ibl_nits",
            "IBL_HORIZON_MIX": "mix"}[name]] = prev
        b = budget(**kw)
        tot = add(*b.values())
        got = tot[2] / tot[0]
        rows.append((name, prev, got, got - base))
    rows.sort(key=lambda r: -r[3])
    print(f"  {'constant':<20} {'shipped':>9} {'->':^4} {'was':>7}   {'B/R':>7}  {'delta':>8}")
    cur = {"AMBIENT_LUX_V3": AMBIENT_LUX_V3, "IBL_NITS_DAY": IBL_NITS_DAY,
           "IBL_HORIZON_MIX": IBL_HORIZON_MIX}
    for name, prev, got, d in rows:
        print(f"  {name:<20} {cur[name]:9.2f} {'->':^4} {prev:7.2f}   {got:7.3f}  {d:+8.3f}")

    both = budget(PREV["AMBIENT_LUX_V3"], PREV["IBL_NITS_DAY"], IBL_HORIZON_MIX)
    tb = add(*both.values())
    print(f"\n  the PAIRED move undone (620->380 AND 330->440, mix left at 0.68):"
          f"  B/R {tb[2] / tb[0]:.3f}  ({tb[2] / tb[0] - base:+.3f})")
    allp = budget(**{"amb_lux": PREV["AMBIENT_LUX_V3"], "ibl_nits": PREV["IBL_NITS_DAY"],
                     "mix": PREV["IBL_HORIZON_MIX"]})
    ta = add(*allp.values())
    print(f"  all three undone (the 2026-08-14 rig):"
          f"                      B/R {ta[2] / ta[0]:.3f}  ({ta[2] / ta[0] - base:+.3f})")

    print("\n\nCOLOUR OF EACH TERM ON ITS OWN (linear channel ratio, colour only):")
    for nm, c in (("Hour::GOLDEN.ambient  :1749", AMBIENT),
                  ("Hour::GOLDEN.sky_fill :1769", SKY_FILL),
                  ("Hour::GOLDEN.bounce   :1774", BOUNCE),
                  ("Hour::GOLDEN.key      :1713", KEY)):
        r, g, b = lin3(c)
        print(f"  {nm:<28} srgb {c}  linear B/R {b / r:6.3f}")
    mid = lerp(lin3(SKY_FILL), lin3(BOUNCE), IBL_HORIZON_MIX)
    mid55 = lerp(lin3(SKY_FILL), lin3(BOUNCE), 0.55)
    print(f"  {'IBL horizon band @ mix 0.68':<28} {'':>26}  linear B/R {mid[2] / mid[0]:6.3f}")
    print(f"  {'IBL horizon band @ mix 0.55':<28} {'':>26}  linear B/R {mid55[2] / mid55[0]:6.3f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
