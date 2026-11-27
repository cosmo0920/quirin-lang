#!/usr/bin/env python3
"""Archive the AFTER census readings and region facts (gzip, with a manifest).

Usage: archive.py CENSUS_DIR REGION_FACTS_DIR BIN_DIR OUT_DIR

Writes each archived file as NAME.gz into OUT_DIR and `manifest.json` with
the raw SHA-256 and size of each, plus `binaries.sha256` for the binaries
that produced them. Read-only on the inputs.
"""

import gzip
import hashlib
import json
import pathlib
import sys

CENSUS = ["fixed-configured.json", "fixed-configured-predefined.json",
          "fixed-configured-control.json", "fixed.json", "previous-configured.json",
          "inputs.sha256"]
FACTS = ["fixed.json", "previous.json", "inputs.sha256"]


def main():
    census, facts, binaries, out = (pathlib.Path(p) for p in sys.argv[1:5])
    out.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for directory, names, prefix in ((census, CENSUS, "census-"), (facts, FACTS, "region-facts-")):
        for name in names:
            data = (directory / name).read_bytes()
            target = f"{prefix}{name}.gz"
            (out / target).write_bytes(gzip.compress(data, mtime=0))
            manifest[f"{prefix}{name}"] = {"bytes": len(data), "file": target,
                                           "sha256": hashlib.sha256(data).hexdigest()}
    (out / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    with open(out / "binaries.sha256", "w") as handle:
        for path in sorted(binaries.iterdir()):
            handle.write(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n")


if __name__ == "__main__":
    main()
