#!/usr/bin/env python3
"""summary.py NAME... -- LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: per run name and
route (corpus and system under private/lpp/, the slice under the shared
prepared attempt as lpp-NAME): the custody totals and report sha256, the null
property's tally and rows digest, the sidecar's dependency counts and Safe
roll-up, the wall time, and the binary digest. With two names it also says
whether the null rows are byte-identical. Prints JSON."""
import collections
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
PRIVATE = HERE.parents[3] / "private/lpp"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")


def report(name, route):
    if route == "slice":
        return SLICE / f"lpp-{name}" / "resource-diagnostics.json"
    return PRIVATE / (name if route == "corpus" else f"{name}-system") / "resource-diagnostics.json"


def one(name, route):
    path = report(name, route)
    data = path.read_bytes()
    counts = collections.Counter(f["status"] for u in json.loads(data)["units"] for f in u["functions"])
    sidecar = json.loads(path.with_suffix(".contracts.json").read_text())
    deps = sidecar.get("contract_dependencies", {})
    suffix = {"corpus": "", "system": "-system", "slice": "-slice"}[route]
    null_rows = PRIVATE / f"{name}{suffix}-null.jsonl"
    null = json.loads((PRIVATE / f"{name}{suffix}-null.summary.json").read_text())["null"]
    seconds = (PRIVATE / f"{name}{suffix}.seconds").read_text().strip()
    return {
        "totals": dict(sorted(counts.items())),
        "sha256": hashlib.sha256(data).hexdigest(),
        "null_tally": null,
        "null_rows_sha256": hashlib.sha256(null_rows.read_bytes()).hexdigest(),
        "dependency_counts": deps.get("counts"),
        "safe_rollup": {k: v for k, v in deps.get("safe_rollup", {}).items() if not isinstance(v, list)},
        "seconds": float(seconds),
    }


def main():
    names = sys.argv[1:]
    out = {}
    for route in ("corpus", "system", "slice"):
        out[route] = {name: one(name, route) for name in names}
        if len(names) == 2:
            a, b = (out[route][n] for n in names)
            out[route]["null_rows_identical"] = a["null_rows_sha256"] == b["null_rows_sha256"]
    for name in names:
        out[f"binary:{name}"] = (PRIVATE / f"{name}.binary.sha256").read_text().split()[0]
    print(json.dumps(out, indent=1))


main()
