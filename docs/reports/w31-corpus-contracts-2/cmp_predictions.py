#!/usr/bin/env python3
"""Compare the two prediction inventories section by section."""
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent


def sections(path: pathlib.Path) -> dict:
    out: dict = {}
    key = "header"
    out[key] = []
    for line in path.read_text().splitlines():
        if line.startswith("==="):
            key = line
            out[key] = []
            continue
        out[key].append(line)
    return out


old = sections(ROOT / "prediction-old-baseline.txt")
new = sections(ROOT / "prediction-new-baseline.txt")

for key in sorted(set(old) | set(new)):
    a = old.get(key, [])
    b = new.get(key, [])
    if a == b:
        print(f"{key}: IDENTICAL ({len(a)} lines)")
        continue
    print(f"{key}: DIFFERS")
    sa, sb = set(a), set(b)
    for line in a:
        if line not in sb and line.strip():
            print(f"  -old {line.strip()}")
    for line in b:
        if line not in sa and line.strip():
            print(f"  +new {line.strip()}")
sys.exit(0)
