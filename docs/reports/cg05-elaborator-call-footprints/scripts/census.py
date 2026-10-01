#!/usr/bin/env python3
"""CG05-ELABORATOR-CALL-FOOTPRINTS: the two follow-ups' populations in one
run's `QCORE_TYPED_PATH_PARITY` stream, de-duplicated by function, line,
tokens and position (consumer for nested-call rows), as CG05-NESTED-CALL-EFFECTS
measured them.

usage: census.py PARITY.jsonl [--json OUT] [--rows OUT]

* `elaborator`: every `position: "elaboration-*"` row, by outcome; the
  CG05-ELABORATOR-CALL-FOOTPRINTS population is the rows refused as a call
  unsequenced with another event (`... unsequenced with another event (CG05)`)
  plus, after this card, the rows refused with a footprint or a call-lowering
  reason, and the admitted rows whose statement holds a call.
* `nested`: every `position: "nested-call"` statement row refused as
  `footprints do not commute` (CG05-FOOTPRINT-ADDRESS-CONSERVATISM).
"""

import argparse
import collections
import json
import pathlib

UNORDERED = "unsequenced with another event (CG05)"
FOOTPRINT = "whose contract footprints do not commute (CG05)"


def holds_call(texts):
    return any(
        (text[:1].isalpha() or text[:1] == "_")
        and following == "("
        and text not in ("sizeof", "_Alignof", "__typeof__", "typeof", "__builtin_offsetof")
        for text, following in zip(texts, texts[1:])
    )


def elaborator_class(row):
    if row["new_admitted"]:
        return "admitted" + (" (holds a call)" if holds_call(row["texts"]) else "")
    reason = row.get("new_refusal") or ""
    if UNORDERED in reason:
        return "call unsequenced with another event"
    if FOOTPRINT in reason:
        return "call footprints do not commute"
    return reason


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    parser.add_argument("--rows", type=pathlib.Path)
    args = parser.parse_args()
    elaboration = {}
    nested = {}
    for line in args.parity.open():
        row = json.loads(line)
        position = row.get("position") or ""
        if position.startswith("elaboration-"):
            elaboration[(row["function"], row["line"], tuple(row["texts"]), position)] = row
        elif position == "nested-call" and row.get("consumer") == "statement":
            nested[(row["function"], row["line"], tuple(row["texts"]))] = row
    unordered = [r for r in elaboration.values() if UNORDERED in (r.get("new_refusal") or "")]
    calls = [
        r
        for r in elaboration.values()
        if holds_call(r["texts"]) and (r["new_admitted"] or UNORDERED in (r.get("new_refusal") or "")
                                        or FOOTPRINT in (r.get("new_refusal") or ""))
    ]
    conflict = [
        r for r in nested.values() if not r["new_admitted"] and "footprints do not commute" in (r.get("new_refusal") or "")
    ]
    admitted_nested = [r for r in nested.values() if r["new_admitted"]]
    record = {
        "elaboration_rows": len(elaboration),
        "elaboration_by_outcome": dict(collections.Counter(elaborator_class(r) for r in elaboration.values()).most_common()),
        "elaboration_by_position": dict(collections.Counter(k[3] for k in elaboration).most_common()),
        "unordered_call_rows": len(unordered),
        "unordered_call_functions": len({r["function"] for r in unordered}),
        "unordered_call_by_position": dict(collections.Counter(r["position"] for r in unordered).most_common()),
        "call_rows_admitted_or_call_refused": len(calls),
        "call_rows_admitted": len([r for r in calls if r["new_admitted"]]),
        "call_rows_admitted_functions": len({r["function"] for r in calls if r["new_admitted"]}),
        "nested_statement_rows": len(nested),
        "nested_statement_admitted": len(admitted_nested),
        "nested_footprint_conflicts": len(conflict),
        "nested_footprint_conflict_functions": len({r["function"] for r in conflict}),
    }
    text = json.dumps(record, indent=1, ensure_ascii=False)
    if args.json:
        args.json.write_text(text + "\n")
    print(text)
    if args.rows:
        with args.rows.open("w") as out:
            seen = set()
            for row in sorted(unordered + calls, key=lambda r: (r["function"], r["line"])):
                key = (row["function"], row["line"], tuple(row["texts"]), row["position"])
                if key in seen:
                    continue
                seen.add(key)
                out.write(json.dumps({k: row.get(k) for k in ("function", "line", "position", "texts", "new_admitted", "new_refusal")}, ensure_ascii=False) + "\n")
            for row in sorted(conflict, key=lambda r: (r["function"], r["line"])):
                out.write(json.dumps({k: row.get(k) for k in ("function", "line", "position", "texts", "new_admitted", "new_refusal")}, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
