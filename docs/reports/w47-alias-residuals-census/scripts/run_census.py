#!/usr/bin/env python3
"""Card W47-ALIAS-RESIDUALS-CENSUS: one census run of either scope.

Usage: run_census.py corpus OUT_DIR SOURCE_COMMIT BINARY
       run_census.py system OUT_DIR SOURCE_COMMIT BINARY

W46-ACCESS-PATH-CENSUS's runner, moved to the live commands: the corpus is
`quirin-audit pinned` (scripts/run_pinned_audit.py's command, since the
compiler example was retired by b64be69e), the system slice is
`quirin-audit current` at 046accf6 (w46-contract-gap-expansion's
run_system.py command). BINARY is the `indirect-census` build. Two streams
are switched on: the access-path unit stream (`QCORE_ACCESS_PATH_CENSUS`,
each unit's analysed text) and the typed-path parity stream
(`QCORE_TYPED_PATH_PARITY`). A census stream forces one unit at a time. The
output directory must not exist; the report artifacts must reproduce the
default build's byte for byte (the inertness check).
"""

import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys

PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")
PREPARED = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1")
ROOT = pathlib.Path(__file__).resolve().parents[4]


def main() -> int:
    scope, out, commit, binary = sys.argv[1], pathlib.Path(sys.argv[2]).resolve(), sys.argv[3], \
        pathlib.Path(sys.argv[4]).resolve()
    out.mkdir(parents=True, exist_ok=False)
    environment = {k: v for k, v in os.environ.items() if not k.startswith("QCORE_")}
    if scope == "corpus":
        command = [str(binary), "pinned",
                   str(PIN / "c-gateway-wave-pin/fluent-bit-clang-source"),
                   str(PIN / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
                   str(out / "monkey-source.json"), str(out / "resource-diagnostics.json")]
        environment["QCORE_CLANG"] = "/usr/bin/clang"
        environment["QCORE_CHUNKIO_PACKAGE"] = str(PIN / "provider-packages-wave20/qcore.chunkio")
    elif scope == "system":
        source = PREPARED / "fluent-bit-clang-source"
        revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
        if revision != "046accf6aded7906f66a1baf0202da349e521736":
            raise SystemExit("prepared source revision does not match the selected slice")
        if subprocess.check_output(["git", "-C", str(source), "status", "--porcelain"]):
            raise SystemExit("prepared source is dirty")
        command = [str(binary), "current", "--checkout", str(source),
                   "--profile-directory", str(ROOT / "contract/profiles/fluent-bit"),
                   "--compile-database", str(PREPARED / "fluent-bit-clang-production/compile_commands.json"),
                   "--selection", str(ROOT / "contract/profiles/fluent-bit/system-selection.json"),
                   "--report", str(out / "resource-diagnostics.json")]
        environment["QCORE_CLANG"] = "/usr/lib/llvm-21/bin/clang"
    else:
        raise SystemExit(f"unknown scope {scope!r}")
    environment["QCORE_ACCESS_PATH_CENSUS"] = str(out / "access-path-units")
    environment["QCORE_TYPED_PATH_PARITY"] = str(out / "parity.jsonl")
    record = {
        "scope": scope,
        "command": command,
        "source_commit": commit,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "environment": {k: v for k, v in environment.items() if k.startswith("QCORE_")},
        "start": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    }
    with open(out / "audit.log", "wb") as log:
        exit_code = subprocess.run(command, cwd=ROOT, env=environment, stdout=log, stderr=log).returncode
    record["exit"] = exit_code
    record["end"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
