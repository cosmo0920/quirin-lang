#!/usr/bin/env python3
"""attribute.py COMPARE_JSON PARITY_JSONL OUT_JSON -- card CG10-COMMON-TYPED-PLACE:
attribute every moving corpus function of one compare.py result.

For each moved (unit, function): its status move, the leaves removed and
added, and the `typed-place` parity rows this run recorded for it (the place
decisions: root, steps, admitted or the refusal). A function is attributed to

* `tracked-place` when an admitted typed-place row names it;
* `occurrence` when no such row does but a removed or added leaf's line hosts
  two or more identical admitted statement texts in the parity stream (I04);
* `callee-summary` when a removed leaf names a callee that moved to Safe;
* `premise-3` when neither holds and the status moved Safe -> non-Safe
  (the W44-AGGREGATE-PREMISE3-MEMBER-NAME repair; confirmed by reading);
* `unattributed` otherwise (a finding to read).

It also joins CG00's frozen CG10 list (predictions/CG10.md): which moved
functions were predicted, and the predicted verdict. Prints a summary; writes
the full rows to OUT_JSON.
"""
import collections
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
PREDICTION = HERE.parents[1] / "cg00-clang-gap-baseline/predictions/CG10.md"


def predicted():
    rows = {}
    for line in PREDICTION.read_text().splitlines():
        match = re.match(r"- `([^`]+)::([^`]+)` — (Safe|Unknown \(alias\))", line)
        if match:
            rows[(match.group(1), match.group(2))] = match.group(3)
    return rows


def main() -> int:
    compare = json.loads(pathlib.Path(sys.argv[1]).read_text())
    places = collections.defaultdict(list)
    statement_texts = collections.defaultdict(collections.Counter)
    with open(sys.argv[2]) as stream:
        for line in stream:
            row = json.loads(line)
            if row.get("position") == "typed-place":
                places[row["function"]].append(row)
            elif "position" not in row:
                statement_texts[(row["function"], row["line"])][tuple(row["texts"])] += 1
    frozen = predicted()
    safe_now = {moved["function"] for moved in compare["moved_functions"]
                if moved["status"][1] == "Safe" and moved["status"][0] != "Safe"}
    rows = []
    for moved in compare["moved_functions"]:
        unit = pathlib.Path(moved["unit"]).name
        function = moved["function"]
        admitted = [row for row in places.get(function, []) if row["new_admitted"]]
        removed_lines = {leaf[2] for leaf in moved["leaves_removed"] + moved["leaves_added"]}
        duplicated = any(
            max(statement_texts[(function, line)].values(), default=0) > 1
            for line in removed_lines
        )
        before, after = moved["status"]
        if admitted:
            cause = "tracked-place"
        elif duplicated:
            cause = "occurrence"
        elif any(f"`{name}`" in (leaf[1] or "") for leaf in moved["leaves_removed"]
                 for name in safe_now):
            cause = "callee-summary"
        elif before == "Safe" and after != "Safe":
            cause = "premise-3"
        else:
            cause = "unattributed"
        rows.append({
            "unit": unit,
            "function": function,
            "status": moved["status"],
            "cause": cause,
            "predicted": frozen.get((unit, function)),
            "leaves_removed": moved["leaves_removed"],
            "leaves_added": moved["leaves_added"],
            "admitted_places": [
                {"line": row["line"], "texts": " ".join(row["texts"]), "position": row["of_position"],
                 "decisions": [place.get("refusal") or "admitted" for place in row["places"]]}
                for row in admitted
            ],
        })
    moves = collections.Counter((tuple(row["status"]), row["cause"]) for row in rows)
    added = collections.Counter(
        leaf[0] + ":" + (leaf[1] or "")[:60] for row in rows for leaf in row["leaves_added"]
    )
    removed = collections.Counter(
        leaf[0] + ":" + (leaf[1] or "")[:60] for row in rows for leaf in row["leaves_removed"]
    )
    to_safe = [row for row in rows if row["status"][1] == "Safe" and row["status"][0] != "Safe"]
    summary = {
        "moved": len(rows),
        "moves": {f"{a}->{b} {cause}": n for ((a, b), cause), n in sorted(moves.items())},
        "to_safe": len(to_safe),
        "to_safe_predicted": collections.Counter(row["predicted"] for row in to_safe),
        "from_safe": [
            (row["unit"], row["function"], row["status"], row["cause"])
            for row in rows if row["status"][0] == "Safe" and row["status"][1] != "Safe"
        ],
        "to_unsafe": [
            (row["unit"], row["function"], row["status"], row["leaves_added"])
            for row in rows if row["status"][1] == "Unsafe"
        ],
        "leaves_added": dict(added.most_common(20)),
        "leaves_removed": dict(removed.most_common(20)),
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps({"summary": summary, "rows": rows}, indent=1))
    json.dump(summary, sys.stdout, indent=1, default=str)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
