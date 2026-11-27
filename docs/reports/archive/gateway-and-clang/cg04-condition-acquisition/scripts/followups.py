#!/usr/bin/env python3
"""CG04-CONDITION-ACQUISITION: re-measure the absorbed follow-ups on a corpus run.

Usage: followups.py RUN_DIR PARITY_JSONL [--json OUT]

Joins the run's condition-refusal leaves (`unsupported` diagnostics whose
description is a controlling-expression refusal, in Unknown functions) with
the parity stream's consultation rows at the same (function, line): this
card's `condition-desugar` rows (the desugaring's answer) and CG02's
`elaboration-condition` rows (the call-free elaborator's). A leaf with no row
was not consulted by either (an older route's refusal of a call-free
condition that no typed or elaborated operand needed). Rows are keyed by
(function, line) without the unit, so two same-named functions of two units
at one internal line would share rows (measured: stated in the output).

Prints, per class: leaves, functions, and functions whose every diagnostic is
a condition leaf of that class ("sole"), then the classes each absorbed
follow-up names.
"""

import argparse
import collections
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import census  # noqa: E402

CONDITION = re.compile(
    r"^unmodeled call, aggregate or indirect access in "
    r"(branch condition|loop condition|for-loop condition|do-while condition|switch expression)$")

FOLLOWUPS = {
    "W44-CONDITION-CALL-ASSIGNMENT": ["AssignmentDestination", "AssignmentShape"],
    "W44-CONDITION-CALL-FREE-OPERAND": ["CallFreeLeafRefused"],
    "W44-CONDITION-CALL-TYPE-OPERATOR": ["TypeOperator", "LeafShape(CallInTypeOperand)"],
    "W44-CONDITION-CALL-REFUSED-SHAPES": [
        "IndirectCallee", "Braces", "IncrementOrDecrement", "LeafShape(IncrementBesideCall)",
        "NestedConnective", "LeafShape(ConnectiveBesideCall)", "CustodyResult", "Elvis",
        "Unparsed", "UnsequencedEffects", "LeafShape(AcquisitionOnCycle)",
        "LeafShape(CarrierMismatch)", "CallRefused: other"],
}

WRITE = re.compile(r"(\+ \+|- -|(?<![=!<>+\-*/%&|^]) = (?!=))")


def load_rows(path: pathlib.Path):
    desugar = census.rows(path)
    by_line = collections.defaultdict(list)
    for (_, function, line, texts), row in desugar.items():
        by_line[(function, line)].append(("desugar", census.classify(row), " ".join(texts)))
    elaborated = {}
    with path.open() as stream:
        for text in stream:
            if '"elaboration-condition"' not in text:
                continue
            row = json.loads(text)
            if row.get("position") != "elaboration-condition":
                continue
            elaborated[(row.get("source_sha256"), row["function"], row["line"],
                        tuple(row["texts"]))] = row
    for (_, function, line, texts), row in elaborated.items():
        label = "elaborated" if row.get("new_admitted") else "elaboration-refused"
        by_line[(function, line)].append(("elaboration", label, " ".join(texts)))
    return by_line


def leaf_class(entries):
    desugar = [label for kind, label, _ in entries if kind == "desugar"]
    if desugar:
        label = sorted(desugar)[0]
        if label.startswith("CallRefused") and "indirect memory access" not in label \
                and "uncontracted" not in label:
            return "CallRefused: other"
        return label
    other = [label for kind, label, _ in entries]
    return other[0] if other else "no-consultation"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    by_line = load_rows(args.parity)
    data = json.loads((args.run / "resource-diagnostics.json").read_text())
    leaves = collections.Counter()
    functions = collections.defaultdict(set)
    sole = collections.Counter()
    writes = collections.Counter()
    write_functions = set()
    for unit in data["units"]:
        for function in unit["functions"]:
            if function["status"] != "Unknown":
                continue
            key = (unit["file"], function["function"])
            classes = []
            other = False
            for diagnostic in function.get("diagnostics", []):
                description = diagnostic.get("description") or ""
                if diagnostic.get("category") != "unsupported" or not CONDITION.match(description):
                    other = True
                    continue
                entries = by_line.get((function["function"], diagnostic.get("line")), [])
                label = leaf_class(entries)
                leaves[label] += 1
                functions[label].add(key)
                classes.append(label)
                # W43-INDEX-TRAVERSAL-CONDITION-WRITE's class: a call-free
                # controlling expression (answered by the elaborator only)
                # that writes.
                if entries and all(kind == "elaboration" for kind, _, _ in entries) and any(
                        WRITE.search(texts) for _, _, texts in entries):
                    writes["refused-with-write"] += 1
                    write_functions.add(key)
            if classes and not other and len(set(classes)) == 1:
                sole[classes[0]] += 1
    record = {
        "classes": {label: {"leaves": leaves[label], "functions": len(functions[label]),
                            "sole": sole[label]}
                    for label in sorted(leaves, key=lambda label: -leaves[label])},
        "followups": {},
        "condition_writes": {"leaves": writes["refused-with-write"],
                             "functions": len(write_functions)},
    }
    for followup, labels in FOLLOWUPS.items():
        keys = set().union(*(functions[label] for label in labels))
        record["followups"][followup] = {
            "leaves": sum(leaves[label] for label in labels),
            "functions": len(keys),
            "sole": sum(sole[label] for label in labels),
            "classes": {label: leaves[label] for label in labels if leaves[label]},
        }
    for label, entry in record["classes"].items():
        print(f"{entry['leaves']:6} {entry['functions']:5} {entry['sole']:4}  {label}")
    print()
    for followup, entry in record["followups"].items():
        print(f"{followup}: {entry['leaves']} leaves / {entry['functions']} functions, "
              f"{entry['sole']} sole  {entry['classes']}")
    print(f"call-free condition leaves that write: {record['condition_writes']}")
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
