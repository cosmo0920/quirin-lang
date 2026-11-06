#!/usr/bin/env python3
"""Run the built-in pinned Fluent Bit replay into a fresh private directory.

Usage: run_pinned_audit.py OUT_DIR SOURCE_COMMIT [--inputs-root DIR] [--jobs N] [--census]
       [--checkpoint-dir DIR [--resume]]

The input root holds c-gateway-wave-pin/ and provider-packages-wave20/.
It defaults to this checkout's build/cycle19; QCORE_PINNED_INPUTS may name a
persistent input root in another checkout. No archived runner is modified.
--checkpoint-dir seals each library group's `.qo` checkpoints there and
--resume reuses those whose identity is unchanged (card CORPUS-GROUP-CHECKPOINTS);
the directory may outlive OUT_DIR, which is always fresh.
"""

import argparse
import datetime
import hashlib
import json
import os
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out_dir", type=pathlib.Path)
    parser.add_argument("source_commit")
    parser.add_argument("--inputs-root", type=pathlib.Path)
    parser.add_argument("--jobs", type=int)
    parser.add_argument("--census", action="store_true")
    parser.add_argument("--checkpoint-dir", type=pathlib.Path)
    parser.add_argument("--resume", action="store_true")
    args = parser.parse_args()
    if args.resume and args.checkpoint_dir is None:
        parser.error("--resume requires --checkpoint-dir")
    if args.jobs is not None and args.jobs < 1:
        parser.error("--jobs needs a positive integer")
    inputs = args.inputs_root or pathlib.Path(
        os.environ.get("QCORE_PINNED_INPUTS", ROOT / "build/cycle19")
    )
    inputs = inputs.resolve()
    source = inputs / "c-gateway-wave-pin/fluent-bit-clang-source"
    monkey = inputs / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"
    package = inputs / "provider-packages-wave20/qcore.chunkio"
    for path in (source, monkey, package):
        if not path.exists():
            parser.error(f"pinned input is missing: {path}")
    binary = ROOT / "target/release/quirin-audit"
    if not binary.is_file():
        parser.error(f"build quirin-audit release binary first: {binary}")
    out = args.out_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    command = [str(binary), "pinned"]
    if args.jobs is not None:
        command.extend(["--jobs", str(args.jobs)])
    if args.checkpoint_dir is not None:
        command.extend(["--checkpoint-dir", str(args.checkpoint_dir.resolve())])
        if args.resume:
            command.append("--resume")
    command.extend([
        str(source), str(monkey), str(out / "monkey-source.json"),
        str(out / "resource-diagnostics.json"),
    ])
    environment = dict(os.environ)
    environment["QCORE_CLANG"] = "/usr/bin/clang"
    environment["QCORE_CHUNKIO_PACKAGE"] = str(package)
    environment.pop("QCORE_SOURCE_CONTRACTS", None)
    if args.census:
        environment["QCORE_RETURN_CENSUS"] = str(out / "return-census.jsonl")
    record = {
        "command": command,
        "source_commit": args.source_commit,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "environment": {
            "QCORE_CLANG": environment["QCORE_CLANG"],
            "QCORE_CHUNKIO_PACKAGE": environment["QCORE_CHUNKIO_PACKAGE"],
            "QCORE_SOURCE_CONTRACTS": "unset",
            "QCORE_RETURN_CENSUS": environment.get("QCORE_RETURN_CENSUS", "unset"),
            "QCORE_AUDIT_JOBS": str(args.jobs) if args.jobs is not None else environment.get("QCORE_AUDIT_JOBS", "unset"),
            # Card GATEWAY-RULE-API: the rule switches the run had off.
            "QCORE_GATEWAY_RULES_OFF": environment.get("QCORE_GATEWAY_RULES_OFF", "unset"),
        },
        "start": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    }
    if args.checkpoint_dir is not None:
        record["checkpoints"] = {
            "directory": str(args.checkpoint_dir.resolve()),
            "resume": args.resume,
        }
    with (out / "audit.log").open("wb") as log:
        code = subprocess.run(command, env=environment, stdout=log, stderr=log, check=False).returncode
    record["exit"] = code
    record["end"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
