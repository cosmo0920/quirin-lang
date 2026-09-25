#!/usr/bin/env python3
"""Frozen forecast of card W45-RETURN-EXPRESSION-FAMILIES, computed before the
rules were implemented.

Usage: predict.py CENSUS_JSON CENSUS_JSONL [--json OUT]

CENSUS_JSON is `return_census.py --json` over the BEFORE run, CENSUS_JSONL the
same run's `QCORE_RETURN_CENSUS` stream (for the chain facts). The enclosing
function's declared result is read heuristically from the pinned source text
(the Rust rule reads the preprocessed tokens instead, so this is a forecast,
not the rule): the definition line `PREFIX NAME (` in a file with the leaf's
basename. Each leaf is assigned the family the card intends to admit, or kept.

* S  scalar-result read: the function's result has no `*` and is not `void`;
     the expression carries no call (other than a type operator), brace,
     assignment, increment, comma or unary `&`; every chain root is
     never-bound, or may-bind with one selection in an unconditionally
     evaluated position.
* K  cast of a whole call: `( T ) … CALL ( … )`. A contracted `Owned` result
     under pointer casts is admitted as the uncast W4 return; every other
     classification the classifier refuses is lowered to that refusal.
* R  everything else keeps the return-position refusal.
"""

import collections
import json
import pathlib
import re
import sys

PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin")
TYPE_OPERATORS = {"sizeof", "_Alignof", "__alignof__", "alignof"}


def index_sources():
    files = collections.defaultdict(list)
    for root in (PIN / "fluent-bit-clang-source", PIN / "monkey"):
        for path in root.rglob("*.[ch]"):
            files[path.name].append(path)
    return files


def result_prefix(files, file, function):
    """The declared prefix of `function`'s definition in a file named `file`."""
    pattern = re.compile(r"^([A-Za-z_][^;{}()=]*?)\b" + re.escape(function) + r"\s*\(")
    for path in files.get(pathlib.Path(file).name, []):
        lines = path.read_text(errors="replace").splitlines()
        for index, line in enumerate(lines):
            # `CJSON_PUBLIC(type) name(`: the export macro's operand is the type.
            exported = re.match(r"^[A-Z_]+\(([^()]*)\)\s*" + re.escape(function) + r"\s*\(", line)
            if exported:
                return exported.group(1).strip()
            match = pattern.match(line)
            if match:
                return match.group(1).strip()
            # K&R-style: the type on the previous line, the name first here.
            if re.match(r"^" + re.escape(function) + r"\s*\(", line) and index > 0:
                return lines[index - 1].strip()
    return None


def carrier(prefix):
    if prefix is None:
        return "unread"
    if "*" in prefix:
        return "pointer"
    words = prefix.replace("static", "").replace("inline", "").split()
    if words == ["void"] or not words:
        return "void"
    if words and words[0] in ("struct", "union"):
        return "aggregate"
    return "scalar"


def conditional_positions(tokens):
    """Token indices C may skip: after a `?` or a `&&`/`||` at their depth."""
    conditional = [False] * len(tokens)
    depth = 0
    starts = []
    for index, token in enumerate(tokens):
        if token in ("(", "["):
            depth += 1
        elif token in (")", "]"):
            depth -= 1
            starts = [s for s in starts if s[0] <= depth]
        if token == "?" or (token in ("&", "|") and index + 1 < len(tokens)
                            and tokens[index + 1] == token):
            starts.append((depth, index))
        if any(d <= depth for d, _ in starts):
            conditional[index] = True
    return conditional


def scalar_shape(tokens):
    for index, token in enumerate(tokens):
        nxt = tokens[index + 1] if index + 1 < len(tokens) else None
        prev = tokens[index - 1] if index > 0 else None
        if token in ("{", "}", ","):
            return False, "brace or comma"
        if token[:1].isalpha() or token[:1] == "_":
            if nxt == "(" and token not in TYPE_OPERATORS:
                return False, "call"
        if token == "=" and nxt != "=" and prev not in ("=", "!", "<", ">"):
            return False, "assignment"
        if token in ("+", "-") and nxt == token:
            return False, "increment"
        if token == "&" and nxt != "&" and prev != "&":
            if prev is None or prev in ("(", "?", ":", "!", "=", "|", "+", "-", "*", "/", "<", ">", "~", "^", "%"):
                return False, "address-of"
    return True, None


def whole_call_under_cast(tokens):
    index = 0
    casts = 0
    while index < len(tokens) and tokens[index] == "(":
        depth = 0
        for close in range(index, len(tokens)):
            if tokens[close] == "(":
                depth += 1
            elif tokens[close] == ")":
                depth -= 1
                if depth == 0:
                    break
        inner = tokens[index + 1:close]
        if not inner or not all(t == "*" or t[:1].isalpha() or t[:1] == "_" for t in inner):
            return None
        casts += 1
        index = close + 1
    if casts == 0 or index + 2 >= len(tokens):
        return None
    if (not (tokens[index][:1].isalpha() or tokens[index][:1] == "_") or tokens[index + 1] != "("
            or tokens[index] in TYPE_OPERATORS):
        return None
    depth = 0
    for close in range(index + 1, len(tokens)):
        if tokens[close] == "(":
            depth += 1
        elif tokens[close] == ")":
            depth -= 1
            if depth == 0:
                return tokens[index] if close == len(tokens) - 1 else None
    return None


def main():
    census = json.loads(pathlib.Path(sys.argv[1]).read_text())
    records = {}
    for line in pathlib.Path(sys.argv[2]).read_text().splitlines():
        if line:
            record = json.loads(line)
            records[(record["function"], record["line"])] = record
    files = index_sources()
    out = []
    for leaf in census["leaves"]:
        tokens = leaf["expression"].split(" ")
        record = records.get((leaf["function"], leaf["line"]), {"chains": []})
        prefix = result_prefix(files, leaf["file"], leaf["function"])
        kind = carrier(prefix)
        callee = whole_call_under_cast(tokens)
        forecast, why = "R", None
        if callee is not None:
            forecast = "K"
            why = "cast of a whole call"
        elif kind == "scalar":
            ok, why = scalar_shape(tokens)
            if ok:
                conditional = conditional_positions(tokens)
                for chain in record["chains"]:
                    if chain["base_never_bound"] and not chain["base_may_bind"]:
                        continue
                    if chain["depth"] != 1:
                        ok, why = False, "may-bind root with a chain"
                        break
                    positions = [i for i, t in enumerate(tokens) if t == chain["base"]]
                    if any(conditional[i] for i in positions):
                        ok, why = False, "may-bind root in a conditional position"
                        break
                if ok:
                    forecast, why = "S", "scalar-result read"
        else:
            why = f"result {kind}"
        out.append({**leaf, "result_prefix": prefix, "carrier": kind, "forecast": forecast, "why": why})

    unknown = [r for r in out if r["status"] == "Unknown"]
    by_function = collections.defaultdict(list)
    for row in unknown:
        by_function[(row["library"], row["file"], row["function"])].append(row)
    freed = [key for key, rows in by_function.items()
             if rows[0]["sole"] and all(r["forecast"] == "S" or (r["forecast"] == "K" and r["family"].startswith("B.")) for r in rows)]
    summary = {
        "unknown_leaves": len(unknown),
        "forecast_leaves": dict(collections.Counter(r["forecast"] for r in unknown)),
        "forecast_by_family": {
            f"{fam} -> {fc}": n
            for (fam, fc), n in sorted(collections.Counter((r["family"], r["forecast"]) for r in unknown).items())
        },
        "carrier": dict(collections.Counter(r["carrier"] for r in unknown)),
        "sole_functions": len({k for k, rows in by_function.items() if rows[0]["sole"]}),
        "sole_functions_freed": len(freed),
        "kept_reasons": dict(collections.Counter(r["why"] for r in unknown if r["forecast"] == "R")),
    }
    print(json.dumps(summary, indent=1))
    if "--json" in sys.argv:
        path = pathlib.Path(sys.argv[sys.argv.index("--json") + 1])
        path.write_text(json.dumps({"summary": summary, "freed": sorted(freed), "leaves": out}, indent=1) + "\n")


if __name__ == "__main__":
    main()
