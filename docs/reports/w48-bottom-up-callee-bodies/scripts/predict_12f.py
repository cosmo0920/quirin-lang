#!/usr/bin/env python3
"""W48-BOTTOM-UP-CALLEE-BODIES: the frozen prediction's candidate set for
rule 12f (a scalar return value read as a statement value).

Usage: predict_12f.py DIAGNOSTICS RETURN_REASONS_JSON OUT_JSON

A lexical approximation of the W43 value walker over the return leaves'
parity texts: no held call, no `&`, `[`, `=`, `?`, increment or comma, at
least one `.` or `->`, and no residual feature naming a pointer or array
value. The walker itself also requires every selected member to be a scalar
of a completely read record and every name a declared scalar, so this is an
upper bound on the leaves the rule can admit. The closure then frees the
Unknown functions whose every non-call leaf is such a candidate and whose
every call is served by a Safe same-unit body (same-unit route; a freed
function counts Safe in the next round).
"""
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import closure_r3  # noqa: E402
import held_calls  # noqa: E402

FAMILY = "unsupported: unmodeled call, aggregate or indirect access in return expression"
POINTERISH = ("const-pointer", "function-pointer", "array-value", "array-decay")


def main() -> int:
    diagnostics, reasons, out = sys.argv[1:4]
    candidates = {}
    for row in json.loads(pathlib.Path(reasons).read_text()):
        texts = (row["texts"] or "").split()
        if not texts or held_calls.held(texts):
            continue
        if any(t in texts for t in ("&", "[", "=", "?", "++", "--", ",")):
            continue
        if "." not in texts and "->" not in texts:
            continue
        if row["features"] and any(f in row["features"] for f in POINTERISH):
            continue
        candidates[(row["function"], row["line"])] = {"owners": row["owners"], "texts": row["texts"],
                                                      "features": row["features"]}
    defined_in, functions = closure_r3.analyse(closure_r3.load(diagnostics))
    unknown = {k for k, v in functions.items() if v["status"] == "Unknown"}
    safe = {k for k, v in functions.items() if v["status"] == "Safe"}
    freed, number = {}, 0
    while True:
        added = [k for k in unknown if k not in freed
                 and all(closure_r3.family(l) == FAMILY and (k[1], l.get("line")) in candidates
                         for l in functions[k]["others"])
                 and all(n in defined_in and k[0] in defined_in[n] and (k[0], n) in safe
                         for n in functions[k]["calls"])]
        if not added:
            break
        for k in added:
            freed[k] = number
        safe |= set(added)
        number += 1
    document = {"candidate_leaves": [{"function": k[0], "line": k[1], **v} for k, v in sorted(
        candidates.items(), key=lambda kv: (kv[0][0], kv[0][1] or 0))],
        "freed_upper_bound": sorted(f"{k[0]}::{k[1]}@{r}" for k, r in freed.items())}
    pathlib.Path(out).write_text(json.dumps(document, indent=1) + "\n")
    for k, v in sorted(candidates.items()):
        print(f"{v['owners'][0]} | {v['features']} | {v['texts'][:90]}")
    print(f"candidate leaves {len(candidates)}; freed (upper bound) {len(freed)}:")
    for name in document["freed_upper_bound"]:
        print("   ", name)
    return 0


if __name__ == "__main__":
    sys.exit(main())
