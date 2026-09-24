#!/usr/bin/env python3
"""Card W43-INDIRECT-ACCESS-GENERAL: the shape census of the
`indirect memory access is outside the resource projection` family.

Input: one instrumented run directory holding `resource-diagnostics.json` (the
authority for which leaves exist and for every function's other blockers) and
`indirect-census.jsonl` (one record per refused statement, written by the
`indirect-census` feature after the classifier had already refused). The
records carry the statement's complete token text as the classifier read it,
so the shape is read from the analyzer's own token view, never from source
text (card C-GATEWAY-INDIRECT-CENSUS-2 §2).

The join follows that card: `unit_seq` is matched to a document unit by
greatest overlap (checked injective), a leaf `(unit, function, line)` is
explained by every distinct statement record at that key, and a leaf clears
only if every one of its statements does.

Usage: shape_census.py RUN_DIR OUT_JSON [--text OUT_TXT]
"""

import collections
import json
import pathlib
import re
import sys

INDIRECT = "indirect memory access is outside the resource projection"
ASSIGN_OPS = {"+", "-", "*", "/", "%", "&", "|", "^", "<", ">"}
NON_CALLS = {"sizeof", "_Alignof", "__alignof__", "alignof"}


def normalize(description):
    return re.sub(r"`[^`]*`", "`X`", description)


def family(description):
    text = normalize(description)
    return text


def load_document(path):
    doc = json.load(open(path))
    blockers = {}
    unit_leaves = collections.defaultdict(set)
    statuses = {}
    for unit in doc["units"]:
        ukey = (unit.get("library"), unit["file"])
        for function in unit["functions"]:
            fkey = ukey + (function["function"],)
            statuses[fkey] = function["status"]
            blockers[fkey] = [family(d["description"]) for d in function["diagnostics"]]
            for d in function["diagnostics"]:
                if d["description"] == INDIRECT:
                    unit_leaves[ukey].add((function["function"], d["line"]))
    return doc, blockers, dict(unit_leaves), statuses


def load_census(path):
    census = collections.defaultdict(lambda: collections.defaultdict(list))
    seen = collections.defaultdict(set)
    for line in open(path):
        record = json.loads(line)
        seq = record.pop("unit_seq")
        key = (record["function"], record["line"])
        blob = json.dumps(record, sort_keys=True)
        if blob in seen[(seq, key)]:
            continue
        seen[(seq, key)].add(blob)
        census[seq][key].append(record)
    return census


def match_units(census, unit_leaves):
    scores = []
    for ukey, keys in unit_leaves.items():
        for seq, records in census.items():
            overlap = len(keys & set(records))
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


def is_name(text):
    return bool(re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", text))


def is_literal(text):
    return bool(
        re.fullmatch(r"[0-9][0-9A-Za-z_.]*", text)
        or (text.startswith("'") and text.endswith("'"))
    )


def top_level_assignment(texts):
    """Index of the assignment operator's `=` at bracket depth 0, and the
    compound operator run before it ('' for plain `=`), or (None, None)."""
    depth = 0
    for index, text in enumerate(texts):
        if text in "([":
            depth += 1
        elif text in ")]":
            depth -= 1
        elif text == "=" and depth == 0:
            nxt = texts[index + 1] if index + 1 < len(texts) else ""
            prev = texts[index - 1] if index else ""
            if nxt == "=":
                return None, None  # `==` at depth 0 is not an assignment statement
            if prev == "=":
                continue
            if prev in ("!",):
                continue
            # `<=`/`>=` are comparisons unless a shift precedes (`<<=`).
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
    return any(
        texts[i] == texts[i + 1] and texts[i] in "+-" for i in range(len(texts) - 1)
    )


def access_kind(texts):
    """Whether a run of tokens carries an indirect access, lexically."""
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
    """The syntactic form of a destination place (token list)."""
    toks = list(place)
    while len(toks) >= 2 and toks[0] == "(" and toks[-1] == ")":
        toks = toks[1:-1]
    if len(toks) == 1 and is_name(toks[0]):
        return "name", toks[0]
    if len(toks) == 2 and toks[0] == "*" and is_name(toks[1]):
        return "deref", toks[1]
    if len(toks) >= 4 and is_name(toks[0]) and toks[1] == "[" and toks[-1] == "]":
        depth = 0
        for index, text in enumerate(toks[1:], 1):
            if text == "[":
                depth += 1
            elif text == "]":
                depth -= 1
                if depth == 0 and index != len(toks) - 1:
                    return "chain", toks[0]
        return "subscript", toks[0]
    if len(toks) == 4 and is_name(toks[0]) and toks[1:3] == ["-", ">"] and is_name(toks[3]):
        return "member", toks[0]
    if toks and is_name(toks[0]):
        return "chain", toks[0]
    if toks and toks[0] == "*":
        return "deref-expr", None
    return "other", None


def value_kind(texts, declared):
    """scalar / pointer / call / access / unknown for an assignment's value."""
    if not texts:
        return "empty"
    if has_call(texts):
        return "call"
    if access_kind(texts):
        return "access"
    if has_increment(texts) or "=" in texts:
        return "mutating"
    labels = []
    for index, text in enumerate(texts):
        if is_name(text):
            if texts[index - 1:index] == ["("] and index + 1 < len(texts) and texts[index + 1] in (")", "*"):
                continue  # a cast's type name
            labels.append(declared.get(text))
    if any(label and (label.startswith("record_pointer") or "Pointer" in label) for label in labels):
        return "pointer"
    if "&" in texts:
        return "address"
    if all(label == "scalar" for label in labels if label is not None) and all(
        label is not None for label in labels
    ):
        return "scalar"
    if all(label in (None, "scalar") for label in labels):
        # Undeclared names: enumeration constants, macros-turned-literals, file-scope names.
        return "scalar-or-undeclared"
    return "unknown"


def classify(record):
    """(shape, value kind, base name) for one refused statement."""
    texts = record["texts"]
    declared = record.get("declared", {})
    equal, operator = top_level_assignment(texts)
    if equal is not None:
        left, right = texts[:equal - len(operator)], texts[equal + 1:]
        form, base = place_form(left)
        value = value_kind(right, declared)
        compound = operator != ""
        if form in ("deref", "subscript", "member", "chain", "deref-expr"):
            kind = {"deref": "*p", "subscript": "p[i]", "member": "p->f", "chain": "chain", "deref-expr": "*(expr)"}[form]
            if has_increment(left):
                return ("store:" + kind + ":lhs-increment", value, base)
            return (("compound:" if compound else "store:") + kind, value, base)
        if form == "name":
            # The access is on the value side: a read.
            rform, rbase = place_form(right)
            if compound:
                return ("compound-read:" + rform, value, rbase)
            if rform in ("deref", "subscript", "member", "chain"):
                kind = {"deref": "*p", "subscript": "p[i]", "member": "p->f", "chain": "chain"}[rform]
                return ("read:" + kind, declared.get(base) or "undeclared", rbase)
            if has_call(right):
                return ("read:in-call", value, base)
            return ("read:expression", value, base)
        return ("assign:other-destination", value, base)
    if has_increment(texts):
        stripped = [t for t in texts]
        body = [t for t in stripped if t not in ("+", "-")] if len(stripped) > 2 else stripped
        form, base = place_form(body)
        return ("increment:" + form, "-", base)
    if has_call(texts):
        return ("call-argument", "-", None)
    return ("expression", "-", None)


def base_axis(record, base):
    if base is None:
        return "no-base"
    if base in record.get("derived_names", []):
        return "derived"
    if base in record.get("owned_names", []):
        return "owned"
    for chain in record["chains"]:
        if chain["base"] == base:
            if not chain["base_may_bind"]:
                return "never-bound"
            return "may-bind"
    return "may-bind" if base else "no-base"


def base_type(record, base):
    if base is None:
        return "-"
    label = record.get("declared", {}).get(base)
    if label is None:
        return "undeclared"
    if label.startswith("record_pointer"):
        return "record_pointer"
    if label.startswith("record:"):
        return "record"
    if label.startswith("opaque:"):
        return label
    return label


def statement_key(record):
    shape, value, base = classify(record)
    return (shape, value, base_axis(record, base), base_type(record, base))


def main():
    run = pathlib.Path(sys.argv[1])
    out_json = pathlib.Path(sys.argv[2])
    text_path = None
    if "--text" in sys.argv:
        text_path = pathlib.Path(sys.argv[sys.argv.index("--text") + 1])
    doc, blockers, unit_leaves, statuses = load_document(run / "resource-diagnostics.json")
    census = load_census(run / "indirect-census.jsonl")
    assignment = match_units(census, unit_leaves)

    leaves = []  # (fkey, line, [statement keys], [records])
    unexplained = []
    for ukey, keys in unit_leaves.items():
        seq = assignment.get(ukey)
        for function, line in sorted(keys):
            records = census.get(seq, {}).get((function, line), []) if seq is not None else []
            fkey = ukey + (function,)
            if not records:
                unexplained.append(list(fkey) + [line])
                leaves.append((fkey, line, [("unexplained", "-", "-", "-")], []))
                continue
            leaves.append((fkey, line, [statement_key(r) for r in records], records))

    # Per-function facts.
    other_families = {}
    for fkey, families in blockers.items():
        other_families[fkey] = sorted({f for f in families if f != INDIRECT})
    function_leaves = collections.defaultdict(list)
    for leaf in leaves:
        function_leaves[leaf[0]].append(leaf)

    def shape_of(key):
        return key[0]

    # Distribution by shape (a leaf counts under each distinct shape it holds;
    # also under its "hardest" = the lexicographically largest non-admitted key).
    per_shape = collections.defaultdict(lambda: {"statements": 0, "leaves": set(), "functions": set()})
    per_key = collections.defaultdict(lambda: {"statements": 0, "leaves": set(), "functions": set()})
    for fkey, line, keys, records in leaves:
        for key in keys:
            per_shape[shape_of(key)]["statements"] += 1
            per_shape[shape_of(key)]["leaves"].add((fkey, line))
            per_shape[shape_of(key)]["functions"].add(fkey)
            per_key[key]["statements"] += 1
            per_key[key]["leaves"].add((fkey, line))
            per_key[key]["functions"].add(fkey)

    unknown_functions = [f for f, s in statuses.items() if s == "Unknown"]
    touched = sorted(function_leaves)
    sole = [f for f in touched if not other_families[f] and statuses[f] == "Unknown"]
    pairs = collections.Counter()
    for f in touched:
        if len(other_families[f]) == 1 and statuses[f] == "Unknown":
            pairs[other_families[f][0]] += 1

    def clears(fkey, admitted):
        return all(all(admitted(key) for key in keys) for _, _, keys, _ in function_leaves[fkey])

    def reach(admitted):
        sole_cleared = [f for f in sole if clears(f, admitted)]
        pair_cleared = collections.Counter()
        for f in touched:
            if statuses[f] == "Unknown" and len(other_families[f]) == 1 and clears(f, admitted):
                pair_cleared[other_families[f][0]] += 1
        leaves_cleared = sum(1 for _, _, keys, _ in leaves if all(admitted(k) for k in keys))
        return {
            "leaves_cleared": leaves_cleared,
            "sole_functions_cleared": len(sole_cleared),
            "sole_functions": [list(f) for f in sole_cleared],
            "pair_functions_cleared": dict(pair_cleared.most_common()),
        }

    result = {
        "run": str(run),
        "family_leaves": len(leaves),
        "unexplained_leaves": unexplained,
        "units_matched": len(assignment),
        "units_with_leaves": len(unit_leaves),
        "functions_touched": len(touched),
        "unknown_functions": len(unknown_functions),
        "sole_blocked_functions": len(sole),
        "sole_blocked_leaves": sum(len(function_leaves[f]) for f in sole),
        "two_blocker_partners": dict(pairs.most_common(12)),
        "shapes": {},
        "keys": [],
    }
    for shape, data in sorted(per_shape.items(), key=lambda kv: -len(kv[1]["leaves"])):
        single = lambda key, shape=shape: shape_of(key) == shape
        r = reach(single)
        result["shapes"][shape] = {
            "statements": data["statements"],
            "leaves": len(data["leaves"]),
            "functions": len(data["functions"]),
            "sole_functions_touched": sum(1 for f in data["functions"] if f in set(sole)),
            "alone": {k: v for k, v in r.items() if k != "sole_functions"},
        }
    sole_set = set(sole)
    for key, data in sorted(per_key.items(), key=lambda kv: -len(kv[1]["leaves"])):
        result["keys"].append(
            {
                "shape": key[0],
                "value": key[1],
                "base": key[2],
                "base_type": key[3],
                "statements": data["statements"],
                "leaves": len(data["leaves"]),
                "functions": len(data["functions"]),
                "sole_functions_touched": sum(1 for f in data["functions"] if f in sole_set),
            }
        )
    out_json.write_text(json.dumps(result, indent=1) + "\n")
    if text_path:
        lines = []
        lines.append(f"run: {run}")
        lines.append(f"family leaves {len(leaves)}; unexplained {len(unexplained)}; units {len(assignment)}/{len(unit_leaves)}")
        lines.append(f"functions touched {len(touched)}; sole-blocked {len(sole)} holding {result['sole_blocked_leaves']} leaves")
        lines.append("")
        lines.append(f"{'shape':40} {'stmts':>7} {'leaves':>7} {'funcs':>6} {'soleT':>6} {'clrL':>6} {'clrS':>5}  pairs completed")
        for shape, data in result["shapes"].items():
            alone = data["alone"]
            pairs_text = ", ".join(f"{k[:28]}:{v}" for k, v in list(alone["pair_functions_cleared"].items())[:3])
            lines.append(
                f"{shape:40} {data['statements']:7} {data['leaves']:7} {data['functions']:6} {data['sole_functions_touched']:6} {alone['leaves_cleared']:6} {alone['sole_functions_cleared']:5}  {pairs_text}"
            )
        lines.append("")
        lines.append("top statement keys (shape, value, base axis, base type)")
        for row in result["keys"][:80]:
            lines.append(
                f"{row['leaves']:6} {row['functions']:5} {row['sole_functions_touched']:4}  {row['shape']:30} {row['value']:22} {row['base']:12} {row['base_type']}"
            )
        text_path.write_text("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
