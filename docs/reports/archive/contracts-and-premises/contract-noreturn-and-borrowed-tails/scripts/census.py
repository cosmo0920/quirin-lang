#!/usr/bin/env python3
"""CONTRACT-NORETURN-AND-BORROWED-TAILS: the card's census and prediction input.

Usage: census.py DIAGNOSTICS PARITY.jsonl OUT.json [--route NAME]

Two families, read from one route's `resource-diagnostics.json` and its
`QCORE_TYPED_PATH_PARITY` stream:

* **noreturn**: leaves `call to uncontracted function `C`` for every callee C
  the stream's `call-exit-census` rows resolve `NoReturn` (CG21's census rows:
  an uncontracted whole-call statement whose Clang declaration is noreturn).
* **tail**: leaves `arguments beyond the declared parameters of `C` are
  outside the resource projection` per callee C (O04-TRACE's tail refusal under
  a declared data tail).

Per callee: leaves, functions, `alone` (Unknown functions whose every leaf is
that callee's family leaf), and the statement shapes (`variadic-tail-statement`
rows, de-duplicated by unit source, function, line and tokens): each trailing
actual is classified by its tokens (`name`, `literal`, `chain` for a member
access chain with no call, `other`), and a statement is *T8-shaped* when every
trailing actual is a plain name or a literal, *chain-shaped* when every one is
a name, a literal or a chain. Jointly (`joint`): Unknown functions whose every
leaf is in either family for the card's callees, round 1; round 2 adds the
Unknown functions whose remaining leaves are calls (`call to uncontracted
function` / `nested call`) to a same-unit function freed in round 1 (the
existing W4 summary rule), iterated to a fixed point (`rounds`). Upper bounds:
a freed statement's actuals are still checked by the flow (a released or
unresolved identity in a borrowed position keeps a refusal), and a T8 shape
does not move when the function's declarations are unreadable.
"""

import argparse
import collections
import json
import pathlib
import re

UNC = re.compile(r"^call to uncontracted function `([^`]+)`$")
NESTED = re.compile(r"^nested call `([^`]+)` inside a modeled call is outside the resource projection$")
TAIL = re.compile(r"^arguments beyond the declared parameters of `([^`]+)` are outside the resource projection$")
NAMED = {"printf": 1, "fprintf": 2, "snprintf": 3, "sprintf": 2, "mk_print": 2,
         "flb_log_print": 4, "prctl": 1, "open": 2, "fcntl": 2, "mk_vhost_set": 2,
         "flb_service_set": 1}
CARD_TAILS = {"printf", "fprintf", "snprintf", "mk_print", "flb_log_print"}
ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}
IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
NUMBER = re.compile(r"^[0-9][0-9A-Za-z_.]*$")


def split_arguments(texts):
    """The top-level actuals of `callee ( a , b , ... )`."""
    if len(texts) < 3 or texts[1] != "(" or texts[-1] != ")":
        return None
    out, depth, current = [], 0, []
    for token in texts[2:-1]:
        if token in "([{":
            depth += 1
        elif token in ")]}":
            depth -= 1
        if token == "," and depth == 0:
            out.append(current)
            current = []
        else:
            current.append(token)
    out.append(current)
    return out


def shape(argument):
    joined = []
    for token in argument:  # the parity stream spells `->` as `-` `>`
        if token == ">" and joined and joined[-1] == "-":
            joined[-1] = "->"
        else:
            joined.append(token)
    argument = joined
    tokens =[t for t in argument if t not in ("(", ")")] if all(
        t in ("(", ")") or IDENT.match(t) or NUMBER.match(t) for t in argument) else argument
    if not tokens:
        return "other"
    if all(t == "__qcore_literal__" or NUMBER.match(t) or t in ("__func__", "__FUNCTION__",
                                                                 "__PRETTY_FUNCTION__") for t in tokens):
        return "literal"
    if len(tokens) == 1 and IDENT.match(tokens[0]):
        return "name"
    if (IDENT.match(tokens[0]) and "(" not in tokens
            and all(IDENT.match(t) or t in ("->", ".") for t in tokens)):
        return "chain"
    return "other"


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    return json.loads(path.read_text())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("diagnostics")
    parser.add_argument("parity")
    parser.add_argument("out")
    parser.add_argument("--route", default="corpus")
    args = parser.parse_args()
    data = load(args.diagnostics)

    noreturn, seen = collections.Counter(), set()
    tail_rows = collections.defaultdict(list)
    for line in open(args.parity, encoding="utf-8"):
        if '"call-exit-census"' in line:
            row = json.loads(line)
            key = (row["source_sha256"], row["function"], row["line"], tuple(row["texts"]))
            if key in seen or row["resolution"] != "NoReturn":
                continue
            seen.add(key)
            noreturn[row["callee"]] += 1
        elif '"variadic-tail-statement"' in line:
            row = json.loads(line)
            key = ("t", row["source_sha256"], row["function"], row["line"], tuple(row["texts"]))
            if key in seen:
                continue
            seen.add(key)
            arguments = split_arguments(row["texts"])
            named = NAMED.get(row["callee"])
            shapes = None if arguments is None or named is None else [shape(a) for a in arguments[named:]]
            tail_rows[row["callee"]].append({
                "function": row["function"], "line": row["line"],
                "texts": " ".join(row["texts"]), "admitted": row["new_admitted"],
                "promoted": row["promoted_scalar_or_pointer"], "shapes": shapes})

    def family(description):
        match = UNC.match(description)
        if match and match.group(1) in noreturn:
            return ("noreturn", match.group(1))
        match = TAIL.match(description)
        if match:
            return ("tail", match.group(1))
        return None

    functions, defined = {}, collections.defaultdict(set)
    for unit in data["units"]:
        for entry in unit["functions"]:
            key = (unit["file"], entry["function"])
            defined[entry["function"]].add(unit["file"])
            record = functions.setdefault(key, {"status": entry["status"], "leaves": []})
            if ORDER.get(entry["status"], 2) > ORDER.get(record["status"], 2):
                record["status"] = entry["status"]
            record["leaves"] += [d.get("description", "") for d in entry.get("diagnostics") or []]

    per = {}
    for key, record in functions.items():
        fams = [family(d) for d in record["leaves"]]
        for fam in set(f for f in fams if f):
            row = per.setdefault("%s:%s" % fam, {"leaves": 0, "functions": 0, "alone": [],
                                                 "statuses": collections.Counter()})
            row["functions"] += 1
            row["leaves"] += fams.count(fam)
            row["statuses"][record["status"]] += 1
            if record["status"] == "Unknown" and all(f == fam for f in fams):
                row["alone"].append("::".join(key))

    card = {("noreturn", "rd_kafka_crash")} | {("tail", c) for c in CARD_TAILS}

    def ours(description):
        return family(description) in card

    def served(file, description, freed):
        match = UNC.match(description) or NESTED.match(description)
        return bool(match) and (file, match.group(1)) in freed

    freed_rounds, freed = [], set()
    while True:
        this_round = []
        for key, record in functions.items():
            if record["status"] != "Unknown" or key in freed:
                continue
            leaves = record["leaves"]
            moved = [d for d in leaves if ours(d) or served(key[0], d, freed)]
            if moved and len(moved) == len(leaves):
                this_round.append(key)
        if not this_round:
            break
        freed_rounds.append(sorted("::".join(k) for k in this_round))
        freed.update(this_round)

    shapes = {}
    for callee, rows in tail_rows.items():
        counts = collections.Counter()
        for row in rows:
            if row["shapes"] is None:
                counts["unparsed"] += 1
            elif row["admitted"]:
                counts["admitted"] += 1
            elif not row["promoted"]:
                counts["promoted-refused"] += 1
            elif all(s in ("name", "literal") for s in row["shapes"]):
                counts["t8-shaped"] += 1
            elif all(s in ("name", "literal", "chain") for s in row["shapes"]):
                counts["chain-shaped"] += 1
            else:
                counts["other"] += 1
        shapes[callee] = {"statements": len(rows), "classes": dict(counts)}

    out = {
        "route": args.route,
        "statuses": dict(collections.Counter(r["status"] for r in functions.values())),
        "noreturn_census_rows": dict(noreturn),
        "families": {k: {**v, "alone_count": len(v["alone"]), "statuses": dict(v["statuses"])}
                     for k, v in sorted(per.items())},
        "joint_rounds": [{"round": i + 1, "count": len(r), "functions": r}
                         for i, r in enumerate(freed_rounds)],
        "tail_statement_shapes": shapes,
        "tail_statements": {c: rows for c, rows in sorted(tail_rows.items())},
    }
    pathlib.Path(args.out).write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: out[k] for k in ("route", "statuses", "noreturn_census_rows",
                                          "tail_statement_shapes")}, indent=1))
    for k, v in out["families"].items():
        print(k, "leaves", v["leaves"], "functions", v["functions"], "alone", v["alone_count"],
              v["statuses"])
    for r in out["joint_rounds"]:
        print("round", r["round"], r["count"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
