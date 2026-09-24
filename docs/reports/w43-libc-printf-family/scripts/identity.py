#!/usr/bin/env python3
"""Run `w43-return-expression/scripts/baseline_identity.py` with this checkout's
root aliased to the checkout the W43-LIBC-RESULT-ALIAS baseline was measured
in (`agent-a4d52a22505f866ac`, since deleted; the alias is only a path string).

Usage: identity.py RUN_DIR BASELINE_ARTIFACT_DIR OUT.json [ALIAS_ROOT]

Writes the tool's JSON to OUT.json and exits with the tool's exit status.
"""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
BASELINE_ROOT = ROOT.parent / "agent-a4d52a22505f866ac"


def main() -> int:
    run, baseline, out = sys.argv[1:4]
    alias_root = sys.argv[4] if len(sys.argv) > 4 else str(BASELINE_ROOT)
    tool = ROOT / "docs/reports/w43-return-expression/scripts/baseline_identity.py"
    result = subprocess.run(
        [sys.executable, str(tool), run, baseline, "--root-alias", f"{ROOT}={alias_root}"],
        capture_output=True,
        text=True,
    )
    pathlib.Path(out).write_text(result.stdout + result.stderr)
    print(result.stdout[-3000:], result.stderr[-2000:])
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
