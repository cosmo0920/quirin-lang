#!/usr/bin/env python3
"""totals.py RUN... -- card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: per run, the
status totals, the access family (functions touched, leaves), the Unknown
functions blocked by the access refusal alone, those blocked by uncontracted
calls alone, and the leak / violation leaf counts."""
import collections
import json
import pathlib
import sys

IND = "indirect memory access is outside the resource projection"
UNC = "call to uncontracted function `"


def main() -> int:
    for run in sys.argv[1:]:
        path = pathlib.Path(run)
        path = path / "resource-diagnostics.json" if path.is_dir() else path
        data = json.loads(path.read_text())
        status, categories = collections.Counter(), collections.Counter()
        ind_functions = ind_leaves = ind_sole = unc_sole = 0
        for unit in data["units"]:
            for function in unit["functions"]:
                status[function["status"]] += 1
                leaves = function.get("diagnostics") or []
                for leaf in leaves:
                    categories[leaf.get("category")] += 1
                ind = [leaf for leaf in leaves if leaf.get("description") == IND]
                ind_leaves += len(ind)
                ind_functions += bool(ind)
                if function["status"] == "Unknown" and leaves:
                    ind_sole += len(ind) == len(leaves)
                    unc_sole += all(leaf.get("description", "").startswith(UNC) for leaf in leaves)
        print(json.dumps({
            "run": run, "status": dict(status), "access_functions": ind_functions,
            "access_leaves": ind_leaves, "access_alone": ind_sole,
            "uncontracted_alone": unc_sole, "categories": dict(categories)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
