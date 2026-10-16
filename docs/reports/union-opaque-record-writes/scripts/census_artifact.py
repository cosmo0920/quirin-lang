#!/usr/bin/env python3
"""census_artifact.py -- UNION-OPAQUE-RECORD-WRITES-BY-TYPE: the census
evidence in one file (artifacts/census.json): the closures
`member_facts_census` printed over the 94 librdkafka units at the base
(`census-base.log`, `census-base-sub.log`) and at the card's code
(`census-a1.log`), every opaque write `opaque_sites.py` listed, and
`union_places.py`'s reading of the stores and held records meeting the two
members (base and a1). Inputs under private/uorw/. A reading aid."""
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
PRIVATE = HERE.parents[3] / "private/uorw"
OUT = HERE.parent / "artifacts/census.json"


def closures(log):
    out = {}
    for line in (PRIVATE / log).read_text().splitlines():
        if ": closed over" in line or ": open:" in line:
            key, value = line.split(": ", 1)
            out[key] = value
    return out


sites = json.loads((PRIVATE / "opaque-sites.json").read_text())
flat = [dict(site, unit=unit.split("/src/")[-1])
        for unit, listed in sorted(sites.items()) if isinstance(listed, list) for site in listed]
artifact = {
    "meaning": __doc__,
    "closures_before": {**closures("census-base.log"), **closures("census-base-sub.log")},
    "closures_after": closures("census-a1.log"),
    "opaque_writes": flat,
    "union_places_before": json.loads((PRIVATE / "union-places-base.json").read_text()),
    "union_places_after": json.loads((PRIVATE / "union-places-a1.json").read_text()),
}
OUT.write_text(json.dumps(artifact, indent=1) + "\n")
print(len(flat), "opaque writes;", OUT)
