#!/usr/bin/env python3
"""Card CHAINED-ASSIGNMENT-INNER-DESTINATION: census of chained assignments.

Lexical, read-only. Scans every C source and header of a source tree for an
assignment whose right operand holds another assignment (``a = b = E``, longer
chains, compound links such as ``x = y += k``, the parenthesised
``a = (b = E)``) and, separately, for an assignment nested inside another
operand of an assignment's right-hand side (``a = f(b = E)``). Each site is
then joined to the function rows of one or more ``resource-diagnostics.json``
reports by function name and origin ``file:line``.

    census.py --tree ROOT [--tree ROOT ...] --contracts DIR [--contracts DIR ...]
              --report NAME=PATH [--report NAME=PATH ...] --out census.json

The joined reading is what each report shows at the chain's line (the
diagnostics of the enclosing function whose origin is that line); the route's
own lowering is stated per shape class in the README and pinned by the card's
fixtures, not inferred from the report alone.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import Counter, defaultdict

ASSIGN = {"=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>="}
PUNCT = sorted(
    [
        "<<=", ">>=", "...", "->", "++", "--", "<<", ">>", "<=", ">=", "==", "!=",
        "&&", "||", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "##",
    ],
    key=len,
    reverse=True,
)
KEYWORDS = {
    "if", "while", "for", "switch", "return", "sizeof", "do", "else", "case",
    "default", "goto", "break", "continue", "_Alignof", "__typeof__", "typeof",
    "__attribute__", "__asm__", "asm",
}
TYPE_WORDS = {
    "void", "char", "short", "int", "long", "float", "double", "signed",
    "unsigned", "const", "volatile", "static", "struct", "union", "enum",
    "register", "auto", "extern", "_Bool", "bool", "restrict", "__restrict",
    "inline", "__inline", "__inline__",
}
IDENT = re.compile(r"[A-Za-z_]\w*")


def tokenize(text: str):
    """Tokens of C text with their 1-based lines; comments, strings and
    character literals become one placeholder token, directives are skipped
    but their bodies are returned separately as (line, tokens, name)."""
    tokens: list[tuple[str, int]] = []
    macros: list[tuple[int, str, list[tuple[str, int]]]] = []
    i, line, n = 0, 1, len(text)
    at_line_start = True
    while i < n:
        c = text[i]
        if c == "\n":
            line += 1
            i += 1
            at_line_start = True
            continue
        if c in " \t\r\f\v":
            i += 1
            continue
        if c == "\\" and i + 1 < n and text[i + 1] == "\n":
            i += 2
            line += 1
            continue
        if text.startswith("//", i):
            while i < n and text[i] != "\n":
                i += 1
            continue
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            end = n if end < 0 else end + 2
            line += text.count("\n", i, end)
            i = end
            continue
        if c == "#" and at_line_start:
            start_line = line
            j = i
            body = []
            while j < n:
                if text[j] == "\\" and j + 1 < n and text[j + 1] == "\n":
                    body.append(" ")
                    j += 2
                    line += 1
                    continue
                if text[j] == "\n":
                    break
                body.append(text[j])
                j += 1
            directive = "".join(body)
            m = re.match(r"#\s*define\s+([A-Za-z_]\w*)(\([^)]*\))?(.*)", directive, re.S)
            if m:
                inner, _ = tokenize(m.group(3))
                macros.append((start_line, m.group(1), [(t, start_line) for t, _ in inner]))
            i = j
            continue
        at_line_start = False
        if c in "\"'":
            j = i + 1
            while j < n and text[j] != c:
                if text[j] == "\\":
                    j += 1
                if j < n and text[j] == "\n":
                    line += 1
                j += 1
            tokens.append(("<str>" if c == '"' else "<chr>", line))
            i = j + 1
            continue
        m = IDENT.match(text, i)
        if m:
            tokens.append((m.group(0), line))
            i = m.end()
            continue
        if c.isdigit() or (c == "." and i + 1 < n and text[i + 1].isdigit()):
            j = i + 1
            while j < n and (text[j].isalnum() or text[j] in "._" or (text[j] in "+-" and text[j - 1] in "eEpP")):
                j += 1
            tokens.append(("<num>", line))
            i = j
            continue
        for p in PUNCT:
            if text.startswith(p, i):
                tokens.append((p, line))
                i += len(p)
                break
        else:
            tokens.append((c, line))
            i += 1
    return tokens, macros


def functions(tokens):
    """(name, parameter names, body start, body end) of every file-scope
    function definition."""
    depth = 0
    out = []
    i = 0
    while i < len(tokens):
        t = tokens[i][0]
        if t == "{":
            if depth == 0:
                # `) {` or `) __attribute__((...)) {`
                j = i - 1
                if j >= 0 and tokens[j][0] == ")":
                    k, d = j, 0
                    while k >= 0:
                        if tokens[k][0] == ")":
                            d += 1
                        elif tokens[k][0] == "(":
                            d -= 1
                            if d == 0:
                                break
                        k -= 1
                    name = tokens[k - 1][0] if k >= 1 else ""
                    if IDENT.fullmatch(name) and name not in KEYWORDS:
                        params = [
                            tokens[p][0]
                            for p in range(k + 1, j)
                            if IDENT.fullmatch(tokens[p][0])
                            and tokens[p + 1][0] in {",", ")", "["}
                        ]
                        end, d2 = i, 0
                        while end < len(tokens):
                            if tokens[end][0] == "{":
                                d2 += 1
                            elif tokens[end][0] == "}":
                                d2 -= 1
                                if d2 == 0:
                                    break
                            end += 1
                        out.append((name, params, i, end))
                        i = end + 1
                        continue
            depth += 1
        elif t == "}":
            depth = max(0, depth - 1)
        i += 1
    return out


def nesting(tokens, lo, hi):
    """Paren/bracket depth before each token in [lo, hi)."""
    depths, d = {}, 0
    for k in range(lo, hi):
        t = tokens[k][0]
        if t in ")]":
            d -= 1
        depths[k] = d
        if t in "([":
            d += 1
    return depths


def right_extent(tokens, op, hi, depth):
    """Exclusive end of the right operand of the assignment at `op`."""
    rel, question = 0, 0
    k = op + 1
    while k < hi:
        t = tokens[k][0]
        if t in {";", "{", "}"}:
            break
        if t in "([":
            rel += 1
        elif t in ")]":
            if rel == 0:
                break
            rel -= 1
        elif rel == 0 and t == ",":
            break
        elif rel == 0 and t == "?":
            question += 1
        elif rel == 0 and t == ":":
            if question == 0:
                break
            question -= 1
        k += 1
    return k


LEFT_STOP = {";", "{", "}", ",", "?", ":", "&&", "||", "==", "!=", "<", ">", "<=", ">=", "return", "!"} | ASSIGN


def left_extent(tokens, op, lo):
    """Inclusive start of the left operand of the assignment at `op`."""
    rel = 0
    k = op - 1
    while k >= lo:
        t = tokens[k][0]
        if t == ")" and rel == 0:
            # A control header's `)` ends the left operand: a cast is never
            # an lvalue, so `if (c) d = ...` starts at `d`.
            m, d = k, 0
            while m >= lo:
                if tokens[m][0] == ")":
                    d += 1
                elif tokens[m][0] == "(":
                    d -= 1
                    if d == 0:
                        break
                m -= 1
            if m - 1 >= lo and tokens[m - 1][0] in {"if", "while", "for", "switch"}:
                break
            rel += 1
        elif t in ")]":
            rel += 1
        elif t in "([":
            if rel == 0:
                break
            rel -= 1
        elif rel == 0 and t in LEFT_STOP | {"else", "do"}:
            break
        k -= 1
    return k + 1


def strip_parens(seq):
    while len(seq) >= 2 and seq[0] == "(" and seq[-1] == ")":
        d = 0
        for idx, t in enumerate(seq):
            if t == "(":
                d += 1
            elif t == ")":
                d -= 1
                if d == 0 and idx != len(seq) - 1:
                    return seq
        seq = seq[1:-1]
    return seq


def declared_locals(tokens, lo, hi, typedefs):
    """Names this body declares (approximate: a name after a type word,
    typedef name or `*` that is followed by `=`, `;`, `,` or `[`)."""
    names = set()
    start = lo + 1
    k = lo + 1
    while k < hi:
        t = tokens[k][0]
        if t in {";", "{", "}"}:
            statement = [tokens[m][0] for m in range(start, k)]
            # strip a `for (` header opening
            if statement[:2] == ["for", "("]:
                statement = statement[2:]
            first = statement[0] if statement else ""
            second = statement[1] if len(statement) > 1 else ""
            if first in TYPE_WORDS or first in typedefs or (
                IDENT.fullmatch(first) and first not in KEYWORDS
                and (IDENT.fullmatch(second) or second == "*")
            ):
                depth = 0
                for m, x in enumerate(statement[1:], 1):
                    if x in "([":
                        depth += 1
                    elif x in ")]":
                        depth -= 1
                    elif depth == 0 and IDENT.fullmatch(x) and x not in TYPE_WORDS:
                        nxt = statement[m + 1] if m + 1 < len(statement) else ";"
                        prev = statement[m - 1]
                        if nxt in {"=", ",", ";", "["} and (IDENT.fullmatch(prev) or prev in {"*", ","}):
                            names.add(x)
                if not any(x == "=" for x in statement):
                    pass
            start = k + 1
        k += 1
    return names


def destination(seq, params, locals_):
    """(kind, spelling, declared) of a left operand."""
    declared = False
    s = list(seq)
    # A declaration prefix: specifiers and declarator stars before the name.
    if len(s) >= 2 and all(IDENT.fullmatch(x) or x == "*" for x in s) and IDENT.fullmatch(s[-1]):
        declared = True
        s = [s[-1]]
    s = strip_parens(s)
    text = " ".join(s)
    if len(s) == 1 and IDENT.fullmatch(s[0]):
        name = s[0]
        if declared or name in locals_:
            kind = "local"
        elif name in params:
            kind = "formal"
        else:
            kind = "file-scope or other name"
        return kind, name, declared
    if s and s[0] == "*":
        return "deref", text, declared
    if "->" in s or "." in s:
        return "member", text, declared
    if "[" in s:
        return "subscript", text, declared
    return "other", text, declared


def value_kind(seq, results):
    s = strip_parens(list(seq))
    # one leading cast `( T * )`
    while len(s) > 2 and s[0] == "(":
        d, close = 0, None
        for idx, t in enumerate(s):
            if t == "(":
                d += 1
            elif t == ")":
                d -= 1
                if d == 0:
                    close = idx
                    break
        inner = s[1:close] if close else []
        if close and inner and all(IDENT.fullmatch(x) or x == "*" for x in inner) and close < len(s) - 1:
            s = strip_parens(s[close + 1 :])
        else:
            break
    if not s:
        return "empty", ""
    if len(s) >= 3 and IDENT.fullmatch(s[0]) and s[1] == "(" and s[-1] == ")":
        d = 0
        whole = True
        for idx, t in enumerate(s[1:], 1):
            if t == "(":
                d += 1
            elif t == ")":
                d -= 1
                if d == 0 and idx != len(s) - 1:
                    whole = False
                    break
        if whole and s[0] not in KEYWORDS:
            mode = results.get(s[0])
            if mode in {"owned", "reallocated"}:
                return "acquisition", s[0]
            if mode == "derived":
                return "derived call", s[0]
            if mode is None:
                return "uncontracted call", s[0]
            return "call without custody result", s[0]
    if len(s) == 1:
        t = s[0]
        if t in {"NULL", "<num>", "<chr>", "true", "false"}:
            return "scalar", t
        if t == "<str>":
            return "string literal", t
        if IDENT.fullmatch(t):
            return "name", t
    if s[0] == "&":
        return "address", " ".join(s)
    if any(IDENT.fullmatch(x) and i + 1 < len(s) and s[i + 1] == "(" and x not in KEYWORDS for i, x in enumerate(s)):
        return "expression with a call", " ".join(s)
    return "expression", " ".join(s)


def context_of(tokens, start, lo):
    k = start - 1
    rel = 0
    while k >= lo:
        t = tokens[k][0]
        if t in ")]":
            rel += 1
        elif t in "([":
            if rel == 0:
                head = tokens[k - 1][0] if k - 1 >= lo else ""
                if head in {"if", "while", "switch"}:
                    return "condition"
                if head == "for":
                    return "for clause"
                return "operand"
            rel -= 1
        elif rel == 0 and t in {";", "{", "}"}:
            return "statement"
        elif rel == 0 and t == "return":
            return "return"
        k -= 1
    return "statement"


def scan_body(tokens, lo, hi, params, typedefs, results, emit):
    locals_ = declared_locals(tokens, lo, hi, typedefs)
    depth = nesting(tokens, lo, hi)
    ops = [k for k in range(lo, hi) if tokens[k][0] in ASSIGN]
    inner_of = set()
    for op in ops:
        end = right_extent(tokens, op, hi, depth[op])
        for other in ops:
            if op < other < end:
                inner_of.add(other)
    for op in ops:
        if op in inner_of:
            continue
        end = right_extent(tokens, op, hi, depth[op])
        inner = [o for o in ops if op < o < end]
        if not inner:
            continue
        # Walk the chain: each link's right operand is (parenthesised)
        # exactly the next assignment.
        links = [op]
        shape = "chain"
        cursor = op
        while True:
            cend = right_extent(tokens, cursor, hi, depth[cursor])
            rhs = [tokens[k][0] for k in range(cursor + 1, cend)]
            stripped = strip_parens(rhs)
            offset = cursor + 1 + rhs.index(stripped[0]) if stripped else cursor + 1
            # the first assignment at the stripped right operand's own level
            nxt = None
            for o in ops:
                if offset <= o < cend and depth[o] == depth[offset] and left_extent(tokens, o, offset) == offset:
                    nxt = o
                    break
            if nxt is None:
                remaining = [o for o in ops if cursor < o < cend]
                if remaining:
                    shape = "nested in an operand" if len(links) == 1 else "chain with a nested operand write"
                break
            if stripped is not rhs and len(stripped) != len(rhs):
                shape = "parenthesised chain" if shape == "chain" else shape
            links.append(nxt)
            cursor = nxt
        start = left_extent(tokens, op, lo)
        dests = []
        prev = start
        for link in links:
            lstart = left_extent(tokens, link, lo if link == op else prev)
            seq = [tokens[k][0] for k in range(lstart, link)]
            if link == op:
                seq = [tokens[k][0] for k in range(start, link)]
            kind, spelling, declared = destination(seq, params, locals_)
            dests.append({"kind": kind, "place": spelling, "declared": declared, "operator": tokens[link][0]})
            prev = link + 1
        last = links[-1]
        vend = right_extent(tokens, last, hi, depth[last])
        value = [tokens[k][0] for k in range(last + 1, vend)]
        vkind, vtext = value_kind(value, results)
        whole = [tokens[k][0] for k in range(start, end)]
        emit(
            {
                "line": tokens[op][1],
                "end_line": tokens[end - 1][1] if end > start else tokens[op][1],
                "shape": shape,
                "context": context_of(tokens, start, lo),
                "destinations": dests,
                "value": {"kind": vkind, "text": vtext},
                "text": " ".join(whole),
            }
        )


def contract_results(dirs):
    """Result mode of every `extern fn` in the contract modules under `dirs`."""
    blocks = {}
    for root in dirs:
        for base, _, files in os.walk(root):
            for f in files:
                if not f.endswith((".qv", ".qcore")):
                    continue
                text = open(os.path.join(base, f), encoding="utf-8", errors="replace").read()
                for m in re.finditer(r"extern fn\s+([A-Za-z_]\w*)\s*\((.*?)\)(.*?);\s*(?=extern fn|\Z|//|module)", text, re.S):
                    blocks.setdefault(m.group(1), []).append(m.group(3))
    results = {}

    def mode(name, seen=()):
        if name in results:
            return results[name]
        if name in seen:
            return "unknown"
        found = "none"
        for body in blocks.get(name, []):
            if "resource_return_owned" in body:
                found = "owned"
            elif "resource_return_reallocated" in body:
                found = "reallocated"
            elif "resource_return_derived" in body or "resource_return_borrowed" in body:
                found = "derived"
            else:
                m = re.search(r"contract\s+inherit\s+(?:\w+::)?([A-Za-z_]\w*)", body)
                if m:
                    found = mode(m.group(1), seen + (name,))
            if found != "none":
                break
        results[name] = found
        return found

    for name in list(blocks):
        mode(name)
    return results


def typedef_names(text):
    return set(re.findall(r"\btypedef\b[^;]*?\b([A-Za-z_]\w*)\s*;", text))


def scan_tree(root, results):
    sites = []
    for base, dirs, files in os.walk(root):
        # The 046accf6 checkout nests a second copy of itself under its own
        # name; no configuration compiles it.
        dirs[:] = sorted(
            d for d in dirs
            if d not in {".git", "build"} and d != os.path.basename(root.rstrip("/"))
        )
        for f in sorted(files):
            if not f.endswith((".c", ".h")):
                continue
            path = os.path.join(base, f)
            rel = os.path.relpath(path, root)
            try:
                text = open(path, encoding="utf-8", errors="replace").read()
            except OSError:
                continue
            if "=" not in text:
                continue
            tokens, macros = tokenize(text)
            typedefs = typedef_names(text)
            for name, params, lo, hi in functions(tokens):
                scan_body(
                    tokens, lo, min(hi + 1, len(tokens)), set(params), typedefs, results,
                    lambda site, name=name: sites.append({"file": rel, "function": name, **site}),
                )
            for line, name, body in macros:
                if not body:
                    continue
                body = [("{", line)] + body + [("}", line)]
                scan_body(
                    body, 0, len(body), set(), typedefs, results,
                    lambda site, name=name: sites.append({"file": rel, "function": f"#define {name}", **site}),
                )
    return sites


def macro_uses(root, unit_files, macro_sites):
    """Uses, inside a function body of a unit's own file, of a macro whose
    definition holds a chain: the chain the preprocessor expands there."""
    chains = defaultdict(list)
    for site in macro_sites:
        chains[site["function"].removeprefix("#define ")].append(site)
    uses = []
    for rel in sorted(unit_files):
        path = os.path.join(root, rel)
        if not os.path.isfile(path):
            continue
        text = open(path, encoding="utf-8", errors="replace").read()
        tokens, _ = tokenize(text)
        for name, _params, lo, hi in functions(tokens):
            for k in range(lo, min(hi + 1, len(tokens))):
                macro = tokens[k][0]
                if macro in chains:
                    for site in chains[macro]:
                        uses.append(
                            {
                                **site,
                                "file": rel,
                                "function": name,
                                "line": tokens[k][1],
                                "end_line": tokens[k][1],
                                "shape": f"{site['shape']} (expansion of {site['function']}, {site['file']}:{site['line']})",
                            }
                        )
    return uses


class Rows(defaultdict):
    """Function rows by name, with each unit file's not-projected names."""

    def __init__(self, *args):
        super().__init__(*args)
        self.units: dict[str, set[str]] = {}


def load_report(path):
    with open(path, encoding="utf-8") as handle:
        data = json.load(handle)
    rows = Rows(list)
    for unit in data["units"]:
        rows.units.setdefault(unit.get("file"), set()).update(unit.get("not_projected") or [])
        for function in unit.get("functions", []):
            rows[function["function"]].append(
                {
                    "unit": unit.get("file"),
                    "library": unit.get("library"),
                    "unit_status": unit.get("status"),
                    "status": function.get("status"),
                    "diagnostics": [
                        {
                            "category": d.get("category"),
                            "description": d.get("description"),
                            "origin": d.get("origin"),
                        }
                        for d in function.get("diagnostics", [])
                    ],
                }
            )
    return rows


def origin_matches(origin, site, tree_prefixes):
    if not origin:
        return False
    file_, _, line = origin.rpartition(":")
    if not line.isdigit():
        return False
    if not (site["line"] <= int(line) <= site["end_line"]):
        return False
    return file_ == site["file"] or file_.endswith("/" + site["file"]) or site["file"].endswith("/" + file_)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--tree", action="append", required=True, help="NAME=ROOT")
    parser.add_argument("--contracts", action="append", default=[])
    parser.add_argument("--report", action="append", default=[], help="NAME=TREE=PATH")
    parser.add_argument("--out", required=True)
    parser.add_argument("--analysed-out", help="the compact record of analysed sites")
    args = parser.parse_args()
    results = contract_results(args.contracts)
    trees = dict(item.split("=", 1) for item in args.tree)
    sites = {name: scan_tree(root, results) for name, root in trees.items()}
    joined = {}
    unjoined = {}
    for item in args.report:
        name, tree, path = item.split("=", 2)
        rows = load_report(path)
        root = trees[tree]
        all_c = []
        for base, dirs, files in os.walk(root):
            dirs[:] = [d for d in dirs if d not in {".git", "build"} and d != os.path.basename(root.rstrip("/"))]
            all_c.extend(os.path.relpath(os.path.join(base, f), root) for f in files if f.endswith(".c"))
        unit_files = {
            p for p in all_c for u in rows.units if u and (p == u or p.endswith("/" + u))
        }
        expanded = macro_uses(
            root, unit_files, [s for s in sites[tree] if s["function"].startswith("#define ")]
        )
        sites[tree] = sites[tree] + [s for s in expanded if s not in sites[tree]]
        out = []
        for site in sites[tree]:
            for row in rows.get(site["function"], []):
                here = [d for d in row["diagnostics"] if origin_matches(d["origin"], site, ())]
                in_file = any(
                    d["origin"] and (d["origin"].rpartition(":")[0].endswith(site["file"]) or site["file"].endswith(d["origin"].rpartition(":")[0]))
                    for d in row["diagnostics"]
                )
                unit = row["unit"] or ""
                same_unit = unit == site["file"] or unit.endswith("/" + site["file"]) or site["file"].endswith("/" + unit)
                # A same-named function of another file is a different
                # function unless the row shows evidence in the site's file;
                # a header's function is any unit's row of that name with no
                # evidence in another file.
                foreign = any(
                    d["origin"] and not (d["origin"].rpartition(":")[0].endswith(site["file"]) or site["file"].endswith(d["origin"].rpartition(":")[0]))
                    for d in row["diagnostics"]
                )
                header = site["file"].endswith(".h") and not foreign
                if not (same_unit or in_file or header):
                    continue
                # A leak's origin is the exit; its acquisition is named in
                # the description.
                for d in row["diagnostics"]:
                    m = re.search(r"acquired at (\S+):(\d+)", d["description"] or "")
                    if m and d not in here and site["line"] <= int(m.group(2)) <= site["end_line"] and (
                        m.group(1).endswith(site["file"]) or site["file"].endswith(m.group(1))
                    ):
                        here.append(d)
                out.append(
                    {
                        "file": site["file"],
                        "function": site["function"],
                        "line": site["line"],
                        "unit": row["unit"],
                        "status": row["status"],
                        "unit_status": row["unit_status"],
                        "same_file_evidence": in_file,
                        "at_line": here,
                    }
                )
        joined[name] = out
        # Sites in a unit's own file whose function is no row of the report
        # (not projected, or build-excluded).
        units = rows.units
        seen = {(r["file"], r["function"], r["line"]) for r in out}
        unjoined[name] = [
            {"file": s["file"], "function": s["function"], "line": s["line"],
             "not_projected": any(s["function"] in units[u] for u in units if u == s["file"] or s["file"].endswith("/" + u))}
            for s in sites[tree]
            if (s["file"], s["function"], s["line"]) not in seen
            and any(u == s["file"] or s["file"].endswith("/" + u) for u in units)
        ]
    summary = {
        name: {
            "sites": len(s),
            "by_shape": Counter(x["shape"] for x in s),
            "by_value": Counter(x["value"]["kind"] for x in s if "chain" in x["shape"]),
            "by_destinations": Counter(
                " / ".join(d["kind"] for d in x["destinations"]) for x in s if "chain" in x["shape"]
            ),
        }
        for name, s in sites.items()
    }
    json.dump(
        {
            "schema": "chained-assignment-census/1",
            "trees": trees,
            "summary": summary,
            "sites": sites,
            "joined": joined,
            "unjoined": unjoined,
        },
        open(args.out, "w"),
        indent=1,
        sort_keys=True,
    )
    if args.analysed_out:
        # The committed record: every site inside a function some report
        # analysed, with its lexical reading and the report's evidence at
        # its line, plus the whole-tree counts.
        analysed = {}
        for name, rows in joined.items():
            by_key = {}
            for row in rows:
                site = next(
                    s for s in sites[name]
                    if (s["file"], s["function"], s["line"]) == (row["file"], row["function"], row["line"])
                )
                key = (row["file"], row["function"], row["line"], site["shape"])
                entry = by_key.setdefault(
                    key,
                    {
                        "file": row["file"],
                        "function": row["function"],
                        "line": row["line"],
                        "shape": site["shape"],
                        "context": site["context"],
                        "destinations": site["destinations"],
                        "value": site["value"],
                        "text": site["text"],
                        "rows": [],
                    },
                )
                entry["rows"].append(
                    {"unit": row["unit"], "status": row["status"], "at_line": row["at_line"]}
                )
            analysed[name] = sorted(by_key.values(), key=lambda e: (e["file"], e["line"], e["function"]))
        json.dump(
            {
                "schema": "chained-assignment-census-analysed/1",
                "trees": trees,
                "tree_counts": summary,
                "analysed": analysed,
                "unjoined": unjoined,
            },
            open(args.analysed_out, "w"),
            indent=1,
            sort_keys=True,
        )
    json.dump(summary, sys.stdout, indent=1, sort_keys=True)


if __name__ == "__main__":
    main()
