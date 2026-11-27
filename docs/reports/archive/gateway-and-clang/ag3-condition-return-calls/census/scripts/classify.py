"""Lexical sub-shape classification of AG3 sites (census step 1).

Reads ag3_rows.json (from ag3_census.py) and prints per-position, per-shape
counts: sites, rows mentioning, rows where AG3 is the only blocker, and rows
where that shape is the only AG3 shape left (sole and single-shape).
"""
import json
import re
import sys
from collections import Counter, defaultdict

rows = json.load(open(sys.argv[1]))
OUT = sys.argv[2] if len(sys.argv) > 2 else None

KEYWORDS = {"if", "while", "for", "switch", "return", "sizeof", "_Alignof", "__alignof__",
            "typeof", "__typeof__", "do", "else", "__builtin_expect", "likely", "unlikely",
            "__builtin_offsetof", "offsetof"}
TOK = re.compile(r"[A-Za-z_][A-Za-z_0-9]*|->|&&|\|\||==|!=|<=|>=|\+\+|--|<<|>>|[-+*/%&|^!~?:<>=()\[\]{},.;]|\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])*'|\d[\w.]*")


def expr_of(text, position):
    if text is None:
        return None
    t = text
    if position == "return expression":
        m = re.search(r"\breturn\b(.*)", t)
        if m:
            return m.group(1).rstrip(";").strip()
        return t
    if position in ("branch condition", "loop condition", "switch expression"):
        m = re.search(r"\b(if|while|switch)\s*\((.*)", t)
        if m:
            body = m.group(2)
            depth = 1
            for i, ch in enumerate(body):
                if ch == "(":
                    depth += 1
                elif ch == ")":
                    depth -= 1
                    if depth == 0:
                        return body[:i]
            return body
        return t
    if position == "for-loop condition":
        m = re.search(r"\bfor\s*\((.*)", t)
        if m:
            parts = m.group(1).split(";")
            if len(parts) >= 2:
                return parts[1].strip()
        return t
    return t


def shapes(expr, position):
    if expr is None:
        return ["no-source-text"]
    toks = TOK.findall(expr)
    out = []
    macro_call = any(re.fullmatch(r"[A-Z][A-Z0-9_]{2,}", toks[i]) and i + 1 < len(toks) and toks[i + 1] == "("
                     for i in range(len(toks)))
    calls = []
    indirect_calls = 0
    for i in range(len(toks) - 1):
        if toks[i + 1] != "(":
            continue
        name = toks[i]
        if re.fullmatch(r"[A-Za-z_]\w*", name) and name not in KEYWORDS:
            prev = toks[i - 1] if i > 0 else ""
            if prev in ("->", "."):
                indirect_calls += 1
            elif re.fullmatch(r"[A-Z][A-Z0-9_]{2,}", name):
                pass
            else:
                # exclude casts: `(T)(` is handled as cast-of-group, ignore
                calls.append(name)
        elif name == ")":
            # `(expr)(args)` or cast-of-group; count a call through an expression
            # only when the group before is not a type cast (heuristic: contains -> or .)
            pass
    member = sum(1 for t in toks if t in ("->",)) + sum(1 for i, t in enumerate(toks) if t == "." and i > 0 and re.fullmatch(r"[\w\])]+", toks[i - 1] or ""))
    subscript = "[" in toks
    address = any(t == "&" and (i == 0 or toks[i - 1] in ("(", ",", "=", "return", "&&", "||", "!", "?", ":"))
                  for i, t in enumerate(toks))
    deref = any(t == "*" and (i == 0 or toks[i - 1] in ("(", ",", "=", "&&", "||", "!", "?", ":", "==", "!="))
                for i, t in enumerate(toks))
    assign = any(t == "=" for t in toks)
    connective = any(t in ("&&", "||", "?") for t in toks)
    incdec = any(t in ("++", "--") for t in toks)
    if position == "do-while condition":
        return ["do-while (condition at loop end; text not read)"]
    if macro_call:
        out.append("macro")
    if indirect_calls:
        out.append("call-through-member")
    if calls:
        builtin = [c for c in calls if c.startswith("__builtin")]
        if builtin and len(builtin) == len(calls):
            out.append("builtin-call")
        else:
            out.append("call")
    if assign:
        out.append("assignment")
    if incdec:
        out.append("incdec")
    if address:
        out.append("address-of")
    if member:
        out.append("member" if member == 1 else "member-chain")
    if subscript:
        out.append("subscript")
    if deref:
        out.append("deref")
    if connective:
        out.append("connective")
    if not out:
        out.append("plain-other")
    return out


def primary(sh):
    # precedence: what most likely refuses first
    for key in ("macro", "call-through-member", "call", "builtin-call", "assignment", "incdec",
                "address-of", "member-chain", "subscript", "member", "deref", "connective",
                "do-while (condition at loop end; text not read)", "no-source-text", "plain-other"):
        if key in sh:
            return key
    return sh[0]


site = Counter()
mention = defaultdict(set)
sole = defaultdict(set)
single = defaultdict(set)
samples = defaultdict(list)
records = []
for idx, r in enumerate(rows):
    keys = set()
    for a in r["ag3"]:
        expr = expr_of(a["text"], a["position"])
        sh = shapes(expr, a["position"])
        p = primary(sh)
        key = (a["position"], p)
        site[key] += 1
        mention[key].add(idx)
        keys.add(key)
        if r["sole"]:
            sole[key].add(idx)
            if len(samples[key]) < 6:
                samples[key].append(f'{r["function"]} @ {a["origin"]}: {expr}')
        records.append({"function": r["function"], "library": r["library"], "status": r["status"],
                        "sole": r["sole"], "position": a["position"], "origin": a["origin"],
                        "expr": expr, "shapes": sh, "primary": p})
    if r["sole"] and len(keys) == 1:
        single[next(iter(keys))].add(idx)

print(f"{'position':22s} {'primary shape':32s} {'sites':>6s} {'rows':>6s} {'sole':>5s} {'only':>5s}")
for key in sorted(site, key=lambda k: (-len(single[k]), -len(sole[k]), -site[k])):
    print(f"{key[0]:22s} {key[1]:32s} {site[key]:6d} {len(mention[key]):6d} {len(sole[key]):5d} {len(single[key]):5d}")
print()
for key in sorted(samples, key=lambda k: -len(sole[k])):
    print("==", key)
    for s in samples[key]:
        print("   ", s[:180])
if OUT:
    json.dump(records, open(OUT, "w"), indent=0)
