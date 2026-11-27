#!/usr/bin/env python3
"""Compare two runs and attribute every moving function and changed leaf.

Usage: compare_steps.py BEFORE_DIR AFTER_DIR OUT_DIR [--system]

Each directory holds `resource-diagnostics.json` and `parity.jsonl` (plain or
`.gz`). OUT_DIR must not exist. A row is **newly admitted** when AFTER's
parity stream admits it (`new_admitted`, an `old_refusal` row of any
position, statement rows included, or a `return-connective` row) and
BEFORE's does not admit the same (function, line, tokens, position). A
changed leaf is:

* `direct` when a newly admitted row names its function and line, or one of
  the next six lines (a multi-line statement's refusal names its first line);
* otherwise `nondirect`, classified here: `callee-now-safe` (a removed
  uncontracted-call leaf whose callee moved to Safe in the same unit),
  `later-obligation` (an added `unknown_alias` in a function with a newly
  admitted row: a base read later is now reached), `poison-lifted` (a removed
  `unknown_alias` in such a function), `exchanged` (an added refusal on the
  line of a removed one), else `review` (read by hand; listed).

Also writes the soundness interlock (rule 10l's disputed set): for every
function that moves to Safe, the roots of its newly admitted rows and every
`root-declarations` disagreement of kind `ast-binds-lexical-never-bound` in
AFTER for that function, with the AST's binding reason.

With `--system` the unit list is matched by ordinal as well (the system slice
has the same units in both runs); functions are reported per configuration.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

CONTINUATION = 6
ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}


def open_any(path):
    if path.exists():
        return open(path, "rt")
    return gzip.open(path.with_name(path.name + ".gz"), "rt")


def family(d):
    if d["category"] != "unsupported":
        return d["category"]
    text = d["description"]
    if text.startswith("call to uncontracted function"):
        return "UNC"
    if text == "indirect memory access is outside the resource projection":
        return "IND"
    if text == "unmodeled call, aggregate or indirect access in return expression":
        return "RET"
    if text.startswith("unmodeled call, aggregate or indirect access in"):
        return "COND"
    return "OTHER"


def leaf_key(d):
    return json.dumps({k: d.get(k) for k in ("category", "description", "line", "origin")},
                      sort_keys=True)


def admitted_rows(path):
    rows = {}
    disputes = collections.defaultdict(list)
    with open_any(path) as stream:
        for line in stream:
            row = json.loads(line)
            position = row.get("position", "statement")
            if position == "root-declarations":
                by_name = {n["name"]: n for n in row["names"]}
                for d in row["disagreements"]:
                    if "ast-binds-lexical-never-bound" in d["kinds"]:
                        disputes[row["function"]].append(
                            (d["name"], by_name[d["name"]].get("ast_binding")))
                continue
            if row.get("old_route") != "old_refusal" or not row.get("new_admitted"):
                continue
            if "line" not in row:
                continue
            key = (row["function"], row["line"], tuple(row["texts"]), position)
            rows[key] = row
    return rows, disputes


def roots(row):
    names = set(re.findall(r'root: "([^"]+)"', row.get("new_chains") or ""))
    for group in re.findall(r"\bbases: \[([^\]]*)\]", row.get("new_scalar") or ""):
        names |= set(re.findall(r'"([^"]+)"', group))
    return names


def main():
    before_dir, after_dir, out = map(pathlib.Path, sys.argv[1:4])
    out.mkdir(parents=True, exist_ok=False)
    with open_any(before_dir / "resource-diagnostics.json") as stream:
        before = json.load(stream)
    with open_any(after_dir / "resource-diagnostics.json") as stream:
        after = json.load(stream)
    before_rows, _ = admitted_rows(before_dir / "parity.jsonl")
    after_rows, disputes = admitted_rows(after_dir / "parity.jsonl")
    new = {k: v for k, v in after_rows.items() if k not in before_rows}
    lost = {k: v for k, v in before_rows.items() if k not in after_rows}
    new_lines = collections.defaultdict(set)
    new_positions = collections.defaultdict(set)
    for (function, line, _, position) in new:
        new_lines[function].add(line)
        new_positions[(function, line)].add(position)
    if len(before["units"]) != len(after["units"]):
        raise SystemExit("unit denominator changed")

    status = {"before": collections.Counter(), "after": collections.Counter()}
    units = {"before": collections.Counter(), "after": collections.Counter()}
    counts = {"before": collections.Counter(), "after": collections.Counter()}
    moves, leaves = [], []
    for ordinal, (old_unit, new_unit) in enumerate(zip(before["units"], after["units"])):
        if old_unit["file"] != new_unit["file"]:
            raise SystemExit(f"unit identity changed at {ordinal}")
        units["before"][old_unit["status"]] += 1
        units["after"][new_unit["status"]] += 1
        old_rows = {r["function"]: r for r in old_unit.get("functions", [])}
        new_rows = {r["function"]: r for r in new_unit.get("functions", [])}
        if old_rows.keys() != new_rows.keys():
            raise SystemExit(f"function denominator changed at {ordinal}")
        safe_now = {name for name in new_rows
                    if new_rows[name]["status"] == "Safe" and old_rows[name]["status"] != "Safe"}
        for name in sorted(old_rows):
            old, cur = old_rows[name], new_rows[name]
            status["before"][old["status"]] += 1
            status["after"][cur["status"]] += 1
            for d in old.get("diagnostics", []):
                counts["before"][family(d)] += 1
            for d in cur.get("diagnostics", []):
                counts["after"][family(d)] += 1
            a = collections.Counter(leaf_key(d) for d in old.get("diagnostics", []))
            b = collections.Counter(leaf_key(d) for d in cur.get("diagnostics", []))
            base = {"unit": ordinal, "file": new_unit["file"], "library": new_unit.get("library"),
                    "function": name, "before": old["status"], "after": cur["status"]}
            if old["status"] != cur["status"]:
                moves.append(dict(base))
            if a == b:
                continue
            removed = list((a - b).elements())
            added = list((b - a).elements())
            removed_lines = {json.loads(t)["line"] for t in removed}
            admitted_here = bool(new_lines.get(name))
            for change, bag in (("removed", removed), ("added", added)):
                for text in bag:
                    d = json.loads(text)
                    lines = new_lines.get(name, set())
                    hit = [l for l in lines if d["line"] <= l <= d["line"] + CONTINUATION]
                    if hit:
                        kind = "direct"
                        positions = sorted({p for l in hit for p in new_positions[(name, l)]})
                    else:
                        positions = []
                        fam = family(d)
                        callee = re.match(r"call to uncontracted function `([^`]*)`", d["description"])
                        if change == "removed" and callee and callee.group(1) in safe_now:
                            kind = "callee-now-safe"
                        elif fam == "unknown_alias" and admitted_here:
                            kind = "later-obligation" if change == "added" else "poison-lifted"
                        elif change == "added" and d["line"] in removed_lines:
                            kind = "exchanged"
                        else:
                            kind = "review"
                    leaves.append(dict(base, change=change, family=family(d), leaf=d,
                                       attribution=kind, positions=positions))

    tally = collections.Counter(f'{r["change"]}|{r["family"]}|{r["attribution"]}' for r in leaves)
    by_move = []
    for move in moves:
        mine = [r for r in leaves if r["unit"] == move["unit"] and r["function"] == move["function"]]
        kinds = sorted({r["attribution"] for r in mine})
        positions = sorted({p for r in mine for p in r["positions"]})
        rows = [dict(texts=" ".join(k[2]), line=k[1], position=k[3], roots=sorted(roots(v)))
                for k, v in new.items() if k[0] == move["function"]]
        disputed = sorted({(n, reason) for n, reason in disputes.get(move["function"], [])})
        root_names = {r for row in rows for r in row["roots"]}
        by_move.append(dict(move, leaf_attributions=kinds, positions=positions,
                            new_rows=rows,
                            disputed_roots_of_new_rows=[
                                [n, reason] for n, reason in disputed if n in root_names],
                            disputed_names_in_function=[[n, reason] for n, reason in disputed]))
    result = {
        "functions": {k: dict(v) for k, v in status.items()},
        "units": {k: dict(v) for k, v in units.items()},
        "leaf_families": {k: dict(sorted(v.items())) for k, v in counts.items()},
        "move_counts": dict(collections.Counter(f'{m["before"]}->{m["after"]}' for m in moves)),
        "moves": by_move,
        "leaf_attribution": dict(sorted(tally.items())),
        "newly_admitted_rows": dict(collections.Counter(k[3] for k in new)),
        "no_longer_admitted_rows": dict(collections.Counter(k[3] for k in lost)),
        "interlock_violations": [m for m in by_move
                                 if m["after"] == "Safe" and m["disputed_roots_of_new_rows"]],
    }
    (out / "comparison.json").write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    with gzip.open(out / "changed-leaves.jsonl.gz", "wt") as stream:
        for row in leaves:
            stream.write(json.dumps(row, sort_keys=True) + "\n")
    summary = {k: result[k] for k in ("functions", "units", "move_counts", "leaf_attribution",
                                      "newly_admitted_rows", "no_longer_admitted_rows")}
    summary["interlock_violations"] = len(result["interlock_violations"])
    print(json.dumps(summary, indent=1, sort_keys=True))
    for m in by_move:
        print(m["before"], "->", m["after"], m["file"], m["function"], m["leaf_attributions"],
              m["positions"], "disputed:", m["disputed_roots_of_new_rows"])


if __name__ == "__main__":
    main()
