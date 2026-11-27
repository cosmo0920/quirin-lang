#!/usr/bin/env python3
"""Whether two route reports carry the same `units` (every row and leaf).

The configured route's report also records the checkout-specific profile
directory, so two builds from different worktrees differ in bytes while
their rows agree; this compares the rows. usage: units_equal.py A B
"""
import json
import sys

a, b = (json.load(open(path))["units"] for path in sys.argv[1:3])
equal = json.dumps(a, sort_keys=True) == json.dumps(b, sort_keys=True)
print("units equal" if equal else "units differ")
sys.exit(0 if equal else 1)
