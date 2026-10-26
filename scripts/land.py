#!/usr/bin/env python3
"""The integrator's post-gate landing sequence, and a check that the
regenerated records are fresh before the Python suite reruns (card
RECORDS-TOOLING; the sequence's single authority).

After `scripts/apply_amendments.py` writes a card's records and the landing
gate has run, the integrator:

1. writes the landing gate record `docs/reports/<card>/integration/gate.json`
   (only when the merged measurement differs from the card's own gate, as
   `docs/agents/operations.md` says; otherwise the card's `gate.json`);
2. regenerates the matrix: `python3 docs/reports/remaining-d02/build_matrix.py
   . --gate <gate.json>`;
3. regenerates the report index: `python3 scripts/build_docs_index.py`;
4. regenerates the per-rule contributions page:
   `python3 scripts/contribution_ledgers.py render` (card CONTRIBUTION-LEDGERS;
   the page names each live profile's baseline, so a landing that moves a
   baseline or adds a ledger changes it);
5. runs `python3 scripts/land.py check`, which must exit 0;
6. commits the records;
7. reruns the Python suite.

The recurring landing failure this guards is the Python suite running before
step 2: a ledger edit changes `len(named_followups)` while
`dimensions.named_followups` still holds the old count, so
`test_build_matrix_sections` fails. `check` predicts that (and a stale
`dimensions.requirements` or report index) without running the suite, and
names the step to run first; likewise a stale contributions page.

Usage:

    python3 scripts/land.py            # print the sequence
    python3 scripts/land.py check [--root DIR]

`check` exits 0 when the records are fresh, 1 naming each stale record and the
command that regenerates it.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MATRIX = Path("docs/reviews/r0_3/acceptance_matrix.json")
MATRIX_COMMAND = "python3 docs/reports/remaining-d02/build_matrix.py . --gate <gate.json>"
INDEX_COMMAND = "python3 scripts/build_docs_index.py"
CONTRIBUTIONS_COMMAND = "python3 scripts/contribution_ledgers.py render"
# The dimensions whose name is a section they count (test_build_matrix_sections).
COUNTED_SECTIONS = ("requirements", "named_followups")
SEQUENCE = (
    "write the landing gate record docs/reports/<card>/integration/gate.json "
    "(when the merged measurement differs from the card's gate)",
    MATRIX_COMMAND,
    INDEX_COMMAND,
    CONTRIBUTIONS_COMMAND,
    "python3 scripts/land.py check",
    "git commit the records",
    "python3 -m unittest discover -s scripts/tests",
)


def stale_records(root):
    """Each stale record as `(what, regenerating command)`."""
    stale = []
    try:
        matrix = json.loads((root / MATRIX).read_text(encoding="utf-8"))
        dimensions = matrix["dimensions"]
        for section in COUNTED_SECTIONS:
            if dimensions[section] != len(matrix[section]):
                stale.append((f"{MATRIX} dimensions.{section} is {dimensions[section]} against "
                              f"{len(matrix[section])} entries", MATRIX_COMMAND))
    except (OSError, ValueError, KeyError, TypeError) as error:
        stale.append((f"{MATRIX} cannot be read as the acceptance matrix ({error!r})",
                      MATRIX_COMMAND))
    index = subprocess.run([sys.executable, str(root / "scripts/build_docs_index.py"), "--check"],
                           capture_output=True, text=True)
    if index.returncode != 0:
        stale.append(("docs/reports/INDEX.md: " + (index.stderr or index.stdout).strip(),
                      INDEX_COMMAND))
    ledgers = root / "scripts/contribution_ledgers.py"
    if ledgers.is_file():
        page = subprocess.run([sys.executable, str(ledgers), "--root", str(root), "render",
                               "--check"], capture_output=True, text=True)
        if page.returncode != 0:
            stale.append(((page.stderr or page.stdout).strip(), CONTRIBUTIONS_COMMAND))
    return stale


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("command", nargs="?", choices=("sequence", "check"), default="sequence")
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args(argv)
    if args.command == "sequence":
        for number, step in enumerate(SEQUENCE, 1):
            print(f"{number}. {step}")
        return 0
    stale = stale_records(args.root.resolve())
    for what, command in stale:
        print(f"land: stale: {what}; run first: {command}", file=sys.stderr)
    if stale:
        print("land: regenerate, then rerun `python3 scripts/land.py check` before the Python "
              "suite", file=sys.stderr)
        return 1
    print("land: matrix dimensions, report index and contributions page are fresh")
    return 0


if __name__ == "__main__":
    sys.exit(main())
