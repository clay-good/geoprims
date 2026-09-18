#!/usr/bin/env python3
"""Keeps regenerated vector files honest about published history.

    supersede.py BASE FILE REASON      # rewrite FILE against git BASE
    supersede.py BASE --check          # list silent edits in core/vectors

Run after a generator rewrites FILE. Every vector published at BASE keeps its
line. When its input or expectations changed, the published line gains
"supersededBy" (the new id) and REASON, and the regenerated vector moves to
the next free id at the end of the file. Unchanged vectors stay as they were.
"""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def published(base, rel):
    try:
        out = subprocess.run(["git", "show", f"{base}:{rel}"], cwd=ROOT, capture_output=True, text=True, check=True).stdout
    except subprocess.CalledProcessError:
        return None
    return [json.loads(line) for line in out.splitlines() if line.strip()]


def same(a, b):
    return a.get("input") == b.get("input") and a.get("expect") == b.get("expect")


def dump(vs):
    return "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs)


def rewrite(base, path, reason):
    rel = str(Path(path).resolve().relative_to(ROOT))
    old = published(base, rel)
    new = [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]
    if old is None:
        return 0
    by_id = {v["id"]: v for v in new}
    out, moved, changed = [], [], 0
    for was in old:
        now = by_id.pop(was["id"], None)
        if was.get("supersededBy") or (now is not None and same(was, now)):
            out.append(was)
            continue
        changed += 1
        if now is not None:
            moved.append(now)
        out.append(dict(was, supersededBy="", reason=reason))
    moved.extend(by_id.values())  # vectors new in this regeneration
    used = {v["id"] for v in out}
    n = max([int(v["id"][1:]) for v in out + moved if v["id"][1:].isdigit()] + [0])
    renamed = {}
    for v in moved:
        if v["id"] in used:
            n += 1
            renamed[v["id"]] = f"v{n:03d}"
            v["id"] = renamed[v["id"]]
        used.add(v["id"])
        out.append(v)
    for v in out:
        if v.get("supersededBy") == "":
            v["supersededBy"] = renamed.get(v["id"], "removed")
    Path(path).write_text(dump(out))
    return changed


def check(base):
    bad = []
    for f in sorted((ROOT / "core/vectors").glob("*.jsonl")):
        old = published(base, str(f.relative_to(ROOT)))
        if old is None:
            continue
        now = {json.loads(line)["id"]: json.loads(line) for line in f.read_text().splitlines() if line.strip()}
        for was in old:
            is_ = now.get(was["id"])
            if is_ is None:
                bad.append(f"{f.name} {was['id']} deleted")
            elif not is_.get("supersededBy") and not same(was, is_):
                bad.append(f"{f.name} {was['id']} changed")
    print("\n".join(bad) or "no silent edits")
    return 1 if bad else 0


if __name__ == "__main__":
    if sys.argv[2] == "--check":
        sys.exit(check(sys.argv[1]))
    print(f"{rewrite(sys.argv[1], sys.argv[2], sys.argv[3])} superseded in {sys.argv[2]}")
