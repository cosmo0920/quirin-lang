#!/usr/bin/env python3
"""Card W44-COLLECTION-ALLOCATION: the `sizeof` operand census.

Usage: sizeof_census.py RUN_DIR UNIT_TEXTS_DIR CONTRACTS_JSON OUT_JSON OUT_TXT

RUN_DIR holds one audit run (`resource-diagnostics.json`). UNIT_TEXTS_DIR is
the output of `w43-libc-contracts/scripts/unit_texts.py` (one analysed
`clang -E -P` text per unit; its line numbers are the diagnostics' `line`).
CONTRACTS_JSON is the run's `resource-diagnostics.contracts.json` (or `.gz`),
read only for which callees have an owned result.

It measures, over every projected function of every paired unit:

(a) every `sizeof` occurrence, by operand shape: a parenthesised type name
    (scalar keywords, a `struct`/`union`/`enum` tag, a typedef name, any of
    them with `*`), `*NAME` / `**NAME` (with or without parentheses),
    `NAME[LITERAL]`, a bare or parenthesised name, a member access, and
    everything else;
(b) the allocation forms: a statement `DEST = [cast] CALL(...)` or a
    declaration initialised by one, whose callee has an owned result and whose
    arguments carry a `sizeof`, by operand shape and by whether the operand
    names DEST itself (`v = malloc(n * sizeof(*v))`);
(c) the leaves the operand rule could reach: `indirect memory access` leaves
    whose line's only access is a dereference or literal-subscript `sizeof`
    operand, and `C operator \`sizeof\` requires type/operand analysis` leaves,
    each by the operand shapes on its line, with the Unknown functions they
    sit on and the functions blocked by nothing else.

A forecast instrument reading source tokens, never a verdict: the analyzer's
own classification of each statement is the authority.
"""

import collections
import gzip
import importlib.util
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve()
SPEC = importlib.util.spec_from_file_location(
    "collection_census",
    HERE.parents[2] / "w43-indexed-resource-collections/scripts/collection_census.py",
)
base = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(base)

NAME = re.compile(r"[A-Za-z_]\w*$")
SCALAR_KEYWORDS = {
    "char", "short", "int", "long", "signed", "unsigned", "float", "double", "_Bool",
    "const", "volatile", "void",
}
ACCESS_DESCRIPTION = "indirect memory access is outside the resource projection"
OPERATOR_DESCRIPTION = "C operator `sizeof` requires type/operand analysis"
DEREFERENCE_SHAPES = {"(*NAME)", "*NAME", "(**NAME)", "**NAME", "(NAME[LIT])", "NAME[LIT]"}


def operand(tokens, index, limit):
    """(shape, name, end) of the operand of the `sizeof` at `index`."""
    values = lambda a, b: [tokens[p][0] for p in range(a, b)]
    start = index + 1
    if start > limit:
        return "other", None, start
    if tokens[start][0] == "(":
        close = base.matching(tokens, start, "(", ")")
        if close is None:
            return "other", None, start
        inner = values(start + 1, close)
        end = close + 1
        # `sizeof ( ... ) [ ... ]` or `sizeof (x) -> f`: a postfix applies to a
        # parenthesised expression; classify the whole as other.
        if end <= limit and tokens[end][0] in ("[", "->", "."):
            return "other", None, end
        return classify_parenthesised(inner), name_of(inner), end
    # unparenthesised unary-expression: stars then a postfix expression
    cursor = start
    stars = 0
    while cursor <= limit and tokens[cursor][0] == "*":
        stars += 1
        cursor += 1
    if cursor > limit or not NAME.match(tokens[cursor][0]):
        return "other", None, cursor
    name = tokens[cursor][0]
    cursor += 1
    if cursor <= limit and tokens[cursor][0] == "[":
        close = base.matching(tokens, cursor, "[", "]")
        index_values = values(cursor + 1, close) if close else []
        if stars == 0 and close and len(index_values) == 1 and re.fullmatch(r"[0-9]\w*", index_values[0]):
            if close + 1 <= limit and tokens[close + 1][0] in ("[", "->", ".", "("):
                return "other", name, close + 1
            return "NAME[LIT]", name, close + 1
        return "other", name, (close or cursor) + 1
    if cursor <= limit and tokens[cursor][0] in ("->", ".", "("):
        return "member-or-call", name, cursor
    if stars == 0:
        return "NAME", name, cursor
    return ("*NAME" if stars == 1 else "**NAME" if stars == 2 else "other"), name, cursor


def name_of(inner):
    names = [v for v in inner if NAME.match(v) and v not in SCALAR_KEYWORDS and v not in ("struct", "union", "enum")]
    return names[-1] if names else None


def classify_parenthesised(inner):
    if not inner:
        return "other"
    if inner[0] in ("struct", "union", "enum") and len(inner) >= 2 and NAME.match(inner[1]):
        rest = inner[2:]
        if not rest:
            return "(struct T)"
        if all(v == "*" for v in rest):
            return "(struct T *)"
        return "other"
    if all(v in SCALAR_KEYWORDS for v in inner):
        return "(scalar type)"
    stripped = [v for v in inner if v not in ("const", "volatile")]
    if all(v in SCALAR_KEYWORDS or v == "*" for v in stripped) and "*" in stripped:
        return "(scalar type *)"
    if stripped[0] == "*" and len(stripped) == 2 and NAME.match(stripped[1]):
        return "(*NAME)"
    if stripped[:2] == ["*", "*"] and len(stripped) == 3 and NAME.match(stripped[2]):
        return "(**NAME)"
    if len(stripped) == 4 and NAME.match(stripped[0]) and stripped[1] == "[" and stripped[3] == "]" and re.fullmatch(r"[0-9]\w*", stripped[2]):
        return "(NAME[LIT])"
    if len(stripped) == 1 and NAME.match(stripped[0]):
        # a typedef name or an object name: decided by the unit's typedefs
        return "(NAME)"
    if len(stripped) >= 2 and NAME.match(stripped[0]) and all(v == "*" for v in stripped[1:]):
        return "(NAME *)"
    if len(stripped) >= 3 and NAME.match(stripped[0]) and stripped[1] in ("->", "."):
        return "(member access)"
    if stripped[0] == "*" and len(stripped) >= 3 and NAME.match(stripped[1]) and stripped[2] in ("->", "."):
        return "(*member access)"
    if stripped[0] == "\"" or stripped[0].startswith('"'):
        return "(string literal)"
    return "other"


def typedef_names(tokens):
    names = set()
    for index, (value, _) in enumerate(tokens):
        if value != "typedef":
            continue
        end = index
        depth = 0
        while end < len(tokens):
            v = tokens[end][0]
            depth += v in "({["
            depth -= v in ")}]"
            if v == ";" and depth == 0:
                break
            end += 1
        if end - 1 > index and NAME.match(tokens[end - 1][0]):
            names.add(tokens[end - 1][0])
    return names


def declared_stars(tokens, head, open_, close, name):
    """Stars on each declarator of NAME in the parameter list and body, and
    whether any carries a `[` (an array: a possible VLA)."""
    found = []
    for index in range(head, close):
        if tokens[index][0] != name:
            continue
        previous = tokens[index - 1][0]
        if previous not in ("*", ",", "(") and not NAME.match(previous):
            continue
        # walk back over stars
        cursor = index - 1
        stars = 0
        while tokens[cursor][0] == "*":
            stars += 1
            cursor -= 1
        head_value = tokens[cursor][0]
        if not (NAME.match(head_value) and head_value not in ("return", "sizeof", "case", "goto")):
            continue
        after = tokens[index + 1][0] if index + 1 < len(tokens) else ""
        if after not in (";", ",", "=", ")", "["):
            continue
        # a call argument `f(a, name)` also matches `, name )`; require a
        # type-ish token before the stars.
        if head_value in ("if", "while", "for", "switch") or stars == 0 and tokens[cursor - 1][0] in ("(", ","):
            continue
        found.append((stars, after == "["))
    return found


def main() -> int:
    run, texts = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    contracts_path = pathlib.Path(sys.argv[3])
    out_json, out_txt = pathlib.Path(sys.argv[4]), pathlib.Path(sys.argv[5])
    diagnostics = json.loads((run / "resource-diagnostics.json").read_text())
    opener = gzip.open if str(contracts_path).endswith(".gz") else open
    with opener(contracts_path, "rt") as handle:
        contracts = base.load_contracts(json.load(handle)["sources"])
    shapes = collections.Counter()
    shapes_by_status = collections.Counter()
    allocations = collections.Counter()
    allocation_rows = []
    access_leaves = collections.Counter()
    operator_leaves = collections.Counter()
    access_functions = set()
    operator_functions = set()
    per_function = []
    paired = 0
    for unit in diagnostics["units"]:
        key = f"{unit['library']}__{unit['file'].replace('/', '__')}.i"
        path = texts / key
        if not path.exists():
            continue
        paired += 1
        text = path.read_text(errors="replace")
        lines_of = text.split("\n")
        tokens = base.tokenize(text)
        bodies = base.function_bodies(tokens)
        typedefs = typedef_names(tokens)
        for function in unit.get("functions", []):
            name = function["function"]
            if name not in bodies:
                continue
            head, open_, close = bodies[name]
            status = function.get("status")
            occurrences = []
            for index in range(open_, close):
                if tokens[index][0] != "sizeof":
                    continue
                shape, operand_name, end = operand(tokens, index, close)
                if shape == "(NAME)" and operand_name in typedefs:
                    shape = "(typedef)"
                elif shape == "(NAME *)" and operand_name is not None:
                    shape = "(typedef *)"
                declarations = []
                if shape in DEREFERENCE_SHAPES | {"(NAME)", "NAME"} and operand_name:
                    declarations = declared_stars(tokens, head, open_, close, operand_name)
                occurrences.append({
                    "shape": shape, "name": operand_name, "line": tokens[index][1],
                    "declared": declarations,
                })
                shapes[shape] += 1
                shapes_by_status[(status, shape)] += 1
            # (b) allocation forms
            for index, call_close in base.Function.calls(type("F", (), {"open": open_, "close": close, "value": lambda self, i: tokens[i][0], "tokens": tokens})()):
                callee = tokens[index][0]
                contract = contracts.get(callee)
                if not (contract and contract["owned"]):
                    continue
                inside = [o for o in occurrences if tokens[index][1] <= o["line"] <= tokens[call_close][1]]
                sizeofs = [p for p in range(index, call_close) if tokens[p][0] == "sizeof"]
                if not sizeofs:
                    continue
                cursor = index - 1
                if tokens[cursor][0] == ")":
                    depth = 0
                    while cursor > open_:
                        depth += tokens[cursor][0] == ")"
                        depth -= tokens[cursor][0] == "("
                        if depth == 0:
                            break
                        cursor -= 1
                    cursor -= 1
                destination = None
                if tokens[cursor][0] == "=" and NAME.match(tokens[cursor - 1][0]):
                    destination = tokens[cursor - 1][0]
                for position in sizeofs:
                    shape, operand_name, _ = operand(tokens, position, call_close)
                    if shape == "(NAME)" and operand_name in typedefs:
                        shape = "(typedef)"
                    self_ref = destination is not None and operand_name == destination
                    stars = declared_stars(tokens, head, open_, close, destination) if destination else []
                    allocations[(shape, "self" if self_ref else "other")] += 1
                    allocation_rows.append({
                        "unit": key, "function": name, "status": status, "line": tokens[index][1],
                        "callee": callee, "destination": destination, "shape": shape,
                        "self_referential": self_ref, "destination_stars": sorted({s for s, _ in stars}),
                    })
            # (c) leaves
            leaves = [d for d in function.get("diagnostics", []) if d.get("category") != "leak"]
            by_line = collections.defaultdict(list)
            for occurrence in occurrences:
                by_line[occurrence["line"]].append(occurrence["shape"])
            reachable = []
            for leaf in leaves:
                line = leaf["line"]
                line_shapes = by_line.get(line, [])
                if not line_shapes:
                    continue
                if leaf["description"] == ACCESS_DESCRIPTION:
                    if not any(s in DEREFERENCE_SHAPES for s in line_shapes):
                        continue
                    source = lines_of[line - 1] if 0 < line <= len(lines_of) else ""
                    rest = re.sub(r"sizeof\s*(?:\(\s*\*{1,2}\s*[A-Za-z_]\w*\s*\)|\*{1,2}\s*[A-Za-z_]\w*|\(\s*[A-Za-z_]\w*\s*\[\s*[0-9]\w*\s*\]\s*\)|[A-Za-z_]\w*\s*\[\s*[0-9]\w*\s*\])", " 1 ", source)
                    if re.search(r"->|\[|(?<![\w)\s])\s*\*\s*[A-Za-z_(]|\.[A-Za-z_]|\(\s*\*", rest):
                        continue
                    access_leaves[(status, ",".join(sorted(set(line_shapes))))] += 1
                    reachable.append(leaf)
                    if status == "Unknown":
                        access_functions.add((key, name))
                elif leaf["description"] == OPERATOR_DESCRIPTION:
                    operator_leaves[(status, ",".join(sorted(set(line_shapes))))] += 1
                    reachable.append(leaf)
                    if status == "Unknown":
                        operator_functions.add((key, name))
            if reachable or occurrences:
                per_function.append({
                    "unit": key, "function": name, "status": status,
                    "occurrences": occurrences,
                    "reachable_leaves": [{"line": l["line"], "description": l["description"]} for l in reachable],
                    "other_leaves": len(leaves) - len(reachable),
                    "sole": status == "Unknown" and bool(reachable) and len(reachable) == len(leaves),
                })
    sole_access = sum(1 for r in per_function if r["sole"] and all(l["description"] == ACCESS_DESCRIPTION for l in r["reachable_leaves"]))
    sole_operator = sum(1 for r in per_function if r["sole"] and all(l["description"] == OPERATOR_DESCRIPTION for l in r["reachable_leaves"]))
    sole_either = sum(1 for r in per_function if r["sole"])
    summary = {
        "paired_units": paired,
        "sizeof_occurrences_by_shape": dict(shapes.most_common()),
        "sizeof_occurrences_by_status_and_shape": {f"{k[0]}/{k[1]}": v for k, v in sorted(shapes_by_status.items())},
        "owned_result_allocations_with_sizeof_by_shape": {f"{k[0]}/{k[1]}": v for k, v in sorted(allocations.items(), key=lambda i: -i[1])},
        "allocation_self_dereference_by_destination_stars": dict(collections.Counter(
            f"{r['shape']}/stars={r['destination_stars']}" for r in allocation_rows
            if r["self_referential"] and r["shape"] in DEREFERENCE_SHAPES).most_common()),
        "access_leaves_blocked_only_by_a_dereference_operand": {f"{k[0]}/{k[1]}": v for k, v in sorted(access_leaves.items(), key=lambda i: -i[1])},
        "access_leaves_total": sum(access_leaves.values()),
        "access_leaves_on_unknown": sum(v for k, v in access_leaves.items() if k[0] == "Unknown"),
        "access_unknown_functions": len(access_functions),
        "operator_leaves_by_line_shapes": {f"{k[0]}/{k[1]}": v for k, v in sorted(operator_leaves.items(), key=lambda i: -i[1])},
        "operator_leaves_total": sum(operator_leaves.values()),
        "operator_leaves_on_unknown": sum(v for k, v in operator_leaves.items() if k[0] == "Unknown"),
        "operator_unknown_functions": len(operator_functions),
        "unknown_functions_blocked_only_by_access_operand_leaves": sole_access,
        "unknown_functions_blocked_only_by_operator_leaves": sole_operator,
        "unknown_functions_blocked_only_by_either": sole_either,
    }
    out_json.write_text(json.dumps({"summary": summary, "allocations": allocation_rows, "functions": per_function}, indent=1) + "\n")
    lines = ["W44-COLLECTION-ALLOCATION sizeof census", ""]
    for key, value in summary.items():
        lines.append(f"{key}:")
        if isinstance(value, dict):
            for inner, count in value.items():
                lines.append(f"  {inner}: {count}")
        else:
            lines.append(f"  {value}")
    lines.append("")
    lines.append("Unknown functions blocked only by reachable sizeof leaves:")
    for record in per_function:
        if record["sole"]:
            lines.append(f"  {record['unit']}::{record['function']} " + ", ".join(
                f"L{l['line']} {'access' if l['description'] == ACCESS_DESCRIPTION else 'operator'}" for l in record["reachable_leaves"]))
    lines.append("")
    lines.append("self-referential dereference allocations into a `T **` destination:")
    for row in allocation_rows:
        if row["self_referential"] and row["shape"] in DEREFERENCE_SHAPES and 2 in row["destination_stars"]:
            lines.append(f"  {row['status']:8} {row['unit']}::{row['function']} L{row['line']} {row['destination']} = {row['callee']}(... sizeof {row['shape']} ...)")
    out_txt.write_text("\n".join(lines) + "\n")
    print("\n".join(lines[:120]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
