#!/usr/bin/env python3
"""Register the wave-36 residual names `--residuals` found reaching no tracker.

`named_followups` is a `hand_maintained` section: the generator carries it
verbatim, so an entry is added here and `dimensions.named_followups` follows on
the next regeneration. Never the other way round (`D02-DIMENSIONS-FOLLOWUP-COUNT`).

Each `why` is drawn from the source report's own §9 text, and each carries a
DISCHARGE clause taken from that report rather than invented here.

Run from the repository root:  python3 docs/reports/wave36-close/scripts/register_followups.py
"""

import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")

W36 = "docs/reports/w36-corpus-contracts-3/README.md"
CENSUS = "docs/reports/c-gateway-indirect-census-2/README.md"

ENTRIES = [
    # --- W36-CORPUS-CONTRACTS-3 §9, all nine, its §10 item 9 asked for them ---
    {
        "id": "W36-COVER-HIDDEN-BLOCKER",
        "title": "A sole-blocker cover is an upper bound on function moves, never a forecast",
        "why": "A refusal reachable only after the first one is removed is invisible to a cover "
               "built from emitted leaves: it cost card W36-CORPUS-CONTRACTS-3 three of its 23 "
               "predicted function moves, the cJSON_Delete trio exchanging its decided leaf for a "
               "`nested call ... inside a modeled call` refusal that was unreachable while the "
               "outer call was uncontracted. This bounds what any future cover card may promise. "
               "DISCHARGE: a cover modelling the projection's reachability, not only its emitted "
               "leaves.",
        "source": W36,
    },
    {
        "id": "CJSON-DELETE-NESTED-DETACH",
        "title": "Three cJSON functions exchanged a decided leaf for a nested-call refusal",
        "why": "The three cJSON functions W36-CORPUS-CONTRACTS-3 predicted would move did not: "
               "each exchanged its decided leaf for `nested call ... inside a modeled call`. "
               "DISCHARGE: nested-call-in-argument-position admission, or W4 summaries for the "
               "three `cJSON_DetachItemFrom*` callees, which are in the same unit.",
        "source": W36,
    },
    {
        "id": "KAFKA-MEM-FREE-SUMMARY-CONFLICT",
        "title": "W4 derives Released{nonnull} where the faithful declaration says nullable",
        "why": "The corpus's first `summary_conflict` (0 -> 1), and it costs `rd_kafka_mem_free` "
               "its own Safe. The declaration is the faithful one - `rd_free(NULL)` reaches "
               "`free(NULL)` - and declaring `nonnull` to silence the conflict would assert a "
               "precondition the body does not have (constraint 4), so this is constraint 8's "
               "gate working in the conservative direction and not a defect. It is still a live "
               "W4 defect with a concrete repair. DISCHARGE: a W4 derivation that reads a "
               "delegate's declared nullability instead of defaulting a released formal to "
               "`nonnull`.",
        "source": W36,
    },
    {
        "id": "MK-PRINT-TAIL-ACTUALS",
        "title": "26 sites still refuse arguments beyond mk_print's declared parameters",
        "why": "The `resource_variadic_data_tail()` clause admits nothing on its own, as designed, "
               "so 26 sites keep the `arguments beyond the declared parameters of mk_print` "
               "refusal. DISCHARGE: a per-site tail-actual admission, or acceptance as a measured "
               "boundary.",
        "source": W36,
    },
    {
        "id": "CJSON-DELETE-SIBLING-CHAIN",
        "title": "cJSON_Delete releases a reachable set; v1 can only state the fact about one parameter",
        "why": "`cJSON_Delete` releases the whole `next` chain and the subtree beneath it; the "
               "declaration states the fact about `item` alone, so the corpus carries a "
               "declaration weaker than the C. DISCHARGE: a v1 mode meaning 'releases a set "
               "reachable from this parameter', or a call site binding two items of one chain.",
        "source": W36,
    },
    {
        "id": "LIBC-MEM-RESULT-ALIAS",
        "title": "v1 cannot say a result aliases parameter 1, so memcpy/memset stay undeclared",
        "why": "`memcpy`/`memset` were read and DELIBERATELY NOT declared by "
               "W36-CORPUS-CONTRACTS-3, because resource-v1 has no way to say 'the result aliases "
               "parameter 1'. Worth roughly 9 sole-blocked functions plus 130 leaves for one "
               "vocabulary addition, which makes it the highest-yield single item the card found. "
               "DISCHARGE: a v1 result mode meaning exactly that.",
        "source": W36,
    },
    {
        "id": "ADD-NAME-COLLISION",
        "title": "`add` cannot be declared: three definitions, three signatures, one of them static inline",
        "why": "Declined by W36-CORPUS-CONTRACTS-3 because the pinned corpus carries three "
               "distinct definitions of `add` with three signatures, the cmetrics one "
               "`static inline` with no header declaration, and a `.qcore` declaration is "
               "corpus-wide. DISCHARGE: a unit-scoped declaration vocabulary, which v1 has none "
               "of; W4 summary inference is the right lever meanwhile.",
        "source": W36,
    },
    {
        "id": "W36-CTRACES-FAMILY-FIRST",
        "title": "contract/qcore/ctraces/ is a new family whose convention is one card's choice",
        "why": "The family has one module. Its manifest placement (after `cmetrics/`, before "
               "`monkey/`) and the module name `context_lifetime.qcore`, paired with cmetrics' "
               "identically-named sibling, are W36-CORPUS-CONTRACTS-3's choice alone and no "
               "convention was found to follow. DISCHARGE: integrator confirmation, or a second "
               "`ctraces` module settling the convention.",
        "source": W36,
    },
    {
        "id": "STANDALONE-HOST-CLOSED-INPUT-LOAD",
        "title": "The sixth constraint-16 row measured failing at mid-range load, after its repair",
        "why": "`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` "
               "(crates/qcore-verifier-host/tests/standalone_host.rs:175) failed once inside a "
               "full gate at load average ~40 on 16 cores and passed 12/12 in isolation at load "
               "23.7 to 27.6. This is the row's first measurement at the MIDDLE of the load range "
               "- wave 35 measured it at ~100 and at ~6.5 - and it is post-poll(2)-repair, so the "
               "repair narrows the failure window without closing it. The card proposed the "
               "measurement as ADDING a sixth row; the row had been the sixth since the wave-35 "
               "close, so the WAVE-36-CLOSE declined that half and landed the measurement beside "
               "the existing row instead. DISCHARGE: the 1s wall-clock bound is pinned against a "
               "measured margin, or the repair is reopened - never by weakening the bound.",
        "source": W36,
    },
    # --- C-GATEWAY-INDIRECT-CENSUS-2 §9, all nine, its §12.2 left them to the close ---
    {
        "id": "BASE-TYPE-READER-UNMEASURED",
        "title": "78.5% of unresolved-member leaves carry no record tag: a reader nobody has measured",
        "why": "10,514 of the 13,396 unresolved-member leaves carry no record tag at all, because "
               "the reader never committed to the BASE's declared type and the member was never a "
               "question. That is `classify_specifiers`, typedef resolution and `read_locals` - a "
               "different reader from the record-body one C-GATEWAY-INDIRECT-CENSUS-2 "
               "instrumented. Every reach number in that report is therefore a statement about "
               "the routes it measured and not about the family's ceiling, which is why the card "
               "recommends cutting no wave-37 arbiter on this family. DISCHARGE: instrument the "
               "base-type reader the same way - what it refuses, on which spelling - and publish "
               "the sole-blocked reach of repairing each, BEFORE any card is cut on it.",
        "source": CENSUS,
    },
    {
        "id": "CENSUS-TWO-PREMISE-READINGS",
        "title": "Two over-approximations of constraint 5's one sentence disagree on 16% of the family",
        "why": "`FunctionTypes::may_bind` (bound_names) and `UntrackedBases::never_bound` "
               "(untracked_bases) disagree on 2,933 leaves (16.0%): 2,893 where may_bind says the "
               "base is bindable and never_bound says it is not, and 40 the other way. Constraint "
               "5's wording is may_bind's, and may_bind is the stricter refuser here, so a card "
               "reading never_bound works in a space roughly FOUR TIMES larger than the "
               "constraint describes without saying so. Neither function is defective; they are "
               "consumed by different rules. It is a trap for any wave-37 arbiter. DISCHARGE: "
               "state in constraint 5 which reading the premise names, or prove the two agree "
               "wherever an admission consults them.",
        "source": CENSUS,
    },
    {
        "id": "RECORD-BODY-TOTAL-OR-REJECT",
        "title": "One unparsable member declaration makes every member of that record unclassifiable",
        "why": "`TypedAccessModel::read_record_body` refuses a whole record body on any '(', ')', "
               "'{', '}', ':' or '=', so 173 of 909 record tags (19.0%) in the pinned corpus are "
               "refused wholesale - 98 on '{', 66 on '(', 9 on ':' - accounting for 2,713 "
               "unresolved-member leaves. The one genuine CODE DEFECT the census found, filed on "
               "its own merits. It is NOT a promising arbiter: the measured ceiling is 2 "
               "sole-blocked functions, 1 with constraint 5's premise intact. DISCHARGE: a "
               "per-member total-or-reject reader - classify the member declarations that parse, "
               "mark the ones that do not as explicitly unreadable so they stay refused - landed "
               "with its own measured re-pin, or a recorded decision that the 2-function reach "
               "does not justify the risk. NOT A DISCHARGE: citing the 2,713 leaves.",
        "source": CENSUS,
    },
    {
        "id": "POISON-UNMASK-ON-ADMISSION",
        "title": "Admitting an access can move counts in the admitting direction for unrelated reasons",
        "why": "A cross-cutting soundness hazard for every admission candidate, and the one most "
               "easily missed. Admitting an access makes its statement no longer refused, so that "
               "statement's RefusedWrites no longer poisons any binding. Constraint 10a already "
               "records that sparing a binding can UNMASK a stale scalar fact a NoOp mutation "
               "left behind (AOM-POISON-NOOP-INCREMENT: `pass++` lowering to NoOp so a "
               "Clear-bound scalar stays definitely-null for ever, making a `pass != 0` edge "
               "Infeasible and suppressing the refusals inside it). A wave-37 arbiter's re-pin can "
               "therefore move counts for reasons unrelated to the access it admits. DISCHARGE: "
               "the arbiter measures, for its own admitted set, how many refusals disappear "
               "through an edge becoming infeasible rather than through the access being "
               "modelled, and publishes the two numbers apart.",
        "source": CENSUS,
    },
    {
        "id": "LOAD-SENSITIVE-ROW-UNNAMED",
        "title": "A seventh test shows constraint 16's signature and is none of its six named rows",
        "why": "`a_forced_override_never_weakens_an_execution` "
               "(crates/qcore-verifier-host/tests/execution_quota.rs:777) failed with "
               "Io(\"Broken pipe (os error 32)\") in a workspace run deliberately concurrent with "
               "two pinned corpus audits, and passes in the card's isolated gate. That is "
               "constraint 16's exact signature and crate, but the test is not one of its six "
               "named rows nor the deliberately-excluded seventh. Filed rather than promoted: "
               "each constraint-16 row carries a measured load figure and one observation under a "
               "heavy self-inflicted load is not one. Named in constraint 16 by WAVE-36-CLOSE so "
               "the list is known incomplete rather than assumed complete. DISCHARGE: a card that "
               "bisects the load at which it flips, as vp04_vp07_vp08 was measured failing at 50 "
               "and passing at 37, and either adds the row with its figure or records that it is "
               "the same mechanism as an existing row. NOT A DISCHARGE: re-running until green.",
        "source": CENSUS,
    },
    {
        "id": "INDIRECT-EXCHANGE-PROXY-UNVALIDATED",
        "title": "The census's three-way split rests on a proxy never validated against an admission",
        "why": "A statement carrying an uncontracted call, or a second chain of a different shape, "
               "is PREDICTED to stay refused. The proxy has never been validated against an "
               "actual admission, because the census makes none. It errs toward exchange, so the "
               "CLEAR column is an upper bound - but the SIZE of the error is unmeasured. "
               "DISCHARGE: the first wave-37 arbiter to admit any shape publishes its measured "
               "three-way split beside this card's prediction for the same shape, and the "
               "difference is recorded.",
        "source": CENSUS,
    },
    {
        "id": "INDIRECT-CENSUS-FEATURE-UNBUILT",
        "title": "The indirect-census instrumentation is built by no CI job and will rot silently",
        "why": "The `indirect-census` feature is built by no CI job and by no gate step other than "
               "the one the card adds to its own gate script. A later refactor of `legacy_c.rs` "
               "or `typed_field_access.rs` will break it silently, and the next card wanting the "
               "census pays to repair it. The card states this is a DECISION rather than a task "
               "and declines to take it, since the answer depends on whether wave 37 takes a "
               "gateway card at all - and the card's own recommendation is that it should not "
               "take this one. DISCHARGE: either a CI job building "
               "`-p qcore-c-gateway --features indirect-census`, or a recorded decision that the "
               "instrumentation is disposable and a future census re-derives it.",
        "source": CENSUS,
    },
    {
        "id": "CENSUS-DECLARATION-WINDOW-APPROXIMATE",
        "title": "The Declaration arm's census reads a window the gate did not read",
        "why": "For the 15 `Declaration`-arm leaves the gate's predicate is not "
               "`contains_unmodeled_access` but a separate bracket/dot test over the declaration "
               "prefix, so the census reads a different window from the gate. 15 leaves of "
               "18,555; recorded rather than repaired. DISCHARGE: give the declaration arm its own "
               "census window, or state the arm is out of every arbiter's scope and drop the axis "
               "for it.",
        "source": CENSUS,
    },
    {
        "id": "CENSUS-UNEXPLAINED-LEAF",
        "title": "One published leaf of 18,555 is explained by no census statement",
        "why": "0.01% of the family. The likeliest cause is the `for`-clause path: "
               "`classify_simple` is also called for a `for` init and increment clause, where the "
               "statement the parser attributes is the `for`'s own line rather than the clause's "
               "first token, so the census's (function, line) key can miss. Not chased, because it "
               "is one leaf and the direction is understood. DISCHARGE: thread the statement's "
               "attributed line into `classify_simple` rather than reading it off the first "
               "token, and re-run; or identify the leaf and record why it is unexplainable.",
        "source": CENSUS,
    },
]


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    followups = matrix["named_followups"]
    existing = {entry["id"] for entry in followups}
    before = len(followups)
    added = []
    for entry in ENTRIES:
        if entry["id"] in existing:
            print(f"  already registered, skipped: {entry['id']}")
            continue
        followups.append({**entry, "recorded_by": "WAVE-36-CLOSE"})
        added.append(entry["id"])
    MATRIX.write_text(
        json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"named_followups {before} -> {len(followups)} (+{len(added)})")
    for name in added:
        print(f"  + {name}")
    print("\nRETIREMENTS BY THIS SCRIPT: none. It only adds.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
