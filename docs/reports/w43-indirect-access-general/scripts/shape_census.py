#!/usr/bin/env python3
"""Card W43-INDIRECT-ACCESS-GENERAL: the shape census of indirect memory access.

Input: one instrumented run directory holding

* `resource-diagnostics.json` - the authority for which leaves exist and for
  every function's other blockers;
* `indirect-census.jsonl` - one record per refused `indirect memory access`
  statement (every `classify_simple` call site), written by the
  `indirect-census` feature after the classifier had already refused;
* `statement-census.jsonl` - one record per ordinary statement the classifier
  refused, or lowered to an indirect destination that only an out-parameter
  contract can sanction (`*p = v`), with the refusal's own description.

Every record carries the statement's complete token text as the classifier
read it, so the shape is read from the analyzer's own token view, never from
source text (card C-GATEWAY-INDIRECT-CENSUS-2 section 2).

The join follows that card: `unit_seq` is matched to a document unit by
greatest overlap (checked injective); a leaf `(unit, function, line,
description)` is explained by every distinct statement record at that key
whose outcome maps to that description; a leaf clears only if every one of its
statements is admitted; and a function is decided only if every leaf it holds,
in every family, clears.

Usage: shape_census.py RUN_DIR OUT_JSON OUT_TXT
"""

import collections
import json
import pathlib
import re
import sys

INDIRECT = "indirect memory access is outside the resource projection"
ASSIGNMENT_EXPRESSION = "unsupported assignment expression"
UNRECOGNIZED_DESTINATION = "unrecognized assignment destination"
INCREMENT = "unmodeled increment or decrement expression"
STORE_THROUGH = re.compile(r"store through `(\*[A-Za-z_][A-Za-z0-9_]*)` without an out-parameter contract")
FAMILIES = (INDIRECT, ASSIGNMENT_EXPRESSION, UNRECOGNIZED_DESTINATION, INCREMENT, "store through `X` without an out-parameter contract")
ASSIGN_OPS = {"+", "-", "*", "/", "%", "&", "|", "^"}
NON_CALLS = {"sizeof", "_Alignof", "__alignof__", "alignof", "typeof", "__typeof__", "__typeof"}


def normalize(description):
    return re.sub(r"`[^`]*`", "`X`", description)


def is_name(text):
    return bool(re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", text))


def is_literal(text):
    return bool(
        re.fullmatch(r"[0-9][0-9A-Za-z_.]*", text)
        or (len(text) >= 3 and text.startswith("'") and text.endswith("'"))
    )


def strip_parens(tokens):
    tokens = list(tokens)
    while len(tokens) >= 2 and tokens[0] == "(" and tokens[-1] == ")":
        depth = 0
        whole = True
        for index, text in enumerate(tokens):
            if text == "(":
                depth += 1
            elif text == ")":
                depth -= 1
                if depth == 0 and index != len(tokens) - 1:
                    whole = False
                    break
        if not whole:
            break
        tokens = tokens[1:-1]
    return tokens


def top_level_assignment(texts):
    """(index of the assignment's `=`, compound operator or '') or (None, None).

    The importer lexes one token per punctuation character, so `+=` is `+ =`,
    `==` is `= =` and `<<=` is `< < =`."""
    depth = 0
    for index, text in enumerate(texts):
        if text in ("(", "["):
            depth += 1
        elif text in (")", "]"):
            depth -= 1
        elif text == "=" and depth == 0:
            nxt = texts[index + 1] if index + 1 < len(texts) else ""
            prev = texts[index - 1] if index else ""
            if nxt == "=" or prev in ("=", "!"):
                continue
            if prev in ("<", ">"):
                before = texts[index - 2] if index >= 2 else ""
                if before != prev:
                    continue
                return index, prev + prev
            if prev in ASSIGN_OPS:
                return index, prev
            return index, ""
    return None, None


def has_call(texts):
    return any(
        is_name(text) and index + 1 < len(texts) and texts[index + 1] == "(" and text not in NON_CALLS
        for index, text in enumerate(texts)
    )


def has_increment(texts):
    return any(texts[i] == texts[i + 1] and texts[i] in "+-" for i in range(len(texts) - 1))


def has_access(texts):
    for index, text in enumerate(texts):
        if text == "-" and index + 1 < len(texts) and texts[index + 1] == ">":
            return True
        if text in (".", "["):
            return True
        if text == "*":
            prev = texts[index - 1] if index else None
            if prev is None or not (is_name(prev) or is_literal(prev) or prev in (")", "]")):
                return True
    return False


def place_form(place):
    """The syntactic form of a place and its base name."""
    toks = strip_parens(place)
    if len(toks) == 1 and is_name(toks[0]):
        return "name", toks[0]
    if len(toks) == 2 and toks[0] == "*" and is_name(toks[1]):
        return "*p", toks[1]
    if len(toks) >= 4 and is_name(toks[0]) and toks[1] == "[" and toks[-1] == "]":
        depth = 0
        for index, text in enumerate(toks[1:], 1):
            if text == "[":
                depth += 1
            elif text == "]":
                depth -= 1
                if depth == 0 and index != len(toks) - 1:
                    return "chain", toks[0]
        return "p[i]", toks[0]
    if len(toks) == 4 and is_name(toks[0]) and toks[1:3] == ["-", ">"] and is_name(toks[3]):
        return "p->f", toks[0]
    if len(toks) == 3 and is_name(toks[0]) and toks[1] == "." and is_name(toks[2]):
        return "s.f", toks[0]
    if toks and is_name(toks[0]):
        return "chain", toks[0]
    if len(toks) >= 2 and toks[0] == "*":
        return "*(expr)", None
    return "other", None


def value_kind(texts, declared):
    """scalar / pointer / call / access / mutating / undeclared for a value."""
    if not texts:
        return "empty"
    if has_call(texts):
        return "call"
    if has_increment(texts) or "=" in texts:
        return "mutating"
    if has_access(texts):
        return "access"
    if "&" in texts and texts.index("&") == 0:
        return "address"
    labels = []
    for index, text in enumerate(texts):
        if not is_name(text):
            continue
        # A cast's type name: `( T )` or `( T * )`.
        if index and texts[index - 1] == "(" and index + 1 < len(texts) and texts[index + 1] in (")", "*"):
            continue
        labels.append(declared.get(text, "undeclared"))
    if any(label.startswith("record_pointer") or "Pointer" in label for label in labels):
        return "pointer"
    if any(label.startswith("opaque") for label in labels):
        return "opaque"
    if any(label == "undeclared" for label in labels):
        return "undeclared-name"
    return "scalar"


def classify(texts, declared):
    """(shape, value kind, base name) for one statement's token text."""
    equal, operator = top_level_assignment(texts)
    if equal is not None:
        left = texts[: equal - len(operator)]
        right = texts[equal + 1:]
        value = value_kind(right, declared)
        if has_increment(left):
            stripped = [t for i, t in enumerate(left) if not (t in "+-" and ((i + 1 < len(left) and left[i + 1] == t) or (i and left[i - 1] == t)))]
            form, base = place_form(stripped)
            return ("store-postinc:" + form, value, base)
        form, base = place_form(left)
        if form != "name":
            return (("compound:" if operator else "store:") + form, value, base)
        rform, rbase = place_form(right)
        if operator:
            return ("compound-into-name:" + rform, value, rbase)
        if rform in ("*p", "p[i]", "p->f", "s.f", "chain", "*(expr)"):
            return ("read:" + rform, declared.get(base, "undeclared"), rbase)
        if has_call(right):
            return ("assign-call", value, base)
        return ("assign-expression", value, base)
    if has_increment(texts):
        body = [t for t in texts if t not in ("+", "-")]
        form, base = place_form(body)
        return ("increment:" + form, "-", base)
    if has_call(texts):
        return ("call-statement", "call", None)
    return ("expression", "-", None)


def base_axis(record, base):
    if base is None:
        return "no-base"
    if base in record.get("derived_names", []):
        return "derived"
    if base in record.get("owned_names", []):
        return "owned"
    may_bind = record.get("may_bind")
    if may_bind is not None:
        return "may-bind" if base in may_bind else "never-bound"
    for chain in record.get("chains", []):
        if chain["base"] == base:
            return "may-bind" if chain["base_may_bind"] else "never-bound"
    return "may-bind"


def base_type(record, base):
    if base is None:
        return "-"
    label = record.get("declared", {}).get(base, "undeclared")
    if label.startswith("record_pointer"):
        return "record_pointer"
    if label.startswith("record:"):
        return "record"
    return label


def statement_key(record, family=None):
    shape, value, base = classify(record["texts"], record.get("declared", {}))
    return (shape, value, base_axis(record, base), base_type(record, base), normalize(family or ""))


def outcome_family(outcome):
    match = re.fullmatch(r"indirect destination `(\*[^`]*)`", outcome)
    if match:
        return f"store through `{match.group(1)}` without an out-parameter contract"
    return outcome


def load_document(path):
    doc = json.load(open(path))
    functions = {}
    unit_keys = collections.defaultdict(set)
    for unit in doc["units"]:
        ukey = (unit.get("library"), unit["file"])
        for function in unit["functions"]:
            fkey = ukey + (function["function"],)
            functions[fkey] = {
                "status": function["status"],
                "leaves": [(d["line"], d["description"]) for d in function["diagnostics"]],
            }
            for d in function["diagnostics"]:
                unit_keys[ukey].add((function["function"], d["line"]))
    return doc, functions, unit_keys


def load_stream(path, family_of):
    stream = collections.defaultdict(lambda: collections.defaultdict(list))
    seen = collections.defaultdict(set)
    if not path.exists():
        return stream
    for line in open(path):
        record = json.loads(line)
        seq = record.pop("unit_seq")
        description = family_of(record)
        key = (record["function"], record["line"], description)
        blob = json.dumps(record, sort_keys=True)
        if blob in seen[(seq, key)]:
            continue
        seen[(seq, key)].add(blob)
        stream[seq][key].append(record)
    return stream


def match_units(streams, unit_keys):
    per_seq = collections.defaultdict(set)
    for stream in streams:
        for seq, records in stream.items():
            per_seq[seq].update((f, l) for f, l, _ in records)
    scores = []
    for ukey, keys in unit_keys.items():
        for seq, records in per_seq.items():
            overlap = len(keys & records)
            if overlap:
                scores.append((overlap, ukey, seq))
    scores.sort(key=lambda row: (-row[0], str(row[1]), row[2]))
    assigned, used = {}, set()
    for _, ukey, seq in scores:
        if ukey in assigned or seq in used:
            continue
        assigned[ukey] = seq
        used.add(seq)
    return assigned


def main():
    run = pathlib.Path(sys.argv[1])
    out_json = pathlib.Path(sys.argv[2])
    out_txt = pathlib.Path(sys.argv[3])
    _, functions, unit_keys = load_document(run / "resource-diagnostics.json")
    indirect = load_stream(run / "indirect-census.jsonl", lambda record: INDIRECT)
    statements = load_stream(run / "statement-census.jsonl", lambda record: outcome_family(record["outcome"]))
    assignment = match_units([indirect, statements], unit_keys)

    # Every leaf of a census-covered family, with its statement keys.
    leaf_keys = {}
    leaf_records = {}
    unexplained = collections.Counter()
    for fkey, data in functions.items():
        if data["status"] != "Unknown":
            continue
        ukey, function = fkey[:2], fkey[2]
        seq = assignment.get(ukey)
        for line, description in data["leaves"]:
            fam = normalize(description)
            if fam not in FAMILIES:
                continue
            key = (function, line, description)
            records = []
            if seq is not None:
                records = statements.get(seq, {}).get(key, [])
                if description == INDIRECT:
                    # The indirect stream sees every classify_simple call site,
                    # for-clauses and synthetic condition calls included.
                    records = indirect.get(seq, {}).get(key, []) or records
            if not records:
                unexplained[fam] += 1
                leaf_keys[(fkey, line, description)] = [("unexplained", "-", "-", "-", fam)]
                continue
            leaf_keys[(fkey, line, description)] = sorted({statement_key(r, description) for r in records})
            leaf_records[(fkey, line, description)] = records

    unknown = {f for f, d in functions.items() if d["status"] == "Unknown"}

    def blockers(fkey):
        return {normalize(description) for _, description in functions[fkey]["leaves"]}

    def decided(fkey, admitted):
        """Every leaf of the function clears under `admitted`."""
        for line, description in functions[fkey]["leaves"]:
            keys = leaf_keys.get((fkey, line, description))
            if keys is None or not all(admitted(k) for k in keys):
                return False
        return True

    family_rows = {}
    for fam in FAMILIES:
        touched = {f for f in unknown if fam in blockers(f)}
        sole = {f for f in touched if blockers(f) == {fam}}
        leaves = sum(1 for (f, l, d) in leaf_keys if normalize(d) == fam)
        family_rows[fam] = {
            "leaves": leaves,
            "functions": len(touched),
            "sole_functions": len(sole),
            "unexplained_leaves": unexplained[fam],
        }

    per_shape = collections.defaultdict(lambda: collections.defaultdict(lambda: {"statements": 0, "leaves": set(), "functions": set()}))
    for (fkey, line, description), keys in leaf_keys.items():
        fam = normalize(description)
        for key in keys:
            row = per_shape[fam][key]
            row["statements"] += 1
            row["leaves"].add((fkey, line))
            row["functions"].add(fkey)

    def reach(admitted):
        decided_functions = sorted(f for f in unknown if decided(f, admitted))
        leaves_cleared = collections.Counter()
        for (fkey, line, description), keys in leaf_keys.items():
            if all(admitted(k) for k in keys):
                leaves_cleared[normalize(description)] += 1
        # Functions left blocked by exactly one other family once admitted leaves clear.
        completes = collections.Counter()
        for f in unknown:
            remaining = set()
            for line, description in functions[f]["leaves"]:
                keys = leaf_keys.get((f, line, description))
                if keys is None or not all(admitted(k) for k in keys):
                    remaining.add(normalize(description))
            before = blockers(f)
            if len(before) == 2 and len(remaining) == 1 and remaining < before:
                completes[next(iter(remaining))] += 1
        return {
            "leaves_cleared": dict(leaves_cleared),
            "functions_decided": len(decided_functions),
            "decided": ["/".join(str(p) for p in f) for f in decided_functions],
            "pairs_reduced_to_one": dict(completes.most_common(8)),
        }

    shape_rows = []
    for fam in FAMILIES:
        by_shape = collections.defaultdict(lambda: {"statements": 0, "leaves": set(), "functions": set()})
        for key, row in per_shape[fam].items():
            agg = by_shape[key[0]]
            agg["statements"] += row["statements"]
            agg["leaves"] |= row["leaves"]
            agg["functions"] |= row["functions"]
        for shape, row in sorted(by_shape.items(), key=lambda kv: -len(kv[1]["leaves"])):
            r = reach(lambda key, shape=shape, fam=fam: key[0] == shape and key[4] == fam)
            shape_rows.append(
                {
                    "family": fam,
                    "shape": shape,
                    "statements": row["statements"],
                    "leaves": len(row["leaves"]),
                    "functions": len(row["functions"]),
                    "sole_touched": sum(1 for f in row["functions"] if f in unknown and len(blockers(f)) == 1),
                    "alone": {k: v for k, v in r.items() if k != "decided"},
                    "alone_decided": r["decided"],
                }
            )
    key_rows = []
    for fam in FAMILIES:
        for key, row in sorted(per_shape[fam].items(), key=lambda kv: -len(kv[1]["leaves"])):
            key_rows.append(
                {
                    "family": fam,
                    "shape": key[0],
                    "value": key[1],
                    "base": key[2],
                    "base_type": key[3],
                    "statements": row["statements"],
                    "leaves": len(row["leaves"]),
                    "functions": len(row["functions"]),
                    "sole_touched": sum(1 for f in row["functions"] if f in unknown and len(blockers(f)) == 1),
                }
            )
    result = {
        "run": str(run),
        "units_matched": len(assignment),
        "units_with_leaves": len(unit_keys),
        "families": family_rows,
        "shapes": shape_rows,
        "keys": key_rows,
    }
    out_json.write_text(json.dumps(result, indent=1) + "\n")

    lines = [f"run: {run}", f"units matched {len(assignment)} / {len(unit_keys)}", ""]
    lines.append(f"{'family':62} {'leaves':>7} {'funcs':>6} {'sole':>5} {'unexpl':>6}")
    for fam, row in family_rows.items():
        lines.append(f"{fam[:62]:62} {row['leaves']:7} {row['functions']:6} {row['sole_functions']:5} {row['unexplained_leaves']:6}")
    for fam in FAMILIES:
        lines += ["", f"== {fam}", f"{'shape':34} {'stmts':>6} {'leaves':>6} {'funcs':>6} {'soleT':>5} {'clrL':>6} {'dec':>4}  pairs reduced to one"]
        for row in shape_rows:
            if row["family"] != fam:
                continue
            alone = row["alone"]
            cleared = alone["leaves_cleared"].get(fam, 0)
            pairs = ", ".join(f"{k[:24]}:{v}" for k, v in list(alone["pairs_reduced_to_one"].items())[:3])
            lines.append(f"{row['shape'][:34]:34} {row['statements']:6} {row['leaves']:6} {row['functions']:6} {row['sole_touched']:5} {cleared:6} {alone['functions_decided']:4}  {pairs}")
    lines += ["", "statement keys (family, shape, value, base axis, base type): leaves funcs soleTouched"]
    for row in key_rows:
        if row["leaves"] < 15:
            continue
        lines.append(f"{row['leaves']:6} {row['functions']:5} {row['sole_touched']:4}  {row['family'][:18]:18} {row['shape'][:24]:24} {row['value'][:18]:18} {row['base']:11} {row['base_type'][:40]}")
    out_txt.write_text("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
