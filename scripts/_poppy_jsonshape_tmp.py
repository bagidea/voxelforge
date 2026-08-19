import json

d = json.load(open("docs/assets/artgap/artgap.json", encoding="utf-8"))
KEYS = [
    "sky_ground_ratio",
    "sky_L_range",
    "sky_void_pct",
    "sky_hue_span_deg",
    "crush_pct",
    "sky_blown_pct",
]
fr = d["frames"]
print("frames type:", type(fr).__name__, "len", len(fr))
items = fr.items() if isinstance(fr, dict) else [(None, v) for v in fr]
for k, v in items:
    print(" key:", repr(k), "| file field:", v.get("file"))
    print("   ", {x: v.get(x) for x in KEYS})
print()
print("REF file:", d["ref"].get("file"))
print("REF:", {x: d["ref"].get(x) for x in KEYS})
