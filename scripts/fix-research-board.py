import json
from pathlib import Path

SRC = Path(r"E:\Projects\bagidea-ai-agents-office\plugins\research-board\data\sources.json")

data = json.loads(SRC.read_text(encoding="utf-8"))

# 1. Remove placeholder rb_1911539f
data = [r for r in data if r.get("id") != "rb_1911539f"]

# 2. Merge three duplicate Bevy 0.16 entries into one (keep rb_92e7ceb8)
bevy_ids = {"rb_92e7ceb8", "rb_f5b9e3ad", "rb_f382774f"}
bevy_entries = [r for r in data if r.get("id") in bevy_ids]
other_entries = [r for r in data if r.get("id") not in bevy_ids]

if len(bevy_entries) == 3:
    by_id = {r["id"]: r for r in bevy_entries}
    merged_claim = (
        "Bevy 0.16 release notes cover GPU-driven rendering (MDI/MDIC, bindless, GPU transform/cull, retained render world), "
        "platform support (Vulkan full, Metal partial, WebGPU transform-only, WebGL2 none), "
        "and rendering features (deferred rendering optional since 0.12, SSAO since 0.11, bloom/DOF built-in, procedural atmosphere)."
    )
    merged = by_id["rb_92e7ceb8"]
    merged["claim"] = merged_claim
    merged["title"] = "Bevy 0.16 Release Notes"
    merged["tags"] = ["voxelforge", "bevy", "rendering", "gpu-driven"]
    data = other_entries + [merged]

# 3. Add voxelforge tag to all entries captured on 2026-07-26T16:04:55 (the new batch from this task)
for r in data:
    cap = r.get("capturedAt", "")
    if isinstance(cap, str) and cap.startswith("2026-07-26T16:04:55"):
        tags = list(r.get("tags", []))
        if "voxelforge" not in tags:
            tags.insert(0, "voxelforge")
        r["tags"] = tags

SRC.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
print(f"Wrote {len(data)} sources to {SRC}")

# Print summary
bevy_count = sum(1 for r in data if r.get("url") == "https://bevy.org/news/bevy-0-16/")
new_batch = sum(1 for r in data if str(r.get("capturedAt", "")).startswith("2026-07-26T16:04:55"))
print(f"Bevy 0.16 URL entries: {bevy_count}")
print(f"New batch entries (2026-07-26T16:04:55): {new_batch}")
print(f"Placeholder present: {any(r.get('id') == 'rb_1911539f' for r in data)}")
