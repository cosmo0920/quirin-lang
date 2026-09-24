#!/usr/bin/env python3
"""Run the W43 baseline identity check with this checkout's root aliased.

Usage: identity.py RUN_DIR BASELINE_ARTIFACT_DIR OUT_JSON

Calls `docs/reports/w43-return-expression/scripts/baseline_identity.py` with
`--root-alias <this checkout>=<the W43-LIBC-RESULT-ALIAS checkout>`, the
brief's alias: a path string only, which lets the `contracts` artifact's
source keys be compared across checkouts. Writes the tool's JSON to OUT_JSON,
prints a one-line summary per artifact, and exits with the tool's own exit.
"""

import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
CANONICAL = ROOT.parent / "agent-a4d52a22505f866ac"
TOOL = ROOT / "docs/reports/w43-return-expression/scripts/baseline_identity.py"


def main() -> int:
    run, baseline, out = sys.argv[1:4]
    completed = subprocess.run(
        [sys.executable, str(TOOL), run, baseline, "--root-alias", f"{ROOT}={CANONICAL}"],
        capture_output=True,
        text=True,
    )
    pathlib.Path(out).write_text(completed.stdout)
    if completed.stderr:
        sys.stderr.write(completed.stderr)
    try:
        record = json.loads(completed.stdout)
    except json.JSONDecodeError:
        print(completed.stdout)
        return completed.returncode or 1
    for label, entry in record.get("artifacts", {}).items():
        summary = {key: value for key, value in entry.items() if "sha" not in key}
        print(label, summary)
    print("exit", completed.returncode)
    return completed.returncode


if __name__ == "__main__":
    sys.exit(main())
