#!/usr/bin/env python3
"""Attribute every delta between the BEFORE and AFTER corpus diagnostics.

Usage: attribute.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS FAMILY_CENSUS SOURCE_ROOT... [--json OUT]

FAMILY_CENSUS is the AFTER run's `QCORE_RETURN_FAMILY_CENSUS` stream: one
record per `return` the card's module decided (`scalar-read`,
`cast-call-lowered`, `cast-call-admitted`), with the unit ordinal, function,
internal line, token view and the read node's bases. SOURCE_ROOT are the
pinned source trees the diagnostics' `origin` (`file:line`) coordinates are
read from.

A delta is attributed only if it is one of:

* `direct` — a removed return-expression leaf at a coordinate the family
  census decided (the AFTER analysis took the card's route there);
* `exchange` — an added `unsupported` leaf at a `cast-call-lowered`
  coordinate (the classifier's own refusal replacing the return wording);
* `base-obligation` — an added `unknown_alias` leaf at a `scalar-read`
  coordinate naming one of that read's bases;
* `cascade` — a removed or added leaf whose statement calls a function in
  the backward call-graph closure (function bodies and function-like macros of
  the pinned sources) of the functions the card's route decided (a family
  record, or a move Unknown -> Safe): such a callee can now carry a validated
  summary, so the call is classified (or desugared) where it was refused
  before, and whatever the classifier then says about the rest of the
  statement is published at the same coordinate. A refusal naming its callee
  (`call to uncontracted function `X``, `nested call `X``) is read from the
  description; otherwise the calls are read from the statement's source lines;
* `cascade-flow` — any other leaf of a function that decided a return itself
  or whose body calls into the closure: a flow effect of that newly modeled
  call or return elsewhere in the same body (a binding no longer poisoned, a
  formal no longer passed to an uncontracted callee, a loop condition whose
  origin is its `do`).

A function status move is attributed when every leaf it lost is attributed
and it gained none. Anything else is UNATTRIBUTED and makes the script exit 1.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

RETURN = "unmodeled call, aggregate or indirect access in return expression"
PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin")


def load(path):
    data = pathlib.Path(path).read_bytes()
    if str(path).endswith(".gz"):
        data = gzip.decompress(data)
    return json.loads(data)


def index(document):
    functions = {}
    for unit in document["units"]:
        for entry in unit["functions"]:
            functions[(unit["library"], unit["file"], entry["function"])] = entry
    return functions


def multiset(entry):
    return collections.Counter(
        (d["category"], d["description"], d["line"], d.get("origin"))
        for d in entry.get("diagnostics") or []
    )


class Sources:
    def __init__(self, roots):
        self.files = collections.defaultdict(list)
        for root in roots:
            for path in pathlib.Path(root).rglob("*.[ch]"):
                self.files[path.name].append(path)
        self.cache = {}

    def line(self, origin):
        if not origin or ":" not in origin:
            return []
        name, number = origin.rsplit(":", 1)
        try:
            number = int(number)
        except ValueError:
            return []
        texts = []
        for path in self.files.get(pathlib.Path(name).name, []):
            if path not in self.cache:
                self.cache[path] = path.read_text(errors="replace").splitlines()
            lines = self.cache[path]
            if 0 < number <= len(lines):
                texts.append(lines[number - 1])
        return texts


def calls(text):
    return set(re.findall(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(", text))


def strip_comments_and_strings(text):
    text = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)
    text = re.sub(r"//[^\n]*", "", text)
    text = re.sub(r'"(?:\\.|[^"\\\n])*"', '""', text)
    text = re.sub(r"'(?:\\.|[^'\\\n])*'", "''", text)
    return text


class CallGraph:
    """Function definitions and function-like macros of the pinned sources,
    each with the names its body calls. Read heuristically from the text: it
    serves attribution (why a leaf could move), never a verdict."""

    def __init__(self, sources):
        self.bodies = collections.defaultdict(set)
        self.spans = collections.defaultdict(list)
        for paths in sources.files.values():
            for path in paths:
                self.read(path)

    def read(self, path):
        try:
            raw = path.read_text(errors="replace")
        except OSError:
            # A dangling link in the pinned tree.
            return
        text = strip_comments_and_strings(raw)
        for match in re.finditer(r"^[ \t]*#[ \t]*define[ \t]+([A-Za-z_]\w*)\(([^)]*)\)((?:[^\n]*\\\n)*[^\n]*)", text, re.M):
            self.bodies[match.group(1)] |= calls(match.group(3))
        depth = 0
        index = 0
        length = len(text)
        while index < length:
            char = text[index]
            if char == "{":
                if depth == 0:
                    head = text[max(0, index - 2000):index]
                    found = re.search(r"\b([A-Za-z_]\w*)\s*\((?:[^()]|\([^()]*\))*\)\s*(?:__attribute__\s*\(\(.*?\)\)\s*)*$", head, re.S)
                    close = self.match(text, index)
                    if found and close is not None and found.group(1) not in ("if", "while", "for", "switch", "sizeof"):
                        name = found.group(1)
                        self.bodies[name] |= calls(text[index:close])
                        first = text.count("\n", 0, index) + 1
                        last = text.count("\n", 0, close) + 1
                        self.spans[(path.name, name)].append((first, last))
                        index = close + 1
                        continue
                depth += 1
            elif char == "}":
                depth = max(0, depth - 1)
            index += 1

    @staticmethod
    def match(text, open_index):
        depth = 0
        for index in range(open_index, len(text)):
            if text[index] == "{":
                depth += 1
            elif text[index] == "}":
                depth -= 1
                if depth == 0:
                    return index
        return None

    def distances(self, seeds):
        """Backward breadth-first layers: 0 for a seed, k for a body calling a
        name at layer k-1."""
        distance = {name: 0 for name in seeds}
        frontier = set(seeds)
        layer = 0
        while frontier:
            layer += 1
            reached = {
                name for name, called in self.bodies.items()
                if name not in distance and called & frontier
            }
            for name in reached:
                distance[name] = layer
            frontier = reached
        return distance


def main():
    before = index(load(sys.argv[1]))
    after = index(load(sys.argv[2]))
    stream = pathlib.Path(sys.argv[3]).read_bytes()
    if sys.argv[3].endswith(".gz"):
        stream = gzip.decompress(stream)
    records = [json.loads(line) for line in stream.decode().splitlines() if line]
    roots = list(sys.argv[4:])
    if "--json" in roots:
        at = roots.index("--json")
        del roots[at : at + 2]
    roots = roots or [str(PIN / "fluent-bit-clang-source"), str(PIN / "monkey")]
    sources = Sources(roots)

    # Join each census unit ordinal to one published unit by the BEFORE
    # return leaves it names (the same vote W43's return census uses).
    by_function_line = collections.defaultdict(set)
    for key, entry in before.items():
        for d in entry.get("diagnostics") or []:
            if d["description"] == RETURN:
                by_function_line[(key[2], d["line"])].add(key[:2])
    votes = collections.defaultdict(collections.Counter)
    for record in records:
        for unit in by_function_line.get((record["function"], record["line"]), ()):
            votes[record["unit_seq"]][unit] += 1
    seq_unit = {}
    for seq, counter in votes.items():
        best, count = counter.most_common(1)[0]
        if [u for u, c in counter.items() if c == count] == [best]:
            seq_unit[seq] = best
    decided = {}
    for record in records:
        unit = seq_unit.get(record["unit_seq"])
        if unit is None:
            continue
        # The last pass over a body is the published analysis.
        decided[(unit[0], unit[1], record["function"], record["line"])] = record

    moves = {}
    removed = []
    added = []
    for key in sorted(set(before) | set(after)):
        b, a = before.get(key), after.get(key)
        if b is None or a is None:
            moves[key] = (b and b["status"], a and a["status"])
            continue
        if b["status"] != a["status"]:
            moves[key] = (b["status"], a["status"])
        lb, la = multiset(b), multiset(a)
        for leaf, count in (lb - la).items():
            removed.extend([(key, leaf)] * count)
        for leaf, count in (la - lb).items():
            added.extend([(key, leaf)] * count)

    # The functions the card's route decided directly, then the cascade.
    direct_names = {record["function"] for record in decided.values()}
    moved_safe = {key[2] for key, (b, a) in moves.items() if a == "Safe"}
    deciders = set(direct_names) | moved_safe

    attributed_removed = {}
    attributed_added = {}
    for position, (key, leaf) in enumerate(removed):
        category, description, line, origin = leaf
        coordinate = (*key, line)
        record = decided.get(coordinate)
        if description == RETURN and record is not None:
            attributed_removed[position] = ("direct", record["family"])
    for position, (key, leaf) in enumerate(added):
        category, description, line, origin = leaf
        record = decided.get((*key, line))
        if record is None:
            continue
        if record["family"] == "cast-call-lowered" and category == "unsupported":
            attributed_added[position] = ("exchange", record["family"])
        elif record["family"] == "scalar-read" and category == "unknown_alias":
            variable = description.split("`")[1] if "`" in description else None
            if variable in record["subjects"]:
                attributed_added[position] = ("base-obligation", variable)

    # A callee can validate a summary it could not before only if its body
    # (or a macro's expansion) reaches, through calls, a function the card
    # decided: the call graph's backward closure over the deciders.
    graph = CallGraph(sources)
    distance = graph.distances(deciders)
    reach = set(distance)

    def nearest(names):
        hits = sorted((distance[name], name) for name in names if name in distance)
        return (hits[0][1], hits[0][0]) if hits else None

    def statement_calls(leaf):
        """Calls on the leaf's source line and its continuation lines, up to
        the end of the statement or the opening of its body."""
        name_number = leaf[3]
        named = set()
        if not name_number or ":" not in name_number:
            return named
        name, number = name_number.rsplit(":", 1)
        for path in sources.files.get(pathlib.Path(name).name, []):
            if path not in sources.cache:
                sources.cache[path] = path.read_text(errors="replace").splitlines()
            lines = sources.cache[path]
            at = int(number) - 1
            for offset in range(8):
                if at + offset >= len(lines):
                    break
                text = lines[at + offset]
                named |= calls(text)
                if offset and (";" in text or text.rstrip().endswith("{")):
                    break
                if not offset and (text.rstrip().endswith(";") or text.rstrip().endswith("{")) \
                        and text.count("(") == text.count(")"):
                    break
        return named

    def statement_cascade(leaf):
        return nearest(statement_calls(leaf))

    direct_functions = {key[:3] for key in decided}

    def function_cascade(key):
        """The enclosing function decided a return itself, or its body calls
        a function in the closure: a flow effect of a newly modeled call or
        return elsewhere in the same body."""
        if key in direct_functions:
            return (key[2], 0)
        called = set()
        for (file, name), spans in graph.spans.items():
            if name == key[2] and file == pathlib.Path(key[1]).name:
                called |= graph.bodies.get(name, set())
        return nearest(called)

    for items, attributed in ((removed, attributed_removed), (added, attributed_added)):
        for position, (key, leaf) in enumerate(items):
            if position in attributed:
                continue
            named = re.findall(r"`([^`]*)`", leaf[1])
            if leaf[0] == "unsupported" and named and named[0] in reach and (
                    "uncontracted function" in leaf[1] or "nested call" in leaf[1]):
                attributed[position] = ("cascade", named[0], distance[named[0]])
                continue
            hit = statement_cascade(leaf)
            if hit is not None:
                attributed[position] = ("cascade", *hit)
                continue
            hit = function_cascade(key)
            if hit is not None:
                attributed[position] = ("cascade-flow", *hit)

    unattributed_removed = [removed[p] for p in range(len(removed)) if p not in attributed_removed]
    unattributed_added = [added[p] for p in range(len(added)) if p not in attributed_added]

    move_rows = []
    unattributed_moves = []
    for key, (b, a) in sorted(moves.items()):
        lost = [attributed_removed.get(p) for p, (k, _) in enumerate(removed) if k == key]
        gained = [p for p, (k, _) in enumerate(added) if k == key]
        reasons = collections.Counter(reason[0] if reason else "UNATTRIBUTED" for reason in lost)
        callees = sorted({reason[1] for reason in lost if reason and reason[0] in ("cascade", "cascade-flow")})
        families = sorted({reason[1] for reason in lost if reason and reason[0] == "direct"})
        row = {
            "function": list(key),
            "before": b,
            "after": a,
            "lost": dict(reasons),
            "families": families,
            "cascade_callees": callees,
            "gained": len(gained),
        }
        ok = a is not None and b is not None and "UNATTRIBUTED" not in reasons
        if a == "Safe" and gained:
            ok = False
        row["attributed"] = ok
        move_rows.append(row)
        if not ok:
            unattributed_moves.append(row)

    def label(item):
        key, leaf = item
        return {"function": list(key), "category": leaf[0], "description": leaf[1], "line": leaf[2], "origin": leaf[3]}

    summary = {
        "family_records": len(records),
        "family_records_joined": len(decided),
        "family_decisions_by_family": dict(collections.Counter(r["family"] for r in decided.values())),
        "removed_leaves": len(removed),
        "added_leaves": len(added),
        "removed_by_reason": dict(collections.Counter(
            f"{r[0]}:{r[1] if r[0] == 'direct' else '*'}" for r in attributed_removed.values())),
        "added_by_reason": dict(collections.Counter(
            f"{r[0]}" for r in attributed_added.values())),
        "removed_by_category": dict(collections.Counter(
            f"{leaf[0]}: {re.sub(r'`[^`]*`', '`_`', leaf[1])}" for _, leaf in removed)),
        "added_by_category": dict(collections.Counter(
            f"{leaf[0]}: {re.sub(r'`[^`]*`', '`_`', leaf[1])}" for _, leaf in added)),
        "function_moves": dict(collections.Counter(f"{b} -> {a}" for b, a in moves.values())),
        "moves_direct_only": sum(1 for r in move_rows if set(r["lost"]) == {"direct"}),
        "moves_with_cascade": sum(1 for r in move_rows if "cascade" in r["lost"] or "cascade-flow" in r["lost"]),
        "call_graph_closure": len(reach),
        "cascade_by_distance": dict(sorted(collections.Counter(
            f"{r[0]}@{r[2]}" for r in list(attributed_removed.values()) + list(attributed_added.values())
            if r[0] in ("cascade", "cascade-flow")).items())),
        "cascade_callees": dict(collections.Counter(
            c for r in list(attributed_removed.values()) + list(attributed_added.values())
            if r[0] in ("cascade", "cascade-flow") for c in [r[1]])),
        "unattributed_removed": len(unattributed_removed),
        "unattributed_added": len(unattributed_added),
        "unattributed_moves": len(unattributed_moves),
    }
    summary["attribution_complete"] = not (unattributed_removed or unattributed_added or unattributed_moves)
    print(json.dumps(summary, indent=1))
    for item in unattributed_removed[:40]:
        print("UNATTRIBUTED removed", json.dumps(label(item)))
    for item in unattributed_added[:40]:
        print("UNATTRIBUTED added", json.dumps(label(item)))
    for row in unattributed_moves[:40]:
        print("UNATTRIBUTED move", json.dumps(row))
    if "--json" in sys.argv:
        out = pathlib.Path(sys.argv[sys.argv.index("--json") + 1])
        out.write_text(json.dumps({
            "summary": summary,
            "moves": move_rows,
            "removed": [dict(label(removed[p]), reason=list(attributed_removed.get(p) or ["UNATTRIBUTED"])) for p in range(len(removed))],
            "added": [dict(label(added[p]), reason=list(attributed_added.get(p) or ["UNATTRIBUTED"])) for p in range(len(added))],
        }, indent=1) + "\n")
    return 0 if summary["attribution_complete"] else 1


if __name__ == "__main__":
    sys.exit(main())
