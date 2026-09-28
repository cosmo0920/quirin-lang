#!/usr/bin/env python3
"""Compare a run directory with a baseline directory file by file (raw
SHA-256) and the parity stream as a sorted multiset of rows.

Usage: identity.py RUN BASELINE OUT_JSON [RUN_ROOT=BASELINE_ROOT]

With a root alias, every file that differs is compared again after replacing
the run's worktree root by the baseline's in the run's text, and the
remaining differing JSON leaves are counted by key (``normalised``).
"""
import hashlib
import json
import pathlib
import sys

run, base, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
alias = sys.argv[4].split("=", 1) if len(sys.argv) > 4 else None
FILES = [
    "resource-diagnostics.json",
    "monkey-source.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
    "resource-diagnostics.invocation.json",
    "parity.jsonl.contexts.jsonl",
]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


result = {"run": str(run), "baseline": str(base), "files": {}}
for name in FILES:
    a, b = sha(run / name), sha(base / name)
    if a is None and b is None:
        continue
    result["files"][name] = {"run": a, "baseline": b, "identical": a == b}
def leaves(a, b, path, found):
    if isinstance(a, dict) and isinstance(b, dict):
        for key in sorted(set(a) | set(b)):
            leaves(a.get(key), b.get(key), path + [str(key)], found)
    elif isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
        for index, (x, y) in enumerate(zip(a, b)):
            leaves(x, y, path + [str(index)], found)
    elif a != b:
        found.append(path)


def parse(text):
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return [json.loads(line) for line in text.splitlines() if line.strip()]


if alias:
    for name, entry in result["files"].items():
        if entry["identical"] or entry["run"] is None or entry["baseline"] is None:
            continue
        text = (run / name).read_text().replace(alias[0], alias[1])
        base_text = (base / name).read_text()
        entry["normalised_identical"] = text == base_text
        if text != base_text:
            found = []
            leaves(parse(text), parse(base_text), [], found)
            counts = {}
            for path in found:
                key = path[-1] if path else "<root>"
                if key.isdigit() and len(path) > 1:
                    key = path[-2] + "[]"
                counts[key] = counts.get(key, 0) + 1
            entry["normalised_differing_leaves"] = len(found)
            entry["normalised_by_key"] = counts
rows = []
for label, directory in (("run", run), ("baseline", base)):
    path = directory / "parity.jsonl"
    rows.append(sorted(path.read_text().splitlines()) if path.exists() else None)
if rows[0] is not None and rows[1] is not None:
    result["parity_sorted_identical"] = rows[0] == rows[1]
    result["parity_rows"] = {"run": len(rows[0]), "baseline": len(rows[1])}
result["verdict_bearing_identical"] = all(
    entry["identical"]
    for name, entry in result["files"].items()
    if name in ("resource-diagnostics.json", "monkey-source.json", "resource-diagnostics.chunk-contracts.json")
)
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(result, indent=1) + "\n")
print(json.dumps(result, indent=1))
