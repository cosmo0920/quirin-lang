"""AG3 census: rows whose blockers include the condition/return refusal.

Reads the user's `current` report read-only, writes JSON to argv[2].
"""
import json
import os
import sys
from collections import Counter

REPORT = sys.argv[1]
OUT = sys.argv[2]
SRC_ROOT = os.path.join(os.path.dirname(os.path.dirname(REPORT)), "fluent-bit-clang-source")
PREFIX = "unmodeled call, aggregate or indirect access in "


def position(desc):
    if desc.startswith(PREFIX):
        return desc[len(PREFIX):]
    return None


_cache = {}


def source_line(origin):
    if not origin or ":" not in origin:
        return None
    path, _, line = origin.rpartition(":")
    try:
        n = int(line)
    except ValueError:
        return None
    full = os.path.join(SRC_ROOT, path)
    if full not in _cache:
        try:
            with open(full, encoding="utf-8", errors="replace") as fh:
                _cache[full] = fh.read().split("\n")
        except OSError:
            _cache[full] = None
    lines = _cache[full]
    if not lines or n < 1 or n > len(lines):
        return None
    # Join continuation lines until parentheses balance (best effort, 6 lines).
    text = lines[n - 1].strip()
    depth = text.count("(") - text.count(")")
    k = n
    while depth > 0 and k < n + 6 and k < len(lines):
        nxt = lines[k].strip()
        text += " " + nxt
        depth += nxt.count("(") - nxt.count(")")
        k += 1
    return text


def main():
    d = json.load(open(REPORT))
    rows = []
    for unit in d["units"]:
        for f in unit.get("functions", []):
            diags = f.get("diagnostics", [])
            ag3 = [x for x in diags if x.get("category") == "unsupported" and position(x.get("description", ""))]
            if not ag3:
                continue
            others = [x for x in diags if not (x.get("category") == "unsupported" and position(x.get("description", "")))]
            rows.append({
                "file": unit.get("file"),
                "library": unit.get("library"),
                "function": f.get("function"),
                "status": f.get("status"),
                "sole": not others,
                "other_kinds": sorted({(x.get("category"), x.get("description")) for x in others}.__iter__())[:12],
                "ag3": [
                    {
                        "position": position(x["description"]),
                        "origin": x.get("origin"),
                        "line": x.get("line"),
                        "text": source_line(x.get("origin")),
                    }
                    for x in ag3
                ],
            })
    json.dump(rows, open(OUT, "w"), indent=1)
    c = Counter()
    s = Counter()
    for r in rows:
        for p in {a["position"] for a in r["ag3"]}:
            c[p] += 1
            if r["sole"]:
                s[p] += 1
    print("rows", len(rows), "sole", sum(r["sole"] for r in rows))
    for p, n in c.most_common():
        print(f"{n:6d} {s[p]:4d} {p}")


main()
