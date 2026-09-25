#!/usr/bin/env python3
"""Run the pinned 185-unit corpus audit with a saved binary, recording the
contract bytes the run read.

Usage: run_binary.py OUT_DIR SOURCE_LABEL BINARY FILE...

The command shape, environment and outputs are those of
`../../w43-return-expression/scripts/run_audit.py` (`--pinned-evidence`, the
pinned Fluent Bit source and Monkey compile database, `QCORE_CLANG=/usr/bin/clang`,
the wave-20 ChunkIO package), except that the binary is named rather than
taken from `target/release`, so staged binaries can be built once, saved and
run later. As `stage_run.py` does, it records the SHA-256 of every FILE (the
contract modules and manifest a stage changes) before and after the run into
`stage-files.json` and refuses a stage whose files changed while it ran.
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


def digests(files):
    return {f: hashlib.sha256((ROOT / f).read_bytes()).hexdigest() for f in files}


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    commit = sys.argv[2]
    binary = pathlib.Path(sys.argv[3]).resolve()
    files = sys.argv[4:]
    out.mkdir(parents=True, exist_ok=False)
    before = digests(files)
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
    record = {
        "command": command,
        "source_commit": commit,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "environment": {
            "QCORE_CLANG": environment["QCORE_CLANG"],
            "QCORE_CHUNKIO_PACKAGE": environment["QCORE_CHUNKIO_PACKAGE"],
            "QCORE_SOURCE_CONTRACTS": "unset",
            "QCORE_RETURN_CENSUS": "unset",
        },
        "start": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    }
    environment.pop("QCORE_RETURN_CENSUS", None)
    with open(out / "audit.log", "wb") as log:
        code = subprocess.run(command, env=environment, stdout=log, stderr=log).returncode
    record["exit"] = code
    record["end"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    after = digests(files)
    stage = {"files": before, "unchanged_during_run": before == after, "exit": code}
    (out / "stage-files.json").write_text(json.dumps(stage, indent=1, sort_keys=True) + "\n")
    if before != after:
        print("contract files changed during the run; the stage is not evidence", file=sys.stderr)
        return 2
    return code


if __name__ == "__main__":
    sys.exit(main())
