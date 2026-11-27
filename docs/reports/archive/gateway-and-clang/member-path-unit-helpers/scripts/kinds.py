#!/usr/bin/env python3
"""kinds.py FUNCTIONS.json NAME KIND -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS:
a reading aid. Print each KIND node in function NAME (unit_functions.py
output) with the direct callees and member names below it, to locate the
construct the probe names."""
import json
import pathlib
import sys

sys.setrecursionlimit(100000)


def walk(node, out):
    out.append(node)
    for child in node.get("inner", []):
        if isinstance(child, dict):
            walk(child, out)


def main():
    functions = json.loads(pathlib.Path(sys.argv[1]).read_text())
    name, wanted = sys.argv[2], sys.argv[3]
    for function in functions:
        if function.get("name") != name:
            continue
        nodes = []
        walk(function, nodes)
        for node in nodes:
            if node.get("kind") != wanted:
                continue
            below = []
            walk(node, below)
            calls = sorted({n["referencedDecl"]["name"] for n in below
                            if n.get("kind") == "DeclRefExpr"
                            and n.get("referencedDecl", {}).get("kind") == "FunctionDecl"})
            members = sorted({n.get("name") for n in below if n.get("kind") == "MemberExpr"})
            print(f"{wanted}: calls {calls} members {members}")


main()
