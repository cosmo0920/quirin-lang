#!/usr/bin/env python3
"""W45-FB-ENGINE-PROGRESS: mechanical loop inventory of the system slice.

Usage: loop_inventory.py ATTEMPT_DIR --out FILE [--work DIR]

Read-only over the prepared attempt (`quirin-audit prepare --output` root). For
each of the eight system-selection files it takes the first (static) compile
record from the unmodified compile database, runs that command with
`-fsyntax-only -Xclang -ast-dump=json` (Clang 21, the prepared compiler), and
walks the AST of every function *defined in that file*. It records:

* every ForStmt / WhileStmt / DoStmt, with the begin/end line of its expansion
  in the file (a loop written through a macro such as `mk_list_foreach` is
  reported at the macro use, with the macro's name when Clang records one);
* inside each loop body: calls (callee name), `return`, `break`, `continue`,
  `goto`, and nested loops;
* per function: the number of loops and the line range.

Nothing is classified here. The classification (bounded / decreasing /
yielding / unresolved) is the hand-written `classification.json`, which
`census.py` joins to this inventory and refuses to publish when a loop is
unclassified or a classified loop is absent (fail-closed both ways).
"""

import argparse
import hashlib
import json
import os
import pathlib
import shlex
import subprocess
import sys

FILES = [
    "plugins/in_storage_backlog/sb.c",
    "src/flb_input_chunk.c",
    "src/flb_storage.c",
    "src/flb_task.c",
    "src/flb_engine.c",
    "src/flb_engine_dispatch.c",
    "src/flb_scheduler.c",
    "src/flb_output.c",
]

LOOP_KINDS = {"ForStmt": "for", "WhileStmt": "while", "DoStmt": "do"}


class Walker:
    """Walk Clang's JSON AST in document order, tracking the current file.

    Clang's JSON omits `file` when it equals the previous location's file, so
    the last seen file is carried through the traversal in order.
    """

    def __init__(self, target: str):
        self.target = os.path.realpath(target)
        self.last_file = None

    def loc_file_line(self, loc):
        """Return (file, line) of the expansion location of a JSON loc."""
        if not loc:
            return None, None
        if "spellingLoc" in loc or "expansionLoc" in loc:
            # Macro location: update the carried file from both, report expansion.
            spell = loc.get("spellingLoc", {})
            exp = loc.get("expansionLoc", {})
            if "file" in spell:
                self.last_file = spell["file"]
            sp_file = self.last_file
            if "file" in exp:
                self.last_file = exp["file"]
            return self.last_file, exp.get("line"), exp.get("isMacroArgExpansion"), sp_file
        if "file" in loc:
            self.last_file = loc["file"]
        return self.last_file, loc.get("line"), None, None

    def touch(self, node):
        """Update the carried file from a node's loc/range in document order."""
        rng = node.get("range", {})
        out = {}
        for key, loc in (("loc", node.get("loc")), ("begin", rng.get("begin")), ("end", rng.get("end"))):
            if loc:
                out[key] = self.loc_file_line(loc)
        return out


def callee_name(node):
    """Best-effort direct callee name of a CallExpr."""
    inner = node.get("inner", [])
    if not inner:
        return None
    fn = inner[0]
    while fn.get("kind") in ("ImplicitCastExpr", "ParenExpr", "CStyleCastExpr") and fn.get("inner"):
        fn = fn["inner"][0]
    if fn.get("kind") == "DeclRefExpr":
        return fn.get("referencedDecl", {}).get("name")
    if fn.get("kind") == "MemberExpr":
        return "(*)." + fn.get("name", "?")
    return "(indirect)"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("attempt", type=pathlib.Path)
    parser.add_argument("--out", type=pathlib.Path, required=True)
    parser.add_argument("--work", type=pathlib.Path)
    args = parser.parse_args()
    attempt = args.attempt.resolve()
    source = attempt / "fluent-bit-clang-source"
    database = attempt / "fluent-bit-clang-production" / "compile_commands.json"
    entries = json.loads(database.read_text())
    work = args.work or (attempt / "loop-inventory-work")
    work.mkdir(parents=True, exist_ok=True)
    result = {
        "schema": "w45-fb-engine-progress/loop-inventory/v1",
        "attempt": str(attempt),
        "compile_database_sha256": hashlib.sha256(database.read_bytes()).hexdigest(),
        "files": [],
    }
    for rel in FILES:
        target = source / rel
        records = [e for e in entries if pathlib.Path(os.path.realpath(e["file"])) == target.resolve()]
        if not records:
            sys.exit(f"no compile record for {rel}")
        entry = records[0]
        argv = entry["arguments"] if "arguments" in entry else shlex.split(entry["command"])
        # Drop the output and the -c; add a syntax-only JSON dump.
        cleaned = []
        skip = False
        for a in argv:
            if skip:
                skip = False
                continue
            if a == "-o":
                skip = True
                continue
            if a == "-c":
                continue
            cleaned.append(a)
        cleaned += ["-fsyntax-only", "-Xclang", "-ast-dump=json"]
        dump = work / (rel.replace("/", "__") + ".ast.json")
        with open(dump, "wb") as fh:
            proc = subprocess.run(cleaned, cwd=entry["directory"], stdout=fh, stderr=subprocess.PIPE)
        if proc.returncode != 0:
            sys.exit(f"clang AST dump failed for {rel}: {proc.stderr.decode()[-2000:]}")
        ast = json.loads(dump.read_text())
        walker = Walker(str(target))
        functions = []

        def walk_body(node, fn, loop_stack):
            locs = walker.touch(node)
            kind = node.get("kind")
            here = locs.get("begin") or locs.get("loc")
            line = here[1] if here else None
            record = None
            if kind == "CallExpr" and "calls" in fn:
                name = callee_name(node)
                if name and name not in fn["calls"]:
                    fn["calls"].append(name)
            if kind in LOOP_KINDS:
                end = locs.get("end")
                record = {
                    "kind": LOOP_KINDS[kind],
                    "line": line,
                    "end_line": end[1] if end else None,
                    "macro": bool(here and here[3] is not None),
                    "depth": len(loop_stack),
                    "calls": [],
                    "returns": [],
                    "breaks": [],
                    "continues": [],
                    "gotos": [],
                    "nested_loop_lines": [],
                }
                if loop_stack:
                    loop_stack[-1]["nested_loop_lines"].append(line)
                fn["loops"].append(record)
                loop_stack = loop_stack + [record]
            elif loop_stack:
                cur = loop_stack[-1]
                if kind == "CallExpr":
                    name = callee_name(node)
                    if name and name not in [c["callee"] for c in cur["calls"] if c["line"] == line]:
                        cur["calls"].append({"callee": name, "line": line})
                elif kind == "ReturnStmt":
                    for l in loop_stack:
                        l["returns"].append(line)
                elif kind == "BreakStmt":
                    cur["breaks"].append(line)
                elif kind == "ContinueStmt":
                    cur["continues"].append(line)
                elif kind == "GotoStmt":
                    for l in loop_stack:
                        l["gotos"].append(line)
            for child in node.get("inner", []):
                walk_body(child, fn, loop_stack)

        for decl in ast.get("inner", []):
            locs = walker.touch(decl)
            if decl.get("kind") != "FunctionDecl":
                continue
            here = locs.get("loc") or locs.get("begin")
            body = [c for c in decl.get("inner", []) if c.get("kind") == "CompoundStmt"]
            if not body:
                # Still walk children to keep the carried file current.
                for child in decl.get("inner", []):
                    walk_body(child, {"loops": []}, [])
                continue
            fn = {"function": decl.get("name"), "loops": [], "calls": []}
            # Parameters first (document order), then the body.
            for child in decl.get("inner", []):
                if child.get("kind") == "CompoundStmt":
                    begin = walker.touch(child)
                    fn_file = begin.get("begin", (None,))[0]
                    fn["line"] = here[1] if here else None
                    fn["end_line"] = begin.get("end", (None, None))[1]
                    fn["file"] = fn_file
                    for grand in child.get("inner", []):
                        walk_body(grand, fn, [])
                else:
                    walk_body(child, {"loops": []}, [])
            if fn.get("file") and os.path.realpath(fn["file"]) == os.path.realpath(str(target)):
                del fn["file"]
                functions.append(fn)
        loops = sum(len(f["loops"]) for f in functions)
        result["files"].append({
            "file": rel,
            "source_sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
            "compile_record_directory": entry["directory"],
            "functions_defined": len(functions),
            "functions_with_loops": sum(1 for f in functions if f["loops"]),
            "loops": loops,
            "functions": functions,
        })
        print(f"{rel}: {len(functions)} functions, {loops} loops")
        dump.unlink()
    result["recursion"] = recursion_cycles(result["files"])
    print(f"recursion cycles in the slice's direct call graph: {len(result['recursion'])}")
    args.out.write_text(json.dumps(result, indent=1) + "\n")


def recursion_cycles(files):
    """Strongly connected components of the slice's direct call graph.

    Nodes are the functions defined in the eight files; an edge is a direct
    call by name to another slice function. Indirect calls (function pointers,
    callbacks through structs) are not edges, so a cycle through a callback
    pointer is not found here; the census records those by reading.
    """
    owner = {}
    collisions = set()
    for f in files:
        for fn in f["functions"]:
            if fn["function"] in owner:
                collisions.add(fn["function"])
            owner[fn["function"]] = f["file"]
    graph = {name: [] for name in owner}
    for f in files:
        for fn in f["functions"]:
            graph[fn["function"]] = sorted({c for c in fn.get("calls", []) if c in owner})
    index = {}
    low = {}
    stack = []
    on_stack = set()
    out = []
    counter = [0]

    def strongconnect(v):
        index[v] = low[v] = counter[0]
        counter[0] += 1
        stack.append(v)
        on_stack.add(v)
        for w in graph[v]:
            if w not in index:
                strongconnect(w)
                low[v] = min(low[v], low[w])
            elif w in on_stack:
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            component = []
            while True:
                w = stack.pop()
                on_stack.discard(w)
                component.append(w)
                if w == v:
                    break
            if len(component) > 1 or v in graph[v]:
                out.append({
                    "functions": sorted(component),
                    "files": sorted({owner[c] for c in component}),
                    "edges": sorted([a, b] for a in component for b in graph[a] if b in component),
                })

    sys.setrecursionlimit(10000)
    for v in sorted(graph):
        if v not in index:
            strongconnect(v)
    return {"cycles": out, "name_collisions": sorted(collisions)} if collisions else out


if __name__ == "__main__":
    main()
