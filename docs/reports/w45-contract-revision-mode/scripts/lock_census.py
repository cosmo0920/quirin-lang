#!/usr/bin/env python3
"""Census of the committed contract locks (format version 2): per crate, the
revisions its identities were reviewed at, the pinned and flexible counts, and
every pinned function's reviewed identities (revisions, site paths and kinds).
Writes JSON to stdout.

usage: lock_census.py            (from the repository root)
"""
import collections
import json
import pathlib
import sys


def revision(source):
    return source.get("revision") or source["kind"]


def main() -> int:
    rows = []
    for lock_path in sorted(pathlib.Path("providers").glob("*/contracts.lock")):
        lock = json.loads(lock_path.read_text())
        pinned = []
        shapes = collections.Counter()
        for entry in lock["pinned"]:
            identities = [{
                "reviewed_at": [revision(s) for s in identity["reviewed_at"]],
                "sites": [f'{site["kind"]}:{site["path"]}' for site in identity["sites"]],
            } for identity in entry["identities"]]
            shapes[" | ".join(" + ".join(i["reviewed_at"]) for i in identities)] += 1
            pinned.append({"function": entry["function"], "module": entry["module"],
                           "identities": identities})
        rows.append({
            "lock": str(lock_path),
            "lock_version": lock["lock_version"],
            "owner": lock["owner"],
            "reviewed_at": sorted({r for e in pinned for i in e["identities"]
                                   for r in i["reviewed_at"]}),
            "pinned": len(lock["pinned"]),
            "flexible": len(lock["flexible"]),
            "identity_shapes": dict(sorted(shapes.items())),
            "pinned_without_site": [e["function"] for e in pinned
                                    if not any(i["sites"] for i in e["identities"])],
            "pinned_without_definition": [
                e["function"] for e in pinned
                if not any(site.startswith("definition:")
                           for i in e["identities"] for site in i["sites"])
            ],
            "entries": pinned,
        })
    json.dump({"locks": rows,
               "pinned": sum(r["pinned"] for r in rows),
               "flexible": sum(r["flexible"] for r in rows)}, sys.stdout, indent=2)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
