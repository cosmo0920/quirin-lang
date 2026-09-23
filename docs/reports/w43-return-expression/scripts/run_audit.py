#!/usr/bin/env python3
"""Run the pinned 185-unit corpus audit once into a fresh private directory.

Usage: run_audit.py OUT_DIR SOURCE_COMMIT [--census]

Reproduces the W42 command shape (`--pinned-evidence`, pinned Fluent Bit source
and Monkey compile database, `QCORE_CLANG=/usr/bin/clang`, the wave-20 ChunkIO
package) with the release binary of the current worktree. `--census` also sets
`QCORE_RETURN_CENSUS`; it has an effect only in a binary built with
`--features qcore-c-gateway/indirect-census`. The output directory must not
exist. Records `run.json` with the command, commit, binary digest, times and
exit; the audit's own stdout/stderr go to `audit.log`.
"""

import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys

PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")
ROOT = pathlib.Path(__file__).resolve().parents[4]


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    commit = sys.argv[2]
    census = "--census" in sys.argv[3:]
    out.mkdir(parents=True, exist_ok=False)
    binary = ROOT / "target/release/examples/cycle19_fluent_bit_libraries_audit"
    command = [
        str(binary),
        "--pinned-evidence",
        str(PIN / "c-gateway-wave-pin/fluent-bit-clang-source"),
        str(PIN / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
        str(out / "monkey-source.json"),
        str(out / "resource-diagnostics.json"),
    ]
    environment = dict(os.environ)
    environment["QCORE_CLANG"] = "/usr/bin/clang"
    environment["QCORE_CHUNKIO_PACKAGE"] = str(PIN / "provider-packages-wave20/qcore.chunkio")
    environment.pop("QCORE_SOURCE_CONTRACTS", None)
    if census:
        environment["QCORE_RETURN_CENSUS"] = str(out / "return-census.jsonl")
    record = {
        "command": command,
        "source_commit": commit,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "environment": {
            "QCORE_CLANG": environment["QCORE_CLANG"],
            "QCORE_CHUNKIO_PACKAGE": environment["QCORE_CHUNKIO_PACKAGE"],
            "QCORE_SOURCE_CONTRACTS": "unset",
            "QCORE_RETURN_CENSUS": environment.get("QCORE_RETURN_CENSUS", "unset"),
        },
        "start": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    }
    with open(out / "audit.log", "wb") as log:
        exit_code = subprocess.run(command, env=environment, stdout=log, stderr=log).returncode
    record["exit"] = exit_code
    record["end"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
