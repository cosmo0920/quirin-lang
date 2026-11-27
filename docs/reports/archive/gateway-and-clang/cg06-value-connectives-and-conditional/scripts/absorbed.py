#!/usr/bin/env python3
"""CG06-VALUE-CONNECTIVES-AND-CONDITIONAL: re-measure the absorbed follow-ups on
a corpus run and its parity stream.

usage: absorbed.py RUN_DIR PARITY.jsonl [--json OUT]

* returns: every `value-connective-return` consultation (de-duplicated by
  function, line and tokens, the last answer kept), split by whether the
  returned expression holds a call, admitted or refused, with the refused
  ones' reasons, and the functions whose row still carries a return leaf on
  that line (RETURN-POSITION-CONDITIONAL, W43-RETURN-CONDITIONAL-OPERANDS);
* actuals: rule 10n's `residual-shape` rows of a call actual whose features
  include `conditional` (W47-ACCESS-PATH-POSITION-RESIDUALS's `?:` actual),
  joined on (function, line) with this card's statement consultations;
* comma returns: the consultations refused under a comma (the nanopb
  `PB_RETURN_ERROR` shape of CG02-COMMA-OPERAND-RESIDUALS).
"""

import argparse
import collections
import json
import pathlib

RETURN_LEAF = "unmodeled call, aggregate or indirect access in return expression"


def last_rows(parity, keep):
    rows = {}
    with parity.open() as stream:
        for line in stream:
            row = json.loads(line)
            if keep(row):
                key = (row.get("function"), row.get("line"), row.get("position"),
                       tuple(row.get("texts", [])))
                rows[key] = row
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    data = json.loads((args.run / "resource-diagnostics.json").read_text())
    return_leaves = set()
    status = {}
    for unit in data["units"]:
        for function in unit["functions"]:
            status[function["function"]] = function["status"]
            for diagnostic in function.get("diagnostics", []):
                if diagnostic.get("description") == RETURN_LEAF:
                    return_leaves.add((function["function"], diagnostic.get("line")))
    ours = last_rows(args.parity, lambda r: str(r.get("position", "")).startswith("value-connective"))
    residual = last_rows(
        args.parity,
        lambda r: r.get("position") == "residual-shape"
        and r.get("of_position") == "call-argument"
        and "conditional" in (r.get("features") or []),
    )
    returns = collections.Counter()
    return_reasons = collections.Counter()
    refused_return_functions = set()
    comma = collections.Counter()
    for (function, line, position, texts), row in ours.items():
        call = "(" in texts and any(
            texts[i + 1] == "(" and texts[i].isidentifier() for i in range(len(texts) - 1)
        )
        admitted = bool(row.get("new_admitted"))
        if position == "value-connective-return":
            returns[("call" if call else "call-free", admitted)] += 1
            if not admitted:
                return_reasons[row.get("new_refusal")] += 1
                if (function, line) in return_leaves:
                    refused_return_functions.add(function)
        if str(row.get("new_refusal") or "").endswith("`BinaryOperator ,`"):
            comma[(position, function)] += 1
    statement_answer = {}
    for (function, line, position, _), row in ours.items():
        if position == "value-connective-statement":
            statement_answer.setdefault((function, line), []).append(bool(row.get("new_admitted")))
    actuals = collections.Counter()
    actual_functions = collections.defaultdict(set)
    for (function, line, _, _) in residual:
        answers = statement_answer.get((function, line))
        verdict = "not consulted" if answers is None else ("admitted" if any(answers) else "refused")
        actuals[verdict] += 1
        actual_functions[verdict].add(function)
    record = {
        "returns": {f"{k[0]} {'admitted' if k[1] else 'refused'}": n for k, n in sorted(returns.items())},
        "return_refusals": dict(return_reasons.most_common()),
        "functions_keeping_a_return_leaf_on_a_refused_consultation": len(refused_return_functions),
        "unknown_among_them": sum(1 for f in refused_return_functions if status.get(f) == "Unknown"),
        "conditional_actual_rows": dict(actuals),
        "conditional_actual_functions": {k: len(v) for k, v in actual_functions.items()},
        "comma_refusals": {"rows": sum(comma.values()),
                           "functions": len({f for (_, f) in comma}),
                           "by_position": dict(collections.Counter(p for (p, _) in comma.elements()))},
    }
    print(json.dumps(record, indent=1))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1) + "\n")


if __name__ == "__main__":
    main()
