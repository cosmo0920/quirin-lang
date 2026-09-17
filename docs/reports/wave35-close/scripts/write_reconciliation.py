#!/usr/bin/env python3
"""Build docs/reports/wave35-close/reconciliation.json.

Written as a script rather than by hand so the record's shape is mechanical and
the verdicts are the only hand-written part. Every verdict below was MEASURED
AGAINST THE TREE - the target file grepped for the proposed text or its effect -
and never inferred from a report's own claim that it had landed.
"""

import json
import pathlib

OUT = pathlib.Path("docs/reports/wave35-close/reconciliation.json")

A = "applied"
D = "declined"
S = "superseded"
F = "deferred"
N = "noted"


def v(verdict, note, where=None):
    entry = {"verdict": verdict, "note": note}
    if where is not None:
        entry["where"] = where
    return entry


VERDICTS = {
    # ------------------------------------------------------------------ #
    # C-GATEWAY-CONSTANT-DO-WHILE (wave 35, arbiter #1) - 19 items
    # ------------------------------------------------------------------ #
    "c-gateway-constant-do-while#for-the-integrator#1": v(
        N,
        "Section heading '### 12.1 Applied by this card'. Carries no action; items 2-5 are its "
        "content and each was re-measured against the tree here rather than accepted as applied.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#2": v(
        S,
        "Applied by the card and then SUPERSEDED IN THE SAME WAVE by arbiter #2. The baseline "
        "pointer no longer names this card's artifacts, which is correct: JOINED-PROVENANCE-CYCLES "
        "merged after it, its fresh BEFORE reproduced this card's diagnostics.json and "
        "chunk-contracts.json byte for byte, and it moved zero leaves, zero functions and zero "
        "units. The chain rule the item asks for is therefore working exactly as described - each "
        "arbiter's BEFORE reproducing its predecessor's archives is what makes a delta attributable. "
        "Constraint 21's frozen list reads 'those six', as the item asked.",
        "AGENTS.md:108-112 - 'Headline numbers after `JOINED-PROVENANCE-CYCLES` (2026-09-18). The "
        "corpus baseline is `docs/reports/joined-provenance-cycles/artifacts/`'; "
        "c-gateway-constant-do-while stands in the chain list beneath it.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#3": v(
        A,
        "Verified in the tree at the close, not read off the card. 186 Safe / 84 provisional Unsafe "
        "/ 4,564 Unknown and double_release 0 are present; leak 272, unknown_alias 4,214 and "
        "unsupported 50,106 stand unmoved, which is what the item asked for since they did not "
        "move; and the 'no unit status has moved at any arbiter from wave 31 through wave 35' "
        "sentence is present. Arbiter #2 then moved none of these, as its own item 2 said.",
        "AGENTS.md, the Cycle 19 raw corpus row of the status ledger.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#4": v(
        A,
        "Applied by the card with the wave-33 gate, as stated, and the close has since regenerated "
        "the same file through the same generator with its OWN gate - which is what CT-CTWIN-LANE's "
        "item 3 asks for and what restores an integrator_supplied provenance naming this wave. "
        "lean_audits is derived and now reads 132 (129 + one audit per arbiter + one for the arms "
        "card), with AuditConstantDoWhile.lean in lean_audit_files.",
        "docs/reviews/r0_3/acceptance_matrix.json: validation.lean_audits 132, "
        "measurement_provenance 'integrator_supplied (docs/reports/wave35-close/gate.json)'. "
        "Regeneration confirmed a byte-identical fixed point and "
        "scripts/tests/test_build_matrix_sections.py passes 17/17.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#5": v(
        A,
        "The card's paragraph is present in the wave-35 increment record, newest-first ordering "
        "preserved; arbiter #2's paragraph now sits above it and this close's above that.",
        "docs/development/increments/wave-35.md.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#6": v(
        N,
        "Section heading '### 12.2 Not applied - needs your decision'. Its content is items 7-9.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#7": v(
        D,
        "DECLINED, taking the alternative the card itself offered in item 9 ('If you would rather "
        "the row stay a single-tip measurement, decline this and record it declined'). GROUND: the "
        "proposed multi-tip form - '130 Lean audits (129 at the DOCS-COMPACT-6 tip; ...)' - exists "
        "to avoid re-attributing a row to a tip at which it was not measured, and that problem does "
        "not arise here because THE CLOSE RE-MEASURED THE WHOLE ROW at its own base 4cf9bdf9. Every "
        "figure in it is now one measurement of one tree: 4,580 / 0 / 8, 132 Lean, 247 Python, both "
        "probes 0, fmt and clippy clean. Landing the multi-tip parenthetical would have left the row "
        "carrying DOCS-COMPACT-6's qualifications about a tip two waves behind, and the figure it "
        "proposed (130) is itself two tips stale - the wave added three audits, not one. The card's "
        "reasoning was right and its conclusion is obsolete, which is the ordinary fate of a "
        "'needs your decision' item that a later measurement answers.",
        "AGENTS.md, Merged gate row, rewritten by this close; '129 Lean audits' now occurs zero "
        "times in the file.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#8": v(
        D,
        "The block quote of item 7's proposed replacement text. Declined with item 7, same ground; "
        "it is not a separate amendment.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#9": v(
        A,
        "This is the sentence offering the decline arm, and the close took it. Recorded `applied` "
        "rather than folded into item 7 so that the record shows the decline was the card's own "
        "stated alternative rather than the integrator overriding it.",
        "See item 7's `where`.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#10": v(
        N,
        "Section heading '### 12.3 Proposed standing constraint - exact words to land'. Content is "
        "items 11 and 12.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#11": v(
        A,
        "The placement instruction, applied exactly: 5a and 5b were indeed taken, 5c was free, and "
        "the new paragraph is inserted immediately after 5b under 'W4 custody inference and the C "
        "gateway'. The card's ground for the number - that 5a is precisely the rule this card's key "
        "obeys - was checked and holds.",
        "AGENTS.md, constraint 5c, immediately following 5b's closing "
        "'`c-gateway-return-position`.' line.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#12": v(
        S,
        "SUPERSEDED, NOT APPLIED, AND THE DIFFERENCE IS PUBLISHED IN BOTH DIRECTIONS - the wave-34 "
        "precedent for an amendment that could not land in the words proposed. The rule landed "
        "whole; TWO CLAUSES of the card's text did not, and the paragraph in AGENTS.md says so in "
        "its own body rather than only here. (1) THE STALE CLAUSE. The card wrote that removing a "
        "back edge makes a function acyclic 'and `legacy_joined_provenance::derive` returns no "
        "conditional-provenance facts at all for a cyclic CFG (constraint 10b)'. Wave 35's SECOND "
        "arbiter, JOINED-PROVENANCE-CYCLES, falsified that later in the same wave: a cyclic CFG is "
        "now solved by the greatest fixed point of the same transfer. VERIFIED IN SOURCE, not "
        "inferred - crates/qcore-c-gateway/src/legacy_joined_provenance.rs:151 returns "
        "`fixed_point(cfg, &reachable)`, and constraint 10b was itself rewritten by that card to say "
        "so. Landing the sentence verbatim would have put a statement contradicting 10b into the "
        "same file, so it reads 'and that changes which route `legacy_joined_provenance::derive` "
        "takes for it (constraint 10b)'. THE RULE IS UNCHANGED: a widening of the key still moves "
        "two subsystems and must re-measure both, which is the load-bearing sentence. (2) THE SCRIPT "
        "PATH. The card wrote `measure/.../acyclicity.py`, its own gitignored worktree-private path; "
        "replaced by the committed copy the card itself archived, "
        "docs/reports/c-gateway-constant-do-while/scripts/acyclicity.py, verified to exist. Nothing "
        "was softened in either substitution and neither changes what the rule forbids.",
        "AGENTS.md, constraint 5c, including its closing paragraph 'Two clauses of this paragraph "
        "are the integrator's, not the card's', which names both substitutions and points here.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#13": v(
        N,
        "Section heading '### 12.4 Constraint 14 and the Q namespace'. Content is item 14.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#14": v(
        A,
        "Landed in the words asked for. The claim was independently checked rather than accepted: "
        "the card reserves no word, moves no lexer rule and allocates no Q code, and it is not even "
        "a condition extension since it changes the CFG rather than a diagnostic's condition - so "
        "the 'do not engage' list is the right one for it, unlike MUTEX-SHARE-CROSS (see "
        "sweep-blind-spots#8).",
        "AGENTS.md, constraint 14, 'Cards that do not engage this constraint at all' bullet, "
        "wave 35's entry: C-GATEWAY-CONSTANT-DO-WHILE named among the seven.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#15": v(
        N,
        "Section heading '### 12.5 Residuals for the tracker'. Content is item 16.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#16": v(
        A,
        "FIVE of the six registered, and the sixth deliberately not - which is a better outcome than "
        "six and is why the item is not simply ticked. The card predicted "
        "`reconciliation_sweep.py --residuals` would find all six, and it did: all six were in this "
        "close's 62. CONSTANT-DO-WHILE-ZERO-SPELLINGS, -OTHER-CONSTANTS, -LATENT-REACH, "
        "-SUCCESSOR-ORDER and -SCALAR-ROUTE-UNOWNED are registered with their §8 discharge "
        "conditions. CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH is NOT registered because arbiter #2 "
        "ANSWERED it, in exactly the terms its discharge condition set: that card quoted its "
        "pre-pass coverage against this card's tree (148eef79) rather than a300a39d, reproduced the "
        "654-function flip independently (1,927 - 654 = 1,273 cyclic), and stated the effect - six "
        "functions and 20 leaves of servable population moved to the topological pass, combined "
        "diagnostic effect zero. Registering an answered residual would put closed work in the "
        "open-work ledger.",
        "docs/reviews/r0_3/acceptance_matrix.json named_followups (five new entries, "
        "recorded_by WAVE-35-CLOSE); docs/reports/joined-provenance-cycles/README.md §11 item 8 for "
        "the sixth.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#17": v(
        N,
        "Section heading '### 12.6 Hand-off to wave 35's second arbiter'. Content is items 18-19.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#18": v(
        N,
        "A statement of fact about the two arbiters, not an action, and it held. The two cards did "
        "not conflict in the tree - this one edits legacy_c.rs and never "
        "legacy_joined_provenance.rs - and the predicted interaction in the MEASUREMENTS is exactly "
        "what arbiter #2 reported: 654 more functions reaching its pre-pass, diagnostic effect zero.",
    ),
    "c-gateway-constant-do-while#for-the-integrator#19": v(
        N,
        "A merge-ordering instruction addressed to the integrator at merge time, already discharged "
        "before this close ran, and recorded here rather than silently dropped. The order it names "
        "as the expected one is the order that happened: this card merged first at 3ead460b as "
        "arbiter #1, and JOINED-PROVENANCE-CYCLES' BEFORE run reproduced this card's archives byte "
        "for byte, so no re-measurement of the 654 was owed and none was done.",
    ),
    # ------------------------------------------------------------------ #
    # CT-CTWIN-LANE (wave 35) - 15 items. THE WAVE'S ONE ENTIRELY
    # UNAPPLIED SECTION.
    # ------------------------------------------------------------------ #
    "ct-ctwin-lane#for-the-integrator#1": v(
        N,
        "Preamble sentence, 'These are the exact words to land.' No action of its own; it states the "
        "discipline items 2-15 are held to.",
    ),
    "ct-ctwin-lane#for-the-integrator#2": v(
        A,
        "LANDED BY THIS CLOSE, AND IT WAS ENTIRELY UNAPPLIED BEFORE IT - the wave's one unapplied "
        "finding, and the reason this record measures every item against the tree instead of reading "
        "reports. Measured at the close's base: `grep -c 'CT46' AGENTS.md` returned 0, "
        "`grep -c 'CT-CTWIN-LANE' AGENTS.md` returned 0 and `grep -c '46/50' AGENTS.md` returned 0, "
        "while the ledger row still read '45/50 mapped, 5 unmapped'. The card merged at cfd410d0 and "
        "THREE further cards merged after it, none of which carried the row.",
        "AGENTS.md, CT01-CT50 conformance row: now '**46/50 mapped**, 4 unmapped'.",
    ),
    "ct-ctwin-lane#for-the-integrator#3": v(
        A,
        "The three figures landed in the card's exact words. 'proof 3 of 46' replaced 'proof 3 of "
        "45' in place rather than being appended - a first attempt appended it and produced BOTH "
        "strings in one row, which was caught by grep and reverted before commit.",
        "AGENTS.md, CT01-CT50 conformance row: '46/50 mapped, 4 unmapped; sanitized-native **9 of "
        "46**' and '**proof 3 of 46**'. Verified 'proof 3 of 45' now occurs zero times.",
    ),
    "ct-ctwin-lane#for-the-integrator#4": v(
        A,
        "The placement instruction, followed: the wave-35 block is inserted immediately after the "
        "wave-32 sentence ending 'attribution separated in both directions.' and before the row's "
        "closing 'Each publishes the boundary ...' sentence, which is general to all mapping lanes.",
        "AGENTS.md, CT01-CT50 conformance row.",
    ),
    "ct-ctwin-lane#for-the-integrator#5": v(
        A,
        "The CT46 block quote landed word for word, including the four re-measurements that mapped "
        "nothing (CT41 structural, CT42 both arms measured, CT10 half a scenario, CT24 withdrawn) - "
        "which matter more than the mapping, since they are the lane saying what it did NOT do. The "
        "row keeps constraint 23's sentence that a grown denominator is mapping and not conformance.",
        "AGENTS.md, CT01-CT50 conformance row, the 'Wave 35 (`CT-CTWIN-LANE`) mapped **CT46** ...' "
        "passage.",
    ),
    "ct-ctwin-lane#for-the-integrator#6": v(
        A,
        "Done, by the rule the item itself names: the wave's own close re-measures. The row now "
        "carries this close's numbers at 4cf9bdf9 rather than the card's §7 numbers at its own tip, "
        "which is the stronger form of what was asked, and it states which it saw for the contested "
        "host row.",
        "AGENTS.md, Merged gate row: 4,580 passed / 0 failed / 8 ignored, 228 executables / 230 "
        "summaries, 247 Python, 132 Lean, provider 122 carried.",
    ),
    "ct-ctwin-lane#for-the-integrator#7": v(
        A,
        "Regenerated at the close with --gate, as asked. The provenance the item worried about had "
        "in fact already been restored by two later cards passing the WAVE-33 gate; this close "
        "supersedes that with its own.",
        "docs/reviews/r0_3/acceptance_matrix.json: validation.measurement_provenance = "
        "'integrator_supplied (docs/reports/wave35-close/gate.json)'.",
    ),
    "ct-ctwin-lane#for-the-integrator#8": v(
        A,
        "The literal command block. Run as written, with the close's own gate path substituted for "
        "the placeholder: `python3 docs/reports/remaining-d02/build_matrix.py . --gate "
        "docs/reports/wave35-close/gate.json`, exit 0. NOTE FOR THE NEXT CLOSE: the generator "
        "REFUSES a gate.json carrying keys outside its fixed nine, so this close's first gate.json - "
        "which added measured_at, lean_audits and the split pass/fail/ignored counts - was rejected "
        "with exit 1 and those figures were folded into the note instead. Worth knowing before "
        "writing a richer gate record.",
        "Generator output: 'gate provenance: integrator_supplied "
        "(docs/reports/wave35-close/gate.json)', 'named followups 121', 'lean audits 132'.",
    ),
    "ct-ctwin-lane#for-the-integrator#9": v(
        A,
        "Checked rather than assumed. Nothing in `validation` was hand-touched; the generator "
        "rewrote its derived fields and the numbers are as the item predicted.",
        "docs/reviews/r0_3/acceptance_matrix.json, validation block.",
    ),
    "ct-ctwin-lane#for-the-integrator#10": v(
        N,
        "'Do not re-append them' - obeyed, and verified rather than trusted. All four "
        "(CT41-TYPED-SOURCE-UNDISCRIMINATING, CT42-CELL-UNROUTED, CT-FIXTURE-CROSS-CASE-DUPLICATES, "
        "HOST-ROW-STARVATION-DESCRIPTION) are present exactly once each; none was re-appended. "
        "HOST-ROW-STARVATION-DESCRIPTION was AMENDED in place - see item 12 - which is a different "
        "operation and is recorded there.",
        "docs/reviews/r0_3/acceptance_matrix.json named_followups.",
    ),
    "ct-ctwin-lane#for-the-integrator#11": v(
        F,
        "DEFERRED, with the deferral recorded rather than the item quietly ticked. The item asks for "
        "an OWNER outside the lane, and no card of wave 35 owns the CT02/CT14 or CT03/CT04 fixtures "
        "where the two undisclosed byte-identical pairs live. GROUND: assigning an owner is work "
        "allocation, which is the user's and the next wave's, not a close's; and the entry is "
        "already registered, measured and explicitly 'not fixed', so nothing is lost by carrying it. "
        "The close adds no new information about it and says so rather than inventing a disposition.",
        "docs/reviews/r0_3/acceptance_matrix.json: CT-FIXTURE-CROSS-CASE-DUPLICATES present and "
        "unchanged.",
    ),
    "ct-ctwin-lane#for-the-integrator#12": v(
        A,
        "Applied, and applied as REPLACEMENT rather than re-issue, which is precisely what the item "
        "asked for and the harder half. Constraint 16's wave-34 paragraph - which recorded the row "
        "beside the five as 'not a load flake' on a determinism claim - is gone, replaced by "
        "VERIFIER-HOST-CONTROL-BUDGET's proposed text making it a SIXTH load-sensitive row sensitive "
        "at the TOP of the range. This lane's own contribution is preserved in the replacement: what "
        "the old paragraph got right is kept and what it got wrong is named, including that "
        "'deterministically 3/3 in isolation' was three samples of a biased coin (22 repetitions gave "
        "7 pass / 15 fail). The lane's starvation HYPOTHESIS was refuted by its sibling card and the "
        "residual is amended in place to say so - see the hand_sweep section.",
        "AGENTS.md constraint 16, 'The sixth, landed at the wave-35 close ...'; "
        "docs/reviews/r0_3/acceptance_matrix.json HOST-ROW-STARVATION-DESCRIPTION, amended title and "
        "why.",
    ),
    "ct-ctwin-lane#for-the-integrator#13": v(
        N,
        "'Stated here so a later card does not have to rediscover it' - the item is explicitly "
        "FORWARD-LOOKING and asks for nothing now. Its three coupled edits (the "
        "checked_mutex_crossing stage plan, CT46's four manifest rows, run_mutex_crossing's "
        "is_none() assertion) would land in ONE change or the harness refuses. The close's "
        "contribution is to make sure the instruction is findable: MUTEX-SHARE-CROSS-EMISSION is now "
        "registered in named_followups and its entry names this section as the statement of the "
        "three coupled edits.",
        "docs/reviews/r0_3/acceptance_matrix.json, MUTEX-SHARE-CROSS-EMISSION.",
    ),
    "ct-ctwin-lane#for-the-integrator#14": v(
        N,
        "Conditional item - 'update if the CT headline appears there'. MEASURED: `grep -n "
        "'45/50\\|CT01-CT50\\|CT coverage' README.md` returns nothing, so the CT headline does not "
        "appear in README.md and the condition does not fire. No edit owed, and the sync rule "
        "(AGENTS.md: a new increment 'leaves README.md alone') agrees.",
        "README.md - no CT headline present.",
    ),
    "ct-ctwin-lane#for-the-integrator#15": v(
        N,
        "A '---' horizontal rule, extracted as a contentless item. This is "
        "SWEEP-SEPARATOR-ITEM-ARTIFACT, the known extractor artifact card SWEEP-BLIND-SPOTS recorded "
        "rather than suppressed, on the ground that filtering it would mean the extractor deciding "
        "what is and is not content. Given a verdict for the same reason it was given one in the "
        "wave-32 record: the artifact is cheaper than the judgment.",
    ),
    # ------------------------------------------------------------------ #
    # JOINED-PROVENANCE-CYCLES (wave 35, arbiter #2) - 10 items
    # ------------------------------------------------------------------ #
    "joined-provenance-cycles#for-the-integrator#1": v(
        N,
        "Preamble, 'Apply in these words.' States the discipline for items 2-10.",
    ),
    "joined-provenance-cycles#for-the-integrator#2": v(
        A,
        "Applied in the card's own commit and verified standing at the close, with the card's own "
        "request honoured: 'please re-read both edits against this report before merging, since they "
        "were written while arbiter 1 was in flight.' Re-read. The baseline, the description, the "
        "chain order and the 'six' both check out.",
        "AGENTS.md:108-112.",
    ),
    "joined-provenance-cycles#for-the-integrator#3": v(
        A,
        "Verified against the card's report, as it asked. The four counts stand exactly as arbiter "
        "#1 left them - 272 / 4,214 / 50,106 / 0 and 186 Safe / 84 provisional Unsafe / 4,564 "
        "Unknown - and the zero-movement sentence with its 40 exercised functions and 14 blocked "
        "sites is present. A SOUND EXTENSION THAT MOVES NOTHING is the strongest form this evidence "
        "can take and the easiest to misread as trivial; the row does not misread it.",
        "AGENTS.md, Cycle 19 raw corpus row.",
    ),
    "joined-provenance-cycles#for-the-integrator#4": v(
        A,
        "Applied in the card's exact words, both halves: the replacement sentence is in and the "
        "later sentence 'and cyclic CFGs derive no facts' is deleted. Checked in both directions - "
        "`grep -c 'over acyclic CFGs only' AGENTS.md` is 0, and 10b's every other sentence stands "
        "unchanged including the input_chunk_get one the card re-measured and confirmed. This is "
        "also the item that made arbiter #1's proposed 5c stale; see "
        "c-gateway-constant-do-while#12.",
        "AGENTS.md, constraint 10b, 'An **acyclic** CFG is solved in one topological pass; a "
        "**cyclic** CFG is solved by the greatest fixed point of the same transfer'. Corroborated in "
        "source at crates/qcore-c-gateway/src/legacy_joined_provenance.rs:151.",
    ),
    "joined-provenance-cycles#for-the-integrator#5": v(
        N,
        "A statement that no diagnostic action is owed. Verified rather than accepted: the card "
        "emits no new diagnostic, allocates no code and widens no band, so the Q-NAMESPACE-DECISION "
        "condition-extension rule is not engaged. Correctly distinguished from the thirteen cards "
        "that ARE instances of it.",
    ),
    "joined-provenance-cycles#for-the-integrator#6": v(
        A,
        "Landed in wave 35's entry, and the claim was checked rather than taken: the card reserves "
        "no word and does not move the lexer.",
        "AGENTS.md, constraint 14, 'Cards that do not engage this constraint at all', wave 35's "
        "seven - JOINED-PROVENANCE-CYCLES named.",
    ),
    "joined-provenance-cycles#for-the-integrator#7": v(
        A,
        "Present, newest-first, at the head of the file until this close's own paragraph was "
        "prepended above it.",
        "docs/development/increments/wave-35.md.",
    ),
    "joined-provenance-cycles#for-the-integrator#8": v(
        A,
        "All three registered and present, and the item's care is worth recording: it states exactly "
        "what moved rather than asserting the matrix is fine, and every claim checks out. The counts "
        "it quotes (95 -> 98) are its own worktree's; the tip carried 103 after the integrator's "
        "merges, and this close leaves 121.",
        "docs/reviews/r0_3/acceptance_matrix.json: JOINED-PROVENANCE-CYCLE-SITES, "
        "JOINED-PROVENANCE-RELEASE-NULLABLE, JOINED-PROVENANCE-FACTS-NAME-NOTHING all present; "
        "AuditJoinedProvenanceCycles.lean in the derived lean_audit_files.",
    ),
    "joined-provenance-cycles#for-the-integrator#9": v(
        A,
        "Accepted as the discharge, and the close acts on it by NOT registering the residual - see "
        "c-gateway-constant-do-while#16. The discharge is measured against arbiter #1's stated "
        "condition clause by clause: coverage quoted against 148eef79 rather than a300a39d (yes), "
        "the 654 reproduced independently (yes, 1,927 - 654 = 1,273), and whether any of the 654 "
        "changes an answer (yes - stated, and it is zero, with the cost quantified at six functions "
        "and 20 leaves of servable population moving to the topological pass). Retiring it in this "
        "card's record rather than by editing arbiter #1's is constraint 21 working.",
        "Not registered in named_followups, deliberately; recorded here and in "
        "docs/reports/wave35-close/README.md.",
    ),
    "joined-provenance-cycles#for-the-integrator#10": v(
        N,
        "A boundary statement carrying no action, and the close repeats it rather than letting it "
        "pass: a sound extension that moves nothing is evidence about the route's premises, not new "
        "conformance. Nothing in wave 35 promotes any unit or function, and the corpus's Unsafe "
        "findings remain provisional and subject to their reports' boundaries (invariant 2).",
    ),
    # ------------------------------------------------------------------ #
    # MUTEX-SHARE-CROSS-ARMS (wave 35) - 11 items
    # ------------------------------------------------------------------ #
    "mutex-share-cross-arms#for-the-integrator#1": v(
        S,
        "The claim held - seven workspace tests and one Lean audit added, nothing else edited - but "
        "the FIGURE it proposes is superseded by the close's own measurement. The card asked for the "
        "Lean row to go 129 -> 130; the wave added THREE audits, so the row reads 132 as a single "
        "re-measured figure at 4cf9bdf9. Same reason as "
        "c-gateway-constant-do-while#7: a per-card delta to a row that the close re-measures whole "
        "is superseded rather than applied.",
        "AGENTS.md, Merged gate row: 132 Lean audits. AuditMutexShareCrossArms.lean present in the "
        "matrix's derived lean_audit_files.",
    ),
    "mutex-share-cross-arms#for-the-integrator#2": v(
        A,
        "Applied in the place the item asked for - 'if the ledger or any index quotes that sentence, "
        "it should point here instead' - and the frozen archive is NOT edited (constraint 21). The "
        "superseding sentence is written into the new standing constraint 15o, which is where the "
        "ledger now carries this card's rules, and it says in the card's own terms that four arms "
        "are now two driven and two proved unreachable.",
        "AGENTS.md, constraint 15o, closing paragraph: 'Wave 35's `MUTEX-SHARE-CROSS-ARMS` "
        "supersedes one sentence of that card's report from its own record (constraint 21, not by "
        "editing it)'.",
    ),
    "mutex-share-cross-arms#for-the-integrator#3": v(
        A,
        "APPLIED BY THIS CLOSE, AND THE HALF THAT WAS MISSING IS THE POINT. The card's arithmetic is "
        "explicit - 'one replaced, four added' - and at the close's base the FOUR ADDITIONS AND THE "
        "SUCCESSOR WERE ALL PRESENT WHILE THE REPLACEMENT'S REMOVAL WAS NOT: "
        "MUTEX-SHARE-CROSS-UNEXERCISED-ARMS still stood beside MUTEX-SHARE-CROSS-ARM-C-REVIVAL, "
        "whose own `why` opens by stating it discharged the entry sitting next to it. So the ledger "
        "carried a residual and its retirement notice simultaneously. This is the third instance in "
        "three waves of a REMOVAL from this hand-maintained section being dropped at a merge "
        "resolution (8d158c4e this wave, 3bbec4d0 in wave 32), and it is a pattern worth naming: "
        "additions survive a union merge and removals do not.",
        "docs/reviews/r0_3/acceptance_matrix.json: MUTEX-SHARE-CROSS-UNEXERCISED-ARMS retired by "
        "docs/reports/wave35-close/scripts/fix_lost_matrix_edits.py, its discharge recorded in the "
        "close's README; the three successors remain.",
    ),
    "mutex-share-cross-arms#for-the-integrator#4": v(
        A,
        "The sharpening is carried into the standing constraint rather than left in the report, "
        "because it is the part a later reader would otherwise get wrong. 15o records that the two "
        "dead arms are NOT dead for the same reason - arm C's admitting protocol is a literal, so "
        "its residue is empty for every kind independent of the only-pair property, while arm D does "
        "rest on that property - which the original residual treated as one fact.",
        "AGENTS.md, constraint 15o, closing paragraph.",
    ),
    "mutex-share-cross-arms#for-the-integrator#5": v(
        N,
        "'A seam, noted and not taken.' No action requested and none owed; the close confirms the "
        "seam is still open and untouched - TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED remains "
        "registered, constraint 15n is unedited this wave, and no card of wave 35 grew the F02 "
        "observable alphabet. Recorded so that a later STABILITY-REEVAL can see the seam was "
        "consciously left rather than missed.",
    ),
    "mutex-share-cross-arms#for-the-integrator#6": v(
        A,
        "Landed in wave 35's entry, claim checked: the card reserves no word and does not move the "
        "lexer.",
        "AGENTS.md, constraint 14, 'Cards that do not engage this constraint at all', wave 35's "
        "seven - MUTEX-SHARE-CROSS-ARMS named.",
    ),
    "mutex-share-cross-arms#for-the-integrator#7": v(
        A,
        "Landed, as a new bullet immediately after the measurement-outputs rule, exactly where the "
        "item asked.",
        "CLAUDE.md, 'Worktree and measurement hygiene', the bullet beginning '**A mutation harness "
        "restores by rewriting, never by an mtime-preserving copy.**'",
    ),
    "mutex-share-cross-arms#for-the-integrator#8": v(
        A,
        "The block quote landed WORD FOR WORD, including the sentence that makes it a rule rather "
        "than a tip - that the same mistake in the discharging direction produces a false green. "
        "Nothing was softened.",
        "CLAUDE.md, 'Worktree and measurement hygiene'.",
    ),
    "mutex-share-cross-arms#for-the-integrator#9": v(
        F,
        "DEFERRED. The rule is landed (item 8); the SECOND READER the item asks for - an audit of "
        "docs/reports/c-gateway-return-call-w4/scripts/mutations.py, the other mutation harness in "
        "the tree, which this card did not examine - is NOT done and is not claimed. GROUND: reading "
        "a second card's harness for a restore-semantics defect is a measurement, and a close that "
        "performed it would be doing card work inside a reconciliation. The deferral is made VISIBLE "
        "rather than left in this file: CLAUDE.md's new bullet says in its own text that the second "
        "reader is outstanding and names MATURATION via MUTATION-HARNESS-MTIME-RESTORE, which is "
        "registered.",
        "CLAUDE.md, the new bullet's closing sentence; "
        "docs/reviews/r0_3/acceptance_matrix.json, MUTATION-HARNESS-MTIME-RESTORE.",
    ),
    "mutex-share-cross-arms#for-the-integrator#10": v(
        A,
        "Folded in, as the item asked and as the wave-34 close had already said the integrator "
        "should do. This card's three-way observation - red in isolation, red under a full suite AND "
        "green under a full suite, on one host in one session - is now part of constraint 16's "
        "replacement paragraph and of the retirement note for "
        "VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED, where it is the second independent refutation of "
        "that entry's 'deterministic' claim. The card was right not to edit another card's entry and "
        "right to hand the data over.",
        "AGENTS.md constraint 16; docs/reviews/r0_3/acceptance_matrix.json, the retirement recorded "
        "in docs/reports/wave35-close/scripts/reconcile_followups.py's RETIRE note.",
    ),
    "mutex-share-cross-arms#for-the-integrator#11": v(
        N,
        "A scoping statement, verified: both names are present in named_followups and neither is "
        "claimed repaired beyond this card's own harness. MUTATION-HARNESS-MTIME-RESTORE is now also "
        "carrying the deferred second-reader audit (item 9), which is a widening of its scope "
        "recorded here rather than silently.",
        "docs/reviews/r0_3/acceptance_matrix.json: MUTATION-HARNESS-MTIME-RESTORE and "
        "PYTHON-AUDIT-COMMAND-ROW-LOAD-SENSITIVE both present.",
    ),
    # ------------------------------------------------------------------ #
    # SWEEP-BLIND-SPOTS (wave 35) - 16 items
    # ------------------------------------------------------------------ #
    "sweep-blind-spots#for-the-integrator#1": v(
        N,
        "Preamble: the items below are ones the card cannot land itself because AGENTS.md is the "
        "integrator's, and CLAUDE.md requires an amendment to land in the card's words or be "
        "declined with a ground. Correct, and the discipline this whole record applies.",
    ),
    "sweep-blind-spots#for-the-integrator#2": v(
        N,
        "Section heading '### 6.1 MUTEX-SHARE-CROSS's proposed standing constraint, verbatim'. "
        "Content is items 3 and 4.",
    ),
    "sweep-blind-spots#for-the-integrator#3": v(
        A,
        "LANDED, three waves late, which is the finding. The card's measurement was reproduced at "
        "the close before acting on it: `grep -c 'MUTEX-SHARE-CROSS' AGENTS.md` returned 0 at the "
        "base, while docs/QCORE_SPEC_r0_3.md:1004 - the authority AGENTS.md calls its own table 'a "
        "summary of, not a second allocation' - carried the widening in full. The summary had "
        "drifted from its authority for one card.",
        "AGENTS.md, standing constraint 15o.",
    ),
    "sweep-blind-spots#for-the-integrator#4": v(
        A,
        "The block quote landed word for word, including its own opening '**(card "
        "`MUTEX-SHARE-CROSS`.)**' and its closing '`mutex-share-cross`.' citation. ONE THING IS THE "
        "INTEGRATOR'S AND IS LABELLED AS SUCH IN THE FILE: the constraint NUMBER. The card left it "
        "unassigned; 15a-15n were taken, so it is 15o under 'Concurrency source routes', and the "
        "paragraph says in its own body that the number is the integrator's. The card's text itself "
        "is unaltered.",
        "AGENTS.md, constraint 15o, whose second paragraph records the landing, its lateness and the "
        "number's provenance.",
    ),
    "sweep-blind-spots#for-the-integrator#5": v(
        N,
        "Section heading '### 6.2 Three AGENTS.md rows the same card asked for'. Content is items "
        "6-8.",
    ),
    "sweep-blind-spots#for-the-integrator#6": v(
        A,
        "Applied, following the authority as the item asks. The row read 'Four later cards move this "
        "row' and named four; it now reads FIVE and names MUTEX-SHARE-CROSS as the fifth, with its "
        "two protocol gates (absent, mismatched) at Q600 at the clause and the third refusal routed "
        "to Q611, summarising spec §L21 rather than re-allocating anything.",
        "AGENTS.md, the `Q600` diagnostics-namespace row.",
    ),
    "sweep-blind-spots#for-the-integrator#7": v(
        A,
        "Applied. The row was one unchanged line and now records that Q611 UnguardedProtectedAccess "
        "is reached from a NEW ROUTE - the declared crossing's third refusal - under the unedited "
        "checker's own code rather than at Q600, because a protected footprint has no unguarded "
        "access at all. No code allocated; a route recorded.",
        "AGENTS.md, the `Q610`-`Q627` diagnostics-namespace row.",
    ),
    "sweep-blind-spots#for-the-integrator#8": v(
        A,
        "BOTH ARMS OF THE CARD'S DISJUNCTION TAKEN, DELIBERATELY, BECAUSE THEY ARE NOT "
        "INTERCHANGEABLE - and the ground is recorded in AGENTS.md itself, not only here. The card "
        "offered: add MUTEX-SHARE-CROSS, PROOF-LEG-BATCH and DOCS-COMPACT-5 to the wave-32 "
        "enumeration, OR narrow 'every card of waves 32, 33 and 34'. Taking arm (a) alone would have "
        "been WRONG for one of the three: MUTEX-SHARE-CROSS DOES engage constraint 14 - its report "
        "§9 is headed 'Keyword census (standing constraint 14)' and §13 asks for 'this card's +0 / "
        "+0 / +0 row' in the census ledger - so listing it among cards that 'do not engage this "
        "constraint at all' would replace a false universal with a false member. So: the quantifier "
        "is narrowed, PROOF-LEG-BATCH and DOCS-COMPACT-5 join the non-engaging list, and "
        "MUTEX-SHARE-CROSS gets the census row it actually asked for, with its measured figures (490 "
        "files, 33 hits, 8 in code, 0 occurrences of `under` in code anywhere).",
        "AGENTS.md, constraint 14: the new '`MUTEX-SHARE-CROSS` **+0 reserved words, +0 tokens, +0 "
        "name-resolution exclusions**' bullet in the census family, and the rewritten 'Cards that do "
        "not engage this constraint at all' bullet, which states the false universal and why both "
        "arms were taken.",
    ),
    "sweep-blind-spots#for-the-integrator#9": v(
        N,
        "Section heading '### 6.3 The lost matrix edit'. Content is item 10.",
    ),
    "sweep-blind-spots#for-the-integrator#10": v(
        A,
        "Applied, taking the SECOND arm of the disjunction: DOCS-COMPACT-4-TABLE-SYNC is registered "
        "as its own named_followups entry. GROUND FOR THE CHOICE: restoring the lost prose to the "
        "DOCS-COMPACT-O row would put a description of one card's finding inside another card's row "
        "while leaving the NAME untracked, which is the defect rather than its repair. Its sibling "
        "DOCS-COMPACT-5-RECONCILIATION-LIST is deliberately NOT registered - CLAUDE.md records it as "
        "already discharged by W32-RECONCILE-SWEEP, so registering it would put closed work in the "
        "open-work ledger. W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE is retired accordingly. Its SECOND "
        "question - whether a close should re-verify the `where` of verdicts an earlier close "
        "recorded - is answered by this close DOING it for all 78 items, which is what found "
        "CT-CTWIN-LANE's unapplied row, the lost MUTEX-SHARE-CROSS-UNEXERCISED-ARMS retirement and "
        "5c's stale clause. The card was right not to do this itself (e0052b14 precedent: "
        "named_followups prose for another card's row is integrator-owned).",
        "docs/reviews/r0_3/acceptance_matrix.json: DOCS-COMPACT-4-TABLE-SYNC added, "
        "W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE removed, by "
        "docs/reports/wave35-close/scripts/fix_lost_matrix_edits.py.",
    ),
    "sweep-blind-spots#for-the-integrator#11": v(
        N,
        "Section heading '### 6.4 Wave 34 has no close record'. Content is item 12.",
    ),
    "sweep-blind-spots#for-the-integrator#12": v(
        A,
        "Discharged - by another card of the SAME WAVE, merging after this one. WAVE-34-CLOSE landed "
        "at aba64a7a and did exactly what the item names: all 25 integrator-addressed items of "
        "stability-reeval-1, c-gateway-return-call-w4 and docs-compact-6 carry verdicts. VERIFIED AT "
        "THIS CLOSE rather than assumed: docs/reports/wave34-close/ now holds README.md, gate.json "
        "and reconciliation.json, and that record contains 25 verdicts over those three cards. It "
        "found one unapplied item - four residuals of C-GATEWAY-RETURN-CALL-W4 that DOCS-COMPACT-6 "
        "did not carry when it merged afterwards - so the sweep earned its keep there too. "
        "WAVE-34-CLOSE-UNSWEPT retired.",
        "docs/reports/wave34-close/reconciliation.json (25 verdicts); retirement in "
        "docs/reports/wave35-close/scripts/reconcile_followups.py.",
    ),
    "sweep-blind-spots#for-the-integrator#13": v(
        N,
        "Section heading '### 6.5 Retirement candidates'. Content is item 14.",
    ),
    "sweep-blind-spots#for-the-integrator#14": v(
        A,
        "All five retired, each against evidence measured at this close rather than against the "
        "card's claim. WAVE-32-CLOSE-UNSWEPT: docs/reports/wave32-close/reconciliation.json now "
        "exists with 67 verdicts over all seven of wave 32's cards, where before it held only "
        "gate.json. SWEEP-RESIDUALS-SCANS-REPORTS-ONLY: the widened scan returned 62 names including "
        "whole families from waves 29-30 increments the old scan could not reach. "
        "RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK: 11 of the 62 came back annotated "
        "'mentioned in AGENTS.md - a RULE, not a tracker', and every one proved to be a CARD NAME "
        "rather than open work, which is the distinction the repair exists to force. "
        "WAVE-BASE-UNSWEPT: --uncarded returned four commits and the wave-open commit a300a39d was "
        "among them, the exact commit the old exclusive range could not see. "
        "WAVE-33-DEFERRED-DECISIONS-UNTRACKED: --check re-listed all seven deferred decisions and "
        "this record restates each. The card was right not to retire them itself.",
        "docs/reviews/r0_3/acceptance_matrix.json, five entries removed by "
        "docs/reports/wave35-close/scripts/reconcile_followups.py, each with its discharge recorded "
        "in that script and in the close's README.",
    ),
    "sweep-blind-spots#for-the-integrator#15": v(
        N,
        "Section heading '### 6.6 A note on the brief'. Content is item 16.",
    ),
    "sweep-blind-spots#for-the-integrator#16": v(
        A,
        "The correction propagated, which is what the item asked for ('worth correcting in whatever "
        "template the brief came from, since the wrong path would send the next card hunting'). The "
        "wave-35 close's own brief carries the corrected generator path, and this close used "
        "`docs/reports/remaining-d02/build_matrix.py` throughout; `scripts/build_matrix.py` was "
        "never invoked and does not exist. Card VERIFIER-HOST-CONTROL-BUDGET's item 1 independently "
        "confirms it received the correction mid-card and did not invent a path.",
        "docs/reports/wave35-close/scripts/*.py and the generator invocations recorded in this "
        "close's README all name docs/reports/remaining-d02/build_matrix.py.",
    ),
    # ------------------------------------------------------------------ #
    # VERIFIER-HOST-CONTROL-BUDGET (wave 35) - 7 items
    # ------------------------------------------------------------------ #
    "verifier-host-control-budget#for-the-integrator#1": v(
        A,
        "Both flipped, as the item says they are the integrator's to flip. "
        "VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED is retired on discharge option (c) - an evidenced "
        "investigation PLUS the repair it justified, which is stronger than the option (a) "
        "documentation arm the wave-34 close left open - and FB-PLAN-GATE-HOST-TIMEOUT with it, "
        "exactly as its own text requires ('this entry closes when that one does'). The retirement "
        "note carries the mechanism (8 KiB pipe capacity against a 1 ms backoff quantum), the repair "
        "(poll(2), test file untouched), the 38/38, and BOTH corrections to the retired entry's "
        "determinism claim. Five successors registered by this close carry what stays open.",
        "docs/reviews/r0_3/acceptance_matrix.json: both entries removed, "
        "VERIFIER-HOST-CONTROL-MARGIN-UNPINNED, HOST-PIPE-CAPACITY-UNEXPLAINED, "
        "HOST-BACKOFF-FALLBACK-ARMS-UNDRIVEN, VERIFIER-HOST-READINESS-APPLE-UNRUN and "
        "VERIFIER-HOST-EARLY-ARM-OVERLOAD added.",
    ),
    "verifier-host-control-budget#for-the-integrator#2": v(
        A,
        "Carried into the replacement text, as the item asks ('worth carrying into whatever text "
        "replaces it'). The correction is stated in AGENTS.md in the strong form: "
        "'deterministically 3/3 in isolation' was THREE SAMPLES OF A BIASED COIN, against 22 "
        "pre-repair isolated repetitions giving 7 pass / 15 fail - and the entry's CONCLUSION held "
        "while its determinism claim did not, which is the distinction the item draws. Card "
        "MUTEX-SHARE-CROSS-ARMS' independent three-way observation is folded in beside it.",
        "AGENTS.md constraint 16, the paragraph beginning 'This replaces, rather than re-issues, the "
        "wave-34 paragraph'.",
    ),
    "verifier-host-control-budget#for-the-integrator#3": v(
        A,
        "Landed as written. The count at the head of constraint 16 moves five -> six to match.",
        "AGENTS.md, constraint 16: '**six** provider/host rows are load- or parallelism-sensitive' "
        "and '**The sixth, landed at the wave-35 close in card `VERIFIER-HOST-CONTROL-BUDGET`'s own "
        "words:**'.",
    ),
    "verifier-host-control-budget#for-the-integrator#4": v(
        A,
        "The block quote landed VERBATIM, including its closing instruction 'do not re-file that "
        "half here' - which is the sentence that keeps the repaired large-response half from being "
        "re-recorded as a standing load sensitivity it no longer is. Nothing softened.",
        "AGENTS.md, constraint 16, the block quote.",
    ),
    "verifier-host-control-budget#for-the-integrator#5": v(
        A,
        "Reconciled by re-measurement rather than by carrying the card's number, which is the "
        "stronger form. The card correctly left AGENTS.md alone per its brief; the close measured "
        "the whole gate at 4cf9bdf9 and the Merged gate row now carries 4,580 / 0 / 8 with every "
        "other figure from the same run.",
        "AGENTS.md, Merged gate row; docs/reports/wave35-close/gate.json.",
    ),
    "verifier-host-control-budget#for-the-integrator#6": v(
        N,
        "MEASURED, not taken on trust, because it is the load-bearing claim of the whole card - a "
        "repair that had touched the test would be indistinguishable from weakening the assertion "
        "(invariant 1). `git diff --stat cc7dbe78^ cc7dbe78` returns exactly three paths: "
        "crates/qcore-verifier-host/src/lib.rs (+91/-6), the card's own README and the increment "
        "record. crates/qcore-verifier-host/tests/standalone_host.rs is NOT among them.",
        "git diff --stat cc7dbe78^ cc7dbe78.",
    ),
    "verifier-host-control-budget#for-the-integrator#7": v(
        N,
        "Confirmed at the close: nothing under measure/ is committed anywhere in the wave, the path "
        "being gitignored per CLAUDE.md's measurement-hygiene rule. This close follows the same rule "
        "- its own gate logs live in measure/ and its committed scripts live under "
        "docs/reports/wave35-close/scripts/.",
    ),
}

RECORD = {
    "wave": 35,
    "cards": [
        "sweep-blind-spots",
        "wave34-close",
        "verifier-host-control-budget",
        "ct-ctwin-lane",
        "c-gateway-constant-do-while",
        "mutex-share-cross-arms",
        "joined-provenance-cycles",
    ],
    "note": (
        "Wave-35 close reconciliation. SEVEN merged cards and TWO ARBITERS, swept with "
        "`--since a300a39d` - the FIRST close to use card SWEEP-BLIND-SPOTS' repaired tool, whose "
        "--since is now inclusive of its own base. 78 integrator-addressed items across 7 cards, "
        "every one given a verdict MEASURED AGAINST THE TREE - the target file grepped for the "
        "proposed text or its effect - and never inferred from a report's own claim that it had "
        "landed. THAT DISTINCTION FOUND FOUR THINGS THIS WAVE, three of them real losses. (1) "
        "CT-CTWIN-LANE's ENTIRE §9 item 1 was unapplied: at the close's base `grep -c CT46 "
        "AGENTS.md` returned 0 and the ledger still read '45/50 mapped, 5 unmapped' although the "
        "card merged at cfd410d0 with three further cards merging after it. Landed here. (2) "
        "MUTEX-SHARE-CROSS-ARMS asked for 'one replaced, four added' in named_followups; the four "
        "additions and the successor all landed and THE REMOVAL DID NOT, so the ledger carried "
        "MUTEX-SHARE-CROSS-UNEXERCISED-ARMS beside the successor entry whose own text says it "
        "discharged it. Retired here. That is the THIRD instance in three waves of a REMOVAL from "
        "this hand-maintained section being dropped at a merge resolution (8d158c4e this wave, "
        "3bbec4d0 in wave 32): additions survive a union merge and removals do not. (3) "
        "MUTEX-SHARE-CROSS's whole integrator list was still unlanded three waves on, exactly as "
        "SWEEP-BLIND-SPOTS measured - `grep -c MUTEX-SHARE-CROSS AGENTS.md` returned 0 while spec "
        "§L21 carried the widening in full. Its standing constraint is landed verbatim as 15o and "
        "both namespace rows are updated. (4) Arbiter #1's proposed constraint 5c contained a clause "
        "that arbiter #2 FALSIFIED LATER IN THE SAME WAVE, so it could not land in the words "
        "proposed; it is recorded `superseded` with both before-and-after texts, and AGENTS.md's own "
        "5c says so in its body. ONE AMENDMENT IS DECLINED, taking the card's own offered "
        "alternative: C-GATEWAY-CONSTANT-DO-WHILE's multi-tip Lean figure, because the close "
        "RE-MEASURED the whole merged-gate row at its own base, which makes the row one measurement "
        "of one tree rather than a carried figure with a parenthetical - and the 130 it proposed was "
        "itself two tips stale, the wave having added three audits. TWO ITEMS ARE DEFERRED with "
        "grounds: an owner for CT-FIXTURE-CROSS-CASE-DUPLICATES (work allocation is not a close's), "
        "and the second reader on the other mutation harness (a measurement, not a reconciliation). "
        "THE 62 RESIDUAL-SHAPED NAMES the widened --residuals returned were classified ONE BY ONE, "
        "not in bulk: 27 registered as open work with discharge conditions, 22 recorded as CARD "
        "NAMES, 6 as discharged by a named card, 3 as decisions already carried in "
        "deferred_decisions, 1 as a bucket LABEL, and 3 as rules or prospective names that are not "
        "open work. named_followups moves 103 -> 121 (28 added, 10 retired), derived and confirmed a "
        "byte-identical generator fixed point with test_build_matrix_sections.py green at 17/17. THE "
        "STANDING GATE WAS MEASURED AND PASSES at 4cf9bdf9: 4,580 passed / 0 failed / 8 ignored over "
        "230 result summaries, 132 Lean audits, 247 Python, both probes exit 0, fmt and clippy "
        "clean, every exit captured with no pipe. NOTHING IS PROMOTED: no unit or function status "
        "moved at either arbiter, the contract corpus is unchanged at 76, CT coverage grew by "
        "MAPPING with both numerators unmoved, and full language conformance stays false. The sweep "
        "tool decides nothing; every verdict here is a human one."
    ),
    "verdicts": VERDICTS,
    "no_section": {
        "wave34-close": (
            "Swept and found to carry no integrator-addressed section, correctly. A WAVE CLOSE IS "
            "THE INTEGRATOR: it addresses the next close only through deferred_decisions, which is a "
            "separate mechanism the tool re-lists on its own (and did - all seven of this card's "
            "decisions are restated in this record's deferred_decisions block). Checked both ways "
            "rather than assumed: docs/reports/wave34-close/README.md was read and contains no "
            "'For the integrator', 'Integrator reconciliation', 'Integrator carry list' or 'Proposed "
            "amendment' heading, so the sweep's silence is the document's shape and not an "
            "unrecognised spelling. What that close DID owe forward is carried in its own "
            "reconciliation.json (25 verdicts) and in its deferred_decisions, both of which this "
            "close read and acted on."
        )
    },
    "hand_sweep": {
        "note": (
            "What the tool cannot see, measured by hand. The tool's own boundaries are honest and "
            "documented (SWEEP-UNCARDED-RULE-APPROXIMATE, SWEEP-SEPARATOR-ITEM-ARTIFACT, and its "
            "standing statement that a green --check means a verdict was RECORDED and never that an "
            "item was APPLIED), so a close that ran only the tool would have missed everything "
            "below."
        ),
        "items": {
            "uncarded_commits": (
                "`--since a300a39d --uncarded` returned FOUR commits in the range that write no card "
                "report. Each examined by eye and ACCOUNTED FOR: a300a39d 'Open wave 35' is the "
                "wave-open commit that creates the increment record and moves the index pointer "
                "before any card launches - the commit the OLD exclusive range could not see at all, "
                "which is WAVE-BASE-UNSWEPT's live instance now visible; b6ea42df and 8d158c4e are "
                "the two integrator corrections (a pre-existing --since HEAD test rewritten to "
                "--range HEAD..HEAD under the new inclusive base, and the restoration of "
                "PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW lost at the wave-34 close's merge); b954b753 "
                "is MUTEX-SHARE-CROSS-ARMS' own code commit, whose report lands in a later commit of "
                "the same card. NONE needs a card. The tool's caveat is right that writing no report "
                "is necessary and not sufficient for being uncarded, and three of these four are the "
                "'card's own code commit' and 'integrator correction' shapes it warns about."
            ),
            "host_row_two_cards_one_row": (
                "RECONCILED, and this is the item the tool could not have posed because it needs two "
                "cards read against each other. CT-CTWIN-LANE and VERIFIER-HOST-CONTROL-BUDGET both "
                "described `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` "
                "in ONE WAVE and neither could see the other. The lane proposed HOST STARVATION as "
                "the likelier account, explicitly not vouching for it; the other card then MEASURED "
                "the mechanism and it was a TRANSPORT DEFECT - the row failed when the host was LEAST "
                "loaded, the opposite sign from starvation - an 8 KiB pipe capacity against a 1 ms "
                "backoff quantum, repaired with poll(2). The lane's caution was well placed and its "
                "conclusion was wrong. HOST-ROW-STARVATION-DESCRIPTION is therefore AMENDED IN PLACE "
                "rather than retired, because its two discharge arms have different fates: the first "
                "('replace the standing description') IS discharged, in constraint 16, in the other "
                "card's own words; the second ('re-measure the row on a quiet machine') IS NOT - NO "
                "UNLOADED-HOST NUMBER EXISTS ANYWHERE IN THE TREE. The 38/38 was measured on the same "
                "busy host, this close's own green observation is a full-suite run at load ~6.5, and "
                "VERIFIER-HOST-NPROC-ROW-UNVERIFIABLE-UNDER-WAVE records that a quiet account was not "
                "reachable during wave 35 at all. The entry is narrowed to exactly that one open "
                "thing."
            ),
            "sixty_two_residual_names": (
                "The widened --residuals returned 62 names the tracker does not carry - the first "
                "honest count, NOT a wave-35 regression, since the scan now reaches "
                "docs/development/increments/ and the standing-constraints text and counts only "
                "named_followups as a tracker. All 62 classified individually. 27 REGISTERED as open "
                "work, each with a discharge condition drawn from its own source report: the five "
                "live CONSTANT-DO-WHILE-*, the six VERIFIER-HOST/HOST-* from this wave's host card, "
                "the three MUTEX-SHARE-CROSS-*, the five CT42-*, the three CT-COMPOSED-NATIVE-*, the "
                "two CT-PROOF-*, plus W31-SIZE-SIBLINGS-REMAINDER, WRITE-LEND-RELEND and "
                "RECONCILE-SWEEP-DIMENSIONS-UNPINNED. 22 are CARD NAMES, not residuals (14 with an "
                "exactly-matching docs/reports/ directory; CT46-SENDSYNC and EVENT-V1-WINDOWS-AFUNIX "
                "have directories under other names; DOCS-COMPACT-1 is a past card, DOCS-COMPACT-7 a "
                "future one, and the four FB-* are QUEUED cards from the Fluent Bit plan, which the "
                "wave-34 close had already classified the same way). 6 are DISCHARGED residuals with "
                "the discharging card named: CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH by "
                "JOINED-PROVENANCE-CYCLES, W29-CONTENT-SIZE-UNDECLARED and "
                "W29-SIZE-SIBLINGS-UNDECLARED by W31-CORPUS-CONTRACTS-2 (the latter partly, its "
                "remainder registered here as W31-SIZE-SIBLINGS-REMAINDER), W4-RECSUM-CONSTRAINT-4 "
                "and DOCS-COMPACT-5-MIGRATED-SECTION-CLAUSE by W32-RECONCILE-SWEEP, and "
                "CT22-CT-MAPPING by CT22-WIRE. 3 are DECISIONS already carried in this record's "
                "deferred_decisions (VERIFIER-HOST-ROW-DISPOSITION, "
                "PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED, "
                "DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK) and must not be registered as followups "
                "as well. 1 is a LABEL: DECIDABLE-NOW is card T00's unmapped-reason bucket and is "
                "already a KEY of the derived newly_decidable_not_mapped_reasons map. 3 are RULES OR "
                "PROSPECTIVE NAMES rather than open work: CT24-CLEANUP-TASK-COMPOSITION is "
                "explicitly the name a FUTURE card WOULD have, DOCS-COMPACT-5-EXAMPLES-HOST-GATE was "
                "marked STANDING and deliberately not reworded by W32-RECONCILE-SWEEP, and "
                "W4-RECSUM-ALREADY-LANDED records a stale brief that its own card then verified. 22 "
                "+ 27 + 6 + 3 + 1 + 3 = 62."
            ),
            "arbiter_findings_named_but_unregistered": (
                "The two findings arbiter #1's report NAMES but did not register both reached the "
                "ledger, which is exactly the shape --residuals exists to catch and it did catch "
                "them: CONSTANT-DO-WHILE-SUCCESSOR-ORDER (CFG successor order is observable in the "
                "worklist schedule - a mutation CONTROL expected to be harmless was CAUGHT, the same "
                "scheduling sensitivity constraint 6 records for report_leaks) and "
                "CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED (a pre-existing route decides a do-while "
                "from a modelled VALUE where this card decides from a TOKEN; it has no card, no Lean "
                "and no report, and is pinned only by one test this card happened to write). Both "
                "registered."
            ),
            "arbiter_2_residuals": (
                "Arbiter #2's three residuals were already registered by the card itself and were "
                "verified present at the close rather than assumed: JOINED-PROVENANCE-CYCLE-SITES, "
                "JOINED-PROVENANCE-RELEASE-NULLABLE and JOINED-PROVENANCE-FACTS-NAME-NOTHING. That "
                "card also registered them BEFORE its final regeneration and updated AGENTS.md's own "
                "stated count in the same commit, which is the behaviour the ledger wants and the "
                "reason its items needed no repair here."
            ),
            "removals_lost_at_merges": (
                "A PATTERN, recorded because it has now happened three times in three waves and "
                "nothing indexes it. named_followups is hand-maintained (constraint 25), and at a "
                "merge resolution its ADDITIONS survive while its REMOVALS are dropped: wave 32's "
                "DOCS-COMPACT-O row lost 2,208 bytes at 3bbec4d0; wave 34's close lost "
                "PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW and the integrator restored it by hand at "
                "8d158c4e; and this wave MUTEX-SHARE-CROSS-ARMS' single retirement was dropped while "
                "its five additions landed. No standing rule is proposed here - that would be a card's "
                "work, and RECONCILE-SWEEP-DIMENSIONS-UNPINNED (registered) already asks for the "
                "tripwire that would catch a drifted derived count. What is recorded is that the "
                "close must diff this section in BOTH directions, which is how all three were found."
            ),
            "stability_reeval_and_compaction_cadence": (
                "NOTED FOR THE RECORD WITHOUT ACTING, as CLAUDE.md's cadences require. "
                "STABILITY-REEVAL #2 IS DUE AFTER THIS WAVE - the cadence is every two waves and #1 "
                "ran in wave 34. This close deliberately does not perform it, but three of its "
                "measurements are the kind #2 exists to consume and are flagged as such: "
                "HOST-BACKOFF-FALLBACK-ARMS-UNDRIVEN is a fresh 'defensive, not verified' arm in a "
                "subsystem repaired THIS wave (method item 1, hunt vacuous greenness); "
                "HOST-ROW-STARVATION-DESCRIPTION is a boundary whose first arm is now discharged and "
                "whose second has no measurement at all (method item 2, separate the proved side "
                "from the observed side); and the removals-lost-at-merges pattern above is a "
                "semantic edit made with no card and no residual name, three times over (method item "
                "3, find semantic edits made without a card). DOCS-COMPACT-7 falls around wave 37, "
                "and the open decision DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK sets its rule; this "
                "close adds one data point to that decision and does not take it - the three files "
                "GREW again here (a new standing constraint 5c, a new 15o, a rewritten constraint 16 "
                "and a new CLAUDE.md bullet), and every one of those was a correction landing late "
                "rather than narrative accreting, which is the user's question restated rather than "
                "answered."
            ),
        },
    },
    "deferred_decisions": {
        "note": (
            "All SEVEN decisions the wave-34 close carried are re-listed, as card SWEEP-BLIND-SPOTS' "
            "fourth repair now requires at every later close until each carries resolved or "
            "withdrawn with a ground. FOUR of them are wave 33's, carried unchanged for a third "
            "wave. THESE ARE THE USER'S AND THIS CLOSE TAKES NONE OF THEM; where this wave produced "
            "new evidence, the evidence is stated and the decision is still left open. The one "
            "exception is recorded as `resolved` only because a CARD was assigned to it and "
            "discharged it, not because the close decided anything."
        ),
        "items": [
            {
                "name": "items",
                "verdict": "withdrawn",
                "ground": (
                    "NOT A DECISION - a SHAPE ARTIFACT of the wave-34 record, withdrawn here rather "
                    "than by editing that record (constraint 21). The wave-34 close wrote its "
                    "deferred_decisions `items` as a MAP of name -> sentence; "
                    "reconciliation_sweep.decisions_in() reads `items` only when it is a LIST of "
                    "{name, verdict, ground} objects, and otherwise falls through to a plain-name "
                    "map, so it read the literal key 'items' as a decision name carrying no verdict "
                    "- which defaults to open and is then re-listed forever. MEASURED at this close: "
                    "`_decisions_of(pathlib.Path('docs/reports/wave34-close'))` returned exactly "
                    "['items'] and NONE of that close's seven decisions. THE REAL DEFECT BEHIND THIS "
                    "PSEUDO-ENTRY IS REPAIRED, not papered over: the three decisions wave 34 RAISED "
                    "reached the re-listing mechanism nowhere and would have expired in precisely "
                    "the silence that mechanism was landed in the same wave to prevent, so all seven "
                    "are registered in the new docs/reports/wave34-close/deferred_decisions.json - "
                    "the sibling file the tool provides for exactly this case - and all seven are "
                    "re-listed below. Withdrawing the artifact is safe only BECAUSE that file now "
                    "exists; without it this withdrawal would silently drop seven decisions. A "
                    "future close writing this block should use the LIST shape."
                ),
            },
            {
                "name": "MATRIX-UPDATED-STALE-AT-WAVE-27",
                "verdict": "deferred",
                "ground": (
                    "Still deferred; the decision is untaken. NEW EVIDENCE THIS WAVE, which the "
                    "decision should be re-read against: the matrix's `updated` field still carries "
                    "wave 27's long prose, while the file itself has now been REGENERATED THROUGH "
                    "THE GENERATOR AT EVERY WAVE-35 MERGE - by C-GATEWAY-CONSTANT-DO-WHILE, "
                    "MUTEX-SHARE-CROSS-ARMS and JOINED-PROVENANCE-CYCLES, each passing "
                    "`--gate docs/reports/wave33-close/gate.json` deliberately to preserve the "
                    "integrator-supplied provenance label - and again by this close with its own "
                    "gate. So the file is current and one field inside it advertises wave 27; the "
                    "gap is now wider and more visible than when the decision was first deferred, "
                    "not narrower. The close does not edit the field: `updated` is one of the "
                    "generator's carried sections and rewriting it is precisely the decision being "
                    "deferred."
                ),
            },
            {
                "name": "CT10-REASON-ROW",
                "verdict": "deferred",
                "ground": (
                    "Still deferred, carried from the wave-33 close for a third wave. NEW EVIDENCE: "
                    "card CT-CTWIN-LANE re-measured all four remaining unmapped cases this wave and "
                    "recorded CT10 as staying unmapped 'on half a scenario' - so the case was looked "
                    "at by a lane that could have moved it and did not, which is information the "
                    "decision did not previously have. The decision itself is untouched."
                ),
            },
            {
                "name": "EXAMPLES-STYLE-SLICE-FANOUT",
                "verdict": "deferred",
                "ground": (
                    "Still deferred, carried from the wave-33 close for a third wave. No card of "
                    "wave 35 touched examples/ and this close produces NO new evidence about it. "
                    "Re-listed so it does not expire in silence, which is the whole point of the "
                    "re-listing mechanism."
                ),
            },
            {
                "name": "SYNC-RULE-TOOLING-COMMIT-SCOPE",
                "verdict": "deferred",
                "ground": (
                    "Still deferred, carried from the wave-33 close for a third wave. NEW EVIDENCE, "
                    "and it sharpens the question: this close's own --uncarded run returned FOUR "
                    "commits, three of which are exactly the shapes the decision is about - a "
                    "wave-open commit, and two integrator corrections that change behaviour "
                    "(b6ea42df rewrote a test, 8d158c4e restored a lost ledger entry). Each was "
                    "accounted for BY EYE in this record's hand sweep, which is what the absence of "
                    "a rule currently costs: a person must do it every wave. The live instance the "
                    "decision names, 37bc9004, is unchanged and still registered as "
                    "WINDOWS-COHERENCE-GATES-UNCARDED. Coupled to "
                    "SWEEP-UNCARDED-RULE-APPROXIMATE, also open."
                ),
            },
            {
                "name": "DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK",
                "verdict": "deferred",
                "ground": (
                    "Still deferred - it is explicitly a question RAISED FOR THE USER and no card "
                    "may answer it. NEW EVIDENCE from this close, offered as a data point and not as "
                    "an argument for either reading: the three files GREW again at this close, and "
                    "every addition was a CORRECTION LANDING LATE rather than narrative accreting - "
                    "constraint 5c (a rule arbiter #1 landed), constraint 15o (a rule proposed three "
                    "waves ago and never landed), a rewritten constraint 16 (a description replaced "
                    "because its mechanism became known), and a CLAUDE.md hygiene bullet (a defect "
                    "measured this wave). That is the same pattern DOCS-COMPACT-6 reported and used "
                    "to justify amending the cadence rule, now observed OUTSIDE a compaction pass, "
                    "which is arguably evidence for the second reading - that corrections should "
                    "land as their own work and compaction be measured only on redundancy - since "
                    "here they did land outside one. The answer sets DOCS-COMPACT-7's rule and "
                    "DOCS-COMPACT-7 falls around wave 37."
                ),
            },
            {
                "name": "VERIFIER-HOST-ROW-DISPOSITION",
                "verdict": "resolved",
                "ground": (
                    "RESOLVED BY THE CARD IT WAS ASSIGNED TO, not by this close. The wave-34 close "
                    "recorded it as 'NOTED NOT TAKEN - the brief assigns it to card "
                    "VERIFIER-HOST-CONTROL-BUDGET this wave', so it was a work assignment rather "
                    "than a user decision, and the assignee reported. Its open question was whether "
                    "documentation option (a) ALONE retires "
                    "VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED; that question no longer arises, "
                    "because the card took option (c) - an evidenced investigation PLUS the repair "
                    "it justified. The mechanism is named (8 KiB pipe capacity against a 1 ms "
                    "backoff quantum), the repair is in crates/qcore-verifier-host/src/lib.rs with "
                    "the TEST FILE UNTOUCHED, the control passes 38/38, and both matrix entries are "
                    "retired by this close as the card asked. WHAT THIS DOES NOT RESOLVE, and is "
                    "carried on rather than closed with it: no unloaded-host measurement of the row "
                    "exists, which is HOST-ROW-STARVATION-DESCRIPTION's surviving arm, and the first "
                    "arm's fixed one-second guard is now constraint 16's sixth load-sensitive row "
                    "with VERIFIER-HOST-EARLY-ARM-OVERLOAD open against it."
                ),
            },
            {
                "name": "PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED",
                "verdict": "deferred",
                "ground": (
                    "Still deferred, and the close deliberately did NOT resolve it by completing the "
                    "registration, because the decision is precisely WHICH of the two arms to take - "
                    "complete the registration, or state the rule that keeps platform residuals out "
                    "of the matrix - and taking one silently would be the close deciding it. NEW "
                    "EVIDENCE: this wave added ANOTHER member to exactly that family, "
                    "VERIFIER-HOST-READINESS-APPLE-UNRUN (the poll(2) readiness wait is cfg(unix), "
                    "Apple would run it, no macOS execution is claimed), and this close DID register "
                    "it - along with CT-COMPOSED-NATIVE-APPLE, a second -APPLE-RUN-shaped name - "
                    "because both were named by cards whose residual lists this close was sweeping. "
                    "So the split the decision calls 'neither' has widened rather than closed: more "
                    "platform residuals are now registered, and the six CLAUDE.md names that "
                    "prompted the decision still are not."
                ),
            },
        ],
    },
    "gate": "docs/reports/wave35-close/gate.json",
}


def main():
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(RECORD, indent=1, ensure_ascii=False) + "\n")
    print(f"wrote {OUT} with {len(VERDICTS)} verdicts")
    counts = {}
    for entry in VERDICTS.values():
        counts[entry["verdict"]] = counts.get(entry["verdict"], 0) + 1
    for key in sorted(counts):
        print(f"  {key}: {counts[key]}")


if __name__ == "__main__":
    main()
