#!/usr/bin/env python3
"""update_ledger.py -- RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: the acceptance
matrix's named follow-ups (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent.

* Retired: `RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION` (its DISCHARGE holds: each
  opening store classified by reading; the union's places separated by
  Clang's layout; rko_op_cb's run set closed over its 27 candidates;
  measured).
* Registered: the discriminant route for rko_u, opaque record writes keyed
  by type, anonymous-record pointer roots, and rd_list_s.rl_free_cb's
  remaining opening (casts of designators, a same-record element shift)."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION"
SOURCE = "docs/reports/rko-op-cb-union-closure/README.md"
RUN = ("corpus 66910c10 on main e8158562d: 620fbd75 936/9/3,889 -> e93a68bd 936/9/3,889, no "
       "status move, 83 Unknown rows trade leaves (rd_kafka_op_destroy's 131 callback-member-open "
       "leaves become callback-uncontracted over the run's closed set of 27), binary a1 bb13eda5 "
       "against before 63548490; system 93/4/351 and slice 52/1/347/4 Pending byte-identical; "
       "null rows identical on all three")

RETIRE = [CARD]

REGISTER = [
    {
        "id": "RKO-U-ARMS-BY-DISCRIMINANT",
        "title": "rd_kafka_op_s.rko_u's arms are kept apart by Clang's x86-64 record layout, not by the op type that selects them",
        "why": (
            f"Registered by {CARD} ({RUN}). The card separated the union's places by "
            "Clang's layout (the brief's second evidence): rko_u.*@leaders.cb lies at union "
            "bytes 168..176, which no other function-pointer position (offset_commit.cb, "
            "broker_monitor.cb, terminated.cb at 8; admin_result.result_cb 56; the timers' "
            "rtmr_callback 56, 112, 704; the lists' rl_free_cb 24, 88, 744, 872) and no "
            "placement of an opened record meets. That is specific to the target the corpus is "
            "built for and rests on every unit agreeing on the layout. The preferred evidence, "
            "the discriminant rko_type (rule 12o), was sized and not taken: 204 sites in 22 "
            "units can put a function-pointer value into rko_u's bytes (a function-pointer "
            "member store, an address of an rd_list_t or a timer in an arm, a whole-arm "
            "write); 89 carry constant local evidence of the op's type (an enclosing switch on "
            "rko_type 4, a constructor with a constant type 51, an earlier assertion in the "
            "function 34), 80 take the type from a variable (rd_kafka_admin_request_op_new(rk, "
            "optype, ...)) and 35 have none; the leaders read itself rests on "
            "RD_KAFKA_OP_TYPE_ASSERT. It needs a stated arm-to-type map (a new surface word, "
            "the user chooses syntax) and an interprocedural reading of the op's type. "
            "DISCHARGE: a stated arm map, checked like rule 12o's stability over every write "
            "that can put a function-pointer value into an arm, separating rko_u's places "
            "independently of the target's layout (or recorded as refused), with Lean parity "
            "and a measured re-pin."
        ),
    },
    {
        "id": "UNION-OPAQUE-RECORD-WRITES-BY-TYPE",
        "title": "An opaque write of a record is keyed by the record's type, not its target, so it opens every union place a placement of that record meets",
        "why": (
            f"Registered by {CARD} ({RUN}). A whole-object assignment from a call or another "
            "type and a memcpy-family call open the written record by type "
            "(MemberFacts::opened); a laid-out union then opens each place where a placement "
            "of that record meets it (Tables::meets_record). Census: the whole-object "
            "rd_kafka_fetch_pos_s writes (rktp->rktp_committed_pos = rd_kafka_fetch_pos_make(...) "
            "at rdkafka_request.c:1313, rdkafka_cgrp.c:4322, rdkafka_partition.c:1382, and a "
            "local at rdkafka_partition.c:2233) target toppar fields and a local, never rko_u, "
            "yet they still open rko_u's places at union bytes 0..16 (offset_commit.cb, "
            "broker_monitor.cb and terminated.cb at 8). No invoked member reads one of those "
            "places today; leaders.cb (168..176) is clear. DISCHARGE: an opaque write keyed by "
            "its target place (a member path rooted at a record no union holds, a local, a "
            "global), the type-keyed reading kept for a write through an untargeted pointer, "
            "with Lean parity and a measured re-pin."
        ),
    },
    {
        "id": "ANONYMOUS-RECORD-POINTER-ROOT-KEYS",
        "title": "A member place whose root is a pointer to an anonymous record is keyed by the record's holder with its path dropped",
        "why": (
            f"Registered by {CARD} ({RUN}). Clang spells an anonymous record "
            "`struct T::(unnamed at ...)` in its desugared type, which the member keys read as "
            "the holder T. The card stops that below a named root (an anonymous union is keyed "
            "T.path.*, not T.*, and a place below an anonymous structure keeps its path, as the "
            "tables normalise a declared member), but a root that is itself a pointer to an "
            "anonymous record (rdkafka_mock.c: mcluster->handlers[i].cb, a "
            "`struct { ... } *handlers` member) is still keyed T.NAME "
            "(rd_kafka_mock_cluster_s.cb): it does not reach a member's normal key T.anon.NAME, "
            "and keying it ?.NAME instead would open every union member named cb. No invoked "
            "member is affected in the corpus. DISCHARGE: such a root keyed by the anonymous "
            "record's declared path (or read as an unread record restricted to its holder, "
            "read side included), with tests, Lean parity where the key rule changes, and a "
            "measured re-pin."
        ),
    },
    {
        "id": "RL-FREE-CB-CAST-AND-ELEMENT-SHIFT",
        "title": "rd_list_s.rl_free_cb reads open over librdkafka through casts of function designators and a unit test's element shift",
        "why": (
            f"Registered by {CARD} ({RUN}). Two of the four stores that opened rko_op_cb before "
            "the card still open rd_list_s.rl_free_cb (no invoked member reads it once rko_u is "
            "laid out): a C-style cast of a function designator handed to rd_list_init / "
            "rd_list_new as the free callback is an open flow (rdkafka.c:5576 and :5756 "
            "`(void *)rd_kafka_partition_leader_destroy`, rdkafka_interceptor.c:235, "
            "rdkafka_sasl_oauthbearer.c:898 and :1308 `(void (*)(void *))rd_strtup_destroy`, "
            "rdkafka_mock_sharegrp.c:579, rdkafka_plugin.c:151 `(void *)&rd_kafka_plugin_destroy`, "
            "rdkafka_fetcher.c:3455), and the memmove of rd_kafka_group_member_t elements in "
            "three sticky-assignor unit tests (rdkafka_sticky_assignor.c:2878, :3007, :3089) "
            "writes rkgm_eligible.rl_free_cb opaquely. DISCHARGE: a cast of a function "
            "designator read as that designator (the conversion keeps the target), and a "
            "memcpy-family copy between two pointers to one record by whole elements read as a "
            "copy of values already counted, with Lean parity and a measured re-pin."
        ),
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    known = {entry["id"] for entry in entries}
    for entry in REGISTER:
        if entry["id"] in known:
            continue
        entries.append({**entry, "source": SOURCE, "recorded_by": CARD})
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(before, "->", len(entries))


if __name__ == "__main__":
    main()
