#!/usr/bin/env python3
"""Shape census of the refused return expressions of the pinned corpus.

Usage: return_census.py CENSUS_JSONL DIAGNOSTICS_JSON [--json OUT]

CENSUS_JSONL is the `QCORE_RETURN_CENSUS` stream an `indirect-census` build of
the audit writes when every return route has already refused: one record per
refused `return` the parser met, carrying the analyzer's own token view. The
parser may meet one body more than once (summary derivation, validation and
the published run), so records are de-duplicated by unit, function, line and
tokens after the join.

DIAGNOSTICS_JSON is the same run's `resource-diagnostics.json`. Each census
unit ordinal is joined to the published unit whose return-expression leaves it
covers; a record that joins to no published leaf is counted and reported, never
silently dropped, and every published leaf must be covered by a record.

The population is the published leaves on **Unknown** functions (the W42 blocker
census's row), and its **sole** subset: functions whose only distinct blocker is
this refusal. Families are assigned by one ordered, total rule over the token
view (first match wins), so each leaf has exactly one family.
"""

import collections
import json
import pathlib
import sys

RETURN = "unmodeled call, aggregate or indirect access in return expression"


def strip_parens(tokens):
    while len(tokens) >= 2 and tokens[0] == "(" and matching(tokens, 0) == len(tokens) - 1:
        tokens = tokens[1:-1]
    return tokens


def matching(tokens, start):
    depth = 0
    for index in range(start, len(tokens)):
        if tokens[index] in ("(", "["):
            depth += 1
        elif tokens[index] in (")", "]"):
            depth -= 1
            if depth == 0:
                return index
    return None


def top_level(tokens, predicate):
    depth = 0
    for index, token in enumerate(tokens):
        if token in ("(", "[", "{"):
            depth += 1
        elif token in (")", "]", "}"):
            depth -= 1
        elif depth == 0 and predicate(index, token):
            return True
    return False


def is_name(token):
    return bool(token) and (token[0].isalpha() or token[0] == "_")


RESERVED_CALLS = {"sizeof", "_Alignof", "__alignof__", "__builtin_expect"}
TYPE_WORDS = {
    "void", "char", "short", "int", "long", "float", "double", "signed", "unsigned",
    "_Bool", "const", "volatile", "struct", "union", "enum", "size_t", "ssize_t",
    "uint8_t", "uint16_t", "uint32_t", "uint64_t", "int8_t", "int16_t", "int32_t",
    "int64_t", "uintptr_t", "intptr_t", "bool", "off_t", "time_t",
}


def leading_cast(tokens):
    """The end index of a leading `( TYPE-NAME )` cast, or None."""
    if not tokens or tokens[0] != "(":
        return None
    end = matching(tokens, 0)
    if end is None or end + 1 >= len(tokens):
        return None
    inner = tokens[1:end]
    if not inner or not all(is_name(t) or t == "*" for t in inner):
        return None
    if not (inner[0] in TYPE_WORDS or inner[-1] == "*" or inner[0].endswith("_t")):
        return None
    return end


def whole_call(tokens):
    """`f ( ... )` spanning the whole expression."""
    return (
        len(tokens) >= 3
        and is_name(tokens[0])
        and tokens[1] == "("
        and matching(tokens, 1) == len(tokens) - 1
    )


PAIRS = {
    ("-", ">"): "->", ("=", "="): "==", ("!", "="): "!=", ("<", "="): "<=",
    (">", "="): ">=", ("&", "&"): "&&", ("|", "|"): "||", ("<", "<"): "<<",
    (">", ">"): ">>", ("+", "+"): "++", ("-", "-"): "--",
}


def merge(tokens):
    """The gateway tokenizes punctuation one character at a time; rejoin the
    two-character operators so the shape rules can read them."""
    out = []
    index = 0
    while index < len(tokens):
        pair = tuple(tokens[index:index + 2])
        if pair in PAIRS:
            out.append(PAIRS[pair])
            index += 2
        else:
            out.append(tokens[index])
            index += 1
    return out


def family(record):
    tokens = strip_parens(merge(record["tokens"]))
    calls = record["calls"]
    if not tokens:
        return "empty"
    if "{" in tokens:
        return "statement expression / braces"
    if top_level(tokens, lambda i, t: t == "?"):
        return "conditional ?:"
    if top_level(tokens, lambda i, t: t in ("&&", "||")):
        return "short-circuit && / ||"
    if top_level(tokens, lambda i, t: t == ","):
        return "comma operator"
    cast = leading_cast(tokens)
    if cast is not None:
        inner = strip_parens(tokens[cast + 1:])
        if whole_call(inner):
            return "cast of a whole call"
        return "cast of a non-call operand"
    if whole_call(tokens):
        callee = tokens[0]
        call = next((c for c in calls if c["callee"] == callee), None)
        if callee in RESERVED_CALLS:
            return "type operator"
        if call is None:
            return "whole call (unclassified callee)"
        if not call["contracted"]:
            where = "defined in unit" if call["defined_in_unit"] else "external"
            return f"whole call, uncontracted callee ({where})"
        return f"whole call, contracted callee (result {call['result_mode']})"
    if tokens[0] == "&":
        return "address-of"
    if tokens[0] == "*" :
        return "dereference"
    if tokens[0] == "!" and not calls:
        return "logical not over access"
    if calls:
        contracted = all(c["contracted"] for c in calls)
        return (
            "calls inside compound expression (all contracted)"
            if contracted
            else "calls inside compound expression (some uncontracted)"
        )
    if "++" in tokens or "--" in tokens or top_level(tokens, lambda i, t: t == "="):
        return "mutation inside expression (no call)"
    if top_level(tokens, lambda i, t: t in ("==", "!=", "<", ">", "<=", ">=")):
        return "comparison over accesses (no call)"
    operators = {"+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>", "~"}
    if top_level(tokens, lambda i, t: t in operators and i > 0):
        return "arithmetic over accesses (no call)"
    if "[" in tokens:
        return "subscript access (no call)"
    chains = record["chains"]
    if len(chains) == 1 and all(t not in operators for t in tokens):
        chain = chains[0]
        bound = "bound base" if chain["base_may_bind"] else "never-bound base"
        return f"single access chain, {bound}, leaf {chain['member_class']}"
    return "other"


def main() -> int:
    census_path = pathlib.Path(sys.argv[1])
    diagnostics = json.loads(pathlib.Path(sys.argv[2]).read_text())
    out = None
    if "--json" in sys.argv:
        out = pathlib.Path(sys.argv[sys.argv.index("--json") + 1])

    # Published leaves, keyed by (file, function, line), with multiplicity.
    published = collections.Counter()
    status = {}
    reasons = collections.defaultdict(set)
    by_function_line = collections.defaultdict(set)
    for unit in diagnostics["units"]:
        for entry in unit["functions"]:
            key = (unit["library"], unit["file"], entry["function"])
            status[key] = entry.get("status")
            for diagnostic in entry.get("diagnostics") or []:
                reasons[key].add(diagnostic["description"])
                if diagnostic["description"] == RETURN:
                    published[(*key, diagnostic["line"])] += 1
                    by_function_line[(entry["function"], diagnostic["line"])].add(key[:2])

    # Join each census unit ordinal to exactly one published unit.
    records = [json.loads(line) for line in census_path.read_text().splitlines() if line]
    per_seq = collections.defaultdict(list)
    for record in records:
        per_seq[record["unit_seq"]].append(record)
    seq_unit = {}
    unjoined_seq = []
    for seq, group in per_seq.items():
        votes = collections.Counter()
        for record in group:
            for unit in by_function_line.get((record["function"], record["line"]), ()):
                votes[unit] += 1
        if not votes:
            unjoined_seq.append(seq)
            continue
        best, count = votes.most_common(1)[0]
        ties = [unit for unit, c in votes.items() if c == count]
        if len(ties) != 1:
            unjoined_seq.append(seq)
            continue
        seq_unit[seq] = best

    joined = {}
    unmatched_records = 0
    for record in records:
        unit = seq_unit.get(record["unit_seq"])
        if unit is None:
            unmatched_records += 1
            continue
        key = (*unit, record["function"], record["line"])
        if key not in published:
            unmatched_records += 1
            continue
        joined.setdefault(key, set()).add(tuple(record["tokens"]))
        joined[key + ("record",)] = record

    leaves = []
    uncovered = []
    ambiguous = 0
    for key, multiplicity in sorted(published.items()):
        spellings = joined.get(key)
        if not spellings:
            uncovered.append(list(key))
            continue
        if len(spellings) != 1:
            ambiguous += 1
        record = joined[key + ("record",)]
        function_key = key[:3]
        distinct = reasons[function_key]
        for _ in range(multiplicity):
            leaves.append(
                {
                    "library": key[0],
                    "file": key[1],
                    "function": key[2],
                    "line": key[3],
                    "status": status[function_key],
                    "sole": distinct == {RETURN},
                    "family": family(record),
                    "expression": " ".join(record["tokens"]),
                }
            )

    def table(rows):
        leaf_count = collections.Counter(row["family"] for row in rows)
        function_sets = collections.defaultdict(set)
        for row in rows:
            function_sets[row["family"]].add((row["library"], row["file"], row["function"]))
        return [
            {"family": name, "leaves": count, "functions": len(function_sets[name])}
            for name, count in leaf_count.most_common()
        ]

    unknown = [row for row in leaves if row["status"] == "Unknown"]
    sole = [row for row in unknown if row["sole"]]
    summary = {
        "census_records": len(records),
        "census_unit_ordinals": len(per_seq),
        "unjoined_unit_ordinals": len(unjoined_seq),
        "records_joined_to_no_published_leaf": unmatched_records,
        "published_leaves": sum(published.values()),
        "published_leaves_uncovered": len(uncovered),
        "published_leaves_with_ambiguous_spelling": ambiguous,
        "unknown_population": {
            "leaves": len(unknown),
            "functions": len({(r["library"], r["file"], r["function"]) for r in unknown}),
            "families": table(unknown),
        },
        "sole_population": {
            "leaves": len(sole),
            "functions": len({(r["library"], r["file"], r["function"]) for r in sole}),
            "families": table(sole),
        },
        "all_status_population": {
            "leaves": len(leaves),
            "families": table(leaves),
        },
    }
    print(json.dumps({k: v for k, v in summary.items()}, indent=2))
    if out:
        out.write_text(json.dumps({"summary": summary, "uncovered": uncovered, "leaves": leaves}, indent=1) + "\n")
    return 0 if not uncovered else 1


if __name__ == "__main__":
    sys.exit(main())
