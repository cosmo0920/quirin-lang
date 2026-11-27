#!/usr/bin/env python3
"""Census of C11 `_Generic` selections in the analysed (`-E -P`) units.

Usage: generic_census.py CAPTURED_PPP_DIR DIAGNOSTICS_JSON[.gz] OUT.json

CAPTURED_PPP_DIR holds the `clang -E -P` outputs a transparent Clang wrapper
saved during a private run of the pinned audit
(`docs/reports/w43-libc-result-alias/scripts/clang-tee.sh`). The analysed text is
the `-E -P` text, whose line numbers are the diagnostics' internal `line`.
Identical texts are read once.

Each capture is matched to the diagnostics unit whose projected functions it
defines (every `source_functions` name of the unit is defined at file scope in
the text, up to five the crude definition reader misses; the unit with the
most found definitions, if unique), so
that an occurrence can be joined with the refusals the BEFORE run gave at its
line in its enclosing function.

For every `_Generic` token the census records the unit, the preprocessed line,
the enclosing function (the file-scope definition whose body contains it, or
none), its shape (the glibc `__glibc_const_generic` expansion with its callee,
or other), its syntactic position, whether this card's selection-invariant
reader (re-implemented here in Python, independently of the Rust) admits it,
and the refusals the BEFORE diagnostics give at that line in that function.
"""

import collections
import gzip
import hashlib
import json
import pathlib
import re
import sys

TOKEN = re.compile(
    r'[A-Za-z_][A-Za-z0-9_]*|0[xX][0-9a-fA-F]+[uUlL]*|\d+(?:\.\d+)?[uUlLfF]*'
    r'|"(?:[^"\\\n]|\\.)*"|\'(?:[^\'\\\n]|\\.)*\''
    r'|->|\+\+|--|<<=|>>=|<<|>>|&&|\|\||[<>=!+\-*/%&|^]=|\.\.\.|\S'
)
IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
QUALIFIERS = {"const", "volatile", "restrict", "__restrict", "__restrict__"}


def tokens(text):
    out = []
    for number, line in enumerate(text.split("\n"), start=1):
        for token in TOKEN.findall(line):
            out.append((token, number))
    return out


def matching(toks, start, open_, close):
    depth = 0
    for index in range(start, len(toks)):
        if toks[index][0] == open_:
            depth += 1
        elif toks[index][0] == close:
            depth -= 1
            if depth == 0:
                return index
    return None


def split_top(seq, sep):
    parts, current, depth = [], [], 0
    for token in seq:
        if token in ("(", "[", "{"):
            depth += 1
        elif token in (")", "]", "}"):
            depth -= 1
        if token == sep and depth == 0:
            parts.append(current)
            current = []
        else:
            current.append(token)
    parts.append(current)
    return parts


def wraps(seq):
    """`( X )` whose parentheses enclose the whole sequence."""
    if len(seq) < 2 or seq[0] != "(" or seq[-1] != ")":
        return False
    depth = 0
    for index, token in enumerate(seq):
        if token == "(":
            depth += 1
        elif token == ")":
            depth -= 1
            if depth == 0 and index != len(seq) - 1:
                return False
    return True


def is_direct_call(seq):
    return len(seq) >= 3 and IDENT.match(seq[0]) and seq[1] == "(" and wraps(seq[1:])


def is_pointer_type_name(seq):
    if not seq or not IDENT.match(seq[0]):
        return False
    try:
        first_star = seq.index("*")
    except ValueError:
        return False
    if not all(IDENT.match(t) for t in seq[:first_star]):
        return False
    return all(t == "*" or t in QUALIFIERS for t in seq[first_star:])


def closed_operand(seq):
    return wraps(seq) or is_direct_call(seq) or (len(seq) == 1 and IDENT.match(seq[0]))


def strip(seq):
    casts = 0
    while True:
        if wraps(seq):
            seq = seq[1:-1]
            continue
        if seq and seq[0] == "(":
            depth, close = 0, None
            for index, token in enumerate(seq):
                if token == "(":
                    depth += 1
                elif token == ")":
                    depth -= 1
                    if depth == 0:
                        close = index
                        break
            if close is not None and is_pointer_type_name(seq[1:close]) and closed_operand(seq[close + 1:]):
                seq = seq[close + 1:]
                casts += 1
                continue
        return seq, casts


def reader(texts, at, pointer_result):
    """Python re-implementation of the selection-invariant reader.

    Returns (verdict, detail): verdict is `admitted` or a refusal ground."""
    if at + 1 >= len(texts) or texts[at + 1] != "(":
        return "no-parenthesis", None
    depth, close = 0, None
    for index in range(at + 1, len(texts)):
        if texts[index] == "(":
            depth += 1
        elif texts[index] == ")":
            depth -= 1
            if depth == 0:
                close = index
                break
    if close is None:
        return "unbalanced", None
    parts = split_top(texts[at + 2:close], ",")
    if len(parts) < 2 or not parts[0]:
        return "malformed", None
    cores, defaults, any_cast = [], 0, False
    for association in parts[1:]:
        halves = split_top(association, ":")
        if len(halves) < 2 or not halves[0]:
            return "malformed-association", None
        type_name = halves[0]
        expression = association[len(type_name) + 1:]
        if not expression:
            return "malformed-association", None
        if type_name == ["default"]:
            defaults += 1
        core, casts = strip(expression)
        any_cast = any_cast or casts > 0
        cores.append(core)
    if defaults > 1:
        return "two-defaults", None
    if any(core != cores[0] for core in cores):
        return "not-selection-invariant", None
    core = cores[0]
    if any_cast:
        if not is_direct_call(core):
            return "cast-operand-not-a-call", None
        if not pointer_result(core[0]):
            return "callee-result-not-a-declared-pointer", core[0]
    return "admitted", " ".join(core)


def definitions(toks):
    """(name, body_open, body_close) of every file-scope function definition."""
    out, depth, index = [], 0, 0
    while index < len(toks):
        token = toks[index][0]
        if token == "{":
            if depth == 0:
                # Walk back over `) ... {`: find the parameter list's `)`.
                back = index - 1
                while back >= 0 and toks[back][0] != ")" and toks[back][0] not in (";", "}", "="):
                    back -= 1
                name = None
                if back >= 0 and toks[back][0] == ")":
                    d, open_ = 0, None
                    for b in range(back, -1, -1):
                        if toks[b][0] == ")":
                            d += 1
                        elif toks[b][0] == "(":
                            d -= 1
                            if d == 0:
                                open_ = b
                                break
                    if open_ and IDENT.match(toks[open_ - 1][0]):
                        name = toks[open_ - 1][0]
                close = matching(toks, index, "{", "}")
                if close is None:
                    break
                if name:
                    out.append((name, index, close))
                index = close + 1
                continue
            depth += 1
        elif token == "}":
            depth -= 1
        index += 1
    return out


def declared_pointer_results(texts):
    """Names declared at a call-shaped position `T * NAME (` (a crude reader:
    a `*` immediately before the name, with a type word before the stars)."""
    names = set()
    for index in range(2, len(texts) - 1):
        if texts[index + 1] == "(" and IDENT.match(texts[index]) and texts[index - 1] == "*":
            back = index - 1
            while back >= 0 and texts[back] == "*":
                back -= 1
            if back >= 0 and IDENT.match(texts[back]) and texts[back] not in ("return",):
                names.add(texts[index])
    return names


def position(texts, at):
    previous = texts[at - 1] if at else ""
    if previous == "return":
        return "return"
    if previous == "=":
        # declaration initializer or assignment
        back = at - 2
        while back >= 0 and texts[back] not in (";", "{", "}"):
            back -= 1
        head = texts[back + 1:at - 1]
        if len(head) >= 2 and IDENT.match(head[0]) and any(t in ("char", "void", "int", "const", "wchar_t") or t == "*" for t in head[:-1]):
            return "initializer"
        return "assignment"
    if previous in ("(", ","):
        # inside a call argument list or a condition?
        depth, back = 0, at - 1
        while back >= 0:
            if texts[back] == ")":
                depth += 1
            elif texts[back] == "(":
                if depth == 0:
                    break
                depth -= 1
            back -= 1
        opener = texts[back - 1] if back > 0 else ""
        if opener in ("if", "while", "switch"):
            return "condition"
        if opener == "_Generic":
            return "nested-in-generic"
        if IDENT.match(opener):
            return "argument"
        return "parenthesised"
    if previous == ":":
        return "nested-in-generic"
    if previous in ("!", "&&", "||", "==", "!="):
        return "condition-operand"
    return f"after:{previous}"


def main():
    captured = pathlib.Path(sys.argv[1])
    raw = pathlib.Path(sys.argv[2]).read_bytes()
    if sys.argv[2].endswith(".gz"):
        raw = gzip.decompress(raw)
    diagnostics = json.loads(raw)
    by_unit = {}
    for unit in diagnostics["units"]:
        lines = collections.defaultdict(list)
        for function in unit["functions"]:
            for leaf in function["diagnostics"]:
                lines[(function["function"], leaf["line"])].append(leaf["description"])
        by_unit[unit["file"]] = {
            "library": unit.get("library"),
            "functions": set(unit.get("source_functions") or [f["function"] for f in unit["functions"]]),
            "leaves": lines,
            "generic_lines": {leaf["line"] for f in unit["functions"] for leaf in f["diagnostics"] if "_Generic" in leaf["description"]},
        }
    seen = set()
    rows = []
    texts_with = 0
    unmatched = []
    for path in sorted(captured.glob("*.i")):
        text = path.read_text(errors="replace")
        digest = hashlib.sha256(text.encode()).hexdigest()
        if digest in seen:
            continue
        seen.add(digest)
        if "_Generic" not in text:
            continue
        texts_with += 1
        toks = tokens(text)
        texts = [t for t, _ in toks]
        defs = definitions(toks)
        defined = {name for name, _, _ in defs}
        lines_text = text.split("\n")
        scored = []
        for name, unit in by_unit.items():
            if not unit["functions"]:
                continue
            # The text also defines header-inline bodies, and the unit's
            # source inventory can name definitions this crude reader misses
            # (attributes, `#ifdef _WIN32` arms). A unit matches when at most
            # five of its projected functions are not found; the unit with the
            # most found functions is the one, if it is unique.
            missing = len(unit["functions"] - defined)
            if missing > 5:
                continue
            scored.append((len(unit["functions"] & defined), name))
        scored.sort(reverse=True)
        candidates = []
        if scored and scored[0][0] > 0 and (len(scored) == 1 or scored[1][0] < scored[0][0]):
            candidates = [scored[0][1]]
        pointer_names = declared_pointer_results(texts)
        occurrences = [i for i, t in enumerate(texts) if t == "_Generic"]
        if not candidates:
            unmatched.append({"capture": path.name, "occurrences": len(occurrences)})
        spans = {}
        for at in occurrences:
            close = matching(toks, at + 1, "(", ")")
            if close is not None:
                spans[at] = close
        for at in occurrences:
            line = toks[at][1]
            # Where this selection sits relative to an enclosing one: in its
            # controlling expression (dropped whole) or in an association.
            enclosing_selections = [outer for outer, close in spans.items() if outer < at < close]
            nesting = "top-level"
            if enclosing_selections:
                outer = max(enclosing_selections)
                first_comma, depth = None, 0
                for k in range(outer + 2, spans[outer]):
                    if texts[k] in ("(", "[", "{"):
                        depth += 1
                    elif texts[k] in (")", "]", "}"):
                        depth -= 1
                    elif texts[k] == "," and depth == 0:
                        first_comma = k
                        break
                nesting = "in-control" if first_comma is None or at < first_comma else "in-association"
            enclosing = None
            for name, open_, close in defs:
                if open_ < at < close:
                    enclosing = name
                    break
            verdict, detail = reader(texts, at, lambda n: n in pointer_names)
            end = matching(toks, at + 1, "(", ")")
            spelled = " ".join(texts[at:end + 1]) if end else None
            glibc = None
            match = re.match(r"^_Generic \( 0 \? \( (.*) \) : \( void \* \) 1 , const (void|char|wchar_t) \* : \( const (void|char|wchar_t) \* \) \( ([A-Za-z_]\w*) \( .*\) \) , default : ([A-Za-z_]\w*) \( .*\) \)$", spelled or "")
            if match and match.group(4) == match.group(5):
                glibc = match.group(4)
            refusals = {}
            leaf_line = None
            for unit_name in candidates:
                if enclosing is not None:
                    leaves = by_unit[unit_name]["leaves"]
                    # The statement's leaf is at its first line: the nearest
                    # leaf line at or above the selection, within 4 lines.
                    for back in range(0, 5):
                        if (enclosing, line - back) in leaves:
                            leaf_line = line - back
                            break
                    refusals[unit_name] = leaves.get((enclosing, leaf_line), []) if leaf_line else []
            rows.append({
                "capture": path.name,
                "units": candidates,
                "line": line,
                "leaf_line": leaf_line,
                "function": enclosing,
                "shape": f"glibc-const-generic:{glibc}" if glibc else "other",
                "nesting": nesting,
                "position": position(texts, at),
                "reader": verdict,
                "resolved": detail,
                "refusals_at_line": refusals,
                "spelling": spelled if not glibc else None,
            })
    summary = {
        "captures_read": len(seen),
        "captures_with_generic": texts_with,
        "occurrences": len(rows),
        "by_shape": collections.Counter(r["shape"] for r in rows),
        "by_position": collections.Counter(r["position"] for r in rows),
        "by_reader": collections.Counter(r["reader"] for r in rows),
        "by_nesting": collections.Counter(r["nesting"] for r in rows),
        "top_level_by_position": collections.Counter(r["position"] for r in rows if r["nesting"] == "top-level"),
        "top_level_by_shape": collections.Counter(r["shape"] for r in rows if r["nesting"] == "top-level"),
        "in_function_bodies": sum(1 for r in rows if r["function"]),
        "unmatched_captures": unmatched,
    }
    out = {"summary": summary, "rows": rows}
    pathlib.Path(sys.argv[3]).write_text(json.dumps(out, indent=1, default=list) + "\n")
    print(json.dumps(summary, indent=1, default=list))


if __name__ == "__main__":
    main()
