#!/usr/bin/env python3
"""FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: the ledger update (idempotent).

Narrows FB-EVENT-LOOP-RETENTION-AND-REGISTRATION and
W47-LOOP-OWNER-UNDECLARABLE-OPERATIONS, retires RETAINING-CALL-MEMBER-ACTUAL
(its DISCHARGE holds), and registers FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME
(user decision 3: T1/T2 as a follow-up card) and MK-EVENT-DESCRIPTOR-CUSTODY.
Writes the matrix with indent=1, ensure_ascii=False and
dimensions.named_followups = the list length."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "../../../.."))
MATRIX = os.path.join(REPO, "docs/reviews/r0_3/acceptance_matrix.json")
SOURCE = "docs/reports/fb-event-loop-retention/README.md"
CARD = "FB-EVENT-LOOP-RETENTION-AND-REGISTRATION"
MEASURED = os.environ.get("FBEL_MEASURED", "(measured: see the card's report)")

NARROWED = {
    "FB-EVENT-LOOP-RETENTION-AND-REGISTRATION": (
        "Fluent Bit event-loop registrations not yet declared: the scheduler's timer registration "
        "and the collector operations by id",
        "Narrowed by FB-EVENT-LOOP-RETENTION-AND-REGISTRATION " + MEASURED + ". USER DECISIONS "
        "2026-10-16 (\"Tracked holder path\"; removal \"R3: quant state\"; forwarding \"Result + "
        "list\"; cross-function lifetime \"Follow-up card\") are implemented: holder paths and "
        "removals (rule 12t, C04.14.3), the registration forwarded through an id with its body "
        "check (rule 12n, C04.19.3), Lean QCore.RetentionPath and QCore.RegistrationById; Monkey's "
        "seven event-loop operations declared in qcore-provider-libevent (monkey/event_loop.qv) and "
        "the three collector setters in quirin-provider-fluent-bit "
        "(fluent_bit/input_collector_registration.qv), pinned at 66910c10, 046accf6 and the 90978ac6 "
        "export; flb_input_set_collector_server does not exist. Left here: (E) "
        "flb_sched_timer_cb_create / flb_sched_timer_coro_cb_create (20 calls in 15 files): the call "
        "retains `data` and registers `cb` in a timer it links into sched->timers and returns "
        "through `out_timer` (an id-free out slot), with the event registered at sched.evl inside "
        "it; no decided form states a record returned through an out slot. (F) "
        "flb_input_collector_start / _pause / _resume / _delete / _running by id (the slice's "
        "flb_input_collectors_signal_start, flb_input_thread_collectors_start, "
        "flb_input_collector_delete): a call taking the id must resolve the record the "
        "registration's `#id` names (decision 2's \"a later call taking that id resolves the same "
        "record\"), which no consumer reads yet; collector_start's own registration of "
        "&coll->event at coll.evl is a path holder and refuses at its exit "
        "(FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME). DISCHARGE: the timer registration stated and "
        "declared (a decided form for a record returned through an out slot and linked into a "
        "parameter's list), and the by-id operations resolving their id to the registration's "
        "record (each body read at both revisions, pinned), with Lean parity, measured on the "
        "system and slice routes with every moved row read.",
    ),
    "W47-LOOP-OWNER-UNDECLARABLE-OPERATIONS": (
        "Loop-owner obligation for collector registration and the Monkey event operations",
        "Narrowed by FB-EVENT-LOOP-RETENTION-AND-REGISTRATION " + MEASURED + ". The eight "
        "operations' custody is now declared without the loop-owner clause: "
        "flb_input_set_collector_time/event/socket (flb_input_set_collector_server does not exist "
        "at 66910c10, 046accf6 or 90978ac6) as registrations forwarded through an id "
        "(fluent_bit/input_collector_registration.qv, rule 12n), and mk_event_add, "
        "mk_event_timeout_create (retentions), mk_event_del, mk_event_timeout_destroy (removals) in "
        "qcore-provider-libevent (monkey/event_loop.qv, rule 12t), each pinned in its owner's crate "
        "at both revisions. Left: their loop-owner obligation (C1: registration and the Monkey "
        "operations touch loop-owned state the epoll thread safety does not cover), which no "
        "declaration states and the thread-entry census does not check. DISCHARGE: "
        "`requires resource_loop_owner(...)` on each of the seven, pinned, checked by the census "
        "route (W47-LOOP-OWNER-AUDIT-ROUTE).",
    ),
}

REGISTERED = [
    {
        "id": "FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME",
        "title": "A loop registration whose removal, release or loop destroy is in another function is never decided",
        "why": "Registered by FB-EVENT-LOOP-RETENTION-AND-REGISTRATION " + MEASURED + " on USER "
        "DECISION 2026-10-16 3 (\"Follow-up card\"). Rule 12t never forwards a retention into a "
        "summary, so a registration at a path holder (collector_start's &coll->event at coll.evl, "
        "flb_sched_timer_cb_create's &timer->event at sched.evl, net_io_*_async's "
        "&connection->event, flb_engine_start's config events) refuses at the registering "
        "function's exit as retention-undecided: the census (PREDICTION.md section 1) finds every "
        "one of the 37 mk_event_add / mk_event_timeout_create sites registering for a later "
        "function, and every one of the 37 removals in another function. The read ordering of a "
        "threaded input (flb_input_exit_all joins the input thread, whose exit destroys thi->evl, "
        "before flb_input_instance_destroy frees the collectors; flb_input_collector_destroy "
        "removes from coll->instance->config->evl, not coll->evl) spans four functions and a "
        "thread join. Two designs: T1, a run-wide closure per loop member like rule 12n's (the "
        "stores into evl members, every registration, removal, release of the held record type "
        "and loop destroy in the run, with an ordering the run must establish); T2, retentions "
        "forwarded into summaries along call edges (a change to rule 12t's \"do not forward\"). "
        "DISCHARGE: one of T1 or T2 decided by the user and implemented with Lean parity (a "
        "forwarded or run-wide record ends only by its own removal or its holder's release, and an "
        "unresolved one never decides), measured on the system and slice routes with every moved "
        "row read.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "MK-EVENT-DESCRIPTOR-CUSTODY",
        "title": "The descriptors Monkey's timeout and channel operations create and close are a named premise",
        "why": "Registered by FB-EVENT-LOOP-RETENTION-AND-REGISTRATION " + MEASURED + ". "
        "mk_event_timeout_create creates a timerfd (or a pipe and a detached worker) stored in the "
        "event and returned; mk_event_channel_create a pipe written through r_fd / w_fd; "
        "mk_event_timeout_destroy closes event->fd; mk_event_channel_destroy closes r_fd and w_fd "
        "only when event->fd == r_fd. v1 has no descriptor out slot and no identity for a "
        "member-held descriptor (C04.6), so the declarations state no descriptor effect and carry "
        "`requires resource_premise(mk_event_descriptors, ...)`; every verdict resting on them "
        "names it. DISCHARGE: a descriptor out slot or a member-held descriptor identity in v1, "
        "the four declarations restated with it (bodies read at both revisions, pinned), measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]

RETIRED = ["RETAINING-CALL-MEMBER-ACTUAL"]


def main():
    with open(MATRIX, encoding="utf-8") as f:
        matrix = json.load(f)
    entries = matrix["named_followups"]
    before = len(entries)
    for entry in entries:
        if entry["id"] in NARROWED:
            title, why = NARROWED[entry["id"]]
            if CARD not in entry["why"] or "Narrowed by " + CARD not in entry["why"]:
                entry["title"] = title
                entry["why"] = entry["why"].rstrip() + " AMENDED (" + CARD + "): " + why
    entries[:] = [e for e in entries if e["id"] not in RETIRED]
    ids = {e["id"] for e in entries}
    for new in REGISTERED:
        if new["id"] not in ids:
            entries.append(new)
    matrix["dimensions"]["named_followups"] = len(entries)
    with open(MATRIX, "w", encoding="utf-8") as f:
        json.dump(matrix, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
