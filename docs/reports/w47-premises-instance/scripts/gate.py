#!/usr/bin/env python3
"""W47-PREMISES-INSTANCE: the standing gate.

Usage: gate.py LOG_DIR rust|records

Every step's output goes to LOG_DIR/NAME.log and its exit to LOG_DIR/NAME.exit
(never piped). providers/ is unchanged, so the provider gate is not due.

  rust:    submodule, fmt, Rust suite, clean, clippy, both probes, Lean
           audits, clean
  records: Python suite and docs-index check (after the ledger, matrix and
           index regeneration), and the Clang recording
"""

import os
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
STEPS = {
    "rust": [
        ("01-submodule", ["git", "submodule", "update", "--init", "--recursive"]),
        ("02-fmt", ["cargo", "fmt", "--all", "--", "--check"]),
        ("03-tests", ["python3", "scripts/run_test_binaries.py"]),
        ("04-clean", ["cargo", "clean", "--profile", "dev"]),
        ("05-clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]),
        ("06-probe", ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_probe"]),
        ("07-followup", ["cargo", "run", "-p", "qcore-compiler", "--example",
                         "reevaluation_followup"]),
        ("08-lean", ["python3", "scripts/check_lean_audits.py"]),
        ("09-clean", ["cargo", "clean", "--profile", "dev"]),
    ],
    "records": [
        ("10-python", ["python3", "-m", "unittest", "discover", "-s", "scripts/tests"]),
        ("11-docs-index", ["python3", "scripts/build_docs_index.py", "--check"]),
        ("12-clang", ["/usr/lib/llvm-21/bin/clang", "--version"]),
    ],
}


def main():
    logs, phase = pathlib.Path(sys.argv[1]).resolve(), sys.argv[2]
    logs.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ, CARGO_INCREMENTAL="0",
                       PATH=f"{pathlib.Path.home() / '.elan/bin'}:{os.environ['PATH']}")
    for name, command in STEPS[phase]:
        with open(logs / f"{name}.log", "wb") as log:
            code = subprocess.run(command, cwd=REPO, env=environment, stdout=log,
                                  stderr=subprocess.STDOUT, check=False).returncode
        (logs / f"{name}.exit").write_text(f"{code}\n")
        print(name, code, flush=True)


if __name__ == "__main__":
    main()
