#!/usr/bin/env python3
"""WAVE-38-CLOSE: the tracker edits, in one place, on the add_followups.py precedent.

`named_followups` is hand-maintained (the generator carries it verbatim), so this
appends / amends / removes and rewrites in the generator's byte format
(indent=1, ensure_ascii=False, trailing newline); `dimensions.named_followups`
is then re-derived by

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/wave38-close/gate.json

Phase 1 (`--show`): print the full text of every entry this close proposes to
retire, so the discharge condition is read before anything moves.
Phase 2 (no flag): register two, amend five, retire three. Idempotent.
"""
import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/wave38-close/README.md"
CARD = "WAVE-38-CLOSE"

RETIRE = [
    "WAVE-38-QUEUE-NAMES-UNTRACKED",
    "LEAN-AUDIT-GATE-KEY-OPTIONAL",
    "CLANG-GUARDED-GREEN-FAMILY-750",
]


def entry(id_, title, why, source):
    return {"id": id_, "title": title, "why": why, "source": source, "recorded_by": CARD}


NEW = [
    entry(
        "STRING-CONCAT-LOOP-WINDOW",
        "The composed string route refuses a for, because its borrow window is exact only over one flat statement sequence",
        "Registered by the wave-38 close from the constraint text that names it (AGENTS.md standing constraint "
        "15h), the first half of WAVE-38-QUEUE-NAMES-UNTRACKED's discharge; the name has been written in that "
        "constraint since card STRING-CONCAT and reached no tracker (mentioned only inside STD-MODULES-O's why). "
        "Constraint 15h's words: a borrow's 'window runs from the borrow to the last statement reading the view "
        "or anything derived from it, a slice of a borrow being itself a borrow of the same buffer. Do not widen "
        "the window's computation without revisiting it: the interval is exact because the route is one flat "
        "statement sequence, and a loop construct invalidates that premise - which is why the composed route "
        "refuses a for today (STRING-CONCAT-LOOP-WINDOW)'; re-measured by SLICE-PERMISSIONS-WRITE-LEND "
        "('STRING-CONCAT-LOOP-WINDOW re-measured, not assumed'), the refusal at Q068. Measured by "
        "STABILITY-REEVAL-3 (wave 38, step 4): last_view_uses (compiler/src/std_string.rs:1057) computes the "
        "window over TOP-LEVEL event indices with through = unwrap_or(index) (:1509), so a view used only inside "
        "a Repeat body would close before the loop opens and every store in the body would be admitted - 15h's "
        "admitting arm; the Repeat refusal at :1814-1843 sits BEFORE that computation and must not be reordered. "
        "Its buffer-route sibling SLICE-PERMISSIONS-LOOP-WINDOW is deliberately NOT registered: constraint 15h "
        "states it as one of 'three refusals that must survive refactors', a RULE under "
        "CONSTRAINT-NAMED-BOUNDARIES-NOT-OPEN-WORK, which the lifting card names then. DISCHARGE (in the "
        "constraint's words): revisit the window's computation for a loop construct - a window over iterations, "
        "last_view_uses walking Repeat bodies under a runs-many-times semantics - keeping the refusal in front "
        "until then, and amend 15h's sentence deliberately when it lands; the queued LOOPS-ON-COMPOSED-ROUTES "
        "card's target.",
        "AGENTS.md standing constraint 15h; docs/reports/stability-reeval-3/README.md section 4; " + REPORT,
    ),
    entry(
        "AGGREGATE-OWNED-FIELD",
        "A v1 record field carries no custody: an owned field needs a field-path owner vocabulary and four rules the cycle-3 IR does not have",
        "Registered by the wave-38 close from the constraint text that names it (AGENTS.md standing constraint "
        "15k), the second half of WAVE-38-QUEUE-NAMES-UNTRACKED's discharge; the name has been written there "
        "since card AGGREGATES-V1 (wave 27) and reached no tracker (mentioned only inside AGGREGATES-O's why). "
        "Constraint 15k's words: 'Every field is an i32, so a record contributes no qcore_core::ownership "
        "operation, opens no obligation and derives no effect - do not give a field custody without the four "
        "rules that do not exist: the cycle-3 ownership IR addresses an owner by a plain name and has no path, "
        "so p.buf is not a question check_ownership can be asked, and an owned field needs a field-path owner "
        "vocabulary, a partial-move rule, a drop-through-aggregate rule and a leak rule at normal exit that sees "
        "into the aggregate (AGGREGATE-OWNED-FIELD; AGGREGATE-VIEW-FIELD for the smaller view-field step, which "
        "constraint 15h's exact borrow window cannot reach through)'. Measured by STABILITY-REEVAL-3 (wave 38, "
        "step 4): every owner-addressing field of the IR is a plain String (crates/qcore-core/src/ownership.rs"
        ":169-176, :242, :845) and check_ownership (:972) has no path type, so the owner vocabulary breaks "
        "first; the sentence that turns false first is 'a record contributes no ownership operation', an "
        "explicit no-op arm (compiler/src/std_buffer.rs:914-918); and Q055's buffer-at-a-field-position refusal "
        "is the fail-closed gate a card must knowingly lift. DISCHARGE (in the constraint's words): the four "
        "rules that do not exist - a field-path owner vocabulary, a partial-move rule, a drop-through-aggregate "
        "rule and a leak rule at normal exit that sees into the aggregate - landed in the IR first (an IR card "
        "before a route card), with Q055's lift stated deliberately and constraint 15k amended in the same "
        "change; the queued AGGREGATE-OWNED-FIELD card. Under the re-affirmed Q-space icebox (2026-09-18) it "
        "allocates no code, widening the existing StandardBuffer and Ownership bands with each widening stated "
        "in section L21.",
        "AGENTS.md standing constraint 15k; docs/reports/stability-reeval-3/README.md section 4; " + REPORT,
    ),
]

AMEND = {
    "MAIN-GUARD-REFUSES-PACK-REFS": (
        " AMENDED by the wave-38 close (STABILITY-REEVAL-3 section 6 item 9, applied): the exemption must "
        "cover BOTH transactions git pack-refs performs on refs/heads/main - 0000->X (the packed write) and "
        "X->0000 (the loose ref's deletion, byte-for-byte the shape test_deleting_main_is_refused protects) - "
        "and the refusal also fires in the MAIN checkout whenever the token is absent, so 'every delegated "
        "worktree' understates the scope; whichever behaviour is chosen is then pinned by a committed test "
        "for both transactions, which is MAIN-GUARD-PACK-REFS-UNPINNED's discharge and stays open."
    ),
    "F02-WINDOWS-PIN-STALE-SINCE-WAVE-36": (
        " WAVE-38 CLOSE: the platform-run half is carried as the deferred decision F02-WINDOWS-PIN-RUN "
        "(docs/reports/wave38-close/reconciliation.json), a user-validated Windows run in the "
        "HTTP-V1-WINDOWS-RUN pattern, and ending either half is visibly half the job. The Linux-side half of "
        "F02-DIGEST-PIN-DUPLICATED's discharge is landed: compiler/tests/f02_windows_pin_agreement.rs reads "
        "both files' literals and asserts they agree, #[ignore]d with this residual's name in its reason "
        "because it is RED at 4f663934 by design (run with --ignored to reproduce), and shown green by a "
        "temporary Windows-literal edit restored sha256-equal and never committed (mutations.log). The "
        "Windows literals were not moved."
    ),
    "MATRIX-RECORDED-BY-ABSENT-67": (
        " WAVE-38 CLOSE: deferred, as the decision MATRIX-RECORDED-BY-DECISION "
        "(docs/reports/wave38-close/reconciliation.json), with the cheap fact that decides it measured: of the "
        "67, 49 name exactly one docs/reports/<dir>/README.md in source and 18 name several (up to ten); 0 of "
        "the 49 directory names upper-case to a value any entry already uses as recorded_by. A back-fill is "
        "therefore an inference for 49 and impossible by rule for 18, which is why it is a decision and not an "
        "edit; ending either the decision or this entry is visibly half the job."
    ),
    "RETURN-CALL-TESTS-CLANG-GUARDED-GREEN": (
        " RE-SCOPED by the wave-38 close to the FAMILY CLANG-GUARDED-GREEN-FAMILY-750 measured: the 14-row "
        "file is one of 79 test files carrying 750 `if !clang_available()` early-return guards, every one "
        "reporting ok on a Clang-less host having measured nothing. This entry is now the family's tracker "
        "for that defect (Clang absence read as ok), and its discharge is the family's: one shared helper "
        "that reports Clang absence as ignored so the count moves visibly, or the record in each report that "
        "the guarded rows are a Clang-conditional measurement. Presence itself is now RECORDED and CHECKED "
        "per close: since the wave-38 close a --gate file must carry a `clang` presence claim agreeing with "
        "the clang-version.txt beside it (docs/reports/remaining-d02/build_matrix.py), so a green workspace "
        "figure taken without Clang is no longer indistinguishable in the record from one taken with it."
    ),
    "GATE-ROW-FIGURE-ATTRIBUTION-DRIFT": (
        " WAVE-38 CLOSE: still PARTIAL. The lean_audits key is now REQUIRED in a --gate file (not optional) "
        "and a `clang` presence key stands beside it, both checked by the generator; the '(and any other "
        "figure the row publishes)' arm - a key for the result-summary and test-target counts - and the "
        "row-level check of AGENTS.md's text against gate.json remain unlanded."
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
