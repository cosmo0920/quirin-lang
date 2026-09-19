#!/usr/bin/env python3
"""Card `CT-PROOF-LEG-3`'s tracker edit: two retirements, three amendments,
three registrations. Idempotent — re-running changes nothing.

Every retirement is verified by id **before** it is removed, and every
amendment is applied by exact-substring replacement so a drifted entry is
refused rather than silently rewritten. Run from the repository root:

    python3 docs/reports/ct-proof-leg-3/scripts/edit_named_followups.py
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"

RETIRE = {
    # Discharged by this card: the cross-check exists
    # (compiler/tests/proof_leg_digests.rs), it runs in the standing gate's
    # first step, it is shown red by four mutations (D1-D4), and the entry's
    # second half -- correcting LEASE-BOUNDARY-TRANSCRIPTION-PREMISE's
    # discharge sentence -- is the amendment below.
    "PROOF-LEG-DIGESTS-UNCROSSCHECKED",
    # Discharged by this card: the survey's conclusion is registered as
    # CT-PROOF-F02-CANDIDATES-EXHAUSTED, and the CT ledger row already carries
    # the "two shapes, not one" sentence the entry's second half asked for
    # (verified at base, re-read at this tip).
    "CT-PROOF-FOURTH-CASE-UNTRACKED",
}

AMEND = [
    (
        "LEASE-BOUNDARY-TRANSCRIPTION-PREMISE",
        "The close DECLINED to build it - a gate script extracting each "
        "`theorem ct*_digest` and comparing it with the corresponding `const` "
        "is code with a mutation control, not close work - and names it as "
        "brief material for the next implementation wave.",
        "The close DECLINED to build it - a gate script extracting each "
        "`theorem ct*_digest` and comparing it with the corresponding `const` "
        "is code with a mutation control, not close work - and names it as "
        "brief material for the next implementation wave. "
        "DISCHARGE TEXT REPAIRED at wave 41 by card CT-PROOF-LEG-3, which "
        "built that mechanism: the second arm no longer names a "
        "nonexistent pattern and now reads -- bind the Lean term to the Rust "
        "pin the way compiler/tests/proof_leg_digests.rs binds the proof "
        "legs' digests, which means giving QCore.TaskTraceLeaseBoundary's "
        "flagshipRun a kernel-decided digest theorem named ct<N>..._digest, "
        "pinning the same numeral in a compiler/tests/ constant named "
        "CT<N>_LEAN_..._DIGEST, and registering the case in that file's "
        "CASES_WITH_A_DIGEST_PIN so the total-or-reject cross-check covers "
        "it. THE ENTRY STAYS OPEN: CT-PROOF-LEG-3 built the mechanism and "
        "deliberately did NOT apply it to the lease-boundary module, which is "
        "a different card's artifact and outside that card's file ownership. "
        "The first arm -- carry the module's own transcription sentence into "
        "constraint 15n -- is unchanged and remains the cheaper discharge.",
    ),
    (
        "CT-PROOF-CONTROL-ARM-TIER",
        "DISCHARGE: as stated in PROOF-LEG-BATCH's own follow-on entry, which "
        "is authoritative for the condition's text.",
        "DISCHARGE: as stated in PROOF-LEG-BATCH's own follow-on entry, which "
        "is authoritative for the condition's text. DISPOSED at wave 41 by "
        "card CT-PROOF-LEG-3 as a MEASURED BOUNDARY, not landed, with the "
        "ground stated: the tier is a stage-vocabulary and manifest change, "
        "which is spec work above a card, and the numerator sentence it asks "
        "for already stands in the CT ledger row (the proof leg 'has two "
        "shapes, not one'). What that card ADDS is the measurement the tier "
        "exists for, re-run at wave 41 rather than inherited: compiling "
        "CT12's and CT14's own fixtures through compile_files, executing the "
        "checked task modules and deriving with the production observer "
        "yields, for BOTH, the seven-event trace [Allocated 0 1 42, Spawned 1 "
        "0, Completed 1 Buffer(1), Joined 1, Read 0 1 42, Dropped 0 1, "
        "Completed 0 I32(42)] -- event for event CT01's roundTrip, whose "
        "proof leg already exists. So without the tier a proof row for either "
        "case would credit CT01's artifact under a second name; the tier is "
        "load-bearing, not a formality. Survey and raw output: "
        "docs/reports/ct-proof-leg-3/probes/. The entry stays open.",
    ),
    (
        "CT-PROOF-CREATION-FAILURE-ALPHABET",
        "DISCHARGE: as stated in PROOF-LEG-BATCH's own follow-on entry; see "
        "that report, which is authoritative for the condition's text.",
        "DISCHARGE: as stated in PROOF-LEG-BATCH's own follow-on entry; see "
        "that report, which is authoritative for the condition's text. "
        "RESTATED at wave 41 by card CT-PROOF-LEG-3 as a measured boundary, "
        "and SHARPENED: the entry names ONE alphabet extension, and the "
        "survey measured that the cases it lists need TWO. (1) Every scripted "
        "creation-failure arm is refused by the relation, not by the "
        "encoding: CT12, CT14, CT17 and CT20 derive to "
        "Violation(UnattributableEvent { index: 1 }) and CT13, CT15, CT16 to "
        "index 0 -- so the fix is attribution in AdmittedCausalOrder, as the "
        "entry says. (2) But the ALLOCATION-failure arms, which do derive in "
        "Rust (CT12, CT14, CT17, CT20, CT50 all give [AllocationFailed 0, "
        "Completed 0 Error(\"AllocError\")]), carry an Error OUTCOME that "
        "QCore.TaskTraceRelation.Outcome has no constructor for -- so a proof "
        "row on any failure arm needs the Outcome alphabet to grow as well, "
        "a second deliberate extension with every digest re-pinned. CT16's "
        "DEFAULT run needs it too (its trace carries Completed _ "
        "Error(\"WorkerError::Failed\") with no script at all). Measured at "
        "wave 41 through the real pipeline; raw output: "
        "docs/reports/ct-proof-leg-3/probes/survey.tsv. The entry stays open "
        "and no claim is made that either extension is worth the cost.",
    ),
]

REGISTER = [
    {
        "id": "CT-PROOF-F02-CANDIDATES-EXHAUSTED",
        "title": (
            "The F02-pattern proof-leg candidate set is empty after CT50, measured "
            "rather than inherited"
        ),
        "why": (
            "MEASURED by card CT-PROOF-LEG-3 (wave 41), discharging "
            "CT-PROOF-FOURTH-CASE-UNTRACKED by registering the fact it said reached no "
            "tracker. The survey compiled all 47 mapped cases' own fixtures through "
            "compile_files, executed every task-route module's reference execution and "
            "handed each to AdmittedCausalOrder::derive, on the default script and on "
            "five failure scripts. Result: exactly eleven mapped cases derive on the "
            "default script (CT01, CT03, CT12, CT13, CT14, CT15, CT16, CT17, CT20, "
            "CT37, CT50); four of those are already proved; CT12 and CT14 derive to "
            "CT01's own seven-event trace event for event; CT13, CT15, CT16, CT17 and "
            "CT20 derive but their DEFAULT runs do not exhibit their scenarios, whose "
            "defining arms need one or both of the extensions "
            "CT-PROOF-CREATION-FAILURE-ALPHABET now names. CT50 was the last case "
            "whose default run both derives and exhibits its scenario, and this card "
            "used it. CONSEQUENCE: a sixth proof leg cannot be an F02-pattern leg on "
            "an existing mapped case. It must be (a) an alphabet extension on "
            "CT03-PROOF-ALPHABET's method -- a conservativity theorem plus a rule shown "
            "non-vacuous in both directions, every digest re-pinned, which the "
            "PROOF-LEG-DIGESTS cross-check now makes visible -- or (b) a new binding "
            "pattern, as CT07's ledger binding was, or (c) a case the mapping lanes "
            "have not yet mapped. DISCHARGE: the next proof-leg card records which of "
            "the three it took, and this entry is retired by the card that takes one; "
            "until then it is the record that the cheap route is spent. Survey table: "
            "docs/reports/ct-proof-leg-3/README.md section 2; raw output: that "
            "report's probes/survey.tsv."
        ),
        "source": "docs/reports/ct-proof-leg-3/README.md",
        "recorded_by": "CT-PROOF-LEG-3",
    },
    {
        "id": "CT50-PROOF-ADDRESS-REUSE-UNPROVED",
        "title": (
            "CT50's proof row covers the capability half of its scenario; the numeric "
            "address reuse the scenario is named after is native observation only"
        ),
        "why": (
            "Registered by card CT-PROOF-LEG-3 (wave 41) with the row it created, so "
            "the row cannot be over-read. CT50's scenario is 'New lifetime reuses old "
            "numeric address/descriptor' and its native evidence places the second "
            "mapping at the first one's address with MAP_FIXED_NOREPLACE "
            "(run_native_task_scenarios, the pinned stderr transcript MAP 1 / UNMAP 1 "
            "/ MAP 2 / REUSED_ADDRESS / UNMAP 2 / RUN_CLEAN 2). The F02 observation "
            "alphabet carries GENERATIONS and not addresses, so the reuse is invisible "
            "to it: QCore.Ct50LifetimeIdentity proves that the two generation "
            "lifetimes are disjoint, that no fact of the run uses the old generation, "
            "and that the one read names the new generation with the new allocation's "
            "own payload -- all of which would hold equally if the second mapping "
            "landed somewhere else. The address-level fact is therefore observed and "
            "not proved, in exactly constraint 15n's 'separate the proved side from "
            "the observed side' sense. DISCHARGE: either an alphabet that carries a "
            "mapping identity beside the generation (an extension on "
            "CT03-PROOF-ALPHABET's method, with the same conservativity obligation), "
            "or an explicit decision that address identity stays native evidence, "
            "recorded in the manifest's CT50 semantic profile rather than only here. "
            "Until then the row reads: the scenario property proved of the checked "
            "artifact's reference run and its admitted renderings, address reuse "
            "covered by observation only."
        ),
        "source": "docs/reports/ct-proof-leg-3/README.md",
        "recorded_by": "CT-PROOF-LEG-3",
    },
    {
        "id": "PROOF-LEG-DIGEST-CROSSCHECK-TEXTUAL",
        "title": (
            "The new digest cross-check compares two source texts, not a value the "
            "Lean kernel handed it"
        ),
        "why": (
            "Registered by card CT-PROOF-LEG-3 (wave 41) with the mechanism it built, "
            "discharging PROOF-LEG-DIGESTS-UNCROSSCHECKED but not more than it. "
            "compiler/tests/proof_leg_digests.rs reads the numeral out of each "
            "'theorem ct<N>...digest : ... = <numeral> :=' line in proof/QCore/ and "
            "compares it with the corresponding compiler/tests/ constant. That closes "
            "the re-transcription hole it was built for -- a digit changed on either "
            "side goes red, measured as D1, D2 and D3, and a Lean theorem deleted goes "
            "red as D4 -- and it rests on one premise it does not itself check: that "
            "the numeral in the .lean SOURCE is the numeral the kernel decided. The "
            "standing scripts/check_lean_audits.py gate is what makes that true (the "
            "theorem is 'by decide' and the audit builds it), so the premise is "
            "discharged by a DIFFERENT gate step rather than by this test, and a tree "
            "where the Lean build was skipped would pass this test on two agreeing but "
            "unchecked texts. The parser is also line-shaped: a digest theorem whose "
            "statement is split across lines, or written with a different numeral "
            "spelling, is not seen -- fail-open in that one direction, which is why "
            "CASES_WITH_A_DIGEST_PIN is pinned beside it as the fail-closed half. "
            "DISCHARGE: emit the digests from Lean itself (a #eval or a generated "
            "file the test reads) so the compared value comes from the kernel's own "
            "elaboration rather than from a line of source, or state this premise in "
            "the file and accept it deliberately."
        ),
        "source": "docs/reports/ct-proof-leg-3/README.md",
        "recorded_by": "CT-PROOF-LEG-3",
    },
]


def main():
    text = MATRIX.read_text()
    matrix = json.loads(text)
    entries = matrix["named_followups"]
    by_id = {entry["id"]: entry for entry in entries}
    before = len(entries)

    for name in sorted(RETIRE):
        if name not in by_id:
            print("already retired:", name)
    kept = [entry for entry in entries if entry["id"] not in RETIRE]

    for name, old, new in AMEND:
        entry = next((e for e in kept if e["id"] == name), None)
        if entry is None:
            raise SystemExit("amendment target not registered: %s" % name)
        if new in entry["why"]:
            print("already amended:", name)
            continue
        if entry["why"].count(old) != 1:
            raise SystemExit(
                "%s: the amendment site occurs %d times, not once"
                % (name, entry["why"].count(old))
            )
        entry["why"] = entry["why"].replace(old, new)

    for record in REGISTER:
        if record["id"] in by_id:
            index = next(i for i, e in enumerate(kept) if e["id"] == record["id"])
            kept[index] = record
            print("already registered, refreshed:", record["id"])
        else:
            kept.append(record)

    matrix["named_followups"] = kept
    trailing = "\n" if text.endswith("\n") else ""
    MATRIX.write_text(json.dumps(matrix, indent=1) + trailing)
    print("named_followups %d -> %d" % (before, len(kept)))


if __name__ == "__main__":
    main()
