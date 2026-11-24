#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""llp64_fixture_scan.py [options] PATH... -- list LLP64 hazards in synthetic
C fixtures (card WINDOWS-CI-LLP64-FIXTURES-2).

Windows CI analyses under `x86_64-pc-windows-msvc`, an LLP64 target with the
MSVC ABI. Three fixture spellings read on Linux but withdraw a declared
contract on Windows (fail-closed, rules 11q and 12u) and flip a test:

  plain-long    plain `long`/`unsigned long` (not `long long`): 32 bits under
                LLP64, and the declared-ABI reader reads it only on LP64. Spell
                sizes `__SIZE_TYPE__`, signed sizes `__PTRDIFF_TYPE__` and
                64-bit values `long long`.
  enum-type     a parameter or result typed by an enumeration (`enum TAG` or a
                `typedef enum` name). It is `unsigned int` on Linux when no
                enumerator is negative and `int` under MSVC, so a `u32`
                contract position disagrees. Spell the type `unsigned int` (or
                `int`) and keep the constants in an untagged `enum { ... };`.
                A fixed underlying type (`enum : unsigned int`) is unreadable
                to the declared-ABI reader on both targets.
  host-include  `#include <h>` of a hosted header while a contracted callee is
                declared by no fixture text: the UCRT's headers are unread on
                Windows. Declare the callee in the fixture. Clang's
                freestanding headers are exempt.

Inputs: Rust files (the C text inside their string literals is scanned, each
literal separately) and C files (.c/.h, scanned whole). A directory is walked
for those suffixes. `--stdin` also reads newline-separated paths, so
`git diff --name-only BASE | llp64_fixture_scan.py --stdin` works.

A hit counts when it binds to a declared contract: it stands in a file-scope
prototype of a contracted name, or in a typedef that a contracted prototype in
the same file uses. Contracted names are the `extern fn` names of the
repository's provider and gateway-fixture contracts, of any `--contracts`
path, and of contract text inside the scanned file. Other hits (callback or
`subject` definitions, locals, record fields, uncontracted prototypes) are
counted only with `--strict` and printed only with `--all` or `--strict`: a
callback definition handed to a contracted callback position is such a hit,
so review `--strict` output for callback fixtures.

A Rust file containing a `cfg(windows)`/`cfg(unix)`/`target_os` test is
annotated: it may expect the Windows reading on purpose (CI's host is
Windows, so a Linux simulation cannot flip `cfg!(windows)`).

A hit is suppressed when its source line, or the line just above the string
literal holding it, contains `llp64-scan: allow`.

The scan is a lister: it never edits. Output: `path:line: [hazard] text
(why)`, then a count on stderr. Exit 0: nothing counted; 1: hazards counted;
2: an unreadable input or usage error.
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from dataclasses import dataclass

ALLOW = "llp64-scan: allow"
SUFFIXES = (".rs", ".c", ".h")
SKIP_DIRS = {"target", "." + "git"}
FREESTANDING = {
    "float.h", "iso646.h", "limits.h", "stdalign.h", "stdarg.h", "stdatomic.h",
    "stdbool.h", "stddef.h", "stdint.h", "stdnoreturn.h", "stdckdint.h",
    "stdbit.h", "varargs.h",
}
KEYWORDS = {"sizeof", "if", "while", "for", "switch", "return", "_Generic", "_Alignof"}
EXTERN = re.compile(r"\bextern\s+fn\s+(\w+)")
LONG = re.compile(r"\blong\b")
INCLUDE = re.compile(r"#\s*include\s*<([^>\s]+)>")
ENUM_TYPEDEF = re.compile(r"\btypedef\s+enum\s*(?:\w+\s*)?(?:\{[^{}]*\}\s*)?\**\s*(\w+)\s*;")
WINDOWS_CFG = re.compile(r"cfg!?\((?:not\()?(?:windows|unix|target_os|target_family)")
ENUM_TAG = re.compile(r"\benum\s+(\w+)\s*\{")


@dataclass
class Fragment:
    """C text with, per character, the 1-based source line it came from."""

    text: str
    lines: list[int]
    open_line: int


@dataclass
class Raw:
    at: int
    hazard: str
    shown: str
    klass: str
    stmt: str


@dataclass
class Hit:
    path: str
    line: int
    hazard: str
    text: str
    why: str
    binds: bool


# --- Rust string-literal extraction --------------------------------------------


def rust_fragments(source: str) -> list[Fragment]:
    out: list[Fragment] = []
    i, n, line = 0, len(source), 1

    def advance(k: int) -> None:
        nonlocal i, line
        line += source.count("\n", i, i + k)
        i += k

    def ident_before() -> bool:
        return i > 0 and (source[i - 1].isalnum() or source[i - 1] == "_")

    while i < n:
        c = source[i]
        if source.startswith("//", i):
            j = source.find("\n", i)
            advance((n if j < 0 else j) - i)
            continue
        if source.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if source.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif source.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            advance(j - i)
            continue
        raw = re.match(r'(?:b|c)?r(#*)"', source[i:i + 260])
        if raw and not ident_before():
            hashes = raw.group(1)
            start = i + raw.end()
            end = source.find('"' + hashes, start)
            end = n if end < 0 else end
            open_line = line
            advance(start - i)
            text, lines = [], []
            for ch in source[start:end]:
                text.append(ch)
                lines.append(line)
                if ch == "\n":
                    line += 1
            i = end
            advance(min(n, end + 1 + len(hashes)) - i)
            out.append(Fragment("".join(text), lines, open_line))
            continue
        if c == '"' or (source.startswith(('b"', 'c"'), i) and not ident_before()):
            open_line = line
            advance(1 if c == '"' else 2)
            text, lines = [], []
            while i < n and source[i] != '"':
                ch = source[i]
                if ch == "\\" and i + 1 < n:
                    nxt = source[i + 1]
                    if nxt == "\n":
                        advance(2)
                        while i < n and source[i] in " \t\r\n":
                            advance(1)
                        continue
                    mapped = {"n": "\n", "t": " ", "r": " ", "0": " ", "x": " ", "u": " "}.get(nxt, nxt)
                    text.append(mapped)
                    lines.append(line)
                    advance(2)
                    continue
                text.append(ch)
                lines.append(line)
                advance(1)
            advance(1)
            out.append(Fragment("".join(text), lines, open_line))
            continue
        if c == "'":
            char = re.match(r"'(?:\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^\\'])'", source[i:i + 12])
            advance(char.end() if char else 1)
            continue
        advance(1)
    return out


def c_fragment(source: str) -> Fragment:
    lines, line = [], 1
    for ch in source:
        lines.append(line)
        if ch == "\n":
            line += 1
    return Fragment(source, lines, 0)


# --- C classification ------------------------------------------------------------


def blank_comments(text: str) -> str:
    def blank(match: re.Match[str]) -> str:
        return re.sub(r"[^\n]", " ", match.group(0))

    return re.sub(r"/\*.*?\*/|//[^\n]*", blank, text, flags=re.S)


def contexts(text: str) -> list[str]:
    """Per character: 'top', 'aggregate' or 'body' (any enclosing body wins)."""
    stack: list[str] = []
    out: list[str] = []
    last = ""
    for ch in text:
        if ch == "{":
            stack.append("body" if ("body" in stack or last == ")") else "aggregate")
        elif ch == "}" and stack:
            stack.pop()
        out.append("body" if "body" in stack else ("aggregate" if stack else "top"))
        if not ch.isspace():
            last = ch
    return out


def classify(text: str, ctx: list[str], at: int) -> str:
    if ctx[at] == "body":
        return "local"
    if ctx[at] == "aggregate":
        return "field"
    depth = 0
    for j in range(at, len(text)):
        ch = text[j]
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        elif depth <= 0 and ch == ";":
            return "declaration"
        elif depth <= 0 and ch == "{":
            return "definition" if text[:j].rstrip().endswith(")") else "declaration"
    return "declaration"


def statement(text: str, at: int) -> str:
    begin = max(text.rfind(";", 0, at), text.rfind("{", 0, at), text.rfind("}", 0, at)) + 1
    ends = [k for k in (text.find(";", at), text.find("{", at)) if k >= 0]
    return text[begin:min(ends) if ends else len(text)]


def prototype_name(stmt: str) -> str | None:
    pointer = re.search(r"\(\s*\*\s*(\w+)\s*\)\s*\(", stmt)
    if pointer:
        return pointer.group(1)
    called = re.search(r"(\w+)\s*\(", stmt)
    return called.group(1) if called else None


def long_hits(text: str) -> list[tuple[int, str]]:
    hits = []
    second = set()
    for m in LONG.finditer(text):
        at = m.start()
        if at in second:
            continue
        follow = re.match(r"long\s+(long|double)\b", text[at:])
        if follow:
            if follow.group(1) == "long":
                second.add(at + follow.start(1))
            continue
        before = re.search(r"(?:(?:unsigned|signed)\s+)?$", text[:at])
        start = at - len(before.group(0))
        tail = re.match(r"long(?:\s+int\b)?", text[at:])
        hits.append((start, " ".join(text[start:at + tail.end()].split())))
    return hits


def raw_hits(text: str, enum_names: set[str]) -> list[Raw]:
    ctx = contexts(text)
    hits: list[Raw] = []
    for at, shown in long_hits(text):
        hits.append(Raw(at, "plain-long", f"`{shown}`", classify(text, ctx, at), statement(text, at)))
    for m in INCLUDE.finditer(text):
        if m.group(1) not in FREESTANDING:
            hits.append(Raw(m.start(), "host-include", f"`#include <{m.group(1)}>`", "include", ""))
    if enum_names:
        names = "|".join(map(re.escape, sorted(enum_names)))
        pattern = re.compile(r"\benum\s+(\w+)\b(?!\s*[{:])|\b(" + names + r")\b")
        for m in pattern.finditer(text):
            name = m.group(1) or m.group(2)
            if m.group(2) is None and name not in enum_names:
                continue
            stmt = statement(text, m.start())
            if re.search(r"\btypedef\s+enum\b", stmt) or "(" not in stmt:
                continue
            if re.match(r"\w+\s*\(", text[m.start(2) if m.group(2) else m.start(1):]):
                continue
            hits.append(Raw(m.start(), "enum-type", f"enumeration type `{name}`",
                            classify(text, ctx, m.start()), stmt))
    return hits


def top_statements(text: str) -> list[str]:
    """File-scope `;`-terminated statements: prototypes and typedefs."""
    ctx = contexts(text)
    out, begin = [], 0
    for k, ch in enumerate(text):
        if ctx[k] != "top" or ch == "}":
            begin = k + 1
        elif ch == ";":
            out.append(text[begin:k])
            begin = k + 1
    return out


LONG_BOUND = {"usize", "isize", "i64", "u64"}
ENUM_BOUND = {"u32"}
SIGNATURE = re.compile(r"\bextern\s+fn\s+(\w+)\s*\(")
INCLUDE_CONTRACT = re.compile(r"include_str!\(\s*\"([^\"]+\.(?:qv|qcore))\"\s*\)")

Signatures = dict[str, list[list[str]]]


def split_top(text: str, sep: str = ",") -> list[str]:
    parts, depth, begin = [], 0, 0
    for k, ch in enumerate(text):
        if ch in "(<[{":
            depth += 1
        elif ch in ")>]}":
            depth -= 1
        elif ch == sep and depth == 0:
            parts.append(text[begin:k])
            begin = k + 1
    parts.append(text[begin:])
    return parts


def signatures(text: str) -> Signatures:
    """`extern fn NAME(a: T, ...) -> R`: per name, the list of [R, T1, T2, ...]."""
    out: Signatures = {}
    for m in SIGNATURE.finditer(text):
        depth, k = 1, m.end()
        while k < len(text) and depth:
            depth += {"(": 1, ")": -1}.get(text[k], 0)
            k += 1
        params = text[m.end():k - 1]
        result = re.match(r"\s*->\s*([\w]+(?:<[^>]*>)?)", text[k:])
        types = [result.group(1) if result else "unit"]
        for param in split_top(params) if params.strip() else []:
            types.append(param.split(":", 1)[1].strip() if ":" in param else param.strip())
        out.setdefault(m.group(1), []).append(types)
    return out


def merge(into: Signatures, more: Signatures) -> Signatures:
    for name, sigs in more.items():
        into.setdefault(name, []).extend(sigs)
    return into


def contract_signatures(paths: list[str]) -> Signatures:
    out: Signatures = {}
    for path in walk(paths, (".qv", ".qcore")):
        with open(path, encoding="utf-8", errors="replace") as handle:
            merge(out, signatures(handle.read()))
    return out


def repository_contracts() -> Signatures:
    root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
    return contract_signatures([os.path.join(root, "providers"),
                                os.path.join(root, "crates", "qcore-c-gateway", "tests", "fixtures")])


def position(stmt: str, offset: int) -> int | None:
    """The C position `offset` stands at in prototype `stmt`: 0 the result,
    k the k-th parameter; None when it is not a direct position."""
    name = prototype_name(stmt)
    if name is None:
        return None
    pointer = re.search(r"\(\s*\*\s*" + re.escape(name) + r"\s*\)\s*\(", stmt)
    opening = pointer.end() - 1 if pointer else re.search(re.escape(name) + r"\s*\(", stmt).end() - 1
    if offset < opening:
        return 0
    depth, k = 0, opening
    while k < len(stmt):
        depth += {"(": 1, ")": -1}.get(stmt[k], 0)
        if depth == 0:
            break
        k += 1
    if offset > k:
        return None
    inner = stmt[opening + 1:k]
    rel, index = offset - opening - 1, 1
    for part in split_top(inner):
        if rel <= len(part):
            return index
        rel -= len(part) + 1
        index += 1
    return None


def bound_types(sigs: Signatures, name: str | None, pos: int | None) -> set[str]:
    if name is None or pos is None:
        return set()
    return {sig[pos] for sig in sigs.get(name, []) if pos < len(sig)}


def scan_file(path: str, repository: Signatures) -> list[Hit]:
    with open(path, encoding="utf-8", errors="replace") as handle:
        source = handle.read()
    frags = rust_fragments(source) if path.endswith(".rs") else [c_fragment(source)]
    texts = [blank_comments(f.text) for f in frags]
    joined = "\n".join(texts)
    local: Signatures = signatures(joined)
    for rel in INCLUDE_CONTRACT.findall(source):
        target = os.path.join(os.path.dirname(path), rel)
        if os.path.exists(target):
            with open(target, encoding="utf-8", errors="replace") as handle:
                merge(local, signatures(handle.read()))
    # A file that carries or includes its own contract text is judged against
    # it; otherwise against every repository contract of the name.
    sigs = local if local else repository
    enum_names = set(ENUM_TYPEDEF.findall(joined)) | set(ENUM_TAG.findall(joined))
    statements = [st for t in texts for st in top_statements(t)]
    prototypes = [st for st in statements if "(" in st and not re.match(r"\s*typedef\b", st)]
    contracted_prototypes = [st for st in prototypes if prototype_name(st) in sigs]
    declared = {prototype_name(st) for st in prototypes}
    declared |= set(re.findall(r"\b(\w+)\s*\([^;{}()]*(?:\([^;{}()]*\)[^;{}()]*)*\)\s*\{", joined))
    called = set(re.findall(r"\b(\w+)\s*\(", joined))
    from_headers = sorted((called & set(sigs)) - declared - KEYWORDS)
    lines = source.split("\n")
    # A Rust file that branches on the host target may expect the Windows
    # reading on purpose; say so, but keep counting (the branch may not cover
    # this fixture).
    gated = bool(WINDOWS_CFG.search(source)) if path.endswith(".rs") else False
    hits: list[Hit] = []
    for frag, text in zip(frags, texts):
        for raw in raw_hits(text, enum_names):
            line = frag.lines[raw.at]
            around = [line, frag.open_line - 1] if frag.open_line else [line]
            if any(0 < k <= len(lines) and ALLOW in lines[k - 1] for k in around):
                continue
            hazardous = LONG_BOUND if raw.hazard == "plain-long" else ENUM_BOUND
            binds = False
            if raw.hazard == "host-include":
                binds = bool(from_headers)
                why = ("contracted callees no fixture text declares: " + ", ".join(from_headers[:6])
                       if binds else "no contracted callee left undeclared")
            elif raw.klass == "declaration" and re.match(r"\s*typedef\b", raw.stmt):
                name = (prototype_name(raw.stmt) if "(" in raw.stmt
                        else (re.findall(r"(\w+)\s*$", raw.stmt) or [None])[0])
                found: set[str] = set()
                user = None
                for st in contracted_prototypes:
                    for m in re.finditer(r"\b" + re.escape(name or "") + r"\b", st) if name else []:
                        types = bound_types(sigs, prototype_name(st), position(st, m.start()))
                        if types:
                            found |= types
                            user = user or prototype_name(st)
                binds = bool(found & hazardous)
                why = (f"typedef `{name}` at a {'/'.join(sorted(found))} position of contracted `{user}`"
                       if found else f"typedef `{name}`, at no contracted position")
            elif raw.klass == "declaration":
                name = prototype_name(raw.stmt)
                begin = text.find(raw.stmt.strip(), max(0, raw.at - len(raw.stmt)))
                pos = position(raw.stmt.strip(), raw.at - begin) if begin >= 0 else None
                types = bound_types(sigs, name, pos)
                binds = bool(types & hazardous)
                where = "the result" if pos == 0 else f"parameter {pos}"
                why = (f"`{name}` {where}, contract {'/'.join(sorted(types))}" if types
                       else f"`{name}`, " + ("uncontracted" if name not in sigs else "no contract position"))
            else:
                why = raw.klass
            if gated:
                why += "; the file branches on a Windows/Unix cfg"
            hits.append(Hit(path, line, raw.hazard, raw.shown, why, binds))
    return hits


def walk(paths: list[str], suffixes: tuple[str, ...]) -> list[str]:
    files: list[str] = []
    for path in paths:
        if os.path.isdir(path):
            for root, dirs, names in os.walk(path):
                dirs[:] = sorted(d for d in dirs if d not in SKIP_DIRS)
                files.extend(os.path.join(root, f) for f in sorted(names) if f.endswith(suffixes))
        elif os.path.exists(path) and path.endswith(suffixes):
            files.append(path)
        elif not os.path.exists(path):
            files.append(path)
    return files


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("paths", nargs="*")
    parser.add_argument("--stdin", action="store_true", help="also read paths from standard input")
    parser.add_argument("--strict", action="store_true", help="count and print every hit, bound or not")
    parser.add_argument("--all", action="store_true", help="also print hits that are not counted")
    parser.add_argument("--contracts", action="append", default=[],
                        help="extra .qv/.qcore file or directory of contract signatures")
    parser.add_argument("--no-repository-contracts", action="store_true",
                        help="do not read the repository's provider and gateway-fixture contracts")
    args = parser.parse_args(argv)
    paths = list(args.paths)
    if args.stdin:
        paths.extend(p.strip() for p in sys.stdin if p.strip())
    if not paths:
        parser.error("no paths given")
    contracted: Signatures = {} if args.no_repository_contracts else repository_contracts()
    merge(contracted, contract_signatures(args.contracts))
    counted = found = 0
    status = 0
    for path in walk(paths, SUFFIXES):
        try:
            hits = scan_file(path, contracted)
        except OSError as error:
            print(f"{path}: unreadable: {error}", file=sys.stderr)
            status = 2
            continue
        for hit in sorted(hits, key=lambda h: (h.line, h.hazard)):
            counts = args.strict or hit.binds
            counted += counts
            found += 1
            if counts or args.all:
                mark = "" if counts else " [not counted]"
                print(f"{hit.path}:{hit.line}: [{hit.hazard}] {hit.text} ({hit.why}){mark}")
    print(f"llp64_fixture_scan: {counted} counted, {found} found", file=sys.stderr)
    if status:
        return status
    return 1 if counted else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
