#!/usr/bin/env python3
"""Card W43-INDIRECT-ACCESS-GENERAL: why the forecast mirror missed a removed
intercepted leaf. For every `removed:rule-unforecast` row of an attribution,
report whether the census explained the leaf at all and, if it did, the
statement texts the mirror refused.

Usage: unforecast.py ATTRIBUTION_JSON CENSUS_RUN OUT_TXT
"""

import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import shape_census as census  # noqa: E402
import typed_scalar  # noqa: E402


def main():
    attribution = json.load(open(sys.argv[1]))
    run = pathlib.Path(sys.argv[2])
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    indirect = census.load_stream(run / "indirect-census.jsonl", lambda record: census.INDIRECT)
    statements = census.load_stream(
        run / "statement-census.jsonl", lambda record: census.outcome_family(record["outcome"])
    )
    assignment = census.match_units([indirect, statements], unit_keys)
    by_unit = {"/".join(map(str, k)): k for k in unit_keys}
    reasons = collections.Counter()
    lines = []
    for function, line, category, description, n in attribution["rows"].get("removed:rule-unforecast", []):
        parts = function.split("/")
        ukey = by_unit.get("/".join(parts[:-1]))
        seq = assignment.get(ukey)
        key = (parts[-1], line, description)
        records = statements.get(seq, {}).get(key, []) + indirect.get(seq, {}).get(key, [])
        if not records:
            reasons[(census.normalize(description), "not explained by the census")] += n
            lines.append(f"{function} L{line} [{description}] not explained")
            continue
        refused = [r for r in records if not typed_scalar.classify(r)]
        reasons[(census.normalize(description), "mirror refused a statement")] += n
        for record in refused[:1]:
            lines.append(f"{function} L{line} [{description}] mirror refused: {' '.join(record['texts'])[:140]}")
    out = ["reasons:"]
    for (description, reason), n in reasons.most_common():
        out.append(f"  {n:4}  {reason:30}  {description}")
    out += [""] + lines
    pathlib.Path(sys.argv[3]).write_text("\n".join(out) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
