#!/usr/bin/env python3
"""CG26-VARIADICS prediction input: what else blocks each function carrying a
`__builtin_va_*` or tail leaf at the BEFORE.

Usage: predict.py BEFORE_DIAGNOSTICS OUT.json

For every function with a va-builtin leaf (the population the protocol and
the variadic parameter reader reach) and every function with a tail leaf, the
other leaves are grouped by shape (backquoted names elided), so the prediction
names what the card cannot remove.
"""

import collections
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from census import family, functions, load  # noqa: E402


def shape(description):
    return re.sub(r"`[^`]*`", "`_`", description)


def main():
    report = load(sys.argv[1])
    rows = {"va-builtin": [], "tail": []}
    other_shapes = {"va-builtin": collections.Counter(), "tail": collections.Counter()}
    for key, status, descriptions in functions(report):
        fams = [family(d) for d in descriptions]
        for name in ("va-builtin", "tail"):
            if name not in fams:
                continue
            others = [d for d, f in zip(descriptions, fams) if f != name]
            for d in others:
                other_shapes[name][shape(d)] += 1
            rows[name].append(
                {
                    "function": "/".join(key),
                    "status": status,
                    "own": sum(1 for f in fams if f == name),
                    "others": len(others),
                    "other_descriptions": others,
                }
            )
    out = {
        name: {
            "functions": len(rows[name]),
            "only_this_family": sum(1 for r in rows[name] if r["others"] == 0),
            "other_shapes": dict(other_shapes[name].most_common()),
            "rows": rows[name],
        }
        for name in rows
    }
    pathlib.Path(sys.argv[2]).write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    for name in rows:
        print(name, out[name]["functions"], out[name]["only_this_family"])
        for s, n in list(out[name]["other_shapes"].items())[:15]:
            print("   ", n, s)
    return 0


if __name__ == "__main__":
    sys.exit(main())
