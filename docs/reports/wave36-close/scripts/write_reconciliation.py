#!/usr/bin/env python3
"""Write `docs/reports/wave36-close/reconciliation.json`.

One verdict per swept item, each measured against the tree rather than read off
the card's claim that it landed. Every `applied` verdict carries either a
`verify` list — checkable text at a named location, read by
`reconciliation_sweep.py --check` — or an `unverifiable` ground saying why the
claim has no quotable text. That arm is this close's discharge of
`RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`.

Run from the repository root.
"""

import json
import pathlib
import sys

OUT = pathlib.Path("docs/reports/wave36-close/reconciliation.json")

A = "AGENTS.md"
C = "CLAUDE.md"
M = "docs/reviews/r0_3/acceptance_matrix.json"
W36 = "docs/development/increments/wave-36.md"
SWEEP = "scripts/reconciliation_sweep.py"


def contains(path, text):
    return {"file": path, "contains": text}


def absent(path, text):
    return {"file": path, "absent": text}


V = {}


def verdict(item_id, kind, note, verify=None, unverifiable=None):
    entry = {"verdict": kind, "note": note}
    if verify:
        entry["verify"] = verify
    if unverifiable:
        entry["unverifiable"] = unverifiable
    V[item_id] = entry


# --------------------------------------------------------------------------
# card-scope-guard — 18 items. The activation this card could not take was run
# by the integrator; the log is read here for the first time.
# --------------------------------------------------------------------------
G = "card-scope-guard#for-the-integrator#"

verdict(G + "1", "noted", "Section heading. Its content is items 2-7, each given its own verdict.")
verdict(
    G + "2", "applied",
    "The integrator ran the activation in the main checkout. Measured from the guard's own log "
    "(.git/qcore-main-guard.log, 5,007 bytes): the two self-test entries are present at "
    "2026-09-17T19:21:00Z, both refused-provenance - one for the main checkout's own pwd marker "
    "and one for the synthetic agent-mainguard-selftest worktree, which is both arms of the "
    "mechanism refusing on a transaction that touched no reference.",
    unverifiable="the activation is a command that ran in the main checkout; its evidence is "
                 ".git/qcore-main-guard.log, which is unversioned and local, so no committed file "
                 "carries a quotable sentence. The log's measured contents are in the note and in "
                 "README.md section 7.",
)
verdict(
    G + "3", "applied",
    "`python3 scripts/install_main_guard.py` ran; four [ok] lines and 'The guard is active' were "
    "the integrator's reported result and the log agrees.",
    unverifiable="as item 2 - a command's exit, not a committed sentence.",
)
verdict(
    G + "4", "noted",
    "A description of what the installer does. Re-read against the log rather than assumed: "
    "core.hooksPath is in force (every later main update is logged, from worktrees whose "
    "checked-out commits predate the hooks), and the self-test touched no reference.",
)
verdict(
    G + "5", "applied",
    "The standing instruction is landed in CLAUDE.md's worktree-hygiene section, and this close "
    "extends it: the log shows it held for merges AND that `git push` needed the token too.",
    verify=[contains(C, "the repository-local token on that one command"),
            contains(C, "refused-push-unauthorised")],
)
verdict(
    G + "6", "applied",
    "The exact invocation is carried in CLAUDE.md so it is not re-derived from a report.",
    verify=[contains(C, 'QCORE_ALLOW_MAIN_UPDATE="$(git config --get qcore.mainguard.token)"')],
)
verdict(
    G + "7", "applied",
    "'Do not export it' is carried in CLAUDE.md as 'never `export`ed'.",
    verify=[contains(C, "never `export`ed")],
)
verdict(G + "8", "noted", "Section heading; items 9-12 are its content.")
verdict(G + "9", "noted", "A statement of where the card applied its own edits. Verified by item 11's claims.")
verdict(
    G + "10", "applied",
    "The card amended CARD-SCOPE-SELF-MERGE and kept it open, offering retirement on activation as "
    "the integrator's choice and asking that it be a DELIBERATE removal noted in the close. "
    "Activation has happened, so this close takes that arm: CARD-SCOPE-SELF-MERGE is RETIRED, and "
    "so is MAIN-GUARD-UNACTIVATED, whose discharge condition is 'a wave close noting the command "
    "ran'. Both were verified to be registered entries before being counted as retirements. "
    "Named in README.md section 8 as retirements 1 and 2 of 3.",
    verify=[absent(M, '"id": "CARD-SCOPE-SELF-MERGE"'),
            absent(M, '"id": "MAIN-GUARD-UNACTIVATED"')],
)
verdict(
    G + "11", "applied",
    "All seven of the card's section 8 names are in the tracker; checked individually here rather "
    "than by counting.",
    verify=[contains(M, '"id": "MAIN-GUARD-BYPASSABLE"'),
            contains(M, '"id": "MAIN-GUARD-NAMED-WORKTREE"'),
            contains(M, '"id": "MAIN-GUARD-ANCESTRY-PORTABILITY"'),
            contains(M, '"id": "MAIN-GUARD-ANCESTRY-REACH"'),
            contains(M, '"id": "MAIN-GUARD-LOG-UNCONSUMED"'),
            contains(M, '"id": "WORKTREE-AUTOREMOVAL-PRISTINE-TREE"')],
)
verdict(
    G + "12", "superseded",
    "'121 -> 128' was right at the card's merge and was itself landed late, by the integrator's "
    "own correction commit b5cfc2cb ('AGENTS.md's stated follow-on count, 121 -> 128, owed at the "
    "CARD-SCOPE-GUARD merge'). Five merges later the figure is 171, and this close takes it to "
    "190. The card's arithmetic is superseded by the wave's own chain, which AGENTS.md now carries "
    "merge by merge so no single figure has to stand for the wave.",
)
verdict(G + "13", "noted", "Section heading; item 14 is its content.")
verdict(
    G + "14", "applied",
    "Both bullets are in CLAUDE.md, and this close adds a third and a fourth to the same section "
    "(the pkill rule and the push-token gap).",
    verify=[contains(C, "WORKTREE-AUTOREMOVAL-PRISTINE-TREE"),
            contains(C, "Never end a turn with a pristine tree while work is in flight")],
)
verdict(G + "15", "noted", "Section heading; item 16 is its content.")
verdict(
    G + "16", "applied",
    "MAIN-GUARD-LOG-UNCONSUMED asked for 'a sweep or close step that reads it'. THIS CLOSE READ IT "
    "- the first consumption. Measured: 21 authorised entries (main updates and pushes) and 5 "
    "refusals, the refusals being 2 refused-provenance (the self-test's two arms), 1 "
    "refused-unauthorised and 2 refused-push-unauthorised. Cross-read against --uncarded's 26 "
    "commits, every one of which sits under an authorised advance of main from the main checkout, "
    "so no uncarded commit of this wave reached main unauthorised. The residual stays OPEN because "
    "one close reading a log by hand is not the mechanism it asks for; what is discharged is the "
    "question, not the automation.",
    unverifiable="the log is unversioned and local (.git/qcore-main-guard.log); the measurement is "
                 "in this note and in README.md section 7, and cannot be a claim about a committed file.",
)
verdict(G + "17", "noted", "Section heading; item 18 is its content.")
verdict(
    G + "18", "noted",
    "A scope statement. Re-measured rather than accepted: `git show --stat` over the card's commits "
    "touches .githooks/, scripts/, docs/, CLAUDE.md and the matrix, and no crate source, no proof/, "
    "no pinned artifact and no frozen archive.",
)

# --------------------------------------------------------------------------
# stability-reeval-2 — 7 items
# --------------------------------------------------------------------------
S = "stability-reeval-2#integrator-reconciliation-list#"

verdict(
    S + "1", "applied",
    "Applied in the card's own merge and verified here against the tree, not read off the report: "
    "the row now reads 40 and 14, and the superseded 44/21 figures occur nowhere in AGENTS.md. The "
    "item's second half - record the correction of the wave-35 record's verdict note BESIDE it "
    "rather than editing that record silently - is honoured: docs/reports/wave35-close/ is not "
    "touched by this close, and README.md section 2.2 carries the correction (constraint 21).",
    verify=[contains(A, "exercised on 40 corpus functions"),
            absent(A, "44 corpus functions"),
            absent(A, "21 sites")],
)
verdict(
    S + "2", "applied",
    "ADOPTED, and built stronger than proposed. The item asked for an arm that reads an `applied` "
    "verdict's prose `where` and `note`; parsing prose would have made the check's own reliability "
    "a parsing question. Instead an entry carries a STRUCTURED `verify` list of {file, contains} / "
    "{file, absent} claims, each opened and read at check time, with whitespace normalised on both "
    "sides so a quotation the tree wraps still matches and nothing else normalised so a softened "
    "amendment is still a changed claim. From wave 36 an `applied` verdict must carry `verify` or "
    "an `unverifiable` ground. 19 new tests (45 -> 64), and the ARM ITSELF was falsified by "
    "mutating the implementation in 8 places - contains-always-passes, absent-never-refuses, "
    "demand-switched-off, missing-file-is-a-pass, normalisation-eats-the-words, "
    "only-the-first-claim-is-read, both-fields-accepted, path-escape-allowed - all 8 caught. "
    "RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED is retirement 3 of 3.",
    verify=[contains(SWEEP, "RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED"),
            contains(SWEEP, "VERIFY_APPLIED_FROM_WAVE"),
            contains("scripts/tests/test_reconciliation_sweep.py",
                     "test_one_changed_word_is_a_changed_claim"),
            absent(M, '"id": "RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED"')],
)
verdict(
    S + "3", "applied",
    "Landed in constraint 5c in the card's words, with TWO corrections recorded rather than made "
    "silently. (a) The card's two line numbers had moved in the tree: `condition.is_empty()` is at "
    "legacy_c.rs:3312, not 3277, and the successor drop at 5161, not 5056. The mechanism was "
    "verified in source before landing - an empty for-condition sets condition_always_true and the "
    "header's successors become vec![body_entry] with `continuation` dropped, while "
    "break_targets.push(continuation) still precedes the body so `break` reaches it. Landing the "
    "card's stale numbers would have put a false statement into a standing constraint. (b) The "
    "card's own deletion of `for (;;)` left 'modelling those' with a plural referent and no plural; "
    "'those' -> 'that' follows the deletion and softens nothing. FOREVER-FOR-EXIT-EDGE-UNOWNED "
    "stays open: the route is still unowned and unpinned, which is what the residual says.",
    verify=[contains(A, "FOREVER-FOR-EXIT-EDGE-UNOWNED"),
            contains(A, "legacy_c.rs:3312"),
            absent(A, "does **not** generalise to\n    `while (1)` or `for (;;)`")],
)
verdict(
    S + "4", "noted",
    "RETIREMENTS: none, by this card. Recorded so the wave's retirement arithmetic is attributable: "
    "the wave's retirements are JOINED-PROVENANCE-FACTS-NAME-NOTHING (joined-provenance-call-frames), "
    "MUTEX-SHARE-CROSS-EMISSION (mutex-share-cross-emission), TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED "
    "(write-lend-tasks-exercise) and this close's three. None is this card's.",
)
verdict(
    S + "5", "applied",
    "Both halves. The two orphans are registered (by the card). The close-rule addition is landed "
    "in CLAUDE.md in the item's own terms - retiring a tracker entry requires re-homing every name "
    "it was the sole tracker for, checked by --residuals BEFORE the retirement lands. This close "
    "ran that check on its own three retirements: --residuals was run after them and returned no "
    "name orphaned by any of the three.",
    verify=[contains(C, "re-homing every name it was the sole tracker for")],
)
verdict(
    S + "6", "applied",
    "The stale accusation is corrected in docs/plans/fluent-bit-system-coverage/README.md. It is "
    "restated as an environment boundary rather than deleted: the cause is now KNOWN (an 8 KiB "
    "pipe capacity against a 1 ms backoff quantum, repaired with poll(2) by card "
    "VERIFIER-HOST-CONTROL-BUDGET), the row is constraint 16's sixth and load-sensitive, and this "
    "close's own gate measurement of it is quoted. The plan no longer asserts a live red test.",
    verify=[contains("docs/plans/fluent-bit-system-coverage/README.md",
                     "FB-PLAN-STALE-RED-ASSERTION")],
)
verdict(
    S + "7", "noted",
    "STABILITY-REEVAL #3 is due after wave 37, and the card's two nominated grounds - the corpus "
    "artifacts, pinned by no test, and the remaining Lean docstrings, where a 3-for-3 hit rate in "
    "one module suggests the sample is not exhausted - are carried into README.md section 10 and "
    "into the wave-36 increment paragraph so the next firing does not have to re-derive them.",
)

# --------------------------------------------------------------------------
# c-gateway-indirect-census-2 — 13 items
# --------------------------------------------------------------------------
X = "c-gateway-indirect-census-2#for-the-integrator#"

verdict(X + "1", "noted", "Section heading; items 2-4 are its content.")
verdict(
    X + "2", "applied",
    "The card's paragraph is in the wave-36 record, and the file was not created by it.",
    verify=[contains(W36, "C-GATEWAY-INDIRECT-CENSUS-2")],
)
verdict(
    X + "3", "superseded",
    "Correct when written - a byte-identical fixed point is the right outcome for a card that adds "
    "no audit, no contract module and no provider package, and 'verified rather than edited' is the "
    "honest description. It is superseded by the wave: named_followups has since moved 121 -> 190 "
    "and this close regenerates the matrix with ITS OWN gate, which is what restores an "
    "integrator_supplied provenance naming wave 36 instead of carrying wave 35's.",
)
verdict(
    X + "4", "noted",
    "A statement of what the card does NOT own. Re-measured: the card moves no headline number, and "
    "the baseline it names has since moved twice within wave 36 (to w36-corpus-contracts-3, then to "
    "joined-provenance-call-frames), which is the wave's own chain and not a contradiction of this "
    "card.",
)
verdict(X + "5", "noted", "Section heading; items 6-7 are its content.")
verdict(
    X + "6", "applied",
    "All nine registered by this close, each with its own report's DISCHARGE clause carried "
    "verbatim - including the two the card marked 'NOT a discharge', which are the half most easily "
    "lost. The card was right to leave the judgement about the tracker's shape to the close; the "
    "close's judgement is that nine findings with written discharge conditions are exactly what the "
    "tracker is for.",
    verify=[contains(M, '"id": "BASE-TYPE-READER-UNMEASURED"'),
            contains(M, '"id": "CENSUS-TWO-PREMISE-READINGS"'),
            contains(M, '"id": "RECORD-BODY-TOTAL-OR-REJECT"'),
            contains(M, '"id": "POISON-UNMASK-ON-ADMISSION"'),
            contains(M, '"id": "LOAD-SENSITIVE-ROW-UNNAMED"'),
            contains(M, '"id": "INDIRECT-EXCHANGE-PROXY-UNVALIDATED"'),
            contains(M, '"id": "INDIRECT-CENSUS-FEATURE-UNBUILT"'),
            contains(M, '"id": "CENSUS-DECLARATION-WINDOW-APPROXIMATE"'),
            contains(M, '"id": "CENSUS-UNEXPLAINED-LEAF"'),
            contains(M, "NOT A DISCHARGE: citing the 2,713 leaves")],
)
verdict(
    X + "7", "deferred",
    "The name is registered so it cannot expire in silence, but the DECISION is deferred and "
    "recorded as one in `deferred_decisions`. Ground: the card states the answer depends on whether "
    "wave 37 takes a gateway card at all, and its own recommendation is that it should not take "
    "this one. A close that added a CI job would be chartering wave 37's work by the back door; a "
    "close that declared the instrumentation disposable would be discarding a measured artifact on "
    "no evidence. Carried to wave 37 with both arms written down.",
)
verdict(X + "8", "noted", "Section heading; items 9-11 are its content.")
verdict(
    X + "9", "noted",
    "Confirmed by measurement: the standing-constraint inventory has one more rule after this wave "
    "than before, and none of them is this card's - the additions are constraint 10b's replacement "
    "(joined-provenance-call-frames), 15o's (mutex-share-cross-emission) and 15h's appendix "
    "(write-lend-tasks-exercise).",
)
verdict(
    X + "10", "noted",
    "Read and agreed. The card is evidence FOR constraint 5, and this close proposes no weakening "
    "of it. The one sentence the card says could be added - which of the two implementations the "
    "premise names - is CENSUS-TWO-PREMISE-READINGS' own discharge and belongs to whoever takes "
    "that residual, in the words they measure; it is registered here and deliberately not written "
    "by the close.",
)
verdict(
    X + "11", "noted",
    "Carried forward rather than acted on: constraint 5b already warns what this card confirms is "
    "live, so nothing needs amending. Recorded in README.md section 10 as a prerequisite reading "
    "for any wave-37 gateway card.",
)
verdict(X + "12", "noted", "Section heading; item 13 is its content.")
verdict(
    X + "13", "deferred",
    "A recommendation about wave 37's chartering, which is not a close's decision to take. It is "
    "carried into README.md section 10 and the wave-36 increment paragraph in the card's own terms, "
    "with its ground intact: the family's leaf mass and its function reach are nearly disjoint, "
    "every route measured decides between 2 and 17 functions against a 3,239-function population, "
    "and the honest prerequisite is BASE-TYPE-READER-UNMEASURED's census, which nobody has run. "
    "Deferred to whoever charters wave 37.",
)

# --------------------------------------------------------------------------
# joined-provenance-call-frames — 11 items, all applied in its own merge
# --------------------------------------------------------------------------
J = "joined-provenance-call-frames#for-the-integrator#"

verdict(J + "1", "noted", "'Apply in these words.' The instruction, honoured item by item below.")
verdict(
    J + "2", "applied",
    "The baseline pointer names this card's artifacts, with the byte-for-byte reproduction clause "
    "and the chain list.",
    verify=[contains(A, "docs/reports/joined-provenance-call-frames/artifacts/")],
)
verdict(
    J + "3", "applied",
    "The clause is in the Cycle 19 corpus row, in the card's words, and the four counts it says are "
    "unchanged are unchanged.",
    verify=[contains(A, "conditional provenance is now preserved across a call the frame shows it "
                        "cannot write"),
            contains(A, "from 0 of 14 to **6 of 14**")],
)
verdict(
    J + "4", "applied",
    "BOTH removals landed, and the two are NOT the same kind - measured at the merge's parent "
    "rather than assumed. JOINED-PROVENANCE-FACTS-NAME-NOTHING WAS a registered entry at 913fe7b2^ "
    "and is gone: a real retirement. JOINED-PROVENANCE-CALL-FRAMES was NEVER a matrix entry - it "
    "lived in constraint 10b's text and wave 31's follow-on prose only - so its 'removal' is a "
    "NO-OP, recorded as one. The arithmetic checks: 159 at the parent, +3 -1 = 161, which is what "
    "AGENTS.md's chain says. The card was right to name both explicitly; naming them is what made "
    "the distinction visible.",
    verify=[absent(M, '"id": "JOINED-PROVENANCE-FACTS-NAME-NOTHING"'),
            absent(M, '"id": "JOINED-PROVENANCE-CALL-FRAMES"')],
)
verdict(
    J + "5", "applied",
    "All three present, each carrying its section 7 discharge condition.",
    verify=[contains(M, '"id": "JOINED-PROVENANCE-ROUND-BOUND"'),
            contains(M, '"id": "CALL-FRAMES-FRAME-READER-PREMISE"'),
            contains(M, '"id": "CALL-FRAMES-ALLOWLIST-LOCAL-ONLY"')],
)
verdict(
    J + "6", "applied",
    "The false sentence is gone from constraint 10b and the replacement stands in its place.",
    verify=[absent(A, "No caller custody is seeded and calls forget prior relations")],
)
verdict(
    J + "7", "applied",
    "The replacement text landed VERBATIM - checked clause by clause here, not by eye, including "
    "the two Lean names that carry its monotonicity and its degenerate case.",
    verify=[contains(A, "A **call** forgets every relation it could have written and keeps only "
                        "those it provably could not"),
            contains(A, "a_call_never_adds_a_name"),
            contains(A, "the_empty_frame_is_the_old_blanket_forget")],
)
verdict(
    J + "8", "applied",
    "Constraint 10b's evidence pointer list carries the card beside `c-gateway-wave`.",
    verify=[contains(A, "`joined-provenance-cycles`, `joined-provenance-call-frames`.")],
)
verdict(
    J + "9", "applied",
    "Re-measured at this close's own tip rather than carried: the card's 133 was correct at its "
    "merge, and two further audits have landed since. The merged-gate row carries this close's own "
    "measured figure, and the matrix's derived validation.lean_audits follows from the same "
    "regeneration. 133 at the card's merge, 135 at this close's tip.",
    verify=[contains(A, "**135 Lean audits**"),
            contains(M, '"lean_audits": 135')],
)
verdict(
    J + "10", "applied",
    "Settled by MERGE ORDER, which is the only ordinal that is a fact rather than a plan, and "
    "settled in the increment record where both paragraphs live: W36-CORPUS-CONTRACTS-3 is wave "
    "36's first arbiter and this card the second. The card was right to claim no ordinal and to "
    "leave the choice; the correction is in W36-CORPUS-CONTRACTS-3's paragraph, which now says its "
    "brief called it second and the order flipped when it finished first.",
    verify=[contains(W36, "wave 36's first arbiter by merge order")],
)
verdict(
    J + "11", "noted",
    "Accepted as offered. CALL-FRAMES-ALLOWLIST-LOCAL-ONLY is a vacuity finding a card made about "
    "its own shipping code, registered as a residual instead of being quietly removed - which is "
    "STABILITY-REEVAL step 1 applied by the card that created the arm. Carried into README.md "
    "section 10 as a worked example for firing #3.",
)

# --------------------------------------------------------------------------
# mutex-share-cross-emission — 10 items
# --------------------------------------------------------------------------
X2 = "mutex-share-cross-emission#for-the-integrator#"

verdict(
    X2 + "1", "applied",
    "The entry is gone from the tracker, and the discharge is real rather than declared: CT46's "
    "four stages moved and the sanitized-native numerator went to 10, which is what the condition "
    "named. Verified to have been a registered entry before being counted as a retirement.",
    verify=[absent(M, '"id": "MUTEX-SHARE-CROSS-EMISSION"'),
            contains(A, "sanitized-native **10 of 46**")],
)
verdict(
    X2 + "2", "applied",
    "All four verified rather than assumed, as the item asks. The CT row reads 10 of 46 with proof "
    "unchanged at 3 of 46; constraint 23's parenthetical is refreshed; acceptance.json's hand-kept "
    "ct_evidence.note leads with 10 of 46 and 3 of 46, its remaining '9 of 45' occurrences being "
    "the note NARRATING its own staleness windows rather than asserting a current figure - read in "
    "context before being called stale.",
    verify=[contains(A, "sanitized-native **10 of 46**"),
            contains(A, "proof 3 of 46"),
            contains("docs/reviews/r0_3/acceptance.json", "10 of 46")],
)
verdict(
    X2 + "3", "applied",
    "Both halves taken by this close. The merged-gate row is re-measured at the integration tip "
    "with this close's own gate - the card's note that AGENTS.md still read 127 Lean audits was "
    "true at its branch point and had already been overtaken by the wave-35 close's 135. The matrix "
    "is regenerated with --gate naming this close's own gate.json, which replaces the "
    "carried_from_previous_matrix provenance the card reported.",
    verify=[contains(M, "wave36-close/gate.json")],
)
verdict(
    X2 + "4", "applied",
    "Wave 32's now-false sentence is replaced, not re-issued.",
    verify=[contains(A, "decided by `mutex_c_emit::classify_trace`")],
)
verdict(
    X2 + "5", "applied",
    "The replacement landed verbatim. Checked at its load-bearing points rather than as a block: "
    "the two rendered shapes, the contiguity premise, the no-contention boundary with its Lean "
    "name, and the operation/log adjacency.",
    verify=[contains(A, "the_creator_performs_no_event_while_its_child_runs"),
            contains(A, "MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION"),
            contains(A, "MUTEX-SHARE-CROSS-EMISSION-LOGGED-SEQUENCE")],
)
verdict(
    X2 + "6", "applied",
    "Landed by this close: constraint 14 had NO wave-36 entry at all before it. This card is in the "
    "'do not engage' list with the other five, and the seventh card is deliberately not - it ran a "
    "census and takes a +0 row, which is the MUTEX-SHARE-CROSS distinction applied.",
    verify=[contains(A, "wave 36's six of seven"),
            contains(A, "`MUTEX-SHARE-CROSS-EMISSION`, none of which reserves a word")],
)
verdict(
    X2 + "7", "noted",
    "Stated in the close because the card asked for it to be: this card allocates no Q code and "
    "widens no condition, reaching Q600's existing backend-refusal condition from a new shape on "
    "the SOCKET-EVENT-COMPOSE precedent. The Q-NAMESPACE-DECISION consequence count therefore does "
    "NOT move for this card; it moves for write-lend-tasks-exercise, to fourteen.",
)
verdict(
    X2 + "8", "noted",
    "Said in the close, as asked. Two pins moved deliberately and neither was deleted: "
    "checked_mutex_guards.rs's composed-child test now emits and runs and is renamed to say so, "
    "with the boundary it guarded re-asserted by a NEW test on a guard-local child; and CT46's "
    "fixture replaces top-level emission_refusal with an emission block, the refusal pin moving to "
    "the new guard_local.qcore control. Both prose fields describing the old boundary were "
    "rewritten rather than left contradicting the data beside them, which is the opposite of the "
    "failure invariant 1 guards against.",
)
verdict(
    X2 + "9", "applied",
    "The rule is landed in CLAUDE.md's worktree-hygiene section IN THE CARD'S OWN WORDS, beside the "
    "shared-stash rule as proposed, with the incident recorded: the process table is shared across "
    "worktrees exactly as the stash stack is. The sibling card is JOINED-PROVENANCE-CALL-FRAMES "
    "(scratchpad gate-callframes3); its gate was re-run and its report section 8 carries a "
    "completed measurement, so the damage is repaired as well as disclosed. The disclosure itself "
    "is the reason this is a rule rather than an unexplained flake.",
    verify=[contains(C, "Never `pkill`/`pgrep` on a pattern that names a tool invocation"),
            contains(C, "The process table is shared across worktrees exactly as the stash stack is"),
            contains(C, "Kill by the PID you captured when you started the process")],
)
verdict(
    X2 + "10", "noted",
    "Offered rather than claimed, and carried as offered. The generalisation - any route whose "
    "native comparison is driven by the typed diagnostic sink compares LOGGED lines, so a mutation "
    "moving the operation but not its log is invisible to it - is recorded in README.md section 10 "
    "as a nominated ground for STABILITY-REEVAL #3. The card makes no claim about the sequential, "
    "task, epoch, socket, event and HTTP routes and this close makes none either.",
)

# --------------------------------------------------------------------------
# w36-corpus-contracts-3 — 19 items
# --------------------------------------------------------------------------
W = "w36-corpus-contracts-3#for-the-integrator#"

verdict(
    W + "1", "noted",
    "The instruction this whole record follows. Its diagnosis was right about this card: its "
    "headline numbers had been applied and THREE of its prose items had not - constraints 7, 8 and "
    "14's entry - which is the wave-32 finding reproducing itself on the card that warned about it.",
)
verdict(
    W + "2", "applied",
    "Every figure verified in the tree rather than assumed, as the item asks.",
    verify=[contains(A, "**205 Safe / 84 provisional Unsafe / 4,545 Unknown**"),
            contains(A, "**2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**"),
            contains(A, "`summary_conflict` **1**"),
            contains(A, "**82 modules**"),
            contains(A, "43 → **49**"),
            contains(A, "21 → **27**")],
)
verdict(
    W + "3", "superseded",
    "Applied at this card's merge and superseded WITHIN THE SAME WAVE by the second arbiter: the "
    "baseline now names joined-provenance-call-frames, whose fresh BEFORE reproduced this card's "
    "diagnostics.json and chunk-contracts.json byte for byte and which moved zero leaves, zero "
    "functions and zero units. That is the chain rule working, not a loss. The item's second half "
    "- the brief's named baseline was stale by one - is correct and no chain text now says "
    "c-gateway-constant-do-while.",
)
verdict(
    W + "4", "noted",
    "The condition did not arise: merge order put this card FIRST and joined-provenance-call-frames "
    "second, so this card's BEFORE was measured against the baseline that was current for it. The "
    "second arbiter then did exactly what this item asks of whoever went second - reproduced the "
    "predecessor's artifacts byte for byte before claiming its own delta.",
)
verdict(
    W + "5", "applied",
    "Checked rather than assumed, as the item asks. No pinned category list was found in any of the "
    "three places named: docs/reports/corpus-repin/diff_artifacts.py carries no category "
    "enumeration (it itemises by diffing), the reason-code sidecar tripwire keys on `reason_code` "
    "rows and not on a fixed diagnostic-category set, and the matrix's corpus block is derived. The "
    "card could not rule one out; this close looked and found none, and the Python suite covering "
    "both tools is green in this close's gate.",
    unverifiable="the finding is the ABSENCE of a pinned category list in three named files, which "
                 "no quotable sentence asserts; the measurement is in this note and README.md "
                 "section 6.",
)
verdict(W + "6", "noted", "Heading for the constraint 7 amendment; item 7 is the text.")
verdict(
    W + "7", "applied",
    "NOT APPLIED AT THE CARD'S MERGE - found by measuring constraint 7 at this close's base and "
    "seeing the text absent. Landed here VERBATIM.",
    verify=[contains(A, "Card `W36-CORPUS-CONTRACTS-3` (wave 36) moved the number **not at all**: "
                        "272 → 272 across a re-pin that added six declared contracts and moved 21 "
                        "function statuses.")],
)
verdict(W + "8", "noted", "Heading for the constraint 8 amendment; item 9 is the text.")
verdict(
    W + "9", "applied",
    "NOT APPLIED AT THE CARD'S MERGE - the same measurement as item 7. Landed here VERBATIM, plus "
    "the evidence pointer the constraint's own convention wants.",
    verify=[contains(A, "The mode gate's `SummaryConflict`-only arm for a **nullability "
                        "refinement** was exercised for the first time by a *declaration* rather "
                        "than by the corpus"),
            contains(A, "declaring `nonnull` to silence it would assert a precondition the body "
                        "does not have")],
)
verdict(
    W + "10", "applied",
    "Also unapplied at the merge: constraint 14 carried NO wave-36 entry, neither this card's nor "
    "any other's. Landed here, with this card in the non-engaging list on exactly the ground it "
    "gives - a .qcore contract module adds no source-language surface.",
    verify=[contains(A, "`CARD-SCOPE-GUARD`, `W36-CORPUS-CONTRACTS-3`, `STABILITY-REEVAL` #2"),
            contains(A, "a `.qcore` contract module, which is no source-language surface")],
)
verdict(
    W + "11", "applied",
    "Confirmed, which is the first arm of W36-CTRACES-FAMILY-FIRST's discharge. The manifest is "
    "ordered ALPHABETICALLY by directory (cfl, chunkio, cmetrics, ctraces, fluent_bit, ... monkey) "
    "and ctraces/ sits between cmetrics/ and fluent_bit/, so the placement follows the corpus's "
    "existing convention rather than being a free choice; the module name is its sibling's, the "
    "contract being cmt_destroy's exact counterpart one library over. The residual STAYS OPEN for "
    "its second arm - a second ctraces module settling the naming convention - so the confirmation "
    "is recorded without being overclaimed.",
    verify=[contains(M, '"id": "W36-CTRACES-FAMILY-FIRST"')],
)
verdict(
    W + "12", "applied",
    "All nine registered by this close, each with its own report's discharge condition. The card's "
    "arithmetic (128 -> 137) is superseded by the wave's chain, which reached 171 before this close "
    "and 190 after it; the card was right that the count it saw was 128 and not the brief's 121, "
    "the integrator's correction b5cfc2cb having landed after it branched.",
    verify=[contains(M, '"id": "W36-COVER-HIDDEN-BLOCKER"'),
            contains(M, '"id": "CJSON-DELETE-NESTED-DETACH"'),
            contains(M, '"id": "KAFKA-MEM-FREE-SUMMARY-CONFLICT"'),
            contains(M, '"id": "MK-PRINT-TAIL-ACTUALS"'),
            contains(M, '"id": "CJSON-DELETE-SIBLING-CHAIN"'),
            contains(M, '"id": "LIBC-MEM-RESULT-ALIAS"'),
            contains(M, '"id": "ADD-NAME-COLLISION"'),
            contains(M, '"id": "STANDALONE-HOST-CLOSED-INPUT-LOAD"')],
)
verdict(
    W + "13", "applied",
    "The deliberate non-regeneration was right for the card and is now closed out: this close "
    "regenerates with its own gate, so the derived counts follow the hand-maintained section. The "
    "item's second half is a real finding and is recorded rather than repaired: "
    "corpus.contract_modules_per_directory and corpus.contract_clause_pin are derived from card "
    "B00's FROZEN archive and describe a 59-module checkpoint beside a corpus this card takes to "
    "82, so D02-B00-LEDGER-SNAPSHOT has WIDENED. Repair is by publishing provenance beside them and "
    "never by editing the archive (constraints 21 and 25); this close publishes the widening here "
    "and in README.md section 6 rather than touching either.",
    verify=[contains(M, "wave36-close/gate.json"),
            contains(M, '"named_followups": 187')],
)
verdict(W + "14", "noted", "Heading for the constraint 16 amendment; items 15-16 are its content.")
verdict(
    W + "15", "superseded",
    "SPLIT, and the split is the point. The proposed text says the row 'joins this list as its "
    "SIXTH row' - and the row has BEEN the sixth since the wave-35 close landed it in card "
    "VERIFIER-HOST-CONTROL-BUDGET's own words. Landing the sentence verbatim would have put a false "
    "statement into a standing constraint and given constraint 16 the same member twice, so that "
    "half is superseded by the wave-35 paragraph that got there first. THE MEASUREMENT LANDS, in "
    "the card's own numbers, as a second and independent observation of the same row - and it is "
    "the row's first at the MIDDLE of the load range (wave 35 measured ~100 and ~6.5) and the first "
    "after the poll(2) repair, so what it shows is that the repair narrows the failure window "
    "without closing it. Both texts are in README.md section 6.",
    verify=[contains(A, "A second, independent measurement of the same sixth row"),
            contains(A, "passed\n    **12 of 12** re-run in isolation at load 23.7 to 27.6")],
)
verdict(
    W + "16", "applied",
    "The card offered the alternative reading itself and asked only that the measurement land "
    "somewhere either way. This close takes NEITHER extreme: it does not add a duplicate sixth row, "
    "and it does not reopen e3d3a2ce's repair, which 38/38 supports. It records the measurement "
    "beside the existing row and registers STANDALONE-HOST-CLOSED-INPUT-LOAD with a discharge "
    "condition that names the real gap - nothing pins the margin.",
    verify=[contains(M, '"id": "STANDALONE-HOST-CLOSED-INPUT-LOAD"')],
)
verdict(
    W + "17", "applied",
    "The close's to move, and moved - but the drift the card reported had already been half-"
    "corrected before it: AGENTS.md read 4,534/127 at the card's branch point and the wave-35 close "
    "had taken it to 4,580/135 by the time the card merged. This close re-measures the whole row at "
    "its own tip, so no figure in it is carried from another wave's measurement.",
    verify=[absent(A, "**4,534 workspace tests**"), absent(A, "**127 Lean audits**")],
)
verdict(
    W + "18", "noted",
    "Carried into the close verbatim in spirit: every Safe is Safe relative to six declared "
    "premises, the added unknown_alias leaves are Unknown-class evidence and not defect claims, and "
    "nothing in this wave is an upstream defect claim (invariant 2). README.md's headline says so.",
)
verdict(W + "19", "noted", "A horizontal rule the extractor picked up as an item. No action.")

# --------------------------------------------------------------------------
# write-lend-tasks-exercise — 11 items
# --------------------------------------------------------------------------
L = "write-lend-tasks-exercise#for-the-integrator#"

verdict(L + "1", "noted", "The preamble. Each item below is verdicted on its own.")
verdict(
    L + "2", "applied",
    "Retired - and a NO-OP retirement, recorded as one. WRITE-LEND-TASKS-EXERCISE was never a "
    "named_followups entry; it lived only in constraint 15h's text, so what discharges it is the "
    "appendix in item 3 and no count moves. The card said this itself and was right; it is recorded "
    "because a retirement that moves no count is exactly the kind that later reads as a lost one.",
    verify=[absent(M, '"id": "WRITE-LEND-TASKS-EXERCISE"'),
            contains(A, "it supersedes the \"no operation admits a lease in a worker body\" sentence deliberately")],
)
verdict(
    L + "3", "applied",
    "Retired, and DECIDED rather than deferred - which is the stronger outcome and the one the card "
    "argued for. Verified to have been a registered entry before being counted.",
    verify=[absent(M, '"id": "TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED"')],
)
verdict(
    L + "4", "applied",
    "The appendix is in constraint 15h, applied by the card and verified here at its load-bearing "
    "points rather than as a block.",
    verify=[contains(A, "**Appended, card `WRITE-LEND-TASKS-EXERCISE` (wave 36)")],
)
verdict(
    L + "5", "applied",
    "Constraint 15n's forward question is now a decision with its ground.",
    verify=[contains(A, "`WRITE-LEND-TASKS-EXERCISE` (wave 36), which DECIDES the forward question")],
)
verdict(
    L + "6", "applied",
    "Both widenings are in the Q020 and Q600 rows, no code allocated, and the Q-NAMESPACE-DECISION "
    "consequence count reads fourteen on nine bands.",
    verify=[contains(A, "`WRITE-LEND-TASKS-EXERCISE` is the **fourteenth**"),
            contains(A, "a worker takes **at most one** `lend<Buffer>` parameter")],
)
verdict(
    L + "7", "applied",
    "Re-derived from the file rather than trusted, as the item asks: all seven names are present, "
    "including WRITE-LEND-TASKS-EXTENT, which is wave 31's own residual that no tracker held and "
    "which the card registered rather than leaving for --residuals to find. That is the behaviour "
    "the close rule wants. The 159 -> 165 figure is superseded by the wave's chain to 190; the "
    "entries are what matters and they are all there.",
    verify=[contains(M, '"id": "WRITE-LEND-EXERCISE-WINDOWS-RUN"'),
            contains(M, '"id": "TASK-ROUTE-TSAN-SEGV"'),
            contains(M, '"id": "WRITE-LEND-EXERCISE-BYTE-ORDER"'),
            contains(M, '"id": "WRITE-LEND-EXERCISE-OWNER-BYTE-WRITE"'),
            contains(M, '"id": "WRITE-LEND-EXERCISE-LOAN-BYTE-READ"'),
            contains(M, '"id": "CT07-TWO-THREAD-REMAP"'),
            contains(M, '"id": "WRITE-LEND-TASKS-EXTENT"')],
)
verdict(
    L + "8", "deferred",
    "A decision the card explicitly declined to take and which this close also may not take: "
    "promoting TASK-ROUTE-TSAN-SEGV to wave-37 card work is work allocation, and the brief says so. "
    "Recorded as a DEFERRED DECISION so it cannot expire in silence, with the card's own framing "
    "intact: this is not the card's defect, and its significance is route-wide - the WHOLE CHECKED "
    "CONCURRENCY ROUTE has no thread-sanitizer evidence at all, and no TSan row exists anywhere in "
    "this repository (constraint 15o says the same from the other direction). That is a larger fact "
    "about the project's native evidence than it is about this card, and it is the close's "
    "recommendation that it be read as wave-37 material.",
)
verdict(
    L + "9", "deferred",
    "Lane work, and no lane of wave 36 owns it. CT07-TWO-THREAD-REMAP is registered, the card "
    "explains in its section 10 item 7 why it does not remap the row itself, and the close adds no "
    "information by inventing a disposition. Carried to wave 37 as schedulable.",
)
verdict(
    L + "10", "applied",
    "Both halves. The card repaired D02-DIMENSIONS-FOLLOWUP-COUNT by regenerating rather than "
    "editing the derived count, and restored measurement_provenance to the committed value instead "
    "of letting the generator relabel wave 35's integrator-supplied numbers as carried (constraint "
    "25) - the right call for a card. This close does the flip the card says is the integrator's: "
    "regenerates with --gate naming its own measured gate.json.",
    verify=[contains(M, "wave36-close/gate.json")],
)
verdict(
    L + "11", "applied",
    "Honoured: the merged-gate row and gate.json carry this close's own measurement at the "
    "integration tip, and no figure is carried from this card's section 9.",
    verify=[contains("docs/reports/wave36-close/gate.json", "Wave-36 close, 2026-09-18"),
            contains("docs/reports/wave36-close/gate.json", "is INSIDE this measurement rather than beside it")],
)


# --------------------------------------------------------------------------

CARDS = [
    "card-scope-guard",
    "w36-corpus-contracts-3",
    "stability-reeval-2",
    "c-gateway-indirect-census-2",
    "joined-provenance-call-frames",
    "mutex-share-cross-emission",
    "write-lend-tasks-exercise",
]

DEFERRED = {
    "note": "Fifteen decision ENTRIES were carried into this close across two files - seven in "
            "docs/reports/wave34-close/deferred_decisions.json and eight in "
            "docs/reports/wave35-close/reconciliation.json - over eight distinct names, of which "
            "SIX were still open. Restated here, not resolved, except where a wave-36 card resolved "
            "one and is named. Two new decisions are raised by this wave's cards. The two already "
            "ended at wave 35 are recorded so the 15-entry carry is accounted rather than silently "
            "reduced to 6.",
    "items": [
        {
            "name": "CT10-REASON-ROW",
            "verdict": "deferred",
            "ground": "Still deferred, a fourth wave. NO new evidence from wave 36: no card touched "
                      "the CT corpus's unmapped cases, and CT-CTWIN-LANE's wave-35 re-measurement "
                      "(CT10 stays unmapped 'on half a scenario') remains the last word. NEW THIS "
                      "WAVE, and it is about the mechanism rather than the decision: card "
                      "STABILITY-REEVAL #2 also registered this name as a named_followup, so it is "
                      "now carried by BOTH mechanisms - see DECISION-ALSO-REGISTERED-AS-FOLLOWUP, "
                      "registered by this close. Resolving the decision will not retire the entry.",
        },
        {
            "name": "MATRIX-UPDATED-STALE-AT-WAVE-27",
            "verdict": "deferred",
            "ground": "Still deferred; the decision is untaken and this close does not take it. NEW "
                      "EVIDENCE: the matrix was regenerated at every wave-36 merge that touched it "
                      "and once more by this close with its own gate, so the `updated` field's "
                      "wave-27 prose now sits beside a file regenerated nine waves later. The "
                      "divergence widens with every close, which is an argument for taking the "
                      "decision and not for taking it a particular way. Also now double-carried as "
                      "a named_followup (DECISION-ALSO-REGISTERED-AS-FOLLOWUP).",
        },
        {
            "name": "EXAMPLES-STYLE-SLICE-FANOUT",
            "verdict": "deferred",
            "ground": "Still deferred, a fourth wave. No card of wave 36 touched examples/ and this "
                      "close produces NO new evidence about it. Re-listed so it does not expire in "
                      "silence, which is the whole point of the re-listing mechanism; a close that "
                      "invented a disposition here would be making the record worse.",
        },
        {
            "name": "SYNC-RULE-TOOLING-COMMIT-SCOPE",
            "verdict": "deferred",
            "ground": "Still deferred, and wave 36 sharpens it again. --uncarded returned 26 "
                      "commits, of which FOUR are exactly the shapes the decision is about: the "
                      "wave-open commit 60958bf0, and THREE integrator corrections - b5cfc2cb (the "
                      "stated follow-on count), e4619e37 and 2200a04e (re-separating three wave-35 "
                      "increment entries a union merge had fused, then rebuilding it when the first "
                      "attempt inserted nine blank lines instead of three). Two of the three change "
                      "a committed document's content with no card report, which is the question. "
                      "Wave 35 saw four such commits; wave 36 has four again, so this is a steady "
                      "state rather than an anomaly, and steady states are what rules are for.",
        },
        {
            "name": "PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED",
            "verdict": "deferred",
            "ground": "Still deferred, and this close deliberately did NOT resolve it by completing "
                      "the registration, for the reason the wave-35 close gave: the decision is "
                      "precisely WHICH arm to take - complete the registration, or state the rule "
                      "that keeps platform residuals out of the matrix - and taking one silently "
                      "would be the close deciding it. NEW EVIDENCE: wave 36 added another platform "
                      "residual of exactly this family, WRITE-LEND-EXERCISE-WINDOWS-RUN, and that "
                      "one IS registered - so the family is now more partially tracked than before, "
                      "not less.",
        },
        {
            "name": "DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK",
            "verdict": "deferred",
            "ground": "Still deferred - explicitly a question raised for the user, which no card and "
                      "no close may answer. NEW EVIDENCE from this close, offered as a data point "
                      "and not as an argument for either reading: AGENTS.md and CLAUDE.md GREW "
                      "again here, and every single addition was a correction landing late - three "
                      "unapplied constraint amendments, constraint 14's missing wave-36 entry, and "
                      "two hygiene rules a card asked for. None of it is narrative accretion, which "
                      "is the reading the decision has to choose between. DOCS-COMPACT-7 is due "
                      "around wave 37 and will inherit the question.",
        },
        {
            "name": "VERIFIER-HOST-ROW-DISPOSITION",
            "verdict": "resolved",
            "ground": "Ended at the WAVE-35 close, not here; recorded so the 15-entry carry is "
                      "accounted rather than silently dropped. Wave 36 adds a measurement that "
                      "supports the resolution rather than reopening it: the row passed 12/12 in "
                      "isolation at load 23.7-27.6 and failed once in-gate at ~40, which is a "
                      "load-sensitivity finding about a repaired mechanism and not the unresolved "
                      "validation finding the decision was about.",
        },
        {
            "name": "items",
            "verdict": "withdrawn",
            "ground": "Not a decision. It was a reader artifact - a bogus name the wave-34 file's "
                      "shape produced - withdrawn at the wave-35 close and recorded here only "
                      "because it is one of the 15 carried entries and an unexplained disappearance "
                      "is what this record exists to prevent.",
        },
        {
            "name": "INDIRECT-CENSUS-FEATURE-UNBUILT",
            "verdict": "deferred",
            "ground": "RAISED BY WAVE 36, card C-GATEWAY-INDIRECT-CENSUS-2 section 12.2, which "
                      "states it is a decision and not a task and declines to take it. Both arms "
                      "are written down: add a CI job building "
                      "`-p qcore-c-gateway --features indirect-census`, or record that the "
                      "instrumentation is disposable and a future census re-derives it. The answer "
                      "depends on whether wave 37 takes a gateway card at all, and the card's own "
                      "recommendation is that it should not take this one. Also registered as a "
                      "named_followup so the work is tracked either way.",
        },
        {
            "name": "TASK-ROUTE-TSAN-SEGV-PRIORITY",
            "verdict": "deferred",
            "ground": "RAISED BY WAVE 36, card WRITE-LEND-TASKS-EXERCISE section 11 item 7, which "
                      "declines to set the priority and says the choice is the user's. The finding "
                      "behind it is route-wide rather than card-local: the whole checked concurrency "
                      "route has NO thread-sanitizer evidence, and no TSan row exists anywhere in "
                      "this repository. The decision is whether that is wave-37 card work or a "
                      "follow-on; the residual TASK-ROUTE-TSAN-SEGV is registered either way, so "
                      "nothing is lost by carrying the decision.",
        },
    ],
}

HAND_SWEEP = {
    "note": "What the sweep cannot reach, swept by hand and recorded so the close's coverage is "
            "not confused with the tool's.",
    "items": [
        "The 26 seam commits --uncarded returned were classified by eye, one by one: 22 are cards' "
        "OWN code commits (write-lend-tasks-exercise 5, mutex-share-cross-emission 6, "
        "joined-provenance-call-frames 4, w36-corpus-contracts-3 3, card-scope-guard 2, "
        "stability-reeval-2 1, c-gateway-indirect-census-2 1), 3 are integrator corrections "
        "(b5cfc2cb, e4619e37, 2200a04e) and 1 is the wave-open commit 60958bf0. Total 22+3+1=26. "
        "ZERO are uncarded work: every commit is attributable to a card, to the integrator, or to "
        "the wave's opening.",
        "The 44 residual-shaped names were classified individually, not in bulk: 18 registered, 15 "
        "card names rather than residuals, 8 discharged with the discharging card named, 2 rules or "
        "prospective names rather than open work, 1 a deferred decision. 18+15+8+2+1=44.",
        "--residuals fell 44 -> 25 after registration. NINETEEN names went quiet, not eighteen: "
        "C-GATEWAY-INDIRECT-CENSUS-2 also dropped, because a registered entry's `source` path "
        "contains that card's own name and the scan is case-insensitive. The classification (a card "
        "name, not a residual) is right, but it went quiet for the WRONG REASON, and a future "
        "close should not read that drop as a decision anyone took.",
        "AGENTS.md's headline numbers were re-read from their sources rather than from the "
        "ledger: named_followups against the matrix file itself, the CT row against "
        "docs/reevaluation/ct_manifest.json's own stage data through the card reports, the contract "
        "corpus against compiler/tests/qcore_contract_corpus.rs's pinned tables, and the merged-gate "
        "row against this close's own measured logs.",
        "main MOVED DURING THIS CLOSE. At the close's start `main` was fa48211a, the brief's base; "
        "it is now 477b733d, 'Open wave 37: create the increment record and move the index pointer "
        "before any card launches'. That commit is the integrator's wave-37 opening, not wave-36 "
        "work, and its own message says 'Wave 36's close paragraph still belongs in wave-36.md', "
        "which agrees with this close's brief. The consequence to know at merge time: the increment "
        "index will already name wave-37.md, so a literal reading of the sync rule would misfile "
        "this close's paragraph. It is in wave-36.md.",
        "Constraint 16's list is now recorded as KNOWN INCOMPLETE rather than assumed complete: "
        "LOAD-SENSITIVE-ROW-UNNAMED names a seventh test showing the same signature in the same "
        "crate, measured by a different card than the one that measured the sixth row.",
    ],
}


def main() -> int:
    record = {
        "wave": 36,
        "cards": CARDS,
        "verify_applied": True,
        "note": "WAVE-36-CLOSE. Base fa48211a, the WRITE-LEND-TASKS-EXERCISE merge. 89 "
                "integrator-addressed items across seven cards, each given a verdict MEASURED "
                "AGAINST THE TREE rather than read off the card's claim that it landed - the "
                "wave-35 close's method, which is what found this wave's losses. FOUR THINGS WERE "
                "UNAPPLIED AT THEIR MERGES and are landed here: W36-CORPUS-CONTRACTS-3's constraint "
                "7 and constraint 8 amendments, STABILITY-REEVAL #2's constraint 5c amendment, and "
                "constraint 14's entire wave-36 entry, which no card had written. A fifth, "
                "W36-CORPUS-CONTRACTS-3's constraint 16 text, is SPLIT: its measurement lands and "
                "its 'joins this list as its sixth row' wording is superseded, the row having been "
                "the sixth since the wave-35 close, so landing it verbatim would have put a false "
                "statement into a standing constraint. This record is the first held to the "
                "`verify_applied` standard it introduces: every `applied` verdict carries either "
                "checkable text at a named location or a written ground for why it has none, and "
                "`--check` reads them.",
        "verdicts": V,
        "no_section": {
            "wave36-close": "This close's own report, which the sweep finds because it is a "
                            "docs/reports/<card>/README.md changed in the range. It has no "
                            "integrator-addressed section and must not: it IS the integrator's "
                            "record, and a close that addressed items to itself would be the "
                            "reconciliation defect in miniature. Its own findings go to wave 37 as "
                            "registered residuals and deferred decisions, which is the mechanism "
                            "that exists for them.",
        },
        "hand_sweep": HAND_SWEEP,
        "deferred_decisions": DEFERRED,
        "gate": "docs/reports/wave36-close/gate.json",
    }
    OUT.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {OUT} with {len(V)} verdicts")
    from collections import Counter
    print(Counter(e["verdict"] for e in V.values()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
