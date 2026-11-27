#!/usr/bin/env python3
"""ctor_census.py OUT.json -- OP-CONSTRUCTOR-DECLARATIONS: a reading aid for
the census (never evidence for a clause: the gateway reads Clang's AST at
implementation). Over librdkafka's sources in the two audited Fluent Bit trees
(`66910c10`, `046accf6`), list every call of the op constructors
(`rd_kafka_op_new` -> `rd_kafka_op_new0`, `rd_kafka_op_new0`,
`rd_kafka_op_new_cb`, the `rd_kafka_txn_op_req` macro that expands to
`rd_kafka_op_new_cb(rk, RD_KAFKA_OP_TXN, op_cb)`) and of the request family
(`rd_kafka_op_req`, `rd_kafka_op_req0`, `rd_kafka_op_req2`), with its
enclosing function and how its type actual is decided:

* `constant`: only `RD_KAFKA_OP_*` enumerators and flags, integer literals,
  `|`, parentheses and casts;
* `formal`: one identifier that is a parameter of the enclosing function;
* `computed`: anything else (a member read, a local, an expression).

For `rd_kafka_op_new_cb` (and the txn macro) the callback actual is
classified the same way (`designator` for a function name). For each call
assigned to a variable, it also says whether the same function names that
variable as `rd_kafka_op_destroy`'s actual (a destroy in the creating body,
the place a decided arm can be read). Prints a table and writes OUT.json."""
import collections
import json
import pathlib
import re
import sys

TREES = {
    "66910c10": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"),
    "046accf6": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"),
}
SRC = "lib/librdkafka-2.15.0/src"
CALLEES = {
    # name: index of the type actual (None: no type actual), index of cb
    "rd_kafka_op_new": (0, None),
    "rd_kafka_op_new0": (1, None),
    "rd_kafka_op_new_cb": (1, 2),
    "rd_kafka_txn_op_req": (None, 1),
    "rd_kafka_op_req": (None, None),
    "rd_kafka_op_req0": (None, None),
    "rd_kafka_op_req2": (1, None),
}
CALL = re.compile(r"(?<![\w.>])(%s)\s*\(" % "|".join(sorted(CALLEES, key=len, reverse=True)))
CONST = re.compile(r"^[\s()|]*(?:\((?:int|rd_kafka_op_type_t)\)\s*)?(?:RD_KAFKA_OP_\w+|\d+)(?:[\s()|]*(?:\((?:int|rd_kafka_op_type_t)\)\s*)?(?:RD_KAFKA_OP_\w+|\d+))*[\s()]*$")
IDENT = re.compile(r"^[A-Za-z_]\w*$")
FUNC_HEAD = re.compile(r"^[A-Za-z_][\w\s\*]*?\b([A-Za-z_]\w*)\s*\(([^;{]*?)\)\s*\{", re.S | re.M)


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", lambda m: re.sub(r"[^\n]", " ", m.group(0)), text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)


def functions(text):
    """(name, params, start, end) of each top-level function definition."""
    out = []
    depth, i, n = 0, 0, len(text)
    top_start = 0
    while i < n:
        c = text[i]
        if c == "{":
            if depth == 0:
                head = text[top_start:i + 1]
                m = None
                for m in FUNC_HEAD.finditer(head):
                    pass
                start = i
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                if m and m.end() == len(head) and not re.search(r"\b(struct|union|enum)\s*\w*\s*$", head[:m.start(1)]) and "=" not in head[m.start(1) - 3:m.start(1)]:
                    params = []
                    for p in m.group(2).split(","):
                        p = re.sub(r"\[.*?\]", "", p)
                        pm = re.search(r"\(\s*\*\s*(\w+)\s*\)|(\w+)\s*$", p.strip())
                        if pm:
                            params.append(pm.group(1) or pm.group(2))
                    out.append((m.group(1), params, start, i))
                top_start = i + 1
        elif c == ";" and depth == 0:
            top_start = i + 1
        elif c == "#" and depth == 0 and (i == 0 or text[i - 1] == "\n"):
            j = i
            while True:
                e = text.find("\n", j)
                if e < 0 or text[e - 1] != "\\":
                    break
                j = e + 1
            top_start = (e if e >= 0 else n) + 1
            i = top_start - 1
        i += 1
    return out


def arguments(text, open_paren):
    """The top-level argument texts of the call whose `(` is at open_paren."""
    depth, i, start, args = 0, open_paren, open_paren + 1, []
    while i < len(text):
        c = text[i]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
            if depth == 0:
                args.append(text[start:i].strip())
                return args, i
        elif c == "," and depth == 1:
            args.append(text[start:i].strip())
            start = i + 1
        i += 1
    return args, i


def classify(actual, params, designator=False):
    a = actual.strip()
    if CONST.match(a):
        return "constant"
    if IDENT.match(a):
        if a in params:
            return "formal"
        if designator and not a.startswith("RD_"):
            return "designator"
    return "computed"


def census(root):
    rows = []
    for path in sorted((root / SRC).glob("*.c")):
        text = strip_comments(path.read_text(errors="replace"))
        funcs = functions(text)
        for name, params, start, end in funcs:
            body = text[start:end + 1]
            destroyed = set(re.findall(r"rd_kafka_op_destroy\s*\(\s*(\w+)\s*\)", body))
            for m in CALL.finditer(body):
                callee = m.group(1)
                if callee == name:
                    continue
                args, close = arguments(body, m.end() - 1)
                type_at, cb_at = CALLEES[callee]
                before = body[:m.start()]
                lhs = re.search(r"(\w+)\s*=\s*$", before)
                line = text.count("\n", 0, start + m.start()) + 1
                row = {
                    "file": path.name, "line": line, "function": name, "callee": callee,
                    "type_actual": args[type_at] if type_at is not None and type_at < len(args) else None,
                    "type_class": classify(args[type_at], params) if type_at is not None and type_at < len(args) else ("txn-constant" if callee == "rd_kafka_txn_op_req" else None),
                    "cb_actual": args[cb_at] if cb_at is not None and cb_at < len(args) else None,
                    "cb_class": classify(args[cb_at], params, True) if cb_at is not None and cb_at < len(args) else None,
                    "assigned_to": lhs.group(1) if lhs else None,
                    "destroyed_in_body": bool(lhs and lhs.group(1) in destroyed),
                }
                rows.append(row)
    return rows


def main():
    out = pathlib.Path(sys.argv[1])
    result = {}
    for rev, root in TREES.items():
        result[rev] = census(root)
    same = [(r["file"], r["function"], r["callee"], r["type_actual"], r["cb_actual"]) for r in result["66910c10"]] == \
        [(r["file"], r["function"], r["callee"], r["type_actual"], r["cb_actual"]) for r in result["046accf6"]]
    rows = result["66910c10"]
    summary = collections.Counter((r["callee"], r["type_class"]) for r in rows)
    cbs = collections.Counter((r["callee"], r["cb_class"]) for r in rows if r["cb_class"])
    destroyed = collections.Counter((r["callee"], r["type_class"]) for r in rows if r["destroyed_in_body"])
    funcs = collections.Counter(r["callee"] for r in rows)
    fam = collections.Counter((r["file"], r["type_class"]) for r in rows if r["callee"] in ("rd_kafka_op_new", "rd_kafka_op_new0", "rd_kafka_op_new_cb", "rd_kafka_txn_op_req"))
    doc = {
        "identical_at_both_revisions": same,
        "calls": len(rows),
        "by_callee": dict(sorted(funcs.items())),
        "type_class": {f"{k[0]} {k[1]}": v for k, v in sorted(summary.items(), key=str)},
        "cb_class": {f"{k[0]} {k[1]}": v for k, v in sorted(cbs.items(), key=str)},
        "destroyed_in_creating_body": {f"{k[0]} {k[1]}": v for k, v in sorted(destroyed.items(), key=str)},
        "by_file_and_class": {f"{k[0]} {k[1]}": v for k, v in sorted(fam.items(), key=str)},
        "rows": rows,
    }
    out.write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")
    print("identical at both revisions:", same, "calls:", len(rows))
    for key in ("by_callee", "type_class", "cb_class", "destroyed_in_creating_body"):
        print(key)
        for k, v in doc[key].items():
            print(f"  {k}: {v}")


if __name__ == "__main__":
    main()
