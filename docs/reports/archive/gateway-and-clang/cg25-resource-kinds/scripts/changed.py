#!/usr/bin/env python3
"""changed.py BEFORE AFTER [OUT_JSON] -- every function whose status or
diagnostic leaves differ between two runs, with its leaf delta (card
CG25-RESOURCE-KINDS; reuses `compare.py`'s loader).
"""

import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from compare import load  # noqa: E402


def main() -> int:
    _, before = load(pathlib.Path(sys.argv[1]))
    _, after = load(pathlib.Path(sys.argv[2]))
    rows = []
    for key in sorted(set(before) | set(after), key=str):
        b_status, b_leaves = before.get(key, ("absent", collections.Counter()))
        a_status, a_leaves = after.get(key, ("absent", collections.Counter()))
        delta = collections.Counter(a_leaves)
        delta.subtract(b_leaves)
        delta = {f"{k[0]}: {k[1]}": v for k, v in delta.items() if v}
        if delta or b_status != a_status:
            rows.append({"unit": key[0], "function": key[1], "before": b_status,
                         "after": a_status, "leaf_delta": delta})
    text = json.dumps(rows, indent=1, ensure_ascii=False) + "\n"
    if len(sys.argv) > 3:
        pathlib.Path(sys.argv[3]).write_text(text)
    print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
