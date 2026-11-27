#!/usr/bin/env python3
"""Run baseline_identity.py for RUN_DIR against BASELINE with the card's root alias.

Usage: identity.py RUN_DIR BASELINE_DIR OUT_TXT
"""
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
CANONICAL = "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da"
run, baseline, out = sys.argv[1:4]
with open(out, "wb") as log:
    code = subprocess.run(
        [sys.executable, str(ROOT / "docs/reports/w43-return-expression/scripts/baseline_identity.py"),
         run, baseline, "--root-alias", f"{ROOT}={CANONICAL}"],
        stdout=log, stderr=log,
    ).returncode
pathlib.Path(out).with_suffix(".exit").write_text(f"{code}\n")
print(pathlib.Path(out).read_text())
print("exit", code)
sys.exit(code)
