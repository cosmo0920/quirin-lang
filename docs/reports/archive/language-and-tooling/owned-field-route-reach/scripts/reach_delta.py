#!/usr/bin/env python3
"""Pair two `reach.py` runs — the base tree's and this tree's — into the card's
reach table, marking every row whose verdict moved.

    python3 .../reach_delta.py measure/reach-base/reach.json measure/reach-after/reach.json
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


def verdict(row: dict | None) -> str:
    if row is None:
        return "(absent)"
    return "ADMITTED" if row["exit"] == 0 else (row["code"] or "refused")


def main() -> int:
    before = {r["id"]: r for r in json.loads(Path(sys.argv[1]).read_text())}
    after = {r["id"]: r for r in json.loads(Path(sys.argv[2]).read_text())}
    width = max(len(k) for k in after)
    moved = 0
    lines = []
    for key, row in after.items():
        was, now = verdict(before.get(key)), verdict(row)
        mark = "MOVED" if was != now else ""
        if mark:
            moved += 1
        lines.append(f"{key:<{width}}  {was:<10} -> {now:<10} {mark:<6} {row['rule']}")
    lines.append("")
    lines.append(f"{len(after)} probes, {moved} moved")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
