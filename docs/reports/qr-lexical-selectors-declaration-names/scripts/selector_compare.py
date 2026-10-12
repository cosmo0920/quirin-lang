#!/usr/bin/env python3
"""selector_compare.py CENSUS.jsonl [OUT.json] -- QR-LEXICAL-SELECTORS-DECLARATION-NAMES.

Compares every row of the declaration-name census
(compiler/tests/qr_lexical_selector_census.rs) with its control (same mode,
base and shape, the word replaced by `flb_noop`) and classifies it:

* `same`: it parses, every selector and the import gate read as the
  control's, and the compile outcome has the control's class (admitted on the
  same mode/profile/variant, or refused with the same codes).
* `selection-moved`: it parses and at least one selector or the import gate
  reads differently from the control. Each moved selector is named.
* `parse-refused`: the word cannot stand in that position at all (the parser
  refuses it; the control parses). Not a selection: no selector ran.
* `outcome-only`: the selection is the control's but the compile outcome
  class differs (a later check reads the name). Listed for reading.

Writes a JSON summary (counts by class, by family and selector, and every
non-`same` row) to OUT.json when given, and prints the counts.
"""
import json
import sys
from collections import Counter, defaultdict


def outcome_class(compile_row):
    if compile_row["outcome"] == "admitted":
        return ("admitted", compile_row["mode"], compile_row["profile"], compile_row["checked"])
    if compile_row["outcome"] == "refused":
        return ("refused", tuple(compile_row["codes"]))
    return (compile_row["outcome"],)


def main():
    rows = [json.loads(line) for line in open(sys.argv[1])]
    controls = {}
    for row in rows:
        if row["word"] == "flb_noop":
            controls[(row["mode"], row["base"], row["shape"])] = row
    classes = Counter()
    by_selector = Counter()
    by_family = defaultdict(Counter)
    listed = []
    for row in rows:
        if row["word"] == "flb_noop":
            continue
        control = controls[(row["mode"], row["base"], row["shape"])]
        assert control["selection"]["parsed"], control
        if not row["selection"]["parsed"]:
            kind = "parse-refused"
            moved = []
        else:
            mine = row["selection"]["selectors"]
            theirs = control["selection"]["selectors"]
            moved = sorted(key for key in mine if mine[key] != theirs[key])
            if moved:
                kind = "selection-moved"
            elif outcome_class(row["compile"]) != outcome_class(control["compile"]):
                kind = "outcome-only"
            else:
                kind = "same"
        classes[kind] += 1
        by_family[row["family"]][kind] += 1
        for key in moved:
            by_selector[key] += 1
        if kind != "same":
            entry = {
                "mode": row["mode"],
                "base": row["base"],
                "shape": row["shape"],
                "word": row["word"],
                "family": row["family"],
                "class": kind,
            }
            if moved:
                entry["moved"] = {
                    key: [control["selection"]["selectors"][key], row["selection"]["selectors"][key]]
                    for key in moved
                }
            if kind == "parse-refused":
                entry["error"] = row["selection"]["error"]
            entry["control_outcome"] = list(outcome_class(control["compile"]))
            entry["outcome"] = list(outcome_class(row["compile"]))
            if row["compile"].get("diagnostics"):
                entry["first_diagnostic"] = row["compile"]["diagnostics"][0].splitlines()[0]
            listed.append(entry)
    summary = {
        "rows": len(rows),
        "controls": len(controls),
        "classes": dict(sorted(classes.items())),
        "moved_selectors": dict(sorted(by_selector.items())),
        "families": {family: dict(sorted(c.items())) for family, c in sorted(by_family.items())},
        "listed": listed,
    }
    print(json.dumps({k: summary[k] for k in ("rows", "controls", "classes", "moved_selectors")}, indent=1))
    if len(sys.argv) > 2:
        with open(sys.argv[2], "w") as out:
            json.dump(summary, out, indent=1, ensure_ascii=False)
            out.write("\n")


if __name__ == "__main__":
    main()
