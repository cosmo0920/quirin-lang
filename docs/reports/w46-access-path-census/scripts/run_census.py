#!/usr/bin/env python3
"""Card W46-ACCESS-PATH-CENSUS: one census run of the pinned 185-unit arbiter.

Usage: run_census.py OUT_DIR SOURCE_COMMIT BINARY

W44-INDIRECT-CHAIN-ACCESS's runner (itself W43-RETURN-EXPRESSION's) with the
census build given explicitly and every stream this card reads switched on:
the access-path unit stream (`QCORE_ACCESS_PATH_CENSUS`, a directory holding
each unit's analysed text), and the statement, indirect, chain, return,
traversal, condition-call and call-argument streams for the analyzer's own
premise (`may_bind`) and lexical view. A census run analyses one unit at a
time. The output directory must not exist; the artifacts must reproduce the
baseline byte for byte (baseline_identity.py), which is the inertness check.
"""

import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys

PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")

STREAMS = {
    "QCORE_ACCESS_PATH_CENSUS": "access-path-units",
    "QCORE_RETURN_CENSUS": "return-census.jsonl",
    "QCORE_INDIRECT_CENSUS": "indirect-census.jsonl",
    "QCORE_STATEMENT_CENSUS": "statement-census.jsonl",
    "QCORE_TRAVERSAL_CENSUS": "traversal-census.jsonl",
    "QCORE_CHAIN_CENSUS": "chain-census.jsonl",
    "QCORE_CONDITION_CALL_CENSUS": "condition-call-census.jsonl",
    "QCORE_CALL_ARGUMENT_CENSUS": "call-argument-census.jsonl",
}


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    commit = sys.argv[2]
    binary = pathlib.Path(sys.argv[3]).resolve()
    out.mkdir(parents=True, exist_ok=False)
    command = [
        str(binary),
        "--pinned-evidence",
        str(PIN / "c-gateway-wave-pin/fluent-bit-clang-source"),
        str(PIN / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
        str(out / "monkey-source.json"),
        str(out / "resource-diagnostics.json"),
    ]
    environment = {k: v for k, v in os.environ.items() if not k.startswith("QCORE_")}
    environment["QCORE_CLANG"] = "/usr/bin/clang"
    environment["QCORE_CHUNKIO_PACKAGE"] = str(PIN / "provider-packages-wave20/qcore.chunkio")
    for variable, name in STREAMS.items():
        environment[variable] = str(out / name)
    record = {
        "command": command,
        "source_commit": commit,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "environment": {k: v for k, v in environment.items() if k.startswith("QCORE_")},
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
