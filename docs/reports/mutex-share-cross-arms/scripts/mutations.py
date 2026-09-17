#!/usr/bin/env python3
"""Falsify every arm this card claims to have driven or proved.

Card `MUTEX-SHARE-CROSS-ARMS` (wave 35), discharging `STABILITY-REEVAL` #1's
residual `MUTEX-SHARE-CROSS-UNEXERCISED-ARMS`.

The finding this card answers is that a green suite is evidence about the paths
it took. A test that cannot tell an arm from its negation verifies nothing, so
each mutation below negates one arm and the run records whether the test that
claims to cover it actually goes red. A mutation that leaves the suite green is
a finding, not a convenience.

The two Lean mutations are the other half: the two dead arms are justified by
theorems rather than by tests, so the theorems get the same treatment — mutate
the proved statement, and the build must fail.

Run from the repository root:

    python3 docs/reports/mutex-share-cross-arms/scripts/mutations.py

Exit 0 means every mutation was caught. Any survivor is printed and the exit is
nonzero.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]

LOWERING = ROOT / "crates/qcore-frontend/src/source_mutex.rs"
JUDGMENT = ROOT / "crates/qcore-core/src/transfer.rs"
PROOF = ROOT / "proof/QCore/MutexShareCrossArms.lean"

ARM_A_MESSAGE = '"a declared crossing names the payload it shares",'
ARM_B_MESSAGE = (
    '"a declared crossing names the mutex generation that protects the payload",'
)

PASS_THROUGH_BEFORE = """            TransferMode::ShareUnderMutex => match shareable_under {
                Some(ShareProtocol::CheckedMutex) => Ok(AdmittedTransfer::ShareUnderMutex(
                    ShareProtocol::CheckedMutex,
                )),
                Some(other) => Err(TransferRefusal::ProtocolUnavailable(other)),"""
PASS_THROUGH_AFTER = """            TransferMode::ShareUnderMutex => match shareable_under {
                Some(protocol) => Ok(AdmittedTransfer::ShareUnderMutex(protocol)),
                Some(_other) => Err(TransferRefusal::ProtocolUnavailable(_other)),"""

SECOND_KIND_BEFORE = """        TransferKind::OwnedBuffer => TransferClass::MovedOnly {
            shareable_under: Some(ShareProtocol::CountedReadShares),
            mutably_shareable_under: Some(ShareProtocol::DisjointWriteLeases),
        },"""
SECOND_KIND_AFTER = """        TransferKind::OwnedBuffer => TransferClass::MovedOnly {
            shareable_under: Some(ShareProtocol::CheckedMutex),
            mutably_shareable_under: Some(ShareProtocol::CheckedMutex),
        },"""


@dataclass
class Mutation:
    """One negated arm, and the check that must notice."""

    name: str
    arm: str
    path: Path
    before: str
    after: str
    # Rust: test names that must fail. Lean: empty, and `lean` is set.
    must_fail: list[str] = field(default_factory=list)
    lean: bool = False
    swap: bool = False


MUTATIONS = [
    Mutation(
        name="arm-a-message",
        arm="A (payload position is not a name)",
        path=LOWERING,
        before=ARM_A_MESSAGE,
        after=ARM_B_MESSAGE,
        must_fail=["a_crossing_whose_payload_position_is_not_a_name"],
    ),
    Mutation(
        name="arm-b-message",
        arm="B (mutex position is not a name)",
        path=LOWERING,
        before=ARM_B_MESSAGE,
        after=ARM_A_MESSAGE,
        must_fail=["a_crossing_whose_mutex_position_is_not_a_name"],
    ),
    Mutation(
        name="arm-ab-positions-swapped",
        arm="A and B (which header position each arm reads)",
        path=LOWERING,
        before="name(header.get(4))",
        after="name(header.get(6))",
        swap=True,
        must_fail=[
            "a_crossing_whose_payload_position_is_not_a_name",
            "a_crossing_whose_mutex_position_is_not_a_name",
        ],
    ),
    Mutation(
        name="arm-c-pass-through",
        arm="C (the Ok(other) protocol residue)",
        path=JUDGMENT,
        before=PASS_THROUGH_BEFORE,
        after=PASS_THROUGH_AFTER,
        must_fail=[
            "the_protocol_residue_is_empty_over_every_kind",
            "only_the_protected_payload_reaches_the_lookup",
        ],
    ),
    Mutation(
        name="arm-d-second-sharing-kind",
        arm="D (the payload lookup that cannot miss)",
        path=JUDGMENT,
        before=SECOND_KIND_BEFORE,
        after=SECOND_KIND_AFTER,
        must_fail=[
            "only_the_protected_payload_reaches_the_lookup",
            "exactly_one_kind_carries_the_payloads_class",
        ],
    ),
    Mutation(
        name="lean-lookup-always-hits",
        arm="D's theorem (theLookupAlwaysHits)",
        path=PROOF,
        before="        k = .protectedResource := by\n  decide",
        after="        k = .ownedBuffer := by\n  decide",
        lean=True,
    ),
    Mutation(
        name="lean-class-form-is-the-judgment",
        arm="D's mirror (theClassFormIsTheJudgment / admitsOfClass)",
        path=PROOF,
        before="""      | .shareUnderMutex => match p with
        | some .checkedMutex => some (.shareUnderMutex .checkedMutex)
        | _ => none""",
        after="""      | .shareUnderMutex => none""",
        lean=True,
    ),
    Mutation(
        name="lean-protocol-residue-is-empty",
        arm="C's theorem (theProtocolResidueIsEmpty)",
        path=PROOF,
        before="""        admitsCapture k .shareUnderMutex .mutexShare
          = some (.shareUnderMutex .checkedMutex) := by
  decide""",
        after="""        admitsCapture k .shareUnderMutex .mutexShare
          = some (.shareUnderMutex .countedReadShares) := by
  decide""",
        lean=True,
    ),
]


def apply(text: str, mutation: Mutation) -> str:
    if mutation.swap:
        if mutation.before not in text or mutation.after not in text:
            raise SystemExit(f"{mutation.name}: a swap target is missing")
        sentinel = "\x00SWAP\x00"
        text = text.replace(mutation.before, sentinel)
        text = text.replace(mutation.after, mutation.before)
        return text.replace(sentinel, mutation.after)
    count = text.count(mutation.before)
    if count != 1:
        raise SystemExit(
            f"{mutation.name}: expected its target exactly once, found {count}. "
            "The source moved; re-aim the mutation rather than relaxing it."
        )
    return text.replace(mutation.before, mutation.after)


def rust_failures() -> set[str]:
    """Run the card's test file and return the names that failed."""
    result = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "qcore-frontend",
            "--test",
            "mutex_share_cross_arms",
            "--",
            "--test-threads=1",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    failed = set()
    for line in (result.stdout + result.stderr).splitlines():
        line = line.strip()
        if line.startswith("test ") and line.endswith("FAILED"):
            failed.add(line.split()[1])
    # A build failure counts as caught for the mutations that change a signature
    # only; none of these do, so surface it instead of silently passing.
    if result.returncode != 0 and not failed:
        failed.add("<build or harness failure>")
    return failed


def lean_builds() -> bool:
    result = subprocess.run(
        ["lake", "build", "QCore.MutexShareCrossArms"],
        cwd=ROOT / "proof",
        capture_output=True,
        text=True,
    )
    return result.returncode == 0


def main() -> int:
    survivors = []
    for mutation in MUTATIONS:
        original = mutation.path.read_text()
        backup = Path(tempfile.mkdtemp()) / mutation.path.name
        shutil.copy2(mutation.path, backup)
        try:
            mutation.path.write_text(apply(original, mutation))
            if mutation.lean:
                caught = not lean_builds()
                detail = "build failed" if caught else "BUILT ANYWAY"
            else:
                failed = rust_failures()
                missing = [
                    name
                    for name in mutation.must_fail
                    if not any(name in f for f in failed)
                ]
                caught = not missing
                detail = (
                    f"{len(failed)} test(s) red"
                    if caught
                    else f"SURVIVED for {missing}"
                )
        finally:
            shutil.copy2(backup, mutation.path)
            shutil.rmtree(backup.parent, ignore_errors=True)
        status = "caught" if caught else "SURVIVED"
        print(f"[{status:8}] {mutation.name}: arm {mutation.arm} -- {detail}")
        if not caught:
            survivors.append(mutation.name)

    if mutation_restore_check():
        print("\nrestored: every mutated file is back to its committed content")
    else:
        print("\nWARNING: a mutated file did not restore cleanly", file=sys.stderr)
        return 2

    if survivors:
        print(f"\n{len(survivors)} mutation(s) survived: {survivors}", file=sys.stderr)
        return 1
    print(f"\nall {len(MUTATIONS)} mutations caught")
    return 0


def mutation_restore_check() -> bool:
    result = subprocess.run(
        ["git", "diff", "--name-only", "--", str(LOWERING), str(JUDGMENT), str(PROOF)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    # The lowering and the proof carry this card's own committed edits, so a
    # clean restore is "no *new* difference against the index", which is what
    # `git diff` against a staged tree reports. Callers run this on a clean
    # tree; a nonempty result here means a mutation leaked.
    return result.returncode == 0 and not result.stdout.strip()


if __name__ == "__main__":
    sys.exit(main())
