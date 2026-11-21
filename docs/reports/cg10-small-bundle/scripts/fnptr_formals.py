#!/usr/bin/env python3
"""fnptr_formals.py REPORT.json PREPARED -- CG10-SMALL-BUNDLE
census for CG10-W4-FUNCTION-POINTER-DECLARATOR-FORMALS (rule 74b).

For every unit of one opt-in `current` report, reads the unit's source text
(comments and strings blanked; not preprocessed, so a definition a macro
spells is missed: a lower bound) and lists each function definition whose
parameter list holds a parenthesised declarator. Each such formal is
classified by the shape rule 74b reads, `SPEC+ ( * NAME ) ( BALANCED )`, and
by whether the body spells a call through it (`NAME (` or `( * NAME ) (`),
which rule 74b keeps refused. One JSON object per function, with its status
and leaf count on the report, then a summary line."""
import collections
import json
import re
import sys

TOKEN = re.compile(r"[A-Za-z_][A-Za-z0-9_]*|\.\.\.|->|[0-9][A-Za-z0-9_.]*|\S")


def blank(text):
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append(re.sub(r"[^\n]", " ", text[i:j]))
            i = j
        elif c in "\"'":
            j = i + 1
            while j < n and text[j] != c:
                j += 2 if text[j] == "\\" else 1
            out.append(c + " " * (min(j, n) - i - 1) + c)
            i = j + 1
        elif c == "#" and (i == 0 or text[i - 1] == "\n"):
            j = i
            while True:
                k = text.find("\n", j)
                if k < 0:
                    k = n
                    break
                if text[k - 1] != "\\":
                    break
                j = k + 1
            out.append(re.sub(r"[^\n]", " ", text[i:k]))
            i = k
        else:
            out.append(c)
            i += 1
    return "".join(out)


def matching(tokens, start, open_, close):
    depth = 0
    for i in range(start, len(tokens)):
        if tokens[i] == open_:
            depth += 1
        elif tokens[i] == close:
            depth -= 1
            if depth == 0:
                return i
    return None


def split_top(tokens):
    groups, depth, current = [], 0, []
    for t in tokens:
        if t in "([":
            depth += 1
        elif t in ")]":
            depth -= 1
        if t == "," and depth == 0:
            groups.append(current)
            current = []
        else:
            current.append(t)
    groups.append(current)
    return groups


def shape(group):
    """The rule 74b class of one parameter group holding `(`."""
    if "(" not in group:
        return None
    k = group.index("(")
    spec = group[:k]
    if (spec and all(re.fullmatch(r"[A-Za-z_]\w*|\*", t) for t in spec)
            and group[k:k + 2] == ["(", "*"] and len(group) > k + 4
            and re.fullmatch(r"[A-Za-z_]\w*", group[k + 2]) and group[k + 3] == ")"
            and group[k + 4] == "(" and matching(group, k + 4, "(", ")") == len(group) - 1
            and "{" not in group):
        return ("read", group[k + 2])
    return ("other", " ".join(group))


def definitions(tokens):
    depth = 0
    i = 0
    while i < len(tokens):
        t = tokens[i]
        if t == "{":
            depth += 1
        elif t == "}":
            depth -= 1
        elif (depth == 0 and re.fullmatch(r"[A-Za-z_]\w*", t)
              and t not in ("if", "while", "for", "switch", "return", "sizeof")
              and i + 1 < len(tokens) and tokens[i + 1] == "("):
            close = matching(tokens, i + 1, "(", ")")
            if close is not None and close + 1 < len(tokens) and tokens[close + 1] == "{":
                end = matching(tokens, close + 1, "{", "}")
                if end is not None:
                    yield t, tokens[i + 2:close], tokens[close + 1:end + 1]
                    i = end + 1
                    continue
        i += 1


def calls(body, name):
    for j, t in enumerate(body):
        if t != name:
            continue
        if j + 1 < len(body) and body[j + 1] == "(":
            return True
        if (j >= 2 and body[j - 1] == "*" and body[j - 2] == "("
                and body[j + 1:j + 3] == [")", "("]):
            return True
    return False


def main():
    report = json.load(open(sys.argv[1]))
    prepared = sys.argv[2]
    roots = [prepared + "/fluent-bit-clang-source/", prepared + "/monkey/monkey-source/"]
    summary = collections.Counter()
    for unit in report["units"]:
        text = None
        for root in roots:
            try:
                text = open(root + unit["file"], errors="replace").read()
                break
            except OSError:
                pass
        if text is None:
            summary["unreadable-unit"] += 1
            continue
        rows = collections.defaultdict(list)
        for function in unit.get("functions", []):
            rows[function["function"]].append(function)
        tokens = TOKEN.findall(blank(text))
        for name, params, body in definitions(tokens):
            groups = split_top(params)
            shapes = [shape(g) for g in groups]
            if not any(shapes):
                continue
            formals = []
            for s in shapes:
                if s is None:
                    continue
                if s[0] == "read":
                    formals.append({"name": s[1], "class": "called" if calls(body, s[1]) else "read"})
                else:
                    formals.append({"class": "other", "text": s[1]})
            classes = {f["class"] for f in formals}
            verdict = ("other-declarator" if "other" in classes
                       else "called-formal" if "called" in classes else "read")
            variadic = any(g == ["..."] for g in groups)
            if variadic and verdict == "read":
                verdict = "variadic"
            summary[verdict] += 1
            found = rows.get(name, [])
            print(json.dumps({
                "unit": unit["file"], "function": name, "verdict": verdict,
                "formals": formals,
                "status": [r["status"] for r in found],
                "leaves": [len(r["diagnostics"]) for r in found],
            }, sort_keys=True))
    print(json.dumps({"summary": dict(summary)}, sort_keys=True))


if __name__ == "__main__":
    main()
