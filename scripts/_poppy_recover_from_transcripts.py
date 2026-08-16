#!/usr/bin/env python3
"""Rebuild files destroyed by `git clean -fd` (2026-08-16) out of Claude Code transcripts.

WHY THIS CAN WORK AT ALL. The 174 files were untracked, so git has nothing. But
every Write/Edit an agent ever performed is recorded verbatim in the session
transcripts under C:\\Users\\BagIdea\\.claude\\projects\\*\\*.jsonl -- a different
drive, untouched by the clean. A Write tool_use carries the FULL file content;
an Edit carries old_string/new_string. Replaying those in timestamp order
reconstructs the file as it stood after the last recorded edit.

WHAT THIS DELIBERATELY DOES NOT DO. It never invents text. If a file's history
starts with an Edit rather than a Write, the base content was never captured and
the result is marked INCOMPLETE rather than patched up with a guess. Same if an
Edit's old_string cannot be found in the reconstructed text -- that means an
edit landed that the transcripts did not record, so the tail is uncertain and
the file is flagged. A file this tool marks INCOMPLETE is not a recovered file.

Usage:
  python scripts/_poppy_recover_from_transcripts.py --list <paths-file> --out <dir>
  python scripts/_poppy_recover_from_transcripts.py --list <paths-file> --out <dir> --apply
"""

import argparse
import json
import os
import sys
from pathlib import Path

TRANSCRIPT_DIRS = [
    Path(r"C:\Users\BagIdea\.claude\projects\E--Projects-bagidea-ai-agents-office-workspace"),
    Path(r"C:\Users\BagIdea\.claude\projects\E--Projects-bagidea-ai-agents-office-workspace-projects-Voxelforge"),
]


def norm(p):
    """Compare paths drive-and-slash agnostically, from the right."""
    return str(p).replace("\\", "/").lower()


def harvest(targets):
    """Return {target: [event, ...]} where event = (timestamp, session, kind, payload).

    A transcript line holds a message whose content is a list of blocks; the
    blocks we want are tool_use blocks naming Write or Edit. `file_path` is
    absolute in the record, so match on suffix -- the same doc can be written
    from a worktree with a different prefix and it is still the same file.
    """
    found = {t: [] for t in targets}
    scanned = files_with_hits = 0

    for d in TRANSCRIPT_DIRS:
        if not d.is_dir():
            continue
        for jf in sorted(d.glob("*.jsonl")):
            scanned += 1
            hit_here = False
            try:
                with open(jf, "r", encoding="utf-8", errors="replace") as fh:
                    for line in fh:
                        if '"Write"' not in line and '"Edit"' not in line:
                            continue  # cheap prefilter: most lines are not tool calls
                        try:
                            rec = json.loads(line)
                        except Exception:
                            continue
                        msg = rec.get("message") or {}
                        content = msg.get("content")
                        if not isinstance(content, list):
                            continue
                        ts = rec.get("timestamp") or ""
                        for blk in content:
                            if not isinstance(blk, dict):
                                continue
                            if blk.get("type") != "tool_use":
                                continue
                            name = blk.get("name")
                            if name not in ("Write", "Edit"):
                                continue
                            inp = blk.get("input") or {}
                            fp = inp.get("file_path")
                            if not fp:
                                continue
                            nfp = norm(fp)
                            for t in targets:
                                if nfp.endswith(norm(t)):
                                    found[t].append((ts, jf.name, name, inp))
                                    hit_here = True
            except Exception as e:
                print(f"  !! unreadable {jf.name}: {e}", file=sys.stderr)
            if hit_here:
                files_with_hits += 1

    print(f"scanned {scanned} transcripts, {files_with_hits} contained a hit")
    return found


def replay(events):
    """Fold Write/Edit events into final text. Returns (text, status, notes)."""
    events = sorted(events, key=lambda e: (e[0], e[1]))
    text = None
    notes = []
    complete = True

    for ts, sess, kind, inp in events:
        if kind == "Write":
            text = inp.get("content", "")
            notes.append(f"{ts}  Write  ({len(text)}B)  {sess}")
        else:
            old = inp.get("old_string", "")
            new = inp.get("new_string", "")
            if text is None:
                complete = False
                notes.append(f"{ts}  Edit   SKIPPED - no base Write seen first  {sess}")
                continue
            if old not in text:
                complete = False
                notes.append(f"{ts}  Edit   FAILED - old_string absent  {sess}")
                continue
            if inp.get("replace_all"):
                text = text.replace(old, new)
            else:
                text = text.replace(old, new, 1)
            notes.append(f"{ts}  Edit   ok -> {len(text)}B  {sess}")

    if text is None:
        return None, "NOT-FOUND", notes
    return text, ("COMPLETE" if complete else "INCOMPLETE"), notes


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--list", required=True, help="file of repo-relative paths, one per line")
    ap.add_argument("--out", required=True, help="staging dir for reconstructed files")
    ap.add_argument("--apply", action="store_true", help="also write into the repo")
    ap.add_argument("--repo", default=".", help="repo root for --apply")
    args = ap.parse_args()

    # utf-8-sig: a list written by PowerShell's `Set-Content -Encoding utf8`
    # carries a BOM, and a ﻿ glued to the first path makes it match nothing.
    # Reconfigure stdout too -- these paths and docs carry Thai, and the default
    # cp1252 console codec raises rather than printing them.
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    targets = [l.strip() for l in open(args.list, encoding="utf-8-sig") if l.strip()]
    print(f"targets: {len(targets)}")

    found = harvest(targets)
    outdir = Path(args.out)
    outdir.mkdir(parents=True, exist_ok=True)

    summary = []
    for t in targets:
        text, status, notes = replay(found[t])
        n_w = sum(1 for e in found[t] if e[2] == "Write")
        n_e = sum(1 for e in found[t] if e[2] == "Edit")
        if text is not None:
            dest = outdir / Path(t).name
            dest.write_text(text, encoding="utf-8", newline="\n")
            if args.apply:
                real = Path(args.repo) / t
                real.parent.mkdir(parents=True, exist_ok=True)
                real.write_text(text, encoding="utf-8", newline="\n")
            size = len(text)
        else:
            size = 0
        summary.append((t, status, n_w, n_e, size))
        print(f"\n--- {t}  [{status}]  writes={n_w} edits={n_e} bytes={size}")
        for n in notes[-6:]:
            print(f"      {n}")

    print("\n================ SUMMARY ================")
    ok = 0
    for t, status, n_w, n_e, size in summary:
        if status == "COMPLETE":
            ok += 1
        print(f"  {status:11s} w={n_w:2d} e={n_e:2d} {size:7d}B  {t}")
    print(f"\nCOMPLETE {ok}/{len(targets)}")


if __name__ == "__main__":
    main()
