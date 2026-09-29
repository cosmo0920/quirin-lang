#!/usr/bin/env python3
"""compare.py ROUTE RUN BASE -- card CG10-COMMON-TYPED-PLACE: one route's
run against a baseline, with W47-FLB-LIB-STRICT-PAIRING's `compare` (the
SHA-256 of resource-diagnostics.json, the function and unit tallies, and every
(unit, function) whose status or leaf multiset differs), unchanged. RUN and
BASE are directories holding resource-diagnostics.json or its .gz. Prints
JSON; exits 0 only when the diagnostics are byte-identical.
"""
import importlib.util
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location(
    "strict_pairing_identity",
    HERE.parents[1] / "w47-flb-lib-strict-pairing/scripts/identity.py",
)
IDENTITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IDENTITY)


def main() -> int:
    route, run, base = sys.argv[1], pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    result = {"route": route, "run": str(run), "base": str(base)}
    result.update(IDENTITY.compare(run, base))
    json.dump(result, sys.stdout, indent=1, sort_keys=True)
    print()
    return 0 if result["identical"] else 1


if __name__ == "__main__":
    sys.exit(main())
