#!/usr/bin/env python3
"""alias_uses.py RUN RUN-PARITY.jsonl [OUT.json] -- card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2: the Unknown functions in which
rule 14b bound an alias, split by whether every remaining leaf is an
unresolved identity of one of those aliases (the dereference of a target-free,
possibly-null alias), i.e. how many a non-null or tracked reading of the alias
would be the only thing left to decide."""
import collections
import json
import pathlib
import re
import sys

UNRESOLVED = re.compile(r"^unresolved resource identity `([^`]*)`$")


def main() -> int:
    aliases = collections.defaultdict(set)
    with open(sys.argv[2], encoding="utf-8") as stream:
        for line in stream:
            if '"residual-access"' not in line:
                continue
            row = json.loads(line)
            if row.get("position") != "residual-access" or row.get("rule") != "14b" \
                    or not row.get("new_admitted"):
                continue
            texts = row.get("texts") or []
            if "=" in texts:
                aliases[row["function"]].add(texts[texts.index("=") - 1])
    report = json.loads((pathlib.Path(sys.argv[1]) / "resource-diagnostics.json").read_text())
    only, other = [], []
    for unit in report["units"]:
        for function in unit["functions"]:
            name = function["function"]
            if name not in aliases or function["status"] != "Unknown":
                continue
            leaves = function.get("diagnostics") or []
            names = [UNRESOLVED.match(leaf.get("description", "")) for leaf in leaves]
            if leaves and all(m and m.group(1) in aliases[name] for m in names):
                only.append([unit["file"], name, sorted(aliases[name])])
            else:
                other.append([unit["file"], name])
    record = {"alias_uses_only": len(only), "other": len(other), "functions": only}
    print(json.dumps({k: record[k] for k in ("alias_uses_only", "other")}))
    for row in only:
        print("  ", row)
    if len(sys.argv) > 3:
        pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
