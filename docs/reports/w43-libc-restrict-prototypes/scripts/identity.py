#!/usr/bin/env python3
"""Run baseline_identity.py for one run against one artifact directory, with
this checkout's root aliased to the baseline's (deleted) worktree path, and
capture its output and exit to files.

Usage: identity.py RUN_DIR BASELINE_DIR OUT_STEM
Writes OUT_STEM.txt and OUT_STEM.exit; exits with the tool's exit.
"""
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
BASELINE_ROOT = "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a4d52a22505f866ac"
TOOL = ROOT / "docs/reports/w43-return-expression/scripts/baseline_identity.py"


def main() -> int:
    run, baseline, stem = sys.argv[1:4]
    command = [sys.executable, str(TOOL), run, baseline, "--root-alias", f"{ROOT}={BASELINE_ROOT}"]
    with open(stem + ".txt", "wb") as out:
        code = subprocess.run(command, stdout=out, stderr=subprocess.STDOUT).returncode
    pathlib.Path(stem + ".exit").write_text(f"{code}\n")
    return code


if __name__ == "__main__":
    sys.exit(main())
