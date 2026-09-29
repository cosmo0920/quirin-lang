#!/usr/bin/env python3
"""Archive the census comparison under artifacts/census/.

Usage: archive.py BEFORE_CENSUS AFTER_CENSUS BEFORE_TYPED AFTER_TYPED CONDVAR_IDENTITY BINARIES_SHA256

Writes the attribution, identity, facts difference, site difference and
premise summary outputs, the condvar identity text, the inputs and binary
digests, the two configured AFTER readings (gzip, no timestamp) and a
manifest of SHA-256s. Read-only on the inputs.
"""

import gzip
import hashlib
import json
import pathlib
import shutil
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
OUT = HERE.parent / "artifacts/census"
READINGS = ["fixed-configured", "previous-configured", "fixed", "previous",
            "fixed-configured-control", "previous-configured-control"]
SITE_DIFF = HERE.parents[1] / "w47-loop-owner-predefined-macro-arms/scripts/site_diff.py"


def run(args):
    return subprocess.run([sys.executable, *map(str, args)], capture_output=True, text=True,
                          check=False).stdout


def main():
    before, after, typed_before, typed_after, condvar, binaries = map(pathlib.Path, sys.argv[1:7])
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "attribution.txt").write_text(run([HERE / "attribute_edges.py", before, after]))
    (OUT / "identity.txt").write_text(run([HERE / "identity.py", before, after]))
    (OUT / "facts-diff.txt").write_text(run([HERE / "facts_diff.py", typed_before, typed_after]))
    (OUT / "summary.txt").write_text(run([HERE / "summary.py", after, "--premises"]))
    (OUT / "site-diff.txt").write_text("".join(
        f"== {r}\n" + run([SITE_DIFF, before / f"{r}.json", after / f"{r}.json"])
        for r in READINGS))
    shutil.copyfile(condvar, OUT / "condvar-identity.txt")
    shutil.copyfile(after / "inputs.sha256", OUT / "inputs.sha256")
    shutil.copyfile(binaries, OUT / "binaries.sha256")
    for r in ("fixed-configured", "previous-configured"):
        data = (after / f"{r}.json").read_bytes()
        (OUT / f"{r}.json.gz").write_bytes(gzip.compress(data, 9, mtime=0))
    manifest = {}
    for path in sorted(OUT.iterdir()):
        if path.name == "manifest.json":
            continue
        data = path.read_bytes()
        raw = gzip.decompress(data) if path.suffix == ".gz" else data
        manifest[path.name.removesuffix(".gz")] = {
            "file": path.name, "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=1) + "\n")


if __name__ == "__main__":
    main()
