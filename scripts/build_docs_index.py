#!/usr/bin/env python3
"""Build/check the report lookup index; titles are not interpreted as status."""
from pathlib import Path
import argparse
import re

ROOT = Path(__file__).resolve().parents[1]


def render():
    rows = ["# Report lookup index\n",
            "[Documentation hub](../README.md) · [Wave history](../development/increments/README.md)\n",
            "Generated directory/title lookup, **not a status ledger**. Search an exact\n"
            "card id here; read its report for evidence and the acceptance matrix's\n"
            "`named_followups` for registered open work. Historical measurements retain their original revision; maintained inputs are attributed in migration reports.\n",
            "Regenerate with `python3 scripts/build_docs_index.py`; validate with `--check`.\n"]
    group = None
    for report in sorted((ROOT / "docs/reports").glob("*/README.md")):
        letter = report.parent.name[0].upper()
        if letter != group:
            rows += [f"## {letter}\n", "| Card directory | Original report title |\n|---|---|\n"]
            group = letter
        match = re.search(r"^# (.+)$", report.read_text(), re.M)
        title = (match[1] if match else report.parent.name).replace("|", "&#124;")
        rows.append(f"| [{report.parent.name}]({report.parent.name}/README.md) | {title} |\n")
    return "\n".join(rows)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    target = ROOT / "docs/reports/INDEX.md"
    expected = render()
    if args.check:
        if not target.exists() or target.read_text() != expected:
            raise SystemExit("report index is stale; run python3 scripts/build_docs_index.py")
        print("Report index matches all report directories")
    else:
        target.write_text(expected)
        print(f"Wrote {target.relative_to(ROOT)}")
