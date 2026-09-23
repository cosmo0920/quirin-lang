#!/usr/bin/env python3
"""Classify every callee of the uncontracted-call family by where C declares it.

Usage: libc_inventory.py DIAGNOSTICS.json.gz PREPROCESSED_DIR SOURCE_ROOT OUT.json

DIAGNOSTICS is a corpus baseline's `diagnostics.json.gz`. PREPROCESSED_DIR
holds the line-marked (`clang -E`, no `-P`) text of every analysed unit, as a
transparent Clang wrapper saved it during a private run of the same audit
command (`*.i`). SOURCE_ROOT is the pinned Fluent Bit checkout.

For each callee named by a `call to uncontracted function `NAME`` leaf, the
script scans every unit's preprocessed text with a small C lexer that tracks
the current file (line markers) and brace depth, and records every file-scope
occurrence `NAME (` -- a declaration, or a definition when the matching `)` is
followed by `{`. The owning Debian package of each declaring system header is
read with `dpkg -S`. A callee is **libc/POSIX** exactly when

* it has a file-scope declaration in a header owned by `libc6-dev` or
  `linux-libc-dev` (glibc and the Linux UAPI headers), and
* no analysed unit carries a file-scope definition of it (neither in corpus
  text nor as a system-header inline body), and
* no corpus (non-system) text declares it at file scope either, and
* no `.c`/`.h` file of the pinned source tree defines it (a textual check for
  `NAME (...) {` at the start of a line, to catch compatibility bodies that the
  analysed units do not include; such a callee is `libc-with-tree-body`).

Everything else is classified by the first rule that applies: `builtin`
(`__builtin_*`, `_Generic`, `__asm__` and other non-functions), `libc-inline`
(defined by a libc header in the unit, e.g. `__bswap_32`), `corpus` (declared
or defined in corpus text), `other-system` (declared only by another package's
header, e.g. OpenSSL), `unresolved` (no file-scope declaration found).
"""

import collections
import gzip
import json
import pathlib
import re
import subprocess
import sys

CALL = re.compile(r"call to uncontracted function `([^`]+)`")
MARKER = re.compile(r'^# (\d+) "((?:[^"\\]|\\.)*)"((?: \d)*)\s*$')
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
LIBC_PACKAGES = {"libc6-dev", "linux-libc-dev"}
NOT_FUNCTIONS = {"_Generic", "__asm__", "asm", "sizeof", "__typeof__", "typeof", "__extension__"}
VCS_DIRECTORY = "." + "git"


def inventory(path):
    data = json.load(gzip.open(path))
    leaves = collections.Counter()
    functions = collections.defaultdict(set)
    statuses = collections.defaultdict(collections.Counter)
    sole = collections.Counter()
    only_calls = collections.Counter()
    for unit in data["units"]:
        for entry in unit["functions"]:
            descriptions = [d["description"] for d in entry.get("diagnostics") or []]
            callees = set()
            for description in descriptions:
                match = CALL.fullmatch(description)
                if match:
                    leaves[match.group(1)] += 1
                    callees.add(match.group(1))
            for callee in callees:
                functions[callee].add((unit["library"], unit["file"], entry["function"]))
                statuses[callee][entry["status"]] += 1
            if entry["status"] != "Unknown" or not callees:
                continue
            if len(set(descriptions)) == 1:
                sole[next(iter(callees))] += 1
            if all(CALL.fullmatch(d) for d in descriptions):
                for callee in callees:
                    only_calls[callee] += 1
    return leaves, functions, statuses, sole, only_calls


def scan(text, names, found, system_files):
    """Record file-scope `NAME (` occurrences, split by current file."""
    current = None
    system = False
    depth = 0
    lines = text.split("\n")
    for index, line in enumerate(lines):
        marker = MARKER.match(line)
        if marker:
            current = marker.group(2)
            system = " 3" in marker.group(3)
            if system:
                system_files.add(current)
            continue
        position = 0
        length = len(line)
        while position < length:
            char = line[position]
            if char in "\"'":
                quote = char
                position += 1
                while position < length and line[position] != quote:
                    position += 2 if line[position] == "\\" else 1
                position += 1
                continue
            if char == "{":
                depth += 1
            elif char == "}":
                depth = max(0, depth - 1)
            elif char.isalpha() or char == "_":
                match = IDENT.match(line, position)
                word = match.group(0)
                position = match.end()
                if depth == 0 and word in names and line[position:].lstrip().startswith("("):
                    kind = definition_kind(lines, index, position)
                    found[word].add((current, system, kind, line.strip()[:200]))
                continue
            position += 1


def definition_kind(lines, index, position):
    """`definition` if the parenthesis after the name closes before a `{`."""
    depth = 0
    started = False
    row, column = index, position
    budget = 0
    while row < len(lines) and budget < 4000:
        line = lines[row]
        if line.startswith("# "):
            row += 1
            column = 0
            continue
        while column < len(line):
            char = line[column]
            budget += 1
            if char == "(":
                depth += 1
                started = True
            elif char == ")":
                depth -= 1
            elif started and depth == 0:
                if char == "{":
                    return "definition"
                if char in ";,=":
                    return "declaration"
            column += 1
        row += 1
        column = 0
    return "declaration"


def owners(files):
    result = {}
    paths = sorted(f for f in files if f.startswith("/"))
    for start in range(0, len(paths), 200):
        chunk = paths[start : start + 200]
        run = subprocess.run(["dpkg", "-S", *chunk], capture_output=True, text=True)
        for line in run.stdout.splitlines():
            package, _, path = line.partition(": ")
            for owned in path.split(", "):
                result[owned.strip()] = package.split(":")[0]
    return result


def tree_definitions(root, names):
    if not names:
        return {}
    alternation = "|".join(map(re.escape, sorted(names)))
    pattern = re.compile(r"^[A-Za-z_][^;()=]*\b(" + alternation + r")\s*\([^;]*$")
    found = collections.defaultdict(set)
    for path in root.rglob("*"):
        if path.suffix not in {".c", ".h"} or VCS_DIRECTORY in path.parts or not path.is_file():
            continue
        try:
            lines = path.read_text(errors="replace").split("\n")
        except OSError:
            continue
        for number, line in enumerate(lines):
            match = pattern.match(line)
            if not match:
                continue
            window = " ".join(lines[number : number + 6])
            head = window.split("{", 1)[0]
            if "{" in window and re.search(r"\)\s*$", head.rstrip()) and ";" not in head:
                found[match.group(1)].add(f"{path.relative_to(root)}:{number + 1}")
    return found


def main() -> int:
    diagnostics, preprocessed, source_root, out = sys.argv[1:5]
    leaves, functions, statuses, sole, only_calls = inventory(diagnostics)
    names = set(leaves)
    found = collections.defaultdict(set)
    system_files = set()
    units = sorted(pathlib.Path(preprocessed).glob("*.i"))
    for unit in units:
        scan(unit.read_text(errors="replace"), names, found, system_files)
    package = owners({f for occ in found.values() for (f, system, _, _) in occ if system})
    candidates = {
        n
        for n in names
        if any(s and package.get(f) in LIBC_PACKAGES for (f, s, _, _) in found[n])
    }
    tree = tree_definitions(pathlib.Path(source_root), candidates)
    rows = []
    for name in sorted(names):
        occurrences = found[name]
        system_decl = {(f, package.get(f, "unowned"), k) for (f, s, k, _) in occurrences if s}
        corpus = {(f, k) for (f, s, k, _) in occurrences if not s}
        libc_decl = sorted({f for (f, p, k) in system_decl if p in LIBC_PACKAGES and k == "declaration"})
        libc_def = sorted({f for (f, p, k) in system_decl if p in LIBC_PACKAGES and k == "definition"})
        other = sorted({(f, p) for (f, p, k) in system_decl if p not in LIBC_PACKAGES})
        if name.startswith("__builtin_") or name in NOT_FUNCTIONS:
            kind = "builtin"
        elif libc_def:
            kind = "libc-inline"
        elif corpus:
            kind = "corpus"
        elif libc_decl and tree.get(name):
            kind = "libc-with-tree-body"
        elif libc_decl:
            kind = "libc"
        elif other:
            kind = "other-system"
        else:
            kind = "unresolved"
        rows.append(
            {
                "callee": name,
                "class": kind,
                "leaves": leaves[name],
                "functions": len(functions[name]),
                "sole_blocked": sole[name],
                "blocked_only_by_uncontracted_calls": only_calls[name],
                "statuses": dict(sorted(statuses[name].items())),
                "libc_headers": libc_decl + libc_def,
                "other_headers": [f"{f} ({p})" for f, p in other],
                "corpus_occurrences": sorted({f"{f} [{k}]" for f, k in corpus})[:8],
                "tree_definitions": sorted(tree.get(name, ()))[:8],
                "blocked_functions": sorted("/".join(t) for t in functions[name]),
            }
        )
    record = {
        "diagnostics": diagnostics,
        "preprocessed_units": len(units),
        "callees": len(rows),
        "class_counts": dict(sorted(collections.Counter(r["class"] for r in rows).items())),
        "rows": rows,
    }
    pathlib.Path(out).write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps({k: record[k] for k in ("preprocessed_units", "callees", "class_counts")}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
