#!/usr/bin/env python3
"""W45-FB-SYSTEM-SELECTION: record the evidence binding of one prepared attempt.

Usage: bind.py ATTEMPT_DIR RUN [RUN...] --out FILE [--binary PATH]

Read-only. For the prepared attempt (the `quirin-audit prepare --output` root)
it records the clone's HEAD and `git status --porcelain --untracked-files=no`
(read-only queries on the private clone), the SHA-256 of prepare.json,
build-provenance.json and both compile databases, the configure and build
commands from those records, the compile-database entries of the eight
selected files (unmodified, with their digests), and, for each RUN
subdirectory, the exit, the digests of every output file and the invocation
record's argv, selection, compiler and job fields. With --binary, the SHA-256
of the audit executable.
"""

import argparse
import hashlib
import json
import os
import pathlib
import subprocess

FILES = [
    "plugins/in_storage_backlog/sb.c",
    "src/flb_input_chunk.c",
    "src/flb_storage.c",
    "src/flb_task.c",
    "src/flb_engine.c",
    "src/flb_engine_dispatch.c",
    "src/flb_scheduler.c",
    "src/flb_output.c",
]


def sha(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(clone: pathlib.Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(clone), *args], check=True, capture_output=True, text=True).stdout.strip()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("attempt", type=pathlib.Path)
    parser.add_argument("runs", nargs="+")
    parser.add_argument("--out", type=pathlib.Path, required=True)
    parser.add_argument("--binary", type=pathlib.Path)
    args = parser.parse_args()
    attempt = args.attempt.resolve()
    clone = attempt / "fluent-bit-clang-source"
    build = attempt / "fluent-bit-clang-production"
    database = build / "compile_commands.json"
    entries = json.loads(database.read_text())
    prepare = json.loads((attempt / "prepare.json").read_text())
    provenance = json.loads((build / "build-provenance.json").read_text())
    selected = []
    for rel in FILES:
        target = clone / rel
        matches = [e for e in entries if pathlib.Path(os.path.realpath(e["file"])) == target]
        selected.append({
            "file": rel,
            "source_sha256": sha(target),
            "records": [
                {"entry": e, "entry_sha256": hashlib.sha256(json.dumps(e, sort_keys=True, separators=(",", ":")).encode()).hexdigest()}
                for e in matches
            ],
        })
    runs = {}
    for run in args.runs:
        directory = attempt / run
        invocation_path = directory / "resource-diagnostics.invocation.json"
        invocation = json.loads(invocation_path.read_text()) if invocation_path.exists() else {}
        runs[run] = {
            "exit": (directory / "exit").read_text().strip(),
            "files": {p.name: sha(p) for p in sorted(directory.iterdir()) if p.is_file()},
            "argv": invocation.get("argv"),
            "selection": invocation.get("selection"),
            "jobs": invocation.get("jobs"),
            "clang": invocation.get("clang"),
            "environment": invocation.get("environment"),
            "source_revision": invocation.get("source_revision"),
            "compile_database_sha256": invocation.get("compile_database_sha256"),
        }
    record = {
        "schema_version": 1,
        "card": "W45-FB-SYSTEM-SELECTION",
        "attempt": str(attempt),
        "clone": {
            "path": str(clone),
            "head": git(clone, "rev-parse", "HEAD"),
            "status_porcelain_tracked": git(clone, "status", "--porcelain", "--untracked-files=no"),
            "alternates": (clone / ".git/objects/info/alternates").exists(),
        },
        "prepare_json_sha256": sha(attempt / "prepare.json"),
        "build_provenance_sha256": sha(build / "build-provenance.json"),
        "compile_database_sha256": sha(database),
        "compile_database_entries": len(entries),
        "monkey_compile_database_sha256": sha(attempt / "monkey/monkey-build/compile_commands.json"),
        "prepare_argv": prepare.get("argv"),
        "compilers": prepare.get("compilers"),
        "cmake": prepare.get("cmake"),
        "configure_command": provenance.get("configure_command"),
        "native_build_command": provenance.get("native_build_command"),
        "build_provenance_source_revision": provenance.get("source_revision"),
        "selected_compile_records": selected,
        "binary": None if args.binary is None else {"path": str(args.binary.resolve()), "sha256": sha(args.binary)},
        "runs": runs,
        "scope": "recorded provenance of one private attempt; not compiler-execution attestation; no native execution of Fluent Bit was performed by this card",
    }
    args.out.write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps({k: record[k] for k in ("clone", "prepare_json_sha256", "build_provenance_sha256", "compile_database_sha256")}, indent=1))


if __name__ == "__main__":
    main()
