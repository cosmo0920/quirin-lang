#!/usr/bin/env python3
"""What the typed facts gain from this card's options, and nothing else.

Usage: facts_diff.py BEFORE_FACTS_DIR AFTER_FACTS_DIR

For library.json and fixed-test.json: every fact kind whose entries differ,
with the keys added or removed; the members and calls this card adds to the
typed options (the handle members, input_coro_list, a descriptor's flags,
is_threaded, flb_input_set_property's key literal, prop_key_check's
arguments inside flb_input_set_property) are expected to be the only
additions. Read-only.
"""

import json
import pathlib
import sys

EXPECTED = ("|coro", "|coll", "|input_coro_list", "|flags", "|is_threaded",
            "flb_input_set_property", "prop_key_check")


def main():
    before, after = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    unexpected = 0
    for name in ("library.json", "fixed-test.json"):
        old = json.loads((before / name).read_text())
        new = json.loads((after / name).read_text())
        for kind in sorted(set(old) | set(new)):
            a, b = old.get(kind), new.get(kind)
            if a == b:
                continue
            if isinstance(a, dict) or isinstance(b, dict):
                a, b = a or {}, b or {}
                added = sorted(set(b) - set(a))
                removed = sorted(set(a) - set(b))
                changed = sorted(k for k in set(a) & set(b) if a[k] != b[k])
            else:
                a, b = set(a or []), set(b or [])
                added, removed, changed = sorted(b - a), sorted(a - b), []
            odd = [k for k in added + removed + changed if not any(e in k for e in EXPECTED)]
            unexpected += len(odd)
            print(f"{name} {kind}: +{len(added)} -{len(removed)} ~{len(changed)}; "
                  f"outside this card's options: {odd[:10]}")
    return 1 if unexpected else 0


if __name__ == "__main__":
    raise SystemExit(main())
