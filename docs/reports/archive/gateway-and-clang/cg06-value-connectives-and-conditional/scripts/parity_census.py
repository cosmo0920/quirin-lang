#!/usr/bin/env python3
"""CG06-VALUE-CONNECTIVES-AND-CONDITIONAL: census of the rule-10z consultations
in a corpus parity stream.

usage: parity_census.py PARITY.jsonl [--json OUT] [--examples N]

Reads every row whose `position` starts with `value-connective`, de-duplicates
by (function, line, position, texts) keeping the last answer (a later parse of
the same body, the reported one, wins), and counts admissions and refusals by
reason and position. The elaborator's own refusals naming CG06 (`position:
"elaboration-…"`, `new_refusal` ending `(CG06)`) are counted beside them, and
the residual-shape rows whose features include `conditional` (rule 10n).
"""

import argparse
import collections
import json
import pathlib


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    parser.add_argument("--examples", type=int, default=0)
    args = parser.parse_args()
    rows = {}
    elaboration = {}
    residual = {}
    with args.parity.open() as stream:
        for line in stream:
            row = json.loads(line)
            position = str(row.get("position", ""))
            key = (row.get("function"), row.get("line"), position, tuple(row.get("texts", [])))
            if position.startswith("value-connective"):
                rows[key] = row
            elif position.startswith("elaboration-") and str(row.get("new_refusal") or "").endswith("(CG06)"):
                elaboration[key] = row
            elif position == "residual-shape" and "conditional" in (row.get("features") or []):
                residual[key] = row
    by_reason = collections.Counter()
    by_position = collections.Counter()
    forms = collections.Counter()
    functions_admitted = set()
    functions_refused = set()
    examples = collections.defaultdict(list)
    for (function, line, position, texts), row in rows.items():
        admitted = bool(row.get("new_admitted"))
        reason = "admitted" if admitted else row.get("new_refusal")
        by_reason[reason] += 1
        by_position[(position, admitted)] += 1
        for form in row.get("regions") or []:
            forms[(form, admitted)] += 1
        (functions_admitted if admitted else functions_refused).add(function)
        if len(examples[reason]) < args.examples:
            examples[reason].append(f"{function}:{line}: {' '.join(texts)[:160]}")
    record = {
        "consultations": len(rows),
        "admitted": by_reason.get("admitted", 0),
        "by_reason": dict(by_reason.most_common()),
        "by_position": {f"{p} {'admitted' if a else 'refused'}": n for (p, a), n in sorted(by_position.items())},
        "forms": {f"{f} {'admitted' if a else 'refused'}": n for (f, a), n in sorted(forms.items())},
        "functions_with_an_admission": len(functions_admitted),
        "functions_with_a_refusal": len(functions_refused),
        "elaborator_cg06_refusals": len(elaboration),
        "residual_shape_conditional_rows": len(residual),
    }
    print(json.dumps(record, indent=1))
    for reason, lines in examples.items():
        print(f"\n## {reason}")
        for text in lines:
            print("  " + text)
    if args.json:
        args.json.write_text(json.dumps(record, indent=1) + "\n")


if __name__ == "__main__":
    main()
