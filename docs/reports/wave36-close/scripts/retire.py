#!/usr/bin/env python3
"""The three retirements this close takes, each NAMED and each CHECKED first.

The brief's rule, and STABILITY-REEVAL #2 item 5's: a name is only counted as a
retirement if it is actually a registered entry, and retiring an entry requires
re-homing every name it was the sole tracker for. Both checks are mechanical
here; the second is `--residuals` re-run after the removals.

Run from the repository root.
"""

import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")

RETIRE = {
    "CARD-SCOPE-SELF-MERGE":
        "Discharged. Its condition was 'a mechanism that makes a delegated card unable to advance "
        "main - not an instruction telling it not to'. Card CARD-SCOPE-GUARD built the mechanism "
        "and said the discharge completes when the integrator runs the activation; the integrator "
        "ran it, and .git/qcore-main-guard.log carries the self-test's two refusals and every "
        "later merge of the wave as authorised. The card offered retirement-on-activation as the "
        "integrator's choice and asked that it be a DELIBERATE removal noted in the close - this "
        "is that removal, and this is that note.",
    "MAIN-GUARD-UNACTIVATED":
        "Discharged by the same fact. Its discharge condition is 'the integrator runs section 9.1 "
        "and records it (a wave close noting the command ran, or --verify exit 0 quoted)'. The "
        "command ran and this close notes it, which is the first arm exactly. No git command was "
        "run against the main checkout by this close; the log was read as a file.",
    "RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED":
        "Discharged by this close. Its discharge condition is an arm of --check that asserts an "
        "applied verdict's quoted text is present at the location the verdict names, shown RED by "
        "mutation before it is believed. The arm is landed with 19 tests, every one of which "
        "mutates the fixture in one place; and the implementation itself was mutated in 8 places "
        "and all 8 were caught. This close's own record is the first held to it.",
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    followups = matrix["named_followups"]
    present = {entry["id"] for entry in followups}

    missing = [name for name in RETIRE if name not in present]
    if missing:
        print("REFUSING: these names are not registered entries, so retiring them would be a")
        print("no-op recorded as a retirement - the thing this check exists to prevent:")
        for name in missing:
            print(f"  {name}")
        return 1

    before = len(followups)
    matrix["named_followups"] = [e for e in followups if e["id"] not in RETIRE]
    MATRIX.write_text(
        json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"named_followups {before} -> {len(matrix['named_followups'])} (-{len(RETIRE)})")
    for name, ground in RETIRE.items():
        print(f"  - {name}\n      {ground[:100]}...")
    print("\nEach was verified to be a registered entry BEFORE being counted.")
    print("Now re-run --residuals: a name orphaned by these removals must be re-homed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
