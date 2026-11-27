#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card FB-CHUNK-LIFECYCLE: a source-level census of rule 90's candidates.

Over the raw (unpreprocessed) C sources under ROOT, list each `static`
function definition with an integer formal (`int`/`unsigned`/`signed`/
`_Bool`/`const` words only) that the body spells only as `if (NAME)` or
`if (!NAME)`, with every occurrence of the function's name in the same file
a definition, a prototype or a call whose actual at that formal is a
decimal constant or one of the macros FLB_TRUE/FLB_FALSE/MK_TRUE/MK_FALSE/
TRUE/FALSE/rd_true/rd_false/CIO_TRUE/CIO_FALSE (which preprocess to 1/0 on
this build), of one truth. Approximate (comments and strings are stripped
crudely, macros are not expanded beyond that list): a prediction aid, not
evidence. usage: candidate_census.py ROOT > out.json
"""
import json
import os
import re
import sys

MACROS = {"FLB_TRUE": 1, "FLB_FALSE": 0, "MK_TRUE": 1, "MK_FALSE": 0,
          "TRUE": 1, "FALSE": 0, "rd_true": 1, "rd_false": 0,
          "CIO_TRUE": 1, "CIO_FALSE": 0}
WORDS = {"const", "int", "signed", "unsigned", "_Bool"}
TOKEN = re.compile(r"[A-Za-z_]\w*|\d\w*|->|\S")


def strip(text):
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    text = re.sub(r"//[^\n]*", " ", text)
    text = re.sub(r'"(\\.|[^"\\])*"', '""', text)
    text = re.sub(r"'(\\.|[^'\\])*'", "''", text)
    text = re.sub(r"^\s*#[^\n]*(\\\n[^\n]*)*", " ", text, flags=re.M)
    return text


def match(tokens, start, open_, close):
    depth = 0
    for index in range(start, len(tokens)):
        if tokens[index] == open_:
            depth += 1
        elif tokens[index] == close:
            depth -= 1
            if depth == 0:
                return index
    return None


def split(run):
    parts, depth = [[]], 0
    for token in run:
        if token in "([{":
            depth += 1
        elif token in ")]}":
            depth -= 1
        if token == "," and depth == 0:
            parts.append([])
            continue
        parts[-1].append(token)
    return [] if parts == [[]] else parts


def truth(actual):
    while len(actual) > 2 and actual[0] == "(" and actual[-1] == ")":
        actual = actual[1:-1]
    if len(actual) != 1:
        return None
    token = actual[0]
    if token in MACROS:
        return MACROS[token] != 0
    if re.fullmatch(r"0|[1-9]\d{0,3}", token):
        return token != "0"
    return None


def census(path):
    tokens = TOKEN.findall(strip(open(path, errors="replace").read()))
    found = []
    for index, token in enumerate(tokens):
        if token != "{" or index == 0 or tokens[index - 1] != ")":
            continue
        close = index - 1
        depth, open_ = 0, None
        for back in range(close, -1, -1):
            if tokens[back] == ")":
                depth += 1
            elif tokens[back] == "(":
                depth -= 1
                if depth == 0:
                    open_ = back
                    break
        if open_ is None or open_ == 0:
            continue
        name = tokens[open_ - 1]
        if not re.fullmatch(r"[A-Za-z_]\w*", name):
            continue
        prefix = []
        for back in range(open_ - 2, -1, -1):
            if tokens[back] in (";", "{", "}"):
                break
            prefix.append(tokens[back])
        if "static" not in prefix or "extern" in prefix:
            continue
        end = match(tokens, index, "{", "}")
        if end is None:
            continue
        body = tokens[index + 1:end]
        params = split(tokens[open_ + 1:close])
        for position, param in enumerate(params):
            if len(param) < 2 or not set(param[:-1]) <= WORDS or param[:-1] == ["const"]:
                continue
            formal = param[-1]
            tests, ok = 0, True
            for at, word in enumerate(body):
                if word != formal:
                    continue
                plain = at >= 2 and body[at - 1] == "(" and body[at - 2] == "if"
                negated = at >= 3 and body[at - 1] == "!" and body[at - 2] == "(" and body[at - 3] == "if"
                if at + 1 < len(body) and body[at + 1] == ")" and (plain or negated):
                    tests += 1
                else:
                    ok = False
                    break
            if not ok or tests == 0:
                continue
            truths, escaped = set(), False
            for at, word in enumerate(tokens):
                if word != name or at == open_ - 1:
                    continue
                if at + 1 >= len(tokens) or tokens[at + 1] != "(":
                    escaped = True
                    break
                stop = match(tokens, at + 1, "(", ")")
                previous = tokens[at - 1] if at else ""
                if previous in (".", "->", "&"):
                    escaped = True
                    break
                if (re.fullmatch(r"[A-Za-z_]\w*", previous) and previous not in
                        ("return", "else", "do", "case")) or previous == "*":
                    if stop is None or stop + 1 >= len(tokens) or tokens[stop + 1] != ";":
                        escaped = True
                        break
                    continue
                actuals = split(tokens[at + 2:stop])
                if len(actuals) != len(params):
                    escaped = True
                    break
                truths.add(truth(actuals[position]))
            found.append({
                "file": path, "function": name, "formal": formal, "tests": tests,
                "escaped": escaped, "truths": sorted(str(t) for t in truths),
                "fixed": (not escaped and len(truths) == 1 and None not in truths),
            })
    return found


def main():
    root = sys.argv[1]
    rows = []
    for directory, _, files in os.walk(root):
        for file in files:
            if file.endswith(".c") and os.path.isfile(os.path.join(directory, file)):
                rows.extend(census(os.path.join(directory, file)))
    for row in rows:
        row["file"] = os.path.relpath(row["file"], root)
    rows.sort(key=lambda row: (row["file"], row["function"], row["formal"]))
    json.dump({"rows": rows, "fixed": [r for r in rows if r["fixed"]]}, sys.stdout, indent=1)


if __name__ == "__main__":
    main()
