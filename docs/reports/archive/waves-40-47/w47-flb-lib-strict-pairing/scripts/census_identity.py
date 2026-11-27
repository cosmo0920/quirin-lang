#!/usr/bin/env python3
"""census_identity.py RUN_DIR ARCHIVE_DIR -- compare a census run (census.sh)
with an archived census (a manifest.json of gzipped files): SHA-256 per
file, and for a differing JSON whether it is identical once the lifecycle
module's path (which names the worktree it ran in) is removed, and the keys
and tallies that differ. Read-only; prints JSON; exit 0.
"""
import gzip
import hashlib
import json
import pathlib
import sys


def load(path):
    path = pathlib.Path(path)
    data = gzip.open(path).read() if path.suffix == ".gz" else path.read_bytes()
    return data


def main():
    run, archive = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    manifest = json.loads((archive / "manifest.json").read_text())
    out = {}
    for name, entry in sorted(manifest.items()):
        mine = run / name
        if not mine.exists():
            out[name] = "missing in run"
            continue
        a = mine.read_bytes()
        b = load(archive / entry["file"])
        same = a == b
        row = {"sha256": [hashlib.sha256(b).hexdigest(), hashlib.sha256(a).hexdigest()],
               "identical": same}
        if not same and name.endswith(".json"):
            da, db = json.loads(a), json.loads(b)
            # The module path names the worktree the census ran in.
            for doc in (da, db):
                if isinstance(doc.get("lifecycle"), dict):
                    doc["lifecycle"].pop("module", None)
            row["identical_except_module_path"] = da == db
            row["tally_equal"] = da.get("tally") == db.get("tally")
            row["differing_keys"] = sorted(k for k in set(da) | set(db) if da.get(k) != db.get(k))
        out[name] = row
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
