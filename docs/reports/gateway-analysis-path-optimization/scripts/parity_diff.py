#!/usr/bin/env python3
"""GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the parity rows two runs disagree on.

Usage: parity_diff.py BEFORE AFTER [SUFFIX]   (SUFFIX: '', -system, -slice)

Rows are compared as a multiset with Clang node addresses erased (as
identity.py does); prints each row only one side has, with its count.
"""
import collections
import pathlib
import re
import sys

GAPO = pathlib.Path(__file__).resolve().parents[4] / "private/gapo"
ADDRESS = re.compile(r'"0x[0-9a-f]+"')


def load(name, suffix):
    rows = collections.Counter()
    for line in (GAPO / f"{name}{suffix}-parity.jsonl").read_text().splitlines():
        rows[ADDRESS.sub('"0x"', line)] += 1
    return rows


suffix = sys.argv[3] if len(sys.argv) > 3 else ""
before, after = load(sys.argv[1], suffix), load(sys.argv[2], suffix)
for label, rows in (("only before", before - after), ("only after", after - before)):
    for row, count in rows.items():
        print(f"{label} x{count}: {row[:600]}")
