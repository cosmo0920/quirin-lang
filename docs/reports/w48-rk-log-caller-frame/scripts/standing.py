#!/usr/bin/env python3
"""W48-RK-LOG-CALLER-FRAME: the standing, premises and Safe-row counts the
pinned arbiter's sidecars publish for the two caller-frame declarations.

usage: standing.py RUN_DIR [NAME...]   (default: rd_kafka_log0 rd_kafka_log_buf)
Prints every value found under a key equal to a NAME, anywhere in
resource-diagnostics.json outside `units` and in resource-diagnostics.contracts.json.
"""
import json
import pathlib
import sys


def walk(value, path, names, out):
    if isinstance(value, dict):
        for key, child in value.items():
            if key in names:
                out.append((path + "/" + key, child))
            else:
                walk(child, path + "/" + key, names, out)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            walk(child, f"{path}[{index}]", names, out)


def main():
    run = pathlib.Path(sys.argv[1])
    names = set(sys.argv[2:] or ["rd_kafka_log0", "rd_kafka_log_buf"])
    found = []
    report = json.loads((run / "resource-diagnostics.json").read_text())
    walk({k: v for k, v in report.items() if k != "units"}, "report", names, found)
    sidecar = run / "resource-diagnostics.contracts.json"
    if sidecar.exists():
        walk(json.loads(sidecar.read_text()), "contracts", names, found)
    for path, value in found:
        print(path, json.dumps(value, sort_keys=True)[:1500])


if __name__ == "__main__":
    main()
