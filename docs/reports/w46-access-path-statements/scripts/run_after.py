#!/usr/bin/env python3
"""Run P2 corpus or system AFTER with the side-by-side statement stream.

Usage: run_after.py corpus|system OUTPUT_DIR. OUTPUT_DIR must not exist.
The binaries come from CARGO_TARGET_DIR/release and are built separately.
"""

import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")
SYSTEM = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1")
CORPUS_SHA = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
SYSTEM_SHA = "046accf6aded7906f66a1baf0202da349e521736"


def git_head(path):
    return subprocess.check_output(["git", "-C", str(path), "rev-parse", "HEAD"], text=True).strip()


def main():
    scope = sys.argv[1]
    out = pathlib.Path(sys.argv[2]).resolve()
    if scope not in ("corpus", "system"):
        raise SystemExit(f"unknown scope {scope}")
    target = pathlib.Path(os.environ["CARGO_TARGET_DIR"]).resolve()
    out.mkdir(parents=True, exist_ok=False)
    environment = dict(os.environ)
    environment["QCORE_TYPED_PATH_PARITY"] = str(out / "parity.jsonl")
    environment["QCORE_AUDIT_JOBS"] = "1"
    if scope == "corpus":
        binary = target / "release/examples/cycle19_fluent_bit_libraries_audit"
        environment["QCORE_CLANG"] = "/usr/bin/clang"
        environment["QCORE_CHUNKIO_PACKAGE"] = str(PIN / "provider-packages-wave20/qcore.chunkio")
        environment.pop("QCORE_SOURCE_CONTRACTS", None)
        command = [
            str(binary), "--pinned-evidence",
            str(PIN / "c-gateway-wave-pin/fluent-bit-clang-source"),
            str(PIN / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
            str(out / "monkey-source.json"), str(out / "resource-diagnostics.json"),
        ]
        source_head = git_head(PIN / "c-gateway-wave-pin/fluent-bit-clang-source")
        if source_head != CORPUS_SHA:
            raise SystemExit(f"corpus source mismatch: {source_head}")
    else:
        binary = target / "release/quirin-audit"
        source = SYSTEM / "fluent-bit-clang-source"
        source_head = git_head(source)
        if source_head != SYSTEM_SHA:
            raise SystemExit(f"system source mismatch: {source_head}")
        if subprocess.check_output(["git", "-C", str(source), "status", "--porcelain"]):
            raise SystemExit("system source is dirty")
        environment["QCORE_CLANG"] = "/usr/lib/llvm-21/bin/clang"
        command = [
            str(binary), "current", "--checkout", str(source),
            "--profile-directory", str(ROOT / "contract/profiles/fluent-bit"),
            "--compile-database", str(SYSTEM / "fluent-bit-clang-production/compile_commands.json"),
            "--selection", str(ROOT / "contract/profiles/fluent-bit/system-selection.json"),
            "--jobs", "1", "--report", str(out / "resource-diagnostics.json"),
        ]
    record = {
        "scope": scope,
        "command": command,
        "qcore_revision": git_head(ROOT),
        "source_revision": source_head,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "environment": {
            key: environment.get(key, "unset") for key in (
                "QCORE_CLANG", "QCORE_CHUNKIO_PACKAGE", "QCORE_SOURCE_CONTRACTS",
                "QCORE_AUDIT_JOBS", "QCORE_TYPED_PATH_PARITY",
            )
        },
        "start": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    }
    with (out / "console.log").open("wb") as log:
        result = subprocess.run(command, cwd=ROOT, env=environment, stdout=log, stderr=log)
    record["exit"] = result.returncode
    record["end"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    (out / "exit").write_text(str(result.returncode) + "\n")
    print(json.dumps(record, indent=2))
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
