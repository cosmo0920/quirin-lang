#!/usr/bin/env python3
"""Attribute a configured reading's movement to the predefined-macro decisions.

Usage: attribute.py CENSUS_DIR NAME

Compares `NAME-configured-control.functions` (the configured reading without
the facts) with `NAME-configured.functions` and prints: the decisions the
recorded compiler made (file → names), every function and edge only one side
has, and whether each removed function's file is one where a decision was
made (a function or edge in a file without a decision is flagged: it would be
an unattributed movement). Read-only.
"""

import json
import pathlib
import sys


def main():
    directory, name = pathlib.Path(sys.argv[1]), sys.argv[2]
    control = set((directory / f"{name}-configured-control.functions").read_text().splitlines())
    after = set((directory / f"{name}-configured.functions").read_text().splitlines())
    data = json.loads((directory / f"{name}-configured.json").read_text())
    decisions = data["predefined_macros"].get("decisions", {})
    print(f"decisions in {len(decisions)} file(s):")
    for path, names in sorted(decisions.items()):
        print(f"  {path}: {', '.join(names)}")
    decided_files = set(decisions)

    def file_of(function):
        return function.split("::")[0] if "::" in function else None

    unattributed = 0
    for label, lines in (("only before", sorted(control - after)),
                         ("only after", sorted(after - control))):
        functions = [line[len("function "):] for line in lines if line.startswith("function ")]
        edges = [line for line in lines if line.startswith("edge ")]
        print(f"{label}: {len(functions)} function(s), {len(edges)} edge(s)")
        for function in functions:
            where = file_of(function)
            mark = "" if where is None or where in decided_files else "  UNATTRIBUTED FILE"
            if mark:
                unattributed += 1
            print(f"  function {function}{mark}")
        for edge in edges:
            print(f"  {edge}")
    print(f"functions in a file without a decision: {unattributed}")


if __name__ == "__main__":
    main()
