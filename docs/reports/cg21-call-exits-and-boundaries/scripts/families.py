#!/usr/bin/env python3
"""CG21-CALL-EXITS-AND-BOUNDARIES: attribute every changed function of a
corpus comparison to the families of its leaf delta.

usage: families.py CHANGED_JSON [--json OUT]   (changed.py's --json output)

A function's delta is split into families by leaf:
  path-end        an uncontracted __assert_fail / abort / exit leaf removed
  refinement      an `unresolved resource identity` (unknown_alias) leaf
                  removed or added (the joined state after a path end)
  rename          an extension-node / assembly leaf renamed (the old and
                  new wording, same count)
  cascade         a leaf naming a callee (or a nested call) whose own body
                  is now Safe, removed
  other           anything else (listed)
and whether any violation kind (Leak, DoubleRelease, ...) changed.
"""

import argparse
import collections
import json
import pathlib
import re

PATH_END = re.compile(r"call to uncontracted function `(__assert_fail|abort|exit)`")
RENAMED_OLD = re.compile(
    r"call to uncontracted function `(__builtin_offsetof|__builtin_va_arg|__asm__)`"
)
RENAMED_NEW = ("is the Clang extension node", "inline assembly is an opaque boundary")
VIOLATIONS = ("leak", "double_release", "release_after_escape", "custody_conflict",
              "invalid_release", "use_after_release")


def family(leaf, moved_callees):
    category, _, description = leaf.partition(": ")
    if PATH_END.search(description):
        return "path-end"
    if category == "unknown_alias":
        return "refinement"
    if RENAMED_OLD.search(description) or any(s in description for s in RENAMED_NEW):
        return "rename"
    if any(f"`{name}`" in description for name in moved_callees):
        return "cascade"
    if category in VIOLATIONS:
        return "violation"
    return "other"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("changed", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    rows = json.loads(args.changed.read_text())
    moved = {row["function"] for row in rows if row["before"] != row["after"]}
    signatures = collections.Counter()
    others = []
    leaf_totals = collections.Counter()
    for row in rows:
        fams = set()
        for leaf, n in row["leaf_delta"].items():
            fam = family(leaf, moved)
            fams.add(fam)
            leaf_totals[(fam, leaf)] += n
            if fam in ("other", "violation"):
                others.append({"unit": row["unit"], "function": row["function"],
                               "move": f"{row['before']} -> {row['after']}", "leaf": leaf, "n": n})
        signatures[(row["before"] + " -> " + row["after"], tuple(sorted(fams)))] += 1
    record = {
        "functions": len(rows),
        "signatures": [{"move": m, "families": list(f), "functions": n}
                       for (m, f), n in sorted(signatures.items())],
        "leaf_totals": [{"family": f, "leaf": leaf, "delta": n}
                        for (f, leaf), n in sorted(leaf_totals.items())],
        "other": others,
    }
    for item in record["signatures"]:
        print(f"{item['functions']:4}  {item['move']:22} {'+'.join(item['families'])}")
    print("other / violation leaves:")
    for item in others:
        print("   ", item)
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
