#!/usr/bin/env python3
"""Card W44-BRANCH-CONDITION-CALLS: size the refused condition classes that remain.

Usage: residuals.py AFTER_DIAGNOSTICS CENSUS_FINAL_GZ BEFORE_DIAGNOSTICS OUT_JSON OUT_TXT

Every condition-family leaf (branch, loop, for-loop, do-while, switch) of an
AFTER Unknown function is classed by the census of record's answers at its
(function, line, position): the refusal code of the rule, with a refused call
split by the classifier's own reason (indirect argument / uncontracted callee /
other). A leaf whose records disagree is classed by the first non-`no-call`
refusal in the order below; `no-call` only when every record is call-free.

The census answers are those of the census binary, whose rule was not
consulted; the AFTER lowering differs from it only by admitting what the census
answered `admitted`, so the join on (function, line) is the rule's own answer.
The unit join uses the BEFORE document (unit identity is unchanged).

For each class: leaves, Unknown functions, and sole-blocked functions (the
function's every leaf is a condition-family leaf of that one class).
"""

import collections
import gzip
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.append(str(HERE.parents[1] / "w43-indirect-access-general" / "scripts"))

import shape_census as census  # noqa: E402

FAMILY = re.compile(
    r"unmodeled call, aggregate or indirect access in "
    r"(branch condition|loop condition|for-loop condition|do-while condition|switch expression)$"
)
POSITION = {
    "branch condition": "if",
    "loop condition": "while",
    "for-loop condition": "for",
    "do-while condition": "do-while",
    "switch expression": "switch",
}
ORDER = [
    "call-indirect-argument",
    "call-uncontracted",
    "call-other",
    "call-free-leaf-refused",
    "assignment-shape",
    "assignment-destination",
    "recursive-local-callee",
    "custody-result",
    "unsequenced-effects",
    "residual-refused",
    "type-operator",
    "braces",
    "indirect-callee",
    "increment-or-decrement",
    "nested-connective",
    "elvis",
    "switch-shape",
    "descriptor-read",
    "unparsed",
    "no-call",
    "admitted",
]


def code(record):
    outcome = record["outcome"]
    if outcome != "call-refused":
        return outcome
    reason = record.get("reason") or ""
    if reason.startswith("indirect memory access"):
        return "call-indirect-argument"
    if reason.startswith("call to uncontracted function"):
        return "call-uncontracted"
    return "call-other"


def main() -> int:
    after_path, census_path, before_path, out_json, out_txt = sys.argv[1:6]
    _, functions, _ = census.load_document(after_path)
    _, _, unit_keys = census.load_document(before_path)
    stream = collections.defaultdict(lambda: collections.defaultdict(list))
    for line in gzip.open(census_path, "rt"):
        record = json.loads(line)
        stream[record["unit_seq"]][(record["function"], record["line"], record["position"])].append(record)
    assignment = census.match_units(
        [{seq: {(f, l, None): [] for (f, l, _) in keys} for seq, keys in stream.items()}],
        unit_keys,
    )
    leaves = collections.Counter()
    funcs = collections.defaultdict(set)
    sole = collections.Counter()
    per_family = collections.Counter()
    examples = collections.defaultdict(list)
    unexplained = 0
    fkey_classes = {}
    only_family_functions = set()
    for fkey, data in functions.items():
        if data["status"] != "Unknown":
            continue
        records = stream.get(assignment.get(fkey[:2]), {})
        classes = set()
        only_family = True
        for line, description in data["leaves"]:
            match = FAMILY.match(description)
            if not match:
                only_family = False
                continue
            found = records.get((fkey[2], line, POSITION[match.group(1)]), [])
            codes = {code(record) for record in found}
            if not codes:
                label = "unexplained"
                unexplained += 1
            else:
                label = next(c for c in ORDER + sorted(codes) if c in codes)
            leaves[label] += 1
            per_family[(match.group(1), label)] += 1
            funcs[label].add(fkey)
            classes.add(label)
            if len(examples[label]) < 3:
                sample = next((r for r in found if code(r) == label), None)
                if sample:
                    examples[label].append(f"{fkey[2]}: {' '.join(sample['texts'])[:160]}")
        if only_family and len(classes) == 1:
            sole[next(iter(classes))] += 1
        fkey_classes[fkey] = classes
        if only_family:
            only_family_functions.add(fkey)
    rows = [
        {"class": label, "leaves": leaves[label], "functions": len(funcs[label]), "sole": sole[label],
         "examples": examples[label]}
        for label in sorted(leaves, key=lambda l: -leaves[l])
    ]
    # The registered follow-ups, each a union of classes (functions counted
    # once; sole = the function's every leaf is a condition leaf of the group).
    groups = {
        "W44-CONDITION-CALL-INDIRECT-ARGUMENT": {"call-indirect-argument"},
        "W44-CONDITION-CALL-UNCONTRACTED": {"call-uncontracted"},
        "W44-CONDITION-CALL-FREE-OPERAND": {"call-free-leaf-refused"},
        "W44-CONDITION-CALL-TYPE-OPERATOR": {"type-operator"},
        "W44-CONDITION-CALL-ASSIGNMENT": {"assignment-destination", "assignment-shape"},
        "W44-CONDITION-CALL-RECURSIVE-LOCAL": {"recursive-local-callee"},
        "W44-CONDITION-CALL-ERRNO-RESIDUAL": {"residual-refused"},
        "W44-CONDITION-CALL-REFUSED-SHAPES": {
            "indirect-callee", "braces", "call-other", "increment-or-decrement",
            "nested-connective", "custody-result", "unsequenced-effects", "elvis",
            "switch-shape", "descriptor-read", "unparsed",
        },
    }
    group_rows = []
    for name, members in groups.items():
        member_functions = set().union(*(funcs[m] for m in members))
        group_sole = sum(
            1 for fkey in member_functions if fkey_classes.get(fkey) and fkey_classes[fkey] <= members
            and fkey in only_family_functions
        )
        group_rows.append({
            "followup": name,
            "classes": sorted(members),
            "leaves": sum(leaves[m] for m in members),
            "functions": len(member_functions),
            "sole": group_sole,
        })
    result = {
        "groups": group_rows,
        "rows": rows,
        "per_family": [
            {"family": f, "class": c, "leaves": n} for (f, c), n in sorted(per_family.items(), key=lambda kv: -kv[1])
        ],
        "unexplained": unexplained,
    }
    pathlib.Path(out_json).write_text(json.dumps(result, indent=1) + "\n")
    with open(out_txt, "w") as text:
        text.write("W44-BRANCH-CONDITION-CALLS residual condition classes (AFTER Unknown functions)\n\n")
        text.write(f"{'class':28} {'leaves':>7} {'functions':>9} {'sole':>5}\n")
        for row in rows:
            text.write(f"{row['class']:28} {row['leaves']:7} {row['functions']:9} {row['sole']:5}\n")
        text.write(f"\nunexplained leaves: {unexplained}\n")
        text.write("\nregistered follow-ups: leaves functions sole\n")
        for row in group_rows:
            text.write(f"  {row['followup']:38} {row['leaves']:6} {row['functions']:6} {row['sole']:4}\n")
    print(pathlib.Path(out_txt).read_text())
    return 0


if __name__ == "__main__":
    sys.exit(main())
