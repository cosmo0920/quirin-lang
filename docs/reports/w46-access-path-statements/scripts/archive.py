#!/usr/bin/env python3
"""Archive a completed P2 run and its complete side-by-side evidence.

Usage: archive.py RUN_DIR COMPARISON_DIR ARCHIVE_DIR. The archive must not exist.
"""

import gzip
import hashlib
import json
import pathlib
import shutil
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def compress(source, destination):
    with source.open("rb") as input_stream:
        with destination.open("wb") as output_stream:
            with gzip.GzipFile(fileobj=output_stream, mode="wb", mtime=0) as zipped:
                shutil.copyfileobj(input_stream, zipped)


def compress_sorted_jsonl(source, destination):
    with source.open(encoding="utf-8") as input_stream:
        rows = sorted(json.dumps(json.loads(line), sort_keys=True) for line in input_stream)
    with destination.open("wb") as output_stream:
        with gzip.GzipFile(fileobj=output_stream, mode="wb", mtime=0) as zipped:
            for row in rows:
                zipped.write((row + "\n").encode())


def main():
    run, comparison, archive = map(pathlib.Path, sys.argv[1:4])
    record = json.loads((run / "run.json").read_text())
    if record.get("exit") != 0 or record.get("state") != "completed":
        raise SystemExit("run has no successful completed record")
    if archive.exists():
        prior = json.loads((archive / "manifest.json").read_text())
        if prior.get("schema") != "qcore.w46-access-path-statements-archive.v1":
            raise SystemExit("existing archive has different schema")
    else:
        archive.mkdir(parents=True)
    files = {
        "console.log": "console.log.gz",
        "resource-diagnostics.json": "diagnostics.json.gz",
        "resource-diagnostics.contracts.json": "contracts.json.gz",
        "resource-diagnostics.chunk-contracts.json": "chunk-contracts.json.gz",
        "resource-diagnostics.source-effects.json": "source-effects.json.gz",
        "parity.jsonl": "parity.jsonl.gz",
        "parity.jsonl.contexts.jsonl": "frontend-contexts.jsonl.gz",
    }
    if record["scope"] == "corpus":
        files["monkey-source.json"] = "source.json.gz"
    else:
        files["resource-diagnostics.invocation.json"] = "invocation.json.gz"
    for source, target in files.items():
        if source.endswith(".jsonl"):
            compress_sorted_jsonl(run / source, archive / target)
        else:
            compress(run / source, archive / target)
    shutil.copy2(run / "run.json", archive / "run.json")
    shutil.copy2(run / "exit", archive / "exit")
    shutil.copy2(comparison / "comparison.json", archive / "comparison.json")
    for name in ("changed-functions.jsonl.gz", "parity-disagreements.jsonl.gz"):
        shutil.copy2(comparison / name, archive / name)
    attribution = comparison / "attribution"
    if attribution.exists():
        for name in ("changed-leaf-attribution.jsonl.gz", "attribution.json"):
            shutil.copy2(attribution / name, archive / name)
    manifest = {
        "schema": "qcore.w46-access-path-statements-archive.v1",
        "run": str(run.resolve()),
        "comparison": str(comparison.resolve()),
        "files": {p.name: {"sha256": digest(p), "bytes": p.stat().st_size}
                  for p in sorted(archive.iterdir()) if p.is_file() and p.name != "manifest.json"},
    }
    (archive / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
