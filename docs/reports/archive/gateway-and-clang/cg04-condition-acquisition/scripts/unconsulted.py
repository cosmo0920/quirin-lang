#!/usr/bin/env python3
"""CG04-CONDITION-ACQUISITION: the condition-refusal leaves no route consulted.

Usage: unconsulted.py RUN_DIR PARITY_JSONL

`followups.py`'s `no-consultation` class: a controlling-expression refusal in
an Unknown function with neither a `condition-desugar` nor an
`elaboration-condition` row at its (function, line). For each, the positions
of every other parity row at that (function, line) and the texts they carry,
which say why neither route was asked (the elaborator declines a descriptor
name, braces or a parenthesised callee before it reads the AST; a call-free
condition no typed operand needed keeps its refusal without a desugar row).
Prints the leaves grouped by the first token pattern that explains them.
"""

import collections
import json
import pathlib
import re
import sys

CONDITION = re.compile(
    r"^unmodeled call, aggregate or indirect access in "
    r"(branch condition|loop condition|for-loop condition|do-while condition|switch expression)$")


def main() -> int:
    run, parity = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    rows = collections.defaultdict(list)
    with parity.open() as stream:
        for text in stream:
            row = json.loads(text)
            if "function" in row and "line" in row:
                rows[(row["function"], row["line"])].append(row)
    data = json.loads((run / "resource-diagnostics.json").read_text())
    groups = collections.Counter()
    samples = collections.defaultdict(list)
    for unit in data["units"]:
        for function in unit["functions"]:
            if function["status"] != "Unknown":
                continue
            for diagnostic in function.get("diagnostics", []):
                if diagnostic.get("category") != "unsupported" or not CONDITION.match(
                        diagnostic.get("description") or ""):
                    continue
                here = rows.get((function["function"], diagnostic.get("line")), [])
                positions = {row.get("position") for row in here}
                if positions & {"condition-desugar", "elaboration-condition"}:
                    continue
                texts = next((row["texts"] for row in here if isinstance(row.get("texts"), list)),
                             [])
                joined = " ".join(texts)
                if not here:
                    group = "no parity row at the line"
                elif "{" in texts:
                    group = "braces"
                elif re.search(r"\) \(", joined):
                    group = "parenthesised callee"
                else:
                    group = "other: " + ",".join(sorted(p for p in positions if p))
                groups[group] += 1
                if len(samples[group]) < 4:
                    samples[group].append(f"{unit['file']}::{function['function']}@"
                                          f"{diagnostic.get('line')}: {joined[:140]}")
    for group, count in groups.most_common():
        print(f"{count:5}  {group}")
        for sample in samples[group]:
            print(f"         {sample}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
