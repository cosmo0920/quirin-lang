#!/usr/bin/env python3
"""Replace AGENTS.md's Merged gate row with this close's own measurement.

A whole-row replacement rather than a figure edit, because every qualification
in the row is a statement about the run that produced it: carrying wave 35's
sentences beside wave 36's numbers is how a row becomes a fiction one figure at
a time. Run from the repository root.
"""

import pathlib
import sys

AGENTS = pathlib.Path("AGENTS.md")
HEAD = "| Merged gate | "

ROW = (
    "| Merged gate | **4,611 passed / 0 failed / 8 ignored, 229 test executables / 231 result "
    "summaries** · 122 provider (carried; **Unix** — see below) · **293 Python** · **135 Lean "
    "audits** · both probes exit 0 · fmt/clippy clean. Measured by the **wave-36 close** at its "
    "own integration tip (base `fa48211a`, the `WRITE-LEND-TASKS-EXERCISE` merge, plus the close's "
    "own commits — so the one code change the close makes, the reconciliation sweep's "
    "content-verifying `--check` arm, is *inside* this measurement rather than beside it), "
    "`git submodule update --init --recursive` run first and exit 0 (`WORKTREE-SUBMODULE-UNGATED`); "
    "one uninterrupted `cargo test --workspace --no-fail-fast` of 21m24s (a full rebuild; see the close's §11), **every step's exit "
    "captured to its own file with no pipe in the command** (`GATE-EXIT-SWALLOWED-BY-PIPE`). "
    "Per-command exits in `docs/reports/wave36-close/gate.json`, and each card's own gate in its "
    "own report. **A gate figure is a measurement on one machine, never a property**, and four "
    "qualifications travel with this one. First, constraint 16's rows, because that constraint "
    "requires a gate report to **state which it saw**: the sixth row "
    "`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` was **green "
    "here**, as were `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` and "
    "`memory_growth_beyond_the_declared_limit_is_bounded` — but this is a full-suite run on a "
    "machine carrying its own build, **not** the quiet-host isolated number "
    "`HOST-ROW-STARVATION-DESCRIPTION` still asks for, and it is not offered as one. Second, and "
    "new this wave, `a_forced_override_never_weakens_an_execution` was **also green here** and is "
    "the **unnamed seventh** row: card `C-GATEWAY-INDIRECT-CENSUS-2` measured it red under two "
    "concurrent corpus audits, so constraint 16's list is now recorded **known incomplete** rather "
    "than assumed complete (`LOAD-SENSITIVE-ROW-UNNAMED`). Third, the Python figure is the "
    "**re-run after a deliberate red**: editing `named_followups` by hand left "
    "`dimensions.named_followups` stale and `CommittedMatrixIsSelfConsistent` failed with "
    "`171 != 187`, which is `D02-DIMENSIONS-FOLLOWUP-COUNT`'s tripwire working; the repair was to "
    "regenerate through `build_matrix.py`, never to touch the count. Fourth, provider **122 is "
    "carried, not re-measured** (`git diff --name-only 60958bf0^ HEAD -- providers/ "
    "crates/qcore-schemas/` is empty and `qcore-schemas` is still 14.0.0, so constraint 18 is not "
    "engaged) **and is a Unix figure** (`PROVIDER-CONFORMANCE-UNIX-ONLY`). Prior figures, for the "
    "delta only: 4,580 / 0 / 8 at the wave-35 close, 4,559 at wave 34, 4,544 at wave 33, 4,534 at "
    "wave 32; the +31 is wholly the wave's own cards, this close adding no cargo test and all "
    "**19** of its new tests being Python |\n"
)


def main() -> int:
    lines = AGENTS.read_text(encoding="utf-8").splitlines(keepends=True)
    hits = [i for i, line in enumerate(lines) if line.startswith(HEAD)]
    if len(hits) != 1:
        print(f"REFUSING: expected exactly one Merged gate row, found {len(hits)}")
        return 1
    index = hits[0]
    if "4,580 passed" not in lines[index]:
        print("REFUSING: the row is not the wave-35 one this replacement was written against.")
        print("Re-read it before replacing - a blind overwrite is how a row loses a qualification.")
        return 1
    lines[index] = ROW
    AGENTS.write_text("".join(lines), encoding="utf-8")
    print("Merged gate row replaced (whole row, not a figure edit).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
