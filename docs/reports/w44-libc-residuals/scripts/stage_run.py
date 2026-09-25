#!/usr/bin/env python3
"""Run one declarative arbiter stage and record the contract bytes it read.

Usage: stage_run.py OUT_DIR SOURCE_LABEL FILE...

The audit binary reads the contract modules and the manifest from disk at run
time, so a declarative stage is the working tree's contract files at the moment
of the run. This wrapper records the sha256 of every FILE named (the modules
and manifest the stage changed) into OUT_DIR/stage-files.json before and after
the run, refuses to publish a stage whose files changed while it ran, and then
delegates to `../../w43-return-expression/scripts/run_audit.py` unchanged.
"""

import hashlib
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]


def digests(files):
    return {f: hashlib.sha256((ROOT / f).read_bytes()).hexdigest() for f in files}


def main() -> int:
    out, label, files = sys.argv[1], sys.argv[2], sys.argv[3:]
    before = digests(files)
    code = subprocess.run(
        [sys.executable, str(ROOT / "docs/reports/w43-return-expression/scripts/run_audit.py"), out, label]
    ).returncode
    after = digests(files)
    record = {"files": before, "unchanged_during_run": before == after, "exit": code}
    (pathlib.Path(out) / "stage-files.json").write_text(json.dumps(record, indent=1, sort_keys=True) + "\n")
    if before != after:
        print("contract files changed during the run; the stage is not evidence", file=sys.stderr)
        return 2
    return code


if __name__ == "__main__":
    sys.exit(main())
