#!/usr/bin/env python3
"""Attribute every leaf and status move between two corpus arbiter reports.

Usage: arbiter_compare.py AFTER_REPORT BEFORE_REPORT OUT_JSON [--source-root DIR]

Reports may be plain or gzipped `resource-diagnostics.json`. A leaf is
(unit file, function, category, description, preprocessed line). The output
lists the totals, unit and function status tallies, every function whose
status moved (with its added and removed leaves), and the added/removed
leaves grouped by description. With --source-root, each removed or added
leaf's original line is classified by `shape_census.classify_*` so the
movement can be attributed to the list-iteration and element shapes.
"""

import collections
import gzip
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import shape_census  # noqa: E402


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def leaves(report):
    rows = collections.Counter()
    status = {}
    origin = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            status[key] = function["status"]
            for d in function["diagnostics"]:
                leaf = (unit["file"], function["function"], d["category"], d["description"], d.get("line", 0))
                rows[leaf] += 1
                origin[leaf] = d.get("origin")
    units = collections.Counter(unit["status"] for unit in report["units"])
    return rows, status, origin, units


def main():
    after_path, before_path, out_path = sys.argv[1], sys.argv[2], sys.argv[3]
    root = None
    if "--source-root" in sys.argv:
        root = pathlib.Path(sys.argv[sys.argv.index("--source-root") + 1])
    after, before = load(after_path), load(before_path)
    a_rows, a_status, a_origin, a_units = leaves(after)
    b_rows, b_status, b_origin, b_units = leaves(before)
    added = a_rows - b_rows
    removed = b_rows - a_rows

    by_name = collections.defaultdict(list)
    cache = {}
    if root is not None:
        for path in root.rglob("*"):
            if path.suffix in (".c", ".h") and path.is_file():
                by_name[path.name].append(path)

    def line_of(origin):
        if root is None or not origin or ":" not in origin:
            return None
        file, number = origin.rsplit(":", 1)
        if not number.isdigit():
            return None
        candidates = []
        for path in by_name.get(pathlib.Path(file).name, []):
            if path not in cache:
                cache[path] = path.read_text(errors="replace").splitlines()
            lines = cache[path]
            index = int(number) - 1
            if 0 <= index < len(lines):
                candidates.append(lines[index])
        for line in candidates:
            if shape_census.ENTRY.search(line) or shape_census.FOREACH.search(line):
                return line
        return candidates[0] if candidates else None

    def shape(leaf, origin):
        line = line_of(origin)
        if line is None:
            return "unclassified"
        if shape_census.ENTRY.search(line):
            return "entry-line"
        if shape_census.FOREACH.search(line):
            return "list-header-line:" + shape_census.classify_for(line)
        return "other-line"

    def group(counter, origins):
        grouped = collections.defaultdict(collections.Counter)
        for leaf, count in counter.items():
            grouped[f"{leaf[2]}: {leaf[3]}"][shape(leaf, origins.get(leaf))] += count
        return {
            key: {"total": sum(value.values()), "by_line_shape": dict(value)}
            for key, value in sorted(grouped.items(), key=lambda item: -sum(item[1].values()))
        }

    moved = []
    for key in sorted(set(a_status) | set(b_status)):
        if a_status.get(key) != b_status.get(key):
            moved.append(
                {
                    "unit": key[0],
                    "function": key[1],
                    "before": b_status.get(key),
                    "after": a_status.get(key),
                    "added": sorted(
                        [list(leaf[2:]) + [a_origin.get(leaf)] for leaf in added if leaf[:2] == key]
                    ),
                    "removed": sorted(
                        [list(leaf[2:]) + [b_origin.get(leaf)] for leaf in removed if leaf[:2] == key]
                    ),
                }
            )
    transitions = collections.Counter(f"{m['before']} -> {m['after']}" for m in moved)
    changed_functions = {leaf[:2] for leaf in (added + removed)}
    result = {
        "after": after_path,
        "before": before_path,
        "totals": {"after": after["diagnostic_counts"], "before": before["diagnostic_counts"]},
        "units": {"after": dict(a_units), "before": dict(b_units)},
        "functions": {
            "after": dict(collections.Counter(a_status.values())),
            "before": dict(collections.Counter(b_status.values())),
        },
        "status_transitions": dict(transitions),
        "functions_with_leaf_changes": len(changed_functions),
        "leaves_added": sum(added.values()),
        "leaves_removed": sum(removed.values()),
        "added_by_description": group(added, a_origin),
        "removed_by_description": group(removed, b_origin),
        "status_moves": moved,
        "added_leaks": sorted(
            [list(leaf) + [a_origin.get(leaf)] for leaf in added if leaf[2] == "leak"]
        ),
        "added_violations": sorted(
            [
                list(leaf) + [a_origin.get(leaf)]
                for leaf in added
                if leaf[2] not in ("unsupported", "unknown_alias", "leak", "summary_conflict")
            ]
        ),
    }
    pathlib.Path(out_path).write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({k: result[k] for k in ("totals", "units", "functions", "status_transitions", "functions_with_leaf_changes", "leaves_added", "leaves_removed")}, indent=1))
    print("added leaks:", len(result["added_leaks"]), "added violations:", len(result["added_violations"]))


if __name__ == "__main__":
    main()
