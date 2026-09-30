#!/usr/bin/env python3
"""CG07-GENERIC-AND-STATEMENT-EXPRESSIONS: the corpus leaves that sit on a line
holding a `_Generic` selection (E17), read from each unit's preprocessed text.

Usage: generic_lines.py DIAGNOSTICS OUT_DIR [--json OUT]

Each unit named by the run is preprocessed once (`clang -E -P` with its own
compile command from the pinned inputs' compilation databases, output under
OUT_DIR; the inputs are only read). A leaf's `line` is a line of that text
(checked on sampled units: the arbiter's analysed text is the unit's
preprocessed text). Reports every leaf on a line spelling `_Generic`, by
description, and the functions they block.
"""

import argparse
import collections
import json
import pathlib
import shlex
import subprocess

INPUTS = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin")
DATABASES = [
    INPUTS / "fluent-bit-clang-source/build/compile_commands.json",
    INPUTS / "monkey/monkey-build/compile_commands.json",
]


def commands():
    by_name = collections.defaultdict(list)
    for database in DATABASES:
        for entry in json.loads(database.read_text()):
            by_name[pathlib.Path(entry["file"]).name].append(entry)
    return by_name


def preprocess(entry, out: pathlib.Path) -> pathlib.Path | None:
    args = shlex.split(entry["command"]) if "command" in entry else entry["arguments"]
    keep, skip = [], False
    for arg in args[1:]:
        if skip:
            skip = False
            continue
        if arg in ("-o", "-c"):
            skip = True
            continue
        if arg.startswith("-W"):
            continue
        keep.append(arg)
    dest = out / (pathlib.Path(entry["file"]).name + ".i")
    if not dest.exists():
        result = subprocess.run(
            ["/usr/bin/clang", "-E", "-P", *keep, entry["file"], "-o", str(dest)],
            cwd=entry["directory"], capture_output=True, check=False)
        if result.returncode != 0:
            return None
    return dest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("diagnostics", type=pathlib.Path)
    parser.add_argument("out", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    args.out = args.out.resolve()
    args.out.mkdir(parents=True, exist_ok=True)
    path = args.diagnostics
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    data = json.loads(path.read_text())
    by_name = commands()
    leaves = collections.Counter()
    functions = set()
    statuses = collections.Counter()
    unmatched = []
    for unit in data["units"]:
        candidates = by_name.get(unit["file"], [])
        if len(candidates) != 1:
            if any(f.get("diagnostics") for f in unit["functions"]):
                unmatched.append(unit["file"])
            continue
        text = None
        for function in unit["functions"]:
            for diagnostic in function.get("diagnostics", []):
                if text is None:
                    dest = preprocess(candidates[0], args.out)
                    text = dest.read_text(errors="replace").splitlines() if dest else []
                line = diagnostic.get("line")
                if not line or line > len(text) or "_Generic" not in text[line - 1]:
                    continue
                label = diagnostic.get("description") or diagnostic.get("category")
                leaves[label] += 1
                if (unit["file"], function["function"]) not in functions:
                    functions.add((unit["file"], function["function"]))
                    statuses[function["status"]] += 1
    record = {
        "leaves_on_generic_lines": sum(leaves.values()),
        "by_description": dict(leaves.most_common()),
        "functions": len(functions),
        "function_statuses": dict(statuses),
        "function_list": sorted(f"{u}::{f}" for u, f in functions),
        "units_not_matched_to_one_compile_command": sorted(set(unmatched)),
    }
    print(json.dumps(record, indent=1))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
