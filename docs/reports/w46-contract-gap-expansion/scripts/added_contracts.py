#!/usr/bin/env python3
"""Inventory added resource-v1 symbols from tracked provider files against BASE.

Usage: added_contracts.py BASE OUTPUT
This names declarations; it does not measure source or safety coverage.
"""

import argparse
import json
import pathlib
import re
import subprocess


DECLARATION = re.compile(r"extern fn\s+(\w+)\s*\([^;]*?\bcontract\s*\{(.*?)\};", re.S)


def declarations(source):
    source = re.sub(r"/\*.*?\*/", "", source, flags=re.S)
    source = re.sub(r"//[^\n]*", "", source)
    return {name for name, body in DECLARATION.findall(source)
            if "requires resource_contract_v1;" in body}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("base")
    parser.add_argument("output", type=pathlib.Path)
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[4]
    files = subprocess.check_output(
        ["git", "ls-files", "-z", "providers"], cwd=root).decode().split("\0")
    previous = set(subprocess.check_output(
        ["git", "ls-tree", "-r", "--name-only", args.base, "providers"], cwd=root,
        text=True).splitlines())
    added = {}
    for name in files:
        if not name.endswith((".qcore", ".qv")):
            continue
        # A suffix-only migration is not an added declaration.
        old_name = str(pathlib.PurePosixPath(name).with_suffix(".qcore"))
        baseline_name = name if name in previous else old_name
        before = subprocess.check_output(
            ["git", "show", f"{args.base}:{baseline_name}"], cwd=root, text=True
        ) if baseline_name in previous else ""
        after = (root / name).read_text()
        for symbol in sorted(declarations(after) - declarations(before)):
            if symbol in added:
                raise SystemExit(f"duplicate added symbol: {symbol}")
            added[symbol] = name
    args.output.write_text(json.dumps(dict(sorted(added.items())), indent=2) + "\n")
    print(f"{len(added)} added declarations; {args.output}")


if __name__ == "__main__":
    main()
