#!/usr/bin/env python3
"""rename_identity.py BEFORE.jsonl AFTER.jsonl -- NULL-MEMBER-INVARIANT-SCOPE:
the `nonnull_` rename changes text only. Every null row of BEFORE, with the
four old spellings replaced by the new ones, must equal the AFTER row of the
same (library, file, function), compared as parsed JSON (the run order of
rows differs at four jobs), apart from the bounds environment's digest
(`identity.contracts_sha256`), which the renamed module text changes; the
digests seen on each side are printed. Prints the counts and every differing row's key;
exit 1 on any difference."""
import json
import re
import sys

OLD_NEW = [(r"\bnull_member_nonnull\b", "nonnull_member"), (r"\bnull_result_nonnull\b", "nonnull_result"),
           (r"\bnull_member_scope\b", "nonnull_member_scope"), (r"\bnull_member_stable\b", "nonnull_member_stable")]


DIGESTS = {True: set(), False: set()}


def load(path, rename):
    rows = {}
    for line in open(path):
        if not line.strip():
            continue
        if rename:
            for old, new in OLD_NEW:
                line = re.sub(old, new, line)
        row = json.loads(line)
        DIGESTS[rename].add((row.get("identity") or {}).pop("contracts_sha256", None))
        rows[(row["library"], row["file"], row["function"])] = row
    return rows


before = load(sys.argv[1], True)
after = load(sys.argv[2], False)
differ = [key for key in sorted(set(before) | set(after)) if before.get(key) != after.get(key)]
verdicts = sum(1 for key in set(before) & set(after)
               if (before[key].get("null") or {}).get("verdict") != (after[key].get("null") or {}).get("verdict"))
print(json.dumps({"rows": [len(before), len(after)], "rows_differing_after_renaming": len(differ),
                  "verdicts_differing": verdicts,
                  "contracts_sha256": [sorted(map(str, DIGESTS[True])), sorted(map(str, DIGESTS[False]))], "first": [list(k) for k in differ[:10]]}))
sys.exit(1 if differ else 0)
