#!/usr/bin/env python3
"""The TILT-DOWN golden recipe -- one source, two lanes (native env / web query).

`scripts/render_wide_hero.sh` is the ONLY place the CEO-approved tilt-down recipe
is written down. This module PARSES that file rather than restating it, so the web
URL and the native control can never drift apart: change the driver, both move.

Why this exists at all: until 2026-07-29 the wasm `read_cfg()` returned `None` for
every look knob, so the web build could not render the locked recipe and parity had
to be graded against whatever the baked defaults produced. `client/src/main.rs`
(`#[cfg(target_arch = "wasm32")] fn read_cfg`) now parses the query string key-by-key
-- `?hero&wide&cam=...` on web == the same `VOXELFORGE_*` set on native -- so the
right baseline is the approved TILT-DOWN frame on BOTH sides, not the bare default.

  python scripts/hero_recipe.py                      # show env / query / URL
  python scripts/hero_recipe.py --env                 # `VOXELFORGE_X=Y` lines for `env`
  python scripts/hero_recipe.py --url http://127.0.0.1:8080/
  python scripts/hero_recipe.py --check-url "<captured url>"   # exit 1 on drift

`--env` and `--query` are the SAME dict rendered two ways: the shell drivers
(render_native_control_v3.sh / verify_web_v3.sh) call them instead of restating
the knobs, so a hand-copied value can never drift on one side only. That drift is
not hypothetical -- the first v3 control was typed out by hand from the "Hero
tilt-B" note and silently dropped SHOULDER/DUST/BOUNCE, which cost p95 +19
(196.6, outside the 150..185 band) and micro -1.3 (the dust motes) against the
baseline. Numbers here, values nowhere else.
"""
import argparse
import re
import sys
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
RENDER_SH = SCRIPTS / "render_wide_hero.sh"
BASELINE_PNG = SCRIPTS.parent / "docs" / "assets" / "wide-hero-final.png"

# VOXELFORGE_<X> -> query key. Mirrors client/src/main.rs::read_cfg (wasm arm);
# if a knob is added there, add it here or the web URL silently drops it.
ENV_TO_QS = {
    "VOXELFORGE_HERO": "hero",
    "VOXELFORGE_WIDE": "wide",
    "VOXELFORGE_FGAPRON": "fgapron",
    "VOXELFORGE_CAM": "cam",
    "VOXELFORGE_SUN": "sun",
    "VOXELFORGE_DOF": "dof",
    "VOXELFORGE_FOG": "fog",
    "VOXELFORGE_DFOG": "dfog",
    "VOXELFORGE_EXPOSURE": "exposure",
    "VOXELFORGE_GRADE": "grade",
    "VOXELFORGE_AMBIENT": "ambient",
    "VOXELFORGE_AMBCOLOR": "ambcolor",
    "VOXELFORGE_EMISSIVE": "emissive",
    "VOXELFORGE_BLUESCALE": "bluescale",
    "VOXELFORGE_BOUNCE": "bounce",
    "VOXELFORGE_BOUNCE2": "bounce2",
    "VOXELFORGE_SHOULDER": "shoulder",
    "VOXELFORGE_DUST": "dust",
    "VOXELFORGE_SOFT": "soft",
    "VOXELFORGE_SEED": "seed",
    "VOXELFORGE_GRID": "grid",
}
# Bare-presence flags: `?wide` not `?wide=1` (main.rs `qs_flag` matches the key only).
FLAGS = {"hero", "wide", "fgapron"}
# Native-only: no filesystem in the browser (main.rs sets shot/map_* to None on wasm).
NATIVE_ONLY = {"VOXELFORGE_SHOT", "VOXELFORGE_MAP_LOAD", "VOXELFORGE_MAP_SAVE",
               "VOXELFORGE_PRESENT"}

# The shot binary always runs setup_hero, so render_wide_hero.sh never sets
# VOXELFORGE_HERO. The browser DOES need it -- `?hero` is what selects the scene
# (client/src/main.rs:153). Injected here so the URL is never scene-wrong (W0-B).
IMPLIED = {"VOXELFORGE_HERO": "1"}

ENV_LINE = re.compile(r"^\s*(VOXELFORGE_[A-Z0-9_]+)=(\S*?)\s*\\?\s*$")


def load_recipe(path=RENDER_SH):
    """env-var -> value, read straight out of the driver script."""
    recipe = dict(IMPLIED)
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        if line.lstrip().startswith("#"):
            continue
        m = ENV_LINE.match(line)
        if not m:
            continue
        key, val = m.group(1), m.group(2).strip('"').strip("'")
        if key in NATIVE_ONLY or val.startswith("$"):
            continue
        recipe[key] = val
    return recipe


def to_query(recipe):
    """env dict -> ordered [(qs_key, value)]; flags carry value None."""
    out = []
    for env_key, val in recipe.items():
        qs = ENV_TO_QS.get(env_key)
        if qs is None:
            print(f"[warn] {env_key} has no query-string counterpart in "
                  "main.rs::read_cfg -- the web build CANNOT receive it", file=sys.stderr)
            continue
        out.append((qs, None if qs in FLAGS else val))
    order = list(ENV_TO_QS.values())
    return sorted(out, key=lambda kv: order.index(kv[0]))


def query_string(recipe):
    return "&".join(k if v is None else f"{k}={v}" for k, v in to_query(recipe))


def parse_query(url_or_qs):
    """URL or bare query -> {key: value or ''}; mirrors main.rs qs_raw/qs_flag."""
    q = url_or_qs.split("?", 1)[1] if "?" in url_or_qs else url_or_qs
    q = q.split("#", 1)[0]
    got = {}
    for kv in q.split("&"):
        if not kv:
            continue
        k, _, v = kv.partition("=")
        got[k] = v.replace("%2C", ",").replace("%2c", ",")
    return got


def _num_eq(a, b):
    """`8` == `8.0`, `0.02,1.00,1.30` == `0.02,1,1.3` -- compare as numbers."""
    pa, pb = a.split(","), b.split(",")
    if len(pa) != len(pb):
        return False
    try:
        return all(abs(float(x) - float(y)) < 1e-6 for x, y in zip(pa, pb))
    except ValueError:
        return a == b


def diff_query(recipe, url_or_qs):
    """[] when the captured URL carries exactly this recipe, else one line per drift."""
    want = to_query(recipe)
    got = parse_query(url_or_qs)
    problems = []
    for k, v in want:
        if k not in got:
            problems.append(f"missing `{k}`" + ("" if v is None else f"={v}"))
        elif v is not None and not _num_eq(v, got[k]):
            problems.append(f"`{k}` is {got[k]!r}, recipe says {v!r}")
    # An extra key only matters if it is a LOOK knob -- `?fgapron` or `?soft=…`
    # changes the render, `?debug=1` does not. Harness params are not drift.
    knobs = set(ENV_TO_QS.values())
    extra = [k for k in got if k not in dict(want) and k in knobs]
    if extra:
        problems.append("extra look knob(s) the recipe does not set: " + ", ".join(sorted(extra)))
    return problems


def main():
    # LF, always -- this script's stdout is CONSUMED, not read. On Windows the
    # default text mode turns every "\n" into "\r\n", and `$(...)` / `mapfile -t`
    # strip "\n" but NOT "\r": `env` then exports `VOXELFORGE_AMBIENT=2800\r`.
    # Rust's `env_floats` calls `.trim()` per component so cam/sun/dof/grade/
    # ambcolor survive, but the SCALAR knobs go through a bare `v.parse()` which
    # rejects the "\r" and silently falls back to the baked default -- exposure,
    # ambient, bluescale, bounce, bounce2, shoulder, dust all quietly reverted.
    # Cost when this was live (2026-07-29): the native control came out warmth
    # -5.13 / blue -1.17 off a baseline the same driver reproduces to +0.02, and
    # W0-D read it as a look regression. The web lane had the same hole (the
    # trailing "\r" lands on the last query knob).
    sys.stdout.reconfigure(newline="\n")
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", metavar="BASE", help="print the full capture URL for BASE")
    ap.add_argument("--check-url", metavar="URL", help="verify a captured URL carries the recipe")
    ap.add_argument("--query", action="store_true", help="print the query string only")
    ap.add_argument("--env", action="store_true",
                    help="print `VOXELFORGE_X=Y` lines (feed to `env` for the native lane)")
    a = ap.parse_args()
    recipe = load_recipe()

    if a.check_url:
        problems = diff_query(recipe, a.check_url)
        if problems:
            print("RECIPE MISMATCH (W0-C):")
            for p in problems:
                print(f"  - {p}")
            print(f"\nexpected: ?{query_string(recipe)}")
            sys.exit(1)
        print("OK: the URL carries the locked tilt-down recipe")
        sys.exit(0)
    if a.query:
        print(query_string(recipe))
        return
    if a.env:
        # Emitted in to_query() order and filtered through the SAME map, so the
        # native lane can only ever carry knobs the web lane can also receive.
        qs_to_env = {v: k for k, v in ENV_TO_QS.items()}
        for qs, _v in to_query(recipe):
            print(f"{qs_to_env[qs]}={recipe[qs_to_env[qs]]}")
        return
    if a.url:
        sep = "" if a.url.endswith("?") else ("&" if "?" in a.url else "?")
        print(f"{a.url}{sep}{query_string(recipe)}")
        return

    print(f"recipe source : {RENDER_SH}")
    print(f"native baseline: {BASELINE_PNG}")
    print("\nnative (env):")
    for k, v in recipe.items():
        print(f"  {k}={v}")
    print(f"\nweb (query):\n  ?{query_string(recipe)}")


if __name__ == "__main__":
    main()
