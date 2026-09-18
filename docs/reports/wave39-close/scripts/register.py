#!/usr/bin/env python3
"""WAVE-39-CLOSE: the tracker edits, in one place, on the wave-38 close's precedent.

`named_followups` is hand-maintained (the generator carries it verbatim), so this
appends / amends / removes and rewrites in the generator's byte format
(indent=1, ensure_ascii=False, trailing newline); `dimensions.named_followups`
is then re-derived by

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/wave39-close/gate.json

Phase 1 (`--show`): print the full text of every entry this close proposes to
retire, so the discharge condition is read before anything moves.
Phase 2 (no flag): register three, amend one, retire two. Idempotent.
"""
import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/wave39-close/README.md"
CARD = "WAVE-39-CLOSE"

RETIRE = [
    # Discharge (its own words): "a citation appended to both CT46 texts on the CT07
    # precedent (docs/reports/tsan-rows-2/scripts/cite_ct07.py), naming the row and the
    # recorded decision that this route's native evidence stays serial by construction."
    # Done by scripts/cite_ct46.py: docs/reevaluation/ct_manifest.json CT46 semantic_profile
    # and compiler/tests/fixtures/ct/CT46/expected.json notes.
    "CT46-NOTES-PREDATE-TSAN-ROW",
    # Discharge (its own words): "read the other harness, and land the proposed CLAUDE.md
    # measurement-hygiene bullet quoted in docs/reports/mutex-share-cross-arms/README.md
    # section 11." The bullet is in CLAUDE.md (landed before this close; the bullet itself
    # said the second reader was not done). The other harness,
    # docs/reports/c-gateway-return-call-w4/scripts/mutations.py, was read by this close:
    # it restores by path.write_text(text) at lines 120 and 131 - a rewrite, no copy2.
    # The family goes forward as MUTATION-HARNESS-STALE-ARTIFACTS (the mv mechanism and
    # the mechanical check still owed).
    "MUTATION-HARNESS-MTIME-RESTORE",
]


def entry(id_, title, why, source):
    return {"id": id_, "title": title, "why": why, "source": source, "recorded_by": CARD}


NEW = [
    entry(
        "MUTATION-HARNESS-STALE-ARTIFACTS",
        "A mutation harness that restores a source file by mv from a cp backup leaves the file older than the mutant's build, and cargo keeps the mutated rlib",
        "Proposed, not registered, by card LOOPS-ON-COMPOSED-ROUTES (wave 39) section 12 item 8 and "
        "section 6, and registered by the wave-39 close. The mechanism is the second route to the "
        "hazard MUTATION-HARNESS-MTIME-RESTORE (wave 35) recorded for shutil.copy2: `mv` of a `cp` "
        "backup (and `cp -p`) keeps the backup's mtime, which predates the build cargo made from the "
        "mutant, so cargo's fingerprint reads the restored file as unchanged and every later run links "
        "the mutated crate while git status reads clean. That card's first full gate ran against exactly "
        "that: exit 101 with three red rows of its own test file (docs/reports/loops-on-composed-routes/"
        "gate/run1/), repaired by one `touch` after each restore plus a green baseline before and after "
        "(its mutations.sh). MEASURED by the wave-39 close over every shipped harness in docs/reports/ "
        "(seventeen files, `find docs/reports -name 'mutat*'`): twelve restore by rewriting "
        "(write_text/write_bytes - aggregate-owned-field, c-gateway-constant-do-while, "
        "c-gateway-return-call-w4, ct07-two-thread-proof-leg and stability-reeval-3 (both also os.utime "
        "into the future), ct42-cell-route, joined-provenance-call-frames, joined-provenance-cycles, "
        "mutex-share-cross-arms, scalar-route-exit-edge, wave36-close, wave38-close); four by `cp` from "
        "a backup, which writes a fresh mtime (c-gateway-return-position without a touch; "
        "task-route-tsan's mutate.sh and mutate2.sh and tsan-rows-2's mutate.sh each with a touch); one "
        "by `mv` plus `touch` (loops-on-composed-routes, the hazard's own, repaired). None is "
        "mtime-preserving today, and the CLAUDE.md rule now names the mv/cp -p mechanism beside copy2. "
        "DISCHARGE: the survey made mechanical - a script under scripts/ that lists every shipped "
        "harness's restore mechanism and exits nonzero on an mtime-preserving one (mv of a backup "
        "without a touch, copy2, cp -p), with a test red by mutation, run at each close; until then a "
        "close re-runs the hand survey and records it.",
        "docs/reports/loops-on-composed-routes/README.md section 12 item 8; " + REPORT,
    ),
    entry(
        "INTEGRATION-GATE-CLANG-CLAIM-BORROWED",
        "An integration gate's clang presence claim is checked against the card's recording beside it, not against a recording of the integration run",
        "Found by the wave-39 close while sweeping the three integration-gate.json files the integrator "
        "wrote at the merges e80b58d6 (docs/reports/aggregate-owned-field/, tip 3fa88fc6), 0150c904 "
        "(docs/reports/tsan-rows-2/, tip 601acde6) and b6d1f101 (docs/reports/loops-on-composed-routes/, "
        "tip 4786ae4c), each because no card's gate.json could agree with a merged tree carrying sibling "
        "cards' Lean audits. Each file's note says the Clang claim was 'taken from the card's own "
        "recording beside this file (gate/clang-version.txt, same host)'. The wave-38 close's check "
        "(build_matrix.py --gate: a clang claim must agree with the clang-version.txt beside the gate) "
        "was landed so that a green workspace figure taken without Clang is distinguishable in the "
        "record from one taken with it; a claim checked against a SIBLING run's recording satisfies the "
        "check by construction and records nothing about the integration run itself, so for those three "
        "regenerations the check was met vacuously. Same host is likely and is not what the check "
        "measures. DISCHARGE: an integration gate carries its own clang-version.txt and .exit from the "
        "integration run (a docs/reports/<card>/integration-gate/ directory the generator's beside-the-"
        "gate lookup reaches, or an explicit layout), and the CLAUDE.md convention line says so; or a "
        "recorded decision that an integration gate's Clang claim is the card's, stated in the file's "
        "note in those words. Until then each integration-gate.json is read as: workspace figure the "
        "integrator's own, Clang presence the card's.",
        REPORT,
    ),
    entry(
        "GATE-SUMMARY-PARSER-OK-ONLY",
        "A merge message's measured line read 0 failed while naming three failures: the integrator's summary parser summed only ok result lines",
        "Recorded by the wave-39 close as the integrator's own defect (STABILITY-REEVAL step 3, a "
        "semantic edit made without a card). Merge 0d9e922f (CT07-TWO-THREAD-PROOF-LEG) records at its "
        "integration tip f1fcd14d 'workspace 4620 passed / 0 failed / 9 ignored' and in the same "
        "sentence names three constraint-16 verifier-host rows that FAILED under four sibling builds "
        "(re-verified 12/12 and 12/12 in isolation). A parser that sums only `test result: ok.` "
        "summaries drops every binary whose summary reads FAILED, so passed is undercounted and failed "
        "reads zero. The brief to this close states the true totals as 4,641 / 3 / 9; 4,641 + 3 = 4,644 "
        "is the card's own figure at 81f783ce (its gate.json), which is the corroboration available to "
        "this close - the integration run's log is not in the tree and the close did not re-derive the "
        "triple. No ledger figure carried it: the merged-gate row at this close's base is the card's own "
        "4,644 / 0 / 9 and the close re-measured the row at its own tip. The defect lives in a permanent "
        "merge message and in whatever produced its line. DISCHARGE: the summary parser used at a merge "
        "counts every `test result:` line (ok and FAILED) and is shown to reproduce the right triple "
        "from a log carrying a FAILED summary (a gate log with a constraint-16 row red is such a log), "
        "and merge messages state passed/failed/ignored from it; until then a merge message's measured "
        "line that names failures while reading 0 failed is read as this defect, not as a measurement.",
        REPORT,
    ),
]

AMEND = {
    "PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW": (
        " WAVE-39 CLOSE: a row is NAMED for the first time, on the integrator's record rather than a "
        "captured log: at the amend of merge 0150c904 (TSAN-ROWS-2) one run of the suite failed "
        "test_every_audit_runs_once_with_the_sequential_command "
        "(scripts/tests/test_check_lean_audits.py:141) and the re-run was green. The run's per-test "
        "output is not in the tree, so this close records the name as reported and measured what the "
        "test does: it patches check_lean_audits.run_step and asserts the sorted command list "
        "check_audits issues through its ThreadPoolExecutor (scripts/check_lean_audits.py:160) - a "
        "concurrency-shaped row, not the wall-clock one the wave-34 hypothesis named. Discharge (a) is "
        "half met (a row named once, not reproduced under captured output); whether it joins constraint "
        "16's list or is repaired is still undecided, and the wave-34 caution stands: one green re-run "
        "is not a discharge."
    ),
}


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    nf = matrix["named_followups"]
    ids = {e["id"]: e for e in nf}
    if "--show" in sys.argv:
        for name in RETIRE:
            e = ids.get(name)
            print("##", name, "-", "REGISTERED" if e else "ABSENT")
            if e:
                print(json.dumps(e, indent=1, ensure_ascii=False))
        for name in AMEND:
            print("## amend target", name, "-", "REGISTERED" if name in ids else "ABSENT")
        for e in NEW:
            print("## new", e["id"], "-", "ALREADY PRESENT" if e["id"] in ids else "absent, will register")
        return
    before = len(nf)
    for e in NEW:
        if e["id"] in ids:
            print("skip (present):", e["id"])
            continue
        nf.append(e)
        ids[e["id"]] = e
        print("registered:", e["id"])
    for name, text in AMEND.items():
        e = ids[name]
        if text.strip() in e["why"]:
            print("skip (amended):", name)
            continue
        e["why"] = e["why"] + text
        print("amended:", name)
    for name in RETIRE:
        if name not in ids:
            print("skip (already retired):", name)
            continue
        assert ids[name] in nf
        nf.remove(ids[name])
        del ids[name]
        print("retired:", name)
    matrix["named_followups"] = nf
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print("named_followups", before, "->", len(nf))


if __name__ == "__main__":
    main()
