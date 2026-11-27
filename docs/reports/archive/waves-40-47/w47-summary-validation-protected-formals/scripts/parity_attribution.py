#!/usr/bin/env python3
"""parity_attribution.py BASELINE_PARITY_GZ RUN_PARITY CENSUS_JSON

Multiset difference of the typed-path parity rows (this card's own census
rows excluded), grouped by function, and the functions that differ outside the withdrawn
summaries and the functions reaching them through the unit call graph
(expected: none). Prints JSON; exit 1 if any row differs outside that set.
"""
import collections
import gzip
import json
import sys


def main():
    base_path, run_path, census_path = sys.argv[1:4]
    census = json.load(open(census_path))
    withdrawn = {w["function"] for w in census["withdrawn"]}
    reaching = {name for unit in census["units"] for name in unit["reaching_withdrawn"]}
    with gzip.open(base_path, "rt") as handle:
        base = collections.Counter(handle)
    with open(run_path) as handle:
        run = collections.Counter(line for line in handle if '"summary-premises' not in line)
    extra = run - base
    missing = base - run
    added = collections.Counter(json.loads(line)["function"] for line in extra.elements())
    removed = collections.Counter(json.loads(line)["function"] for line in missing.elements())
    differing = set(added) | set(removed)
    outside = sorted(differing - withdrawn - reaching)
    print(json.dumps({"rows_added": sum(extra.values()), "rows_removed": sum(missing.values()),
                      "added_by_function": added, "removed_by_function": removed,
                      "withdrawn_functions_differing": sorted(differing & withdrawn),
                      "callers_differing": sorted(differing & reaching - withdrawn),
                      "outside_attribution": outside}, indent=1, sort_keys=True))
    return 1 if outside else 0


if __name__ == "__main__":
    sys.exit(main())
