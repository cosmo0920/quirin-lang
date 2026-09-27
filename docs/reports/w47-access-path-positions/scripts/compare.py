#!/usr/bin/env python3
"""Compare two complete audit runs (corpus or system) and attribute every change.

Usage:
  compare.py BEFORE_DIR AFTER_DIR OUT_DIR [--prediction PREDICTION_JSON] [--p1 P1_JSON]

BEFORE_DIR and AFTER_DIR each hold `resource-diagnostics.json` and
`parity.jsonl` (plain or `.gz`). OUT_DIR must not exist. Writes:

* `comparison.json`: function and unit status counts, leaf family counts,
  every moving function (with its before/after diagnostics), the position
  parity row counts, and the forecast comparison when a prediction is given;
* `changed-leaves.jsonl.gz`: every added or removed diagnostic leaf of every
  changed function, with its origin and its attribution: `direct` when an
  admitted W47 position row of the AFTER parity stream names the same
  function and line, otherwise `nondirect` (reviewed by reading);
* `changed-functions.jsonl.gz`: every function whose diagnostic multiset
  changed.

Rows are keyed by (unit ordinal, function). Nothing is filtered out: every
Unknown stays in every denominator.
"""

import collections
import gzip
import io
import json
import pathlib
import sys

POSITIONS = {"condition", "condition-assignment", "return", "call-argument"}


def open_any(path):
    if path.exists():
        return open(path, "rt")
    gz = path.with_name(path.name + ".gz")
    return gzip.open(gz, "rt")


def load(directory):
    with open_any(directory / "resource-diagnostics.json") as stream:
        diagnostics = json.load(stream)
    rows = []
    with open_any(directory / "parity.jsonl") as stream:
        for line in stream:
            if line.strip():
                rows.append(json.loads(line))
    return diagnostics, rows


def leaf_key(d):
    return json.dumps(
        {k: d.get(k) for k in ("category", "description", "line", "origin")}, sort_keys=True
    )


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


def writer(path):
    return io.TextIOWrapper(gzip.GzipFile(filename=str(path), mode="wb", mtime=0), encoding="utf-8")


def main():
    before_dir, after_dir, out = map(pathlib.Path, sys.argv[1:4])
    options = sys.argv[4:]
    prediction = None
    p1 = None
    if "--prediction" in options:
        prediction = json.load(open(options[options.index("--prediction") + 1]))
    if "--p1" in options:
        text = open(options[options.index("--p1") + 1]).read()
        p1 = json.loads(text[text.index("\n") + 1:])
    out.mkdir(parents=True, exist_ok=False)
    before, before_parity = load(before_dir)
    after, after_parity = load(after_dir)
    if len(before["units"]) != len(after["units"]):
        raise SystemExit("unit denominator changed")

    admitted = collections.defaultdict(list)
    position_rows = collections.Counter()
    for row in after_parity:
        position = row.get("position")
        if position:
            key = (position, row.get("old_route"), bool(row.get("new_admitted")))
            position_rows["|".join(map(str, key))] += 1
        if position in POSITIONS and row.get("old_route") == "old_refusal" and row.get("new_admitted"):
            admitted[(row["function"], row["line"])].append(position)

    status = {"before": collections.Counter(), "after": collections.Counter()}
    units = {"before": collections.Counter(), "after": collections.Counter()}
    leaves = {"before": collections.Counter(), "after": collections.Counter()}
    moves = []
    changed_rows = []
    leaf_rows = []
    attribution = collections.Counter()
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
            old_leaves = collections.Counter(leaf_key(d) for d in old.get("diagnostics", []))
            new_leaves = collections.Counter(leaf_key(d) for d in new.get("diagnostics", []))
            for d in old.get("diagnostics", []):
                leaves["before"][family(d)] += 1
            for d in new.get("diagnostics", []):
                leaves["after"][family(d)] += 1
            base = {"unit": ordinal, "library": new_unit["library"], "file": new_unit["file"],
                    "function": name, "before": old["status"], "after": new["status"]}
            if old["status"] != new["status"]:
                moves.append(dict(base))
            if old_leaves == new_leaves:
                continue
            removed = old_leaves - new_leaves
            added = new_leaves - old_leaves
            changed_rows.append(dict(base, removed=sorted(removed.elements()),
                                     added=sorted(added.elements())))
            same_function_admitted = sorted({line for (function, line) in admitted if function == name})
            for change, bag in (("removed", removed), ("added", added)):
                for text in bag.elements():
                    d = json.loads(text)
                    positions = admitted.get((name, d["line"]))
                    kind = "direct" if positions else "nondirect"
                    attribution[f"{change}|{family(d)}|{kind}"] += 1
                    leaf_rows.append(dict(base, change=change, family=family(d), leaf=d,
                                          attribution=kind, positions=sorted(set(positions or [])),
                                          admitted_lines_in_function=same_function_admitted[:50]))

    result = {
        "functions": {k: dict(v) for k, v in status.items()},
        "units": {k: dict(v) for k, v in units.items()},
        "leaf_families": {k: dict(sorted(v.items())) for k, v in leaves.items()},
        "moves": moves,
        "move_counts": dict(collections.Counter(f'{m["before"]}->{m["after"]}' for m in moves)),
        "changed_functions": len(changed_rows),
        "leaf_attribution": dict(sorted(attribution.items())),
        "after_parity_rows": len(after_parity),
        "before_parity_rows": len(before_parity),
        "after_position_rows": dict(sorted(position_rows.items())),
    }

    if prediction is not None:
        after_status = {}
        for ordinal, unit in enumerate(after["units"]):
            for row in unit.get("functions", []):
                after_status.setdefault((unit["file"], row["function"]), []).append(row["status"])
        forecast = []
        tally = collections.Counter()
        predicted_names = set()
        for row in prediction["rows"]:
            found = [s for (file, name), ss in after_status.items()
                     if name == row["function"] and file.endswith(row["file"]) for s in ss]
            observed = found[0] if found and len(set(found)) == 1 else "ambiguous"
            predicted_names.add(row["function"])
            outcome = "match" if observed == row["predicted"] else "miss"
            tally[f'{row["step"]}|{row["predicted"]}->{observed}|{outcome}'] += 1
            forecast.append(dict(row, observed=observed, outcome=outcome))
        unforecast = [m for m in moves if m["function"] not in predicted_names]
        result["forecast"] = {"tally": dict(sorted(tally.items())), "rows": forecast,
                              "unforecast_moves": unforecast}
        if p1 is not None:
            p1_tally = collections.Counter()
            for row in forecast:
                p1_outcome = "match" if row["observed"] == row["p1_verdict"] else "miss"
                p1_tally[f'{row["p1_verdict"]}->{row["observed"]}|{p1_outcome}'] += 1
            result["forecast"]["p1_tally"] = dict(sorted(p1_tally.items()))

    (out / "comparison.json").write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    with writer(out / "changed-leaves.jsonl.gz") as stream:
        for row in sorted(leaf_rows, key=lambda r: (r["unit"], r["function"], r["change"], r["leaf"]["line"], json.dumps(r["leaf"], sort_keys=True))):
            stream.write(json.dumps(row, sort_keys=True) + "\n")
    with writer(out / "changed-functions.jsonl.gz") as stream:
        for row in changed_rows:
            stream.write(json.dumps(row, sort_keys=True) + "\n")
    summary = {k: result[k] for k in ("functions", "units", "move_counts", "changed_functions",
                                      "leaf_attribution")}
    summary["leaf_families"] = result["leaf_families"]
    if "forecast" in result:
        summary["forecast"] = result["forecast"]["tally"]
        summary["unforecast_moves"] = len(result["forecast"]["unforecast_moves"])
    print(json.dumps(summary, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
