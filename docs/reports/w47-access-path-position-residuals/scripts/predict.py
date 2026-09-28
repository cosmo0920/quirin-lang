#!/usr/bin/env python3
"""Per-step, per-function forecast from the BEFORE residual-shape stream.

Usage: predict.py RUN_DIR OUT_JSON

Uses `rank_shapes.py`'s join (a refused row belongs to the nearest COND, RET
or IND leaf of its function at or above its line, within 6 lines). Each
planned step admits a predicate over refused rows; steps are cumulative. A
leaf is covered after step k when it has rows and every row satisfies one of
steps 1..k. An Unknown function is forecast **Safe** after step k when every
leaf is covered (`strict`), and **possible** when every non-alias leaf is
(`with-alias`: its alias leaves may be poisoned by a refusal the step
removes, rule 10a, or may stay).

The predicates read the shape rows only, so they over-approximate: a row
the step's code admits may still refuse by its root premise (the relaxed
answer was taken with the address or scope refusal still in place), and a
leaf may carry a second refusal the stream does not name.
"""

import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from rank_shapes import CONTINUATION, POSITIONS, family  # noqa: E402

ADDRESS = {"address", "aggregate-value"}


def step_address_values(position, row):
    features = set(row["features"])
    return (row["node"] == "unique" and "address" in features and features <= ADDRESS
            and position in ("call-argument", "condition"))


def step_scope(position, row):
    return row["node"] == "no-unique-node" and position in (
        "condition", "condition-assignment", "return")


def step_anonymous_enum(position, row):
    return row["node"] == "unique" and set(row["features"]) == {"anonymous-enum"}


def step_return_connective(position, row):
    features = set(row["features"])
    return (position == "return" and row["node"] == "unique" and features
            and features <= {"logical", "conditional", "anonymous-enum"})


STEPS = [
    ("1-address-values", step_address_values),
    ("2-controlling-scope-match", step_scope),
    ("3-anonymous-enum-member", step_anonymous_enum),
    ("4-return-connective", step_return_connective),
]


def main():
    run, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    report = json.loads((run / "resource-diagnostics.json").read_text())
    refused = set()
    shapes = {}
    with open(run / "parity.jsonl") as handle:
        for line in handle:
            row = json.loads(line)
            position = row.get("position")
            if position in POSITIONS and row.get("old_route") == "old_refusal" \
                    and not row.get("new_admitted"):
                refused.add((row["source_sha256"], row["function"], row["line"],
                             tuple(row["texts"]), position))
            elif position == "residual-shape":
                if row.get("admitted_with_every_root_premise"):
                    row = dict(row, node="root-premise")
                shapes[(row["source_sha256"], row["function"], row["line"],
                        tuple(row["texts"]), row["of_position"])] = row
    by_function = collections.defaultdict(list)
    for key in refused:
        by_function[key[1]].append((key[2], key[4], shapes[key]))

    def first_step(position, row):
        for number, (_, predicate) in enumerate(STEPS, 1):
            if predicate(position, row):
                return number
        return None

    forecast = {name: {"strict": [], "with-alias": []} for name, _ in STEPS}
    rows_by_step = collections.Counter()
    for (_, _, _, _, position), row in ((k, shapes[k]) for k in refused):
        rows_by_step[STEPS[first_step(position, row) - 1][0] if first_step(position, row) else "none"] += 1
    for unit in report["units"]:
        for entry in unit.get("functions", []):
            if entry["status"] != "Unknown":
                continue
            leaves = entry.get("diagnostics", [])
            lines = sorted({leaf["line"] for leaf in leaves
                            if family(leaf) in ("COND", "RET", "IND")})
            need = collections.defaultdict(int)  # leaf line -> highest step its rows need
            unsolved = set()
            for line, position, row in by_function.get(entry["function"], ()):
                owners = [l for l in lines if l <= line <= l + CONTINUATION]
                if not owners:
                    continue
                owner = max(owners)
                step = first_step(position, row)
                if step is None:
                    unsolved.add(owner)
                else:
                    need[owner] = max(need[owner], step)
            alias = False
            worst = 0
            blocked = False
            for leaf in leaves:
                name = family(leaf)
                if name == "unknown_alias":
                    alias = True
                    continue
                line = leaf["line"]
                if name in ("COND", "RET", "IND") and line in need and line not in unsolved:
                    worst = max(worst, need[line])
                    continue
                blocked = True
            if blocked or worst == 0:
                continue
            label = STEPS[worst - 1][0]
            name = f"{unit['file']}::{entry['function']}"
            forecast[label]["with-alias"].append(name)
            if not alias:
                forecast[label]["strict"].append(name)
    result = {"steps": [name for name, _ in STEPS], "rows_by_first_step": dict(rows_by_step),
              "forecast": {k: {v: sorted(n) for v, n in t.items()} for k, t in forecast.items()}}
    out.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    print(json.dumps(dict(rows_by_step), indent=1))
    for name, table in result["forecast"].items():
        print(name, "strict", len(table["strict"]), "with-alias", len(table["with-alias"]))
        for f in table["with-alias"]:
            print("   ", f, "(strict)" if f in table["strict"] else "(alias)")


if __name__ == "__main__":
    main()
