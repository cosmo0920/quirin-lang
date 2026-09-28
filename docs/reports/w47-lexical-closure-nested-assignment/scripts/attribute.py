#!/usr/bin/env python3
"""Compare two runs and attribute every moved function and changed leaf to rule 10o.

Usage: attribute.py BEFORE_DIR AFTER_DIR OUT_DIR

Each directory holds `resource-diagnostics.json` and `parity.jsonl` (plain or
`.gz`). OUT_DIR must not exist. For every function whose diagnostic multiset
changed, the AFTER `root-declarations` parity row gives the names the join
binds (`ast_binds` while the extended lexical closure calls them never bound:
**join**) and the names only the nested reading binds (`nested_reading_binds`:
**nested**). A changed leaf is attributed:

* `typed-row` — a P2/P3 typed row at the leaf's (function, line) was admitted
  in BEFORE and its admission or its chains' `root_never_bound` changed in
  AFTER; sub-classed by whether a root of that row is a join or a nested name;
* `alias-root` — an `unknown_alias` leaf whose variable's root is a join or a
  nested name;
* `nested-consumer` — the function has a nested name and no typed row explains
  the leaf: one of the non-typed `never_bound` consumers (C02-P, W44, O04-B08,
  rule 5j/5l) now refuses through it;
* `unexplained` — none of the above (read by hand; must be empty or justified).

Writes `comparison.json`, `changed-leaves.jsonl.gz`, `moves.json`.
"""

import collections
import gzip
import io
import json
import pathlib
import re
import sys

ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}


def open_any(path):
    if path.exists():
        return open(path, "rt")
    return gzip.open(path.with_name(path.name + ".gz"), "rt")


def load(directory):
    with open_any(directory / "resource-diagnostics.json") as stream:
        diagnostics = json.load(stream)
    rows = []
    with open_any(directory / "parity.jsonl") as stream:
        for line in stream:
            if line.strip():
                rows.append(json.loads(line))
    return diagnostics, rows


def family(d):
    description = d["description"]
    if d["category"] != "unsupported":
        return d["category"]
    if description.startswith("call to uncontracted function"):
        return "unsupported:UNC"
    if description == "indirect memory access is outside the resource projection":
        return "unsupported:IND"
    if description.startswith("unmodeled call, aggregate or indirect access in return"):
        return "unsupported:RET"
    if description.startswith("unmodeled call, aggregate or indirect access in"):
        return "unsupported:COND"
    return "unsupported:OTHER"


def leaf_key(d):
    return json.dumps({k: d.get(k) for k in ("category", "description", "line", "origin")},
                      sort_keys=True)


def typed_rows(rows):
    """(function, line) -> set of (position, texts, admitted, chains) of typed rows."""
    out = collections.defaultdict(set)
    for row in rows:
        position = row.get("position", "statement")
        if position == "root-declarations" or row.get("old_route") != "old_refusal":
            continue
        out[(row.get("function"), row.get("line"))].add((
            position, " ".join(row.get("texts") or []), bool(row.get("new_admitted")),
            row.get("new_chains") or "", row.get("new_scalar") or ""))
    return out


def roots_of(entry):
    _, _, _, chains, scalar = entry
    roots = set(re.findall(r'root: "([^"]+)"', chains))
    for group in re.findall(r'\bbases: \[([^\]]*)\]', scalar):
        roots |= set(re.findall(r'"([^"]+)"', group))
    return roots


def root_facts(rows):
    """function -> {'join': names, 'nested': names} over every analysis of it."""
    out = collections.defaultdict(lambda: {"join": set(), "nested": set()})
    for row in rows:
        if row.get("position") != "root-declarations":
            continue
        facts = out[row["function"]]
        for name in row.get("names", []):
            if name.get("nested_reading_binds"):
                facts["nested"].add(name["name"])
            if name.get("ast_binds") and name.get("lexical_never_bound") and row.get("lexical_readable"):
                facts["join"].add(name["name"])
    return out


def writer(path):
    return io.TextIOWrapper(gzip.GzipFile(filename=str(path), mode="wb", mtime=0), encoding="utf-8")


def main():
    before_dir, after_dir, out = map(pathlib.Path, sys.argv[1:4])
    out.mkdir(parents=True, exist_ok=False)
    before, before_rows = load(before_dir)
    after, after_rows = load(after_dir)
    if len(before["units"]) != len(after["units"]):
        raise SystemExit("unit denominator changed")
    typed_before, typed_after = typed_rows(before_rows), typed_rows(after_rows)
    facts = root_facts(after_rows)
    status = {"before": collections.Counter(), "after": collections.Counter()}
    units = {"before": collections.Counter(), "after": collections.Counter()}
    families = {"before": collections.Counter(), "after": collections.Counter()}
    attribution = collections.Counter()
    moves, leaf_rows = [], []
    changed_functions = 0
    for ordinal, (old_unit, new_unit) in enumerate(zip(before["units"], after["units"])):
        if (old_unit["file"], old_unit["library"]) != (new_unit["file"], new_unit["library"]):
            raise SystemExit(f"unit identity changed at {ordinal}")
        units["before"][old_unit["status"]] += 1
        units["after"][new_unit["status"]] += 1
        old_rows = {r["function"]: r for r in old_unit.get("functions", [])}
        new_rows = {r["function"]: r for r in new_unit.get("functions", [])}
        if old_rows.keys() != new_rows.keys():
            raise SystemExit(f"function denominator changed at {ordinal}")
        for name in sorted(old_rows):
            old, new = old_rows[name], new_rows[name]
            status["before"][old["status"]] += 1
            status["after"][new["status"]] += 1
            for d in old.get("diagnostics", []):
                families["before"][family(d)] += 1
            for d in new.get("diagnostics", []):
                families["after"][family(d)] += 1
            old_leaves = collections.Counter(leaf_key(d) for d in old.get("diagnostics", []))
            new_leaves = collections.Counter(leaf_key(d) for d in new.get("diagnostics", []))
            base = {"unit": ordinal, "library": new_unit["library"], "file": new_unit["file"],
                    "function": name, "before": old["status"], "after": new["status"]}
            if old_leaves == new_leaves and old["status"] == new["status"]:
                continue
            changed_functions += 1
            fact = facts.get(name, {"join": set(), "nested": set()})
            reasons = []
            for change, bag in (("removed", old_leaves - new_leaves), ("added", new_leaves - old_leaves)):
                for text in bag.elements():
                    d = json.loads(text)
                    key = (name, d["line"])
                    kind = "unexplained"
                    changed_typed = typed_before.get(key, set()) ^ typed_after.get(key, set())
                    roots = set().union(*[roots_of(e) for e in changed_typed]) if changed_typed else set()
                    if changed_typed:
                        if roots & fact["join"]:
                            kind = "typed-row:join"
                        elif roots & fact["nested"]:
                            kind = "typed-row:nested"
                        else:
                            kind = "typed-row:other-root"
                    elif d["category"] == "unknown_alias":
                        variable = d["description"].split("`")[1] if "`" in d["description"] else ""
                        root = re.split(r"[/\-.\[]", variable)[0] if variable else ""
                        if root in fact["join"]:
                            kind = "alias-root:join"
                        elif root in fact["nested"]:
                            kind = "alias-root:nested"
                    if kind == "unexplained" and fact["nested"]:
                        kind = "nested-consumer"
                    if kind == "unexplained" and fact["join"]:
                        kind = "join-function"
                    attribution[f"{change}|{family(d)}|{kind}"] += 1
                    reasons.append(kind)
                    leaf_rows.append(dict(base, change=change, family=family(d), leaf=d,
                                          attribution=kind, join=sorted(fact["join"]),
                                          nested=sorted(fact["nested"]),
                                          typed_roots=sorted(roots)))
            if old["status"] != new["status"]:
                moves.append(dict(base, join=sorted(fact["join"]), nested=sorted(fact["nested"]),
                                  leaf_reasons=dict(collections.Counter(reasons)),
                                  after_leaves=sorted({leaf_key(d) for d in new.get("diagnostics", [])})))
    result = {
        "functions": {k: dict(v) for k, v in status.items()},
        "units": {k: dict(v) for k, v in units.items()},
        "leaf_families": {k: dict(sorted(v.items())) for k, v in families.items()},
        "move_counts": dict(collections.Counter(f'{m["before"]}->{m["after"]}' for m in moves)),
        "moves": [{k: m[k] for k in ("library", "file", "function", "before", "after", "join",
                                      "nested", "leaf_reasons")} for m in moves],
        "changed_functions": changed_functions,
        "leaf_attribution": dict(sorted(attribution.items())),
    }
    (out / "comparison.json").write_text(json.dumps(result, indent=1) + "\n")
    (out / "moves.json").write_text(json.dumps(moves, indent=1) + "\n")
    with writer(out / "changed-leaves.jsonl.gz") as stream:
        for row in leaf_rows:
            stream.write(json.dumps(row, sort_keys=True) + "\n")
    print(json.dumps({k: result[k] for k in ("functions", "units", "leaf_families", "move_counts",
                                              "changed_functions", "leaf_attribution")}, indent=1))
    for m in result["moves"]:
        print("MOVE", m["function"], m["before"], "->", m["after"], m["join"][:6], m["nested"][:6])


if __name__ == "__main__":
    main()
