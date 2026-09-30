#!/usr/bin/env python3
"""CG08-CASTS-AND-TYPE-QUERIES: the absorbed families' sizes on one run.

usage: families.py RUN [--json OUT]

Counts, over every function of RUN's resource-diagnostics.json, the
`unsupported` leaves of the families this card absorbs (leaves / functions,
and how many of those functions are Unknown):

* `sizeof-analysis`: `C operator `sizeof` requires type/operand analysis`
  (W44-SIZEOF-CAST-SCALAR-QUERY's family);
* `sizeof-line`: any other `unsupported` leaf whose source line (the leaf's
  `origin`, read from the pinned fluent-bit checkout) spells `sizeof`
  (W44-SIZEOF-OPERAND-RESIDUALS' neighbourhood, CG00's E16 construct feature);
* `cast-return`: the cast-return refusal;
* `assignment-cast`: `unsupported assignment expression` on a line spelling
  a parenthesised cast `( IDENT … * )` (CG00's E15 `cast-in-assignment`,
  approximated lexically).
"""
import argparse
import collections
import functools
import json
import pathlib
import re

ROOT = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/clang/"
    "66910c10a4d7eafa810b84229cf2dcf7b0f26f97/fluent-bit-clang-source"
)
SIZEOF = "C operator `sizeof` requires type/operand analysis"
CAST_RETURN = "cast of a possibly tracked value to another carrier in return expression"
POINTER_CAST = re.compile(r"\(\s*(const\s+)?[A-Za-z_][A-Za-z_0-9 ]*\*+\s*\)")


@functools.lru_cache(None)
def lines_of(name: str):
    hits = [p for p in ROOT.rglob(pathlib.Path(name).name) if str(p).endswith(name)]
    if len(hits) != 1:
        return None
    return hits[0].read_text(errors="replace").splitlines()


def line_text(origin: str) -> str:
    name, line = origin.rsplit(":", 1)
    lines = lines_of(name)
    if not lines:
        return ""
    number = int(line)
    return lines[number - 1] if 0 < number <= len(lines) else ""


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    data = json.loads((args.run / "resource-diagnostics.json").read_text())
    leaves = collections.Counter()
    functions = collections.defaultdict(set)
    unknown = collections.defaultdict(set)
    for unit in data["units"]:
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            for diagnostic in function.get("diagnostics", []):
                if diagnostic.get("category") != "unsupported":
                    continue
                description = diagnostic.get("description", "")
                text = line_text(diagnostic.get("origin", ":0"))
                family = None
                if description == SIZEOF:
                    family = "sizeof-analysis"
                elif description == CAST_RETURN:
                    family = "cast-return"
                elif "sizeof" in text:
                    family = "sizeof-line"
                elif description == "unsupported assignment expression" and POINTER_CAST.search(text):
                    family = "assignment-cast"
                if family:
                    leaves[family] += 1
                    functions[family].add(key)
                    if function["status"] == "Unknown":
                        unknown[family].add(key)
    record = {
        family: {
            "leaves": leaves[family],
            "functions": len(functions[family]),
            "unknown_functions": len(unknown[family]),
        }
        for family in sorted(leaves)
    }
    print(json.dumps(record, indent=1))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
