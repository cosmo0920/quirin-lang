#!/usr/bin/env python3
"""Apply W47-LOOP-OWNER-INSTANCE-FLOW's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires nothing: the fixed test's 3 sites stay unresolved at 90978ac6, so
  W47-LOOP-OWNER-INSTANCE-FLOW's DISCHARGE clause (a data-flow derivation of
  the instance) is met for the holder escape and P-CALLBACK-INSTANCE but not
  for the list-head instance on the test's path;
* registers every entry of `followups.json` not yet registered;
* appends this card's measured amendment to W47-LOOP-OWNER-INSTANCE-FLOW and
  W47-LOOP-OWNER-COROUTINE-POST-YIELD, once each.

It applies what it can and reports, rather than refuses, an entry already
registered or amended, or missing. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-loop-owner-instance-flow"
AMEND = {
    "W47-LOOP-OWNER-INSTANCE-FLOW": (
        " AMENDED by W47-LOOP-OWNER-INSTANCE-FLOW (90978ac6, both census readings): "
        "Step B now derives, with typed member-access facts, the holder escape (&test "
        "escapes only into the installed instance's data; struct tail_pause_test is named "
        "only in the test file; the copy member is written nowhere else; an instance's p is "
        "written only by flb_input_new and the test's installation) and P-CALLBACK-INSTANCE "
        "(every struct flb_input_plugin::cb_pre_run dispatch passes the instance whose p it "
        "goes through, or is the callback on its own first parameter; a collector's instance "
        "is written only by collector_create from its registration's first parameter), and "
        "a list-head instance's creation (loop_owner_instance.rs: the context from "
        "flb_create, exactly one hazard-reaching call on the test's path, a wrapper passing "
        "a literal plugin name to one flb_input_new call, every descriptor matching it "
        "ignoring case running no coroutine). At the export the list-head derivation stops "
        "on one call: flb_service_set (tests/runtime/in_tail.c:218), whose key-dependent "
        "Plugins_File branch registers plugins (W47-LOOP-OWNER-SERVICE-PROPERTY-KEYS); the "
        "3 sites stay unresolved, no site moves, and the old test's paths stay not "
        "discharged. The stated premises (P-HEAD-INSTANCE, P-FRESH-CONTEXT, "
        "P-REGISTERED-DESCRIPTORS) are recorded as review-held provider contracts "
        "(W47-INSTANCE-FLOW-CONTRACT-REVIEW). The general P-INSTANCE, threaded-flag and "
        "engine-label parts of this entry are untouched."
    ),
    "W47-LOOP-OWNER-COROUTINE-POST-YIELD": (
        " AMENDED by W47-LOOP-OWNER-INSTANCE-FLOW: unchanged at 90978ac6; the fixed test's "
        "3 sites stay on this reason while flb_service_set holds its list-head derivation "
        "(W47-LOOP-OWNER-SERVICE-PROPERTY-KEYS)."
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
    kept = [dict(entry) for entry in entries]
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
    print(f"follow-ups {before} -> {len(kept)}; registered {len(added)}; retired 0; "
          f"amended {amended}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
