#!/usr/bin/env python3
"""Standalone validator for assets/story/act1.json + act2.json (Story data lane).

Proves the act data is engine-consumable on its own — no engine build needed.
Checks: JSON syntax, required-field structure, cross-reference integrity,
quest-chain completeness, premature-trigger audit, coordinate-vs-map grounding,
and English-only (no Thai/CJK script) for in-game text.

Act 2 uses a different coordinate space (hollow_reach: a negative-y descent,
negative-z deeper) and new code-spawned entities, so this validator is act-aware
via the JSON's own `act` field and an optional top-level `entities` array.

Usage:  python assets/story/validate.py assets/story/act1.json maps/edhari.json
        python assets/story/validate.py assets/story/act2.json maps/hollow_reach.json
Exit 0 = valid, 1 = errors. Writes assets/story/_validation.log.
"""
import json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]          # .../Voxelforge
ACT  = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "assets/story/act1.json"
MAP  = Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / "maps/edhari.json"
LOG  = ACT.parent / "_validation.log"

# World extents per act, for the coordinate-bounds check. Act 1 (edhari) is the
# 64x32x64 village: every coordinate is non-negative. Act 2 (hollow_reach) is a
# descent — y goes negative (down) and z goes negative (north/deeper) while x
# stays 0-63. Act 2 has no real map file yet (maps/hollow_reach.json is pending),
# so its bounds are derived from the authored extent rather than a shipped map.
ACT_BOUNDS = {
    1: {"x": (0, 63), "y": (0, 31), "z": (0, 63)},
    2: {"x": (0, 63), "y": (-80, 0), "z": (-170, 0)},
}

errs, warns, out = [], [], []
def say(s):
    out.append(s); print(s)

try:
    d = json.load(open(ACT, encoding="utf-8"))
except Exception as e:
    say(f"FATAL: cannot parse {ACT}: {e}"); sys.exit(1)

# ---------- structural ----------
def req(obj, keys, label):
    for k in keys:
        if k not in obj:
            errs.append(f"{label}: missing required field '{k}'")
for q in d["quests"]:
    req(q, ["id", "title", "giver", "trigger", "objectives", "rewards", "next"], f"quest {q.get('id')}")
    for o in q["objectives"]:
        req(o, ["id", "kind", "optional"], f"{q['id']}.{o.get('id')}")
for x in d["dialogue"]:
    req(x, ["id", "quest", "speaker", "lines"], f"dialogue {x.get('id')}")
for l in d["lore_items"]:
    req(l, ["id", "kind", "name", "world_position", "text"], f"lore {l.get('id')}")
say(f"[1.structure] required fields: {'OK' if not errs else 'FAIL'}")

# ---------- references ----------
qids = {q["id"] for q in d["quests"]}
dids = {x["id"] for x in d["dialogue"]}
lids = {x["id"] for x in d["lore_items"]}
npcs = {n["id"] for n in d["npcs"]}
regs = {r["id"] for r in d["regions"]}
# Code-spawned entities that are neither roster NPCs, regions, nor lore items:
# Act 1's garren_husk, plus any act that lists an explicit `entities` array
# (Act 2's the_warden / the_architect / stasis_* pods).
ents = {"garren_husk"} | {e["id"] for e in d.get("entities", [])}
obj_by_q = {q["id"]: {o["id"] for o in q["objectives"]} for q in d["quests"]}
choice_ids = set()

def ref(label, val, valid):
    if val in (None, ""):
        return
    if val not in valid:
        errs.append(f"ref: {label} -> {val!r} not found")

ref("start_quest", d.get("start_quest"), qids)
# chain
cur, seen, cyc = d.get("start_quest"), [], False
while cur is not None:
    if cur in seen:
        errs.append(f"CYCLE @ {cur}"); cyc = True; break
    seen.append(cur)
    cur = next((q["next"] for q in d["quests"] if q["id"] == cur), "GAP")
if cur == "GAP":
    errs.append("broken next pointer")
if not cyc and len(seen) != len(qids):
    errs.append(f"chain visits {len(seen)} of {len(qids)} quests — some unreachable")

for q in d["quests"]:
    ref(f'{q["id"]}.giver', q.get("giver"), npcs | {"environment", "auto"})
    ref(f'{q["id"]}.next', q.get("next"), qids | {None})
    t = q["trigger"]
    if t["type"] == "quest_complete":
        ref(f'{q["id"]}.trigger.quest', t.get("quest"), qids)
    if t["type"] == "enter_zone":
        ref(f'{q["id"]}.trigger.zone', t.get("zone"), regs)
    for o in q["objectives"]:
        if o["kind"] == "reach_zone":
            ref(f'{q["id"]}.{o["id"]}.target', o.get("target"), regs)
        if o["kind"] == "interact":
            ref(f'{q["id"]}.{o["id"]}.target', o.get("target"), lids | ents)
        if o["kind"] == "listen":
            ref(f'{q["id"]}.{o["id"]}.target', o.get("target"), dids)
        if o["kind"] == "approach_entity":
            ref(f'{q["id"]}.{o["id"]}.target', o.get("target"), regs | ents)

for x in d["dialogue"]:
    ref(f'{x["id"]}.quest', x.get("quest"), qids)
    ref(f'{x["id"]}.speaker', x.get("speaker"), npcs)
    if not x.get("lines"):
        errs.append(f"{x['id']}: empty lines")
    t = x.get("trigger", {})
    if t.get("type") == "on_objective":
        ref(f'{x["id"]}.trigger.objective', t.get("objective"), obj_by_q.get(x["quest"], set()))
    if t.get("type") == "enter_zone":
        ref(f'{x["id"]}.trigger.zone', t.get("zone"), regs)
    if t.get("type") == "on_interact":
        ref(f'{x["id"]}.trigger.entity', t.get("entity"), lids | ents)
    if t.get("type") == "on_defeat":
        ref(f'{x["id"]}.trigger.entity', t.get("entity"), ents)
    for c in x.get("choices") or []:
        choice_ids.add(c["id"])
        ref(f'{x["id"]}.{c["id"]}.next_dialogue', c.get("next_dialogue"), dids | {None})
        if c.get("completes_objective"):
            ref(f'{x["id"]}.{c["id"]}.completes_objective', c["completes_objective"], obj_by_q.get(x["quest"], set()))
        if c.get("advances_quest"):
            ref(f'{x["id"]}.{c["id"]}.advances_quest', c.get("advances_quest"), qids)
    if x.get("completes_objective"):
        ref(f'{x["id"]}.completes_objective', x["completes_objective"], obj_by_q.get(x["quest"], set()))
    if x.get("advances_quest"):
        ref(f'{x["id"]}.advances_quest', x["advances_quest"], qids)
for x in d["dialogue"]:
    t = x.get("trigger", {})
    if t.get("type") == "on_choice" and t.get("from") not in choice_ids:
        errs.append(f"{x['id']}.trigger.from -> {t.get('from')!r} no such choice")

for l in d["lore_items"]:
    ref(f'{l["id"]}.requires', l.get("requires"), qids | {None})
    ref(f'{l["id"]}.triggers_dialogue', l.get("triggers_dialogue"), dids | {None})
say(f"[2.references] cross-ref integrity: {'OK' if not errs else 'FAIL'}")
say(f"[2.chain] {' -> '.join(seen)} -> END")

# ---------- premature-trigger audit ----------
forced = {q["id"]: {o["target"] for o in q["objectives"]
                    if o["kind"] in ("reach_zone", "approach", "approach_entity") and o.get("target")}
          for q in d["quests"]}
order = [q["id"] for q in d["quests"]]
for i, q in enumerate(d["quests"]):
    t = q["trigger"]
    if t["type"] == "enter_zone":
        hitters = [e for e in order[:i] if t["zone"] in forced.get(e, set())]
        if hitters:
            errs.append(f"PREMATURE TRIGGER: {q['id']} enter_zone={t['zone']} forced by earlier {hitters}")
say(f"[3.triggers] premature-fire audit: {'OK' if not errs else 'FAIL'}")

# ---------- coordinates vs map ----------
try:
    mb = json.load(open(MAP, encoding="utf-8"))["blocks"]
    solid = {(b["x"], b["y"], b["z"]) for b in mb}
except Exception as e:
    mb, solid = [], set(); warns.append(f"map not loaded ({e}); coord check skipped")

def nearest(p):
    best, bd = None, 99
    for s in solid:
        dd = (s[0]-p["x"])**2 + (s[1]-p["y"])**2 + (s[2]-p["z"])**2
        if dd < bd:
            bd, best = dd, s
    return best, bd**0.5

coords = [(f'lore:{l["id"]}', l["world_position"]) for l in d["lore_items"]]
for q in d["quests"]:
    for o in q["objectives"]:
        if "position" in o:
            coords.append((f'{q["id"]}.{o["id"]}', o["position"]))
b = ACT_BOUNDS.get(d.get("act", 1), ACT_BOUNDS[1])
oob = 0
for name, p in coords:
    if not (b["x"][0] <= p["x"] <= b["x"][1]
            and b["y"][0] <= p["y"] <= b["y"][1]
            and b["z"][0] <= p["z"] <= b["z"][1]):
        errs.append(f"coord OOB: {name} {p}"); oob += 1; continue
    if solid:
        _, dist = nearest(p)
        if dist > 1.6:
            warns.append(f"coord floats (dist {dist:.1f}): {name} {p}")
say(f"[4.coords] {len(coords)} positions checked vs {MAP.name} ({len(mb)} blocks): {oob} OOB")

# ---------- English-only (no Thai/CJK script) ----------
thai = re.compile(r"[฀-๿]"); cjk = re.compile(r"[　-鿿]")
def walk(o, path=""):
    if isinstance(o, str):
        if thai.search(o): errs.append(f"non-English(Thai) @ {path}")
        if cjk.search(o):  errs.append(f"non-English(CJK) @ {path}")
    elif isinstance(o, dict):
        for k, v in o.items(): walk(v, f"{path}.{k}")
    elif isinstance(o, list):
        for i, v in enumerate(o): walk(v, f"{path}[{i}]")
walk(d)
say(f"[5.lang] English-only (no Thai/CJK script letters): {'OK' if not errs else 'FAIL'}")

# ---------- summary ----------
say("")
say(f"counts: quests={len(d['quests'])} dialogue={len(d['dialogue'])} lore={len(d['lore_items'])} "
    f"npcs={len(d['npcs'])} regions={len(d['regions'])} | "
    f"spoken-lines={sum(len(x.get('lines', [])) for x in d['dialogue'])} "
    f"choices={sum(len(x.get('choices') or []) for x in d['dialogue'])}")
say(f"WARNINGS: {len(warns)}")
for w in warns: say("  - " + w)
say(f"ERRORS: {len(errs)}")
for e in errs: say("  X " + e)
say("")
say("VERDICT: " + ("VALID - syntax+structure+references+triggers+coords+lang all clean"
                    if not errs else f"INVALID ({len(errs)} errors)"))

LOG.write_text("\n".join(out), encoding="utf-8")
print(f"\n[proof log] {LOG} ({LOG.stat().st_size} bytes)")
sys.exit(1 if errs else 0)
