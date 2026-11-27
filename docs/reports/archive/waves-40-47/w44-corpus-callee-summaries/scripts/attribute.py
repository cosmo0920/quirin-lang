#!/usr/bin/env python3
"""Attribute every AFTER-vs-BEFORE diagnostic delta, leaf by leaf.

Usage: attribute.py BEFORE.json[.gz] AFTER.json[.gz] SCOPE_CENSUS.jsonl[.gz] OUT.json [CAPTURE_DIR]

A leaf is `(unit, function, line, category, description)`; the diff is a
multiset difference per function. Every changed leaf is assigned to the first
class that explains it:

* `decided-callee` -- a removed `call to uncontracted function `C`` leaf whose
  callee `C` validated in that unit under the widened scope (census stream);
* `same-line` -- any other change on a line that carried a decided leaf in
  BEFORE (the modeled call now checks its arguments);
* `decided-statement` -- (with CAPTURE_DIR, whose `-P` captures give each
  unit's analysed text) a change on a line whose analysed statement calls a
  callee that validated in that unit, although BEFORE refused the statement
  earlier than the call (a branch condition, an indirect-access argument);
* `decided-function` -- any other change in a function with a decided leaf;
* `transitive` -- a change in a function that calls (by uncontracted leaf) a
  function whose status moved;
* `unattributed` -- anything else. The card's claim is that this is empty.

Status moves (functions and units) and diagnostic-count deltas are reported
beside the classes, with every violation-category leaf (leak, double_release,
invalid_release, use_after_release) added or removed listed individually.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import definition_sites  # noqa: E402

NAMED_CALL = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")

CALL = re.compile(r"call to uncontracted function `([^`]+)`")
VIOLATIONS = {"leak", "double_release", "invalid_release", "use_after_release"}


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    return json.load(opener(path))


def leaves(data):
    out = {}
    statuses = {}
    units = {}
    for unit in data["units"]:
        key = unit["library"] + "/" + unit["file"]
        units[key] = unit["status"]
        for entry in unit["functions"]:
            fkey = (key, entry["function"])
            statuses[fkey] = entry["status"]
            out[fkey] = collections.Counter(
                (d["line"], d["category"], d["description"]) for d in entry.get("diagnostics") or []
            )
    return out, statuses, units


def main() -> int:
    before_path, after_path, scope_path, out = sys.argv[1:5]
    capture = sys.argv[5] if len(sys.argv) > 5 else None
    before = load(before_path)
    after = load(after_path)
    b_leaves, b_status, b_units = leaves(before)
    a_leaves, a_status, a_units = leaves(after)
    opener = gzip.open if scope_path.endswith(".gz") else open
    by_selected = {}
    for line in opener(scope_path, "rt"):
        row = json.loads(line)
        by_selected.setdefault(tuple(sorted(row["selected"])), row)
    validated = {}
    for unit in before["units"]:
        key = unit["library"] + "/" + unit["file"]
        row = by_selected.get(tuple(sorted(e["function"] for e in unit["functions"])))
        validated[key] = {c["name"]: c["summary"] for c in (row or {"callees": []})["callees"]
                          if c["outcome"] == "validated"}
    moved = {k for k in b_status if a_status.get(k) != b_status[k]}
    moved_names = collections.defaultdict(set)
    for unit, name in moved:
        moved_names[unit].add(name)
    decided_lines = collections.defaultdict(set)
    decided_functions = set()
    for fkey, counter in b_leaves.items():
        for (line, _, description) in counter:
            match = CALL.fullmatch(description)
            if match and match.group(1) in validated.get(fkey[0], {}):
                decided_lines[fkey].add(line)
                decided_functions.add(fkey)
    texts = {}
    if capture:
        changed_units = {k[0] for k in set(b_leaves) | set(a_leaves)
                         if b_leaves.get(k, collections.Counter()) != a_leaves.get(k, collections.Counter())}
        captured = []
        for path in sorted(pathlib.Path(capture, "pp").glob("*-p1.i")):
            text = path.read_text(errors="replace")
            captured.append((set(definition_sites.definitions(text)), text))
        for unit in before["units"]:
            key = unit["library"] + "/" + unit["file"]
            if key not in changed_units:
                continue
            names = {e["function"] for e in unit["functions"]}
            matches = sorted((c for c in captured if names <= c[0]), key=lambda c: len(c[0]))
            if matches:
                texts[key] = matches[0][1].split("\n")

    def statement_calls_decided(unit, line):
        lines = texts.get(unit)
        if not lines or not 0 < line <= len(lines):
            return []
        return sorted(set(NAMED_CALL.findall(lines[line - 1])) & set(validated.get(unit, {})))

    changes = []
    for fkey in sorted(set(b_leaves) | set(a_leaves)):
        b = b_leaves.get(fkey, collections.Counter())
        a = a_leaves.get(fkey, collections.Counter())
        for sign, diff in (("removed", b - a), ("added", a - b)):
            for (line, category, description), count in diff.items():
                match = CALL.fullmatch(description)
                if sign == "removed" and match and match.group(1) in validated.get(fkey[0], {}):
                    kind = "decided-callee"
                elif line in decided_lines.get(fkey, ()):
                    kind = "same-line"
                elif statement_calls_decided(fkey[0], line):
                    kind = "decided-statement"
                elif fkey in decided_functions:
                    kind = "decided-function"
                elif any(CALL.fullmatch(d) and CALL.fullmatch(d).group(1) in moved_names[fkey[0]]
                         for (_, _, d) in b):
                    kind = "transitive"
                else:
                    kind = "unattributed"
                for _ in range(count):
                    changes.append({"unit": fkey[0], "function": fkey[1], "change": sign,
                                    "line": line, "category": category,
                                    "description": description, "class": kind,
                                    "statement_callees": statement_calls_decided(fkey[0], line),
                                    "callee_summary": validated.get(fkey[0], {}).get(match.group(1))
                                    if match else None})
    classes = collections.Counter((c["class"], c["change"]) for c in changes)
    counts_before = before["diagnostic_counts"]
    counts_after = after["diagnostic_counts"]
    record = {
        "changed_leaves": len(changes),
        "classes": {f"{k[0]}:{k[1]}": v for k, v in sorted(classes.items())},
        "function_moves": sorted(
            [{"unit": u, "function": f, "before": b_status[(u, f)], "after": a_status.get((u, f))}
             for (u, f) in moved], key=lambda r: (r["unit"], r["function"])),
        "function_totals": {
            "before": dict(collections.Counter(b_status.values())),
            "after": dict(collections.Counter(a_status.values())),
        },
        "unit_moves": sorted(
            [{"unit": u, "before": b_units[u], "after": a_units.get(u)} for u in b_units
             if a_units.get(u) != b_units[u]], key=lambda r: r["unit"]),
        "unit_totals": {
            "before": dict(collections.Counter(b_units.values())),
            "after": dict(collections.Counter(a_units.values())),
        },
        "diagnostic_counts": {k: {"before": counts_before.get(k, 0), "after": counts_after.get(k, 0)}
                              for k in sorted(set(counts_before) | set(counts_after))},
        "violation_leaves": [c for c in changes if c["category"] in VIOLATIONS],
        "changes": changes,
    }
    with open(out, "w") as handle:
        json.dump(record, handle, indent=1)
        handle.write("\n")
    summary = {k: record[k] for k in ("changed_leaves", "classes", "function_moves", "function_totals",
                                      "unit_moves", "unit_totals", "diagnostic_counts", "violation_leaves")}
    print(json.dumps(summary, indent=1))
    return 0 if not classes.get(("unattributed", "removed")) and not classes.get(("unattributed", "added")) else 1


if __name__ == "__main__":
    sys.exit(main())
