#!/usr/bin/env python3
"""Rose — parse the recap grade log into a BEFORE(ctrl)/AFTER(new) markdown table.

Reads the combined grade log (_flamingo_recap_grade_20260806.sh output) which
runs grade_gate + grade_axes + grade_g3 + measure_penumbra per stem×arm, and
emits one compact table per stem: ctrl (old look constants) vs new (baked
default), so the patch's contribution is the column delta, isolated from the
5-day stale-binary confound the original scorecard carried.

Usage: python scripts/_rose_summary.py <grade_log> [out.md]
"""
import re, sys, os

# Windows stdout defaults to cp1252, which cannot encode unicode minus / >=.
# Force UTF-8 so `print(md)` never crashes the finisher when it is redirected.
try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass

def parse(log):
    # Sections look like:
    #   # gate3-boot  [new]   path
    cur = None          # (stem, arm)
    rows = {}           # (stem,arm) -> dict
    ax_re = {
        "warmth": re.compile(r"\[\w+\]\s+warmth R-B \(mid\)\s+(-?\d+\.\d+)"),
        "blue":   re.compile(r"\[\w+\]\s+blue B \(mid\)\s+(-?\d+\.\d+)"),
        "sat":    re.compile(r"\[\w+\]\s+saturation \(mid\)\s+(-?\d+\.\d+)"),
        "micro":  re.compile(r"\[\w+\]\s+micro-contrast\s+(-?\d+\.\d+)"),
        "p95":    re.compile(r"\[\w+\]\s+highlight p95\s+(-?\d+\.\d+)"),
    }
    gates_re = re.compile(r"MEASURABLE GATES:\s*G3=(\w)\s+G5=(\w)\s+G6=(\w)")
    # grade_gate G3 line: "interior p05-L=X.X% (need >=8)  p50-L=X.X%"
    gate_p05 = re.compile(r"interior p05-L=(-?\d+\.\d+)%")
    gate_rb  = re.compile(r"darkest shade.*R-B=([+-]?\d+)")            # int in gate
    g3_p05   = re.compile(r"p05\s*L\s*=\s*(-?\d+\.\d+)%")              # grade_g3
    pen_re   = re.compile(r"median\D*(\d+(?:\.\d+)?)\s*px", re.I)
    sect_re  = re.compile(r"^#\s+(\S+)\s+\[(new|ctrl)\]")

    for line in log.splitlines():
        m = sect_re.match(line)
        if m:
            stem, arm = m.group(1), m.group(2)
            cur = (stem, arm)
            rows.setdefault(cur, {"stem": stem, "arm": arm})
            continue
        if cur is None:
            continue
        r = rows[cur]
        for key, rgx in ax_re.items():
            mm = rgx.search(line)
            if mm:
                r[key] = float(mm.group(1))
        mm = gates_re.search(line)
        if mm:
            r["G3"], r["G5"], r["G6"] = mm.group(1), mm.group(2), mm.group(3)
        mm = gate_p05.search(line)
        if mm and "p05" not in r:
            r["p05"] = float(mm.group(1))
        mm = g3_p05.search(line)
        if mm:
            r["p05_g3"] = float(mm.group(1))
        mm = gate_rb.search(line)
        if mm and "rb" not in r:
            r["rb"] = int(mm.group(1))
        mm = pen_re.search(line)
        if mm and "pen" not in r:
            r["pen"] = float(mm.group(1))
    return rows

def fmt_delta(ctrl, new, better="higher"):
    if ctrl is None or new is None:
        return "—"
    d = new - ctrl
    sign = "+" if d > 0 else ""
    return f"{ctrl:.1f} → {new:.1f} ({sign}{d:.1f})"

def main():
    log_path = sys.argv[1]
    out_path = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(log_path) or ".", "_rose_summary.md")
    with open(log_path, encoding="utf-8", errors="replace") as f:
        log = f.read()
    rows = parse(log)
    stems = []
    for k in rows:
        if k[1] == "new" and k not in stems:
            stems.append(k[0])
    # preserve a sensible order
    order = ["gate3-boot", "gate3-walk", "gate3-combat", "grade-vista"]
    stems.sort(key=lambda s: order.index(s) if s in order else 99)

    out = []
    out.append("# Rose — AAA scorecard patch: BEFORE(ctrl) vs AFTER(new)\n")
    out.append("ctrl = old look constants via env (1100 lux / ambient B 0.60 / PCSS 3.0); "
               "new = baked default (2200 lux / B 0.48 / PCSS 4.0). Same binary. "
               "Delta = the patch's contribution alone.\n")
    out.append("## Per-axis delta (midtone band, gameplay profile)\n")
    out.append("| stem | warmth R-B (>=110) | blue B (<=10) | sat (>=90) | micro (>=5) | p95 (150..185) |")
    out.append("|---|---|---|---|---|---|")
    for s in stems:
        c = rows.get((s, "ctrl"), {}); n = rows.get((s, "new"), {})
        out.append(f"| **{s}** | {fmt_delta(c.get('warmth'), n.get('warmth'))} | "
                   f"{fmt_delta(c.get('blue'), n.get('blue'))} | {fmt_delta(c.get('sat'), n.get('sat'))} | "
                   f"{fmt_delta(c.get('micro'), n.get('micro'))} | {fmt_delta(c.get('p95'), n.get('p95'))} |")

    out.append("\n## G3 (the #1 gap) + gates + penumbra\n")
    out.append("| stem | p05-L % (>=8) | darkest R-B (>=0) | G3 | G5 | G6 | penumbra px (>=5) |")
    out.append("|---|---|---|---|---|---|---|")
    for s in stems:
        c = rows.get((s, "ctrl"), {}); n = rows.get((s, "new"), {})
        p05c = c.get("p05", c.get("p05_g3")); p05n = n.get("p05", n.get("p05_g3"))
        out.append(f"| **{s}** | {fmt_delta(p05c, p05n)} | {fmt_delta(c.get('rb'), n.get('rb'))} | "
                   f"{c.get('G3','?')}→{n.get('G3','?')} | {c.get('G5','?')}→{n.get('G5','?')} | "
                   f"{c.get('G6','?')}→{n.get('G6','?')} | {fmt_delta(c.get('pen'), n.get('pen'))} |")

    md = "\n".join(out) + "\n"
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(md)
    print(md)
    print(f"# wrote {out_path}")

if __name__ == "__main__":
    main()
