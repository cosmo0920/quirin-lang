#!/usr/bin/env python3
"""Card W43-INDIRECT-ACCESS-GENERAL: the pre-implementation forecast.

Applies the Python mirror of the planned classifier (`typed_scalar.py`) to
every refused statement the census streamed, joined to the BEFORE leaves as
`shape_census.py` joins them, and reports per admitted shape:

* statements admitted by the classifier (the lexical half);
* leaves whose every statement is admitted - predicted to leave the refusal
  family they are in (removed, or exchanged for a flow refusal);
* the static base axis of each admitted dereference (derived / owned /
  neither), which is what the flow half decides on;
* Unknown functions whose EVERY leaf, in every family, is predicted to clear,
  listed by name - the upper bound on function moves before the flow discount.

Usage: predict.py RUN_DIR OUT_JSON OUT_TXT
"""

import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import shape_census as census  # noqa: E402
import typed_scalar  # noqa: E402

TARGETS = {
    census.INDIRECT,
    census.ASSIGNMENT_EXPRESSION,
    census.UNRECOGNIZED_DESTINATION,
    census.INCREMENT,
}


def intercepted(description):
    """The refusals the planned route is consulted for."""
    return description in TARGETS or census.STORE_THROUGH.fullmatch(description) is not None


def base_axis(record, base):
    if base in record.get("derived_names", []):
        return "derived"
    if base in record.get("owned_names", []):
        return "owned"
    return "untracked-or-other"


def main():
    run = pathlib.Path(sys.argv[1])
    out_json = pathlib.Path(sys.argv[2])
    out_txt = pathlib.Path(sys.argv[3])
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    indirect = census.load_stream(run / "indirect-census.jsonl", lambda record: census.INDIRECT)
    statements = census.load_stream(
        run / "statement-census.jsonl", lambda record: census.outcome_family(record["outcome"])
    )
    assignment = census.match_units([indirect, statements], unit_keys)

    unknown = {f for f, d in functions.items() if d["status"] == "Unknown"}
    # `--all-statuses` counts leaves in every function (the corpus-wide
    # `unsupported` delta); the default counts Unknown functions only, as the
    # blocker census does. Function forecasts are always over Unknown ones.
    domain = set(functions) if "--all-statuses" in sys.argv else unknown
    leaf_verdict = {}
    shape_leaves = collections.Counter()
    shape_statements = collections.Counter()
    shape_family = collections.Counter()
    shape_functions = collections.defaultdict(set)
    base_axes = collections.Counter()
    unexplained = 0
    for fkey in domain:
        ukey, function = fkey[:2], fkey[2]
        seq = assignment.get(ukey)
        for line, description in functions[fkey]["leaves"]:
            if not intercepted(description):
                continue
            key = (function, line, description)
            records = []
            if seq is not None:
                records = statements.get(seq, {}).get(key, [])
                if description == census.INDIRECT:
                    records = indirect.get(seq, {}).get(key, []) or records
            if not records:
                unexplained += 1
                leaf_verdict[(fkey, line, description)] = None
                continue
            plans = [typed_scalar.classify(record) for record in records]
            if all(plans):
                shapes = sorted({plan["shape"] for plan in plans})
                leaf_verdict[(fkey, line, description)] = shapes
                label = shapes[0] if len(shapes) == 1 else "+".join(shapes)
                shape_leaves[label] += 1
                shape_family[(label, census.normalize(description))] += 1
                shape_functions[label].add(fkey)
                for record, plan in zip(records, plans):
                    shape_statements[plan["shape"]] += 1
                    for base in plan["bases"]:
                        base_axes[(plan["shape"], base_axis(record, base))] += 1
            else:
                leaf_verdict[(fkey, line, description)] = None

    def clears(fkey):
        for line, description in functions[fkey]["leaves"]:
            if not intercepted(description):
                return False
            if not leaf_verdict.get((fkey, line, description)):
                return False
        return True

    decided = sorted(f for f in unknown if clears(f))
    # Transitive W4 forecast: a caller in the same unit whose remaining leaves
    # are only `call to uncontracted function `NAME`` for NAMEs predicted to
    # gain a validated summary (a newly decided function) - iterated. This is
    # a forecast of constraint 4's route, not a claim that the summary will
    # validate (the callee must end Safe and its summary must match).
    summarised = {f for f in decided}
    transitive = []
    changed = True
    while changed:
        changed = False
        names = {(f[:2], f[2]) for f in summarised}
        for f in sorted(unknown - summarised):
            ok = True
            for line, description in functions[f]["leaves"]:
                if intercepted(description) and leaf_verdict.get((f, line, description)):
                    continue
                match = census.re.fullmatch(r"call to uncontracted function `([^`]*)`", description)
                if match and (f[:2], match.group(1)) in names:
                    continue
                ok = False
                break
            if ok:
                summarised.add(f)
                transitive.append(f)
                changed = True
    # Two-blocker functions reduced to a single remaining family.
    reduced = collections.Counter()
    for f in unknown:
        families = {census.normalize(d) for _, d in functions[f]["leaves"]}
        remaining = {
            census.normalize(d)
            for line, d in functions[f]["leaves"]
            if not (intercepted(d) and leaf_verdict.get((f, line, d)))
        }
        if remaining and remaining != families and len(remaining) == 1:
            reduced[next(iter(remaining))] += 1
    decided_rows = []
    for f in decided:
        shapes = sorted(
            {
                shape
                for line, d in functions[f]["leaves"]
                for shape in (leaf_verdict.get((f, line, d)) or [])
            }
        )
        decided_rows.append({"function": "/".join(str(p) for p in f), "leaves": len(functions[f]["leaves"]), "shapes": shapes})
    result = {
        "run": str(run),
        "mirror_gaps": typed_scalar.MIRROR_GAPS,
        "leaves_predicted_to_leave_their_family": sum(shape_leaves.values()),
        "unexplained_intercepted_leaves": unexplained,
        "by_shape": {
            label: {
                "leaves": count,
                "functions": len(shape_functions[label]),
                "families": {fam: n for (lab, fam), n in shape_family.items() if lab == label},
            }
            for label, count in shape_leaves.most_common()
        },
        "statements_by_shape": dict(shape_statements.most_common()),
        "base_axes": {f"{shape} | {axis}": n for (shape, axis), n in sorted(base_axes.items())},
        "functions_all_leaves_cleared": decided_rows,
        "functions_reduced_to_one_family": dict(reduced.most_common()),
        "transitive_callers_forecast": ["/".join(str(p) for p in f) for f in transitive],
    }
    out_json.write_text(json.dumps(result, indent=1) + "\n")
    lines = [f"run: {run}", f"leaves predicted to leave their family: {result['leaves_predicted_to_leave_their_family']}",
             f"unexplained intercepted leaves: {unexplained}", "", "by shape (leaves, functions, families):"]
    for label, row in result["by_shape"].items():
        fams = ", ".join(f"{k[:30]}={v}" for k, v in row["families"].items())
        lines.append(f"  {row['leaves']:6} {row['functions']:5}  {label:40} {fams}")
    lines += ["", "dereference bases by static axis:"]
    for key, n in result["base_axes"].items():
        lines.append(f"  {n:6}  {key}")
    lines += ["", f"Unknown functions whose every leaf is predicted to clear: {len(decided)}"]
    for row in decided_rows:
        lines.append(f"  {row['function']}  leaves={row['leaves']}  {','.join(row['shapes'])}")
    lines += ["", f"transitive callers forecast (W4 summaries of the above): {len(transitive)}"]
    for f in transitive:
        lines.append("  " + "/".join(str(p) for p in f))
    lines += ["", "functions reduced to exactly one remaining family:"]
    for fam, n in result["functions_reduced_to_one_family"].items():
        lines.append(f"  {n:5}  {fam}")
    out_txt.write_text("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
