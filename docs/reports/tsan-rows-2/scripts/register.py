#!/usr/bin/env python3
"""TSAN-ROWS-2 (wave 39): the tracker edits, in one place, on the wave-38 close's precedent.

`named_followups` is hand-maintained (the generator carries it verbatim), so this
retires three entries whose discharge conditions are met word for word (each
verified registered, and printed with `--show` before anything moves), registers
six with `recorded_by` set to this card, and rewrites in the generator's byte
format (indent=1, ensure_ascii=False, trailing newline); `dimensions.named_followups`
is then re-derived by

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/tsan-rows-2/gate.json

Idempotent. Two entries the brief names are deliberately NOT moved here and are
left to the integrator with proposed text in the report's "For the integrator":
MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION (this card takes its second arm, the
recorded decision) and TASK-ROUTE-TSAN-EXECUTED-SCHEDULES (met on repeated runs
and named schedules, NOT on the failure arms).
"""
import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/tsan-rows-2/README.md"
CARD = "TSAN-ROWS-2"

RETIRE = [
    "TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES",
    "WLT-EXERCISE-TSAN-TEST-SILENT-ARMS",
    "CT07-TWO-THREAD-REPLAY-ORDER",
]


def entry(id_, title, why):
    return {"id": id_, "title": title, "why": why, "source": REPORT, "recorded_by": CARD}


NEW = [
    entry(
        "TSAN-FORCED-SCHEDULE-FAILURE-ARMS",
        "No ThreadSanitizer row takes a task program's failure arms; schedule forcing cannot reach them",
        "Card TSAN-ROWS-2 (wave 39) forced the task route's schedules through the runtime's own "
        "QCORE_TASK_CREATE/QCORE_TASK_JOIN seam and ran every subject clean under two forced schedules "
        "plus the unforced one, but every run still takes the SUCCESS path: a failed qcore_buffer_new "
        "and a failed spawn (the Result::Err arms of the flagship and of CT07's case) are executed by "
        "no TSan row. Forcing the SCHEDULE cannot reach them - they are selected by an outcome, not by "
        "an interleaving - and the card did not compose the binding with the outcome hooks that can: "
        "CT07's own refuse_second_spawn.h already redefines QCORE_TASK_CREATE to refuse the second "
        "creation (the CT heap runner executes it under ASan/UBSan, not TSan), and QCORE_BUFFER_MALLOC "
        "can return NULL. This is the first narrowness TASK-ROUTE-TSAN-EXECUTED-SCHEDULES recorded, "
        "carried forward by name now that its other half (repeated runs, named schedules) is met. "
        "DISCHARGE: TSan rows composing tests/support/thread_sanitizer.rs's binding with a refusing "
        "QCORE_TASK_CREATE and a NULL-returning QCORE_BUFFER_MALLOC on the flagship and CT07's case, "
        "stdout compared with the reference run scripted for the same outcomes, each arm shown "
        "non-vacuous by mutation; or state permanently that TSan evidence covers the success path only.",
    ),
    entry(
        "TSAN-FORCED-CATCH-POWER-EQUAL",
        "No emitted-program mutation was found that the unforced ThreadSanitizer harness misses and a forced schedule catches",
        "MEASURED by card TSAN-ROWS-2 (wave 39), docs/reports/tsan-rows-2/artifacts/catch-rates.json: "
        "the overlap mutation (both workers' byte writes rebased onto one byte) on CT07's case and on "
        "the flagship, and the join-edge-removal mutation on shared_readers and on CT07's case, are "
        "each reported in 20 of 20 runs under the unforced schedule AND under both forced schedules. "
        "The brief anticipated a mutation the unforced harness would miss at least once; none of these "
        "is it, and the reason is the one wave 37 stated - ThreadSanitizer detects a missing "
        "happens-before edge, not an observed interleaving, so two unordered accesses that both "
        "execute are reported whatever order the forcing puts them in. What the forced schedules "
        "measurably change is the REPORT SET, not the verdict: under halt_on_error=0 the shared_readers "
        "join-edge mutation yields 16 race reports unforced and 6 under either forced schedule. What "
        "they add is executed-schedule coverage: each forced row means 'this schedule, forced', with "
        "the realised schedule read back from the harness trace. The two asymmetries the card DID find "
        "run the other way and are registered separately (TSAN-FORCED-HARNESS-SYNC-INVISIBILITY, "
        "TSAN-FORCED-GATE-OPENED-BY-THE-JOIN). DISCHARGE: exhibit an emitted-program mutation the "
        "unforced harness provably misses - the candidate class is TSan's bounded shadow history "
        "(four cells per eight-byte word), which none of this route's programs can exhaust on the "
        "payload word - or state permanently that forcing adds coverage and not detection power.",
    ),
    entry(
        "TSAN-FORCED-GATE-OPENED-BY-THE-JOIN",
        "The forcing header releases workers at the creator's join, so a mutation that deletes the join without opening the gate hides every race",
        "MEASURED by card TSAN-ROWS-2 (wave 39): replacing the emitted QCORE_TASK_JOIN with the "
        "unforced 50 ms no-edge stand-in (named by its function so the forcing header's macro does not "
        "reach it) is reported 20 of 20 runs unforced and 0 of 20 under either forced schedule, with "
        "the schedule trace empty - no worker was ever released, the process exited with both parked, "
        "and ThreadSanitizer had nothing to see (artifacts/catch-rates.json, "
        "'join-deleted-gate-closed'). This is a failure mode the unforced harness does not have: a "
        "forced-schedule harness whose release trigger is the join loses the race with the join. It is "
        "pinned by a_join_deleted_without_opening_the_gate_hides_the_race_under_forcing, and it is why "
        "QCORE_TSAN_DROP_JOIN is a gate-opening stand-in under forcing (a join-edge mutation must "
        "remove the edge and keep the release). The trigger is the join because it is the one point "
        "every program of this route reaches after creating its cohort with no per-program count to "
        "supply. DISCHARGE: a release trigger independent of the join (a per-program cohort count "
        "supplied by the row, or a creator-side rendezvous the runtime already emits), with the "
        "gate-closed row re-measured under it; or state permanently, keeping the pinning test.",
    ),
    entry(
        "TSAN-FORCED-BODY-OVERLAP-NOT-FORCED",
        "The together schedule forces creation, release and liveness, not the physical overlap of the two stores",
        "Card TSAN-ROWS-2 (wave 39). Under Schedule::ForcedTogether every worker of a cohort is "
        "created and released before any body runs and every thread of it stays live until every "
        "body has returned (an exit barrier), asserted on the harness's own stderr trace: all starts "
        "precede all leaves. What that does NOT force is that the two workers' store instructions "
        "overlap in time: nothing outside an emitted body can place a rendezvous inside it, and the "
        "harness changes no emitted byte by design. ThreadSanitizer's verdict does not depend on the "
        "overlap (it reasons about happens-before), so the clean rows' meaning is unaffected; what is "
        "unaffected too is CT07-TWO-THREAD-REPLAY-ORDER's observation that no evidence witnesses the "
        "two writes overlapping in time, which this card retires on its second arm (the TSan row is "
        "landed) while restating this boundary in its place. DISCHARGE: state permanently (the honest "
        "default), or an in-body rendezvous reachable through a runtime hook the emitter already "
        "carries - there is none at qcore_buffer_byte_write today, and adding one is an emission change "
        "with its own card.",
    ),
    entry(
        "TSAN-FORCED-HARNESS-SYNC-INVISIBILITY",
        "A forced schedule realised with synchronization ThreadSanitizer can see hides worker-versus-worker races; the shipped header is uninstrumented for that reason",
        "MEASURED by card TSAN-ROWS-2 (wave 39), and it is the card's finding. The first version of "
        "the forcing header (artifacts/schedule_forcing.v1-pthread.h) parked and released workers on a "
        "pthread mutex and condition variable; under the reverse policy the overlap mutation on CT07's "
        "case went UNREPORTED (artifacts/catch-point-reverse-mutex-edge.log) while the unforced and "
        "together schedules reported it: worker 2's byte write preceded its unlock, worker 1 woke by "
        "acquiring the same mutex, and TSan saw a release/acquire edge between the two writes. The "
        "shipped SCHEDULE_FORCING compiles every harness function with no_sanitize(\"thread\"), keeps "
        "its bookkeeping behind an uninstrumented spinlock and parks a worker on a relaxed flag with "
        "sched_yield, so the only edges TSan observes are the program's own create and join; the "
        "reverse row then reports the overlap 20 of 20. The reproduction is shipped as mutate.sh's "
        "hook_visible_to_tsan (re-instrument the header and take the spinlock once before a body: the "
        "reverse arm goes red by MISSING the race). This is a rule about any future forcing harness, "
        "stated here so it is indexed. DISCHARGE: keep the header uninstrumented and the reproduction "
        "mutation red whenever the header changes; a header that must synchronize visibly needs a "
        "row showing each forced schedule still reports the overlap mutation.",
    ),
    entry(
        "CT46-NOTES-PREDATE-TSAN-ROW",
        "CT46's fixture notes and manifest profile still say ThreadSanitizer is run nowhere in the repository",
        "Found by card TSAN-ROWS-2 (wave 39) while giving CT07's row its TSan citation. "
        "compiler/tests/fixtures/ct/CT46/expected.json's notes and the manifest's CT46 semantic_profile "
        "both read 'TSan is not run anywhere in this repository and no TSan-shaped claim is made "
        "(MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION)'. That was true at wave 36; since wave 37 the "
        "crossing has a TSan row on a CT46-shaped twin (crates/qcore-core/tests/unit/mutex_c_emit.rs) "
        "and since this card on CT46's OWN fixture lowering "
        "(ct46s_own_case_runs_clean_bound_and_the_forcing_header_finds_no_seam: dies unbound at the "
        "one fault, runs clean bound, stdout byte-identical to interpret_mutex_program's observation, "
        "and the forcing header attaches to nothing on this backend - no worker released, no schedule "
        "line). The row is serial by construction and claims no contention. Not edited by this card, "
        "which owns CT07's citation only. DISCHARGE: a citation appended to both CT46 texts on the "
        "CT07 precedent (docs/reports/tsan-rows-2/scripts/cite_ct07.py), naming the row and the "
        "recorded decision that this route's native evidence stays serial by construction.",
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
        for id_ in RETIRE:
            print(f"## {id_}")
            print(json.dumps(by_id[id_], indent=1, ensure_ascii=False))
        return
    before = len(followups)
    for id_ in RETIRE:
        if id_ not in by_id:
            print(f"{id_}: NOT REGISTERED (already retired?)")
    followups = [item for item in followups if item["id"] not in RETIRE]
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
