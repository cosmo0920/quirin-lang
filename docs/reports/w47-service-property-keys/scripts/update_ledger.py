#!/usr/bin/env python3
"""Apply W47-SERVICE-PROPERTY-KEYS's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires W47-LOOP-OWNER-SERVICE-PROPERTY-KEYS: its DISCHARGE clause (a
  reading that evaluates a variadic call's literal keys against
  flb_config_set_property's key comparisons over the service_configs table,
  follows only the branches those keys take, fail-closed on any non-literal
  key, measured on the census) is met; at 90978ac6 configured the
  flb_service_set blocker is gone. The 3 sites it held stay unresolved on a
  different blocker, registered as W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS;
* registers every entry of `followups.json` not yet registered;
* appends this card's measured amendment to W47-INSTANCE-FLOW-CONTRACT-REVIEW,
  W47-LOOP-OWNER-COROUTINE-POST-YIELD and W47-LOOP-OWNER-INSTANCE-FLOW, once
  each.

It applies what it can and reports, rather than refuses, an entry already
registered, retired or amended, or missing. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-service-property-keys"
RETIRE = ["W47-LOOP-OWNER-SERVICE-PROPERTY-KEYS"]
AMEND = {
    "W47-INSTANCE-FLOW-CONTRACT-REVIEW": (
        " AMENDED by W47-SERVICE-PROPERTY-KEYS: fluent_bit/input_instance.qv gains a fifth "
        "declaration, flb_config_set_property (config held for review with the derived "
        "per-key effects as its reason; k, v borrowed and the scalar result bare), and "
        "flb_service_set's held variadic tail is reworded to the derived key-first pairing; "
        "both re-pinned at 66910c10, 046accf6 and the 90978ac6 export (5 declarations, 8 held "
        "clauses). DISCHARGE unchanged."
    ),
    "W47-LOOP-OWNER-COROUTINE-POST-YIELD": (
        " AMENDED by W47-SERVICE-PROPERTY-KEYS: unchanged at 90978ac6; the fixed test's 3 "
        "sites stay on this reason, their list-head derivation now held only by the "
        "predefined-macro arms of W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS (flb_service_set's "
        "keys are read)."
    ),
    "W47-LOOP-OWNER-INSTANCE-FLOW": (
        " AMENDED by W47-SERVICE-PROPERTY-KEYS: the list-head derivation reads a "
        "literal-keyed property setter call as reaching only its keys' branches "
        "(loop_owner_property_keys.rs); at 90978ac6 configured flb_service_set no longer "
        "counts as a second hazard-reaching call; the remaining blocker is the unmodelled-call "
        "one over 2 Windows-only definitions (W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS), and "
        "macro invocations on the region are not counted (W47-LOOP-OWNER-REGION-MACRO-CALLS)."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = len(entries)
    ids = {entry["id"] for entry in entries}
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            print(f"{entry['id']}: already registered")
    for name in RETIRE:
        if name not in ids:
            print(f"{name}: not registered (already retired or never registered)")
    kept = [dict(entry) for entry in entries if entry["id"] not in RETIRE]
    retired = before - len(kept)
    amended = 0
    for entry in kept:
        text = AMEND.get(entry["id"])
        if text is None:
            continue
        if text.strip() in entry["why"]:
            print(f"{entry['id']}: already amended")
        else:
            entry["why"] += text
            amended += 1
    for missing in sorted(set(AMEND) - ids):
        print(f"{missing}: not registered, amendment not applied")
    added = [entry for entry in new if entry["id"] not in ids]
    kept.extend(added)
    if kept == entries:
        print(f"follow-ups {before}: already applied")
        return 0
    matrix["named_followups"] = kept
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(kept)}; registered {len(added)}; retired {retired}; "
          f"amended {amended}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
