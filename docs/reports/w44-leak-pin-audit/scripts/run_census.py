#!/usr/bin/env python3
"""Run the pinned 185-unit corpus audit with the leak-witness census on.

Usage: run_census.py BINARY OUT_DIR SOURCE_LABEL

The command shape is `../../w43-return-expression/scripts/run_audit.py`'s
(`--pinned-evidence`, pinned Fluent Bit source and Monkey compile database,
`QCORE_CLANG=/usr/bin/clang`, the wave-20 ChunkIO package), with two
differences: the binary is named explicitly, so a census binary built with
`--features qcore-c-gateway/indirect-census` can be kept beside the arbiter's
plain release binary, and `QCORE_LEAK_WITNESS` names `OUT_DIR/leak-witness.jsonl`.
The census is a measurement: the run's diagnostics must stay byte-identical to
the plain binary's, which `baseline_identity.py` checks. The output directory
must not exist. Records `run.json`; the audit's own output goes to `audit.log`.
"""

import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys

PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")


def main() -> int:
    binary = pathlib.Path(sys.argv[1]).resolve()
    out = pathlib.Path(sys.argv[2]).resolve()
    label = sys.argv[3]
    out.mkdir(parents=True, exist_ok=False)
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
    environment["QCORE_LEAK_WITNESS"] = str(out / "leak-witness.jsonl")
    record = {
        "command": command,
        "source": label,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "environment": {
            "QCORE_CLANG": environment["QCORE_CLANG"],
            "QCORE_CHUNKIO_PACKAGE": environment["QCORE_CHUNKIO_PACKAGE"],
            "QCORE_SOURCE_CONTRACTS": "unset",
            "QCORE_LEAK_WITNESS": environment["QCORE_LEAK_WITNESS"],
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
