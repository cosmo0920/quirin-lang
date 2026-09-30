#!/usr/bin/env python3
"""Render the Step-0 survey of card W48-QR-USABLE-RECORDS as a Markdown table.

Usage (from the repository root):

    python3 docs/reports/w48-qr-usable-records/survey_table.py

Reads `artifacts/survey-base.json` and `artifacts/survey-tip.json` beside this
script (both written by `compiler/tests/usable_records_survey.rs` with
`QCORE_W48_RECORDS_SURVEY`) and prints one row per operation and one column
per route: `ok` for an admitted cell, the first diagnostic's code for a
refused one, and `BASE → TIP` where the two differ. A cell with no fixture is
`—`.
"""

import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
ROUTES = ["buffer", "scalar", "extern", "task", "ffi"]
OPS = [
    "construct", "read", "write", "write_owned", "write_released", "nested",
    "param", "result", "copy", "move", "loop", "loop_owned", "lend",
    "lend_owned", "view_field",
]


def load(name: str) -> dict:
    rows = json.loads((HERE / "artifacts" / name).read_text())
    return {row["file"]: row for row in rows}


def cell(row: dict) -> str:
    return "ok" if row["admitted"] else row["code"]


def main() -> int:
    base, tip = load("survey-base.json"), load("survey-tip.json")
    print("| Operation | " + " | ".join(ROUTES) + " |")
    print("|---|" + "---|" * len(ROUTES))
    for op in OPS:
        cells = []
        for route in ROUTES:
            name = f"{route}_{op}.qr"
            if name not in tip:
                cells.append("—")
                continue
            before, after = cell(base[name]), cell(tip[name])
            cells.append(before if before == after else f"{before} → **{after}**")
        print(f"| {op} | " + " | ".join(cells) + " |")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
