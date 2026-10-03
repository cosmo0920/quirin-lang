#!/usr/bin/env python3
"""CG26-TAIL-RESIDUAL-ACTUALS: `mk_vhost_set`'s and `mk_vhost_set_property`'s
rows in the round-12 tip probe (scripts/tip-probe.sh), BEFORE against AFTER.

Usage: tip_rows.py RUN_DIR BEFORE_NAME AFTER_NAME OUT.json
"""
import json
import sys

WANTED = ("mk_vhost_set", "mk_vhost_set_property")


def main():
    run, before, after, out_path = sys.argv[1:5]
    out = {}
    for copy in ("tip", "master"):
        for name in (before, after):
            data = json.load(open(f"{run}/{copy}-{name}.json"))
            for unit in data["units"]:
                for entry in unit["functions"]:
                    if entry["function"] in WANTED:
                        out.setdefault(f"{copy}:{entry['function']}", {})[name] = {
                            "status": entry["status"],
                            "leaves": sorted(d.get("description", "")
                                             for d in entry.get("diagnostics") or []),
                        }
    for key, value in out.items():
        same = value[before]["leaves"] == value[after]["leaves"]
        print(key, value[before]["status"], "->", value[after]["status"],
              "same leaves" if same else "leaves differ")
    with open(out_path, "w") as handle:
        json.dump(out, handle, indent=1)
        handle.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
