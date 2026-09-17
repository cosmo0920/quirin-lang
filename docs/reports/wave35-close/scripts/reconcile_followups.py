#!/usr/bin/env python3
"""Wave-35 close: register the classified residuals and retire the discharged.

`named_followups` is `hand_maintained` in the generator's `section_ownership`
(constraint 25), so this script appends and removes entries and then sets
`dimensions.named_followups` from the list's own length, writing with the
generator's byte format (`indent=1`, `ensure_ascii=False`, trailing newline).
Run `docs/reports/remaining-d02/build_matrix.py` afterwards to confirm the
fixed point.

WHY THIS SCRIPT EXISTS. `scripts/reconciliation_sweep.py --since a300a39d
--residuals` returned **62** residual-shaped names the tracker does not carry -
the first honest count after card `SWEEP-BLIND-SPOTS` widened the scan past
`docs/reports/` to `docs/development/increments/` and the standing-constraints
text, and narrowed `TRACKERS` to `named_followups` alone. The close classified
every one of the 62 by hand against the tree. This script lands the two classes
that move the ledger; the other three classes (card names, a bucket label, and
names already carried as deferred decisions) are recorded in the close's
`reconciliation.json` and move nothing, which is the point of classifying
rather than registering reflexively.

Idempotent: an id already present is skipped, an id already absent is not
removed twice.
"""

import json
import pathlib

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")

# ---------------------------------------------------------------------------
# RETIRED. Each one is discharged by a named card whose evidence the close
# measured against the tree; none is retired because it went quiet.
# ---------------------------------------------------------------------------
RETIRE = {
    "WAVE-32-CLOSE-UNSWEPT": (
        "Discharged by card SWEEP-BLIND-SPOTS (wave 35), which retro-swept wave 32. "
        "Measured at the wave-35 close: docs/reports/wave32-close/reconciliation.json now exists "
        "and carries 67 verdicts over all SEVEN of wave 32's merged cards "
        "(ct-composed-native, ct-wire-lane-4, docs-compact-5, mutex-share-cross, proof-leg-batch, "
        "trace-alphabet-lease-boundary, w32-reconcile-sweep), where before it held only gate.json. "
        "Its concrete cost is itself now landed: see MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED, "
        "also retired by this close."
    ),
    "SWEEP-RESIDUALS-SCANS-REPORTS-ONLY": (
        "Discharged by card SWEEP-BLIND-SPOTS (wave 35), repair 1. --residuals now also reads "
        "docs/development/increments/ and the standing-constraints text of AGENTS.md and CLAUDE.md "
        "(RESIDUAL_SOURCES in scripts/reconciliation_sweep.py). Measured at this close: the widened "
        "scan returned 62 names where the report-only scan would have returned a small fraction, and "
        "the names it now sees include whole families introduced in waves 29-30 increments that the "
        "old scan could not reach by construction."
    ),
    "RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK": (
        "Discharged by card SWEEP-BLIND-SPOTS (wave 35), repair 2. TRACKERS is now "
        "named_followups alone; an AGENTS.md or CLAUDE.md mention is reported as 'a RULE, not a "
        "tracker' with the file it was found in. Measured at this close: 11 of the 62 names carried "
        "exactly that annotation, and every one turned out to be a CARD NAME rather than open work - "
        "which is the distinction the repair exists to force a human to draw."
    ),
    "WAVE-BASE-UNSWEPT": (
        "Discharged by card SWEEP-BLIND-SPOTS (wave 35), repair 3. --since is inclusive of its own "
        "base commit, --range A..B takes a seam explicitly, and --uncarded lists commits in a range "
        "writing no card report. Measured at this close: --since a300a39d --uncarded returned FOUR "
        "commits and the wave-open commit a300a39d was one of them, which is precisely the commit "
        "the old exclusive range could not see."
    ),
    "WAVE-33-DEFERRED-DECISIONS-UNTRACKED": (
        "Discharged by card SWEEP-BLIND-SPOTS (wave 35), repair 4. A close's deferred_decisions are "
        "re-listed at every later close until each carries resolved or withdrawn with a ground. "
        "Measured at this close: check() re-listed all SEVEN of the wave-34 close's deferred "
        "decisions - the four inherited from wave 33 among them - and this close's own record "
        "restates each. The DECISIONS themselves remain open; what is discharged is their invisibility."
    ),
    "WAVE-34-CLOSE-UNSWEPT": (
        "Discharged by card WAVE-34-CLOSE (merged aba64a7a in wave 35). Measured at the wave-35 "
        "close: docs/reports/wave34-close/ now holds README.md, gate.json and reconciliation.json, "
        "the last carrying verdicts for all 25 integrator-addressed items of stability-reeval-1, "
        "c-gateway-return-call-w4 and docs-compact-6 - the exact discharge this entry named. That "
        "close found ONE unapplied item (four residuals of C-GATEWAY-RETURN-CALL-W4 lost when "
        "DOCS-COMPACT-6 merged afterwards) and registered them."
    ),
    "VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED": (
        "Discharged by card VERIFIER-HOST-CONTROL-BUDGET (wave 35) via its own discharge option (c) "
        "- an evidenced host investigation PLUS the repair the investigation justified - which is "
        "stronger than the option (a) documentation arm the wave-34 close left open. The mechanism "
        "is named and measured: an 8 KiB pipe capacity against a 1 ms backoff quantum put a "
        "throughput ceiling under the 8 MiB in / 8 MiB out control, repaired with poll(2) in "
        "crates/qcore-verifier-host/src/lib.rs with the TEST FILE UNTOUCHED (the card's diff is one "
        "file). The control now passes 38/38 post-repair. TWO CORRECTIONS travel with this retirement "
        "rather than being dropped: this entry's 'deterministically 3/3 in isolation' was three "
        "samples of a biased coin - 22 pre-repair isolated repetitions gave 7 pass / 15 fail - and "
        "card MUTEX-SHARE-CROSS-ARMS independently saw the row red in isolation, red under a full "
        "suite AND green under a full suite on one host in one session. The entry's CONCLUSION held; "
        "its determinism claim did not. What remains open is carried by five successor entries "
        "registered here (VERIFIER-HOST-CONTROL-MARGIN-UNPINNED is the one that would have caught "
        "this defect in wave 33), on the TRACE-ALPHABET-LEASE-BOUNDARY pattern where the successor "
        "carries the story rather than the original being silently deleted."
    ),
    "FB-PLAN-GATE-HOST-TIMEOUT": (
        "Discharged with VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED, exactly as this entry's own text "
        "requires ('this entry closes when that one does'). The tracking half it recorded - a live "
        "failing gate row named only inside docs/plans/fluent-bit-system-coverage/README.md - is "
        "additionally closed structurally by SWEEP-BLIND-SPOTS' widened --residuals scan."
    ),
}

# ---------------------------------------------------------------------------
# REGISTERED. 27 of the 62, each measured open.
# ---------------------------------------------------------------------------
ADD = [
    # --- arbiter #1, C-GATEWAY-CONSTANT-DO-WHILE §8 (five of its six; the sixth
    # --- is answered by arbiter #2 and is recorded in the close, not here).
    {
        "id": "CONSTANT-DO-WHILE-ZERO-SPELLINGS",
        "title": "The constant do-while rule claims the single token `0`, and the other literal zeros were not exhaustively measured",
        "why": (
            "Card C-GATEWAY-CONSTANT-DO-WHILE (wave 35, arbiter #1) keys its back-edge removal on the "
            "single literal token `0` - constraint 5a's rule applied here. `0u`, `0L`, `0U`, `0x0`, `00` "
            "and '\\0' are literal zeros a `do ... while (...)` may equally spell and each KEEPS its back "
            "edge today, which is the conservative answer. Nothing in the pinned corpus's preprocessed "
            "text was found to use one in this position, but that was NOT exhaustively measured. "
            "DISCHARGE: a preprocessed census of `do ... while (E)` conditions over the 185 units that "
            "classifies every E a single-token integer constant expression, followed either by a widening "
            "of condition_is_literal_zero WITH ITS OWN MEASURED RE-PIN - widening is the admitting "
            "direction (invariant 1) - or by a recorded statement that the set is empty."
        ),
        "source": "docs/reports/c-gateway-constant-do-while/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CONSTANT-DO-WHILE-OTHER-CONSTANTS",
        "title": "`while (1)` and `for (;;)` are untouched, and modelling them removes an EXIT edge rather than a back edge",
        "why": (
            "Named by card C-GATEWAY-CONSTANT-DO-WHILE (wave 35) and carried in standing constraint 5c. "
            "The asymmetry is the whole point: `do S while (0)` runs once so removing its back edge "
            "removes a path the C does not have (admitting direction), whereas `do S while (1)` never "
            "exits, so modelling IT exactly would REMOVE THE EXIT EDGE - the accusing direction, a "
            "different risk profile. DISCHARGE: a separate card, because the exit-removing direction "
            "needs its own Lean statement (an infinite trace set, not a one-element one) and its own "
            "measured re-pin. NOT a discharge: folding it into the constant do-while rule."
        ),
        "source": "docs/reports/c-gateway-constant-do-while/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CONSTANT-DO-WHILE-LATENT-REACH",
        "title": "The claim that the rule forecloses the same false accusation across all 15,053 expansions is not quantified",
        "why": (
            "Card C-GATEWAY-CONSTANT-DO-WHILE (wave 35) §5.5 claims the rule forecloses the same false "
            "repeated-release accusation across all 15,053 `do ... while (0)` expansions. That FORWARD "
            "claim is not quantified; today's MEASURED reach is one leaf. The distinction matters because "
            "the card's headline delta (double_release 1 -> 0) rests on the measured leaf, not on the "
            "forward claim. DISCHARGE: after a later card promotes a body out of Unknown, re-measure "
            "whether any newly modelled release sits inside a `do ... while (0)` and record whether the "
            "count changed; equivalently, an instrumented count of how many of the 15,053 expansions "
            "contain a tracked-identity release at all."
        ),
        "source": "docs/reports/c-gateway-constant-do-while/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CONSTANT-DO-WHILE-SUCCESSOR-ORDER",
        "title": "CFG successor order is observable in the worklist schedule - measured by a control that was expected to be harmless and was caught",
        "why": (
            "Card C-GATEWAY-CONSTANT-DO-WHILE (wave 35) ran a mutation campaign whose CONTROL - swapping "
            "vec![body_entry, continuation] to vec![continuation, body_entry] on the UNCHANGED arm - was "
            "expected to be harmless and was CAUGHT. So successor order is observable in the worklist "
            "schedule. The card does not reorder it and records the fact rather than repairing it; this "
            "is the same scheduling sensitivity constraint 6 records for report_leaks "
            "(C02-P-ALIAS-LEAK-ORDER), now measured on a second analysis. DISCHARGE: either a statement "
            "of which analyses depend on successor order and why that is intended, or a "
            "scheduling-neutral pre-pass of the kind constraint 6 already describes, with AbstractState "
            "byte-identical."
        ),
        "source": "docs/reports/c-gateway-constant-do-while/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED",
        "title": "A pre-existing route decides a do-while from a modelled scalar VALUE, and it has no card, no Lean and no report",
        "why": (
            "Card C-GATEWAY-CONSTANT-DO-WHILE (wave 35) §6 found that a pre-existing route already "
            "decides a `do`-`while` from a modelled scalar value, where the card's own rule decides from "
            "a TOKEN. Both answers agree wherever they overlap and the scalar route answered "
            "conservatively on all four probed shapes - but it has no card, no Lean and no report of its "
            "own, and is pinned only by one test the card happened to write "
            "(a_modeled_scalar_condition_is_a_separate_pre_existing_route). Standing constraint 5c "
            "records it as 'not owned by that card'. DISCHARGE: a card that states the route's premise, "
            "measures its corpus reach, and either gives it a theorem or records it as a measured "
            "boundary - or a finding that the two routes CAN disagree, which would be the more "
            "interesting outcome."
        ),
        "source": "docs/reports/c-gateway-constant-do-while/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    # --- VERIFIER-HOST-CONTROL-BUDGET §8, plus its one inline §7.1 residual.
    {
        "id": "VERIFIER-HOST-EARLY-ARM-OVERLOAD",
        "title": "A sixth load-sensitive host row, and the only one sensitive at the TOP of the load range",
        "why": (
            "Card VERIFIER-HOST-CONTROL-BUDGET (wave 35) §5 measured standalone_host.rs's "
            "assert!(start.elapsed() < Duration::from_secs(1)), a fixed one-second guard over a 40 ms "
            "deadline in the first arm of "
            "closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request, failing 3 of 6 "
            "repetitions at 1-minute load ~100 on a 16-CPU host. Correct but load-sensitive: constraint "
            "16's shape, at the OPPOSITE end of the band from the defect that card repaired. The "
            "amendment text the card proposed for constraint 16 was landed by the wave-35 close. "
            "DISCHARGE: derive the guard from the declared budget instead of a fixed second. NOT a "
            "discharge: raising the second, or deleting the guard."
        ),
        "source": "docs/reports/verifier-host-control-budget/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "HOST-PIPE-CAPACITY-UNEXPLAINED",
        "title": "The measured pipe capacity is 8,192 bytes where PIPE_DEF_BUFFERS would give 65,536, and the repaired defect's size is a function of that number",
        "why": (
            "Card VERIFIER-HOST-CONTROL-BUDGET (wave 35) measured this host's default pipe capacity at "
            "8,192 bytes (two pages) where PIPE_DEF_BUFFERS would give 65,536 - stable across five "
            "samples, with pipe-user-pages-soft at its default 16,384 and no relevant kernel "
            "command-line argument. Capacity is one of the two factors in the ceiling arithmetic "
            "(capacity / quantum) that explains the whole wave-33 defect, so THE SIZE OF THE DEFECT THAT "
            "CARD REPAIRED IS A FUNCTION OF AN UNEXPLAINED NUMBER, and the repair's measured benefit is "
            "host-specific to that extent. DISCHARGE: identify the kernel or distribution setting that "
            "produces it, or measure the capacity on a second Linux host and record whether the ceiling "
            "arithmetic differs there."
        ),
        "source": "docs/reports/verifier-host-control-budget/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "HOST-BACKOFF-FALLBACK-ARMS-UNDRIVEN",
        "title": "Two arms of the repaired backoff_until_ready are reached by no test - defensive, not verified",
        "why": (
            "Card VERIFIER-HOST-CONTROL-BUDGET (wave 35) records two arms of its own new "
            "backoff_until_ready as reached by NO test: the empty-descriptor-set sleep (every pipe "
            "finished while the direct child is not yet reaped - transient, not deterministically "
            "drivable) and the poll failure fallback (arguably unreachable at all, since the descriptors "
            "are owned by live pipes for the duration of the call). Per CLAUDE.md's STABILITY-REEVAL "
            "discipline these are recorded as DEFENSIVE, NOT VERIFIED - the green suite is evidence "
            "about the paths it took. Both fail in the conservative direction (they degrade to the exact "
            "backoff they replaced), so the risk is bounded by DIRECTION, not by a test. This is the "
            "TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED shape in a new subsystem, in the same wave that "
            "MUTEX-SHARE-CROSS-ARMS dispositioned four such arms. DISCHARGE: drive them with an injected "
            "descriptor set, or state them as a measured boundary."
        ),
        "source": "docs/reports/verifier-host-control-budget/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "VERIFIER-HOST-READINESS-APPLE-UNRUN",
        "title": "The poll(2) readiness wait is cfg(unix) and Apple would run it, but no macOS execution is claimed",
        "why": (
            "Card VERIFIER-HOST-CONTROL-BUDGET (wave 35): backoff_until_ready is #[cfg(unix)] and poll(2) "
            "is POSIX, so Apple compiles and would RUN the repaired loop, but NO macOS execution is "
            "claimed - Linux only, as with every other observation in that card. There is explicitly NO "
            "WINDOWS SIBLING to this risk: verified in source that the non-Unix arms of verify/verify_v2 "
            "refuse by name ('bounded process providers require Unix process groups') and reach no "
            "transport loop at all. Joins CLAUDE.md's -APPLE-RUN platform family. DISCHARGE: a macOS run "
            "of the qcore-verifier-host suite at or after this card, including the large-response "
            "admitting control."
        ),
        "source": "docs/reports/verifier-host-control-budget/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "VERIFIER-HOST-CONTROL-MARGIN-UNPINNED",
        "title": "Nothing pins the repaired control's wall-clock margin, which is how the wave-33 defect survived a wave",
        "why": (
            "Card VERIFIER-HOST-CONTROL-BUDGET (wave 35): the large-response admitting control now "
            "completes in 42-112 ms against a 1,000 ms budget, but NOTHING PINS THAT MARGIN. A future "
            "change reintroducing a per-chunk quantum would be caught only when the margin was fully "
            "spent again - which is exactly how this defect survived wave 33 behind a published '0 "
            "failed'. This is the successor that carries the story of the retired "
            "VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED. DISCHARGE: a test asserting the control's elapsed "
            "time against a budget DERIVED FROM the declared timeout, or an explicit decision that a "
            "wall-clock margin assertion is not wanted in this suite - the decision recorded either way."
        ),
        "source": "docs/reports/verifier-host-control-budget/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "VERIFIER-HOST-NPROC-ROW-UNVERIFIABLE-UNDER-WAVE",
        "title": "Constraint 16's RLIMIT_NPROC row cannot be cleanly verified while sibling cards run, because they are inside its measurement",
        "why": (
            "Card VERIFIER-HOST-CONTROL-BUDGET (wave 35) §7.1 measured the RLIMIT_NPROC conformance row "
            "- the first of constraint 16's five - red 5 of 6 EVEN IN ISOLATION, where 'isolation' meant "
            "the test alone in its process on a machine still carrying load 30. The sharper point is not "
            "'a known flake': the row's own comment says RLIMIT_NPROC counts every process of the REAL "
            "UID, not of this process tree, and with five sibling cards running the account held 616-639 "
            "processes throughout. For THIS row the sibling cards are not ambient load - they are INSIDE "
            "ITS MEASUREMENT, and they make its premise hold far more extremely than intended. "
            "Constraint 16's instruction is to verify in isolation and never weaken; the card verified, "
            "could not reach a quiet account, and recorded both facts rather than claiming it green. "
            "DISCHARGE: verify the row on an account with no sibling wave running and record the process "
            "count observed, or state the account-quietness precondition in constraint 16 itself so a "
            "future card is not asked for an unreachable measurement."
        ),
        "source": "docs/reports/verifier-host-control-budget/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    # --- MUTEX-SHARE-CROSS (wave 32), unswept until SWEEP-BLIND-SPOTS.
    {
        "id": "MUTEX-SHARE-CROSS-EMISSION",
        "title": "The declared mutex crossing is checked and never executed - no C is emitted for any multi-task mutex trace",
        "why": (
            "Named by card MUTEX-SHARE-CROSS (wave 32) and carried verbatim in standing constraint 15o, "
            "landed by the wave-35 close. mutex_c_emit::is_single_task_trace gives NO C to any multi-task "
            "mutex trace, a pre-existing boundary that card did not move, so the crossing is CHECKED, "
            "NEVER EXECUTED and no native or data-race evidence may be claimed for it. Wave 35's "
            "CT-CTWIN-LANE made this MEASURED ON A CT ARTIFACT rather than inherited: CT46 is the "
            "corpus's first ADMITTED case that deliberately stops at the checker, the emission boundary "
            "pinned in both directions with a control that does render, which is why four of CT46's "
            "stages read `unassessed` and none reads `executed`. DISCHARGE: a C backend for multi-task "
            "mutex traces (real thrd_t plus the existing mtx_t emission), then a sanitized native run "
            "byte-compared against the reference interpreter, on the WRITE-LEND-TASKS native-run "
            "precedent - which would move CT46's four stages and take the sanitized-native numerator off "
            "9 for the first time since wave 32. CT-CTWIN-LANE §9 item 5 states the three coupled edits "
            "that must land in ONE change or the harness refuses."
        ),
        "source": "docs/reports/mutex-share-cross/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "MUTEX-SHARE-CROSS-TASK-BUFFER",
        "title": "The task route's own<Buffer> still cannot cross under a mutex; MUTEX_SHARING_GAP stays false",
        "why": (
            "The half of CT46's gap that card MUTEX-SHARE-CROSS (wave 32) left open and named: the TASK "
            "route has no mutex, so its own<Buffer> still cannot cross under one, and MUTEX_SHARING_GAP "
            "stays false - a fact standing constraint 15o now carries. Re-measured by wave 35's "
            "CT-CTWIN-LANE while mapping CT46: MUTEX_SHARING_GAP.requestable stays false in that lane's "
            "own stage evidence. DISCHARGE: a mutex for the task route - or a DECISION that the two "
            "routes' payloads stay distinct kinds, recorded as such. The second arm is a legitimate "
            "outcome and must be recorded rather than left implicit."
        ),
        "source": "docs/reports/mutex-share-cross/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "MUTEX-SHARE-CROSS-MULTI",
        "title": "The declared crossing is one wrapper, one payload, one mutex, no nesting",
        "why": (
            "Card MUTEX-SHARE-CROSS (wave 32) bounded its clause to one wrapper, one payload, one mutex "
            "and no nesting. Reported by the wave-34 close's hand sweep as reaching no tracker and left "
            "to the retrospective wave-32 sweep; that sweep (SWEEP-BLIND-SPOTS, wave 35) ran and the "
            "name still reached no tracker, so the wave-35 close registers it. DISCHARGE: a second "
            "declared crossing in one child, or a nested declaring child, each with its own rule for "
            "what the inner child inherits - MEASURED, NOT ASSUMED, in the card's own words."
        ),
        "source": "docs/reports/mutex-share-cross/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    # --- CT42-PROTOCOL-CELL (wave 31) §6.
    {
        "id": "CT42-CELL-DUP-SOURCE",
        "title": "No source form creates two names over one open description (the dup/fdopen shape)",
        "why": (
            "Card CT42-PROTOCOL-CELL (wave 31) §6. A source form creating two names over one open "
            "description needs a census-clean surface AND a custody story for the second name; neither "
            "exists. DISCHARGE: that source form, with its keyword census under constraint 14 and an "
            "explicit rule for what the second name owes."
        ),
        "source": "docs/reports/ct42-protocol-cell/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CT42-CELL-PENDING-PHASE",
        "title": "A surviving pending connect would need a pending-phase cell or a refusal, and neither is written",
        "why": (
            "Card CT42-PROTOCOL-CELL (wave 31) §6: if a route ever admits a program with a SURVIVING "
            "PENDING CONNECT, the pending-phase {connector, listener} coupling needs a cell or a refusal. "
            "Today no route admits one, so the gap is latent rather than live - which is exactly the "
            "shape CLAUDE.md's STABILITY-REEVAL method item 4 names: a rule correct today because a "
            "construct is unreachable becomes load-bearing the moment a queued card makes it reachable. "
            "DISCHARGE: the cell or the refusal, landed together with whatever card admits the surviving "
            "pending connect."
        ),
        "source": "docs/reports/ct42-protocol-cell/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CT42-CELL-UDP-QUEUE",
        "title": "The decision that a datagram pair is not a cell is recorded so a later card revisits it deliberately",
        "why": (
            "Card CT42-PROTOCOL-CELL (wave 31) §6. Registered not because it is broken but because it is "
            "a DECISION that should be reopened deliberately rather than silently - the project's "
            "standing preference for a recorded boundary over an implicit one. DISCHARGE: a later card "
            "either re-affirms the decision with its ground or lands the datagram cell, in both cases "
            "citing this entry."
        ),
        "source": "docs/reports/ct42-protocol-cell/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CT42-CELL-DERIVED-PREMISE",
        "title": "DeclaredProtocolCellIdentity is not narrowed on the footprint certificate for derived cells",
        "why": (
            "Card CT42-PROTOCOL-CELL (wave 31) §6: narrowing DeclaredProtocolCellIdentity on the "
            "footprint certificate for DERIVED cells needs a deliberate T09 checker edit, which the card "
            "did not take. Constraint 28 already records DeclaredFootprintShape and its siblings as "
            "PREMISES rather than proofs; this entry names the one place a derived cell relies on that "
            "premise. DISCHARGE: the T09 checker edit, or a recorded statement that the derived case "
            "stays a declared premise."
        ),
        "source": "docs/reports/ct42-protocol-cell/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CT42-MAP",
        "title": "CT42's mapping lane - fixture, manifest row and stage evidence on either arm - is not taken",
        "why": (
            "Card CT42-PROTOCOL-CELL (wave 31) §6 named the mapping lane itself as its own follow-on. "
            "Wave 35's CT-CTWIN-LANE re-measured CT42 and MAPPED IT NOT, with BOTH arms measured and the "
            "socket arm found UNREACHABLE FROM SOURCE (CT42-CELL-UNROUTED, registered by that lane): "
            "'This lane measured WHY it is open and did not take it.' So the measurement exists and the "
            "mapping does not. DISCHARGE: a fixture, a manifest row and stage evidence on one of the two "
            "arms - which for the socket arm is blocked behind CT42-CELL-UNROUTED."
        ),
        "source": "docs/reports/ct42-protocol-cell/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    # --- CT-COMPOSED-NATIVE (wave 32) and PROOF-LEG-BATCH (wave 32).
    {
        "id": "CT-COMPOSED-NATIVE-ROWS",
        "title": "The composed-epoch native runner executes, and three stage rows it backs still read unassessed",
        "why": (
            "Card CT-COMPOSED-NATIVE (wave 32), its own follow-on. The runner really compiles and runs "
            "an admitted case's emitted C under the platform sanitizers and byte-compares stdout with "
            "that module's checked task half - and three stage rows it backs still read `unassessed`. A "
            "stage may read `executed` only where a named workspace test runs it on the case's own "
            "artifact (constraint 23), so the gap is between evidence that exists and evidence that is "
            "CREDITED. DISCHARGE: credit the rows in a stage-evidence record, or state why the runner "
            "does not back them."
        ),
        "source": "docs/reports/ct-composed-native/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CT-COMPOSED-NATIVE-SCRIPT",
        "title": "The composed-epoch runner executes the all-success script only; the route's failure edges have no native run",
        "why": (
            "Card CT-COMPOSED-NATIVE (wave 32). The runner executes the ALL-SUCCESS script only; the "
            "composed route's failure edges are CHECKED FACTS WITH NO NATIVE RUN. That is the exact "
            "'proved on one side, observed on the other' shape CLAUDE.md's STABILITY-REEVAL method item "
            "2 asks to be stated in those words. DISCHARGE: a native run of a failure-edge script, "
            "byte-compared against its own checked half, or a recorded statement that the failure edges "
            "stay a checked-only boundary."
        ),
        "source": "docs/reports/ct-composed-native/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CT-COMPOSED-NATIVE-APPLE",
        "title": "The composed-epoch native runner's macOS arm is written and gated but has never been run",
        "why": (
            "Card CT-COMPOSED-NATIVE (wave 32): the Linux arm is the only one measured; the macOS arm is "
            "WRITTEN AND GATED but has not been run, and no claim is made for it. Joins CLAUDE.md's "
            "-APPLE-RUN platform family, whose partial registration is itself the open decision "
            "PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED. DISCHARGE, in the card's own words: a macOS run "
            "of `cargo test -p qcore-compiler` covering the runner's gated tests."
        ),
        "source": "docs/reports/ct-composed-native/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CT-PROOF-CREATION-FAILURE-ALPHABET",
        "title": "A named, measured condition of the proof legs' creation-failure alphabet is open",
        "why": (
            "Card PROOF-LEG-BATCH (wave 32) §named follow-ons. Registered by the wave-35 close because "
            "the proof numerator is the most load-bearing figure the ledger carries - constraint 23 says "
            "a grown proof numerator is evidence about ONE case's artifact - so a named condition on a "
            "proof leg's alphabet must reach the ledger rather than only that card's report. DISCHARGE: "
            "as stated in PROOF-LEG-BATCH's own follow-on entry; see that report, which is authoritative "
            "for the condition's text."
        ),
        "source": "docs/reports/proof-leg-batch/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "CT-PROOF-CONTROL-ARM-TIER",
        "title": "The deliberate version of the proof legs' control-arm tiering was not taken",
        "why": (
            "Card PROOF-LEG-BATCH (wave 32) named this as 'the deliberate version of what this card "
            "did'. Registered by the wave-35 close for the same reason as its sibling: a condition on "
            "how proof legs are controlled belongs in the ledger, not only in the card that noticed it. "
            "DISCHARGE: as stated in PROOF-LEG-BATCH's own follow-on entry, which is authoritative for "
            "the condition's text."
        ),
        "source": "docs/reports/proof-leg-batch/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    # --- Remaining singletons.
    {
        "id": "W31-SIZE-SIBLINGS-REMAINDER",
        "title": "Three callees W29-SIZE-SIBLINGS-UNDECLARED named stay undeclared, each for its own read reason",
        "why": (
            "Successor raised by card W31-CORPUS-CONTRACTS-2 (wave 31), which resolved "
            "W29-CONTENT-SIZE-UNDECLARED outright and resolved W29-SIZE-SIBLINGS-UNDECLARED for ONE "
            "sibling (flb_input_chunk_total_size), narrowing the rest to this entry. The three remaining "
            "callees stay undeclared, EACH FOR ITS OWN READ REASON - so this is not one deferral but "
            "three, and a card taking it must answer them separately. DISCHARGE: a declared .qcore "
            "contract per callee that the unit's own C declaration agrees with (constraint 8), or a "
            "recorded per-callee statement of why the read reason blocks a faithful declaration."
        ),
        "source": "docs/reports/w31-corpus-contracts-2/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "WRITE-LEND-RELEND",
        "title": "There are no transitive loans: lend(LOAN, ...) is not admitted",
        "why": (
            "Card SLICE-PERMISSIONS-WRITE-LEND (wave 29) follow-on table: `lend(LOAN, ...)` - no "
            "transitive loans. The read side already HAS the corresponding shape (a slice of a borrow is "
            "itself a borrow of the same buffer, constraint 15h), so the asymmetry is between a read "
            "loan that narrows and a write loan that cannot. Named as genuine open work reaching no "
            "ledger by card SWEEP-BLIND-SPOTS (wave 35). DISCHARGE: a sub-lend that narrows - the read "
            "card's slice-of-a-borrow shape, WITH AUTHORITY - decided in the lend-extended cycle-3 "
            "ownership IR rather than in a route (constraint 15h forbids moving these rules into a "
            "route), with its own exclusivity statement."
        ),
        "source": "docs/reports/slice-permissions-write-lend/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
    {
        "id": "RECONCILE-SWEEP-DIMENSIONS-UNPINNED",
        "title": "dimensions.* is derived but no test compares the committed values against a regeneration",
        "why": (
            "Card W32-RECONCILE-SWEEP (wave 32) §6: dimensions.* is derived but NO TEST compares the "
            "committed values against a regeneration, which is why that card's §3.1 drift stayed green "
            "for a whole wave. Still true at the wave-35 tip and directly load-bearing for this close, "
            "which moves dimensions.named_followups by 19 and relies on a hand-run generator fixed point "
            "rather than a tripwire to prove it. DISCHARGE: a tripwire in "
            "scripts/tests/test_build_matrix_sections.py that regenerates and compares every "
            "dimensions.* value, so a drifted derived count fails the gate instead of surviving to the "
            "next close."
        ),
        "source": "docs/reports/w32-reconcile-sweep/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    followups = matrix["named_followups"]
    before = len(followups)

    removed = []
    kept = []
    for entry in followups:
        eid = entry.get("id") if isinstance(entry, dict) else None
        if eid in RETIRE:
            removed.append(eid)
        else:
            kept.append(entry)

    present = {e.get("id") for e in kept if isinstance(e, dict)}
    added = []
    for entry in ADD:
        if entry["id"] in present:
            continue
        kept.append(entry)
        added.append(entry["id"])

    matrix["named_followups"] = kept
    matrix["dimensions"]["named_followups"] = len(kept)

    MATRIX.write_text(
        json.dumps(matrix, indent=1, ensure_ascii=False) + "\n"
    )

    print(f"before: {before}")
    print(f"retired ({len(removed)}): {', '.join(sorted(removed))}")
    print(f"added   ({len(added)}): {', '.join(added)}")
    print(f"after:  {len(kept)}")
    for eid in RETIRE:
        if eid not in removed:
            print(f"  NOTE: {eid} was already absent")


if __name__ == "__main__":
    main()
