#!/usr/bin/env python3
"""NULL-MEMBER-INVARIANT-SCOPE: retire NULL-MEMBER-INVARIANT-SCOPE, amend
NULL-CUSTODY-PREMISE-RESIDUALS, register the residuals with sizes.
indent=1, ensure_ascii=False; `dimensions.named_followups` = list length."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "NULL-MEMBER-INVARIANT-SCOPE"
SOURCE = "docs/reports/null-member-invariant-scope/README.md"
MEASURED = (
    "Measured by NULL-MEMBER-INVARIANT-SCOPE (gateway rules 14x-14z; corpus 66910c10, run a2, binary "
    "70d36c4c; custody byte-identical 915 / 9 / 3,910 ded14632, parity multiset-identical; null 1,113 / "
    "1,433 Safe, 2,353 Unknown, 22 provisional Unsafe with 33 findings; candidates 12,648 = 10,777 on the "
    "functions' own checks + 1,871 on travelled caller checks)"
)

NEW = [
    {
        "id": "NULL-MEMBER-EXIT-STORE-OBJECT-IDENTITY",
        "title": "Members set back to NULL on an existing object (a reset, an unlink) have no checked scoped reading",
        "why": MEASURED + ": rd_kafka_buf_s.rkbuf_rkb (3,474 own + 41 travelled candidates; rd_kafka_recv "
        "stores NULL into the broker's receive buffer after reading the response header, rdkafka_broker.c:2134, "
        "and rd_kafka_req_response sets it again before the callbacks run), mk_list.next and cfl_list.next "
        "(602 own + 2 travelled `->next` candidates; mk_list_del and cfl_list_del store NULL into the unlinked "
        "node). Rule 14x's stable member needs every store to an existing object proved; a function scope "
        "(rule 14z) keeps its entry fact across a call only with a stable member, since a reset can run during "
        "the call or on another thread. Most rkbuf_rkb candidates are the parse-failure macros' reads after "
        "`if (rkbuf->rkbuf_rkb)` and an intervening call (rd_snprintf, strlen, mtx_lock), and the response "
        "handlers are reached only through rkbuf_cb (an open indirect call). DISCHARGE: an object-identity "
        "reading that shows an exit store never targets an object a scoped use holds (e.g. the receive buffer "
        "is owned by the broker thread until rd_kafka_req_response hands it over; a node is linked from a live "
        "head), checked, with Lean; never a premise that the reset does not reach.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "NULL-MEMBER-SCOPE-ENTRY-INFERENCE",
        "title": "Function-scope entries are declared for 66 broker functions; the rest of rkb->rkb_rk's first reads stay candidates",
        "why": MEASURED + ": with rd_kafka_broker_s.rkb_rk stable (rule 14x) the own rkb_rk-sourced "
        "candidates fell 682 -> 488 (run a1); the 66 declared `null_member_scope(rd_kafka_broker_s, rkb_rk, rkb)` "
        "entries (rule 14z) made 26 rows Safe conditional and moved the question to 312 caller checks "
        "(38 proved, 95 moved, 179 candidates: 16 Safe rows became Unknown, each named in the report) -> 552 own "
        "rkb_rk-sourced candidates. 117 functions with such a first read were not declared: 45 without a "
        "definition the script reads (header inline functions, macros), 38 whose broker is not the first "
        "parameter (a local from rktrans->rktrans_rkb, rkbuf->rkbuf_rkb or a thread argument), 28 with a "
        "parameter bounds v1 cannot state (a struct by value such as rd_kafka_replyq_t), 6 whose address the "
        "library takes. An actual `&local` (rd_ut_features_check, 3 checks) is read as a memory field. "
        "DISCHARGE: an in-unit inference of the entry (rule 14p's analogue for `P->FIELD` of a stable member: "
        "every path dereferences it before any test or call that could store it) with cross-unit travel, and "
        "a member path for an address actual; each measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "NULL-MEMBER-STABLE-CENSUS",
        "title": "Other memory-field members may be stable; none but rkb_rk is declared",
        "why": MEASURED + ": a stable member keeps a tested or dereferenced fact on `V->FIELD` across calls "
        "and stores. The memory-field candidates by member name on the base (own and travelled): elems 169, "
        "partitions 148, map 143, cluster 123, topics 120, rk_cgrp 114, tqh_last 109, rkcg_group_id 101, "
        "rkgm_assignment 67, msetr_rkb 63, broker 60, tqe_prev 59, rk 55. Each needs its struct tag, every "
        "store read at both revisions, a host, and the run's store check (an exit store withdraws it). "
        "DISCHARGE: each candidate member declared and measured, or recorded with the store that rules it out.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "NULL-MEMBER-WHOLE-OBJECT-WRITES",
        "title": "Whole-object writes are checked for stable members only, and only where the walk can attribute them",
        "why": MEASURED + ": rule 14x records a struct assignment of a `struct TAG` lvalue and a memset/memcpy/"
        "memmove/bzero whose destination, casts stripped, is typed `struct TAG *` as a stable member's store "
        "never proved (none for rkb_rk); a write through an untyped pointer or into an enclosing object stays "
        "the premise `whole-object`. Rule 14t's declared members (five, partly-validated) record neither: a "
        "struct copy or fill of an existing object is covered only by their `construction` premise. DISCHARGE: "
        "rule 14t's store check records the attributable whole-object writes too (re-measured), and the "
        "untyped ones are sized.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]

AMEND = {
    "NULL-CUSTODY-PREMISE-RESIDUALS": (
        " Re-measured by NULL-MEMBER-INVARIANT-SCOPE (corpus a2): custody byte-identical; premise checks the "
        "null property proves 782 -> 786 (candidates 892 -> 888); custody Safe rows with an own premise 12, "
        "every premise proved 1 (unchanged); system 2 / 0 and slice 5 / 1 unchanged."
    ),
}

RETIRE = CARD


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    entries[:] = [e for e in entries if e["id"] != RETIRE]
    for entry in entries:
        if entry["id"] in AMEND and AMEND[entry["id"]] not in entry["why"]:
            entry["why"] += AMEND[entry["id"]]
    known = {e["id"] for e in entries}
    for entry in NEW:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))


main()
