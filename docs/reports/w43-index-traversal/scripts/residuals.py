#!/usr/bin/env python3
"""Card W43-INDEX-TRAVERSAL: size the refused traversal shapes at the AFTER tree.

Input: a census run of the card's implementation (`--census`, an
`indirect-census` build of the tip). Two families are sized, counting leaves
in Unknown functions, the Unknown functions carrying one, and the functions
whose only blocker family is that family ("sole"):

* **conditions** - every leaf of the five controlling-expression refusals,
  classified by its census record(s): `call` (a call the condition-call route
  did not isolate), `write` (an increment or assignment in the condition),
  `address` (a unary `&`), `chain` (two or more member selections, or a
  subscript followed by a member), `untyped` (every remaining access the
  typed-condition recogniser refused: an unclassified base or member, a
  `void *`, a pointer-valued leaf), `unexplained`;
* **steps** - every leaf of the increment / unrecognized-destination /
  unsupported-assignment / formal-gate refusals whose statement the pointer
  step and stepped-access recognisers do not admit and which names an
  arithmetic step: `formal` (the formal-mutation gate on a formal's step),
  `unreadable` (the function's declarations were not read, so no name is
  typed), `untyped-name` (the stepped name is not a declared scalar or
  single-level pointer), `computed` (an access, call or cast in the offset
  or the source), `other`.

Usage: residuals.py CENSUS_RUN OUT_JSON OUT_TXT
"""

import collections
import importlib.util
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "w43-indirect-access-general" / "scripts"))

import shape_census as census  # noqa: E402

_spec = importlib.util.spec_from_file_location("traversal_predict", HERE / "predict.py")
predict = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(predict)

STEP_FAMILIES = {
    "unmodeled increment or decrement expression",
    "unrecognized assignment destination",
    "unsupported assignment expression",
    predict.GATE,
}


def condition_class(texts):
    joined = " ".join(texts)
    if any(
        re.fullmatch(r"[A-Za-z_]\w*", t) and i + 1 < len(texts) and texts[i + 1] == "("
        for i, t in enumerate(texts)
    ):
        return "call"
    if "+ +" in joined or "- -" in joined:
        return "write"
    for i, t in enumerate(texts):
        if t != "=":
            continue
        before = texts[i - 1] if i else ""
        after = texts[i + 1] if i + 1 < len(texts) else ""
        if after == "=" or before in ("=", "!", "<", ">"):
            continue
        return "write"
    for i, t in enumerate(texts):
        if t == "&" and (i == 0 or texts[i - 1] in ("(", "!", "&", "|", "=", "<", ">", ",", "?", ":")):
            return "address"
    arrows = joined.count("- >")
    if arrows >= 2 or re.search(r"\] (\.|- >)", joined) or re.search(r"- > \w+ \.", joined):
        return "chain"
    return "untyped"


def step_class(record, declared):
    texts = record["texts"]
    joined = " ".join(texts)
    if not declared["readable"]:
        return "unreadable"
    if record["outcome"] == predict.GATE:
        return "formal"
    if any(t in ("(", "[", "&", "*", "-") and t != "-" for t in texts) and (
        "(" in texts or "[" in texts or "&" in texts
    ):
        return "computed"
    names = [t for t in texts if re.fullmatch(r"[A-Za-z_]\w*", t)]
    if names and names[0] not in declared["scalars"] | declared["pointers"]:
        return "untyped-name"
    return "other"


def is_arithmetic_step(record):
    joined = " ".join(record["texts"])
    return bool(re.search(r"\+ \+|- -|[+-] =|= [A-Za-z_]\w* [+-] ", joined))


def main():
    run = pathlib.Path(sys.argv[1])
    out_json, out_txt = pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    stream = predict.load(run / "traversal-census.jsonl")
    per_seq = {}
    for seq, records in stream.items():
        keys = collections.defaultdict(list)
        names = collections.defaultdict(
            lambda: {"scalars": set(), "pointers": set(), "readable": False}
        )
        for record in records:
            if record["kind"] == "function":
                entry = names[record["function"]]
                entry["scalars"].update(record["scalars"])
                entry["pointers"].update(record["pointers"])
                entry["readable"] |= bool(record["scalars"] or record["pointers"])
            else:
                keys[(record["function"], record["line"])].append(record)
        per_seq[seq] = (keys, names)
    assignment = census.match_units(
        [{seq: {(f, l, None): [] for (f, l) in keys} for seq, (keys, _) in per_seq.items()}],
        unit_keys,
    )

    sizes = {"conditions": collections.defaultdict(lambda: [0, set(), set()]),
             "steps": collections.defaultdict(lambda: [0, set(), set()])}
    for fkey, data in functions.items():
        if data["status"] != "Unknown":
            continue
        families = {description for _, description in data["leaves"]}
        seq = assignment.get(fkey[:2])
        keys, names = per_seq.get(seq, ({}, {}))
        declared = names.get(
            fkey[2], {"scalars": set(), "pointers": set(), "readable": False}
        )
        for line, description in data["leaves"]:
            records = keys.get((fkey[2], line), [])
            match = predict.CONDITION.fullmatch(description)
            if match:
                conditions = [
                    r for r in records
                    if r["kind"] == "condition" and r["position"] in predict.POSITION[match.group(1)]
                ]
                label = condition_class(conditions[0]["texts"]) if conditions else "unexplained"
                family = "conditions"
            elif description in STEP_FAMILIES:
                statements = [
                    r for r in records
                    if r["kind"] == "statement" and r["outcome"] == description
                    and is_arithmetic_step(r)
                ]
                if not statements:
                    continue
                label = step_class(statements[0], declared)
                family = "steps"
            else:
                continue
            entry = sizes[family][label]
            entry[0] += 1
            entry[1].add(fkey)
            if families == {description}:
                entry[2].add(fkey)

    result = {
        family: {
            label: {"leaves": n, "functions": len(fs), "sole": len(sole)}
            for label, (n, fs, sole) in sorted(classes.items())
        }
        for family, classes in sizes.items()
    }
    out_json.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    lines = ["W43-INDEX-TRAVERSAL residual traversal shapes (Unknown functions)", ""]
    for family, classes in result.items():
        lines.append(f"{family}: class leaves functions sole")
        for label, size in classes.items():
            lines.append(f"  {label:14} {size['leaves']:6} {size['functions']:6} {size['sole']:5}")
        lines.append("")
    out_txt.write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
