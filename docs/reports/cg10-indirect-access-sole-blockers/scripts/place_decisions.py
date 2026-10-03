#!/usr/bin/env python3
"""place_decisions.py PARITY.jsonl [PARITY.jsonl ...] -- CG10-INDIRECT-ACCESS-SOLE-BLOCKERS:
the typed-place decisions of each parity stream (rule 10t's refusal codes, rule
12j's record-object decisions), deduplicated per (function, line, statement
tokens, place), so a body the W4 fixed point classifies more than once counts
once. Also the rule-12i consultations (`return-call-access`), deduplicated per
(function, line, tokens), by form and answer.
"""
import collections
import json
import sys


def main():
    for path in sys.argv[1:]:
        seen = set()
        codes = collections.Counter()
        record = collections.Counter()
        returns = collections.Counter()
        with open(path, encoding="utf-8") as stream:
            for line in stream:
                if '"typed-place"' in line:
                    row = json.loads(line)
                    for place in row.get("places") or []:
                        key = (row["function"], row["line"], tuple(row["texts"]),
                               json.dumps(place, sort_keys=True))
                        if key in seen:
                            continue
                        seen.add(key)
                        if place.get("rule") == "12j":
                            obj = place["record_object"]
                            record[(obj["storage"], obj["access"],
                                    "admit" if place["admitted"] else place["refusal"])] += 1
                        else:
                            codes["admitted" if place.get("admitted") else place.get("refusal")] += 1
                elif '"return-call-access"' in line:
                    row = json.loads(line)
                    key = ("r", row["function"], row["line"], tuple(row["texts"]))
                    if key in seen:
                        continue
                    seen.add(key)
                    returns[(row["form"], row["new_admitted"])] += 1
        print(json.dumps({
            "stream": path,
            "rule_10t_place_decisions": dict(codes.most_common()),
            "rule_12j_decisions": {"/".join(k): v for k, v in record.most_common()},
            "rule_12i_consultations": {f"{form}/{'admitted' if ok else 'refused'}": v
                                       for (form, ok), v in returns.most_common()},
        }, indent=1))


if __name__ == "__main__":
    main()
