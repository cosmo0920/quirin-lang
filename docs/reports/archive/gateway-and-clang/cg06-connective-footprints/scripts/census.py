#!/usr/bin/env python3
"""CG06-VALUE-CONNECTIVE-RESIDUALS (sibling-effect part): census of rule 10z's
consultations in a corpus parity stream, split by what rule 11m's composition
says about the two order refusals.

usage: census.py PARITY.jsonl [--json OUT] [--examples N]

Reads every row whose `position` starts with `value-connective`, de-duplicates
by (function, line, position, texts) keeping the last answer (a later parse of
the same body, the reported one, wins: CG06's parity_census.py), and counts
admissions and refusals by reason. A refusal of rule 10z's order premise
(`an operand unsequenced with the region carries an effect`, the 1,133 class;
`a deciding operand with an effect beside an operand it could reach`, the 4)
carries, from rule 11m on, the composition's outcome after `composed with
CG05's footprints:`; it is classed:

  (a) commutes on both arms                      -> admitted by composition
  (b) footprints do not commute
  (c) a sibling's call `F` has no contract in force (callee counted)
  (d) a sibling's effect is not a call
  (e) the region's own effect is not a call
  (f) a sibling CG05 does not read (CG05's own refusal)
  (g) the region CG05 does not read (deciding operand or arm)
  (h) the deciding operand and a sibling's call meet at an address

An admitted row whose regions needed the composition cannot be told from the
row alone; the AFTER census counts admissions, and `--before` names the
BEFORE stream so that every row refused there as an order premise and
admitted here is counted as admitted by composition.
"""

import argparse
import collections
import json
import pathlib
import re

SIBLING = "an operand unsequenced with the region carries an effect"
DECISION = "a deciding operand with an effect beside an operand it could reach"
MARK = "; composed with CG05's footprints: "


def rows_of(path: pathlib.Path):
    rows = {}
    with path.open() as stream:
        for line in stream:
            row = json.loads(line)
            position = str(row.get("position", ""))
            if not position.startswith("value-connective"):
                continue
            key = (row.get("function"), row.get("line"), position, tuple(row.get("texts", [])))
            rows[key] = row
    return rows


def classify(reason: str):
    if MARK not in reason:
        return None, None
    composed = reason.split(MARK, 1)[1]
    if composed.startswith("commutes on both arms"):
        return "a", None
    if composed.startswith("footprints do not commute"):
        return "b", composed
    match = re.match(r"a sibling's call `([^`]*)` has no contract in force", composed)
    if match:
        return "c", match.group(1)
    if composed.startswith("a sibling's effect is not a call"):
        return "d", None
    if composed.startswith("the region's own effect is not a call"):
        return "e", None
    if composed.startswith("a sibling CG05 does not read"):
        return "f", composed.split(": ", 1)[1]
    if composed.startswith("the region CG05 does not read"):
        return "g", composed.split(": ", 1)[1]
    if composed.startswith("the deciding operand and a sibling's call meet"):
        return "h", composed
    return "?", composed


def premise_of(reason: str):
    if reason.startswith(SIBLING):
        return "sibling"
    if reason.startswith(DECISION):
        return "decision"
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--before", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    parser.add_argument("--examples", type=int, default=0)
    args = parser.parse_args()
    rows = rows_of(args.parity)
    before = rows_of(args.before) if args.before else {}
    by_reason = collections.Counter()
    classes = collections.Counter()
    class_functions = collections.defaultdict(set)
    premise_rows = collections.Counter()
    detail = collections.defaultdict(collections.Counter)
    examples = collections.defaultdict(list)
    admitted_functions, refused_functions = set(), set()
    admitted_by_composition = []
    for key, row in rows.items():
        function, line, position, texts = key
        admitted = bool(row.get("new_admitted"))
        reason = "admitted" if admitted else str(row.get("new_refusal"))
        (admitted_functions if admitted else refused_functions).add(function)
        premise = None if admitted else premise_of(reason)
        if premise:
            premise_rows[premise] += 1
            label, extra = classify(reason)
            label = label or "unclassified"
            classes[(premise, label)] += 1
            class_functions[(premise, label)].add(function)
            if extra:
                detail[(premise, label)][extra[:160]] += 1
            if len(examples[(premise, label)]) < args.examples:
                examples[(premise, label)].append(f"{function}:{line}: {' '.join(texts)[:200]}")
            reason = reason.split(MARK, 1)[0]
        elif admitted and key in before:
            old = str(before[key].get("new_refusal") or "")
            old_premise = premise_of(old) if not before[key].get("new_admitted") else None
            if old_premise:
                classes[(old_premise, "a admitted")] += 1
                class_functions[(old_premise, "a admitted")].add(function)
                admitted_by_composition.append({"function": function, "line": line,
                                                "position": position,
                                                "text": " ".join(texts)[:240]})
        by_reason[reason] += 1
    record = {
        "consultations": len(rows),
        "admitted": by_reason.get("admitted", 0),
        "functions_with_an_admission": len(admitted_functions),
        "functions_with_a_refusal": len(refused_functions),
        "by_reason": dict(by_reason.most_common()),
        "order_premise_rows_still_refused": dict(premise_rows),
        "composition_classes": {f"{p} {c}": n for (p, c), n in sorted(classes.items())},
        "composition_class_functions": {f"{p} {c}": len(f) for (p, c), f in sorted(class_functions.items())},
        "detail": {f"{p} {c}": dict(counter.most_common(25)) for (p, c), counter in sorted(detail.items())},
        "admitted_by_composition": admitted_by_composition,
    }
    print(json.dumps({k: v for k, v in record.items() if k != "admitted_by_composition"},
                     indent=1, ensure_ascii=False))
    for (premise, label), lines in sorted(examples.items()):
        print(f"\n## {premise} {label}")
        for text in lines:
            print("  " + text)
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
