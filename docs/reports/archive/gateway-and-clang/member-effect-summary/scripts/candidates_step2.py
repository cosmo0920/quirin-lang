#!/usr/bin/env python3
"""MEMBER-EFFECT-SUMMARY step 2: candidate rows of a published report for the
member-release summary, from the report's leaves and the pinned sources.

A row is a candidate when it is Unknown and every one of its diagnostics is
the gate's refusal on an origin line that spells a member release through a
name (`R(B->F);`, optionally under `if (...)`), the release statement shape
rule 38i names `member-release-through-formal` where `B` is a formal and `R`'s
contract releases there. The analyzer decides; this only lists rows to read.

usage: candidates_step2.py REPORT SOURCE_ROOT
"""
import json
import pathlib
import re
import sys

IND = "indirect memory access is outside the resource projection"
SHAPE = re.compile(r"^(?:\}?\s*(?:else\s+)?if\s*\(.*\)\s*\{?\s*)?(\w+)\s*\(\s*\(?\s*(\w+)\s*->\s*(\w+)\s*\)?\s*\)\s*;\s*\}?$")


def main() -> int:
    report = json.load(open(sys.argv[1]))
    root = pathlib.Path(sys.argv[2])
    index = {}
    for path in root.rglob("*.c"):
        index.setdefault(path.name, []).append(path)
    for path in root.rglob("*.h"):
        index.setdefault(path.name, []).append(path)
    cache = {}

    def line(origin):
        file, _, number = origin.rpartition(":")
        if file not in cache:
            paths = index.get(pathlib.Path(file).name, [])
            cache[file] = paths[0].read_text(errors="replace").splitlines() if len(paths) == 1 else []
        lines = cache[file]
        return lines[int(number) - 1].strip() if number.isdigit() and int(number) <= len(lines) else ""

    for unit in report["units"]:
        for function in unit["functions"]:
            if function["status"] != "Unknown" or not function["diagnostics"]:
                continue
            shapes = []
            for leaf in function["diagnostics"]:
                if leaf["category"] != "unsupported" or leaf["description"] != IND:
                    break
                text = line(leaf.get("origin") or "")
                match = SHAPE.match(text)
                if not match:
                    break
                shapes.append(f"{match.group(1)}({match.group(2)}->{match.group(3)})")
            else:
                print(f"{unit['file']}::{function['function']} | {'; '.join(shapes)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
