#!/usr/bin/env python3
"""Apply W47-REVIEW-HOLD-CONCRETE-REASON's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires W45-REVIEW-HOLD-UNACCEPTABLE-AS-WRITTEN on the user decision of
  2026-09-29: such a marker stays Pending and the published verdict carries
  its concrete reason. Discharged by the `rewrite_before_acceptance`
  disposition (contract specification C06.1), the `review_holds` row member
  (gateway rule 10i as amended), `QCore.ReviewHoldStanding.ConcreteReason`,
  and the slice measurement (`src/flb_lib.c::flb_input`, both rows, Pending
  with its three `flb_input_new` markers named, two of them
  must-be-rewritten-before-acceptance);
* appends one sentence to W47-INSTANCE-FLOW-CONTRACT-REVIEW and
  W47-LIB-ENGINE-THREAD-REVIEW naming the markers now carrying the
  disposition, which the reviewer confirms or reverts;
* registers every entry of `followups.json` not yet registered.

It applies what it can and reports, rather than refuses, an entry already
retired, amended or registered. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-review-hold-concrete-reason"
RETIRE = {"W45-REVIEW-HOLD-UNACCEPTABLE-AS-WRITTEN"}
AMEND = {
    "W47-INSTANCE-FLOW-CONTRACT-REVIEW": (
        " W47-REVIEW-HOLD-CONCRETE-REASON: the three retained-pointer markers "
        "(flb_input's data; flb_input_new's config and data) now end with "
        "`rewrite_before_acceptance` (C06.1), so every row holding those calls publishes "
        "them as must-be-rewritten-before-acceptance; the review confirms that category "
        "or removes the word, and re-pins at the three trees."
    ),
    "W47-LIB-ENGINE-THREAD-REVIEW": (
        " W47-REVIEW-HOLD-CONCRETE-REASON: flb_start's and "
        "flb_downstream_worker_runtime_stop's markers now end with "
        "`rewrite_before_acceptance` (C06.1), read from their own reasons (ctx retained "
        "until the join, which resource_borrowed denies; runtime not freed on the -1 "
        "return, which resource_released asserts); the review confirms that category or "
        "removes the word, and re-pins at the three trees."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = len(entries)
    ids = {entry["id"] for entry in entries}
    for retired in sorted(RETIRE - ids):
        print(f"{retired}: not registered here (already retired)")
    kept = [dict(entry) for entry in entries if entry["id"] not in RETIRE]
    amended = 0
    for entry in kept:
        sentence = AMEND.get(entry["id"])
        if sentence is None:
            continue
        if sentence.strip() in entry["why"]:
            print(f"{entry['id']}: already amended")
            continue
        entry["why"] = entry["why"].rstrip() + sentence
        amended += 1
    for missing in sorted(set(AMEND) - ids):
        print(f"{missing}: not registered here; amendment not applied")
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            print(f"{entry['id']}: already registered")
    added = [entry for entry in new if entry["id"] not in ids]
    kept.extend(added)
    if kept == entries:
        print(f"follow-ups {before}: already applied")
        return 0
    matrix["named_followups"] = kept
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(kept)}; registered {len(added)}; "
          f"retired {before - len(kept) + len(added)}; amended {amended}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
