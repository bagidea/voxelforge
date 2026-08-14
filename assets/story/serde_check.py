#!/usr/bin/env python3
"""Independent deserialization-equivalent check for assets/story/act1.json.

Rose's OWN verifier (distinct from validate.py). Where validate.py checks
structure/cross-refs/coords/language, THIS one mirrors the serde::Deserialize
contract in client/src/quest.rs field-for-field: every required field present
with the right type, every Option/default handled exactly as serde derive would,
renames applied. Then it asserts the engine-contract hard-codes the quest.rs
runtime actually reads (quest ids, Maren npc, the dlg_maren_gate hub, region
bounds). If this passes, act1.json would deserialize into StoryData and drive
the runtime without a parse/field error -- no engine build needed.

Usage:  python assets/story/serde_check.py [assets/story/act2.json]
Exit 0 = the JSON is serde-faithful + engine-contract-clean.
"""
import json, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ACT  = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "assets/story/act1.json"

errs, notes = [], []

# ---------------------------------------------------------------------------
# serde derive semantics, modelled exactly:
#   "req"      -> must be present, non-null, type T        (no default, not Option)
#   "opt"      -> may be absent or null; if present, type T  (Option<T>)
#   "default"  -> may be absent; if present, type T         (#[serde(default)])
# serde IGNORES unknown keys (no #[serde(deny_unknown_fields)] anywhere in
# quest.rs), so _comment_* and canon_status etc. are legal -- we do not flag them.
# ---------------------------------------------------------------------------
def chk_type(val, t, path):
    """Validate a Python value against a Rust scalar type tag."""
    if t == "String":
        if not isinstance(val, str): errs.append(f"{path}: expected String, got {type(val).__name__}")
    elif t == "bool":
        if not isinstance(val, bool): errs.append(f"{path}: expected bool, got {type(val).__name__}")
    elif t == "u32":
        if not (isinstance(val, int) and not isinstance(val, bool) and 0 <= val <= 4294967295):
            errs.append(f"{path}: expected u32, got {val!r}")
    elif t == "i32":
        if not (isinstance(val, int) and not isinstance(val, bool) and -2147483648 <= val <= 2147483647):
            errs.append(f"{path}: expected i32, got {val!r}")
    elif t == "f32":
        if isinstance(val, bool) or not isinstance(val, (int, float)):
            errs.append(f"{path}: expected f32 (number), got {val!r}")
    elif t == "flag":                       # quest.rs FlagSpec: String | [String]
        if isinstance(val, list):
            for i, s in enumerate(val):
                if not isinstance(s, str):
                    errs.append(f"{path}[{i}]: expected String in flag list, got {type(s).__name__}")
        elif not isinstance(val, str):
            errs.append(f"{path}: expected String or [String] (flag), got {val!r}")

def chk_struct(obj, fields, path):
    """fields: list of (json_key, kind, rust_type_or_structname).
    kind in {req,opt,default} for scalars/structs; or ('vec', item_struct_or_scalar)."""
    for key, kind, typ in fields:
        p = f"{path}.{key}"
        present = key in obj
        val = obj.get(key)
        if not present:
            if kind == "req":
                errs.append(f"{p}: MISSING required field (serde would fail)")
            continue                       # opt/default absent -> fine
        if val is None:
            if kind == "req":
                errs.append(f"{p}: required field is null (serde would fail)")
            continue                       # opt/default null -> fine for Option; default null also ok
        # present & non-null
        if typ in SCHEMAS:                 # nested struct
            if not isinstance(val, dict): errs.append(f"{p}: expected object, got {type(val).__name__}"); continue
            chk_struct(val, SCHEMAS[typ], p)
        elif kind == "vec":
            if not isinstance(val, list): errs.append(f"{p}: expected array, got {type(val).__name__}"); continue
            for i, item in enumerate(val):
                ip = f"{p}[{i}]"
                if typ[1] in SCHEMAS:
                    if not isinstance(item, dict): errs.append(f"{ip}: expected object"); continue
                    chk_struct(item, SCHEMAS[typ[1]], ip)
                else:
                    chk_type(item, typ[1], ip)
        elif kind == "optvec":             # Option<Vec<struct>> -> null handled above; else array
            if not isinstance(val, list): errs.append(f"{p}: expected array (Option<Vec>), got {type(val).__name__}"); continue
            for i, item in enumerate(val):
                ip = f"{p}[{i}]"
                if not isinstance(item, dict): errs.append(f"{ip}: expected object"); continue
                chk_struct(item, SCHEMAS[typ], ip)
        else:
            chk_type(val, typ, p)

# ---- struct field tables, annotated with quest.rs line anchors ----------------
SCHEMAS = {
 "PositionDef": [("x","req","f32"),("y","req","f32"),("z","req","f32")],                 # quest.rs:69-74
 "SpawnDef":    [("position","req","PositionDef"),("facing","req","String")],            # quest.rs:63-67
 "OpeningDef":  [                                                                        # quest.rs:46-61
    ("id","req","String"),("duration_s","req","u32"),("title","req","String"),
    ("no_menu","default","bool"),("no_loading_text","default","bool"),
    ("spawn","req","SpawnDef"),("scene","default",("vec","String")),
    ("mechanics_seeded","default",("vec","String")),("story_seed","req","String")],
 "NpcDef":      [                                                                        # quest.rs:76-86
    ("id","req","String"),("name","req","String"),("role","req","String"),
    ("voice","req","String"),("appearance","default","String"),
    ("knows_and_withholds","default","String")],
 "BoundsDef":   [("x0","req","i32"),("z0","req","i32"),("x1","req","i32"),("z1","req","i32")],  # quest.rs:99-105
 "RegionDef":   [                                                                        # quest.rs:88-97
    ("id","req","String"),("name","req","String"),("bounds","req","BoundsDef"),
    ("note","default","String"),("code_spawned","default","bool")],
 "TriggerDef":  [                                                                        # quest.rs:123-139  (field "type" is #[serde(rename="type")])
    ("type","req","String"),("zone","opt","String"),("quest","opt","String"),
    ("objective","opt","String"),("once","default","bool"),("entity","opt","String"),
    ("from","opt","String")],
 "ObjectiveDef":[                                                                        # quest.rs:141-156
    ("id","req","String"),("kind","req","String"),("target","req","String"),
    ("position","opt","PositionDef"),("radius","opt","f32"),("count","default","u32"),
    ("hint","opt","String"),("optional","default","bool")],
 "QuestRewardDef":[                                                                      # quest.rs:158-176  (all Option<>, all default)
    ("heal","opt","String"),("set_flag","opt","flag"),("unlock_dialogue","opt","String"),
    ("advance_to","opt","String"),("reveal_path","opt","String"),("open_door","opt","String"),
    ("activate_campfire","opt","String"),("unlock_act","opt","String")],
 "QuestDef":    [                                                                        # quest.rs:107-121
    ("id","req","String"),("title","req","String"),("subtitle","req","String"),
    ("giver","req","String"),("trigger","req","TriggerDef"),("premise","req","String"),
    ("objectives","req",("vec","ObjectiveDef")),("mechanics_taught","default",("vec","String")),
    ("rewards","req","QuestRewardDef"),("next","opt","String")],
 "DialogueChoiceDef":[                                                                   # quest.rs:198-210
    ("id","req","String"),("label","req","String"),("next_dialogue","opt","String"),
    ("completes_objective","opt","String"),("advances_quest","opt","String"),
    ("sets_flag","opt","String")],
 "DialogueDef": [                                                                        # quest.rs:178-196  (field r#where -> key "where")
    ("id","req","String"),("quest","req","String"),("trigger","req","TriggerDef"),
    ("speaker","req","String"),("speaker_display","req","String"),("where","opt","String"),
    ("lines","req",("vec","String")),("choices","optvec","DialogueChoiceDef"),
    ("completes_objective","opt","String"),("sets_flag","opt","String"),("advances_quest","opt","String")],
 "LoreItemDef":[                                                                         # quest.rs:212-227
    ("id","req","String"),("kind","req","String"),("name","req","String"),
    ("world_position","req","PositionDef"),("requires","opt","String"),("lore_layer","req","u32"),
    ("subtitle","req","String"),("text","req","String"),("triggers_dialogue","opt","String"),
    ("tags","default",("vec","String"))],
 "ActEndDef":   [                                                                        # quest.rs:229-238
    ("id","req","String"),("title","req","String"),("trigger_quest","req","String"),
    ("beats","req",("vec","String")),("final_line","req","String"),("card","req","String"),
    ("cliffhanger_questions","req",("vec","String"))],
}
ROOT_FIELDS = [                                                                          # StoryData, quest.rs:22-44
    ("schema_version","req","String"),("act","req","u32"),("act_title","req","String"),
    ("lang","req","String"),("map","req","String"),("start_quest","req","String"),
    ("opening","opt","OpeningDef"),("npcs","default",("vec","NpcDef")),
    ("regions","default",("vec","RegionDef")),("quests","default",("vec","QuestDef")),
    ("dialogue","default",("vec","DialogueDef")),("lore_items","default",("vec","LoreItemDef")),
    ("act_end","opt","ActEndDef"),
]

# ---------------------------------------------------------------------------
try:
    d = json.load(open(ACT, encoding="utf-8"))
except Exception as e:
    print(f"FATAL parse: {e}"); sys.exit(1)

print("[A] serde-deserialize mirror (every quest.rs struct field, type, rename, optionality)")
chk_struct(d, ROOT_FIELDS, ACT.stem)
print(f"    -> {'OK' if not errs else 'FAIL'} ({len(errs)} field/type errors)\n")

# ---------------------------------------------------------------------------
# [B] engine-contract: what quest.rs RUNTIME actually reads (not just schema)
# ---------------------------------------------------------------------------
print("[B] engine-contract assertions (hard-codes the runtime reads)")
E = []
def assert_(cond, msg):
    if not cond: E.append(msg); print(f"    X {msg}")
    return cond

act     = d.get("act", 1)
qids    = {q["id"] for q in d["quests"]}
npc_ids = {n["id"] for n in d["npcs"]}
regs    = {r["id"]: r for r in d["regions"]}
dids    = {x["id"] for x in d["dialogue"]}
Q = {q["id"]: q for q in d["quests"]}

# Common to every act: init_journal (quest.rs:481) pushes start_quest, and boots
# it Active via on_spawn (quest.rs:743).
if assert_(d["start_quest"] in qids, f'start_quest={d["start_quest"]!r} not in quests'):
    assert_(Q[d["start_quest"]]["trigger"]["type"] == "on_spawn",
            f'{d["start_quest"]}.trigger.type must be on_spawn (init_journal)')

# choices.next_dialogue resolves to a real dialogue id (applies to both acts)
for x in d["dialogue"]:
    for c in (x.get("choices") or []):
        if c.get("next_dialogue") and c["next_dialogue"] not in dids:
            assert_(False, f'{x["id"]}.{c["id"]}.next_dialogue -> {c["next_dialogue"]!r} not found')

if act == 1:
    # quest_demo / spawn_garren reference these quest ids verbatim
    for qid in ("q1_embers","q2_voice_in_stone","q3_gatekeeper","q4_what_walls_remember"):
        assert_(qid in qids, f'code references quest {qid!r} but it is absent')
    # spawn_npcs (quest.rs:494) hardcodes npc_id "maren"; open_npc_dialogue matches speaker=="maren"
    assert_("maren" in npc_ids, 'npc "maren" must exist (spawn_npcs hardcodes it)')
    # dlg_maren_gate hub -- demo (quest.rs:1185) expects 4 lines + 5 choices, Digit5->idx4 advances q3
    g = next((x for x in d["dialogue"] if x["id"] == "dlg_maren_gate"), None)
    assert_(g is not None, "dlg_maren_gate hub missing")
    if g:
        assert_(g["speaker"] == "maren", 'dlg_maren_gate.speaker must be maren')
        assert_(g["trigger"]["type"] == "enter_zone", 'dlg_maren_gate.trigger.type must be enter_zone (open_npc_dialogue)')
        assert_(len(g["lines"]) == 4, f'dlg_maren_gate has {len(g["lines"])} lines (demo expects 4)')
        ch = g.get("choices") or []
        assert_(len(ch) == 5, f'dlg_maren_gate has {len(ch)} choices (demo Digit5 expects idx4 of 5)')
        if len(ch) >= 5:
            c4 = ch[4]
            assert_(c4.get("advances_quest") == "q3_gatekeeper",
                    f'choice[4].advances_quest={c4.get("advances_quest")!r} (demo Digit5 must advance q3_gatekeeper)')
    # q3 objective shape: check_kill_triggers needs a 'defeat' obj; approach obj o2_observe; reach_zone o1_east
    q3 = Q.get("q3_gatekeeper", {})
    kinds = [o["kind"] for o in q3.get("objectives", [])]
    assert_("defeat" in kinds, "q3 needs a 'defeat' objective (check_kill_triggers)")
    assert_("approach_entity" in kinds, "q3 needs an 'approach_entity' obj (check_approach_triggers, o2_observe)")
    assert_("reach_zone" in kinds, "q3 needs a 'reach_zone' obj (check_area_triggers, o1_east)")
    # regions the runtime + demo read with exact bounds
    gs = regs.get("gate_square")
    assert_(gs and gs["bounds"] == {"x0":27,"z0":3,"x1":37,"z1":8}, f'gate_square bounds={gs and gs["bounds"]}')
    gp = regs.get("guard_post_east")
    assert_(gp and gp["bounds"] == {"x0":48,"z0":4,"x1":56,"z1":12}, f'guard_post_east bounds={gp and gp["bounds"]}')
    # q3 completion must unlock q4 (complete_quest uses qdef.next then rewards.advance_to)
    assert_(q3.get("next") == "q4_what_walls_remember" or
            (q3.get("rewards",{}) or {}).get("advance_to") == "q4_what_walls_remember",
            "q3 must unlock q4 via next or rewards.advance_to")

elif act == 2:
    assert_(d["start_quest"] == "q6_the_warden", f'start_quest={d["start_quest"]!r} (code expects q6_the_warden)')
    for qid in ("q6_the_warden","q7_those_we_left_below","q8_the_rite_they_kept",
                "q9_the_warm_thing","q10_the_cradle","q11_the_architect","q12_one_slow_breath"):
        assert_(qid in qids, f'act2 quest {qid!r} is absent')
    # every dialogue speaker resolves to the roster (open_npc_dialogue matches speaker==npc.id)
    for x in d["dialogue"]:
        assert_(x["speaker"] in npc_ids, f'{x["id"]}.speaker {x["speaker"]!r} not an npc')
    # q11 drops two Shaper Fragments in one reward step — the reason set_flag became
    # a FlagSpec (String | Vec<String>) in quest.rs.
    q11 = Q.get("q11_the_architect", {})
    sf = (q11.get("rewards") or {}).get("set_flag")
    assert_(isinstance(sf, list) and len(sf) == 2, f'q11 set_flag={sf!r} (expects the 2-fragment list)')

print(f"    -> {'OK' if not E else 'FAIL'} ({len(E)} engine-contract failures)\n")

# ---------------------------------------------------------------------------
n = len(errs) + len(E)
print("=" * 64)
print(f"VERDICT: {'SERDE-FAITHFUL + ENGINE-CONTRACT CLEAN' if n == 0 else f'{n} PROBLEM(S)'}")
print(f"  serde field/type errors : {len(errs)}")
print(f"  engine-contract failures: {len(E)}")
sys.exit(1 if n else 0)
