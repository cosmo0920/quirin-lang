#!/usr/bin/env python3
"""NULL-SAFETY-PROPERTY: retire CG10-INTERIOR-ALIAS-NULLABILITY (USER
DECISION 2026-10-10, decision 1, recorded verbatim in the residual entries
below and in contract specification C09.3), amend the follow-ups the card
re-measured, and register the residuals with sizes. Acceptance matrix,
indent=1, ensure_ascii=False; `dimensions.named_followups` = list length.
Idempotent. Sizes are read from the card's artifacts (the a4 runs)."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "NULL-SAFETY-PROPERTY"
SOURCE = "docs/reports/null-safety-property/README.md"
ARTIFACTS = REPO / "docs/reports/null-safety-property/artifacts"

summary = json.loads((ARTIFACTS / "null-summary-corpus.json").read_text())
by_kind = json.loads((ARTIFACTS / "candidates-by-kind.json").read_text())
sizes = json.loads((ARTIFACTS / "residual-sizes.json").read_text())

DECISION = (
    "USER DECISION 2026-10-10 (verbatim): 1. Move the NULL question (resolves "
    "CG10-INTERIOR-ALIAS-NULLABILITY). &B->F, or any use of B as non-NULL, is read as non-NULL "
    "only when every path to it already went through a real read or write via B, with B not "
    "reassigned in between. Forming &B->G and sizeof/unevaluated operands do not count as "
    "dereferences. The NULL question then moves to that dominating dereference, and that "
    "dereference becomes a published check in a new null-safety property. Proved: from a NULL "
    "test, a non-NULL contract result or parameter, a checked allocation, or a further dominating "
    "dereference up the chain. Otherwise: a NULL-dereference candidate, validated for "
    "reachability before any Unsafe and never presented as an upstream defect. A custody verdict "
    "relying on the non-NULL reading lists a named assumption tied to the dereference site until "
    "the null-safety check proves it. It appears in the per-verdict dependency set and roll-up. "
    "User motivation: NULL dereference is one of the most common bug sources, so the value is in "
    "surfacing candidates, not only in freeing functions. 2. Contract syntax (the user called it "
    "\"quite ideal\"): quant null-deref #B; Collocated check form, like quant checks. It means "
    "\"this function dereferences B; B must be non-NULL.\" Caller side: the actual for B must be "
    "proved non-NULL, otherwise it is a candidate at the call site. Callee side: the body check "
    "validates that the body dereferences B without a prior NULL test. Inside the body, after the "
    "first dereference, B and &B->F are non-NULL (decision 1). A body that tests B first does not "
    "need the clause. Report it as stronger than necessary, never as wrong."
)

RETIRE = ["CG10-INTERIOR-ALIAS-NULLABILITY"]

checks = summary["checks"]
REGISTER = [
    {
        "id": "NULL-CUSTODY-PREMISE-RESIDUALS",
        "title": "Decision 1's custody reading frees 4 of the 7 CG10-INTERIOR-ALIAS-NULLABILITY functions; the other 3 are blocked by something else, and custody's older reading of unbound names lists no premise",
        "why": (
            f"{DECISION} Implemented by {CARD} (gateway rule 14n; corpus 66910c10 903/9/3,922 "
            f"24b59d52 -> {sizes['corpus_after']}; system {sizes['system_after']}; slice "
            f"{sizes['slice_after']}): a scalar read or write through a target-free, unknown, "
            "possibly-null borrowed alias of the identity derivation is read non-null under the "
            "named premise null-deref:FUNCTION:LINE:VARIABLE, listed on the summary, the row, the "
            "row's contract_dependencies (transitively) and the Safe roll-up. Freed with premises: "
            "peek, sink_peek, rd_kafka_share_ack_type_from_msg_op, "
            "rd_kafka_topic_partition_list_regex_cnt (and 8 more corpus, 2 system and 5 slice "
            "rows). Left, each measured: append (after iov++ the binding is a stepped derivation; "
            "decision 1 ends the moved question at a reassignment, so custody reads nothing); "
            "mk_vhost_fdt_chain_lookup (its remaining leaf is the return of the unknown alias hc, "
            "an identity question apply_resource_action refuses for any unknown binding, not a "
            "nullness one); rd_kafka_topic_partition_list_has_duplicates (on the base it also "
            "carries an uncontracted rd_kafka_topic_partition_list_sort_by_topic and indirect "
            "accesses: a correction to the follow-up's population). Custody's pre-existing reading "
            "of a dereference through an unbound name (a formal, W43-INDIRECT-ACCESS-GENERAL: "
            "nullness outside the property) lists no premise: "
            f"{sizes['custody_safe_rows_with_formal_candidates']} custody Safe rows have a "
            "formal-sourced null candidate. DISCHARGE: a decision whether those readings list a "
            "premise too (they would join every custody Safe row that dereferences a formal), and, "
            "for append, whether an offset of a moved alias keeps the premise in custody as it "
            "keeps the moved question in the null property (C11 6.5.6p8)."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "NULL-CANDIDATE-TRIAGE",
        "title": f"{summary['functions'] - 87 - sizes['null_safe']} functions carry {checks['by_outcome'].get('candidate', 0)} NULL-dereference candidates; most read a pointer from memory that an invariant keeps non-null",
        "why": (
            f"Measured by {CARD} (gateway rules 14k-14p, corpus 66910c10, run a4; "
            "artifacts/candidates.json.gz, candidates-by-kind.json): candidates by strongest source "
            f"kind {json.dumps(by_kind['by_strongest_source_kind'])}; by library "
            f"{json.dumps(by_kind['by_library'])}. The ranked top 30 (all allocation results) read "
            f"by hand: {sizes['top30_real']} real-looking (unchecked mk_mem_alloc / calloc / malloc "
            "results, NULL only on allocation failure), "
            f"{sizes['top30_guarded']} guarded by something the walk does not read, "
            f"{sizes['top30_infeasible']} infeasible. A stratified sample of the bulk "
            "(memory-field, formal, uncontracted-result, null-literal) is mostly guarded by "
            "invariants the walk does not read: a buffer's broker (rd_kafka_buf_read_* reads "
            "rkbuf->rkbuf_rkb), circular list heads, a list's preallocated elements, a callee "
            "that creates what it returns, counters correlated with a pointer. None is presented "
            "as an upstream defect. DISCHARGE: a vocabulary for non-null fields and results "
            "(e.g. a declared non-null member or a non-null borrowed result, checked against the "
            "bodies that store it), each measured; the real-looking allocation sites reported "
            "upstream only after reachability is validated."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "NULL-CLAUSE-COVERAGE",
        "title": "Inferred null-deref clauses do not leave their unit; 13 are declared, the rest of the cross-unit callers see no obligation, and calls through function pointers are not checked",
        "why": (
            f"Measured by {CARD} (gateway rules 14o-14p): {summary['inference']['inferred_preconditions']} "
            f"inferred preconditions on {summary['inference']['functions_with_an_inferred_precondition']} "
            "corpus functions; 13 external-linkage ones are declared (bounds contract v1, pinned at "
            "66910c10 and 046accf6: rd_kafka_destroy, rd_kafka_topic_partition_list_destroy, _add, "
            "rd_list_destroy, rd_list_add, rd_kafka_queue_destroy, rd_kafka_message_destroy, "
            "cmt_destroy, flb_output_set_context, flb_output_net_default, flb_output_name, "
            "flb_output_set_property, flb_input_chunk_get_name; every body that defines one in "
            "the corpus checks `necessary`). Ranked but not declarable in bounds v1: the cmetrics "
            "setters (cmt_gauge_set 64 calling files, cmt_counter_set 36, _inc 19, _add 18: a "
            "`double` or a `char **` parameter, which G05b admits only with a closed frame), "
            "mk_event_add (its MK_HAVE_TRACE configuration tests `data` first), ctr_destroy (tests "
            f"ctx first at 046accf6). {checks.get('indirect_calls_not_inventoried', 0)} calls through a "
            f"function pointer in {checks.get('functions_with_an_indirect_call', 0)} functions are not "
            "checked (the pointer's own null-ness). DISCHARGE: the next declarations by "
            "scripts/clause_rank.py (each body read at both revisions, pinned), a bounds v1 "
            "admission of scalar floating and slot parameters in a declaration that carries only "
            "null-deref checks, and a check of the function pointer at an indirect call, each "
            "measured."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    entries[:] = [e for e in entries if e["id"] not in RETIRE]
    present = {e["id"] for e in entries}
    for entry in REGISTER:
        if entry["id"] in present:
            for existing in entries:
                if existing["id"] == entry["id"]:
                    existing.update(entry)
        else:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))


if __name__ == "__main__":
    main()
