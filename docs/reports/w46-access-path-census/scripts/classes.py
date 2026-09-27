#!/usr/bin/env python3
"""Card W46-ACCESS-PATH-CENSUS: the registered follow-up classes on this
baseline, sized by their own registering scripts (registered_classes.sh), and
each split by the typed census.

usage: classes.py CLASSES_DIR RUN_DIR CENSUS_OUT OUT_JSON OUT_TXT

Sizes (leaves / Unknown functions / sole-blocked) are the registering
scripts' own answers over this card's census run: W43's residuals.py for the
three W43 statement classes, W44-INDIRECT-CHAIN-ACCESS's chain_census.py
(grouped exactly as its residuals.py groups readers) for the five chain
classes, W44-BRANCH-CONDITION-CALLS's residuals.py for the condition class and
W44-INDIRECT-CALL-ARGUMENT-ACCESS's family_residuals.py (`destination`) for the
call-result destination class. The split joins each class's leaves that are
`IND`, `COND` or `RET` leaves to the typed census rows by (file, function,
line, family): P2 / P3 expressible, or refused with its causes. Leaves of the
other families a class holds (store-through, unrecognized destination,
assignment and increment forms) are outside this census and counted apart.
The W43 and condition memberships are recomputed per leaf with the
registering modules' own classifiers; the call-result class's per-leaf
membership is the typed reading (a store through a path whose value is a
call), labelled as such.
"""

import collections
import gzip
import importlib.util
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPORTS = HERE.parents[1]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    loaded = importlib.util.module_from_spec(spec)
    sys.path.insert(0, str(path.parent))
    spec.loader.exec_module(loaded)
    return loaded


predict = module("w46_predict", HERE / "predict.py")
CHAIN_GROUPS = {
    "W44-CHAIN-NESTED-AGGREGATE": {"dot-link"},
    "W44-CHAIN-TYPEDEF-ROOT": {"root-untyped"},
    "W44-CHAIN-UNDECLARED-ROOT": {"root-undeclared"},
    "W44-CHAIN-INCOMPLETE-RECORD": {"incomplete-record"},
    "W44-CHAIN-CALL": {"call"},
}
W43_GROUPS = {
    "W43-INDIRECT-UNTYPED-SINGLE-ACCESS": "untyped-single-access",
    "W43-INDIRECT-POINTER-VALUE-STORE": "pointer-value-store",
    "W43-INDIRECT-OTHER-STATEMENTS": "other",
}


def family(description):
    if description == "indirect memory access is outside the resource projection":
        return "IND"
    if description.startswith("unmodeled call, aggregate or indirect access in "):
        return "RET" if description.endswith("return expression") else "COND"
    return None


def main():
    classes_dir = pathlib.Path(sys.argv[1])
    run = pathlib.Path(sys.argv[2])
    census_out = pathlib.Path(sys.argv[3])
    rows = predict.jsonl(census_out / "leaves.jsonl.gz")
    by_key = {(r["file"], r["function"], r["line"], r["family"]): r for r in rows}
    members = collections.defaultdict(list)  # class -> [(file, function, line, family)]
    sizes = {}

    # Chain classes: chain_census.py's family leaves, grouped as residuals.py groups them.
    chain = json.loads((classes_dir / "chain.json").read_text())
    for name, readers in CHAIN_GROUPS.items():
        leaves = [leaf for leaf in chain["family_leaves"] if leaf["reader"] in readers]
        sizes[name] = {"leaves": len(leaves), "unknown_functions": len({l["function"] for l in leaves}),
                       "sole_blocked": len({l["function"] for l in leaves if l["sole"]}), "source": "chain_census.py"}
        for leaf in leaves:
            _, file_, function = leaf["function"].split("/", 1)[0], *leaf["function"].split("/", 1)[1].rsplit("/", 1)
            members[name].append((file_, function, leaf["line"], family(leaf["description"])))

    # W43 classes: residuals.py's own classifier, per leaf.
    w43 = json.loads((classes_dir / "w43.json").read_text())
    residuals = module("w43_residuals", REPORTS / "w43-indirect-access-general/scripts/residuals.py")
    census = residuals.census
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    indirect = census.load_stream(run / "indirect-census.jsonl", lambda record: census.INDIRECT)
    statements = census.load_stream(run / "statement-census.jsonl",
                                    lambda record: census.outcome_family(record["outcome"]))
    assignment = census.match_units([indirect, statements], unit_keys)
    for fkey, data in functions.items():
        if data["status"] != "Unknown":
            continue
        seq = assignment.get(fkey[:2])
        for line, description in data["leaves"]:
            if not residuals.predict.intercepted(description):
                continue
            key = (fkey[2], line, description)
            records = []
            if seq is not None:
                records = statements.get(seq, {}).get(key, [])
                if description == census.INDIRECT:
                    records = indirect.get(seq, {}).get(key, []) or records
            classes = {residuals.classify(record) for record in records} or {"other"}
            cls = min(classes, key=residuals.ORDER.index)
            for name, wanted in W43_GROUPS.items():
                if cls == wanted:
                    members[name].append((fkey[1], fkey[2], line, family(description)))
    for name, wanted in W43_GROUPS.items():
        sizes[name] = {"leaves": w43[wanted]["leaves"], "unknown_functions": w43[wanted]["unknown_functions"],
                       "sole_blocked": w43[wanted]["sole_blocked_functions"], "source": "w43 residuals.py"}

    # The condition class: the registering script's group, recomputed per leaf.
    cond_residuals = module("w44_condition", REPORTS / "w44-branch-condition-calls/scripts/residuals.py")
    stream = collections.defaultdict(lambda: collections.defaultdict(list))
    with gzip.open(classes_dir / "condition-call-census.jsonl.gz", "rt") as handle:
        for line in handle:
            record = json.loads(line)
            stream[record["unit_seq"]][(record["function"], record["line"], record["position"])].append(record)
    cassign = census.match_units(
        [{seq: {(f, l, None): [] for (f, l, _) in keys} for seq, keys in stream.items()}], unit_keys)
    cond_leaves = cond_funcs = 0
    for fkey, data in functions.items():
        if data["status"] != "Unknown":
            continue
        records = stream.get(cassign.get(fkey[:2]), {})
        for line, description in data["leaves"]:
            match = cond_residuals.FAMILY.match(description)
            if not match:
                continue
            found = records.get((fkey[2], line, cond_residuals.POSITION[match.group(1)]), [])
            codes = {cond_residuals.code(record) for record in found}
            if not codes:
                continue
            label = next(c for c in cond_residuals.ORDER + sorted(codes) if c in codes)
            if label == "call-indirect-argument":
                members["W44-CONDITION-CALL-INDIRECT-ARGUMENT"].append((fkey[1], fkey[2], line, "COND"))
    text = (classes_dir / "condition.txt").read_text()
    match = re.search(r"W44-CONDITION-CALL-INDIRECT-ARGUMENT\s+(\d+)\s+(\d+)\s+(\d+)", text)
    sizes["W44-CONDITION-CALL-INDIRECT-ARGUMENT"] = {
        "leaves": int(match.group(1)), "unknown_functions": int(match.group(2)),
        "sole_blocked": int(match.group(3)), "source": "w44-branch-condition-calls residuals.py"}

    # The call-result destination class: family_residuals.py's `destination` size;
    # per-leaf membership is the typed reading.
    text = (classes_dir / "call-argument.txt").read_text()
    match = re.search(r"^destination\s+(\d+)\s+(\d+)\s+(\d+)", text, re.M)
    sizes["W44-CALL-RESULT-INDIRECT-DESTINATION"] = {
        "leaves": int(match.group(1)), "unknown_functions": int(match.group(2)),
        "sole_blocked": int(match.group(3)), "source": "family_residuals.py (destination)"}
    for r in rows:
        if r["family"] == "IND" and r["status"] == "Unknown" and any(
                c.get("rhs_call") and any(p["access"] in ("store", "null-store") for p in c["paths"])
                for c in r["constructs"]):
            members["W44-CALL-RESULT-INDIRECT-DESTINATION"].append((r["file"], r["function"], r["line"], "IND"))

    result = {}
    lines = [f"{'class':40} {'leaves':>6} {'funcs':>6} {'sole':>5}   in census: P2 / P3 / refused (top causes); outside"]
    for name in list(W43_GROUPS)[:1] + list(CHAIN_GROUPS) + list(W43_GROUPS)[1:] + [
            "W44-CONDITION-CALL-INDIRECT-ARGUMENT", "W44-CALL-RESULT-INDIRECT-DESTINATION"]:
        split = collections.Counter()
        causes = collections.Counter()
        roots = collections.Counter()
        outside = 0
        seen = set()
        for key in members[name]:
            if key in seen:
                continue
            seen.add(key)
            row = by_key.get(key)
            if row is None:
                outside += 1
                continue
            group, cs = predict.group_scope(row)
            split[group or "refused"] += 1
            for c in cs:
                causes[c] += 1
            for c in row["constructs"]:
                for p in c["paths"]:
                    roots[p["root_kind"]] += 1
        result[name] = {**sizes[name], "census_split": dict(split), "refusal_causes": dict(causes.most_common()),
                        "outside_census_families": outside, "path_roots": dict(roots.most_common())}
        s = sizes[name]
        lines.append(f"{name:40} {s['leaves']:6} {s['unknown_functions']:6} {s['sole_blocked']:5}   "
                     f"{split.get('P2', 0)} / {split.get('P3', 0)} / {split.get('refused', 0)} "
                     f"{dict(causes.most_common(3))}; {outside}")
    pathlib.Path(sys.argv[4]).write_text(json.dumps(result, indent=1) + "\n")
    pathlib.Path(sys.argv[5]).write_text("\n".join(lines) + "\n")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
