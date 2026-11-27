#!/usr/bin/env python3
"""Card W43-INDEXED-RESOURCE-COLLECTIONS: the indexed-resource census.

Usage: collection_census.py RUN_DIR UNIT_TEXTS_DIR OUT_JSON OUT_TXT

RUN_DIR holds one audit run (`resource-diagnostics.json`, and the run's
`contracts` artifact is read from RUN_DIR/contracts.json when present, else
from the archived baseline passed as `--contracts PATH`). UNIT_TEXTS_DIR is the
output of `w43-libc-contracts/scripts/unit_texts.py`: one analysed `clang -E -P`
text per unit, whose line numbers are the diagnostics' `line` coordinate.

It measures, over every projected function of every paired unit:

(a) stores of an owned or derived pointer into an array element
    (`a[i] = ACQUIRE(...)`, `a[i] = p` where `p` was bound by an acquiring
    call in the same body), with the index shape;
(b) releases of an array element (`RELEASE(a[i])`, `close(fds[i])`);
(c) the loops containing (a) or (b), classified as a fill loop, a release loop
    or a partial-cleanup loop (a release loop whose bound is another loop's
    index), and whether the loop header is the canonical counted form
    `for (I = 0; I < B; I++)`;
(d) the functions these block: a leaf is a *collection leaf* when its line is
    the line of one of (a), (b), an element null test, a loop header of (c),
    or the array's own allocation; a function *would clear fully* when it is
    Unknown and every leaf it holds is a collection leaf.

An *acquiring* call is a call to a declaration of the run's contract sources
whose result is `resource_return_owned`; a *releasing* call has a
`resource_released` or `resource_consumed` parameter (inherited declarations
are resolved through `contract inherit NS::NAME`). Function-local W4 summaries
are not visible here, so the census under-counts acquisitions through them and
says so. The census reads source tokens, not the analyzer's own view; it is a
forecast instrument, never a verdict.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

TOKEN = re.compile(
    r"""
    (?P<ws>\s+)
  | (?P<str>"(?:\\.|[^"\\])*")
  | (?P<chr>'(?:\\.|[^'\\])*')
  | (?P<num>\.?[0-9](?:[eEpP][+-]|[0-9A-Za-z_.])*)
  | (?P<id>[A-Za-z_][A-Za-z0-9_]*)
  | (?P<op>->|\+\+|--|<<=|>>=|<=|>=|==|!=|&&|\|\||\+=|-=|\*=|/=|%=|&=|\|=|\^=|<<|>>|\.\.\.|[^\s])
    """,
    re.VERBOSE,
)
KEYWORDS = {
    "if", "else", "for", "while", "do", "switch", "case", "default", "return", "break",
    "continue", "goto", "sizeof", "struct", "union", "enum", "typedef", "static", "const",
    "volatile", "unsigned", "signed", "int", "long", "short", "char", "void", "float",
    "double", "_Bool", "extern", "register", "inline", "__inline", "__inline__",
    "restrict", "__restrict", "_Alignof", "__typeof__", "typeof",
}
DESCRIPTOR_RELEASES = {"close", "closesocket"}
NULLS = {"NULL", "0", "((void*)0)"}


def tokenize(text):
    tokens = []
    line = 1
    for match in TOKEN.finditer(text):
        kind = match.lastgroup
        value = match.group(0)
        if kind == "ws":
            line += value.count("\n")
            continue
        tokens.append((value, line))
        line += value.count("\n")
    return tokens


def matching(tokens, start, open_, close):
    depth = 0
    for index in range(start, len(tokens)):
        if tokens[index][0] == open_:
            depth += 1
        elif tokens[index][0] == close:
            depth -= 1
            if depth == 0:
                return index
    return None


def function_bodies(tokens):
    """File-scope `NAME ( ... ) {` definitions: name -> (open, close) token indices."""
    bodies = {}
    depth = 0
    index = 0
    while index < len(tokens):
        text = tokens[index][0]
        if text == "{":
            depth += 1
        elif text == "}":
            depth -= 1
        elif depth == 0 and re.fullmatch(r"[A-Za-z_]\w*", text) and index + 1 < len(tokens) and tokens[index + 1][0] == "(":
            close = matching(tokens, index + 1, "(", ")")
            if close is not None and close + 1 < len(tokens) and tokens[close + 1][0] == "{":
                end = matching(tokens, close + 1, "{", "}")
                if end is not None:
                    bodies.setdefault(text, (index, close + 1, end))
                    index = end + 1
                    continue
        index += 1
    return bodies


def statement_end(tokens, start, limit):
    """Index of the last token of the statement beginning at `start`."""
    if start > limit:
        return limit
    text = tokens[start][0]
    if text == "{":
        return matching(tokens, start, "{", "}") or limit
    if text in ("if", "for", "while", "switch") and start + 1 <= limit and tokens[start + 1][0] == "(":
        close = matching(tokens, start + 1, "(", ")")
        if close is None:
            return limit
        if text == "while" and close + 1 <= limit and tokens[close + 1][0] == ";":
            return close + 1
        end = statement_end(tokens, close + 1, limit)
        if text == "if" and end + 1 <= limit and tokens[end + 1][0] == "else":
            end = statement_end(tokens, end + 2, limit)
        return end
    if text == "do":
        end = statement_end(tokens, start + 1, limit)
        # while ( ... ) ;
        if end + 1 <= limit and tokens[end + 1][0] == "while":
            close = matching(tokens, end + 2, "(", ")")
            if close is not None:
                return min(close + 1, limit)
        return end
    depth = 0
    for index in range(start, limit + 1):
        value = tokens[index][0]
        if value in "([{":
            depth += 1
        elif value in ")]}":
            depth -= 1
        elif value == ";" and depth == 0:
            return index
    return limit


def load_contracts(sources):
    """name -> {'owned': bool, 'released': [positions]} from contract sources."""
    declared = {}
    inherits = {}
    for text in sources.values():
        text = re.sub(r"//[^\n]*", "", text)
        for match in re.finditer(r"extern\s+fn\s+([A-Za-z_]\w*)\s*\(([^)]*)\)[^;{]*?(contract\s*\{.*?\}\s*;|contract\s+inherit\s+([A-Za-z_]\w*)::([A-Za-z_]\w*)\s*;)", text, re.S):
            name, params, body = match.group(1), match.group(2), match.group(3)
            names = [p.split(":")[0].strip() for p in params.split(",") if p.strip()]
            if match.group(5):
                inherits[name] = (match.group(5), len(names))
                continue
            released = []
            for rel in re.finditer(r"resource_(?:released|consumed)\(\s*([A-Za-z_]\w*)", body):
                if rel.group(1) in names:
                    released.append(names.index(rel.group(1)))
            declared[name] = {"owned": "resource_return_owned" in body, "released": sorted(released), "arity": len(names)}
    for name, (parent, arity) in inherits.items():
        if parent in declared:
            declared[name] = dict(declared[parent], arity=arity)
    return declared


def split_arguments(tokens, open_index, close_index):
    arguments = []
    current = []
    depth = 0
    for index in range(open_index + 1, close_index):
        value = tokens[index][0]
        if value in "([{":
            depth += 1
        elif value in ")]}":
            depth -= 1
        if value == "," and depth == 0:
            arguments.append(current)
            current = []
        else:
            current.append(index)
    if current or arguments:
        arguments.append(current)
    return arguments


def strip_casts(values):
    """Drop leading parenthesised casts and outer parentheses from a token list."""
    changed = True
    while changed and values:
        changed = False
        if values[0] == "(" and values[-1] == ")":
            depth = 0
            balanced = True
            for position, value in enumerate(values):
                depth += value == "("
                depth -= value == ")"
                if depth == 0 and position != len(values) - 1:
                    balanced = False
                    break
            if balanced:
                values = values[1:-1]
                changed = True
                continue
        if values[0] == "(":
            depth = 0
            for position, value in enumerate(values):
                depth += value == "("
                depth -= value == ")"
                if depth == 0:
                    inner = values[1:position]
                    if inner and all(re.fullmatch(r"[A-Za-z_]\w*|\*", v) for v in inner) and position + 1 < len(values):
                        values = values[position + 1:]
                        changed = True
                    break
    return values


def element_place(values):
    """`NAME [ IDX ]` (after casts) -> (name, index tokens) or None."""
    values = strip_casts(values)
    if len(values) >= 4 and re.fullmatch(r"[A-Za-z_]\w*", values[0]) and values[1] == "[" and values[-1] == "]":
        depth = 0
        for position in range(1, len(values)):
            depth += values[position] == "["
            depth -= values[position] == "]"
            if depth == 0:
                if position == len(values) - 1:
                    return values[0], values[2:-1]
                return None
    return None


def index_shape(index):
    if len(index) == 1 and re.fullmatch(r"[0-9].*", index[0]):
        return "constant"
    if len(index) == 1 and re.fullmatch(r"[A-Za-z_]\w*", index[0]):
        return "name"
    if len(index) == 2 and index[1] in ("++", "--") and re.fullmatch(r"[A-Za-z_]\w*", index[0]):
        return "post-increment"
    return "expression"


class Function:
    def __init__(self, unit, name, tokens, span, contracts):
        self.unit, self.name, self.tokens = unit, name, tokens
        self.head, self.open, self.close = span
        self.contracts = contracts
        self.stores = []
        self.releases = []
        self.null_tests = []
        self.loops = []
        self.allocations = []
        self.owned_names = set()
        self.scan()

    def value(self, index):
        return self.tokens[index][0]

    def line(self, index):
        return self.tokens[index][1]

    def calls(self):
        for index in range(self.open, self.close):
            if re.fullmatch(r"[A-Za-z_]\w*", self.value(index)) and self.value(index + 1) == "(" and self.value(index) not in KEYWORDS:
                close = matching(self.tokens, index + 1, "(", ")")
                if close is not None:
                    yield index, close

    def scan(self):
        tokens = self.tokens
        # names bound by an acquiring call: `p = ACQ(` or `T *p = ACQ(`
        for index, close in self.calls():
            contract = self.contracts.get(self.value(index))
            if contract and contract["owned"]:
                cursor = index - 1
                # walk back over a cast
                if self.value(cursor) == ")":
                    depth = 0
                    while cursor > self.open:
                        depth += self.value(cursor) == ")"
                        depth -= self.value(cursor) == "("
                        if depth == 0:
                            break
                        cursor -= 1
                    cursor -= 1
                if self.value(cursor) == "=" and re.fullmatch(r"[A-Za-z_]\w*", self.value(cursor - 1)):
                    name = self.value(cursor - 1)
                    self.owned_names.add(name)
                    # array allocation: the name is declared or used as `T **name` / subscripted later
                    self.allocations.append((name, self.line(index)))
        # (a) element stores `NAME [ IDX ] = RHS ;`
        for index in range(self.open + 1, self.close):
            if self.value(index) != "=":
                continue
            left_end = index - 1
            if self.value(left_end) != "]":
                continue
            depth = 0
            cursor = left_end
            while cursor > self.open:
                depth += self.value(cursor) == "]"
                depth -= self.value(cursor) == "["
                if depth == 0:
                    break
                cursor -= 1
            base = cursor - 1
            if not re.fullmatch(r"[A-Za-z_]\w*", self.value(base)) or self.value(base - 1) in (".", "->", "]"):
                continue
            if self.value(base - 1) not in (";", "{", "}", ")", "else", ":"):
                continue
            end = statement_end(tokens, base, self.close)
            rhs = [self.value(position) for position in range(index + 1, end)]
            index_values = [self.value(position) for position in range(cursor + 1, left_end)]
            rhs_stripped = strip_casts(rhs)
            kind = "other"
            callee = None
            if rhs_stripped and re.fullmatch(r"[A-Za-z_]\w*", rhs_stripped[0]) and len(rhs_stripped) > 1 and rhs_stripped[1] == "(":
                callee = rhs_stripped[0]
                contract = self.contracts.get(callee)
                if contract and contract["owned"]:
                    kind = "acquire"
                elif contract:
                    kind = "contracted-non-owning-call"
                else:
                    kind = "uncontracted-call"
            elif len(rhs_stripped) == 1 and rhs_stripped[0] in NULLS:
                kind = "null"
            elif len(rhs_stripped) == 1 and re.fullmatch(r"[A-Za-z_]\w*", rhs_stripped[0]):
                kind = "owned-name" if rhs_stripped[0] in self.owned_names else "name"
            self.stores.append({
                "base": self.value(base), "index": " ".join(index_values),
                "index_shape": index_shape(index_values), "rhs": kind, "callee": callee,
                "line": self.line(base), "token": base,
            })
        # (b) element releases
        for index, close in self.calls():
            name = self.value(index)
            contract = self.contracts.get(name)
            positions = []
            if contract:
                positions = contract["released"]
            elif name in DESCRIPTOR_RELEASES:
                positions = [0]
            if not positions:
                continue
            arguments = split_arguments(self.tokens, index + 1, close)
            for position in positions:
                if position >= len(arguments):
                    continue
                place = element_place([self.value(t) for t in arguments[position]])
                if place:
                    self.releases.append({
                        "callee": name, "base": place[0], "index": " ".join(place[1]),
                        "index_shape": index_shape(place[1]), "descriptor": name in DESCRIPTOR_RELEASES,
                        "line": self.line(index), "token": index,
                    })
        # element null tests: `if ( ! NAME [ I ] )`, `NAME [ I ] == NULL`
        for index in range(self.open, self.close):
            if self.value(index) in ("if", "while") and self.value(index + 1) == "(":
                close = matching(tokens, index + 1, "(", ")")
                values = [self.value(p) for p in range(index + 2, close)]
                bare = [v for v in values if v not in ("!", "(", ")")]
                for op in ("==", "!="):
                    if op in bare:
                        position = bare.index(op)
                        left, right = bare[:position], bare[position + 1:]
                        bare = left if right in (["NULL"], ["0"]) else right if left in (["NULL"], ["0"]) else bare
                place = element_place(bare) if bare else None
                if place:
                    self.null_tests.append({"base": place[0], "index": " ".join(place[1]), "line": self.line(index)})
        # (c) loops
        for index in range(self.open, self.close):
            value = self.value(index)
            if value not in ("for", "while", "do"):
                continue
            if value == "while" and index > self.open and self.previous_is_do_tail(index):
                continue
            if value == "do":
                end = statement_end(tokens, index, self.close)
                header_line = self.line(index)
                variable, bound, canonical = None, None, False
            else:
                if self.value(index + 1) != "(":
                    continue
                close = matching(tokens, index + 1, "(", ")")
                end = statement_end(tokens, index, self.close)
                header = [self.value(p) for p in range(index + 2, close)]
                header_line = self.line(index)
                variable, bound, canonical = self.loop_header(value, header)
            self.loops.append({"kind": value, "start": index, "end": end, "line": header_line,
                               "variable": variable, "bound": bound, "canonical": canonical})

    def previous_is_do_tail(self, index):
        # `} while ( ... ) ;` closing a do-statement
        return self.value(index - 1) == "}" and any(
            loop["kind"] == "do" and loop["end"] >= index for loop in self.loops
        )

    @staticmethod
    def loop_header(kind, header):
        if kind != "for":
            # while ( I < B )
            if len(header) == 3 and header[1] in ("<", "!=") and re.fullmatch(r"[A-Za-z_]\w*", header[0]):
                return header[0], header[2], False
            return None, None, False
        clauses = [[]]
        depth = 0
        for value in header:
            depth += value in "([{"
            depth -= value in ")]}"
            if value == ";" and depth == 0:
                clauses.append([])
            else:
                clauses[-1].append(value)
        if len(clauses) != 3:
            return None, None, False
        init, condition, step = clauses
        variable = None
        if len(condition) >= 3 and condition[1] in ("<", "!=", "<=") and re.fullmatch(r"[A-Za-z_]\w*", condition[0]):
            variable = condition[0]
            bound = " ".join(condition[2:])
            op = condition[1]
        else:
            return None, None, False
        init_ok = (init[-3:] == [variable, "=", "0"]) and (len(init) == 3 or re.fullmatch(r"[A-Za-z_]\w*", init[0]))
        step_ok = step in ([variable, "++"], ["++", variable], [variable, "+=", "1"])
        bound_ok = len(condition) == 3 and re.fullmatch(r"[A-Za-z_]\w*|[0-9]\w*", condition[2]) is not None
        return variable, bound, bool(init_ok and step_ok and bound_ok and op == "<")

    def classify(self):
        """Attach each loop's role."""
        roles = []
        for loop in self.loops:
            inside = lambda item: loop["start"] < item["token"] <= loop["end"]
            stores = [s for s in self.stores if inside(s) and s["rhs"] in ("acquire", "owned-name")]
            releases = [r for r in self.releases if inside(r)]
            if not stores and not releases:
                continue
            # the innermost loop owning the statement decides the role
            def owned_by_inner(item):
                return any(
                    other is not loop and loop["start"] < other["start"] and other["end"] <= loop["end"]
                    and other["start"] < item["token"] <= other["end"]
                    for other in self.loops
                )
            own_stores = [s for s in stores if not owned_by_inner(s)]
            own_releases = [r for r in releases if not owned_by_inner(r)]
            if not own_stores and not own_releases:
                continue
            if own_stores:
                role = "fill"
            else:
                enclosing_fill = [
                    other for other in self.loops
                    if other is not loop and other["variable"] and loop["bound"] == other["variable"]
                ]
                role = "partial-cleanup" if enclosing_fill else "release"
            indexed_by_variable = all(
                item["index"] == loop["variable"] for item in own_stores + own_releases
            ) if loop["variable"] else False
            roles.append({
                "line": loop["line"], "kind": loop["kind"], "role": role,
                "canonical": loop["canonical"], "variable": loop["variable"], "bound": loop["bound"],
                "indexed_by_variable": indexed_by_variable,
                "stores": len(own_stores), "releases": len(own_releases),
            })
        return roles


def main() -> int:
    run = pathlib.Path(sys.argv[1])
    texts = pathlib.Path(sys.argv[2])
    out_json, out_txt = pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4])
    contracts_path = None
    if "--contracts" in sys.argv:
        contracts_path = pathlib.Path(sys.argv[sys.argv.index("--contracts") + 1])
    diagnostics = json.loads((run / "resource-diagnostics.json").read_text())
    if contracts_path is None:
        contracts_path = run / "contracts.json"
    opener = gzip.open if str(contracts_path).endswith(".gz") else open
    with opener(contracts_path, "rt") as handle:
        contract_doc = json.load(handle)
    contracts = load_contracts(contract_doc["sources"])
    pairing = json.loads((texts / "pairing.json").read_text())
    rows = []
    loops = []
    per_function = []
    status_counts = collections.Counter()
    for unit in diagnostics["units"]:
        key = f"{unit['library']}__{unit['file'].replace('/', '__')}.i"
        path = texts / key
        if not path.exists():
            continue
        tokens = tokenize(path.read_text(errors="replace"))
        bodies = function_bodies(tokens)
        for function in unit.get("functions", []):
            name = function["function"] if "function" in function else function.get("name")
            if name not in bodies:
                continue
            analysed = Function(key, name, tokens, bodies[name], contracts)
            if not (analysed.stores or analysed.releases):
                continue
            roles = analysed.classify()
            collection_lines = set()
            element_stores = [s for s in analysed.stores if s["rhs"] in ("acquire", "owned-name")]
            element_bases = {s["base"] for s in element_stores} | {r["base"] for r in analysed.releases}
            for item in element_stores + analysed.releases:
                collection_lines.add(item["line"])
            for test in analysed.null_tests:
                if test["base"] in element_bases:
                    collection_lines.add(test["line"])
            for role in roles:
                collection_lines.add(role["line"])
            for name_, line in analysed.allocations:
                if name_ in element_bases:
                    collection_lines.add(line)
            leaves = [d for d in function.get("diagnostics", []) if d.get("category") != "leak"]
            status = function.get("status")
            collection_leaves = [d for d in leaves if d["line"] in collection_lines]
            other_leaves = [d for d in leaves if d["line"] not in collection_lines]
            record = {
                "unit": key, "function": name, "status": status,
                "stores": [{k: v for k, v in s.items() if k != "token"} for s in analysed.stores],
                "releases": [{k: v for k, v in r.items() if k != "token"} for r in analysed.releases],
                "loops": roles,
                "collection_leaves": len(collection_leaves),
                "other_leaves": len(other_leaves),
                "would_clear_fully": status == "Unknown" and not other_leaves and bool(collection_leaves),
                "collection_leaf_descriptions": sorted({d["description"] for d in collection_leaves}),
            }
            per_function.append(record)
            status_counts[status] += 1
    # aggregate
    store_kinds = collections.Counter()
    store_shapes = collections.Counter()
    release_shapes = collections.Counter()
    loop_roles = collections.Counter()
    loop_canonical = collections.Counter()
    blocked = collections.Counter()
    leaf_descriptions = collections.Counter()
    for record in per_function:
        for store in record["stores"]:
            store_kinds[store["rhs"]] += 1
            if store["rhs"] in ("acquire", "owned-name"):
                store_shapes[store["index_shape"]] += 1
        for release in record["releases"]:
            release_shapes[("descriptor" if release["descriptor"] else "pointer", release["index_shape"])] += 1
        for loop in record["loops"]:
            loop_roles[loop["role"]] += 1
            loop_canonical[(loop["role"], loop["canonical"] and loop["indexed_by_variable"])] += 1
        if record["status"] == "Unknown" and record["collection_leaves"]:
            blocked["unknown functions with a collection leaf"] += 1
            if record["would_clear_fully"]:
                blocked["would clear fully"] += 1
            for description in record["collection_leaf_descriptions"]:
                leaf_descriptions[description] += 1
        if any(s["rhs"] in ("acquire", "owned-name") for s in record["stores"]):
            blocked[f"functions with an owned element store ({record['status']})"] += 1
        if record["releases"]:
            blocked[f"functions with an element release ({record['status']})"] += 1
    # (e) the array allocation's own blocker: `sizeof(*NAME)` / `sizeof *NAME`
    # operands, which the lexical access gate reads as a dereference.
    sizeof_deref = re.compile(r"sizeof\s*(?:\(\s*\*\s*[A-Za-z_]\w*\s*\)|\*\s*[A-Za-z_]\w*)")
    other_access = re.compile(r"->|\[|(?<![\w)\s])\s*\*\s*[A-Za-z_(]|\.[A-Za-z_]")
    sizeof_leaves = 0
    sizeof_functions = set()
    sizeof_sole = set()
    for unit in diagnostics["units"]:
        key = f"{unit['library']}__{unit['file'].replace('/', '__')}.i"
        path = texts / key
        if not path.exists():
            continue
        lines_of = path.read_text(errors="replace").split("\n")
        for function in unit.get("functions", []):
            if function.get("status") != "Unknown":
                continue
            leaves = [d for d in function.get("diagnostics", []) if d.get("category") != "leak"]
            hits = []
            for leaf in leaves:
                if leaf["description"] != "indirect memory access is outside the resource projection":
                    continue
                source = lines_of[leaf["line"] - 1] if 0 < leaf["line"] <= len(lines_of) else ""
                if not sizeof_deref.search(source):
                    continue
                rest = sizeof_deref.sub("1", source)
                if other_access.search(rest):
                    continue
                hits.append(leaf)
            if hits:
                sizeof_leaves += len(hits)
                sizeof_functions.add((key, function["function"]))
                if len(hits) == len(leaves):
                    sizeof_sole.add((key, function["function"]))
    summary_sizeof = {
        "leaves_only_blocked_by_a_sizeof_dereference_operand": sizeof_leaves,
        "unknown_functions": len(sizeof_functions),
        "sole_blocked_functions": len(sizeof_sole),
    }
    summary = {
        "sizeof_dereference_operand": summary_sizeof,
        "functions_scanned_with_element_statements": len(per_function),
        "status_counts": dict(status_counts),
        "element_store_rhs": dict(store_kinds),
        "owned_store_index_shapes": dict(store_shapes),
        "release_shapes": {f"{k[0]}/{k[1]}": v for k, v in release_shapes.items()},
        "loop_roles": dict(loop_roles),
        "loop_canonical_and_indexed": {f"{k[0]}/{'canonical' if k[1] else 'other'}": v for k, v in loop_canonical.items()},
        "blocked": dict(blocked),
        "collection_leaf_descriptions": dict(leaf_descriptions.most_common()),
    }
    out_json.write_text(json.dumps({"summary": summary, "functions": per_function}, indent=1) + "\n")
    lines = ["W43-INDEXED-RESOURCE-COLLECTIONS census", ""]
    for key, value in summary.items():
        lines.append(f"{key}:")
        if isinstance(value, dict):
            for inner, count in sorted(value.items(), key=lambda item: -item[1] if isinstance(item[1], int) else 0):
                lines.append(f"  {inner}: {count}")
        else:
            lines.append(f"  {value}")
    lines.append("")
    lines.append("functions with an owned element store or an element release:")
    for record in sorted(per_function, key=lambda r: (r["unit"], r["function"])):
        if not (any(s["rhs"] in ("acquire", "owned-name") for s in record["stores"]) or record["releases"]):
            continue
        roles = ",".join(f"{l['role']}{'*' if l['canonical'] and l['indexed_by_variable'] else ''}" for l in record["loops"]) or "-"
        lines.append(
            f"  {record['status']:8} {record['unit']}::{record['function']} stores={sum(1 for s in record['stores'] if s['rhs'] in ('acquire','owned-name'))} "
            f"releases={len(record['releases'])} loops={roles} collection_leaves={record['collection_leaves']} other_leaves={record['other_leaves']}"
            + (" CLEARS" if record["would_clear_fully"] else "")
        )
    out_txt.write_text("\n".join(lines) + "\n")
    print("\n".join(lines[:60]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
