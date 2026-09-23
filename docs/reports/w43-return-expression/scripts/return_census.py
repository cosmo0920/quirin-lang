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
import gzip
import json
import pathlib
import sys


def read_text(path):
    """Plain or gzip-compressed text, so the archived inputs replay directly."""
    data = path.read_bytes()
    return (gzip.decompress(data) if path.name.endswith(".gz") else data).decode()

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


TYPE_OPERATORS = {"sizeof", "_Alignof", "__alignof__", "alignof"}


def strip_casts(tokens):
    """Remove any number of leading `( TYPE-NAME )` casts and grouping parens."""
    tokens = strip_parens(tokens)
    casts = 0
    while True:
        cast = leading_cast(tokens)
        if cast is None:
            return tokens, casts
        tokens = strip_parens(tokens[cast + 1:])
        casts += 1


def is_type_operator_constant(tokens):
    """Every operand is `sizeof`/`_Alignof` `( … )`, a numeric literal, a cast
    or an arithmetic operator: an expression C evaluates at translation time
    (no variable-length array can appear in the pinned corpus's spellings is
    NOT assumed here; the family is only a sizing bucket)."""
    index = 0
    seen = False
    while index < len(tokens):
        token = tokens[index]
        if token in TYPE_OPERATORS and index + 1 < len(tokens) and tokens[index + 1] == "(":
            end = matching(tokens, index + 1)
            if end is None:
                return False
            index = end + 1
            seen = True
            continue
        if token == "(":
            cast = leading_cast(tokens[index:])
            if cast is not None:
                index += cast + 1
                continue
        if token in ("(", ")", "+", "-", "*", "/") or token[:1].isdigit():
            index += 1
            continue
        return False
    return seen


def access_obstacle(record, tokens):
    """The first reason, in the analyzer's own chain facts, an access-only
    expression is outside the admitted neutral families."""
    if "=" in tokens or "++" in tokens or "--" in tokens:
        return "writes inside the expression"
    for chain in record["chains"]:
        if chain["base_kind"] == "CallResult":
            return "access through a call or cast result"
        if chain["base_may_bind"]:
            return "access through a base this body may bind"
        cause = chain["base_cause"]
        if cause != "Committed":
            return "base type the declaration reader will not commit to"
        if "Subscript" in chain["steps"]:
            return "subscript"
        if "Deref" in chain["steps"]:
            return "dereference"
        if chain["member_class"] not in ("Scalar", "RecordPointer", "SinglePointer"):
            return f"member typed {chain['member_class']}"
        if chain["member_class"] != "Scalar":
            return "pointer-valued member leaf"
    if "[" in tokens:
        return "subscript of a non-chain operand"
    if "*" in tokens or "&" in tokens:
        return "address-of or dereference of a name"
    return "unclassified operand"


def family(record):
    tokens = strip_parens(merge(record["tokens"]))
    calls = record["calls"]
    if not tokens:
        return "G. other", "empty"
    if "{" in tokens:
        return "G. other", "statement expression or compound literal"
    inner, casts = strip_casts(tokens)
    under = " under a cast" if casts else ""
    if is_type_operator_constant(tokens):
        return "D. type-operator constant", "sizeof/_Alignof over a type or object" + under
    if whole_call(inner):
        callee = inner[0]
        call = next((c for c in calls if c["callee"] == callee), None)
        if call is None:
            return "A. whole call, uncontracted callee", "unclassified callee" + under
        if callee.startswith(("__builtin_", "__atomic_", "__sync_")):
            return "A. whole call, uncontracted callee", "compiler builtin" + under
        if not call["contracted"]:
            where = "defined in this unit" if call["defined_in_unit"] else "declared elsewhere"
            return "A. whole call, uncontracted callee", where + under
        return "B. whole call, contracted callee", f"result {call['result_mode']}" + under
    if top_level(tokens, lambda i, t: t == "?"):
        return ("C. compound expression with a call" if calls else "E. conditional or short-circuit, no call"), "conditional ?:"
    if top_level(tokens, lambda i, t: t in ("&&", "||")):
        return ("C. compound expression with a call" if calls else "E. conditional or short-circuit, no call"), "short-circuit && / ||"
    if top_level(tokens, lambda i, t: t == ","):
        return "G. other", "comma operator"
    if calls:
        contracted = all(c["contracted"] for c in calls)
        return "C. compound expression with a call", (
            "every call contracted" if contracted else "an uncontracted call"
        )
    return "F. access expression, no call", access_obstacle(record, tokens)


def main() -> int:
    census_path = pathlib.Path(sys.argv[1])
    diagnostics = json.loads(read_text(pathlib.Path(sys.argv[2])))
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
    records = [json.loads(line) for line in read_text(census_path).splitlines() if line]
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
        joined.setdefault(key, set()).add(
            (tuple(record["tokens"]), tuple((c["callee"], c["contracted"]) for c in record["calls"]))
        )
        # The published analysis is the unit's last pass over the body, so the
        # last record for a key is the one whose contract environment the
        # published verdict used; disagreement between passes is counted.
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
        name, detail = family(record)
        callee_status = None
        if name.startswith("A.") and detail.startswith("defined in this unit"):
            callee = strip_casts(strip_parens(merge(record["tokens"])))[0][0]
            callee_status = status.get((key[0], key[1], callee), "not projected")
        for _ in range(multiplicity):
            leaves.append(
                {
                    "library": key[0],
                    "file": key[1],
                    "function": key[2],
                    "line": key[3],
                    "status": status[function_key],
                    "sole": distinct == {RETURN},
                    "family": name,
                    "detail": detail,
                    "callee_status": callee_status,
                    "expression": " ".join(record["tokens"]),
                }
            )

    def table(rows):
        out = []
        for name in sorted({row["family"] for row in rows}):
            members = [row for row in rows if row["family"] == name]
            details = collections.Counter(row["detail"] for row in members)
            out.append(
                {
                    "family": name,
                    "leaves": len(members),
                    "functions": len({(r["library"], r["file"], r["function"]) for r in members}),
                    "details": [
                        {
                            "detail": detail,
                            "leaves": count,
                            "functions": len(
                                {
                                    (r["library"], r["file"], r["function"])
                                    for r in members
                                    if r["detail"] == detail
                                }
                            ),
                        }
                        for detail, count in details.most_common()
                    ],
                }
            )
        return out

    def callee_statuses(rows):
        return dict(
            collections.Counter(
                row["callee_status"] for row in rows if row["callee_status"] is not None
            )
        )

    unknown = [row for row in leaves if row["status"] == "Unknown"]
    sole = [row for row in unknown if row["sole"]]
    summary = {
        "census_records": len(records),
        "census_unit_ordinals": len(per_seq),
        "unjoined_unit_ordinals": len(unjoined_seq),
        "records_joined_to_no_published_leaf": unmatched_records,
        "published_leaves": sum(published.values()),
        "published_leaves_uncovered": len(uncovered),
        "published_leaves_whose_passes_disagree": ambiguous,
        "unknown_population": {
            "leaves": len(unknown),
            "functions": len({(r["library"], r["file"], r["function"]) for r in unknown}),
            "families": table(unknown),
            "defined_callee_status": callee_statuses(unknown),
        },
        "sole_population": {
            "leaves": len(sole),
            "functions": len({(r["library"], r["file"], r["function"]) for r in sole}),
            "families": table(sole),
            "defined_callee_status": callee_statuses(sole),
        },
        "all_status_population": {
            "leaves": len(leaves),
            "families": table(leaves),
        },
    }
    for key in ("census_records", "census_unit_ordinals", "unjoined_unit_ordinals",
                "records_joined_to_no_published_leaf", "published_leaves",
                "published_leaves_uncovered", "published_leaves_whose_passes_disagree"):
        print(f"{key}: {summary[key]}")
    for population in ("unknown_population", "sole_population"):
        block = summary[population]
        print(f"\n{population}: {block['leaves']} leaves / {block['functions']} functions")
        print(f"{'leaves':>7} {'funcs':>6}  family / detail")
        for row in sorted(block["families"], key=lambda r: -r["leaves"]):
            print(f"{row['leaves']:7} {row['functions']:6}  {row['family']}")
            for detail in row["details"]:
                print(f"{detail['leaves']:7} {detail['functions']:6}      {detail['detail']}")
        print(f"  status of the defined-in-unit callee: {block['defined_callee_status']}")
    if out:
        out.write_text(json.dumps({"summary": summary, "uncovered": uncovered, "leaves": leaves}, indent=1) + "\n")
    return 0 if not uncovered else 1


if __name__ == "__main__":
    sys.exit(main())
