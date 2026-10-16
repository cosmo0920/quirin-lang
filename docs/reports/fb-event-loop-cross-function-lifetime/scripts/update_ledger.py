#!/usr/bin/env python3
"""FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME: the ledger update (idempotent).

USER DECISIONS 2026-10-17 (README, "User decisions"): narrows
FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME to S3 declared lifetime protocols after
the blockers card; registers FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS (sized per
row from artifacts/chain-blockers.json) and FB-UNJOINED-TEARDOWN-READINGS
(R1 and R4, rule 10q / C5, readings and not findings). It does not register
RETENTION-UNMATCHED-REMOVAL-CLEARS-STATUS: the fix card
RETENTION-UNMATCHED-REMOVAL-POISONS owns it. Writes the matrix with indent=1,
ensure_ascii=False and dimensions.named_followups = the list length."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPORT = os.path.dirname(HERE)
REPO = os.path.abspath(os.path.join(HERE, "../../../.."))
MATRIX = os.path.join(REPO, "docs/reviews/r0_3/acceptance_matrix.json")
SOURCE = "docs/reports/fb-event-loop-cross-function-lifetime/README.md"
CARD = "FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME"


def sizes():
    with open(os.path.join(REPORT, "artifacts", "chain-blockers.json"), encoding="utf-8") as f:
        data = json.load(f)
    parts = []
    for route, r in data["routes"].items():
        s = r["summary"]
        heavy = sorted((row for row in r["rows"] if "#" not in row["unit"]),
                       key=lambda row: -row["blockers"])
        per_row = ", ".join(f"{row['function']} {row['leaves'].get('access', 0)}/{row['leaves'].get('identity', 0)}"
                            for row in heavy if row["blockers"])
        parts.append(
            f"{route}: {s['rows']} chain rows ({s['rows_first_configuration']} in the first configuration), "
            f"{s['rows_with_blockers']} with an access or identity leaf, {s['rows_only_blockers']} with only "
            f"those; {s['access_leaves']} access and {s['identity_leaves']} identity leaves (statuses "
            f"{json.dumps(s['statuses'], sort_keys=True)}); per row, access/identity, first configuration: {per_row}")
    return "; ".join(parts)


def main():
    measured = ("(BEFORE at main 3f8f1a617, binary fb1e08ca, system 95 / 4 / 349 and slice 52 / 1 / 347 / 4 "
                "Pending, equal to round 31; artifacts/chain-blockers.json)")
    narrowed_title = "S3 declared lifetime protocols for event-loop registrations, after the blockers card"
    narrowed_why = (
        "Narrowed by " + CARD + " (census and design, records only; README sections 1-6, PREDICTION.md) on "
        "USER DECISIONS 2026-10-17: D1/D5 \"S3 declared protocols\" (the lifetime model uses declared lifetime "
        "protocols at record types, each checked against every registration and store site of the run, "
        "a contradicted declaration withdrawn and an uncheckable one a declared assumption, rule 13i); D3 "
        "\"Yes, derived only\" (a guarded loop-path equality only from single proven stores and an immutable "
        "flag, e.g. flb_input_collector.evl's two collector_create stores under flb_input_is_threaded); D6 "
        "\"Blockers first\" (FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS lands before this); D2 (destroy at a path "
        "ends its records, an unmatched removal poisons) belongs to RETENTION-UNMATCHED-REMOVAL-POISONS. The "
        "census found 19 loop members (17 closed candidate sets over 7 allocation sites; 51 registrations, 37 "
        "removals, 11 destroys at 66910c10, 046accf6 and 90978ac6) and one held key whose terminal removal "
        "names another member (flb_input_collector.event: registered at flb_input_collector.evl, removed by "
        "flb_input_collector_destroy at flb_config.evl). No row is predicted to move under any variant until the "
        "blockers clear. DISCHARGE: the user's S3 syntax decision (D4: the clause-kind word, e.g. `quant "
        "registered`, and member guards), then the declarations, their body checks and the composition "
        "implemented with Lean parity (a validated protocol equals the derived reading, a withdrawn one never "
        "decides, an unproven equality never ends a record), measured on the system and slice routes with every "
        "moved row read.")
    registered = [
        {
            "id": "FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS",
            "title": "Access-gate and identity refusals on Fluent Bit's event-loop registration and teardown chains",
            "why": (
                "Registered by " + CARD + " on USER DECISION 2026-10-17 D6 (\"Blockers first\") " + measured + ". "
                "Every row of the chains R1-R6 (README section 1.4: collector registration and teardown, "
                "flb_input_instance_destroy / flb_input_exit_all, the input thread and its instance, the engine "
                "and library lifecycle, output workers, the scheduler's timers and requests) refuses at member "
                "accesses (`indirect memory access is outside the resource projection`, nested or unmodeled "
                "calls) and identities (`unresolved resource identity`) before any retention record is formed "
                "(collector_start never reaches rule 12t), so no lifetime model can move them. Sizes: " + sizes() +
                ". DISCHARGE: each such leaf on these rows cleared, or named with the rule that keeps it, so that a "
                "lifetime model can show movement; measured on the system and slice routes with every moved row "
                "read."),
            "source": SOURCE,
            "recorded_by": CARD,
        },
        {
            "id": "FB-UNJOINED-TEARDOWN-READINGS",
            "title": "Teardown paths that release loop-registered objects without a checked join (readings, not findings)",
            "why": (
                "Registered by " + CARD + " (README sections 1.4 and 1.5; rule 10q, C5) as readings, never "
                "findings or Unsafe, at 66910c10, 046accf6 and 90978ac6 (bodies identical). R1: "
                "flb_input_thread_instance_exit returns -1 without joining when its exit message is not written "
                "(src/flb_input_thread.c:601-605) and otherwise joins without checking (:607); "
                "flb_input_instance_exit drops the result (src/flb_input.c:2134), and flb_input_exit_all then runs "
                "flb_input_instance_destroy, whose flb_input_collector_destroy resets &coll->event's status "
                "through config->evl (:3146/:3150) and frees coll (:3153), then frees ins, while on the unjoined "
                "path the input thread may still wait on thi->evl holding &coll->event and the instance channel. "
                "R4: flb_output_thread_pool_destroy frees th_ins without a join when the stop write fails "
                "(src/flb_output_thread.c:640-644; also W47-ENGINE-THREAD-JOIN-CANDIDATES (iv)) while the worker "
                "may still run on th_ins->evl, and joins unchecked otherwise (:646). Not validated: no trace or "
                "reproduction; the R1 write fails only when write on a blocking pipe returns <= 0. DISCHARGE: each "
                "path decided by rule 10q's census (join admitted, or the release refused with the path named) "
                "on the slice and system routes, or a validated trace before anything is called a defect."),
            "source": SOURCE,
            "recorded_by": CARD,
        },
    ]
    with open(MATRIX, encoding="utf-8") as f:
        matrix = json.load(f)
    entries = matrix["named_followups"]
    before = len(entries)
    for entry in entries:
        if entry["id"] == CARD and "Narrowed by " + CARD not in entry["why"]:
            entry["title"] = narrowed_title
            entry["why"] = (entry["why"].rstrip() + " AMENDED (" + CARD + ", report " + SOURCE + "): "
                            + narrowed_why)
    ids = {e["id"] for e in entries}
    for new in registered:
        if new["id"] not in ids:
            entries.append(new)
    assert "RETENTION-UNMATCHED-REMOVAL-CLEARS-STATUS" not in {e["id"] for e in entries}
    matrix["dimensions"]["named_followups"] = len(entries)
    with open(MATRIX, "w", encoding="utf-8") as f:
        json.dump(matrix, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
