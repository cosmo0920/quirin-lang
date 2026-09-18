#!/usr/bin/env python3
"""WAVE-37-CLOSE: build docs/reports/wave37-close/reconciliation.json.

Every verdict below is a human judgement measured against the tree at the
close's base (`c33c7640`, plus this close's own commits), never read off a
card's claim that something landed. The record is checked by

    python3 scripts/reconciliation_sweep.py . --since 0f6eb0a6 \
        --check docs/reports/wave37-close/reconciliation.json

which verifies that every swept item carries a verdict and that every `applied`
verdict's quoted text is at the named location. The tool decides nothing.
"""
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent.parent
MATRIX = "docs/reviews/r0_3/acceptance_matrix.json"
AGENTS = "AGENTS.md"
W37 = "docs/development/increments/wave-37.md"
GEN = "docs/reports/remaining-d02/build_matrix.py"
GATE = "docs/reports/wave37-close/gate.json"


def c(file, text):
    return {"file": file, "contains": text}


def a(file, text):
    return {"file": file, "absent": text}


V = {}


def verdict(item, kind, note, verify=None, unverifiable=None):
    entry = {"verdict": kind, "note": note}
    if verify:
        entry["verify"] = verify
    if unverifiable:
        entry["unverifiable"] = unverifiable
    assert item not in V, item
    V[item] = entry


# ----------------------------------------------------------------- base-type-reader-census
B = "base-type-reader-census#for-the-integrator#"
verdict(B + "1", "noted", "Section heading (11.1 Applied by this card); its content is items 2-4.")
verdict(
    B + "2",
    "applied",
    "The card's paragraph is in wave-37.md, prepended in the file's newest-first order at the "
    "time; its head already had the wave-36 shape and was not touched by the normalisation.",
    [c(W37, "**`BASE-TYPE-READER-CENSUS` (2026-09-18).** Wave 37's first measurement card")],
)
verdict(
    B + "3",
    "applied",
    "Five of the six names are registered, each verified by id at c33c7640: "
    "TYPEDEF-ANONYMOUS-AGGREGATE-UNREACHED, BASE-CAUSE-SECOND-DISCOUNT-PARTIAL, "
    "BASE-TYPE-CENSUS-FILE-SCOPE-APPROXIMATE, W36-CENSUS-FIGURES-PRE-CONTRACT-CARD and "
    "BASE-TYPE-READER-CENSUS-FEATURE-UNBUILT. The sixth, BASE-TYPE-READER-UNMEASURED, had "
    "ALREADY been registered by the wave-36 close (0f6eb0a6, merged before this card) and the "
    "merge c1814e3c dedup'd the card's copy - which is why the count line read 192 (187+5) and "
    "not 193. The card says the name is DISCHARGED and registered it 'with its discharge'; a "
    "tracker entry whose discharge condition is met word for word (instrument the base-type "
    "reader, publish the sole-blocked reach per cause, before any card is cut on it) is a "
    "retirement, so this close RETIRES BASE-TYPE-READER-UNMEASURED, named and checked "
    "registered first (README section 8).",
    [
        c(MATRIX, '"id": "TYPEDEF-ANONYMOUS-AGGREGATE-UNREACHED"'),
        c(MATRIX, '"id": "BASE-TYPE-READER-CENSUS-FEATURE-UNBUILT"'),
        a(MATRIX, '"id": "BASE-TYPE-READER-UNMEASURED"'),
    ],
)
verdict(
    B + "4",
    "noted",
    "Verified: the card moved no headline number, added nothing to constraint 21's frozen "
    "list and left the baseline pointer alone (it moved later in the wave, to "
    "w37-corpus-contracts-4, by that card's own item 2).",
)
verdict(B + "5", "noted", "Section heading (11.2 Not applied); its content is items 6-7.")
verdict(
    B + "6",
    "superseded",
    "True at the card's base 477b733d and false at c33c7640: the wave-36 close (merged as "
    "0f6eb0a6, before this card) registered all nine of C-GATEWAY-INDIRECT-CENSUS-2's names. "
    "Re-checked by id here, not by eye: CENSUS-TWO-PREMISE-READINGS, "
    "POISON-UNMASK-ON-ADMISSION, INDIRECT-EXCHANGE-PROXY-UNVALIDATED, CENSUS-UNEXPLAINED-LEAF, "
    "CENSUS-DECLARATION-WINDOW-APPROXIMATE, RECORD-BODY-TOTAL-OR-REJECT, "
    "INDIRECT-CENSUS-FEATURE-UNBUILT and LOAD-SENSITIVE-ROW-UNNAMED are all present; the ninth, "
    "BASE-TYPE-READER-UNMEASURED, is retired by this close (item 3).",
    [c(MATRIX, '"id": "CENSUS-TWO-PREMISE-READINGS"'), c(MATRIX, '"id": "LOAD-SENSITIVE-ROW-UNNAMED"')],
)
verdict(
    B + "7",
    "applied",
    "The entry's `why` now carries the card's second measured value in the card's words (five "
    "functions beside Rank 1, against two alone). The DISPOSITION - declined as a standalone "
    "arbiter - is a wave-38 cut decision and is deliberately not changed by a close.",
    [c(MATRIX, "five more functions, because 41.6% of Rank 1's leaves land on a record body refused whole")],
)
verdict(B + "8", "noted", "Section heading (11.3); its content is items 9-11.")
verdict(B + "9", "noted", "The card lands no standing rule; none was landed for it. Constraints 5 and 5b are byte-identical across the wave (DOCS-COMPACT-7 hashed all 49).")
verdict(B + "10", "noted", "Constraint 5b is read as confirmed and sized (12 functions, 17 with Rank 2); no amendment proposed, none made.")
verdict(B + "11", "noted", "Constraint 5 is read as evidence for itself; no amendment proposed, none made.")

# ----------------------------------------------------------------------------- ct-lane-5
verdict(
    "ct-lane-5#retirement-this-card-earns-for-the-integrator#1",
    "applied",
    "CT07-TWO-THREAD-REMAP retired at the merge e88d9917 (the card left the retirement to the "
    "integrator; both halves of wave 36's discharge condition are met in the card's section 8.2). "
    "Verified absent from named_followups at c33c7640.",
    [a(MATRIX, '"id": "CT07-TWO-THREAD-REMAP"')],
)
L = "ct-lane-5#integrator-reconciliation-list#"
verdict(L + "1", "noted", "Preamble to the list.")
verdict(
    L + "2",
    "applied",
    "As the 8.2 entry: retired at e88d9917, the name verified registered before and absent after.",
    [a(MATRIX, '"id": "CT07-TWO-THREAD-REMAP"')],
)
verdict(
    L + "3",
    "applied",
    "The suggested sentence landed in the AGENTS.md CT row at the merge e88d9917 in the card's "
    "words, and was then MOVED VERBATIM by DOCS-COMPACT-7 into wave-37.md's checkpoint section "
    "3 (the CT row as it stood at 2caf7369) when that card compacted the row's per-lane "
    "chronicle - compaction, not deletion. The row itself keeps the numbers the item says do not "
    "move.",
    [
        c(W37, "re-mapped **CT07** off the sequential write lend onto the two-thread range write lease"),
        c(AGENTS, "**46/50 mapped**, 4 unmapped; sanitized-native **10 of 46**; **proof 3 of 46**"),
    ],
)
verdict(L + "4", "noted", "Constraint 23: no amendment proposed, none made; the card is an instance of the rule.")
verdict(
    L + "5",
    "applied",
    "Constraint 14's enumeration brought current for waves 36 and 37 by this close, CT-LANE-5 "
    "listed with its section cited, and the whole seam measured (git diff over the front-end "
    "crate is empty).",
    [c(AGENTS, "`CT-LANE-5` §11 item 4")],
)
verdict(L + "6", "noted", "Q-namespace: nothing allocated, no condition widened; the Q-space note's tally is untouched by this wave.")
verdict(
    L + "7",
    "applied",
    "The three residuals were registered at the merge (with CT-FIXTURE-EXTERNAL-READERS, four); "
    "verified by id.",
    [c(MATRIX, '"id": "CT07-TWO-THREAD-PROOF-LEG"'), c(MATRIX, '"id": "CT10-BYTE-ENCODING-UNSOUND"')],
)
verdict(
    L + "8",
    "deferred",
    "A decision for the user, not the close: closing CT07-TWO-THREAD-REPLAY-ORDER 'by "
    "statement' puts a sentence in the ledger that a later reader of CT07's byte-identical "
    "comparison will rely on, and the card itself declined to pre-empt that call. Recorded as a "
    "NEW deferred decision in this record's deferred_decisions block; the residual stays "
    "registered either way (DECISION-ALSO-REGISTERED-AS-FOLLOWUP's shape).",
)
verdict(
    L + "9",
    "noted",
    "TASK-ROUTE-TSAN landed later in the wave (3fb52b8c). CT07's stage evidence cites nothing "
    "from it; the TSan row it named (every_subject_runs_clean_under_thread_sanitizer_when_bound) "
    "is a citation available to a later lane, not a stage move (constraint 23).",
)
verdict(
    L + "10",
    "applied",
    "The relabel was restored by --gate at every later regeneration (the merge messages say so) "
    "and again by this close with its OWN gate.json, which is the label the block now carries.",
    [c(MATRIX, "integrator_supplied (docs/reports/wave37-close/gate.json)")],
)
verdict(
    L + "11",
    "applied",
    "CT-FIXTURE-EXTERNAL-READERS registered at the merge, verified by id. Its discharge (a "
    "check that every include_str! of compiler/tests/fixtures/ct/ outside ct_corpus.rs is "
    "listed in one place or refused) is NOT built by this close; it is carried in the entry.",
    [c(MATRIX, '"id": "CT-FIXTURE-EXTERNAL-READERS"')],
)
verdict(L + "12", "noted", "The measured list of eight external readers; part of item 11 and carried in that entry's `why`.")
verdict(L + "13", "noted", "The consequence sentence of item 11 (three proof-gated cases among the eight); carried with the entry.")

# ------------------------------------------------------------------------- docs-compact-7
D = "docs-compact-7#for-the-integrator-and-for-the-close#"
verdict(
    D + "1",
    "applied",
    "Landed in the entry's own terms and one step further. This close's gate.json carries a "
    "`lean_audits` key holding the count THIS run measured, so the merged-gate row's Lean figure "
    "now has a machine-readable source in the record it cites. And because the generator's "
    "--gate refused any key outside its nine (the brief's two demands - a lean_audits key AND a "
    "regeneration with this close's gate - could not both hold under that rule), "
    "build_matrix.py now takes `lean_audits` as an optional CHECKED gate key: accepted when it "
    "equals the proof/Audit*.lean count on disk, refused otherwise, and never copied into the "
    "published block, which stays derived. Two tests added; the refusal test shown red by "
    "mutating the comparison to False (private log matrix-mutation.log, exit 1, "
    "'AssertionError: 0 == 0') and green after the restore. The drift is now a refusal where "
    "it would have been written.",
    [c(GATE, '"lean_audits"'), c(GEN, 'CHECKED_GATE_KEYS = ["lean_audits"]')],
)
verdict(
    D + "2",
    "applied",
    "Constraint 14's enumeration now runs through wave 37 (seven of seven, with each report's "
    "section cited) and the wave-36 entry is re-verified by measurement.",
    [c(AGENTS, "**wave 37's seven of seven**"), c(AGENTS, "**Waves 28 through 37 each closed at zero reserved words**")],
)
verdict(
    D + "3",
    "applied",
    "Decided and landed: the Python suite joins the standing gate's fenced block, with a "
    "sentence saying why and that it runs after any matrix regeneration. Every close since "
    "wave 32 had run it; the block now lists what the gate measures.",
    [c(AGENTS, "**The Python suite is listed as of the wave-37 close**")],
)
verdict(
    D + "4",
    "applied",
    "The three deviant heads normalised to the bold inline shape, head lines only, bodies "
    "verbatim: two `**Wave 37 · NAME (date).**` heads become `**NAME (date).**`; the `## "
    "SCALAR-ROUTE-EXIT-EDGE (2026-09-18, wave 37's first arbiter)` heading becomes the inline "
    "head `**SCALAR-ROUTE-EXIT-EDGE (2026-09-18, wave 37's first arbiter).**` joined to its "
    "body's first line (the blank line between heading and body removed, no body text changed; "
    "its parenthetical kept because wave 36's own file carries both `(date)` and `(date, wave "
    "36)` heads and dropping it would drop a fact the body does not restate).",
    [
        c(W37, "**`W37-CORPUS-CONTRACTS-4` (2026-09-18).**"),
        c(W37, "**`JOINED-PROVENANCE-ROUND-BOUND` (2026-09-18).**"),
        c(W37, "**`SCALAR-ROUTE-EXIT-EDGE` (2026-09-18, wave 37's first arbiter).**"),
        a(W37, "## SCALAR-ROUTE-EXIT-EDGE"),
        a(W37, "**Wave 37 ·"),
    ],
)
verdict(
    D + "5",
    "applied",
    "Confirmed by measurement before acting: at c33c7640 the two citations had ALREADY drifted "
    "(`condition.is_empty()` is at legacy_c.rs:3325, the successor assignment at :5177; the "
    "wave-36 close's 3312 and 5161 were moved by SCALAR-ROUTE-EXIT-EDGE), so the standing "
    "constraint carried two false line numbers. Repaired to the identifiers themselves with the "
    "line numbers pinned to the commit they were measured at, so the citation cannot go "
    "silently false again.",
    [c(AGENTS, "lines 3325 and 5177 at `c33c7640`"), a(AGENTS, "`legacy_c.rs:3312`"), a(AGENTS, "`legacy_c.rs:5161`")],
)

# ------------------------------------------------------- joined-provenance-round-bound
J = "joined-provenance-round-bound#integrator-reconciliation-list#"
verdict(
    J + "1",
    "applied",
    "No headline number moved for this card and the baseline pointer was NOT advanced to it "
    "(the card asked that it not be); it moved later, to w37-corpus-contracts-4, by that card's "
    "own item 2. Verified: AGENTS.md names no joined-provenance-round-bound archive.",
    [a(AGENTS, "joined-provenance-round-bound/artifacts")],
)
verdict(
    J + "2",
    "applied",
    "Constraint 10b carries the amendment in the card's own tree's words (item 7): the whole "
    "10b paragraph at c33c7640 is byte-identical to the card's tip 0579369d (diff exit 0). The "
    "tree's text integrates item 2's quotation into the existing sentence rather than appending "
    "it verbatim - the card's own choice in its own tree, not an integrator rewording.",
    [c(AGENTS, "**Do not restore a constant cap**, and do not weaken the declining arm: it is what makes the bound safe to raise.")],
)
verdict(J + "3", "noted", "The proposed text (body of item 2).")
verdict(J + "4", "noted", "'Land it in these words or decline' - landed, item 2.")
verdict(
    J + "5",
    "noted",
    "Checked: AGENTS.md, CLAUDE.md, the matrix, wave-36.md and wave-37.md carry '1,272' only in "
    "wave-37.md's own correction narrative ('correcting a 1,272 mis-transcription'). No tracker "
    "or brief carries the wrong figure; nothing to correct.",
)
verdict(
    J + "6",
    "applied",
    "At 797661f3: JOINED-PROVENANCE-ROUND-BOUND, JOINED-PROVENANCE-DECLINE-INDISTINGUISHABLE and "
    "JOINED-PROVENANCE-FIXED-POINT-BOUND-UNMEASURED retired; the four successors registered; "
    "JOINED-PROVENANCE-RELEASE-NULLABLE kept. All verified by id.",
    [
        a(MATRIX, '"id": "JOINED-PROVENANCE-ROUND-BOUND"'),
        c(MATRIX, '"id": "JOINED-PROVENANCE-BOUND-COST-UNMEASURED"'),
        c(MATRIX, '"id": "JOINED-PROVENANCE-RELEASE-NULLABLE"'),
    ],
)
verdict(
    J + "7",
    "noted",
    "Carried here for the next consumer's brief, with the pointer: "
    "docs/reports/joined-provenance-round-bound/README.md section 5 and section 12 item 5 - "
    "release_site is the relation's only reader today, so the 10 newly derived sites across 4 "
    "functions become live inputs to any second consumer with no further change to the module.",
)
verdict(J + "8", "noted", "Nothing allocated, nothing widened; not an instance of the condition-extension rule.")
verdict(
    J + "9",
    "applied",
    "Verified the wording survived the merge rather than re-derived: 10b byte-identical between "
    "0579369d:AGENTS.md and c33c7640; the evidence pointer list carries "
    "`joined-provenance-round-bound`.",
    [c(AGENTS, "card `JOINED-PROVENANCE-ROUND-BOUND`, wave 37) and **not a constant**")],
)
verdict(
    J + "10",
    "superseded",
    "The Lean figure was folded at the merge (136), DOCS-COMPACT-7 then re-measured the whole "
    "row at 2caf7369 (137) and named the fold's mechanism as the drift "
    "(GATE-ROW-FIGURE-ATTRIBUTION-DRIFT), and this close replaces the row with its OWN "
    "measurement under its own provenance, with the Lean count in its gate.json.",
    [c(AGENTS, "wave37-close/gate.json")],
)
verdict(
    J + "11",
    "applied",
    "The rule the card asked for is in this close's brief and was followed: regenerated with "
    "--gate docs/reports/wave37-close/gate.json.",
    [c(MATRIX, "integrator_supplied (docs/reports/wave37-close/gate.json)")],
)
verdict(J + "12", "noted", "No artifacts directory, deliberately; the reproduction commands regenerate it.")

# --------------------------------------------------------------- scalar-route-exit-edge
S = "scalar-route-exit-edge#for-the-integrator#"
verdict(S + "1", "noted", "Preamble.")
verdict(
    S + "2",
    "applied",
    "The chain sentence landed in the AGENTS.md corpus row at the merge 0b0b4ac1 in the card's "
    "words, and was then MOVED VERBATIM by DOCS-COMPACT-7 into wave-37.md's checkpoint section "
    "2 (the row as it stood at 2caf7369) when that card compacted the per-arbiter chronicle. "
    "The row's numbers are unmoved, as the item says.",
    [c(W37, "`SCALAR-ROUTE-EXIT-EDGE` (wave 37's first arbiter) then moved **zero** leaves")],
)
verdict(S + "3", "noted", "The proposed sentence (body of item 2).")
verdict(
    S + "4",
    "applied",
    "Constraint 5c carries the amendment in the card's own TREE's words, which differ from the "
    "section-10 quotation by one clause the tree adds after '(wave 37)': ', and "
    "\"conservatively wherever measured\" was measured only on positively spelled headers'. "
    "That clause is present at the card's own tip da532616:AGENTS.md, so the divergence is the "
    "card's - between its proposal and its tree - and not an integrator softening; it adds a "
    "boundary in the refusing direction. Recorded rather than smoothed.",
    [
        c(AGENTS, "That neighbour is now owned by card `SCALAR-ROUTE-EXIT-EDGE` (wave 37)"),
        c(AGENTS, "A loop header's exit edge is therefore **never removed**, whatever the value says"),
    ],
)
verdict(S + "5", "noted", "The proposed text (body of item 4).")
verdict(
    S + "6",
    "applied",
    "At 0b0b4ac1: CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED retired (verified registered before, "
    "absent after), five successors registered; verified by id.",
    [a(MATRIX, '"id": "CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED"'), c(MATRIX, '"id": "GATEWAY-VOLATILE-PREMISE"')],
)
verdict(S + "7", "noted", "No Q code, no widening, and NOT an instance of the condition-extension rule; the Q-space note's tally was not moved.")
verdict(
    S + "8",
    "noted",
    "Taken up by W37-CORPUS-CONTRACTS-4 itself: its BEFORE was checked against "
    "joined-provenance-call-frames' artifacts and re-run at 797661f3 after both code arbiters "
    "(merge 2caf7369's message), which confirmed both zero-movement claims from outside them.",
)

# --------------------------------------------------------------------- task-route-tsan
T = "task-route-tsan#integrator-reconciliation-list#"
verdict(
    T + "1",
    "applied",
    "Applied at the merge 3fb52b8c (200 -> 203 at the integration tip; the card's 171 -> 174 "
    "was against its own base). TASK-ROUTE-TSAN-SEGV verified registered before retirement and "
    "absent now; the four registrations verified by id.",
    [c(MATRIX, '"id": "TASK-ROUTE-TSAN-PTHREAD-BINDING"'), a(MATRIX, '"id": "TASK-ROUTE-TSAN-SEGV"')],
)
verdict(
    T + "2",
    "applied",
    "The `why` parenthetical replaced at the merge in the card's words; the one difference is "
    "the matrix's ASCII `--` where the report writes an em dash, the convention every `why` in "
    "the file already uses. The stale sentence is verified absent.",
    [
        c(MATRIX, "TSan now runs on this backend's own emitted crossing (docs/reports/task-route-tsan/README.md; remaining-t04's diagnosis was right and card TASK-ROUTE-TSAN worked around it with a pthread binding that changes no emitted byte), and reports nothing -- but over a schedule in which the creator and child never contend, so the instrument is available and the contention question is still open"),
        a(MATRIX, "TSan is not run anywhere in this repository"),
    ],
)
verdict(T + "3", "noted", "The old text (body of item 2).")
verdict(T + "4", "noted", "'with:' (body of item 2).")
verdict(T + "5", "noted", "The new text (body of item 2).")
verdict(T + "6", "noted", "'Land it in these words' - landed, item 2.")
verdict(T + "7", "noted", "No CT row, manifest entry or stage evidence moved by this card; CT-LANE-5's record cites nothing from it.")
verdict(
    T + "8",
    "noted",
    "Verified: the wave-36 pin is still in compiler/tests/write_lend_tasks_exercise.rs by name.",
    [c("compiler/tests/write_lend_tasks_exercise.rs", "thread_sanitizer_cannot_run_this_routes_programs_lease_or_not")],
)
verdict(
    T + "9",
    "applied",
    "TASK-ROUTE-TSAN listed in constraint 14's wave-37 entry with its section 6.2 cited, the "
    "gap the card refused to write in front of (waves 36 and 37 both missing) closed in the "
    "same edit.",
    [c(AGENTS, "`TASK-ROUTE-TSAN` §6.2")],
)
verdict(
    T + "10",
    "deferred",
    "Not widened here, with the ground the brief asks for: seeing a prose-only open item needs "
    "a definition of that shape - which words, in which section, count as an open item - that "
    "a test can make red by mutation, and no such definition was designed or measured by this "
    "close. The instance (remaining-t04 section 5.1, lost then contradicted) is recorded, and "
    "the question is added to deferred_decisions as SWEEP-RESIDUALS-PROSE-ONLY-ITEMS so it is "
    "re-listed at every close until someone takes it.",
)

# ------------------------------------------------------------ w37-corpus-contracts-4
W = "w37-corpus-contracts-4#for-the-integrator#"
verdict(W + "1", "noted", "Preamble: work the items, not the numbers. Done that way.")
verdict(
    W + "2",
    "applied",
    "Every number verified at c33c7640 against the ledger rows: 213/84/4,537; 2/42/137/4; leak "
    "272; unsupported 49,753; unknown_alias 4,309; summary_conflict 1; corpus 88; v1 59; "
    "manifest 33.",
    [
        c(AGENTS, "**213 Safe / 84 provisional Unsafe / 4,537 Unknown**"),
        c(AGENTS, "`unsupported` **49,753**"),
        c(AGENTS, "**88 modules**, **59** resource-v1 declarations, **33** manifest-selected"),
    ],
)
verdict(
    W + "3",
    "applied",
    "The baseline pointer names this card's artifacts.",
    [c(AGENTS, "baseline is `docs/reports/w37-corpus-contracts-4/artifacts/`")],
)
verdict(
    W + "4",
    "noted",
    "Discharged by the card itself: rebased onto 797661f3, both stages re-run, BEFORE and AFTER "
    "byte-identical to the archived pair (merge 2caf7369's message carries the digests). An "
    "independent confirmation of both code arbiters' zero movement.",
)
verdict(
    W + "5",
    "applied",
    "All ten names verified by id at c33c7640 (LOADER-ABI-INLINE-FNPTR-DECLARATOR, "
    "RDKAFKA-SORT-WITHDRAWN, W37-COVER-FRONTIER-IS-STATIC, NESTED-CALL-IN-ARGUMENT-POSITION, "
    "CTRACES-FORMATTER-CHAIN, LIBC-PRINTF-FAMILY, LOADER-ABI-RESTRICT-QUALIFIER, "
    "CONDITIONAL-RELEASE-BY-LITERAL, CPROFILES-STRAY-DUPLICATE-TREE, "
    "MPACK-WRITER-INTERIOR-REALLOC).",
    [c(MATRIX, '"id": "MPACK-WRITER-INTERIOR-REALLOC"'), c(MATRIX, '"id": "LOADER-ABI-INLINE-FNPTR-DECLARATOR"')],
)
verdict(
    W + "6",
    "noted",
    "No action, as asked; the ledger reads double_release 0.",
    [c(AGENTS, "`double_release` **0**")],
)
verdict(
    W + "7",
    "applied",
    "Constraint 7's paragraph is at the end of the constraint in the card's words (appended at "
    "the merge 2caf7369).",
    [c(AGENTS, "Card `W37-CORPUS-CONTRACTS-4` (wave 37) left the number unmoved a second consecutive time: 272 → 272 across a re-pin adding ten declared contracts and moving eight function statuses.")],
)
verdict(W + "8", "noted", "The proposed text (body of item 7).")
verdict(
    W + "9",
    "applied",
    "Constraint 8's paragraph is at the end of the constraint in the card's words, WITH A "
    "WHITESPACE REPAIR BY THIS CLOSE: the merge's rewrap (done, per its message, so that no "
    "residual name broke at a hyphen) protected LOADER-ABI-INLINE-FNPTR-DECLARATOR and split "
    "the backticked marker `abi-unreadable-declaration` and the word `same-position` across "
    "lines instead. The --check normaliser collapses whitespace but keeps the inserted space, "
    "so the claim below would have FAILED on the merged text; both are now on one line, no "
    "word changed (README section 9, the integrator's own record).",
    [
        c(AGENTS, "the declaration is withdrawn at every call site with `abi-unreadable-declaration`. The withdrawal is **leaf-neutral**: all eleven rows are same-position substitutions and no function's leaf count changed."),
        c(AGENTS, "named `LOADER-ABI-INLINE-FNPTR-DECLARATOR`"),
    ],
)
verdict(W + "10", "noted", "The proposed text (body of item 9).")
verdict(
    W + "11",
    "applied",
    "Listed in constraint 14's wave-37 entry with section 10 item 8 cited.",
    [c(AGENTS, "`W37-CORPUS-CONTRACTS-4` §10 item 8")],
)
verdict(
    W + "12",
    "noted",
    "mpack/ kept separate from msgpack/, the manifest position and module name left as chosen, "
    "the count pins moved deliberately at the merge (invariant 5). The report's parenthetical "
    "'(W37-MPACK-FAMILY-FIRST, registered)' is FALSE: the name is in no named_followups entry "
    "and not in the card's own section-9 table. Classified by this close as a corpus-layout "
    "convention recorded at the pinned table's comment (qcore_contract_corpus.rs), not open "
    "work, and NOT registered; the unchecked-claim shape is registered instead as "
    "REPORT-REGISTERED-CLAIM-UNCHECKED.",
)
verdict(
    W + "13",
    "applied",
    "The restatement appended to ADD-NAME-COLLISION's `why` in the card's words ('definitions "
    "that DISAGREE, not several definitions'); W36's original text kept in front of it.",
    [c(MATRIX, "the rule is about definitions that DISAGREE, not about several definitions")],
)
verdict(
    W + "14",
    "superseded",
    "Regenerated again by this close with the wave-37 close's own gate, which is the label the "
    "block now carries. The card's D02-B00-LEDGER-SNAPSHOT note stands (constraint 25).",
    [c(MATRIX, "integrator_supplied (docs/reports/wave37-close/gate.json)")],
)
verdict(
    W + "15",
    "superseded",
    "DOCS-COMPACT-7 re-measured the row at 2caf7369 after this card, and this close replaces it "
    "with its own measurement at its own base under its own provenance (README section 10).",
    [c(AGENTS, "wave37-close/gate.json")],
)
verdict(W + "16", "noted", "Every Safe is relative to ten declared premises; nothing here is an upstream claim (invariant 2).")
verdict(W + "17", "noted", "A horizontal rule the sweep tokenised as an item; no content.")

assert len(V) == 77, len(V)

record = {
    "wave": 37,
    "cards": [
        "base-type-reader-census",
        "scalar-route-exit-edge",
        "joined-provenance-round-bound",
        "ct-lane-5",
        "task-route-tsan",
        "w37-corpus-contracts-4",
        "docs-compact-7",
    ],
    "verify_applied": True,
    "note": (
        "WAVE-37-CLOSE. Base c33c7640, the DOCS-COMPACT-7 merge (= origin/main at the start). "
        "77 integrator-addressed items across seven cards, each given a verdict MEASURED AGAINST "
        "THE TREE rather than read off the card's claim that it landed. NOTHING WAS FOUND "
        "UNAPPLIED AT ITS MERGE THIS WAVE - every amendment a card proposed is in the tree in the "
        "card's words - and the close still found five things to do that no merge had done: two "
        "standing-constraint texts that had gone false or fragile after landing (5c's line "
        "citations, already moved by the wave's own first arbiter; 8's rewrap splitting a "
        "backticked marker across lines), a tracker carrying two entries whose discharge "
        "conditions were met (retired here, named and checked), a report asserting a name was "
        "'registered' that no tracker carries, and the wave-36 entry of constraint 14 re-verified "
        "by measurement rather than by reading. Two texts landed at merges differ from their "
        "cards' section quotations and both differences are the CARDS' OWN (present at each "
        "card's own tip): 5c carries one extra clause, 10b integrates the quotation into its "
        "sentence. Recorded, not smoothed."
    ),
    "verdicts": V,
    "no_section": {
        "remaining-d02": (
            "Card D02's own report directory, swept because this close changed "
            "docs/reports/remaining-d02/build_matrix.py (the checked `lean_audits` gate key). "
            "Its README is a frozen card report from the D02 wave with no section addressed to "
            "this integrator; the generator lives beside it by history, and the change is this "
            "close's own, recorded in this close's report and verified by --check through the "
            "docs-compact-7 item-1 claims."
        ),
        "wave36-close": (
            "The wave-36 close's own report, found because its branch commits are reachable only "
            "through 0f6eb0a6, the first commit of this range. It has no integrator-addressed "
            "section and must not: it is the previous integrator's record; its findings for this "
            "wave were registered residuals and deferred decisions, which this close re-lists."
        ),
        "wave37-close": (
            "This close's own report. It has no integrator-addressed section and must not: it IS "
            "the integrator's record, and a close that addressed items to itself would be the "
            "reconciliation defect in miniature. Its findings go forward as one registered "
            "residual (REPORT-REGISTERED-CLAIM-UNCHECKED) and the deferred decisions below."
        ),
    },
    "hand_sweep": {
        "note": "What the sweep cannot reach, swept by hand and recorded so the close's coverage is not confused with the tool's.",
        "items": [
            "The 21 seam commits --uncarded returned were classified by eye, one by one, and every one is a card's OWN commit: TASK-ROUTE-TSAN 4 (dd07c480, cbc0a318, 65ac3a26, 633279a6), W37-CORPUS-CONTRACTS-4 3 (c1de087f, 5c96a8a1, df0bd34f), JOINED-PROVENANCE-ROUND-BOUND 3 (5a2d1f6e, 73099b01, 114694bc), WAVE-36-CLOSE 3 (830fbbcf, c7c6c302, 94a9bcbd - reachable only through the 0f6eb0a6 merge that opens this range), DOCS-COMPACT-7 2 (579891ca, 20bdb53f), CT-LANE-5 2 (ca8e7b55, b40888e9), BASE-TYPE-READER-CENSUS 2 (4a1d2d54, 5dfcc8b9), SCALAR-ROUTE-EXIT-EDGE 2 (119043ee, 8aaeb523). 4+3+3+3+2+2+2+2=21. ZERO integrator commits outside merges this wave (wave 36 had three): every integrator edit of wave 37 lives INSIDE a merge commit, which is a different shape of SYNC-RULE-TOOLING-COMMIT-SCOPE's question and is restated under it.",
            "The whole range 0f6eb0a6^..c33c7640 is 63 commits: 8 merges (the wave-36 close's and the seven cards') and 55 branch commits (WAVE-36-CLOSE 11, TASK-ROUTE-TSAN 10, W37-CORPUS-CONTRACTS-4 7, BASE-TYPE-READER-CENSUS 6, DOCS-COMPACT-7 6, SCALAR-ROUTE-EXIT-EDGE 5, JOINED-PROVENANCE-ROUND-BOUND 5, CT-LANE-5 5). 8+55=63, every one attributable.",
            "The 34 residual-shaped names --residuals returned were classified individually, not in bulk (README section 5): 13 card names, 13 discharged with the discharging card named (five of them THIS wave's: CT07-TWO-THREAD-REMAP by CT-LANE-5, JOINED-PROVENANCE-DECLINE-INDISTINGUISHABLE and JOINED-PROVENANCE-FIXED-POINT-BOUND-UNMEASURED by JOINED-PROVENANCE-ROUND-BOUND, and the wave-36 pair MAIN-GUARD-UNACTIVATED / RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED still written in the history that discharged them), 2 standing rules or prospective names, 2 deferred decisions, 2 verdict or bucket labels (DECIDABLE-NOW, RED-AS-REQUIRED), 1 a named premise of a superseded arm (SCALAR-ROUTE-POISON-DEPENDENCE, explained by its card's repair), 1 a corpus-layout convention a report falsely called registered (W37-MPACK-FAMILY-FIRST). 13+13+2+2+2+1+1=34. ZERO registered from that list; the one registration this close makes (REPORT-REGISTERED-CLAIM-UNCHECKED) is the close's own finding about the last of them.",
            "Two RETIREMENTS by this close, each named and verified a registered entry before removal: BASE-TYPE-READER-UNMEASURED (discharge met word for word by BASE-TYPE-READER-CENSUS; the merge dedup'd the card's registration against the wave-36 close's and left the entry open) and CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE (discharge taken in both arms by the wave-35 close and standing in constraint 14's text; the enumeration now current through wave 37). named_followups 216 -> 215 (-2 +1), the count line in AGENTS.md kept in step, and --residuals re-run AFTER the removals: 34 -> 43, every movement accounted (README section 5.2) and NO third-party name orphaned. Two names went quiet FOR THE WRONG REASON and are recorded so no later close reads the drop as a decision: W37-MPACK-FAMILY-FIRST (cited inside REPORT-REGISTERED-CLAIM-UNCHECKED's why, not registered) and CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE (retired here, yet cited twice inside other entries' why texts, so the case-insensitive whole-text scan still finds it) - the wave-36 close's C-GATEWAY-INDIRECT-CENSUS-2 mechanism, twice.",
            "AGENTS.md's headline numbers were re-read from their sources rather than from the ledger: named_followups against the matrix file (215 after this close's edits, dimensions.named_followups derived to the same), the contract corpus against the pinned tables (88 modules; 33 manifest-selected), the CT row against ct-lane-5's stage-evidence record (46/50, 10 of 46, 3 of 46, no count moved), the corpus row against the wave-37 baseline archive named in the headline paragraph, and the merged-gate row REPLACED by this close's own measurement at its own base under its own provenance, with the Lean count carried in this close's gate.json under a key the generator now CHECKS against proof/Audit*.lean.",
            "The integrator's own record this wave (README section 9) is step-3 material and is in the close unsmoothed: the round-bound x scalar compile failure fixed at 797661f3 with one fixture line; two text-wrapping incidents at 2caf7369 (one caught before push, one - the split backticked marker in constraint 8 - found and repaired by this close); a conflict-free merge that self-committed with a placeholder message and was collapsed by reset --soft + --amend into c33c7640 (verified: two parents, one commit, tree identical to what its gate measured); a merge script's `checkout --ours AGENTS.md` step replaced before it bit (verified: 5c, 10b, 7 and 8 all present at c33c7640 in their cards' words); and W37-CORPUS-CONTRACTS-4's mid-wave rebase with BEFORE/AFTER re-run.",
        ],
    },
    "deferred_decisions": {
        "note": (
            "Ten decision ENTRIES were carried into this close from docs/reports/wave36-close/"
            "reconciliation.json (eight open, one resolved at wave 35, one withdrawn reader "
            "artifact), restated here with grounds; ONE is resolved by the tree this wave "
            "(TASK-ROUTE-TSAN-SEGV-PRIORITY, card TASK-ROUTE-TSAN landed at 3fb52b8c) and named "
            "as such; the two already ended stay recorded so the ten-entry carry is accounted "
            "rather than silently reduced. TWO new decisions are raised by this wave's cards and "
            "recorded for the user: CT07-TWO-THREAD-REPLAY-ORDER (CT-LANE-5 section 11 item 7) "
            "and SWEEP-RESIDUALS-PROSE-ONLY-ITEMS (TASK-ROUTE-TSAN section 9 item 6). No close "
            "may answer a decision raised for the user; each is carried until it carries "
            "resolved or withdrawn with a ground."
        ),
        "items": [
            {
                "name": "CT10-REASON-ROW",
                "verdict": "deferred",
                "ground": "Still deferred, a fifth wave. NEW EVIDENCE from wave 37, about the row's content rather than the decision: CT-LANE-5 re-measured CT10 and found the byte encoding of bit-fields unsound in BOTH directions (CT10-BYTE-ENCODING-UNSOUND, registered), so whatever reason row is eventually written must say more than 'half a scenario'. The decision - whether to write such a row at all - is untaken and this close does not take it. Still double-carried as a named_followup (DECISION-ALSO-REGISTERED-AS-FOLLOWUP).",
            },
            {
                "name": "MATRIX-UPDATED-STALE-AT-WAVE-27",
                "verdict": "deferred",
                "ground": "Still deferred; untaken. NEW EVIDENCE: the matrix was regenerated at all seven wave-37 merges that touched it (each with --gate, after two cards hit the provenance downgrade without it) and once more by this close with its own gate, so the `updated` field's wave-27 prose now sits beside a file regenerated ten waves later. Still double-carried as a named_followup.",
            },
            {
                "name": "EXAMPLES-STYLE-SLICE-FANOUT",
                "verdict": "deferred",
                "ground": "Still deferred, a fifth wave. No card of wave 37 touched examples/ (the seam's front-end diff is empty and no example directory moved), so this close produces NO new evidence about it. Re-listed so it does not expire in silence.",
            },
            {
                "name": "SYNC-RULE-TOOLING-COMMIT-SCOPE",
                "verdict": "deferred",
                "ground": "Still deferred, and wave 37 gives it a NEW SHAPE: --uncarded returned 21 commits and every one is a card's own; there were ZERO integrator commits outside merges (wave 36 had three, wave 35 four). The integrator's edits this wave - the compile fix, the constraint rewraps, the recomputed count lines - all live INSIDE merge commits, indexed by nothing but the merge message. So the question the decision asks (what scope of committed-document change needs a report) did not go away; it moved into a place --uncarded cannot see, and this close saw it only because the brief named each incident. A rule about the merge commit's own edits is the same decision in different clothes.",
            },
            {
                "name": "PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED",
                "verdict": "deferred",
                "ground": "Still deferred, and this close again did NOT resolve it by completing the registration, for the wave-35 ground: which arm to take is the decision. NEW EVIDENCE: wave 37 added TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN, registered, so the family is once more MORE partially tracked than before; the CLAUDE.md platform list still carries its unregistered members.",
            },
            {
                "name": "DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK",
                "verdict": "deferred",
                "ground": "Still deferred - a question raised for the user. NEW DATA POINT, offered as such: DOCS-COMPACT-7 fired this wave and did BOTH things at once - compaction (AGENTS.md -5.30%, seven checkpoint blocks moved verbatim) and correction (nine factual corrections measured against the tree, one of them a figure wrong the same way in two consecutive waves). And this close then found two more standing-constraint corrections the compaction pass did not (5c's drifted citations, 8's split marker), so the correction work is not exhausted by the cadence card either. Neither reading is argued for here.",
            },
            {
                "name": "VERIFIER-HOST-ROW-DISPOSITION",
                "verdict": "resolved",
                "ground": "Ended at the wave-35 close; recorded so the ten-entry carry is accounted. No wave-37 card touched the verifier host, and this close's gate saw the host row green under full-suite load (gate.json), which is consistent with the resolution and is not a quiet-host measurement.",
            },
            {
                "name": "items",
                "verdict": "withdrawn",
                "ground": "Not a decision: the wave-34 reader artifact withdrawn at wave 35, carried for accounting only.",
            },
            {
                "name": "INDIRECT-CENSUS-FEATURE-UNBUILT",
                "verdict": "deferred",
                "ground": "Still deferred, RESTATED WITH THIS WAVE'S EVIDENCE and not resolved: the decision now has a SIBLING and a second payer. JOINED-PROVENANCE-ROUND-BOUND added a `provenance-rounds-census` feature to the same crate, also off by default, also asserting byte-identity with the feature on, also built by no workspace member and no CI job; and BASE-TYPE-READER-CENSUS extended the original `indirect-census` feature and registered BASE-TYPE-READER-CENSUS-FEATURE-UNBUILT for it ('now carrying two cards' instrumentation and will rot twice as expensively'). Three cards' measurement instrumentation now sits behind two features nothing builds. Both arms stand as written at wave 36: a CI job building the features, or a recorded decision that the instrumentation is disposable and a future census re-derives it.",
            },
            {
                "name": "TASK-ROUTE-TSAN-SEGV-PRIORITY",
                "verdict": "resolved",
                "ground": "RESOLVED BY THE TREE. The decision was whether the route-wide absence of thread-sanitizer evidence was wave-37 card work or a follow-on; card TASK-ROUTE-TSAN ran in wave 37 and merged at 3fb52b8c, diagnosing the fault outside QCore (ThreadSanitizer's runtime ships no thrd_create interceptor), binding thrd_create/thrd_join to the pthread bodies in the test harness with zero emitted bytes moved, and landing the first TSan rows; TASK-ROUTE-TSAN-SEGV was retired at that merge with its condition met word for word. What remains is registered, not deferred: TASK-ROUTE-TSAN-EXECUTED-SCHEDULES, TASK-ROUTE-TSAN-PTHREAD-BINDING, TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN.",
            },
            {
                "name": "CT07-TWO-THREAD-REPLAY-ORDER",
                "verdict": "deferred",
                "ground": "RAISED BY WAVE 37, card CT-LANE-5 section 11 item 7, and recorded for the user rather than closed. The residual (registered at e88d9917) says CT07's byte-identical trace does NOT witness interleaving, because worker events are buffered privately and replayed by the parent - a deliberate design that makes traces comparable at all (constraint 15a's premise). It can be closed BY STATEMENT: a sentence in the ledger accepting that as a stated boundary of the trace mechanism. The card declined to write that sentence, on the ground that whoever writes it is the one a later reader will rely on when taking CT07's comparison for interleaving evidence; this close declines for the same reason. The other arm is work: a TSan row on CT07's flagship, which TASK-ROUTE-TSAN's harness now makes possible (every_subject_runs_clean_under_thread_sanitizer_when_bound is the shape) but which no card has cited into CT07's stage evidence. Double-carried as a named_followup.",
            },
            {
                "name": "SWEEP-RESIDUALS-PROSE-ONLY-ITEMS",
                "verdict": "deferred",
                "ground": "RAISED BY WAVE 37, card TASK-ROUTE-TSAN section 9 item 6, from its section 7.1 finding: docs/reports/remaining-t04/README.md section 5.1 held the correct TSan diagnosis and remedy in prose waves earlier, reached no tracker because it carried no SCREAMING-KEBAB name, and was then contradicted by wave 36's re-measurement while a registered entry cited it for the right diagnosis - both readings live in the tracker at once and nothing compared them (TSAN-DIAGNOSIS-WAS-LOST-THEN-CONTRADICTED, registered). The decision is whether --residuals should see open items written as prose. NOT taken here: widening the tool needs a definition of the shape - which words, where - that a test can make red by mutation, and none was designed or measured by this close; a heuristic landed without that would be the vacuous-greenness STABILITY-REEVAL hunts. Both arms: a measured definition plus the arm with its tests, or a recorded decision that names are the only currency and an unnamed open item is the author's loss.",
            },
        ],
    },
    "gate": GATE,
}

out = HERE / "reconciliation.json"
out.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
from collections import Counter

print("verdicts:", len(V), dict(Counter(v["verdict"] for v in V.values())))
print("verify claims:", sum(len(v.get("verify", [])) for v in V.values()), "unverifiable:", sum(1 for v in V.values() if "unverifiable" in v))
print("wrote", out)
