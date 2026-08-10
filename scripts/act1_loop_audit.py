#!/usr/bin/env python
"""Act-1 loop audit — does every objective in act1.json have a producer in quest.rs?

The Act-1 loop has been broken twice by the same shape of bug: the story data
asks for something the engine has no code to deliver (or delivers under a
different name). `enter_zone` dialogue read `entered_{zone}` and nothing ever
wrote it; `on_defeat` dialogue was hardwired to `false`. Both are invisible to
`cargo check` and only show up as "the player walks to the gate and nothing
happens".

This audit is the contract test for that seam. It is STATIC: it reads
`assets/story/act1.json` and `client/src/quest.rs` and asserts that

  * every objective `kind` has a system that handles it,
  * every objective carries the data its producer needs (a position, a region
    that exists, a lore item that exists, a dialogue that completes it),
  * every dialogue trigger kind is handled by `dialogue_should_fire`, and the
    journal flag it reads has a writer,
  * the quest chain q1..q5 is reachable end to end.

It does NOT prove anything about a running binary — no marker printed here is a
runtime PASS. It proves the data and the code agree.

    python scripts/act1_loop_audit.py                     # exit 1 if any FAIL
    python scripts/act1_loop_audit.py <path/to/quest.rs>  # audit another revision

"""

import io
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STORY = os.path.join(ROOT, "assets", "story", "act1.json")
QUEST_RS = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "client", "src", "quest.rs")
# The ordering rules were split out of quest.rs so they could be tested without
# Bevy; both halves are the engine, so both are read as one source.
QUEST_RULES_RS = os.path.join(ROOT, "client", "src", "quest_rules.rs")

fails = []
warns = []


def emit(level, msg):
    print("AUDIT %-4s %s" % (level, msg))
    if level == "FAIL":
        fails.append(msg)
    elif level == "WARN":
        warns.append(msg)


def load():
    with io.open(STORY, encoding="utf-8") as f:
        data = json.load(f)
    with io.open(QUEST_RS, encoding="utf-8", errors="replace") as f:
        src = f.read()
    if os.path.exists(QUEST_RULES_RS):
        with io.open(QUEST_RULES_RS, encoding="utf-8", errors="replace") as f:
            src += "\n" + f.read()
    return data, src


def handled_objective_kinds(src):
    """Kinds the engine branches on, e.g. `if obj.kind != "defeat" { continue }`
    or `o.kind() == "defeat"` in the pure rules."""
    return set(re.findall(r'\.kind(?:\(\))?\s*[!=]=\s*"(\w+)"', src))


def handled_trigger_kinds(src):
    """Trigger kinds matched inside `fn dialogue_should_fire`."""
    m = re.search(r"pub fn dialogue_should_fire\b", src)
    if not m:
        emit("FAIL", "quest.rs has no `dialogue_should_fire` — dialogue triggers cannot be audited")
        return set()
    body = src[m.start():m.start() + 3000]
    return set(re.findall(r'"(\w+)"\s*=>', body))


def objective_completers(data):
    """objective id -> list of (dialogue id, path) that can complete it."""
    out = {}
    for dlg in data["dialogue"]:
        if dlg.get("completes_objective"):
            out.setdefault(dlg["completes_objective"], []).append((dlg["id"], "dialogue"))
        for ch in (dlg.get("choices") or []):
            if ch.get("completes_objective"):
                out.setdefault(ch["completes_objective"], []).append((dlg["id"], "choice:" + ch["id"]))
    return out


def audit_objectives(data, src, kinds, completers):
    regions = {r["id"] for r in data["regions"]}
    lore = {l["id"] for l in data.get("lore_items", [])}
    dialogues = {d["id"]: d for d in data["dialogue"]}

    for q in data["quests"]:
        for o in q["objectives"]:
            where = "%s/%s (%s)" % (q["id"], o["id"], o["kind"])
            kind = o["kind"]
            optional = o.get("optional", False)

            if kind not in kinds and kind != "listen":
                if optional:
                    emit("WARN", "%s — no producer in quest.rs, but optional" % where)
                else:
                    emit("FAIL", "%s — no system in quest.rs handles kind %r" % (where, kind))
                continue

            if kind in ("approach", "approach_entity", "place_block"):
                if not o.get("position"):
                    emit("FAIL", "%s — producer needs `position`, data has none" % where)
                else:
                    emit("ok  ", "%s — position %s r=%s" % (
                        where, o["position"], o.get("radius", "default")))

            elif kind == "reach_zone":
                if o["target"] not in regions:
                    emit("FAIL", "%s — target region %r is not in regions[]" % (where, o["target"]))
                else:
                    emit("ok  ", "%s — region %s exists" % (where, o["target"]))

            elif kind == "interact":
                if o["target"] not in lore:
                    emit("FAIL", "%s — target lore item %r is not in lore_items[]" % (where, o["target"]))
                else:
                    emit("ok  ", "%s — lore item %s exists" % (where, o["target"]))

            elif kind == "defeat":
                # check_kill_triggers credits ANY dead enemy; nothing in the code
                # names the story target, so the match is by objective order only.
                if o["target"] not in src:
                    emit("WARN", "%s — target %r appears nowhere in quest.rs; any enemy "
                                 "death is credited to it" % (where, o["target"]))
                else:
                    emit("ok  ", "%s — target %s named in quest.rs" % (where, o["target"]))

            elif kind == "listen":
                paths = completers.get(o["id"], [])
                if not paths:
                    emit("FAIL", "%s — nothing in act1.json completes this objective" % where)
                    continue
                reachable = []
                for dlg_id, how in paths:
                    dlg = dialogues[dlg_id]
                    tkind = dlg["trigger"]["type"]
                    if tkind in TRIGGER_KINDS:
                        reachable.append("%s via %s (%s)" % (dlg_id, how, tkind))
                    else:
                        emit("WARN", "%s — %s completes it but its trigger %r is not "
                                     "handled by dialogue_should_fire" % (where, dlg_id, tkind))
                if reachable:
                    emit("ok  ", "%s — %s" % (where, "; ".join(reachable)))
                else:
                    emit("FAIL", "%s — every completer sits behind an unhandled trigger" % where)


def audit_dialogue_triggers(data, src):
    regions = {r["id"] for r in data["regions"]}
    writes_zone_flag = "zone_flag(" in src and re.search(r"flags\.insert\(\s*flag", src) is not None

    for dlg in data["dialogue"]:
        t = dlg["trigger"]
        kind = t["type"]
        if kind not in TRIGGER_KINDS:
            # on_choice / on_interact are reached by other paths on purpose.
            if kind in ("on_choice", "on_interact"):
                emit("ok  ", "dialogue %s — %s reached via apply_choice/lore_interact" % (dlg["id"], kind))
            else:
                emit("FAIL", "dialogue %s — trigger %r not handled by dialogue_should_fire"
                     % (dlg["id"], kind))
            continue

        if kind == "enter_zone":
            zone = t.get("zone")
            if zone not in regions:
                emit("FAIL", "dialogue %s — enter_zone %r is not a region" % (dlg["id"], zone))
            elif not writes_zone_flag:
                emit("FAIL", "dialogue %s — enter_zone reads a journal flag no system writes"
                     % dlg["id"])
            else:
                emit("ok  ", "dialogue %s — enter_zone %s has a flag writer" % (dlg["id"], zone))

        elif kind == "on_defeat":
            if "defeated_targets" not in src:
                emit("FAIL", "dialogue %s — on_defeat has no kill log to read" % dlg["id"])
            else:
                emit("ok  ", "dialogue %s — on_defeat reads KillLog::defeated_targets" % dlg["id"])

        elif kind == "on_objective":
            obj_id = t.get("objective")
            owner = next((q for q in data["quests"] if q["id"] == dlg["quest"]), None)
            if owner is None:
                emit("FAIL", "dialogue %s — quest %r does not exist" % (dlg["id"], dlg["quest"]))
            elif obj_id not in [o["id"] for o in owner["objectives"]]:
                emit("FAIL", "dialogue %s — on_objective %r is not an objective of %s"
                     % (dlg["id"], obj_id, dlg["quest"]))
            else:
                emit("ok  ", "dialogue %s — on_objective %s belongs to %s"
                     % (dlg["id"], obj_id, dlg["quest"]))


def audit_chain(data):
    """Walk start_quest -> next/advance_to and check every quest is reached."""
    by_id = {q["id"]: q for q in data["quests"]}
    seen, cur = [], data["start_quest"]
    while cur and cur not in seen:
        seen.append(cur)
        q = by_id.get(cur)
        if q is None:
            emit("FAIL", "chain — quest %r referenced but not defined" % cur)
            return
        cur = q.get("next") or (q.get("rewards") or {}).get("advance_to")
    emit("ok  ", "chain — %s" % " -> ".join(seen))
    for qid in by_id:
        if qid not in seen:
            emit("FAIL", "chain — %s is never reached from %s" % (qid, data["start_quest"]))


def audit_world_rewards(data, src):
    """Every reward the data asks for should have somewhere to land."""
    for q in data["quests"]:
        rw = q.get("rewards") or {}
        for key in ("open_door", "activate_campfire"):
            val = rw.get(key)
            if not val:
                continue
            if "pending_world_rewards" not in src:
                emit("FAIL", "%s — rewards.%s=%r is parsed and dropped (no reward queue)"
                     % (q["id"], key, val))
            elif val not in src:
                emit("WARN", "%s — rewards.%s=%r is queued but no arm handles that id"
                     % (q["id"], key, val))
            else:
                emit("ok  ", "%s — rewards.%s=%s has a handler" % (q["id"], key, val))


def _consts(text):
    """`const NAME: f32 = 12.5;` -> {"NAME": 12.5} for one source file."""
    out = {}
    for name, val in re.findall(
        r"const\s+([A-Z0-9_]+)\s*:\s*f32\s*=\s*(-?\d+(?:\.\d+)?)\s*;", text
    ):
        out[name] = float(val)
    return out


def _route(text, name, consts):
    """Pull a `const NAME: [(f32, f32); N] = [ ... ];` array out, resolving
    any constant used as a coordinate."""
    m = re.search(
        r"const\s+%s\s*:\s*\[\(f32,\s*f32\);\s*\d+\]\s*=\s*\[(.*?)\n\s*\];" % name,
        text, re.S,
    )
    if not m:
        return None
    legs = []
    for a, b in re.findall(r"\(\s*([A-Za-z0-9_.\-]+)\s*,\s*([A-Za-z0-9_.\-]+)\s*\)", m.group(1)):
        try:
            legs.append((float(a) if _num(a) else consts[a], float(b) if _num(b) else consts[b]))
        except KeyError as e:
            return ("UNRESOLVED", str(e))
    return legs


def _num(tok):
    try:
        float(tok)
        return True
    except ValueError:
        return False


def audit_chaos_routes(data):
    """The runtime-proof scenarios (`client/src/quest_chaos.rs`) are geometry
    stated twice: once beside the walk it belongs to, once in the pure module
    the headless tests read. A drifted copy is a demo that walks into a wall and
    a log that blames the quest fix for it.

    quest_chaos.rs cannot import quest.rs or combat.rs (they pull in Bevy), so
    those numbers are mirrored with a comment pointing here. This is the check
    that comment promises.
    """
    chaos_path = os.path.join(ROOT, "client", "src", "quest_chaos.rs")
    combat_path = os.path.join(ROOT, "client", "src", "combat.rs")
    if not os.path.exists(chaos_path):
        emit("WARN", "quest_chaos.rs absent — runtime-proof scenarios not wired up")
        return
    with io.open(chaos_path, encoding="utf-8", errors="replace") as f:
        chaos = f.read()
    with io.open(QUEST_RS, encoding="utf-8", errors="replace") as f:
        quest = f.read()
    combat = ""
    if os.path.exists(combat_path):
        with io.open(combat_path, encoding="utf-8", errors="replace") as f:
            combat = f.read()

    cc = _consts(chaos)

    # ---- the shared route prefix --------------------------------------------
    shared = re.search(r"GATE_ROUTE_SHARED_LEGS:\s*usize\s*=\s*(\d+)", chaos)
    n = int(shared.group(1)) if shared else 5
    base = _route(quest, "GATE_ROUTE", _consts(quest))
    detour = _route(chaos, "GATE_ROUTE_ZONE_EARLY", cc)
    if not base or not detour or (isinstance(detour, tuple)):
        emit("FAIL", "chaos routes — could not parse GATE_ROUTE / GATE_ROUTE_ZONE_EARLY (%r)"
             % (detour,))
    elif base[:n] != detour[:n]:
        emit("FAIL", "chaos routes — zone_early's first %d legs drifted from GATE_ROUTE:\n"
                     "            quest.rs      %r\n            quest_chaos.rs %r"
             % (n, base[:n], detour[:n]))
    else:
        emit("ok  ", "chaos routes — zone_early shares GATE_ROUTE's first %d legs verbatim" % n)

    # ---- mirrored constants -------------------------------------------------
    if "const POST_LANE_Z: f32 = quest_chaos::LANE_Z;" in quest:
        emit("ok  ", "chaos routes — quest.rs POST_LANE_Z is quest_chaos::LANE_Z (one number)")
    else:
        emit("FAIL", "chaos routes — quest.rs POST_LANE_Z is not quest_chaos::LANE_Z; the "
                     "walk east and the chaos detour can drift apart")

    gp = re.search(r"const\s+GARREN_POS:\s*\(f32,\s*f32\)\s*=\s*\(\s*(-?[\d.]+)\s*,", quest)
    if gp and "GARREN_X" in cc:
        if abs(float(gp.group(1)) - cc["GARREN_X"]) > 1e-6:
            emit("FAIL", "chaos routes — GARREN_X=%s mirrors quest.rs GARREN_POS.x=%s"
                 % (cc["GARREN_X"], gp.group(1)))
        else:
            emit("ok  ", "chaos routes — GARREN_X matches quest.rs GARREN_POS.x=%s" % gp.group(1))

    leash = re.search(r"husk_leash:\s*(-?[\d.]+)", combat)
    if leash and "HUSK_LEASH" in cc:
        if abs(float(leash.group(1)) - cc["HUSK_LEASH"]) > 1e-6:
            emit("FAIL", "chaos routes — HUSK_LEASH=%s mirrors combat.rs husk_leash=%s"
                 % (cc["HUSK_LEASH"], leash.group(1)))
        else:
            emit("ok  ", "chaos routes — HUSK_LEASH matches combat.rs husk_leash=%s" % leash.group(1))

    span = re.search(r"patrol_origin\.x\)\.abs\(\)\s*>\s*(-?[\d.]+)", combat)
    if span and "HUSK_PATROL_SPAN" in cc:
        if abs(float(span.group(1)) - cc["HUSK_PATROL_SPAN"]) > 1e-6:
            emit("FAIL", "chaos routes — HUSK_PATROL_SPAN=%s mirrors combat.rs patrol turn=%s"
                 % (cc["HUSK_PATROL_SPAN"], span.group(1)))
        else:
            emit("ok  ", "chaos routes — HUSK_PATROL_SPAN matches combat.rs patrol turn=%s"
                 % span.group(1))

    # ---- the ambush stand against the shipped region -------------------------
    region = next((r for r in data["regions"] if r["id"] == "guard_post_east"), None)
    if region and "AMBUSH_X" in cc:
        x0 = float(region["bounds"]["x0"])
        if cc["AMBUSH_X"] >= x0:
            emit("FAIL", "chaos routes — AMBUSH_X=%s is inside guard_post_east (x0=%s); the "
                         "kill would score o1_east first and kill_early would prove nothing"
                 % (cc["AMBUSH_X"], x0))
        else:
            emit("ok  ", "chaos routes — AMBUSH_X=%s is west of guard_post_east.x0=%s"
                 % (cc["AMBUSH_X"], x0))

    # ---- the env lever the driver script types -------------------------------
    driver = os.path.join(ROOT, "scripts", "act1_runtime_proof.sh")
    m = re.search(r'ENV_VAR:\s*&str\s*=\s*"([A-Z_]+)"', chaos)
    if m and os.path.exists(driver):
        with io.open(driver, encoding="utf-8", errors="replace") as f:
            dsrc = f.read()
        if m.group(1) in dsrc:
            emit("ok  ", "chaos routes — driver sets %s, the lever the engine reads" % m.group(1))
        else:
            emit("FAIL", "chaos routes — engine reads %s but act1_runtime_proof.sh never sets it"
                 % m.group(1))


data, src = load()
TRIGGER_KINDS = handled_trigger_kinds(src)
KINDS = handled_objective_kinds(src)

print("AUDIT ---- act1 loop audit")
print("AUDIT ---- story=%s quests=%d dialogue=%d regions=%d lore=%d"
      % (os.path.relpath(STORY, ROOT), len(data["quests"]), len(data["dialogue"]),
         len(data["regions"]), len(data.get("lore_items", []))))
print("AUDIT ---- quest.rs handles objective kinds: %s" % ", ".join(sorted(KINDS)))
print("AUDIT ---- dialogue_should_fire handles triggers: %s" % ", ".join(sorted(TRIGGER_KINDS)))
print("AUDIT ----")

audit_chain(data)
audit_objectives(data, src, KINDS, objective_completers(data))
audit_dialogue_triggers(data, src)
audit_world_rewards(data, src)
audit_chaos_routes(data)

print("AUDIT ----")
print("AUDIT ---- %d FAIL, %d WARN" % (len(fails), len(warns)))
sys.exit(1 if fails else 0)
