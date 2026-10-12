#!/usr/bin/env python3
"""selector_outcomes.py BEFORE.jsonl AFTER.jsonl OUT.json -- QR-LEXICAL-SELECTORS-DECLARATION-NAMES.

For two runs of the declaration-name census (the base and the fix), every row
whose compile outcome class (admitted mode/profile/variant, or the refusal
codes) changes, grouped by (mode, base, shape, before, after) with the words
of each group and the first AFTER diagnostic line. The census itself is
compared to its controls by selector_compare.py; this file is what the fix
moved, controls included.
"""
import json
import sys


def load(path):
    return {(r["mode"], r["base"], r["shape"], r["word"]): r for r in map(json.loads, open(path))}


def outcome(compile_row):
    if compile_row["outcome"] == "admitted":
        return ["admitted", compile_row["mode"], compile_row["profile"], compile_row["checked"]]
    return [compile_row["outcome"], compile_row.get("codes", [])]


def first(compile_row):
    lines = (compile_row.get("diagnostics") or [""])[0].splitlines()
    return lines[0].split(": ", 1)[-1] if lines else ""


def main():
    before, after = load(sys.argv[1]), load(sys.argv[2])
    assert before.keys() == after.keys()
    groups = {}
    for key in sorted(before):
        a, b = outcome(before[key]["compile"]), outcome(after[key]["compile"])
        if a == b:
            continue
        group = groups.setdefault(
            json.dumps([key[0], key[1], key[2], a, b]),
            {"mode": key[0], "base": key[1], "shape": key[2], "before": a, "after": b,
             "words": [], "after_diagnostic": first(after[key]["compile"])},
        )
        group["words"].append(key[3])
    rows = sum(len(g["words"]) for g in groups.values())
    summary = {"rows": len(before), "moved_rows": rows, "groups": list(groups.values())}
    with open(sys.argv[3], "w") as out:
        json.dump(summary, out, indent=1, ensure_ascii=False)
        out.write("\n")
    print(f"rows {len(before)}; outcome moved {rows} in {len(groups)} groups")


if __name__ == "__main__":
    main()
