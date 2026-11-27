#!/usr/bin/env python3
"""member_pairs.py PARITY_JSONL [PARITY_JSONL ...] -- CG18-FOOTPRINT-CONNECTION:
the `footprint-member-pair` rows of parity streams (rule 11k), one row per
(source, pair, decision) with its consultation count, as JSON on stdout.
"""
import collections
import json
import sys


def main() -> int:
    counts = collections.Counter()
    for path in sys.argv[1:]:
        with open(path, encoding="utf-8") as stream:
            for line in stream:
                row = json.loads(line)
                if row.get("position") != "footprint-member-pair":
                    continue
                counts[(path.rsplit("/", 1)[-1], row["source_sha256"][:12], row["one"],
                        row["other"], row["decision"])] += 1
    rows = [
        {"stream": key[0], "source": key[1], "one": key[2], "other": key[3],
         "decision": key[4], "consultations": value}
        for key, value in sorted(counts.items())
    ]
    print(json.dumps({"pairs": rows}, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
