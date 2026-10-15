#!/usr/bin/env python3
"""named_rows.py BEFORE AFTER FUNCTION... -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER:
for each named corpus function, its custody row's status in both runs, the
declarations its `contract_dependencies` names, and the premises those
declarations' sidecar records carry (the premises the row lists, rule 13f),
so a row another card made Safe can be shown to keep listing its premises."""
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
P = HERE.parents[3] / "private/lppr"


def load(name):
    path = P / name / "resource-diagnostics.json"
    report = json.loads(path.read_text())
    records = json.loads(path.with_suffix(".contracts.json").read_text()).get(
        "contract_dependencies", {}).get("declarations", {})
    return report, records


def rows(report, function):
    for unit in report["units"]:
        for entry in unit["functions"]:
            if entry["function"] == function:
                yield unit["file"], entry


def main():
    before, after, functions = sys.argv[1], sys.argv[2], sys.argv[3:]
    loaded = {n: load(n) for n in (before, after)}
    for function in functions:
        for name, (report, records) in loaded.items():
            for file, entry in rows(report, function):
                deps = (entry.get("contract_dependencies") or {})
                names = deps.get("declarations", [])
                premises = sorted({(p.get("name") if isinstance(p, dict) else str(p))
                                   for n in names for p in records.get(n, {}).get("premises", [])})
                print(f"{name} {file} {function} {entry['status']} declarations={len(names)} premises={premises}")


main()
