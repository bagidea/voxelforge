import json
from pathlib import Path

SRC = Path(r"E:\Projects\bagidea-ai-agents-office\plugins\research-board\data\sources.json")
OUT = Path(r"E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\docs\research\research-board-sources.jsonl")

data = json.loads(SRC.read_text(encoding="utf-8"))

# Keep only voxelforge-tagged sources, sorted newest first
voxelforge = [r for r in data if "voxelforge" in (r.get("tags") or [])]
voxelforge.sort(key=lambda r: r.get("capturedAt", ""), reverse=True)

def credibility(r):
    kind = (r.get("kind") or "").lower()
    if kind in ("official", "primary"):
        return "primary"
    if kind in ("news", "reputable"):
        return "secondary"
    return "secondary"  # default for our own entries

lines = []
for r in voxelforge:
    rec = {
        "claim": r["claim"],
        "source": r.get("title") or r.get("source") or r["domain"],
        "url": r["url"],
        "credibility": credibility(r),
    }
    lines.append(json.dumps(rec, ensure_ascii=False))

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(f"Wrote {len(lines)} voxelforge sources to {OUT}")
