#!/usr/bin/env python3
"""P06 CTraces adapter census.

Read-only. Enumerates every function *defined* in the pinned CTraces translation
units, using the same Clang and the same recorded compile commands the configured
audit uses, and classifies each definition against the shapes the implemented
core checkers accept today.

This script decides nothing about safety. It measures how many CTraces functions
could even be *presented* to an existing checker; a match here is a candidate, not
a verdict.

Usage:
  python3 -B docs/reports/remaining-p06/ctraces_census.py \
      --compile-database BUILD/fluent-bit-clang-production/compile_commands.json \
      [--clang /usr/bin/clang] [--json OUT.json]
"""

import argparse
import collections
import json
import shlex
import subprocess
import sys


def ast_of(entry, clang):
    argv = shlex.split(entry["command"])
    # Drop the driver and the output/compile selectors; ask for a JSON AST only.
    argv = [a for a in argv[1:] if a not in ("-c",)]
    skip = False
    kept = []
    for arg in argv:
        if skip:
            skip = False
            continue
        if arg == "-o":
            skip = True
            continue
        kept.append(arg)
    command = [clang, "-Xclang", "-ast-dump=json", "-fsyntax-only", *kept]
    done = subprocess.run(
        command, cwd=entry["directory"], capture_output=True, text=True
    )
    if done.returncode != 0:
        raise SystemExit(f"clang failed for {entry['file']}:\n{done.stderr[:4000]}")
    return json.loads(done.stdout)


def walk(node, file_state):
    """Yield (file, FunctionDecl) pairs, tracking Clang's sparse loc encoding."""
    loc = node.get("loc") or {}
    if "file" in loc:
        file_state[0] = loc["file"]
    if node.get("kind") == "FunctionDecl":
        yield file_state[0], node
    for child in node.get("inner", []) or []:
        yield from walk(child, file_state)


def body_of(node):
    for child in node.get("inner", []) or []:
        if child.get("kind") == "CompoundStmt":
            return child
    return None


def kinds(node, out):
    out[node.get("kind") or "<unnamed>"] += 1
    for child in node.get("inner", []) or []:
        kinds(child, out)
    return out


def callees(node, out):
    if node.get("kind") == "CallExpr":
        inner = node.get("inner") or []
        if inner:
            head = inner[0]
            while head.get("kind") in ("ImplicitCastExpr", "ParenExpr"):
                sub = head.get("inner") or []
                if not sub:
                    break
                head = sub[0]
            if head.get("kind") == "DeclRefExpr":
                out.append((head.get("referencedDecl") or {}).get("name"))
            else:
                out.append(None)  # indirect call
    for child in node.get("inner", []) or []:
        callees(child, out)
    return out


def classify(fn):
    """Which implemented core-checker shape, if any, this definition could reach.

    The shapes are the *entry* requirements of the checkers named in
    docs/reports/remaining-p06/README.md; passing the shape filter does not mean
    the checker would accept the body.
    """
    ret, _, params = fn["signature"].partition("(")
    ret = ret.strip()
    params = params.rstrip(")").strip()
    arity = 0 if params in ("", "void") else params.count(",") + 1
    reasons = []
    if arity == 1 and params.endswith("*") and ret == "void":
        reasons.append("boolean-field-unit-shape")
    if arity == 1 and params.endswith("*") and ret in ("int", "_Bool"):
        reasons.append("boolean-field-i32-shape")
    if arity == 1 and params.endswith("*") and ret in (
        "unsigned long", "unsigned int", "size_t", "unsigned long long",
    ):
        reasons.append("unsigned-counter-shape")
    if arity == 1 and params.endswith("*") and ret == "void":
        reasons.append("field-destructor-shape")
    if arity == 1 and params.endswith("*") and ret == "int":
        reasons.append("cleanup-outcome-shape")
    return reasons


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compile-database", required=True)
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--library-fragment", default="/ctraces/")
    parser.add_argument("--json")
    args = parser.parse_args()

    database = json.load(open(args.compile_database))
    entries = [e for e in database if args.library_fragment in e["file"]]
    entries.sort(key=lambda e: e["file"])
    if not entries:
        raise SystemExit("no translation units selected")

    functions = []
    for entry in entries:
        unit = entry["file"]
        ast = ast_of(entry, args.clang)
        state = [None]
        for file, node in walk(ast, state):
            if file != unit:
                continue
            body = body_of(node)
            if body is None:
                continue  # a declaration, not a definition
            statement_kinds = kinds(body, collections.Counter())
            called = callees(body, [])
            functions.append(
                {
                    "unit": unit.rsplit("/", 3)[-1],
                    "name": node.get("name"),
                    "signature": (node.get("type") or {}).get("qualType", ""),
                    "storage": node.get("storageClass", "extern"),
                    "statement_kinds": dict(statement_kinds),
                    "calls": sorted({c for c in called if c}),
                    "indirect_calls": sum(1 for c in called if c is None),
                }
            )

    for fn in functions:
        fn["checker_entry_shapes"] = classify(fn)

    shapes = collections.Counter()
    for fn in functions:
        for shape in fn["checker_entry_shapes"] or ["none"]:
            shapes[shape] += 1

    print(f"translation units: {len(entries)}")
    print(f"defined functions: {len(functions)}")
    print("entry-shape matches (a candidate is not an acceptance):")
    for shape, count in sorted(shapes.items()):
        print(f"  {shape}: {count}")
    print("\ncandidates:")
    for fn in functions:
        if fn["checker_entry_shapes"]:
            leaf = not fn["calls"] and not fn["indirect_calls"]
            print(
                f"  {fn['unit']}:{fn['name']} :: {fn['signature']}"
                f"  shapes={','.join(fn['checker_entry_shapes'])}"
                f"  calls={len(fn['calls'])}{'' if not leaf else ' LEAF'}"
            )

    if args.json:
        with open(args.json, "w") as handle:
            json.dump(
                {"units": [e["file"] for e in entries], "functions": functions},
                handle,
                indent=1,
                sort_keys=True,
            )
            handle.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
