#!/usr/bin/env python3
"""update_ledger.py -- FB-QUANT-CONTRACT-CENSUS: register the three
recommended implementation cards in the acceptance matrix's named follow-ups
(indent=1, ensure_ascii=False; `dimensions.named_followups` = list length).
Idempotent: an id already present is left as it is."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "FB-QUANT-CONTRACT-CENSUS"
SOURCE = "docs/reports/fb-quant-contract-census/README.md"
RUN = ("binary 7bf6e937 built from main d708641bc; corpus 66910c10 71387bd3 936/9/3,889, "
       "system d25e937f 93/4/351 (null 84/125/245), slice 0899ba80 52/1/347/4 Pending "
       "(null 71/103/232); one BEFORE run per route, no implementation")

REGISTER = [
    {
        "id": "FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE",
        "title": "Fluent Bit plugin descriptors (instance->p) are never stated non-null: the null property leaves every ins->p member access a candidate",
        "why": (
            f"Registered by {CARD} ({RUN}), ranked first. On the tail-pause slice 34 null candidates "
            "are `ins->p` / `p` member accesses and they are every candidate of 7 functions "
            "(flb_input_plugin_pause, flb_input_plugin_resume, flb_input_pause, flb_input_resume, "
            "flb_input_instance_pre_run, flb_input_instance_exit, flb_input_thread_instance_pre_run; "
            "obligations C2, C4, C5 of w47-tail-pause-bindings); on the system route 3 output "
            "functions (flb_output_pre_run, flb_output_exit, flb_output_init_all) carry 12 such "
            "candidates beside others. Every store of the member is `instance->p = plugin` after the "
            "constructor dereferenced `plugin` (flb_input.c:490, flb_output.c:857, flb_filter.c:507, "
            "flb_processor.c:2188, flb_custom.c:214). The existing forms fit with no new syntax: "
            "`requires nonnull_member_stable(flb_input_instance, p);` (rule 14x) on the constructor "
            "and `requires nonnull_member_scope(flb_input_instance, p, ins);` (rule 14z) on the "
            "functions that read it, likewise flb_output_instance. Size: 2 stable and about 10 scope "
            "declarations in quirin-provider-fluent-bit's bounds contracts. Risks to measure: the "
            "instance is flb_calloc'ed before the store (whether rule 14x treats the fresh object's "
            "NULL as an examined store), and rule 14z's call-precondition check at every direct "
            "call can add candidates in callers. DISCHARGE: the declarations pinned and checked "
            "(stable stores proved, no withdrawal), a frozen prediction, and the system, slice and "
            "corpus null rows measured before/after with every moved row read, no custody row moved."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "FB-EVENT-LOOP-RETENTION-AND-REGISTRATION",
        "title": "Fluent Bit event-loop registrations (mk_event_*, collectors, scheduler timers) retain an event or a callback no declaration states",
        "why": (
            f"Registered by {CARD} ({RUN}), ranked second; it measures and narrows "
            "W47-LOOP-OWNER-UNDECLARABLE-OPERATIONS (8 operations) and depends on "
            "MEMBER-PREPASS-OTHER-ROUTES for the callback closure. Its leaves (uncontracted "
            "mk_event_add/_del/_timeout_create/_timeout_destroy/_channel_create/_channel_destroy/_wait, "
            "flb_input_set_collector_*, flb_sched_timer_cb_create, flb_input_collector_start, "
            "flb_input_pause/_resume/_pause_all: system 43, slice 43; indirect accesses at member "
            "actuals such as mk_event_channel_create(config->evl, ...): 18, 20) stand in 16 system and "
            "22 slice functions, alone in 1 (flb_sched_timer_cb_disable) and 4 (flb_input_pause_all, "
            "flb_input_rate_gate_timer_schedule, flb_input_test_pause_resume, "
            "flb_input_thread_collectors_start), and in callees of 8 and 8 more; obligations OB-10, "
            "OR-9, C1, C2, C4, C5. Tree: 102 collector registrations in 57 files, 37 "
            "mk_event_add/timeout_create sites, 22 timer-callback registrations in 15 files. Shapes: "
            "`resource_retained_by(data, loop)` (C04.14) for the Monkey operations (owner crate "
            "qcore-provider-libevent) and `resource_callback_member_registered(result, "
            "flb_input_collector.cb_collect, cb)` (C04.19) on collector_create. Missing: a holder "
            "whose lifetime is the loop's (every loop is a member reached through a formal, "
            "config->evl / coll->evl / thi->evl / sched->evl, which rule 12t reads as "
            "retention-undecided; the event is an interior &coll->event) -- a user decision on a "
            "word or a premise; registration forwarding through the id-returning public setters; "
            "the member pre-pass on the current route. DISCHARGE: the user's decision on the loop "
            "holder, the operations declared in their owners' crates and pinned (bodies read at "
            "both revisions), registration forwarding with Lean parity, measured on the system and "
            "slice routes with every moved row read."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "FB-TEARDOWN-DESTROY-DECLARATIONS",
        "title": "Fluent Bit teardown callees in the system and slice routes are uncontracted, and rule 30's member-path states are read only at callback invocations",
        "why": (
            f"Registered by {CARD} ({RUN}), ranked third. Uncontracted teardown calls whose bodies "
            "are in the route but another unit: system flb_output_instance_destroy 16 leaves, "
            "flb_sched_timer_destroy 9, flb_task_destroy 9, flb_sched_request_destroy 5, "
            "flb_task_retry_clean 4, flb_task_queue_destroy 4 and eight more; slice "
            "flb_tail_config_destroy 39, flb_tail_file_remove 17, flb_input_instance_destroy 9 and "
            "six more. They stand in 36 system and 24 slice functions (alone in "
            "flb_engine_destroy_tasks, flb_sched_request_invalidate, flb_sched_timer_cb_destroy; "
            "flb_tail_file_remove_all), in callees of 11 and 6 more, and in 4 corpus "
            "flb_input_chunk.c functions; obligations OB-10, OR-2 to OR-6, OR-8 to OR-11. A "
            "destroyer releases its object (`resource_released`) and leaves member states "
            "(`quant live #task: flb_task.ic |-> #released;`, rule 30), but rule 30 refuses a direct "
            "call (`member-path-direct`) and flb_task_destroy's chunk deletion depends on the scalar "
            "`del`, for which a guard beside `quant live` is new surface (C04.14.1 guards "
            "resource_retained_by only). 30 system leaves are unresolved identities of the argument "
            "(flb_task_retry_destroy(retry), flb_input_chunk_destroy(ic, ...)) that no declaration "
            "decides. Related: W45-FB-USERS-RELEASE-CHECK (flb_task_users_release). DISCHARGE: the "
            "user's decision on a parameter guard (or its rejection), rule 30 read at a direct call "
            "of a body-checked destroyer with Lean parity, the destroyers declared as checked "
            "specifications, measured on the system, slice and corpus routes with every moved row read."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    m = json.loads(MATRIX.read_text(encoding="utf-8"))
    nf = m["named_followups"]
    before = len(nf)
    have = {e["id"] for e in nf}
    added = []
    for e in REGISTER:
        if e["id"] not in have:
            nf.append(e)
            added.append(e["id"])
    m["dimensions"]["named_followups"] = len(nf)
    MATRIX.write_text(json.dumps(m, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{before} -> {len(nf)}; registered {added}")


if __name__ == "__main__":
    main()
