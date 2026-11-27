#!/usr/bin/env python3
"""CG05-NESTED-CALL-EFFECTS: re-measure the absorbed follow-ups from one run's
`position: "nested-call"` parity rows (de-duplicated by function, line and
tokens).

usage: followups.py PARITY.jsonl [--json OUT]

* statement consultations by outer callee family (printf family:
  printf/fprintf/snprintf/sprintf, W44-PRINTF-NESTED-INNER-CALLEES) and by
  outcome class;
* return consultations (W43-RETURN-COMPOUND-CALL): those whose expression holds
  a call, by outcome class.
"""

import argparse
import collections
import json
import pathlib

PRINTF = {"printf", "fprintf", "snprintf", "sprintf"}


def outcome_class(row):
    if row["new_admitted"]:
        return "admitted"
    reason = row["new_refusal"] or ""
    if "has no contract in force" in reason:
        return "uncontracted inner callee"
    if reason.startswith("unsequenced") or "footprints do not commute" in reason:
        return "footprints do not commute"
    if reason.startswith("data actual"):
        return "data actual refused (place, pointer, cast, conditional, effect)"
    if reason.startswith("call `"):
        return "instantiation declined (" + reason.split(": ", 1)[-1].split("(")[0].strip() + ")"
    return reason


def holds_call(texts):
    return any(
        text[:1].isalpha() or text[:1] == "_"
        for text, following in zip(texts, texts[1:])
        if following == "(" and text not in ("sizeof", "_Alignof", "__typeof__", "typeof")
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    rows = {}
    for line in args.parity.open():
        row = json.loads(line)
        if row.get("position") != "nested-call":
            continue
        rows[(row["function"], row["line"], tuple(row["texts"]), row.get("consumer"))] = row
    statements = [r for r in rows.values() if r.get("consumer") == "statement"]
    returns = [r for r in rows.values() if r.get("consumer") == "return"]
    printf = []
    for row in statements:
        texts = row["texts"]
        outer = next(
            (t for t, n in zip(texts, texts[1:]) if n == "(" and (t[:1].isalpha() or t[:1] == "_")),
            None,
        )
        if outer in PRINTF and row["old_refusal"].startswith("nested call `"):
            printf.append(row)
    record = {
        "statement_consultations": len(statements),
        "statement_by_outcome": dict(collections.Counter(outcome_class(r) for r in statements).most_common()),
        "statement_functions": len({r["function"] for r in statements}),
        "printf_family_nested": len(printf),
        "printf_family_functions": len({r["function"] for r in printf}),
        "printf_family_by_outcome": dict(collections.Counter(outcome_class(r) for r in printf).most_common()),
        "return_consultations": len(returns),
        "return_with_calls": len([r for r in returns if holds_call(r["texts"])]),
        "return_with_calls_functions": len({r["function"] for r in returns if holds_call(r["texts"])}),
        "return_with_calls_by_outcome": dict(
            collections.Counter(outcome_class(r) for r in returns if holds_call(r["texts"])).most_common()
        ),
    }
    text = json.dumps(record, indent=1, ensure_ascii=False)
    if args.json:
        args.json.write_text(text + "\n")
    print(text)


if __name__ == "__main__":
    main()
