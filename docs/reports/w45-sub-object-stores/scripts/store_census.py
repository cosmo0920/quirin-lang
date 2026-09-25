#!/usr/bin/env python3
"""Card W45-SUB-OBJECT-STORES: classify the refused sub-object stores and
measure what each shape would free.

Usage: store_census.py STORES_JSONL STATEMENTS_JSONL DIAGNOSTICS_JSON [--json OUT] [--text OUT]

STORES_JSONL is the `QCORE_SUB_OBJECT_STORE_CENSUS` stream and STATEMENTS_JSONL
the `QCORE_STATEMENT_CENSUS` stream of one census run; DIAGNOSTICS_JSON is that
run's (or the byte-identical plain run's) `resource-diagnostics.json`.

Records repeat (W4 rounds analyse a body more than once), so both streams are
de-duplicated on (unit, function, line, texts). Units are joined as the
destination-suffix census joins them (`join_units`).

Every record whose outcome is the lexical gate's refusal is classified:

* **shape** — the left-hand side read as `PREFIX ROOT SELECTORS`: the prefix
  (`plain`, `paren` for `( N )`, `deref` for `( * N )`, `addr` for `( & N )`,
  `cast` for `( ( T * ) N )`, `other`) and the selector string (`->`, `.`,
  `[]`), e.g. `plain:->`, `plain:[].`, `paren:->`, `plain:->->`;
* **base** — the root's resolution: `formal`, `local` (the declaration reader
  committed to a type), or `unresolved` (no committed declaration), with the
  reader's label;
* **value** — `scalar`, `null`, `pointer-untracked` (a pointer name the
  function never binds to a resource identity, a string literal or an
  address), `owned-or-derived` (a name bound to an owned or derived contracted
  result, or one the function may bind), `call:<class>`, `descriptor`,
  `access` (the value reads through memory), `other`.

A **gate line** is a (unit, function, line) whose statement census carries at
least one gate refusal. A gate line is *covered* by a set of classes when every
gate-refused statement recorded at it is a store record in the set. A function
is **freed** by the set when it is Unknown, its only distinct refusal reason is
the gate's (the blocker census's sole-blocked population), and every one of its
gate lines is covered.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

GATE = "indirect memory access is outside the resource projection"
NULLS = (["0"], ["NULL"], ["(", "void", "*", ")", "0"], ["(", "(", "void", "*", ")", "0", ")"])
TYPE_OPERATORS = {"sizeof", "_Alignof", "alignof", "typeof", "__typeof", "__typeof__"}
IDENT = re.compile(r"^[A-Za-z_][A-Za-z_0-9]*$")
NUMBER = re.compile(r"^[0-9]")


def read_json(path):
    data = pathlib.Path(path).read_bytes()
    if str(path).endswith(".gz"):
        data = gzip.decompress(data)
    return json.loads(data)


def load(path):
    with open(path) as stream:
        return [json.loads(line) for line in stream]


def join_units(records, document):
    units = document["units"]
    by_function = collections.defaultdict(set)
    lines = collections.defaultdict(set)
    for index, unit in enumerate(units):
        for function in unit["functions"]:
            by_function[function["function"]].add(index)
            for diagnostic in function["diagnostics"]:
                lines[(index, function["function"])].add(diagnostic["line"])
    sequences = collections.defaultdict(list)
    for record in records:
        sequences[record["unit_seq"]].append(record)
    mapping = {}
    for seq, members in sequences.items():
        candidates = None
        for record in members:
            # A function the document does not publish (a header's static
            # inline body analysed only for its summary) constrains nothing.
            found = by_function.get(record["function"])
            if found is None:
                continue
            candidates = found if candidates is None else candidates & found
        candidates = sorted(candidates or [])
        if len(candidates) == 1:
            mapping[seq] = candidates[0]
            continue
        if not candidates:
            # A header function published by another unit empties the
            # intersection: take the unit publishing the most distinct
            # functions of the sequence, when it is unique.
            names = {record["function"] for record in members}
            counts = collections.Counter(
                index for name in names for index in by_function.get(name, ())
            ).most_common(2)
            if counts and (len(counts) == 1 or counts[0][1] > counts[1][1]):
                mapping[seq] = counts[0][0]
            continue
        scores = []
        for candidate in candidates:
            score = sum(
                1
                for record in members
                if lines[(candidate, record["function"])]
                and min(lines[(candidate, record["function"])]) - 400
                <= record["line"]
                <= max(lines[(candidate, record["function"])]) + 400
            )
            scores.append((score, candidate))
        scores.sort(reverse=True)
        if scores and (len(scores) == 1 or scores[0][0] > scores[1][0]):
            mapping[seq] = scores[0][1]
    return mapping


def close_of(tokens, start, open_, close):
    depth = 0
    for index in range(start, len(tokens)):
        if tokens[index] == open_:
            depth += 1
        elif tokens[index] == close:
            depth -= 1
            if depth == 0:
                return index
    return None


def parse_lhs(left):
    """(prefix, root, selectors) of a left-hand side, or ('other', None, '')."""
    prefix, root, at = "other", None, 0
    if left and IDENT.match(left[0]):
        prefix, root, at = "plain", left[0], 1
    elif left[:1] == ["("]:
        close = close_of(left, 0, "(", ")")
        if close is None:
            return "other", None, ""
        inner = left[1:close]
        if len(inner) == 1 and IDENT.match(inner[0]):
            prefix, root = "paren", inner[0]
        elif len(inner) == 2 and inner[0] == "*" and IDENT.match(inner[1]):
            prefix, root = "deref", inner[1]
        elif len(inner) == 2 and inner[0] == "&" and IDENT.match(inner[1]):
            prefix, root = "addr", inner[1]
        elif inner[:1] == ["("] and IDENT.match(inner[-1]):
            cast_close = close_of(inner, 0, "(", ")")
            if cast_close == len(inner) - 2:
                prefix, root = "cast", inner[-1]
            else:
                return "other", None, ""
        else:
            return "other", None, ""
        at = close + 1
    else:
        return "other", None, ""
    selectors = []
    while at < len(left):
        if left[at : at + 2] == ["-", ">"] and at + 2 < len(left) and IDENT.match(left[at + 2]):
            selectors.append("->")
            at += 3
        elif left[at] == "." and at + 1 < len(left) and IDENT.match(left[at + 1]):
            selectors.append(".")
            at += 2
        elif left[at] == "[":
            close = close_of(left, at, "[", "]")
            if close is None:
                return "other", None, ""
            selectors.append("[]")
            at = close + 1
        else:
            return "other", None, ""
    return prefix, root, "".join(selectors)


def value_class(record):
    texts = record["texts"]
    right = texts[record["equal"] + 1 :]
    declared = record["declared"]
    names = [
        text
        for index, text in enumerate(right)
        if IDENT.match(text)
        and not (index > 0 and (right[index - 1] == "." or right[index - 1] == ">"))
    ]
    if right in NULLS:
        return "null"
    if set(names) & set(record["descriptors"]):
        return "descriptor"
    calls = [
        right[index]
        for index in range(len(right) - 1)
        if IDENT.match(right[index]) and right[index + 1] == "(" and right[index] not in TYPE_OPERATORS
    ]
    if calls:
        callee = next((fact for fact in record["callees"] if fact["callee"] == calls[0]), None)
        if callee is None or callee["result"] is None:
            return "call:uncontracted"
        result = callee["result"]
        if result.startswith("Owned") or result.startswith("Reallocated"):
            return "call:owned"
        if result.startswith("Derived"):
            return "call:derived"
        return "call:none"
    if set(names) & (set(record["owned_names"]) | set(record["derived_names"])):
        return "owned-or-derived"
    if any(text in ("-", ".", "[") for text in right) and (
        "[" in right or "." in right or any(right[i : i + 2] == ["-", ">"] for i in range(len(right)))
    ):
        return "access"
    if "&" in right and right[:1] == ["&"]:
        return "address"
    # Strip casts `( T * )` at the head.
    core = list(right)
    while core[:1] == ["("]:
        close = close_of(core, 0, "(", ")")
        if close is None:
            break
        if close == len(core) - 1:
            core = core[1:-1]
            continue
        core = core[close + 1 :]
    if core == ["__qcore_literal__"] or (core and core[0].startswith('"')):
        return "literal"
    if len(core) == 1 and IDENT.match(core[0]):
        name = core[0]
        label = declared.get(name, "")
        if label == "scalar" or label.startswith("scalar") or label == "enum_constant":
            return "scalar"
        if name in record["may_bind"]:
            return "pointer-may-bind"
        if name in record["never_bound"]:
            return "pointer-untracked"
        return "name-unresolved"
    scalarish = all(
        (not IDENT.match(text))
        or declared.get(text, "").startswith("scalar")
        or declared.get(text) == "enum_constant"
        for text in core
    ) and not any(text in ("&",) for text in core)
    if scalarish and core and all(not text.startswith('"') for text in core):
        return "scalar"
    return "other"


def base_class(record, root):
    if root is None:
        return "none"
    label = record["declared"].get(root)
    if root in record["formals"]:
        kind = "formal"
    elif label is not None:
        kind = "local"
    else:
        return "unresolved"
    if label is None:
        return f"{kind}:untyped"
    if label.startswith("record_pointer:"):
        return f"{kind}:record_pointer"
    if label.startswith("record:"):
        return f"{kind}:record"
    if label.startswith("opaque"):
        return f"{kind}:opaque"
    return f"{kind}:{label.split('|')[0]}"


def member_class(record):
    chains = record["destination_chains"]
    if not chains:
        return "none"
    return chains[0]["member_class"]


def forecast_class(record):
    """The rule's forecast for one gate-refused store, from its own reader.

    * `clear` — both sides resolve and the flow premises hold on every path
      whatever the state: the base (if any) and a named value are never bound,
      so the base resolves unbound and the value is custody-neutral;
    * `flow` — both sides resolve; the flow decides (a base the function may
      bind, a named value it may bind, an owned call result that escapes);
    * `refused:<reason>` — the reader or the call's contract refuses: the
      store keeps its refusal.

    A call value is forecast admitted when its contract in force has every
    parameter `Borrowed` and a `None` result, or an owned heap result stored
    through a caller-reachable base; the classifier's reclassification of the
    call can still refuse it (an argument access, a nested call).
    """
    reader = record.get("reader", "")
    if not reader.startswith("read:"):
        return "refused"
    parts = reader.split(":")
    member, value = parts[1], parts[2]
    visible = parts[3] if len(parts) > 3 else "false"
    texts = record["texts"]
    left = texts[: record["equal"]]
    prefix, root, selectors = parse_lhs(left)
    through_pointer = "->" in selectors and prefix != "addr" or "[]" in selectors
    base_clear = (not through_pointer) or root in record["never_bound"]
    if value == "call":
        right = texts[record["equal"] + 1 :]
        calls = [
            right[index]
            for index in range(len(right) - 1)
            if IDENT.match(right[index]) and right[index + 1] == "(" and right[index] not in TYPE_OPERATORS
        ]
        fact = next((fact for fact in record["callees"] if calls and fact["callee"] == calls[0]), None)
        if fact is None or fact["result"] is None or not fact.get("parameters_borrowed"):
            return "refused"
        if fact["result"] == "None":
            return "clear" if base_clear else "flow"
        if fact["result"].startswith("Owned") and fact.get("result_kind") == "Heap" and visible == "true":
            return "flow"
        return "refused"
    if value == "name":
        names = [t for t in texts[record["equal"] + 1 :] if IDENT.match(t)]
        name = names[-1] if names else None
        if base_clear and name in record["never_bound"]:
            return "clear"
        return "flow"
    return "clear" if base_clear else "flow"


def reader_class(record):
    reader = record.get("reader", "")
    if reader.startswith("read:"):
        member, value = reader.split(":")[1:3]
        return f"read:{member}:{value}"
    return reader


def dedupe(records, mapping):
    seen = {}
    for record in records:
        unit = mapping.get(record["unit_seq"])
        if unit is None:
            continue
        key = (unit, record["function"], record["line"], tuple(record["texts"]))
        seen.setdefault(key, record)
    return seen


def main():
    stores_path, statements_path, diagnostics_path = sys.argv[1:4]
    options = sys.argv[4:]
    json_out = text_out = None
    while options:
        if options[0] == "--json":
            json_out = options[1]
        elif options[0] == "--text":
            text_out = options[1]
        else:
            raise SystemExit(f"unknown option {options[0]}")
        options = options[2:]
    document = read_json(diagnostics_path)
    stores_raw = load(stores_path)
    statements_raw = load(statements_path)
    mapping = join_units(stores_raw + statements_raw, document)
    stores = dedupe(stores_raw, mapping)
    statements = dedupe(statements_raw, mapping)

    status = {}
    reasons = {}
    for index, unit in enumerate(document["units"]):
        for function in unit["functions"]:
            key = (index, function["function"])
            status[key] = function["status"]
            reasons[key] = function["diagnostics"]

    classified = {}
    for key, record in stores.items():
        if record["comparison"]:
            cls = ("comparison", "-", "-", "-", "comparison", "refused")
        else:
            prefix, root, selectors = parse_lhs(record["texts"][: record["equal"]])
            cls = (
                f"{prefix}:{selectors}",
                base_class(record, root),
                member_class(record),
                value_class(record),
                reader_class(record),
                forecast_class(record),
            )
        classified[key] = (record, cls)

    gate_statements = collections.defaultdict(list)
    for key, record in statements.items():
        if record["outcome"] == GATE:
            gate_statements[key[:3]].append(key)

    def gate_record(key):
        record, _ = classified[key]
        return record["outcome"] == "Unknown" and record["description"] == GATE

    population = [key for key in classified if gate_record(key)]
    unknown_population = [key for key in population if status.get(key[:2]) == "Unknown"]

    def reason(text):
        return re.sub(r"`[^`]*`", "`_`", text or "")

    sole = [
        key
        for key, diagnostics in reasons.items()
        if status[key] == "Unknown"
        and diagnostics
        and {reason(d["description"]) for d in diagnostics} == {GATE}
    ]

    def freed(predicate):
        out = []
        for function_key in sole:
            lines = {d["line"] for d in reasons[function_key] if d["description"] == GATE}
            ok = True
            for line in lines:
                keys = gate_statements.get((*function_key, line), [])
                if not keys:
                    ok = False
                    break
                for key in keys:
                    if key not in classified or not gate_record(key) or not predicate(classified[key][1]):
                        ok = False
                        break
                if not ok:
                    break
            if ok:
                out.append(function_key)
        return out

    counters = {
        "shape": collections.Counter(),
        "base": collections.Counter(),
        "member": collections.Counter(),
        "value": collections.Counter(),
        "shape_value": collections.Counter(),
        "reader": collections.Counter(),
        "forecast": collections.Counter(),
        "shape_base_member_value": collections.Counter(),
    }
    functions_by = collections.defaultdict(set)
    for key in unknown_population:
        _, (shape, base, member, value, reader, forecast) = classified[key]
        counters["shape"][shape] += 1
        counters["base"][base] += 1
        counters["member"][member] += 1
        counters["value"][value] += 1
        counters["shape_value"][(shape, value)] += 1
        counters["shape_base_member_value"][(shape, base, member, value)] += 1
        counters["reader"][reader] += 1
        counters["forecast"][forecast] += 1
        for axis, name in (
            ("shape", shape),
            ("base", base),
            ("member", member),
            ("value", value),
            ("reader", reader),
            ("forecast", forecast),
        ):
            functions_by[(axis, name)].add(key[:2])

    freed_sets = {}
    for value in sorted(counters["value"]):
        freed_sets[f"value={value}"] = freed(lambda cls, value=value: cls[3] == value)
    for base in sorted(counters["base"]):
        freed_sets[f"base={base}"] = freed(lambda cls, base=base: cls[1] == base)
    for member in sorted(counters["member"]):
        freed_sets[f"member={member}"] = freed(lambda cls, member=member: cls[2] == member)
    for shape in sorted(counters["shape"]):
        freed_sets[f"shape={shape}"] = freed(lambda cls, shape=shape: cls[0] == shape)
    for reader in sorted(counters["reader"]):
        freed_sets[f"reader={reader}"] = freed(lambda cls, reader=reader: cls[4] == reader)
    freed_sets["all-stores"] = freed(lambda cls: True)
    freed_sets["forecast=clear"] = freed(lambda cls: cls[5] == "clear")
    freed_sets["forecast=flow"] = freed(lambda cls: cls[5] in ("clear", "flow"))
    freed_sets["forecast=refused"] = []

    lines_out = []
    lines_out.append(f"store records (deduplicated): {len(classified)}")
    lines_out.append(f"gate-refused store records: {len(population)}")
    lines_out.append(f"  of which in Unknown functions: {len(unknown_population)}")
    lines_out.append(f"sole-blocked (gate-only) Unknown functions: {len(sole)}")
    lines_out.append(f"  freed if every gate-refused store were admitted: {len(freed_sets['all-stores'])}")
    lines_out.append(f"  freed if the rule admits every store its reader reads (clear+flow): {len(freed_sets['forecast=flow'])}")
    lines_out.append(f"  freed on clear stores alone: {len(freed_sets['forecast=clear'])}")
    for axis in ("forecast", "reader", "shape", "base", "member", "value"):
        lines_out.append("")
        lines_out.append(f"by {axis} (records in Unknown functions / functions / freed alone):")
        for name, count in counters[axis].most_common():
            functions = len(functions_by.get((axis, name), ()))
            freed_count = len(freed_sets.get(f"{axis}={name}", ()))
            lines_out.append(f"  {count:>6} {functions:>6} {freed_count:>5}  {name}")
    lines_out.append("")
    lines_out.append("by shape x base x member x value (top 60):")
    for (shape, base, member, value), count in counters["shape_base_member_value"].most_common(60):
        lines_out.append(f"  {count:>6}  {shape:14} {base:24} {member:16} {value}")
    text = "\n".join(lines_out) + "\n"
    if text_out:
        pathlib.Path(text_out).write_text(text)
    else:
        sys.stdout.write(text)
    if json_out:
        payload = {
            "population": len(population),
            "unknown_population": len(unknown_population),
            "sole": len(sole),
            "freed": {name: sorted(f"{document['units'][u]['file']}::{f}" for u, f in keys) for name, keys in freed_sets.items()},
            "records": [
                {
                    "unit": document["units"][key[0]]["file"],
                    "function": key[1],
                    "line": key[2],
                    "texts": " ".join(record["texts"]),
                    "status": status.get(key[:2]),
                    "outcome": record["outcome"],
                    "description": record["description"],
                    "shape": cls[0],
                    "base": cls[1],
                    "member": cls[2],
                    "value": cls[3],
                    "reader": cls[4],
                    "forecast": cls[5],
                }
                for key, (record, cls) in sorted(classified.items(), key=lambda item: item[0][:3])
            ],
        }
        data = json.dumps(payload, indent=1).encode()
        if json_out.endswith(".gz"):
            data = gzip.compress(data, mtime=0)
        pathlib.Path(json_out).write_bytes(data)
    return 0


if __name__ == "__main__":
    sys.exit(main())
