#!/usr/bin/env python3
"""lock_diff.py BEFORE AFTER -- compare two `contracts.lock` files entry by
entry: the entries whose contract digest moved, and whether each entry's
reviewed identities (sites with their declaration and body digests, and the
revisions each was reviewed at) are equal as sets. Exit 0 when every entry
keeps its identities, 1 otherwise."""
import json
import sys


def entries(path):
    lock = json.load(open(path, encoding="utf-8"))
    return {entry["function"]: entry for entry in lock["pinned"]}, lock


def identities(entry):
    return sorted(
        (
            tuple(sorted(r["revision"] for r in identity["reviewed_at"])),
            json.dumps(identity["sites"], sort_keys=True),
        )
        for identity in entry["identities"]
    )


before, before_lock = entries(sys.argv[1])
after, after_lock = entries(sys.argv[2])
same = True
print("entries", len(before), "->", len(after))
for key in ("lock_version", "owner", "normalisation", "flexible"):
    if before_lock.get(key) != after_lock.get(key):
        print("top-level key differs:", key)
        same = False
for name in sorted(set(before) | set(after)):
    b, a = before.get(name), after.get(name)
    if b is None or a is None:
        print(name, "present on one side only")
        same = False
        continue
    digest = b["contract_sha256"] != a["contract_sha256"]
    kept = identities(b) == identities(a)
    if digest or not kept or b["module"] != a["module"]:
        print(f"{name} ({a['module']}): contract digest {b['contract_sha256'][:12]} -> "
              f"{a['contract_sha256'][:12]}; identities equal: {kept}")
    same = same and kept and b["module"] == a["module"]
sys.exit(0 if same else 1)
