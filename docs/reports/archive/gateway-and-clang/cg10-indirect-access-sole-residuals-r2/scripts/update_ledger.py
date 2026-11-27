#!/usr/bin/env python3
"""update_ledger.py -- card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2:
amend the acceptance matrix's named follow-ups this round re-measured and
register its residual (indent=1, ensure_ascii=False;
dimensions.named_followups = list length). Idempotent: an amendment already
present is not appended twice, an entry already registered is not added
again."""
import json
import pathlib

PATH = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
TAG = "CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2"
RUNS = ("corpus 66910c10, BEFORE main 84ec54d1d ab8f5f14 881/9/3,944 (verdict rows 321a8913), "
        "AFTER 7a70c7b7 905/9/3,920 (verdict rows f08d6ce0); system 87/4/357 -> 89/4/355; "
        "slice 45/1/354/4 Pending, no status move")

AMEND = {
    "CG10-INDIRECT-ACCESS-SOLE-RESIDUALS": (
        "AMENDED (%s; %s; docs/reports/cg10-indirect-access-sole-residuals-r2): the "
        "population re-measured at 228 functions, 380 leaves (203 carried over, 25 exposed by "
        "later contracts). Ranked by functions freed by one fix alone (scripts/rank.py), no "
        "reader lift frees any function of the two uncontracted classes (a function there also "
        "needs its callee's contract: a lift re-attributes). Four readings: gateway rule 14a (a "
        "scalar statement whose value holds one call to a None-result, all-Borrowed contract: "
        "the call, then the typed reads; +8), 14b (a pointer variable given &PATH or PATH +/- n "
        "under never-bound roots binds the pointer read's target-free, possibly-null alias; +1, "
        "and the refusal moves to the alias's dereferences in most functions), 14c (rule "
        "10n(1)'s actual address may be a scalar element &PATH[I]; +7 with three summaries), 14d "
        "(an interior offset P +/- n at a Borrowed actual under the address's premise; +2); and "
        "two neutral declarations cio_errno_print, ctr_errno_print (body-validated; +6). +24 "
        "Unknown -> Safe, no Safe -> non-Safe move, no leak or violation added. RESIZED on the "
        "AFTER (artifacts/residual-after3.json): 201 functions, 333 leaves; functions freed alone "
        "/ leaves: uncontracted callee with an actual neither reader recognises 37 / 87, the "
        "same at a whole-call return 24 / 25, custody-position:released 13 / 52 (by design), "
        "pointer-store:other 12 / 15 (rule 10v, Q36), pointer-store:formal-pointer 11 / 16 "
        "(class 1 retained_by), probe-declined 6 / 11 (owned results and a descriptor stored into "
        "members, two calls in a value, a ?: actual), unclassified:CallArgument 6 / 6 (release "
        "of a caller's array element, rule 5k), whole-call return with no rule-12k reading 5, "
        "call-result-store 4, elaboration no value form 4, pointer-store:arithmetic 4, 26 "
        "smaller classes 35 (each at most 3), 40 functions with two or more classes; fixed point "
        "37, 61, 86, 99, 110, 117, .... Every remaining class above 3 is a design refusal, a "
        "decision class or a contract question; the entry closes when no class is left that an "
        "access reading alone blocks." % (TAG, RUNS)),
    "W45-CALLEE-CASCADE-RESIDUAL": (
        "AMENDED (%s; %s; scripts/callee_rank.py, artifacts/callee-rank-after3.json): the lifts "
        "left 14 more functions behind uncontracted callees alone, 523 -> 537 (by one callee "
        "alone 366 -> 379): rd_kafka_log0 3 (a configured logging callback), cfl_kvlist_create, "
        "ctr_id_set, msync, six opentelemetry__proto__*__init, "
        "rd_kafka_topic_partition_ensure_toppar 1 each; no contract written (none trivial). Two "
        "neutral declarations added and pinned, cio_errno_print (qcore-provider-chunkio) and "
        "ctr_errno_print (qcore-provider-ctraces), both body-validated, freed 6." % (TAG, RUNS)),
}

NEW = [
    {
        "id": "CG10-INTERIOR-ALIAS-NULLABILITY",
        "title": "7 Unknown functions are blocked only by dereferences of an interior alias rule 14b binds possibly null",
        "why": (
            "Registered by %s (%s; scripts/alias_uses.py, artifacts/alias-uses.json). Rule 14b "
            "binds a pointer variable given &PATH or PATH +/- n under never-bound roots exactly as a "
            "pointer-valued path read binds it: target-free, unknown, possibly null (C11 6.5.3.2p3: "
            "&*E may be null, so &p[0] is null when p is). Its later dereferences are then "
            "`unresolved resource identity` refusals (the chain flow refuses a possibly-null "
            "root). In 7 Unknown functions these are the only leaves left (append, peek, sink_peek "
            "in snappy.c; rd_kafka_share_ack_type_from_msg_op; "
            "rd_kafka_topic_partition_list_has_duplicates; rd_kafka_topic_partition_list_regex_cnt; "
            "mk_vhost_fdt_chain_lookup); 67 more carry other blockers. DISCHARGE: a stated "
            "premise or a derivation for when such an alias is non-null (a member or element of an "
            "object the path already dereferenced, `&B->F` / `&B->F[I]`, is non-null when B is), "
            "with Lean and controls, re-measured on these 7; or a decision that the premise is not "
            "taken." % (TAG, RUNS)),
        "source": "docs/reports/cg10-indirect-access-sole-residuals-r2/README.md",
        "recorded_by": "CG10-INDIRECT-ACCESS-SOLE-RESIDUALS",
    },
]


def main() -> int:
    data = json.loads(PATH.read_text())
    items = data["named_followups"]
    by_id = {item["id"]: item for item in items}
    for name, text in AMEND.items():
        item = by_id[name]
        if text not in item["why"]:
            item["why"] = item["why"].rstrip() + " " + text
    for entry in NEW:
        if entry["id"] not in by_id:
            items.append(entry)
    data["dimensions"]["named_followups"] = len(items)
    PATH.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    print("named_followups", len(items))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
