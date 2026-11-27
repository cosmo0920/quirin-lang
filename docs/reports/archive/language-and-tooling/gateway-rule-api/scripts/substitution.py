#!/usr/bin/env python3
"""GATEWAY-RULE-API: check that two corpus report artifacts differ only by a
one-to-one substitution of SHA-256 digests (the pinned route's per-unit build
identity embeds the side's working directory, and every evidence digest and
id derived from it moves with it).

Usage: substitution.py BEFORE AFTER [BEFORE AFTER ...]

For each pair: find every 64-hex token, align the two texts token by token
(the token sequences must have equal length and equal non-digest text between
them), require the before->after map and its inverse to be functions (one to
one), then apply the map to BEFORE and require byte equality with AFTER.
Prints one JSON line per pair; exit 1 when any pair fails.
"""
import hashlib
import json
import re
import sys

HEX = re.compile(rb"\b[0-9a-f]{64}\b")


def check(before_path, after_path):
    before = open(before_path, "rb").read()
    after = open(after_path, "rb").read()
    record = {"before": before_path, "after": after_path,
              "before_sha256": hashlib.sha256(before).hexdigest(),
              "after_sha256": hashlib.sha256(after).hexdigest()}
    if before == after:
        record.update(result="byte-identical", substituted=0)
        return record, True
    tb, ta = HEX.findall(before), HEX.findall(after)
    if len(tb) != len(ta) or HEX.split(before) != HEX.split(after):
        record.update(result="differs outside digest tokens")
        return record, False
    forward, backward = {}, {}
    for b, a in zip(tb, ta):
        if forward.setdefault(b, a) != a or backward.setdefault(a, b) != b:
            record.update(result="substitution is not one-to-one")
            return record, False
    substituted = HEX.sub(lambda m: forward[m.group(0)], before)
    ok = substituted == after
    moved = sum(1 for b, a in forward.items() if b != a)
    record.update(result="identical under one-to-one digest substitution" if ok
                  else "differs after substitution",
                  digests=len(forward), substituted=moved)
    return record, ok


def main(arguments):
    if not arguments or len(arguments) % 2:
        raise SystemExit(__doc__)
    ok = True
    for index in range(0, len(arguments), 2):
        record, passed = check(arguments[index], arguments[index + 1])
        print(json.dumps(record))
        ok &= passed
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
