#!/usr/bin/env python3
"""W47-PREMISES-INSTANCE: archive the AFTER census and facts, with a manifest.

Usage: archive.py CENSUS_AFTER FACTS_AFTER COMPARE_TXT

Writes gzipped copies (mtime 0) of the four premised readings, the census
inputs, the three typed-fact files and the comparison, and a manifest of each
file's uncompressed SHA-256 and size, under artifacts/{census,facts}/.
"""

import gzip
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parents[1] / "artifacts"


def store(target, source, name):
    target.mkdir(parents=True, exist_ok=True)
    data = source.read_bytes()
    with open(target / f"{name}.gz", "wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as handle:
            handle.write(data)
    return name, {"file": f"{name}.gz", "sha256": hashlib.sha256(data).hexdigest(),
                  "bytes": len(data)}


def main():
    census, facts, compare = (pathlib.Path(p) for p in sys.argv[1:4])
    for target, items in (
            (HERE / "census", [(census / f"{n}.json", f"{n}.json") for n in (
                "fixed-premised", "fixed-configured-premised", "previous-premised",
                "previous-configured-premised")]
             + [(census / "inputs.sha256", "inputs.sha256"), (compare, "compare.txt")]),
            (HERE / "facts", [(facts / f"{n}.json", f"{n}.json") for n in (
                "library", "fixed-test", "previous-test")]
             + [(facts / "inputs.sha256", "inputs.sha256")])):
        manifest = dict(store(target, source, name) for source, name in items)
        (target / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
