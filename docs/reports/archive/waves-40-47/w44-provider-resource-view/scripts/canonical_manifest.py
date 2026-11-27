#!/usr/bin/env python3
"""Write the Cycle 19 manifest's entries in whole-crate order.

Usage: canonical_manifest.py MANIFEST LOAD_PATH OUT

Every entry keeps the manifest's own spelling (a `providers/...` file path or a
`crate:module` reference). Entries are ordered as a whole-crate provider
selection lowers them: load-path crate order, then each crate's sorted
resource view. The output is formatted as serde_json's pretty printer writes
a lowered selection (two-space indent, trailing newline), so a run that
selects all eleven crates whole should record exactly this text as its
`contract_manifest`. Refuses an entry no load-path crate owns.
"""
import json
import sys
from pathlib import Path


def main():
    manifest_path, load_path_path, out = map(Path, sys.argv[1:4])
    manifest = json.loads(manifest_path.read_text())
    crates = json.loads(load_path_path.read_text())["crates"]
    order = {record["name"]: index for index, record in enumerate(crates)}
    keyed = []
    for entry in manifest["modules"]:
        if ":" in entry:
            crate, module = entry.split(":", 1)
        else:
            owners = [(record["name"], entry[len(f"providers/{record['contracts']}/"):])
                      for record in crates
                      if entry.startswith(f"providers/{record['contracts']}/")]
            if len(owners) != 1:
                raise SystemExit(f"no single owner for {entry}")
            crate, module = owners[0]
        keyed.append(((order[crate], module), entry))
    manifest["modules"] = [entry for _, entry in sorted(keyed)]
    out.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"{out}: {len(keyed)} entries")


if __name__ == "__main__":
    main()
