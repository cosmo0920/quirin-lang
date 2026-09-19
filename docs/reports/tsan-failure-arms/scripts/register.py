#!/usr/bin/env python3
"""TSAN-FAILURE-ARMS (wave 41): the tracker edits, in one place.

`named_followups` is hand-maintained (the generator carries it verbatim), so this

  * RETIRES three entries whose discharge conditions are met word for word --
    each verified registered first, and printed in full with `--show` before
    anything moves;
  * AMENDS two by APPENDING to their `why` (never rewriting it): the standing
    invisibility rule, with what now enforces it, and the executed-schedules
    boundary, whose last concrete half is met while the residual itself stays
    registered as a permanently stated measured boundary;
  * REGISTERS four with `recorded_by` set to this card;

and rewrites in the generator's byte format (indent=1, ensure_ascii=False,
trailing newline). `dimensions.named_followups` is then re-derived by

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/tsan-failure-arms/gate.json

Idempotent: a retirement of an absent id is reported and skipped, a duplicate
registration is skipped, and an amendment already present is not appended twice.
"""
import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/tsan-failure-arms/README.md"
CARD = "TSAN-FAILURE-ARMS"

RETIRE = [
    "TSAN-FORCED-GATE-OPENED-BY-THE-JOIN",
    "TSAN-FORCED-SCHEDULE-FAILURE-ARMS",
    "TSAN-HEADER-INVISIBILITY-UNENFORCED",
]

AMEND = {
    "TSAN-FORCED-HARNESS-SYNC-INVISIBILITY": (
        " ENFORCED since card TSAN-FAILURE-ARMS (wave 41), which discharged "
        "TSAN-HEADER-INVISIBILITY-UNENFORCED: two committed, non-ignored workspace tests in "
        "compiler/tests/task_route_tsan.rs. the_forcing_header_stays_invisible_to_thread_sanitizer "
        "reads SCHEDULE_FORCING's text - the QCORE_FORCED_UNINSTRUMENTED macro must still expand to "
        "no_sanitize(\"thread\"), the header must name no pthread/C11 synchronization primitive, and "
        "every static qcore_forced_ function must carry the attribute. "
        "a_visible_forcing_header_would_hide_the_overlap_which_is_why_the_shipped_one_is_not builds "
        "the hook_visible_to_tsan mutation as thread_sanitizer::with_visible_synchronization - a "
        "pthread mutex for the bookkeeping lock, instrumentation restored, and a parked worker that "
        "learns of its release by acquiring that mutex - and asserts that the overlap mutation goes "
        "UNREPORTED under ForcedReverse while the shipped header reports it. Measured 0 of 20 visible "
        "against 20 of 20 shipped under BOTH forced schedules, so the false negative is broader than "
        "wave 39 recorded (docs/reports/tsan-failure-arms/artifacts/catch-rates.json, 'invisibility'). "
        "Every substitution asserts its anchor appears exactly once, so a header edit out of the "
        "measured shape panics rather than passes. The rule itself stays registered: half two is one "
        "mutation, not a quantifier over all visible designs - "
        "TSAN-VISIBLE-HEADER-ONE-MUTATION-NOT-A-QUANTIFIER."
    ),
    "TASK-ROUTE-TSAN-EXECUTED-SCHEDULES": (
        " AMENDED by TSAN-FAILURE-ARMS (wave 41): the last concrete half is met. The failure arms this "
        "residual named first - a failed qcore_buffer_new, a failed spawn - are now executed under "
        "ThreadSanitizer on the write-lend flagship and on CT07's own case, under all three named "
        "schedules, clean, with the arm that ran asserted from the program's own stdout and the stdout "
        "compared byte for byte against the reference run scripted for the same outcomes (20/20 on "
        "every one of twelve cells); its carrier TSAN-FORCED-SCHEDULE-FAILURE-ARMS is retired. The "
        "residual is NOT retired: what remains is the permanent epistemic statement in its title, and "
        "its disposition is now the first branch of its own discharge - stated permanently as a "
        "measured boundary. Boundaries carried: TSAN-ALLOC-FAILURE-ARM-SINGLE-THREADED (the "
        "allocation-failure arm creates no thread on either subject) and "
        "TSAN-COHORT-TRIGGER-CATCH-COST."
    ),
}


def entry(id_, title, why):
    return {"id": id_, "title": title, "why": why, "source": REPORT, "recorded_by": CARD}


NEW = [
    entry(
        "TSAN-COHORT-TRIGGER-CATCH-COST",
        "The cohort release trigger costs catches on the join-edge-removal mutation, so it is an addition to the join trigger and not a replacement",
        "MEASURED by card TSAN-FAILURE-ARMS (wave 41), "
        "docs/reports/tsan-failure-arms/artifacts/catch-rates.json. The new QCORE_FORCED_COHORT "
        "trigger opens the forcing gate inside the create hook, before the creator reaches any join, "
        "which is what catches the gate-closed mutation TSAN-FORCED-GATE-OPENED-BY-THE-JOIN recorded "
        "(0 of 20 at cohort 0, 20 of 20 at cohort 2 under both forced schedules). It also COSTS "
        "catches elsewhere: at cohort 2 the join-edge-removal mutation is reported 0 of 20 on "
        "shared_readers AND on CT07's case under forced-together, and 18 of 20 on shared_readers "
        "under forced-reverse, against 20 of 20 at wave 39's join-only trigger. The workers are "
        "released at creation, run and finish long before the creator's post-join access, and what "
        "ThreadSanitizer reports becomes a thread leak rather than a data race. The card's response "
        "is to keep BOTH triggers - the two committed wave-39 mutation rows stay at cohort 0, the "
        "clean rows run at the row's own cohort, and both cohorts are measured in the published "
        "table - rather than to weaken either row. DISCHARGE: a release trigger that fires before the "
        "join AND leaves the creator's post-join access unordered relative to worker bodies "
        "ThreadSanitizer still holds shadow history for; or state permanently, keeping both triggers "
        "measured and the two wave-39 rows at cohort 0.",
    ),
    entry(
        "TSAN-ALLOC-FAILURE-ARM-SINGLE-THREADED",
        "The NULL-allocator failure arm creates no thread, so its clean ThreadSanitizer verdict is about a single-threaded recovery path",
        "Card TSAN-FAILURE-ARMS (wave 41) landed TSan rows over both failure arms "
        "TSAN-FORCED-SCHEDULE-FAILURE-ARMS named. The refusing-QCORE_TASK_CREATE arm IS thread-live: "
        "the first worker is created, released, runs its body and is joined inside the arm. The "
        "NULL-returning-QCORE_BUFFER_MALLOC arm is NOT: on both subjects (the write-lend flagship and "
        "CT07's case.qcore) the only allocation is the entry's own qcore_buffer_new(0)?, which stands "
        "BEFORE any spawn, so the error edge is taken with no thread in existence and TSan has no "
        "second thread to reason about. The row is real - the arm executes, under TSan, clean, with "
        "its stdout byte-identical to the reference run scripted for the same outcome, 20 of 20 under "
        "each of the three schedules - and it is weak, and this says so rather than letting the cell "
        "count read as thread evidence. WHAT WOULD REACH IT: an admitted program on this route that "
        "acquires a buffer AFTER a spawn; none is spelled today, and the nearest existing shape is "
        "CT16's worker_allocation.h fixture, which is not a TSan subject. DISCHARGE: a TSan row over "
        "an allocation failure occurring while a worker is live; or state permanently that this "
        "route's allocation-failure arm is single-threaded by construction.",
    ),
    entry(
        "TSAN-FAILURE-ARM-COHORT-SUPPLIED-BY-HAND",
        "The forcing harness's per-program cohort count is read off each program's text by the row's author, not derived from the checked artifact",
        "Card TSAN-FAILURE-ARMS (wave 41). QCORE_FORCED_COHORT is 'the number of workers this program "
        "parks in one cohort', and every row supplies it as a literal: SUBJECTS' third column in "
        "compiler/tests/task_route_tsan.rs, and the per-arm counts in failure_arms() (1 for the "
        "spawn-refusal arm, which creates one worker; 0 for the allocation-failure arm, which creates "
        "none). Nothing derives it from the checked task IR, so a subject whose source changes its "
        "spawn count silently gets a wrong cohort. The consequence is bounded but real: a too-large "
        "cohort never fires and the row falls back to the join trigger (wave 39's behaviour, still "
        "correct but no longer testing what the row says), and a too-small one releases early, which "
        "the schedule-shape assertion usually but not always catches - mutation "
        "rows_cohort_is_the_row_s shows it catching exactly that. DISCHARGE: derive the count from the "
        "checked task IR (the TrySpawn spans preceding the first Join, the ordering "
        "REGION-CREDIT-JOINS' Q600 condition already reads) and assert each row's literal against it; "
        "or state permanently, with the schedule-shape assertion named as the guard.",
    ),
    entry(
        "TSAN-JOIN-EDGE-ROW-LOAD-SENSITIVE",
        "removing_the_join_edge_is_reported_under_every_schedule misses its ct07-case forced-together cell under concurrent whole-workspace load, and passes it 20 of 20 in isolation",
        "MEASURED by card TSAN-FAILURE-ARMS (wave 41) at its rebased gate, and recorded in BOTH "
        "directions rather than absorbed. Inside `cargo test --workspace --no-fail-fast` with three "
        "sibling worktrees building concurrently, "
        "removing_the_join_edge_is_reported_under_every_schedule (compiler/tests/task_route_tsan.rs, "
        "wave 39's row, whose behaviour this card did not change - it runs at cohort 0, wave 39's "
        "own join trigger) failed at its ct07-case @ forced-together cell: ThreadSanitizer reported "
        "a THREAD LEAK instead of a data race, with the schedule trace showing both workers started "
        "and left (docs/reports/tsan-failure-arms/gate/red-row.log). ON THE SAME BINARY AND THE SAME "
        "TREE the whole file is 18 passed / 0 failed / 1 ignored in isolation "
        "(gate/row-isolation.exit 0), and the row alone ran TWENTY times at load average 5.78 with "
        "0 OF 20 RED (gate/row-isolation-20-runs.log). The card's own 20-run table had measured this "
        "cell at 20 of 20 on a quiet host (artifacts/catch-rates.json, 'join-edge-removed/ct07-case "
        "@ cohort 0 @ forced-together'), so this is a load observation and not a new defect, and the "
        "assertion is deliberately NOT weakened and the row is NOT ignored. The mechanism is the one "
        "TSAN-COHORT-TRIGGER-CATCH-COST names from the other side: what this mutation's detection "
        "depends on is that the creator's post-join access still has the workers' accesses in "
        "ThreadSanitizer's shadow history, and anything that lets the workers finish far earlier - a "
        "release trigger, or a loaded machine descheduling the creator - can cost the report. "
        "DISCHARGE: measure the row under controlled load to establish the rate (the P02-FLAKE "
        "method), then either add it to rule 16's enumerated load-sensitive rows - a decision above "
        "a card, since that rule lives in a preserved span - or make the row insensitive by "
        "asserting on ThreadSanitizer's report SET rather than on a data race specifically, which "
        "would weaken what it measures and needs its own argument.",
    ),
    entry(
        "STATUS-SOURCES-CHECKPOINT-VS-LIVE-TRACKER",
        "docs-structure-compression's status-sources.json is a frozen checkpoint that its own --check compares against live inputs, so the first legitimate tracker edit after it makes that check stale",
        "MEASURED by card TSAN-FAILURE-ARMS (wave 41), the first card to move a tracker count after the "
        "documentation restructure at 96142e49. docs/reports/docs-structure-compression/"
        "rederive_status.py --check re-derives the status snapshot from PRIMARY INPUTS (the CT "
        "manifest, the w37 corpus artifacts, contract/qcore, the wave-40 close's gate.json, "
        "proof/Audit*.lean and the acceptance matrix) and asserts it equals the archived "
        "status-sources.json. This card's tracker delta - three retirements and four registrations, "
        "named_followups 257 -> 258 - moves EXACTLY ONE field of that archive, followups {carried "
        "257, derived 257} -> {258, 258}, and nothing else: lean_audits stays 141, contracts.modules "
        "stays 88, and the ct, corpus and last_merged_gate blocks are untouched (measured, not "
        "assumed). The archive is NOT edited, because it is a frozen checkpoint (constraint 21), and "
        "the script is NOT run by the standing gate - nothing in scripts/tests/, no Rust test and no "
        "CI step references it, so no gate row is red and this is a latent staleness rather than a "
        "failure. Its sibling verify.py is unaffected: it reconstructs the pre-restructure guides "
        "from git show <source_commit>:<source>, so it is checkpoint-scoped by construction and "
        "stays green. This is the D02-B00-LEDGER-SNAPSHOT shape - a derived figure reading a frozen "
        "archive beside a live source - and it is a decision above a card. DISCHARGE: publish "
        "provenance beside status-sources.json (the fields a later wave is expected to move, and "
        "where the live figure is read instead), or make the checkpoint re-derivable on demand with "
        "the archived copy labelled as the migration's own observation; never by editing the "
        "archive to match a later tree.",
    ),
    entry(
        "TSAN-VISIBLE-HEADER-ONE-MUTATION-NOT-A-QUANTIFIER",
        "The enforced invisibility property is a forbidden-primitive list plus one measured mutation, not a statement about every visible forcing design",
        "Card TSAN-FAILURE-ARMS (wave 41) discharged TSAN-HEADER-INVISIBILITY-UNENFORCED with two "
        "committed tests, and states here what they do not cover. Half one "
        "(the_forcing_header_stays_invisible_to_thread_sanitizer) is a LIST: the shipped header must "
        "carry no_sanitize(\"thread\") on every static qcore_forced_ function and must not name any of "
        "pthread_mutex, pthread_cond, pthread_barrier, pthread_rwlock, sem_wait, mtx_lock, mtx_init, "
        "cnd_wait, cnd_signal, cnd_broadcast. A future design synchronising visibly through a "
        "primitive outside that list, or outside the qcore_forced_ namespace, passes it. Half two "
        "(a_visible_forcing_header_would_hide_the_overlap_which_is_why_the_shipped_one_is_not) "
        "measures ONE mutation - the pthread-mutex park-and-release shape wave 39 actually built - "
        "going 0 of 20 against the shipped header's 20 of 20; it is a witness that visible "
        "synchronization can hide a race, not a proof that every visible design does. DISCHARGE: a "
        "property-shaped check (for instance, that no symbol the header references is one "
        "ThreadSanitizer intercepts) or an argument that the namespace check is exhaustive; or state "
        "permanently as the measured boundary of the enforcement.",
    ),
]


def load():
    return json.loads(MATRIX.read_text(encoding="utf-8"))


def save(matrix):
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")


def main():
    matrix = load()
    followups = matrix["named_followups"]
    by_id = {item["id"]: item for item in followups}
    if "--show" in sys.argv:
        for id_ in RETIRE + list(AMEND):
            print(f"## {id_}")
            print(json.dumps(by_id.get(id_, {"id": id_, "MISSING": True}), indent=1, ensure_ascii=False))
        return
    before = len(followups)
    for id_ in RETIRE:
        if id_ not in by_id:
            print(f"{id_}: NOT REGISTERED (already retired?)")
        else:
            print(f"## {id_} - RETIRED")
    followups = [item for item in followups if item["id"] not in RETIRE]
    for id_, appended in AMEND.items():
        item = by_id.get(id_)
        if item is None:
            print(f"{id_}: NOT REGISTERED - amendment not applied")
            continue
        if appended.strip() in item["why"]:
            print(f"{id_}: amendment already present")
            continue
        item["why"] = item["why"] + appended
        print(f"## {id_} - AMENDED (appended, nothing rewritten)")
    present = {item["id"] for item in followups}
    for item in NEW:
        if item["id"] in present:
            print(f"{item['id']}: already registered")
            continue
        followups.append(item)
        print(f"## {item['id']} - REGISTERED")
    matrix["named_followups"] = followups
    save(matrix)
    print(f"named_followups {before} -> {len(followups)}")


if __name__ == "__main__":
    main()
