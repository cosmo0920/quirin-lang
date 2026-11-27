#!/usr/bin/env python3
"""MEMBER-ACCESS-PROJECTION step 1: one census run of the user's `current`
command at Fluent Bit 046accf6 (the invocation recorded in the run's
resource-diagnostics.invocation.json), with the census-feature build of the
card base, reports written under OUT_DIR (never into the checkout).

usage: run_current_census.py OUT_DIR BINARY
"""

import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys

RUN = pathlib.Path(
    "/media/Data/GitHub/quirin-lang/build/cycle19/clang/"
    "046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source")
ROOT = pathlib.Path(__file__).resolve().parents[2]


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    binary = pathlib.Path(sys.argv[2]).resolve()
    out.mkdir(parents=True, exist_ok=False)
    environment = {k: v for k, v in os.environ.items() if not k.startswith("QCORE_")}
    command = [
        str(binary), "current",
        "--checkout", str(RUN / "fluent-bit-clang-source"),
        "--profile-directory", str(ROOT / "contract/profiles/fluent-bit"),
        "--compile-database", str(RUN / "fluent-bit-clang-production/compile_commands.json"),
        "--monkey-database", str(RUN / "monkey/monkey-build/compile_commands.json"),
        "--monkey-source-report", str(out / "monkey-source.json"),
        "--report", str(out / "resource-diagnostics.json"),
        "--jobs", "1",
    ]
    environment["QCORE_CLANG"] = "/usr/lib/llvm-21/bin/clang"
    environment["QCORE_ACCESS_PATH_CENSUS"] = str(out / "access-path-units")
    environment["QCORE_TYPED_PATH_PARITY"] = str(out / "parity.jsonl")
    environment["QCORE_INDIRECT_CENSUS"] = str(out / "indirect.jsonl")
    environment["QCORE_CALL_ARGUMENT_CENSUS"] = str(out / "call-arguments.jsonl")
    environment["QCORE_CHAIN_CENSUS"] = str(out / "chains.jsonl")
    environment["QCORE_SUB_OBJECT_STORE_CENSUS"] = str(out / "sub-object-stores.jsonl")
    environment["QCORE_DESTINATION_SUFFIX_CENSUS"] = str(out / "destination-suffix.jsonl")
    record = {
        "command": command,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "environment": {k: v for k, v in environment.items() if k.startswith("QCORE_")},
        "start": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    }
    with open(out / "audit.log", "wb") as log:
        exit_code = subprocess.run(command, cwd=ROOT, env=environment,
                                   stdout=log, stderr=log).returncode
    record["exit"] = exit_code
    record["end"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
