#!/usr/bin/env python3
"""RETENTION-UNMATCHED-REMOVAL-POISONS: the ledger update (idempotent).

`RETENTION-UNMATCHED-REMOVAL-CLEARS-STATUS` (the defect the
FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME census found) is recorded in the report
and not registered: this card fixes it, so it would be registered and retired
in the same change. Registers the two residuals with sizes:
`RETENTION-DERIVED-LOOP-PATH-EQUALITIES` (decision D3, more than a small
amount of code) and `RETENTION-PATH-LOOP-DESTROY-ADMISSION` (a destroy at a
path is refused by the access gate before rule 12t reads it). Writes the
matrix with indent=1, ensure_ascii=False and dimensions.named_followups = the
list length."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "../../../.."))
MATRIX = os.path.join(REPO, "docs/reviews/r0_3/acceptance_matrix.json")
SOURCE = "docs/reports/retention-unmatched-removal-poisons/README.md"
CARD = "RETENTION-UNMATCHED-REMOVAL-POISONS"

MEASURED = ("(BEFORE binary built from the git archive of main 3f8f1a617, AFTER from the tip 6710d60a8; "
            "corpus 939 / 9 / 3,886, system 95 / 4 / 349, slice 52 / 1 / 347 / 4 Pending and the null "
            "property unchanged on every route)")

ENTRIES = [
    {
        "id": "RETENTION-DERIVED-LOOP-PATH-EQUALITIES",
        "title": "Two loop paths are one loop only by a derivation from single proven stores and an immutable flag",
        "why": (
            "Registered by " + CARD + " " + MEASURED + " on USER DECISION 2026-10-17 D3 (\"Yes, derived "
            "only\"), kept out of the card because it needs the run-wide store census of the pre-pass. Rule "
            "12t now poisons a record when a removal names a holder that does not match it exactly, and two "
            "distinct paths are never one loop, so a removal through an equal but differently spelled path "
            "poisons rather than ends a record. The derivations the FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME "
            "census names: config->sched->evl == config->evl (flb_engine.c:1288/1293 and flb_scheduler.c:"
            "753/883: one store into flb_config.sched, one into flb_sched.evl from the creating call's evl "
            "actual) and flb_input_collector.evl == flb_input_collector.instance.config.evl under "
            "!flb_input_is_threaded(ins) (collector_create's two stores, the threaded flag immutable after "
            "init, rule 10p). Size: no row on the corpus, system or slice route holds a record that such an "
            "equality would end today (every Fluent Bit registration and removal are in different "
            "functions; the census of " + CARD + " finds no rule-12t leaf with a removal in its body). "
            "DISCHARGE: the guarded equality derived from the run's stores (any extra store into either "
            "member, or any write to the flag, drops it), used by a removal or destroy to end a record at an "
            "equal path, with Lean parity (an unproven equality never ends a record; a derived one ends "
            "exactly the records the equal path would), measured on the system and slice routes with every "
            "moved row read."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "RETENTION-PATH-LOOP-DESTROY-ADMISSION",
        "title": "A loop destroy at a member path is refused by the access gate before rule 12t ends its records",
        "why": (
            "Registered by " + CARD + " " + MEASURED + ". Rule 12t's model ends every record at a holder on "
            "its loop's destroy, a path included (QCore.RetentionPath.a_destroy_ends_every_record_at_its_loop, "
            "retained_by::Event::Destroy), and mk_event_loop_destroy is declared "
            "resource_released(loop, nonnull). On route G a destroy at a path (mk_event_loop_destroy("
            "config->evl)) releases a member actual, a custody-taking position the access gate keeps refused "
            "(W44-INDIRECT-CALL-ARGUMENT-ACCESS premise 1), so only a tracked loop's destroy ends records "
            "(tests/event_loop_retention.rs a_loop_destroy_at_a_path_stays_refused). Size: the census of "
            "FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME counts 11 loop destroys at 66910c10, 046accf6 and "
            "90978ac6, every one at a member path (config->evl, thi->evl, ctx->event_loop, log->evl, "
            "th_ins->evl, worker->event_loop, ctx->aws_pod_service_event_loop); none is in a body that "
            "registers at the same path, so no row moves today. DISCHARGE: a destroy of a caller's loop at a "
            "path admitted as the end of that path's records (custody of the member stays the caller's, or "
            "is decided by a member-custody rule), with Lean parity binding the route to Event::Destroy, "
            "measured with every moved row read."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    with open(MATRIX, encoding="utf-8") as f:
        matrix = json.load(f)
    followups = matrix["named_followups"]
    ids = {entry["id"] for entry in followups}
    for entry in ENTRIES:
        if entry["id"] in ids:
            for existing in followups:
                if existing["id"] == entry["id"]:
                    existing.update(entry)
        else:
            followups.append(entry)
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(MATRIX, "w", encoding="utf-8") as f:
        json.dump(matrix, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print("named_followups", len(followups))
    return 0


if __name__ == "__main__":
    sys.exit(main())
