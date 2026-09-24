#!/usr/bin/env python3
"""Run the W43 baseline identity check with this checkout's root aliased.

Usage: identity.py RUN_DIR BASELINE_ARTIFACT_DIR OUT_JSON [CANONICAL_WORKTREE]

Calls `docs/reports/w43-return-expression/scripts/baseline_identity.py` with
`--root-alias <this checkout>=<CANONICAL_WORKTREE>` (default: the
W43-LIBC-RESULT-ALIAS checkout, `agent-a4d52a22505f866ac`; the combined-tree
baseline is W43-INDIRECT-ACCESS-GENERAL's, `agent-a8d24f7f717a987c1`), the
brief's alias: a path string only, which lets the `contracts` artifact's
source keys be compared across checkouts. Writes the tool's JSON to OUT_JSON,
prints a one-line summary per artifact, and exits with the tool's own exit.
"""

import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
TOOL = ROOT / "docs/reports/w43-return-expression/scripts/baseline_identity.py"


def main() -> int:
    run, baseline, out = sys.argv[1:4]
    canonical = ROOT.parent / (sys.argv[4] if len(sys.argv) > 4 else "agent-a4d52a22505f866ac")
    completed = subprocess.run(
        [sys.executable, str(TOOL), run, baseline, "--root-alias", f"{ROOT}={canonical}"],
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
