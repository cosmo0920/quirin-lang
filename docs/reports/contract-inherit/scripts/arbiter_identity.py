#!/usr/bin/env python3
"""Compare this card's arbiter run with the W43 baseline, with the root alias.

Usage: arbiter_identity.py RUN_DIR OUT_JSON

Runs W43-RETURN-EXPRESSION's `baseline_identity.py` with
`--root-alias <this checkout>=<the W43 checkout>` (the W43 checkout root is the
one its archived artifacts spell) and records its output and exit.
"""
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
W43_ROOT = "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/" + "agent-ae2a69ef909727ba3"


def main() -> int:
    run, out = sys.argv[1], pathlib.Path(sys.argv[2])
    command = [
        sys.executable,
        str(ROOT / "docs/reports/w43-return-expression/scripts/baseline_identity.py"),
        run,
        str(ROOT / "docs/reports/w43-return-expression/artifacts"),
        "--root-alias",
        f"{ROOT}={W43_ROOT}",
    ]
    result = subprocess.run(command, capture_output=True, text=True)
    out.write_text(json.dumps({"command": command, "exit": result.returncode,
                               "stdout": result.stdout, "stderr": result.stderr}, indent=2) + "\n")
    print(result.stdout, end="")
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
