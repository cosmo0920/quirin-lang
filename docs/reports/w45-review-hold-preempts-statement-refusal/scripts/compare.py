#!/usr/bin/env python3
"""compare.py BASELINE RUN [--json OUT]

Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL. BASELINE is an archive
directory (a `manifest.json` of raw SHA-256s beside gzipped files) or a run
directory; RUN is a run directory. Prints, for resource-diagnostics.json:

* raw identity against the baseline;
* the top-level and per-unit keys whose values differ (so a difference in the
  run's own paths, `resource_contract_sources` or `inventory_sha256`, is named
  and nothing else is hidden);
* per (unit file, function): status and leaf-multiset differences, and the
  unit status and counts (functions Safe/Pending/Unsafe/Unknown, units,
  diagnostic counts) on both sides.

Exit 0 when no unit or function status and no leaf differs, 1 otherwise.
"""
import gzip
import hashlib
import json
import pathlib
import sys
from collections import Counter


def load(directory: pathlib.Path):
    raw = directory / "resource-diagnostics.json"
    if raw.is_file():
        data = raw.read_bytes()
    else:
        data = gzip.decompress((directory / "resource-diagnostics.json.gz").read_bytes())
    return data, json.loads(data)


def rows(report):
    functions, units, leaves = {}, {}, {}
    for unit in report["units"]:
        key = (unit.get("library"), unit["file"])
        units[key] = unit.get("status")
        for function in unit.get("functions", []):
            fkey = key + (function["function"],)
            functions[fkey] = function.get("status")
            leaves[fkey] = Counter(
                (d.get("category"), d.get("description"), d.get("line"))
                for d in function.get("diagnostics", [])
            )
    return functions, units, leaves


def tally(values):
    return dict(sorted(Counter(values).items(), key=lambda item: str(item[0])))


def main() -> int:
    baseline_dir, run_dir = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    out = sys.argv[sys.argv.index("--json") + 1] if "--json" in sys.argv else None
    base_bytes, base = load(baseline_dir)
    run_bytes, run = load(run_dir)
    identical = hashlib.sha256(base_bytes).digest() == hashlib.sha256(run_bytes).digest()
    top = sorted(k for k in set(base) | set(run) if k != "units" and base.get(k) != run.get(k))
    unit_keys = Counter()
    base_units = {(u.get("library"), u["file"]): u for u in base["units"]}
    for unit in run["units"]:
        other = base_units.get((unit.get("library"), unit["file"]))
        if other is None:
            unit_keys["<unit absent from baseline>"] += 1
            continue
        for key in set(unit) | set(other):
            if key != "functions" and unit.get(key) != other.get(key):
                unit_keys[key] += 1
    bf, bu, bl = rows(base)
    rf, ru, rl = rows(run)
    moved = sorted(str(k) for k in set(bf) | set(rf) if bf.get(k) != rf.get(k))
    leaf_moves = sorted(str(k) for k in set(bl) | set(rl) if bl.get(k, Counter()) != rl.get(k, Counter()))
    unit_moves = sorted(str(k) for k in set(bu) | set(ru) if bu.get(k) != ru.get(k))
    paths = set()

    def walk(x, y, path):
        if isinstance(x, dict) and isinstance(y, dict):
            for key in set(x) | set(y):
                walk(x.get(key), y.get(key), f"{path}/{key}")
        elif isinstance(x, list) and isinstance(y, list) and len(x) == len(y):
            for u, v in zip(x, y):
                walk(u, v, f"{path}[]")
        elif x != y:
            paths.add(path)

    walk(base, run, "")
    result = {
        "resource_diagnostics_identical": identical,
        "differing_json_paths": sorted(paths),
        "differing_top_level_keys": top,
        "differing_unit_keys": dict(unit_keys),
        "function_status_moves": moved,
        "function_leaf_moves": leaf_moves,
        "unit_status_moves": unit_moves,
        "baseline": {
            "functions": tally(bf.values()),
            "units": tally(bu.values()),
            "diagnostic_counts": base.get("diagnostic_counts"),
        },
        "run": {
            "functions": tally(rf.values()),
            "units": tally(ru.values()),
            "diagnostic_counts": run.get("diagnostic_counts"),
        },
    }
    text = json.dumps(result, indent=1, sort_keys=True)
    print(text)
    if out:
        pathlib.Path(out).write_text(text + "\n", encoding="utf-8")
    return 0 if not (moved or leaf_moves or unit_moves) else 1


if __name__ == "__main__":
    raise SystemExit(main())
