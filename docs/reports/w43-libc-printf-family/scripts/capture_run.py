#!/usr/bin/env python3
"""The run_audit.py command shape with QCORE_CLANG set to the transparent tee
wrapper (`../../w43-libc-result-alias/scripts/clang-tee.sh`), which saves every
preprocessing output under this worktree's `measure/capture/` and changes
nothing the audit receives.

Usage: capture_run.py OUT_DIR

A Python port of `w43-libc-result-alias/scripts/capture_run.sh`, with the same
command, environment and outputs (`audit.log`, `exit`).
"""

import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=False)
    environment = dict(os.environ)
    environment["QCORE_CLANG"] = str(ROOT / "docs/reports/w43-libc-result-alias/scripts/clang-tee.sh")
    environment["QCORE_CHUNKIO_PACKAGE"] = str(PIN / "provider-packages-wave20/qcore.chunkio")
    environment.pop("QCORE_SOURCE_CONTRACTS", None)
    command = [
        str(ROOT / "target/release/examples/cycle19_fluent_bit_libraries_audit"),
        "--pinned-evidence",
        str(PIN / "c-gateway-wave-pin/fluent-bit-clang-source"),
        str(PIN / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
        str(out / "monkey-source.json"),
        str(out / "resource-diagnostics.json"),
    ]
    with open(out / "audit.log", "wb") as log:
        code = subprocess.run(command, env=environment, stdout=log, stderr=log).returncode
    (out / "exit").write_text(f"{code}\n")
    return code


if __name__ == "__main__":
    sys.exit(main())
