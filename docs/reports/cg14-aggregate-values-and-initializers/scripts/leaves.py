#!/usr/bin/env python3
"""leaves.py BEFORE AFTER [DESCRIPTION] -- card CG14-AGGREGATE-VALUES-AND-INITIALIZERS:
per-function leaf changes between two runs (same status or not), as JSON:
every function whose diagnostics changed, with its status on each side and
its per-description delta; with DESCRIPTION, only functions whose count of
that description (or category) changed.
"""

import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from compare import load  # noqa: E402


def main() -> int:
    _, before = load(pathlib.Path(sys.argv[1]))
    _, after = load(pathlib.Path(sys.argv[2]))
    wanted = sys.argv[3] if len(sys.argv) > 3 else None
    out = []
    for key in sorted(set(before) | set(after)):
        status_b, leaves_b = before.get(key, (None, {}))
        status_a, leaves_a = after.get(key, (None, {}))
        delta = {}
        for leaf in set(leaves_b) | set(leaves_a):
            change = leaves_a.get(leaf, 0) - leaves_b.get(leaf, 0)
            if change:
                delta[f"{leaf[0]}: {leaf[1]}"] = change
        if not delta and status_a == status_b:
            continue
        if wanted and not any(wanted in name for name in delta):
            continue
        out.append({"unit": key[0], "function": key[1], "before": status_b,
                    "after": status_a, "delta": delta})
    print(json.dumps(out, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
