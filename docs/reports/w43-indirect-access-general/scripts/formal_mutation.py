#!/usr/bin/env python3
"""Card W43-INDIRECT-ACCESS-GENERAL: size the typed scalar statements the
older formal-mutation refusal blocks before the new route is consulted.

`classify_simple` refuses `unmodeled expression may mutate a formal resource
identity` for any statement that increments or nests a write and mentions a
protected name (a formal written through `*`, or any name passed to a
non-borrowed contracted parameter). That refusal is not one the typed route is
consulted for. This counts, in a census run, the leaves of that family whose
statements the forecast mirror would otherwise admit.

Usage: formal_mutation.py CENSUS_RUN
"""

import collections
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import shape_census as census  # noqa: E402
import typed_scalar  # noqa: E402

FAMILY = "unmodeled expression may mutate a formal resource identity"


def main():
    run = pathlib.Path(sys.argv[1])
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    statements = census.load_stream(run / "statement-census.jsonl", lambda record: record["outcome"])
    assignment = census.match_units([statements], unit_keys)
    leaves = 0
    admissible = 0
    touched = set()
    for fkey, data in functions.items():
        if data["status"] != "Unknown":
            continue
        seq = assignment.get(fkey[:2])
        for line, description in data["leaves"]:
            if description != FAMILY:
                continue
            leaves += 1
            records = statements.get(seq, {}).get((fkey[2], line, description), []) if seq is not None else []
            if records and all(typed_scalar.classify(r) for r in records):
                admissible += 1
                touched.add(fkey)
    print(f"{FAMILY}: {leaves} leaves in Unknown functions; {admissible} leaves "
          f"({len(touched)} functions) whose statements the typed rule would admit")
    return 0


if __name__ == "__main__":
    sys.exit(main())
