#!/usr/bin/env python3
"""lock_delta.py BASE.lock CURRENT.lock -- card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS:
the pinned functions a contracts.lock gained and lost against its base, the
base entries it changed, and whether every other field is unchanged."""
import json
import sys


def main() -> int:
    base = json.load(open(sys.argv[1], encoding="utf-8"))
    now = json.load(open(sys.argv[2], encoding="utf-8"))
    old = {entry["function"]: entry for entry in base["pinned"]}
    new = {entry["function"]: entry for entry in now["pinned"]}
    print("gained", sorted(set(new) - set(old)))
    print("lost", sorted(set(old) - set(new)))
    print("changed", sorted(name for name in old if name in new and old[name] != new[name]))
    for name in sorted(set(new) - set(old)):
        identities = new[name].get("identities") or []
        print("  %s: %d identities" % (name, len(identities)))
    same = {k: v for k, v in base.items() if k != "pinned"} == {
        k: v for k, v in now.items() if k != "pinned"}
    print("other fields unchanged", same)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
