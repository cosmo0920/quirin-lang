#!/usr/bin/env python3
"""Run the final arbiter binary once with `fluent_bit/storage_backlog.qcore`
removed from `cycle19-resource-contracts.json`, to show what the module
itself moves in the corpus.

Usage: without_module.py OUT_DIR

The manifest's bytes are rewritten for the run and restored afterwards by
rewriting the original bytes; the restore is asserted by SHA-256. The run uses
`target/release/examples/cycle19_fluent_bit_libraries_audit` with the
command shape of `docs/reports/w43-return-expression/scripts/run_audit.py`.
"""

import hashlib
import json
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")
MANIFEST = ROOT / "cycle19-resource-contracts.json"
LINE = '    "providers/quirin-provider-fluent-bit/contracts/fluent_bit/storage_backlog.qcore",\n'


def main():
    out = pathlib.Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=False)
    original = MANIFEST.read_bytes()
    digest = hashlib.sha256(original).hexdigest()
    text = original.decode()
    if text.count(LINE) != 1:
        raise SystemExit("module line not found exactly once")
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
    environment["QCORE_AUDIT_JOBS"] = "8"
    try:
        MANIFEST.write_text(text.replace(LINE, ""))
        with open(out / "audit.log", "wb") as log:
            code = subprocess.run(command, env=environment, stdout=log, stderr=log, cwd=ROOT).returncode
    finally:
        MANIFEST.write_bytes(original)
        os.utime(MANIFEST, None)
    restored = hashlib.sha256(MANIFEST.read_bytes()).hexdigest()
    assert restored == digest, "manifest not restored"
    record = {
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "manifest_sha256_restored": restored,
        "exit": code,
    }
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))


if __name__ == "__main__":
    main()
