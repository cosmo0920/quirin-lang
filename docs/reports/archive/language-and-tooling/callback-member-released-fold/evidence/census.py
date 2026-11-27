#!/usr/bin/env python3
"""CALLBACK-MEMBER-RELEASED-REQUIREMENT-FOLD census.

For every shipped provider declaration (providers/*/contracts/**/*.qv) that
states an invocation clause (resource_callback_invoked[_or_default],
resource_callback_table, resource_callback_member_invoked,
resource_callback_slot_invoked), list each parameter actual and the member
object with its declared custody mode. A candidate's resource_owned(n)
requirement lands on a parameter actual of the call; the defect class is a
requirement landing on a position that is neither borrowed nor non-null.
Read-only; prints JSON. Usage: census.py <repository root>
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
INVOKE = re.compile(
    r"ensures normal (resource_callback_(?:invoked_or_default|invoked|table"
    r"|member_invoked|slot_invoked))\(([^;]*)\);"
)
MODE = re.compile(
    r"ensures normal resource_(borrowed|owned|consumed|released|out|stored|removed)"
    r"\(([^)]*)\)"
)
RESERVED = {"_", "element", "supplied_owned", "supplied_nullable"}
MEMBER_FORMS = {"resource_callback_member_invoked", "resource_callback_slot_invoked"}
DECLARATION = re.compile(r"extern fn (\w+)\(([^)]*)\)[^{]*contract \{(.*?)\n\};", re.S)


def at_risk(entry):
    return not (entry["mode"] == "borrowed" or entry["mode"].endswith("(nonnull)"))


def main():
    rows = []
    for path in sorted(ROOT.glob("providers/*/contracts/**/*.qv")):
        text = path.read_text()
        for match in DECLARATION.finditer(text):
            name, _params, body = match.groups()
            clauses = INVOKE.findall(body)
            if not clauses:
                continue
            modes = {}
            for kind, args in MODE.findall(body):
                parts = [p.strip() for p in args.split(",")]
                rest = ",".join(parts[1:])
                modes[parts[0]] = f"{kind}({rest})" if rest else kind
            for keyword, args in clauses:
                parts = [p.strip() for p in args.split(",")]
                if keyword in MEMBER_FORMS:
                    obj, actuals = parts[0], parts[2:]
                else:
                    obj, actuals = None, parts[1:]
                rows.append(
                    {
                        "file": str(path.relative_to(ROOT)),
                        "function": name,
                        "clause": keyword,
                        "object": None
                        if obj is None or obj == "supplied_owned"
                        else {"name": obj, "mode": modes.get(obj, "borrowed")},
                        "parameter_actuals": [
                            {"name": a, "mode": modes.get(a, "borrowed")}
                            for a in actuals
                            if a not in RESERVED
                        ],
                    }
                )
    summary = {
        "clauses": len(rows),
        "declarations": len({(r["file"], r["function"]) for r in rows}),
        "by_form": {},
        "non_borrowed_parameter_actuals": [],
        "non_borrowed_objects": [],
    }
    for row in rows:
        summary["by_form"][row["clause"]] = summary["by_form"].get(row["clause"], 0) + 1
        for actual in row["parameter_actuals"]:
            if actual["mode"] != "borrowed":
                summary["non_borrowed_parameter_actuals"].append(
                    {
                        "function": row["function"],
                        "clause": row["clause"],
                        **actual,
                        "at_risk": at_risk(actual),
                    }
                )
        if row["object"] and row["object"]["mode"] != "borrowed":
            summary["non_borrowed_objects"].append(
                {
                    "function": row["function"],
                    "clause": row["clause"],
                    **row["object"],
                    "also_an_actual": any(
                        a["name"] == row["object"]["name"] for a in row["parameter_actuals"]
                    ),
                    "at_risk": at_risk(row["object"]),
                }
            )
    json.dump(summary, sys.stdout, indent=1, sort_keys=True)
    print()


if __name__ == "__main__":
    main()
