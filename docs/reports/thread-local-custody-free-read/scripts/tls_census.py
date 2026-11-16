#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card THREAD-LOCAL-CUSTODY-FREE-READ: census of every reported function that
reads a thread-local (`__thread` / `_Thread_local`) object.

Read-only over the inputs.  For each route's report it finds the unit's
compile command, preprocesses the unit, lists the thread-local objects the
preprocessed text declares, finds the main-file function bodies that name one,
asks Clang for each such function's AST (substring name filter, exact
definition kept) and classifies every reference to a thread-local object and
to every automatic local that receives a copy of one.  The admission it
applies is the card's, mirrored from `thread_local_read.rs`:

* the object is thread-local and of object-pointer type, never shadowed by a
  declaration of the function;
* every reference to it is a whole read (lvalue conversion) consumed by a
  plain `=` into an automatic local, an automatic local's initializer, a
  `return` operand or a null test;
* every reference to a local that receives such a copy is a plain `=` target,
  or a whole read consumed in the same four positions.

Anything else (a write, `&`, a `.` member, a dereference, a call argument, an
arithmetic use) withholds the reading for the whole function.

Usage: tls_census.py --out <dir>
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import subprocess
import sys
from pathlib import Path

ROOT = Path("/media/Data/GitHub/quirin-lang")
BASE = ROOT / "private/baselines/497bdeb906e1ad29a94f54079991f0b62bd701df"
CURRENT = ROOT / "private/current-linked-556460e31/reports-89fa31d21/resource-diagnostics.json"
CORPUS_DB = ROOT / "build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json"
MONKEY_DB = ROOT / "build/cycle19/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"
SYMLINK = "/home/cosmo/Documents/GitHub/qcore-c"
CLANG = "/usr/lib/llvm-21/bin/clang"

ROUTES = {
    "corpus": BASE / "eight-home/corpus/corpus/resource-diagnostics.json",
    "system": BASE / "fluent-bit-system/system/resource-diagnostics.json",
    "slice": BASE / "fluent-bit-tail-pause/slice/resource-diagnostics.json",
    "current": CURRENT,
}
LIBRARY_HINT = {
    "librdkafka": "/librdkafka",
    "cmetrics": "/cmetrics/",
    "ctraces": "/ctraces/",
    "chunkio": "/chunkio/",
    "fluent-bit-chunk": "/src/",
}


def fix(text: str) -> str:
    return text.replace(SYMLINK, str(ROOT))


def corpus_command(unit, dbs):
    library, base = unit["library"], unit["file"]
    db = dbs["monkey"] if library == "monkey-libevent" else dbs["fb"]
    for entry in db:
        if entry["file"].endswith("/" + base) and (
            library == "monkey-libevent" or LIBRARY_HINT[library] in entry["file"]
        ):
            return fix(entry["directory"]), fix(entry["file"]), fix(entry["command"])
    return None


def unit_command(route, unit, dbs):
    if route == "corpus":
        return corpus_command(unit, dbs)
    config = unit.get("compile_configuration")
    if not config or not config.get("command"):
        return None
    return config["directory"], config["file"], config["command"]


def frontend_args(command: str, file: str) -> list[str]:
    args = shlex.split(command)[1:]
    out, skip = [], False
    for arg in args:
        if skip:
            skip = False
            continue
        if arg in ("-o", "-MF", "-MT", "-MQ"):
            skip = True
            continue
        if arg in ("-c", "-MD", "-MMD") or arg == file or arg.endswith("/" + os.path.basename(file)) and arg.endswith(".c"):
            continue
        out.append(arg)
    return out


TOKEN = re.compile(r"[A-Za-z_]\w*|\d[\w.]*|\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])*'|\S")
LINE_MARKER = re.compile(r'^#\s*(\d+)\s+"([^"]*)"')


def preprocess(directory, file, args):
    result = subprocess.run(
        [CLANG, *args, "-E", file], cwd=directory, capture_output=True, text=True
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr[-2000:])
    return result.stdout


def thread_locals(text: str) -> set[str]:
    """Names declared with `__thread` or `_Thread_local` anywhere in the text."""
    names = set()
    for statement in re.split(r"[;{}]", text):
        if "__thread" not in statement and "_Thread_local" not in statement:
            continue
        tokens = TOKEN.findall(statement.split("=")[0])
        idents = [t for t in tokens if re.match(r"[A-Za-z_]\w*$", t)]
        if idents:
            last = idents[-1]
            if last not in ("__thread", "_Thread_local"):
                names.add(last)
    return names


def main_file_functions(text: str, main: str, wanted: set[str]) -> set[str]:
    """Main-file function definitions whose body names one of `wanted`."""
    current_main = False
    tokens: list[str] = []
    for line in text.splitlines():
        marker = LINE_MARKER.match(line)
        if marker:
            current_main = os.path.realpath(marker.group(2)) == os.path.realpath(main)
            tokens.append("\0break")
            continue
        if current_main:
            tokens.extend(TOKEN.findall(line))
        else:
            tokens.append("\0skip")
    found = set()
    depth, i = 0, 0
    while i < len(tokens):
        token = tokens[i]
        if token == "{" and depth == 0:
            # function definition: previous significant token is `)`
            j = i - 1
            while j >= 0 and tokens[j].startswith("\0"):
                j -= 1
            name = None
            if j >= 0 and tokens[j] == ")":
                level, k = 0, j
                while k >= 0:
                    if tokens[k] == ")":
                        level += 1
                    elif tokens[k] == "(":
                        level -= 1
                        if level == 0:
                            break
                    k -= 1
                if k > 0 and re.match(r"[A-Za-z_]\w*$", tokens[k - 1]):
                    name = tokens[k - 1]
            level, k = 0, i
            body = []
            while k < len(tokens):
                if tokens[k] == "{":
                    level += 1
                elif tokens[k] == "}":
                    level -= 1
                    if level == 0:
                        break
                body.append(tokens[k])
                k += 1
            if name and wanted.intersection(body):
                found.add(name)
            i = k + 1
            continue
        i += 1
    return found


def function_ast(directory, file, args, name):
    result = subprocess.run(
        [CLANG, *args, "-fsyntax-only", "-Xclang", "-ast-dump=json", "-Xclang",
         "-ast-dump-filter", "-Xclang", name, file],
        cwd=directory, capture_output=True, text=True,
    )
    decoder = json.JSONDecoder()
    text, at, found = result.stdout, 0, []
    while True:
        while at < len(text) and text[at] not in "{[":
            at += 1
        if at >= len(text):
            break
        value, end = decoder.raw_decode(text, at)
        at = end
        if (
            value.get("kind") == "FunctionDecl"
            and value.get("name") == name
            and any(c.get("kind") == "CompoundStmt" for c in value.get("inner", []))
        ):
            found.append(value)
    return found[0] if len(found) == 1 else None


def kids(node):
    return node.get("inner", []) or []


def walk(node, chain, out):
    out.append((node, chain))
    for child in kids(node):
        walk(child, chain + [node], out)


def object_pointer(qual: str) -> bool:
    qual = qual.strip()
    return qual.endswith("*") and "(" not in qual and "[" not in qual and "volatile" not in qual.rsplit("*", 1)[1] and "_Atomic" not in qual


def strip_up(node, chain):
    """The consumer of a value: skip parentheses and pointer-preserving casts."""
    index = len(chain) - 1
    current = node
    while index >= 0:
        parent = chain[index]
        kind = parent.get("kind")
        if kind == "ParenExpr" or (
            kind in ("ImplicitCastExpr", "CStyleCastExpr")
            and parent.get("castKind") in ("NoOp", "BitCast")
        ):
            current = parent
            index -= 1
            continue
        return parent, current, index
    return None, current, -1


def classify(ref, chain, automatic):
    parent = chain[-1] if chain else {}
    kind = parent.get("kind")
    if kind == "UnaryOperator" and parent.get("opcode") == "&":
        return "address"
    if kind == "UnaryOperator" and parent.get("opcode") in ("++", "--"):
        return "write"
    if kind == "MemberExpr":
        return "member"
    if kind == "ImplicitCastExpr" and parent.get("castKind") == "ArrayToPointerDecay":
        return "array"
    if kind in ("BinaryOperator", "CompoundAssignOperator") and kids(parent) and kids(parent)[0] is ref:
        if kind == "BinaryOperator" and parent.get("opcode") == "=":
            return "assigned"
        if kind == "CompoundAssignOperator":
            return "write"
    if not (kind == "ImplicitCastExpr" and parent.get("castKind") == "LValueToRValue"):
        return f"other:{kind}"
    consumer, value, _ = strip_up(parent, chain[:-1])
    ckind = (consumer or {}).get("kind")
    if ckind == "BinaryOperator" and consumer.get("opcode") == "=" and kids(consumer)[1] is value:
        lhs = kids(consumer)[0]
        if lhs.get("kind") == "DeclRefExpr" and lhs.get("referencedDecl", {}).get("id") in automatic:
            return "copy:" + lhs["referencedDecl"]["name"]
        return "store"
    if ckind == "VarDecl":
        if consumer.get("id") in automatic:
            return "init:" + consumer["name"]
        return "store"
    if ckind == "ReturnStmt":
        return "return"
    if ckind == "BinaryOperator" and consumer.get("opcode") in ("==", "!="):
        return "null-test"
    if ckind == "UnaryOperator" and consumer.get("opcode") == "!":
        return "null-test"
    if ckind in ("IfStmt", "WhileStmt", "DoStmt", "ConditionalOperator") and kids(consumer) and kids(consumer)[0] is value:
        return "null-test"
    if ckind == "ImplicitCastExpr" and consumer.get("castKind") == "PointerToBoolean":
        return "null-test"
    if ckind == "MemberExpr":
        return "deref-member"
    if ckind == "UnaryOperator" and consumer.get("opcode") == "*":
        return "deref"
    if ckind == "ArraySubscriptExpr":
        return "deref-index"
    if ckind == "CallExpr":
        return "call-argument"
    return f"other-read:{ckind}"


ALLOWED_TLS = ("copy:", "init:", "return", "null-test")
ALLOWED_LOCAL = ("copy:", "init:", "return", "null-test", "assigned")


def analyse(function):
    nodes = []
    walk(function, [], nodes)
    automatic, declared = set(), set()
    for node, chain in nodes:
        if node.get("kind") in ("VarDecl", "ParmVarDecl"):
            declared.add(node.get("name"))
        if node.get("kind") == "VarDecl" and "tls" not in node and node.get("storageClass") in (None, "register"):
            automatic.add(node.get("id"))
    automatic_names = {n.get("name") for n, _ in nodes if n.get("id") in automatic}
    return nodes, automatic, automatic_names, declared


def census_function(function, tls_names):
    nodes, automatic, automatic_names, declared = analyse(function)
    uses: dict[str, list[str]] = {}
    tls_refs = {}
    local_refs: dict[str, list[str]] = {}
    for node, chain in nodes:
        if node.get("kind") != "DeclRefExpr":
            continue
        ref = node.get("referencedDecl", {})
        name = ref.get("name")
        if ref.get("kind") != "VarDecl":
            continue
        if name in tls_names and name not in declared:
            tls_refs.setdefault(name, {"type": node.get("type", {}).get("qualType", ""), "uses": []})
            tls_refs[name]["uses"].append(classify(node, chain, automatic))
        elif ref.get("id") in automatic:
            local_refs.setdefault(name, []).append(classify(node, chain, automatic))
    if not tls_refs:
        return None
    copies = set()
    frontier = [u.split(":", 1)[1] for r in tls_refs.values() for u in r["uses"] if u.startswith(("copy:", "init:"))]
    while frontier:
        name = frontier.pop()
        if name in copies:
            continue
        copies.add(name)
        for use in local_refs.get(name, []):
            if use.startswith(("copy:", "init:")):
                frontier.append(use.split(":", 1)[1])
    reasons = []
    for name, record in tls_refs.items():
        if not object_pointer(record["type"]):
            reasons.append(f"{name}: not an object pointer ({record['type']})")
        for use in record["uses"]:
            if not use.startswith(ALLOWED_TLS):
                reasons.append(f"{name}: {use}")
    for name in sorted(copies):
        for use in local_refs.get(name, []):
            if not use.startswith(ALLOWED_LOCAL):
                reasons.append(f"copy {name}: {use}")
    return {
        "thread_locals": tls_refs,
        "copies": sorted(copies),
        "copy_uses": {name: local_refs.get(name, []) for name in sorted(copies)},
        "admitted": not reasons,
        "withheld_because": reasons,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("--routes", default="corpus,system,slice,current")
    options = parser.parse_args()
    out = Path(options.out)
    out.mkdir(parents=True, exist_ok=True)
    dbs = {"fb": json.load(open(CORPUS_DB)), "monkey": json.load(open(MONKEY_DB))}
    census = {}
    for route in options.routes.split(","):
        report = json.load(open(ROUTES[route]))
        rows = []
        seen_units = {}
        for unit in report["units"]:
            located = unit_command(route, unit, dbs)
            if not located:
                rows.append({"unit": unit["file"], "error": "no compile command"})
                continue
            directory, file, command = located
            args = frontend_args(command, file)
            key = (file, tuple(args))
            if key not in seen_units:
                try:
                    text = preprocess(directory, file, args)
                except RuntimeError as error:
                    seen_units[key] = {"error": str(error)[:300]}
                    continue
                names = thread_locals(text)
                functions = main_file_functions(text, file, names) if names else set()
                analysed = {}
                for name in sorted(functions):
                    ast = function_ast(directory, file, args, name)
                    if ast is None:
                        analysed[name] = {"error": "no unique definition"}
                        continue
                    record = census_function(ast, names)
                    if record:
                        analysed[name] = record
                seen_units[key] = {"functions": analysed}
            facts = seen_units[key]
            if "error" in facts:
                rows.append({"unit": unit["file"], "error": facts["error"]})
                continue
            for index, function in enumerate(unit.get("functions", [])):
                name = function.get("function") or (unit["source_functions"][index] if index < len(unit["source_functions"]) else None)
                if name not in facts["functions"]:
                    continue
                record = facts["functions"][name]
                rows.append({
                    "unit": unit["file"],
                    "function": name,
                    "status": function.get("status"),
                    "diagnostics": [d.get("description") for d in function.get("diagnostics", [])],
                    **record,
                })
            print(f"{route}: {unit['file']}: {len(facts['functions'])} thread-local readers", file=sys.stderr)
        census[route] = rows
    (out / "tls_census.json").write_text(json.dumps(census, indent=1, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
