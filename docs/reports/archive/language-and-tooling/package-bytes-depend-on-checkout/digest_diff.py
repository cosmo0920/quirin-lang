#!/usr/bin/env python3
"""Compare two `providers/package.py build` outputs: packages.json fields and
every file's SHA-256. Usage: digest_diff.py BASE_OUT TIP_OUT"""
import hashlib
import json
from pathlib import Path
import sys


def tree(root):
    return {path.relative_to(root).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(Path(root).rglob("*")) if path.is_file()}


def main(base, tip):
    left = json.loads((Path(base) / "packages.json").read_text())
    right = json.loads((Path(tip) / "packages.json").read_text())
    for key in sorted(set(left) | set(right) - {"artifacts"}):
        if key != "artifacts":
            print(f"{key}: {'equal' if left.get(key) == right.get(key) else 'DIFFERENT'}")
    for old, new in zip(left["artifacts"], right["artifacts"]):
        assert old["plugin_id"] == new["plugin_id"]
        print(f"\n{old['plugin_id']}")
        for key in ("executable_sha256", "manifest_sha256", "contracts_manifest_sha256",
                    "archive_sha256"):
            print(f"  {key}: {old[key]} (equal)" if old[key] == new[key]
                  else f"  {key}: {old[key]} -> {new[key]}")
    a, b = tree(base), tree(tip)
    print(f"\nfiles: {len(a)} and {len(b)}, same names: {set(a) == set(b)}")
    print("differing files:")
    for name in sorted(n for n in set(a) | set(b) if a.get(n) != b.get(n)):
        print(f"  {name}")


if __name__ == "__main__":
    main(*sys.argv[1:])
