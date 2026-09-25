#!/usr/bin/env python3
"""Attribute every leaf the T6 tail widening moves (card W44-LIBC-RESIDUALS).

Usage: attribute_tail.py BEFORE_DIR AFTER_DIR UNIT_TEXT_DIR OUT.json

A copy of `../../w43-libc-contracts/scripts/attribute.py` with its first class
replaced, because stage 1 declares nothing: the base contracts are unchanged
and only the tail rule moved. D is the set of callees that carry a declared
variadic data tail in the shipped corpus (`printf`, `fprintf`, `snprintf`,
`sprintf`, `mk_print`). First match wins:

  t6-literal-tail   removed `arguments beyond the declared parameters of X`
                    whose analysed line carries a call of X with a literal-only
                    tail (literals and predefined identifiers), the T6 shape
  t6-mixed-tail     removed tail refusal whose line carries a call of X with at
                    least one literal-only tail actual, the others being names
                    T1-T5 already admitted (a declared scalar beside a literal
                    the closure had marked)
  t6-tail-statement-spans-lines
                    removed tail refusal whose analysed line (the statement's
                    first) does not spell the call of X at all
  t6-tail-unexplained
                    any other removed tail refusal; counted as unattributed
  newly-safe-callee removed `call to uncontracted function F`, F moved to Safe
  same-line         any other changed leaf whose line calls a name in D or S
  same-function     any other changed leaf in a function with an anchored leaf

The unit, top-level and status-move checks are the original script's.
"""

import collections
import json
import pathlib
import re
import sys

UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")
TAIL = re.compile(r"arguments beyond the declared parameters of `([^`]+)` are outside the resource projection")
TOKEN = re.compile(r'[A-Za-z_][A-Za-z0-9_]*|0[xX][0-9a-fA-F]+[uUlL]*|\d+(?:\.\d*)?(?:[eE][-+]?\d+)?[uUlLfF]*|"(?:[^"\\]|\\.)*"|\'(?:[^\'\\]|\\.)*\'|->|\+\+|--|<<|>>|&&|\|\||[<>=!]=|\S')
PREDEFINED = {"__func__", "__FUNCTION__", "__PRETTY_FUNCTION__"}
NAMED = {"printf": 1, "fprintf": 2, "snprintf": 3, "sprintf": 2, "mk_print": 2}


def literal_calls(line, callee, partial=False):
    """Calls of `callee` on the analysed line whose whole tail is literals and
    predefined identifiers (the T6 shape), lexed as tail_nested_lines.py does.
    With `partial`, count instead the calls with at least one such actual."""
    tokens = TOKEN.findall(line)
    found = 0
    for index, token in enumerate(tokens):
        if token != callee or index + 1 >= len(tokens) or tokens[index + 1] != "(":
            continue
        depth, arguments = 0, [[]]
        for text in tokens[index + 1 :]:
            if text == "(":
                depth += 1
                if depth == 1:
                    continue
            elif text == ")":
                depth -= 1
                if depth == 0:
                    break
            elif text == "," and depth == 1:
                arguments.append([])
                continue
            arguments[-1].append(text)
        rest = arguments[NAMED[callee]:]
        literal = [bool(a) and all(t[0] in "\"'" or t[0].isdigit() or t in PREDEFINED for t in a) for a in rest]
        if rest and (any(literal) if partial else all(literal)):
            found += 1
    return found


def has_call(line, callee):
    return re.search(r"(?<![A-Za-z0-9_])" + re.escape(callee) + r"\s*\(", line) is not None


def load(directory):
    return json.loads((pathlib.Path(directory) / "resource-diagnostics.json").read_text())


def functions(data):
    return {(u["library"], u["file"], f["function"]): f for u in data["units"] for f in u["functions"]}


def calls(line, names):
    return sorted(n for n in names if re.search(r"(?<![A-Za-z0-9_])" + re.escape(n) + r"\s*\(", line))


def main() -> int:
    before, after = load(sys.argv[1]), load(sys.argv[2])
    texts, out = pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4])
    declared = set(NAMED)
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
                tail = TAIL.fullmatch(description)
                if direction == "removed" and tail and text is not None and literal_calls(text, tail.group(1)):
                    kind = "t6-literal-tail"
                elif direction == "removed" and tail and text is not None and literal_calls(text, tail.group(1), True):
                    kind = "t6-mixed-tail"
                elif direction == "removed" and tail and text is not None and not has_call(text, tail.group(1)):
                    kind = "t6-tail-statement-spans-lines"
                elif direction == "removed" and tail:
                    kind = "t6-tail-unexplained"
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
    unattributed = [r for r in rows if r["class"] in (None, "t6-tail-unexplained")]
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
        if r["class"] == "newly-safe-callee":
            callee_removed[UNCONTRACTED.fullmatch(r["description"]).group(1)] += r["count"]
        if r["class"] and r["class"].startswith("t6-"):
            callee_removed["tail:" + TAIL.fullmatch(r["description"]).group(1)] += r["count"]
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
