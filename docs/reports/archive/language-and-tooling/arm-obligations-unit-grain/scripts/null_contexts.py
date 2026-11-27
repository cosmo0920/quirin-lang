#!/usr/bin/env python3
"""ARM-OBLIGATIONS-UNIT-GRAIN: the null view's share of BASE -> TIP.

A null row's `hypothesis_context.assumptions` is derived from its custody
row's assumptions, so a custody row that drops rule 37's premise changes the
digest of its null rows. For BASE and TIP `corpus-null.jsonl` (rows matched
by library, file, function and line order) this checks that, after the
build-identity fields every row carries (`build_identity`,
`evidence_version`) are set aside, the two files differ only in
`hypothesis_context.assumptions`, and only on null rows whose custody
artifact is a dropped row of `rows.tsv` (narrowed_carriage.py's list), and
that a dropped artifact's null rows map BASE digest -> TIP digest one to one.
Prints the census and exits 0 only when all of it holds.

Usage: null_contexts.py BASE_NULL TIP_NULL ROWS_TSV
"""
import json
import sys
from collections import defaultdict

SET_ASIDE = ("build_identity", "evidence_version")


def rows(path):
    out = defaultdict(list)
    with open(path) as handle:
        for line in handle:
            row = json.loads(line)
            out[(row["library"], row["file"], row["function"])].append(row)
    return out


def stripped(row):
    row = json.loads(json.dumps(row))
    context = row.get("hypothesis_context", {})
    for key in SET_ASIDE:
        context.pop(key, None)
    assumptions = context.pop("assumptions", None)
    return row, assumptions


def main():
    base, tip, listing = rows(sys.argv[1]), rows(sys.argv[2]), sys.argv[3]
    dropped = set()
    with open(listing) as handle:
        next(handle)
        for line in handle:
            carriage, _status, label = line.rstrip("\n").split("\t")
            if carriage == "dropped":
                unit, function = label.split("::", 1)
                library, file = unit.split("/", 1)
                dropped.add((library, file, function))
    failures = []
    changed_on_dropped = unchanged = 0
    forward, backward = defaultdict(set), defaultdict(set)
    if set(base) != set(tip):
        failures.append("the two files hold different functions")
    for key in sorted(set(base) & set(tip)):
        if len(base[key]) != len(tip[key]):
            failures.append(f"{key}: row counts differ")
            continue
        for before, after in zip(base[key], tip[key]):
            b, b_assumptions = stripped(before)
            a, a_assumptions = stripped(after)
            if b != a:
                failures.append(f"{key}: differs outside the assumptions digest")
                continue
            if b_assumptions == a_assumptions:
                unchanged += 1
                continue
            if key not in dropped:
                failures.append(f"{key}: digest changed on a row that did not drop the premise")
                continue
            changed_on_dropped += 1
            forward[b_assumptions].add(a_assumptions)
            backward[a_assumptions].add(b_assumptions)
    one_to_one = all(len(v) == 1 for v in forward.values()) and all(
        len(v) == 1 for v in backward.values())
    print(f"null rows: {sum(len(v) for v in base.values())}; digest unchanged {unchanged}; "
          f"changed on dropped custody rows {changed_on_dropped} "
          f"({len(forward)} BASE -> {len(backward)} TIP digests, one-to-one: {one_to_one}); "
          f"failures {len(failures)}")
    for failure in failures[:20]:
        print("  FAIL", failure)
    return 0 if not failures and one_to_one else 1


if __name__ == "__main__":
    sys.exit(main())
