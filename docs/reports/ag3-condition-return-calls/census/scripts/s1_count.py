"""Lexical size of candidate sub-shapes over AG3 condition sites.

S1: a direct call plus a member / subscript / deref read outside every call's
argument list (the residual reads memory).
S4: a function-pointer-looking member truth/NULL test (heuristic: `->name`
followed by `!=`/`==` NULL or used as a bare && / || operand, with no call).
"""
import json, re, sys
from collections import defaultdict

sites = json.load(open(sys.argv[1]))
TOK = re.compile(r"[A-Za-z_]\w*|->|&&|\|\||==|!=|<=|>=|\+\+|--|[-+*/%&|^!~?:<>=()\[\]{},.;]|\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])*'|\d[\w.]*")
KW = {"sizeof", "unlikely", "likely", "__builtin_expect", "if", "while", "for", "switch", "return"}


def outside_calls(toks):
    """Tokens outside the argument lists of direct calls; and the call names."""
    out, calls, i = [], [], 0
    while i < len(toks):
        t = toks[i]
        if re.fullmatch(r"[A-Za-z_]\w*", t) and i + 1 < len(toks) and toks[i + 1] == "(" and t not in KW \
                and not (i > 0 and toks[i - 1] in ("->", ".")):
            depth, j = 0, i + 1
            while j < len(toks):
                if toks[j] == "(":
                    depth += 1
                elif toks[j] == ")":
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            calls.append(t)
            out.append("CALL")
            i = j + 1
            continue
        out.append(t)
        i += 1
    return out, calls


res = defaultdict(lambda: [0, set(), set()])
for s in sites:
    if s["position"] == "return expression" or not s["expr"]:
        continue
    toks = TOK.findall(s["expr"])
    rest, calls = outside_calls(toks)
    macro = any(re.fullmatch(r"[A-Z][A-Z0-9_]{2,}", c) for c in calls)
    real = [c for c in calls if not re.fullmatch(r"[A-Z][A-Z0-9_]{2,}", c)]
    reads = any(t in ("->", ".", "[") for t in rest) or any(
        t == "*" and (k == 0 or rest[k - 1] in ("(", "!", "&&", "||", "==", "!=")) for k, t in enumerate(rest))
    key = None
    if real and not macro and reads:
        key = "S1 call + residual memory read"
    elif not calls and re.search(r"->\s*\w+\s*(!=|==)\s*(NULL|0)\b|&&\s*\w+\s*->\s*\w+\s*($|\)|&&|\|\|)", s["expr"]):
        key = "S4 member truth/NULL test, no call"
    if key:
        k = (s["function"], s["origin"].rsplit(":", 1)[0])
        res[key][0] += 1
        res[key][1].add(k)
        if s["sole"]:
            res[key][2].add(k)
for key, (n, rows, sole) in res.items():
    print(f"{key:40s} sites {n:5d} rows {len(rows):5d} sole {len(sole):3d}")
    for k in sorted(sole)[:12]:
        print("    sole:", k)
