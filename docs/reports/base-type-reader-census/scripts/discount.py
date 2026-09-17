#!/usr/bin/env python3
"""Card BASE-TYPE-READER-CENSUS: the second discount.

`base_causes.py` predicts what a reader change committing a cause's base types
would decide, ASSUMING the member then classifies. For the dominant cause the
census can test that assumption rather than carry it, because the cause payload
names the record tag: the tag's body was either read or refused whole
(`RECORD-BODY-TOTAL-OR-REJECT`).

This tool applies that second discount to the cleared set, and reports
constraint 5's premise over it, so the recommendation rests on the narrower
number rather than the wider one.

Usage: python3 discount.py <run-dir> <out-dir>
"""
import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from base_causes import (  # noqa: E402
    INDIRECT, blocking_chain, cause_key, cause_payload, load_census,
    load_document, match_units, shape,
)

CAUSE = "DeclaredOpaque/TypedefDefinesAggregate"


def main():
    run = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    blockers, unit_leaves, leaf_functions = load_document(run / "resource-diagnostics.json")
    census, _ = load_census(run / "indirect-leaves.jsonl")
    assignment = match_units(census, unit_leaves)
    bodies = {}
    for line in open(run / "record-bodies.jsonl"):
        note = json.loads(line)
        bodies.setdefault(note["tag"], note)

    statements = {}
    for ukey, keys in unit_leaves.items():
        seq = assignment.get(ukey)
        records = census.get(seq, {}) if seq is not None else {}
        for key in keys:
            statements[ukey + key] = records.get(key, [])

    own = collections.defaultdict(list)
    touched = set()
    for key, records in statements.items():
        f = leaf_functions[key]
        own[f].extend(records)
        for record in records:
            if any(cause_key(c) == CAUSE for c in record["chains"]):
                touched.add(f)

    def exchanges(record):
        return bool(record["contains_call"]) and not record.get("call_contracted")

    report = []

    def say(text=""):
        print(text)
        report.append(text)

    def resolvable(chain):
        """Would committing this base's type actually yield a usable tag?

        Three answers, and only the first is a clearance:
          - the head is `struct TAG` and TAG's body was READ: both readers agree;
          - the head is `struct TAG` but TAG's body is refused whole: the
            refusal moves one step, to RECORD-BODY-TOTAL-OR-REJECT;
          - the head names no struct tag (`union`, anonymous `struct`, `enum`):
            there is nothing for a base-type reader to commit to.
        """
        head = (cause_payload(chain) or "").split()
        if len(head) != 2 or head[0] != "struct":
            return "no-tag"
        note = bodies.get(head[1])
        if note is None:
            return "body-never-met"
        return "body-read" if note["refused_on"] is None else "body-refused"

    cleared = []
    for f in touched:
        if set(blockers[f]) != {INDIRECT}:
            continue
        records = own[f]
        if not records:
            continue
        if any(not all(cause_key(c) in ("Committed", CAUSE) for c in r["chains"])
               for r in records):
            continue
        if any(exchanges(r) for r in records):
            continue
        cleared.append(f)

    say("=" * 78)
    say("THE SECOND DISCOUNT: does committing the base type resolve the member?")
    say("=" * 78)
    say("cause: %s" % CAUSE)
    say("functions the first discount predicts leaf-free: %d" % len(cleared))
    say()
    say("A function survives the second discount only if EVERY one of its")
    say("chains blocked by this cause names a `struct TAG` whose record body")
    say("the reader actually READ. Where the body is refused whole the refusal")
    say("moves one step rather than clearing, and where the head names no")
    say("struct tag there is nothing to commit to at all.")
    say()
    verdicts = collections.Counter()
    survivors = []
    for f in sorted(cleared):
        answers = set()
        for r in own[f]:
            for c in r["chains"]:
                if cause_key(c) == CAUSE:
                    answers.add(resolvable(c))
        if answers == {"body-read"}:
            verdicts["SURVIVES  -- every blocked base resolves to a read body"] += 1
            survivors.append(f)
        elif "no-tag" in answers:
            verdicts["falls -- a head naming no struct tag"] += 1
        else:
            verdicts["falls -- a record body refused whole"] += 1
    for name, count in verdicts.most_common():
        say("   %4d  %s" % (count, name))
    say()
    say("SURVIVING functions: %d" % len(survivors))
    for f in survivors:
        say("    %s" % "/".join(str(x) for x in f))

    say()
    say("--- constraint 5's premise over the surviving set ---")
    say("An arbiter admitting an access through one of these bases needs the")
    say("premise `a name this function never binds to a resource identity`.")
    prem = collections.Counter()
    for f in survivors:
        answers = set()
        for r in own[f]:
            for c in r["chains"]:
                if cause_key(c) == CAUSE:
                    answers.add(bool(c["base_may_bind"]))
        prem["every blocked base is NEVER-BOUND (premise holds)" if answers == {False}
              else "at least one blocked base MAY BIND (premise unavailable)"] += 1
    for name, count in prem.most_common():
        say("   %4d  %s" % (count, name))

    (out / "second-discount.txt").write_text("\n".join(report) + "\n")
    print()
    print("written: %s" % (out / "second-discount.txt"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
