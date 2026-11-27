#!/usr/bin/env python3
"""Card REFUSED-STORE-OF-CALL-RESULT-ESCAPE: summarise the route-level census.

Usage: route_census_record.py RUN_DIR USER_REPORT OUT.json

RUN_DIR is the one-unit `quirin-audit current` census run (its
`resource-diagnostics.json` and `leak-witness.jsonl`); USER_REPORT is the
user's 046accf6 `resource-diagnostics.json`. Compares the cmt_map.c unit's
function rows between the two and tallies the witness stream by function.
"""

import collections
import json
import os
import sys

UNIT = "lib/cmetrics/src/cmt_map.c"


def unit(report):
    return next(u for u in report["units"] if u["file"] == UNIT)


def rows(u):
    return {
        f["function"]: {
            "status": f["status"],
            "diagnostics": [[d["category"], d["line"], d["description"]] for d in f["diagnostics"]],
        }
        for f in u["functions"]
    }


def main() -> int:
    run = sys.argv[1]
    census = json.load(open(os.path.join(run, "resource-diagnostics.json"), encoding="utf-8"))
    user = json.load(open(sys.argv[2], encoding="utf-8"))
    witnesses = [
        json.loads(line)
        for line in open(os.path.join(run, "leak-witness.jsonl"), encoding="utf-8")
        if line.strip()
    ]
    tally = collections.Counter(
        (record.get("record", "witness"), record["function"]) for record in witnesses
    )
    census_rows, user_rows = rows(unit(census)), rows(unit(user))
    leak_witnesses = sorted(
        {
            (record["function"], record["line"], record["acquisition_line"], record["settled"])
            for record in witnesses
            if "record" not in record
        }
    )
    document = {
        "schema": "refused-store-route-census/1",
        "card": "REFUSED-STORE-OF-CALL-RESULT-ESCAPE",
        "run": "quirin-audit current, one-unit selection (cmt_map.c); the route also analyses the Monkey units",
        "units": len(census["units"]),
        "unit_status": {u["file"]: u["status"] for u in census["units"] if u["status"] != "Unknown"},
        "cmt_map_rows_equal_user_run": census_rows == user_rows,
        # The user's run used a 13337d0af-era binary; the census binary is the
        # card's base, so rows its later cards changed differ.
        "cmt_map_rows_differing_from_user_run": {
            name: {"census": census_rows.get(name, {}).get("status"), "user": user_rows.get(name, {}).get("status")}
            for name in sorted(set(census_rows) | set(user_rows))
            if census_rows.get(name) != user_rows.get(name)
        },
        "cmt_map_source_identity": unit(census)["functions"][0]["hypothesis_context"]["source_identity"],
        "cmt_map_metric_get": census_rows["cmt_map_metric_get"],
        "metric_prepare_storage": census_rows["metric_prepare_storage"],
        "witness_records": len(witnesses),
        "witness_records_by_kind_and_function": sorted(
            [kind, function, count] for (kind, function), count in tally.items()
        ),
        "leak_witnesses": [list(entry) for entry in leak_witnesses],
        "cmt_map_witness_records": sum(
            1 for record in witnesses if record["function"] in census_rows
        ),
    }
    with open(sys.argv[3], "w", encoding="utf-8") as out:
        json.dump(document, out, indent=1)
        out.write("\n")
    print(json.dumps({k: v for k, v in document.items() if k not in ("cmt_map_metric_get", "metric_prepare_storage")}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
