#!/usr/bin/env python3
"""The top-level (and `typed.`) keys whose values differ between two census
readings. Usage: json_keys_diff.py BEFORE.json AFTER.json. Read-only."""

import json
import sys


def main():
    a, b = (json.load(open(p)) for p in sys.argv[1:3])
    for key in sorted(set(a) | set(b)):
        if key == "typed":
            ta, tb = a.get(key, {}), b.get(key, {})
            for sub in sorted(set(ta) | set(tb)):
                if ta.get(sub) != tb.get(sub):
                    print(f"typed.{sub}")
        elif a.get(key) != b.get(key):
            print(key)


if __name__ == "__main__":
    main()
