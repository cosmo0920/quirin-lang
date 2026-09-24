#!/usr/bin/env python3
"""Attribute every BEFORE -> AFTER diagnostic delta to the declared libc contracts.

Usage: attribute.py BEFORE_DIR AFTER_DIR UNIT_TEXT_DIR OUT.json MODULE.qcore...

BEFORE_DIR / AFTER_DIR hold `resource-diagnostics.json` from one audit binary
run with and without the card's modules selected. UNIT_TEXT_DIR is
`unit_texts.py`'s output: each unit's analysed `-E -P` text, which the leaf
`line` coordinate indexes. The MODULE list is the card's new contract modules;
the declared names D are read from their `extern fn` lines.

A function whose status moves to `Safe` supplies a validated W4 summary to its
callers, so its name joins the decided set: S = names of functions that move.
Each changed leaf is accepted under exactly one class, first match wins:

  declared-callee   removed `call to uncontracted function X`, X in D
  newly-safe-callee removed `call to uncontracted function F`, F in S
  same-line         any other removed or added leaf whose analysed line calls
                    (`NAME (` / `NAME(`) a name in D or S
  same-function     any other changed leaf in a function that has at least one
                    leaf of the three classes above (state downstream of a
                    decided call: a binding the old refusal poisoned)

Anything else is unattributed. Every status move must be in a function with a
decided leaf. A unit's `diagnostic_counts` and its count-summary `reason` must
move by exactly the leaf delta of its own functions; any other unit field may
move only in a unit where a function changed status, and unit status moves are
listed. Top-level fields other than `diagnostic_counts` must not move. Exits 0
only when attribution is complete.
"""

import collections
import json
import pathlib
import re
import sys

UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")


def load(directory):
    return json.loads((pathlib.Path(directory) / "resource-diagnostics.json").read_text())


def functions(data):
    return {(u["library"], u["file"], f["function"]): f for u in data["units"] for f in u["functions"]}


def calls(line, names):
    return sorted(n for n in names if re.search(r"(?<![A-Za-z0-9_])" + re.escape(n) + r"\s*\(", line))


def main() -> int:
    before, after = load(sys.argv[1]), load(sys.argv[2])
    texts, out = pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4])
    declared = set()
    for module in sys.argv[5:]:
        text = "\n".join(l.split("//", 1)[0] for l in pathlib.Path(module).read_text().split("\n"))
        declared |= set(re.findall(r"extern fn (\w+)\(", text))
    pairing = json.loads((texts / "pairing.json").read_text())
    lines = {}
    for key, entry in pairing.items():
        if entry.get("text"):
            lines[key] = (texts / entry["text"]).read_text(errors="replace").split("\n")
    fb, fa = functions(before), functions(after)
    assert set(fb) == set(fa), "function set changed"
    moves = {k: (fb[k]["status"], fa[k]["status"]) for k in fb if fb[k]["status"] != fa[k]["status"]}
    safe_names = {k[2] for k, (_, new) in moves.items() if new == "Safe"}
    decided = declared | safe_names
    rows, per_function = [], collections.defaultdict(list)
    for key in sorted(fb):
        leaf = lambda d: (d["line"], d["category"], d["description"], d.get("origin"))
        old = collections.Counter(leaf(d) for d in fb[key].get("diagnostics") or [])
        new = collections.Counter(leaf(d) for d in fa[key].get("diagnostics") or [])
        if old == new:
            continue
        unit_lines = lines.get(f"{key[0]}/{key[1]}")
        for direction, delta in (("removed", old - new), ("added", new - old)):
            for (line, category, description, origin), count in sorted(delta.items()):
                text = unit_lines[line - 1] if unit_lines and 0 < line <= len(unit_lines) else None
                match = UNCONTRACTED.fullmatch(description)
                if direction == "removed" and match and match.group(1) in declared:
                    kind = "declared-callee"
                elif direction == "removed" and match and match.group(1) in safe_names:
                    kind = "newly-safe-callee"
                elif text is not None and calls(text, decided):
                    kind = "same-line"
                else:
                    kind = None
                row = {
                    "library": key[0], "unit": key[1], "function": key[2],
                    "direction": direction, "line": line, "origin": origin,
                    "category": category, "description": description, "count": count,
                    "class": kind, "line_calls": calls(text, decided) if text else None,
                }
                rows.append(row)
                per_function[key].append(row)
    for key, entries in per_function.items():
        anchored = any(r["class"] for r in entries)
        for r in entries:
            if r["class"] is None and anchored:
                r["class"] = "same-function"
    unattributed = [r for r in rows if r["class"] is None]
    unanchored_moves = [
        list(k) for k in moves if not any(r["class"] != "same-function" for r in per_function.get(k, []))
    ]
    unit_changes = []
    moved_units = {(k[0], k[1]) for k in moves}
    leaf_delta = collections.defaultdict(collections.Counter)
    for r in rows:
        leaf_delta[(r["library"], r["unit"])][r["category"]] += r["count"] * (1 if r["direction"] == "added" else -1)
    summary_line = re.compile(
        r"resource projection unresolved: (\d+) unsupported operation\(s\), "
        r"(\d+) unresolved alias\(es\), (\d+) custody summary conflict\(s\)"
    )
    for ub, ua in zip(before["units"], after["units"]):
        assert (ub["library"], ub["file"]) == (ua["library"], ua["file"])
        key = (ub["library"], ub["file"])
        diff = sorted(k for k in set(ub) | set(ua) if k != "functions" and ub.get(k) != ua.get(k))
        if not diff:
            continue
        delta = leaf_delta[key]
        # The unit's category counts and its count-summary `reason` must move by
        # exactly the leaf delta of its own functions; any other unit field may
        # move only when a function of the unit changed status.
        cb, ca = ub.get("diagnostic_counts") or {}, ua.get("diagnostic_counts") or {}
        counts_ok = all(ca.get(c, 0) - cb.get(c, 0) == delta.get(c, 0) for c in set(cb) | set(ca) | set(delta))
        reason_ok = True
        if ub.get("reason") != ua.get("reason"):
            rb, ra = summary_line.fullmatch(ub.get("reason") or ""), summary_line.fullmatch(ua.get("reason") or "")
            reason_ok = bool(rb and ra) and [int(ra.group(i)) - int(rb.group(i)) for i in (1, 2, 3)] == [
                delta.get("unsupported", 0), delta.get("unknown_alias", 0), delta.get("summary_conflict", 0)
            ]
        other = [k for k in diff if k not in ("reason", "diagnostic_counts")]
        unit_changes.append({
            "unit": f"{ub['library']}/{ub['file']}", "fields": diff,
            "status": [ub["status"], ua["status"]],
            "has_function_move": key in moved_units,
            "counts_match_leaf_delta": counts_ok,
            "reason_matches_leaf_delta": reason_ok,
            "other_fields": other,
        })
    unexplained_units = [
        u for u in unit_changes
        if not (u["counts_match_leaf_delta"] and u["reason_matches_leaf_delta"])
        or (u["other_fields"] and not u["has_function_move"])
    ]
    top = sorted(k for k in set(before) | set(after) if k not in ("units", "diagnostic_counts") and before.get(k) != after.get(k))
    classes = collections.Counter()
    by_class_shape = collections.Counter()
    for r in rows:
        classes[(r["direction"], r["class"])] += r["count"]
        shape = re.sub(r"`[^`]*`", "`_`", r["description"])
        by_class_shape[(r["direction"], r["class"], r["category"], shape)] += r["count"]
    callee_removed = collections.Counter()
    for r in rows:
        if r["class"] in ("declared-callee", "newly-safe-callee"):
            callee_removed[UNCONTRACTED.fullmatch(r["description"]).group(1)] += r["count"]
    move_kinds = collections.Counter(f"{a} -> {b}" for a, b in moves.values())
    summary = {
        "declared_names": len(declared),
        "unpaired_units": sorted(k for k, v in pairing.items() if not v.get("text")),
        "leaves_removed": sum(r["count"] for r in rows if r["direction"] == "removed"),
        "leaves_added": sum(r["count"] for r in rows if r["direction"] == "added"),
        "functions_with_leaf_changes": len(per_function),
        "by_class": {f"{d} {c}": n for (d, c), n in sorted(classes.items(), key=str)},
        "by_class_and_shape": [
            {"direction": d, "class": c, "category": cat, "shape": s, "leaves": n}
            for (d, c, cat, s), n in sorted(by_class_shape.items(), key=str)
        ],
        "removed_uncontracted_by_callee": dict(sorted(callee_removed.items(), key=lambda kv: (-kv[1], kv[0]))),
        "status_moves": dict(move_kinds),
        "moved_functions": sorted([list(k) + [a, b] for k, (a, b) in moves.items()]),
        "newly_safe_names": sorted(safe_names),
        "unit_changes": unit_changes,
        "top_level_changes": top,
        "diagnostic_counts": {"before": before.get("diagnostic_counts"), "after": after.get("diagnostic_counts")},
        "unattributed": len(unattributed),
        "unanchored_moves": unanchored_moves,
        "unexplained_unit_changes": unexplained_units,
    }
    summary["attribution_complete"] = not unattributed and not unanchored_moves and not unexplained_units and not [
        t for t in top if t != "diagnostic_counts"
    ]
    out.write_text(json.dumps({"summary": summary, "leaves": rows}, indent=1) + "\n")
    brief = {k: v for k, v in summary.items() if k not in ("by_class_and_shape", "moved_functions", "unit_changes")}
    brief["unit_changes"] = len(unit_changes)
    brief["unit_status_moves"] = [u for u in unit_changes if u["status"][0] != u["status"][1]]
    print(json.dumps(brief, indent=1))
    return 0 if summary["attribution_complete"] else 1


if __name__ == "__main__":
    sys.exit(main())
