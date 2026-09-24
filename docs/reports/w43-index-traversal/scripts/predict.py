#!/usr/bin/env python3
"""Card W43-INDEX-TRAVERSAL: the pre-admission forecast from the traversal census.

Input: one census run directory (`run_audit.py ... --census`) holding
`resource-diagnostics.json` and `traversal-census.jsonl`. The census binary is
the census-instrumentation commit, which admits nothing; its records carry the
answers of the very recognisers the admission uses
(`typed_scalar_condition`, `index_traversal::pointer_step`,
`index_traversal::stepped_access`), so the lexical half of this forecast is
the rule's own answer, not a mirror of it.

Join: `unit_seq` is matched to a document unit by greatest overlap of
(function, line) keys (W43-INDIRECT-ACCESS-GENERAL's `match_units`). A leaf
`(unit, function, line, description)` is explained by every distinct census
record at that (function, line) of the matching family, and is forecast to
leave its family only if every such record is recognised.

Forecast classes (each leaf in at most one):

* `condition` - a refused controlling expression whose every record at the
  line is admitted by the typed-condition recogniser. The leaf leaves its
  family; whether the header then admits or exchanges it for an
  `unknown_alias` is the flow half, which this forecast cannot see.
* `step` - an increment / unsupported-assignment / unrecognized-destination
  refusal whose every record is a recognised pointer step or stepped access.
* `gated-step` - the formal-mutation gate's refusal over a recognised step or
  stepped access. The census does not record which names are formals, so this
  class is an UPPER BOUND: a step of a formal keeps the refusal.
* `scalar-alias` - an `unknown_alias` leaf naming a declared scalar of the
  function (the join, copy and return rules for scalar facts), or naming the
  source of a recognised pointer or scalar copy at the same line. A flow
  forecast: a heuristic over names, attributed leaf by leaf after the run.

Function forecast: an Unknown function whose every leaf is in some class is
forecast to move to Safe (an upper bound: exchanges and gated formals are not
discounted). Reported per class and for the union.

Usage: predict.py RUN_DIR OUT_JSON OUT_TXT
"""

import collections
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "w43-indirect-access-general" / "scripts"))

import shape_census as census  # noqa: E402

CONDITION = re.compile(
    r"unmodeled call, aggregate or indirect access in "
    r"(branch condition|loop condition|for-loop condition|do-while condition|switch expression)"
)
POSITION = {
    "branch condition": {"if"},
    "loop condition": {"while"},
    "for-loop condition": {"for"},
    "do-while condition": {"do-while"},
    "switch expression": {"switch"},
}
CONSULTED = {
    "indirect memory access is outside the resource projection",
    "unsupported assignment expression",
    "unrecognized assignment destination",
    "unmodeled increment or decrement expression",
}
GATE = "unmodeled expression may mutate a formal resource identity"
ALIAS = re.compile(r"unresolved resource identity `([^`]*)`")


def load(path):
    """seq -> list of records, de-duplicated (functions are analysed several times)."""
    stream = collections.defaultdict(list)
    seen = set()
    for line in open(path):
        record = json.loads(line)
        blob = line.strip()
        if blob in seen:
            continue
        seen.add(blob)
        stream[record["unit_seq"]].append(record)
    return stream


def main():
    run = pathlib.Path(sys.argv[1])
    out_json = pathlib.Path(sys.argv[2])
    out_txt = pathlib.Path(sys.argv[3])
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    stream = load(run / "traversal-census.jsonl")

    per_seq = {}
    for seq, records in stream.items():
        keys = collections.defaultdict(list)
        names = collections.defaultdict(lambda: {"scalars": set(), "pointers": set()})
        for record in records:
            if record["kind"] == "function":
                names[record["function"]]["scalars"].update(record["scalars"])
                names[record["function"]]["pointers"].update(record["pointers"])
            else:
                keys[(record["function"], record["line"])].append(record)
        per_seq[seq] = (keys, names)
    assignment = census.match_units(
        [{seq: {(f, l, None): [] for (f, l) in keys} for seq, (keys, _) in per_seq.items()}],
        unit_keys,
    )

    classes = collections.Counter()
    classes_unknown = collections.Counter()
    family_totals = collections.Counter()
    family_removed = collections.Counter()
    leaf_class = {}
    unexplained = collections.Counter()
    for fkey, data in functions.items():
        ukey, function = fkey[:2], fkey[2]
        seq = assignment.get(ukey)
        keys, names = per_seq.get(seq, ({}, {}))
        declared = names.get(function, {"scalars": set(), "pointers": set()})
        for line, description in data["leaves"]:
            records = keys.get((function, line), [])
            label = None
            match = CONDITION.fullmatch(description)
            if match:
                family = description
                wanted = POSITION[match.group(1)]
                conditions = [
                    r for r in records if r["kind"] == "condition" and r["position"] in wanted
                ]
                if not conditions:
                    unexplained[family] += 1
                elif all(r["typed_bases"] for r in conditions):
                    label = "condition"
            elif description in CONSULTED or description == GATE:
                family = description
                statements = [
                    r for r in records if r["kind"] == "statement" and r["outcome"] == description
                ]
                if not statements:
                    unexplained[family] += 1
                elif all(r["recognised"] in ("pointer-step", "stepped-access") for r in statements):
                    label = "gated-step" if description == GATE else "step"
            elif ALIAS.fullmatch(description):
                family = "unknown_alias"
                name = ALIAS.fullmatch(description).group(1)
                copies = [
                    r
                    for r in records
                    if r["kind"] == "statement"
                    and r["recognised"] in ("pointer-copy", "scalar-copy")
                    and r["step"]
                    and r["step"][1] == name
                ]
                if name in declared["scalars"] or copies:
                    label = "scalar-alias"
            else:
                family = description if description.startswith("unmodeled call") else None
            if family is not None and data["status"] == "Unknown":
                family_totals[family] += 1
                if label:
                    family_removed[family] += 1
            if label:
                leaf_class[(fkey, line, description)] = label
                classes[label] += 1
                if data["status"] == "Unknown":
                    classes_unknown[label] += 1

    def forecast(allowed):
        moved = []
        for fkey, data in functions.items():
            if data["status"] != "Unknown" or not data["leaves"]:
                continue
            if all(
                leaf_class.get((fkey, line, description)) in allowed
                for line, description in data["leaves"]
            ):
                moved.append("/".join(str(part) for part in fkey))
        return sorted(moved)

    lexical = {"condition", "step"}
    scenarios = {
        "lexical only (condition, step)": forecast(lexical),
        "lexical + gated-step": forecast(lexical | {"gated-step"}),
        "lexical + scalar-alias": forecast(lexical | {"scalar-alias"}),
        "all classes": forecast(lexical | {"gated-step", "scalar-alias"}),
    }
    by_file = collections.Counter(name.split("/")[1] for name in scenarios["all classes"])
    result = {
        "run": str(run),
        "leaves_by_class_all_statuses": dict(classes),
        "leaves_by_class_unknown_functions": dict(classes_unknown),
        "family_leaves_in_unknown_functions": dict(family_totals),
        "family_leaves_forecast_to_leave": dict(family_removed),
        "unexplained_leaves": dict(unexplained),
        "function_forecast": {name: len(moved) for name, moved in scenarios.items()},
        "functions_all_classes": scenarios["all classes"],
        "functions_lexical": scenarios["lexical only (condition, step)"],
        "functions_all_classes_by_file": dict(by_file),
    }
    out_json.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    lines = ["W43-INDEX-TRAVERSAL forecast from the traversal census", ""]
    lines.append("Leaves by forecast class (all statuses / Unknown functions):")
    for label in sorted(classes):
        lines.append(f"  {label:14} {classes[label]:6} / {classes_unknown[label]:6}")
    lines.append("")
    lines.append("Families in Unknown functions: leaves / forecast to leave:")
    for family in sorted(family_totals):
        lines.append(f"  {family_totals[family]:6} / {family_removed[family]:6}  {family}")
    lines.append("")
    lines.append(f"Unexplained leaves (no census record at the key): {dict(unexplained)}")
    lines.append("")
    lines.append("Unknown functions forecast to move (upper bounds):")
    for name, moved in scenarios.items():
        lines.append(f"  {len(moved):5}  {name}")
    lines.append("")
    lines.append("Functions under the lexical classes alone:")
    lines.extend(f"  {name}" for name in scenarios["lexical only (condition, step)"])
    out_txt.write_text("\n".join(lines) + "\n")
    print("\n".join(lines[:40]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
