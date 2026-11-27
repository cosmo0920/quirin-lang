#!/usr/bin/env python3
"""unit_functions.py AST OUT -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: keep
the top-level FunctionDecl nodes with a body from a unit's Clang JSON AST
(unit_ast.py), each kept whole except its `loc`/`range` keys, as a JSON array
in OUT: the unit's functions as the gateway's AST index retains them, for
the probe (`examples/member_path_probe.rs`)."""
import json
import pathlib
import sys

sys.setrecursionlimit(100000)


def drop(node):
    if isinstance(node, dict):
        node.pop("loc", None)
        node.pop("range", None)
        for child in node.get("inner", []):
            drop(child)
    return node


def main():
    ast = json.loads(pathlib.Path(sys.argv[1]).read_text())
    kept = [drop(node) for node in ast.get("inner", [])
            if node.get("kind") == "FunctionDecl" and node.get("name")
            and any(child.get("kind") == "CompoundStmt" for child in node.get("inner", []))]
    pathlib.Path(sys.argv[2]).write_text(json.dumps(kept))
    print(len(kept))


main()
