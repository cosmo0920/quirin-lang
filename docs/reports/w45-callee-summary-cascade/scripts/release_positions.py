#!/usr/bin/env python3
"""Census of the actuals the lexical gate refused at a `Released` position.

Usage: release_positions.py CALL_ARGUMENT_CENSUS.jsonl OUT.json

Reads the `QCORE_CALL_ARGUMENT_CENSUS` stream of an `indirect-census` build
(W44-INDIRECT-CALL-ARGUMENT-ACCESS's sink). The stream records a statement once
per classification pass, and W4 derivation classifies a body several times, so
records are deduplicated on (unit, function, line, callee, argument index). For
each distinct actual at a `released` position it counts the recogniser's
answer: a recognised pointer read (what rule 1 can admit when the position is
nullable), or the refusal label that kept it out.
"""
import collections
import json
import sys


def main():
    source, out = sys.argv[1:3]
    seen = set()
    answers = collections.Counter()
    callees = collections.Counter()
    for line in open(source):
        record = json.loads(line)
        for argument in record["arguments"]:
            if not argument["access"] or argument["position"] != "released":
                continue
            key = (record["unit_seq"], record["function"], record["line"], record["callee"], argument["index"])
            if key in seen:
                continue
            seen.add(key)
            label = argument["recognised"] or argument["reason"]
            answers[label] += 1
            callees[(record["callee"], label)] += 1
    result = {
        "distinct_released_actuals": len(seen),
        "by_answer": dict(answers.most_common()),
        "top_callees": [[c, l, n] for (c, l), n in callees.most_common(40)],
    }
    json.dump(result, open(out, "w"), indent=1)
    print(json.dumps({k: result[k] for k in ("distinct_released_actuals", "by_answer")}, indent=1))


if __name__ == "__main__":
    main()
