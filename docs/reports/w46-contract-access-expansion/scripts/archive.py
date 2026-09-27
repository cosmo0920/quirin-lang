#!/usr/bin/env python3
"""Archive a completed integration stage with deterministic gzip and digests."""

import argparse
import gzip
import hashlib
import json
import pathlib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("output", type=pathlib.Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    manifest = {}
    for scope, logs in [("corpus", ("audit.log",)),
                        ("system", ("console.log", "stderr.log", "exit"))]:
        source = args.run / scope
        run = json.loads((source / "run.json").read_text())
        if run["exit"] != 0:
            raise SystemExit(f"cannot archive failed {scope} as completed evidence")
        destination = args.output / scope
        destination.mkdir()
        files = sorted(source.glob("*.json")) + sorted(source.glob("*.jsonl"))
        files += [source / name for name in logs]
        for path in files:
            raw = path.read_bytes()
            encoded = gzip.compress(raw, mtime=0)
            name = path.name + ".gz"
            (destination / name).write_bytes(encoded)
            manifest[f"{scope}/{path.name}"] = {
                "file": f"{scope}/{name}",
                "raw_sha256": hashlib.sha256(raw).hexdigest(),
                "gzip_sha256": hashlib.sha256(encoded).hexdigest(),
                "raw_bytes": len(raw),
            }
    (args.output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (args.output / "steps.json").write_bytes((args.run / "steps.json").read_bytes())
    print(f"archived {len(manifest)} files to {args.output}")


if __name__ == "__main__":
    main()
