#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Executable encoding of the O03 `close(2)` endpoint failure post-state relation.

This is the machine-checkable companion to
``docs/design/OS_FAILURE_POSTSTATE_PROFILE.md``. It is a *design* artifact: it is
not imported by any production crate, it issues no verdict, and it promotes
nothing. Its only jobs are

1. to state the relation once, so the design document and its refusal tests
   cannot drift apart, and
2. to refuse forged post-states -- a claim that some outcome leaves the
   descriptor in a state the relation does not derive is rejected with a reason.

The relation is grounded in the primary sources cited in the design document
(Linux man-pages 6.17 ``close(2)``, dated 2026-02-08, sections DESCRIPTION,
RETURN VALUE, ERRORS and CAVEATS). It is **not** grounded in this repository's
suppressed-syscall fixtures, which record a wrapper that returned an error
without reaching the kernel and therefore cannot witness a real failure
post-state.

Nothing here is an operating-system durability theorem, a universal close
policy, or an extension of resource-contract v1.

Usage::

    python3 -B scripts/o03_close_poststate_relation.py --relation
    python3 -B scripts/o03_close_poststate_relation.py --probe [--output FILE]
"""

from __future__ import annotations

import argparse
import errno as errno_module
import json
import os
import platform
import stat
import sys
import tempfile
from dataclasses import dataclass, field
from typing import FrozenSet, Optional, Tuple

# --------------------------------------------------------------------------
# Targets
# --------------------------------------------------------------------------

#: Documented behaviour of the Linux kernel's close(2), for any Linux ABI target.
TARGET_LINUX = "linux"
#: Behaviour a strictly POSIX.1-2024 conforming implementation is required to
#: have. Linux is explicitly non-conforming here; see the design document.
TARGET_POSIX2024 = "posix2024"
#: No target pinned. Every outcome the two targets disagree on is indeterminate.
TARGET_UNPINNED = "unpinned"

TARGETS = (TARGET_LINUX, TARGET_POSIX2024, TARGET_UNPINNED)

#: The concrete target this card selected and probed.
SELECTED_TARGET_TRIPLE = "x86_64-unknown-linux-gnu"

# --------------------------------------------------------------------------
# Post-state vocabulary
# --------------------------------------------------------------------------

# Descriptor-slot post-states. All five are required by the card's acceptance
# item 1 and all five are reachable; see `reachable_descriptor_states`.
NOT_ATTEMPTED = "NotAttempted"
ATTEMPTED = "Attempted"
RETAINED = "Retained"
RELEASED = "Released"
INDETERMINATE = "Indeterminate"

DESCRIPTOR_STATES = (NOT_ATTEMPTED, ATTEMPTED, RETAINED, RELEASED, INDETERMINATE)

#: The two poisoned states. They differ in *why* the epoch is unresolved:
#: `Attempted` is an analysis gap (a better analysis could resolve it),
#: `Indeterminate` is a documentation gap (only pinning the target, or an
#: external oracle, can resolve it). Their duties are identical.
POISONED_STATES = (ATTEMPTED, INDETERMINATE)

# Open-file-description post-states.
DESCRIPTION_RETAINED = "DescriptionRetained"
DESCRIPTION_RELEASED = "DescriptionReleased"
DESCRIPTION_UNKNOWN = "DescriptionUnknown"

# Deferred write-back / data duty post-states. `Discharged` is never derived:
# close(2) CAVEATS states that a successful close does not guarantee the data
# reached the disk, so only fsync(2) could discharge this duty and close(2)
# never does.
WRITEBACK_FAILED = "WriteBackFailed"
WRITEBACK_UNKNOWN = "WriteBackUnknown"
WRITEBACK_DISCHARGED = "WriteBackDischarged"

# Residual duties.
DUTY_RELEASE = "must_release_epoch"
DUTY_NO_USE = "must_not_use_number"
DUTY_NO_RETRY = "must_not_retry_close"
DUTY_NO_REUSE_AS_FREE = "must_not_treat_number_as_free"
DUTY_KEEP_IN_DENOMINATOR = "epoch_stays_in_live_denominator"
DUTY_REPORT_UNRESOLVED = "must_report_unresolved_endpoint"
DUTY_REPORT_DATA_LOSS = "must_report_possible_data_loss"

#: The conservative residual duty set carried by both poisoned states. It
#: forbids *both* directions: the number may not be used and may not be retried
#: or recycled. No automatic retry is ever prescribed (acceptance item 3).
POISONED_DUTIES = frozenset(
    {
        DUTY_NO_USE,
        DUTY_NO_RETRY,
        DUTY_NO_REUSE_AS_FREE,
        DUTY_KEEP_IN_DENOMINATOR,
        DUTY_REPORT_UNRESOLVED,
    }
)

# Recovery operations a post-state may permit on the same number.
RECOVERY_CLOSE = "close"
RECOVERY_NONE: FrozenSet[str] = frozenset()

# Outcome kinds.
OUTCOME_NOT_ATTEMPTED = "not_attempted"
OUTCOME_RESULT_UNOBSERVED = "result_unobserved"
OUTCOME_ZERO = "returned_zero"
OUTCOME_MINUS_ONE = "returned_minus_one"

#: errno values close(2) ERRORS documents. EIO/ENOSPC/EDQUOT are the deferred
#: write-back family: the error is reported by close but was caused earlier.
ERRNO_DEFERRED_IO = ("EIO", "ENOSPC", "EDQUOT")
#: Sentinel for "the failure branch never read errno".
ERRNO_UNOBSERVED = "<unobserved>"

# Premise identifiers, as named in the design document.
A1_TARGET_PINNED = "A1_target_pinned"
A2_EPOCH_LIVE = "A2_epoch_live"
A3_SOLE_CUSTODIAN = "A3_sole_custodian"
A4_RESULT_OBSERVED = "A4_result_observed"
A5_ERRNO_OBSERVED = "A5_errno_observed"
A6_LAST_REFERENCE = "A6_last_reference"
A7_NO_RECORD_LOCKS = "A7_no_record_locks"


class RelationError(Exception):
    """The relation was asked about an input outside its declared domain."""


@dataclass(frozen=True)
class Epoch:
    """An OS endpoint identity: kind, number and acquisition generation.

    Contract spec C10: an endpoint is identified by kind, acquisition
    generation and lifetime, never by a numeric descriptor alone.
    """

    kind: str
    number: int
    generation: int

    def successor(self) -> "Epoch":
        return Epoch(self.kind, self.number, self.generation + 1)


@dataclass(frozen=True)
class Premises:
    """The call-site pre-state this relation requires. Every field is a premise
    the relation consumes; none of them is established here."""

    epoch_live: bool = True
    sole_custodian: bool = True
    result_observed: bool = True
    errno_observed: bool = True
    last_reference: bool = True
    no_record_locks: bool = True


@dataclass(frozen=True)
class Outcome:
    """One observed outcome of one `close(fd)` call."""

    kind: str
    errno_name: Optional[str] = None

    def __post_init__(self) -> None:
        if self.kind not in (
            OUTCOME_NOT_ATTEMPTED,
            OUTCOME_RESULT_UNOBSERVED,
            OUTCOME_ZERO,
            OUTCOME_MINUS_ONE,
        ):
            raise RelationError(f"unknown outcome kind {self.kind!r}")
        if self.kind == OUTCOME_MINUS_ONE and self.errno_name is None:
            raise RelationError("a -1 outcome must carry an errno name or the sentinel")
        if self.kind != OUTCOME_MINUS_ONE and self.errno_name is not None:
            raise RelationError("only a -1 outcome carries an errno name")


@dataclass(frozen=True)
class PostState:
    """The derived post-state of one call, per identity, plus its duties."""

    target: str
    descriptor: str
    description: str
    writeback: str
    duties: FrozenSet[str]
    permitted_recovery: FrozenSet[str]
    premise_violations: Tuple[str, ...] = field(default_factory=tuple)
    note: str = ""

    def is_poisoned(self) -> bool:
        return self.descriptor in POISONED_STATES

    def as_dict(self) -> dict:
        return {
            "target": self.target,
            "descriptor": self.descriptor,
            "description": self.description,
            "writeback": self.writeback,
            "duties": sorted(self.duties),
            "permitted_recovery": sorted(self.permitted_recovery),
            "premise_violations": list(self.premise_violations),
            "note": self.note,
        }


def _poisoned(
    target: str,
    state: str,
    reason: str,
    *,
    violations: Tuple[str, ...] = (),
) -> PostState:
    assert state in POISONED_STATES
    return PostState(
        target=target,
        descriptor=state,
        description=DESCRIPTION_UNKNOWN,
        writeback=WRITEBACK_UNKNOWN,
        duties=POISONED_DUTIES,
        permitted_recovery=RECOVERY_NONE,
        premise_violations=violations,
        note=reason,
    )


def _description_after_release(premises: Premises) -> str:
    """close(2) DESCRIPTION: the open file description is freed only when the
    closed descriptor was its last reference. A live dup, a forked child or an
    in-flight syscall in another thread all keep it alive."""
    return DESCRIPTION_RELEASED if premises.last_reference else DESCRIPTION_RETAINED


def close_poststate(target: str, premises: Premises, outcome: Outcome) -> PostState:
    """The relation R_close[target] : Premises x Outcome -> PostState.

    Fail-closed by construction: every arm the cited documentation does not
    determine yields a poisoned descriptor state with the full conservative
    residual duty set, and no arm ever permits an automatic retry.
    """
    if target not in TARGETS:
        raise RelationError(f"unknown target {target!r}")

    # A7 is not consumed by any conclusion below; close(2) DESCRIPTION records
    # that closing removes every POSIX record lock the process holds on the
    # file, whichever descriptor took the lock. That footprint is outside every
    # identity this relation tracks, so it is refused rather than modelled.
    if not premises.no_record_locks:
        return _poisoned(
            target,
            INDETERMINATE,
            "A7 denied: close removes process-wide record locks on the file, an "
            "effect outside every modelled identity",
            violations=(A7_NO_RECORD_LOCKS,),
        )

    if outcome.kind == OUTCOME_NOT_ATTEMPTED:
        if not premises.epoch_live:
            raise RelationError("A2 denied: there is no epoch to describe")
        return PostState(
            target=target,
            descriptor=NOT_ATTEMPTED,
            description=DESCRIPTION_RETAINED,
            writeback=WRITEBACK_UNKNOWN,
            duties=frozenset({DUTY_RELEASE, DUTY_KEEP_IN_DENOMINATOR}),
            permitted_recovery=frozenset({RECOVERY_CLOSE}),
            note="no call was issued on this path; the epoch is untouched",
        )

    if outcome.kind == OUTCOME_RESULT_UNOBSERVED or not premises.result_observed:
        # A discarded int result cannot be separated from the EBADF arm, which
        # is the one documented arm that does not release. Analysis gap.
        return _poisoned(
            target,
            ATTEMPTED,
            "A4 denied: the result was not observed, so the EBADF arm is not "
            "excluded and no release can be concluded",
        )

    if outcome.kind == OUTCOME_ZERO:
        # close(2) RETURN VALUE: zero on success.
        #
        # A3 is load-bearing here and is not decoration: a successful close
        # proves that *some* epoch on that number was closed. Only sole
        # custody identifies it with the epoch the analysis was tracking.
        if not premises.sole_custodian:
            return _poisoned(
                target,
                INDETERMINATE,
                "A3 denied: a successful close identifies the epoch it closed "
                "only under sole custody of the number",
                violations=(A3_SOLE_CUSTODIAN,),
            )
        return PostState(
            target=target,
            descriptor=RELEASED,
            description=_description_after_release(premises),
            # close(2) CAVEATS: a successful close does not guarantee the data
            # reached the disk. Success discharges the descriptor, never the data.
            writeback=WRITEBACK_UNKNOWN,
            duties=frozenset(),
            permitted_recovery=RECOVERY_NONE,
            note="epoch ended; the number may be reused by a fresh generation",
        )

    # outcome.kind == OUTCOME_MINUS_ONE
    name = outcome.errno_name
    if name == ERRNO_UNOBSERVED or not premises.errno_observed:
        # -1 alone does not select an arm: EBADF and the release-anyway family
        # are indistinguishable without errno. Analysis gap.
        return _poisoned(
            target,
            ATTEMPTED,
            "A5 denied: errno was not observed, so the EBADF arm is not "
            "excluded and no release can be concluded",
        )

    if name == "EBADF":
        # close(2) ERRORS: "fd isn't a valid open file descriptor", and CAVEATS
        # names EBADF as the one case in which implementations do not close.
        # Under A2 that is a contradiction, not information: either the epoch
        # was already gone (double close) or another custodian took it. Note
        # that the *dangerous* double close does not report EBADF at all -- if
        # the number had been recycled it would have returned zero, having
        # closed somebody else's epoch.
        violations = (A2_EPOCH_LIVE,) if premises.epoch_live else ()
        return PostState(
            target=target,
            descriptor=INDETERMINATE,
            description=DESCRIPTION_UNKNOWN,
            writeback=WRITEBACK_UNKNOWN,
            duties=POISONED_DUTIES,
            permitted_recovery=RECOVERY_NONE,
            premise_violations=violations,
            note="EBADF refutes the tracked epoch; this call ended no epoch",
        )

    if name == "EINTR":
        # The divergence this card exists to record.
        if target == TARGET_LINUX:
            # close(2) CAVEATS: the Linux kernel always releases the descriptor
            # early in the close operation; the error-producing steps happen
            # later. Retrying may close a descriptor another thread reused.
            return PostState(
                target=target,
                descriptor=RELEASED,
                description=DESCRIPTION_UNKNOWN,
                writeback=WRITEBACK_FAILED,
                duties=frozenset({DUTY_REPORT_DATA_LOSS}),
                permitted_recovery=RECOVERY_NONE,
                note="Linux releases the descriptor early; retry is forbidden",
            )
        if target == TARGET_POSIX2024:
            # close(2) CAVEATS: POSIX.1-2024 standardized the HP-UX behaviour,
            # in which the descriptor stays open and close must be called again.
            return PostState(
                target=target,
                descriptor=RETAINED,
                description=DESCRIPTION_RETAINED,
                writeback=WRITEBACK_FAILED,
                duties=frozenset(
                    {DUTY_RELEASE, DUTY_KEEP_IN_DENOMINATOR, DUTY_REPORT_DATA_LOSS}
                ),
                permitted_recovery=frozenset({RECOVERY_CLOSE}),
                note="POSIX.1-2024 keeps the descriptor open; close must be called again",
            )
        # Unpinned: Released (Linux) joined with Retained (POSIX.1-2024). The
        # two targets do not merely differ in what is known -- they prescribe
        # *opposite* recovery. Retry is required by one and forbidden by the
        # other, so no recovery is permitted without pinning the target.
        return PostState(
            target=target,
            descriptor=INDETERMINATE,
            description=DESCRIPTION_UNKNOWN,
            writeback=WRITEBACK_FAILED,
            duties=POISONED_DUTIES | {DUTY_REPORT_DATA_LOSS},
            permitted_recovery=RECOVERY_NONE,
            note=(
                "documentation-level divergence: Linux forbids the retry that "
                "POSIX.1-2024 requires; the target must be pinned"
            ),
        )

    if name in ERRNO_DEFERRED_IO:
        # close(2) ERRORS and CAVEATS: an earlier write's error surfaces at the
        # close that releases the open file description. Data may be lost.
        if target == TARGET_LINUX:
            return PostState(
                target=target,
                descriptor=RELEASED,
                description=_description_after_release(premises),
                writeback=WRITEBACK_FAILED,
                duties=frozenset({DUTY_REPORT_DATA_LOSS}),
                permitted_recovery=RECOVERY_NONE,
                note="Linux released the descriptor before the failing flush step",
            )
        # POSIX.1-2008 was silent on whether other errors close the descriptor,
        # and POSIX.1-2024 legislates only the EINTR case. Nothing to conclude.
        return PostState(
            target=target,
            descriptor=INDETERMINATE,
            description=DESCRIPTION_UNKNOWN,
            writeback=WRITEBACK_FAILED,
            duties=POISONED_DUTIES | {DUTY_REPORT_DATA_LOSS},
            permitted_recovery=RECOVERY_NONE,
            note="the standard states no general release rule for this error",
        )

    # Any other errno: documented by neither ERRORS list entry nor a release
    # rule outside Linux.
    if target == TARGET_LINUX:
        return PostState(
            target=target,
            descriptor=RELEASED,
            description=DESCRIPTION_UNKNOWN,
            writeback=WRITEBACK_UNKNOWN,
            duties=frozenset(),
            permitted_recovery=RECOVERY_NONE,
            note="Linux always releases except on EBADF",
        )
    return PostState(
        target=target,
        descriptor=INDETERMINATE,
        description=DESCRIPTION_UNKNOWN,
        writeback=WRITEBACK_UNKNOWN,
        duties=POISONED_DUTIES,
        permitted_recovery=RECOVERY_NONE,
        note="the standard states no general release rule for this error",
    )


def join(left: PostState, right: PostState) -> PostState:
    """Join two post-states reaching the same program point.

    Disagreeing descriptor states widen to `Indeterminate` with the full
    conservative duty set; agreement is preserved exactly. A join never
    produces a permitted recovery that both sides did not permit.
    """
    if left.target != right.target:
        raise RelationError("cannot join post-states of different targets")
    if left.descriptor == right.descriptor:
        descriptor = left.descriptor
        duties = left.duties | right.duties
        recovery = left.permitted_recovery & right.permitted_recovery
    else:
        # Disagreement widens to the poisoned state. `must_release_epoch` is
        # dropped because it contradicts `must_not_retry_close`: a poisoned
        # epoch is neither releasable nor usable, and the duty to report it
        # replaces the duty to release it.
        descriptor = INDETERMINATE
        duties = (POISONED_DUTIES | left.duties | right.duties) - {DUTY_RELEASE}
        recovery = RECOVERY_NONE
    description = (
        left.description if left.description == right.description else DESCRIPTION_UNKNOWN
    )
    if left.writeback == right.writeback:
        writeback = left.writeback
    elif WRITEBACK_FAILED in (left.writeback, right.writeback):
        writeback = WRITEBACK_FAILED
    else:
        writeback = WRITEBACK_UNKNOWN
    return PostState(
        target=left.target,
        descriptor=descriptor,
        description=description,
        writeback=writeback,
        duties=frozenset(duties),
        permitted_recovery=recovery,
        premise_violations=tuple(
            dict.fromkeys(left.premise_violations + right.premise_violations)
        ),
        note="join",
    )


def check_claim(target: str, premises: Premises, outcome: Outcome, claim: dict) -> Optional[str]:
    """Refuse a forged post-state.

    Returns ``None`` when every claimed field agrees with the derived
    post-state, and a refusal reason otherwise. Only the fields present in
    ``claim`` are compared, so a partial claim can be checked, but a claim can
    never widen the relation.
    """
    derived = close_poststate(target, premises, outcome).as_dict()
    for key, value in claim.items():
        if key not in derived:
            return f"refused: unknown post-state field {key!r}"
        expected = derived[key]
        if isinstance(expected, list):
            if sorted(value) != expected:
                return (
                    f"refused: claimed {key}={sorted(value)}, "
                    f"the relation derives {expected}"
                )
        elif value != expected:
            return f"refused: claimed {key}={value!r}, the relation derives {expected!r}"
    return None


def epochs_are_distinct(before: Epoch, after: Epoch) -> bool:
    """C10: numeric reuse after an epoch has ended must not merge the epochs."""
    return before != after


def reachable_descriptor_states(target: str) -> FrozenSet[str]:
    """Every descriptor state the relation actually derives for a target.

    Used by the tests to show the five-way taxonomy is not decorative.
    """
    probes = [
        (Premises(), Outcome(OUTCOME_NOT_ATTEMPTED)),
        (Premises(), Outcome(OUTCOME_RESULT_UNOBSERVED)),
        (Premises(), Outcome(OUTCOME_ZERO)),
        (Premises(), Outcome(OUTCOME_MINUS_ONE, "EBADF")),
        (Premises(), Outcome(OUTCOME_MINUS_ONE, "EINTR")),
        (Premises(), Outcome(OUTCOME_MINUS_ONE, "EIO")),
        (Premises(), Outcome(OUTCOME_MINUS_ONE, ERRNO_UNOBSERVED)),
    ]
    states = {close_poststate(target, p, o).descriptor for p, o in probes}
    # `Retained` is only reachable on the POSIX.1-2024 target and by the
    # not-attempted arm; the union over targets covers all five.
    return frozenset(states)


# --------------------------------------------------------------------------
# Real-Linux probe
# --------------------------------------------------------------------------


def _descriptor_is_absent(number: int) -> bool:
    """A real F_GETFD probe. EBADF means the slot holds no epoch."""
    import fcntl

    try:
        fcntl.fcntl(number, fcntl.F_GETFD)
    except OSError as exc:
        return exc.errno == errno_module.EBADF
    return False


def _identity(number: int) -> Tuple[int, int]:
    info = os.fstat(number)
    return (info.st_dev, info.st_ino)


def probe_linux() -> dict:
    """Execute real, unsuppressed `close(2)` calls and record what they leave.

    Every syscall here reaches the kernel. This probe deliberately observes
    only outcomes that are reproducible without special hardware: success and
    EBADF. It therefore *cannot* witness the EINTR/EIO post-states, which the
    design takes from the man page alone -- and saying so is the point of the
    card.
    """
    if platform.system() != "Linux":
        raise RelationError("this probe measures Linux close(2) semantics only")

    observations = []
    with tempfile.TemporaryDirectory(prefix="o03-close-") as root:
        first_path = os.path.join(root, "first")
        second_path = os.path.join(root, "second")

        # P1: a successful close releases the descriptor slot.
        number = os.open(first_path, os.O_CREAT | os.O_RDWR, stat.S_IRUSR | stat.S_IWUSR)
        first_identity = _identity(number)
        os.write(number, b"o03")
        os.close(number)
        observations.append(
            {
                "probe": "P1_success_releases_the_slot",
                "outcome": OUTCOME_ZERO,
                "number": number,
                "descriptor_absent_after": _descriptor_is_absent(number),
                "relation_descriptor": close_poststate(
                    TARGET_LINUX, Premises(), Outcome(OUTCOME_ZERO)
                ).descriptor,
            }
        )

        # P2: closing the same number again reports EBADF and ends no epoch.
        try:
            os.close(number)
            second_errno = None
        except OSError as exc:
            second_errno = errno_module.errorcode.get(exc.errno, str(exc.errno))
        observations.append(
            {
                "probe": "P2_second_close_reports_EBADF",
                "outcome": OUTCOME_MINUS_ONE,
                "errno": second_errno,
                "relation_descriptor": close_poststate(
                    TARGET_LINUX, Premises(), Outcome(OUTCOME_MINUS_ONE, "EBADF")
                ).descriptor,
            }
        )

        # P3: the number is handed straight back to a fresh, unrelated epoch.
        # This is the measured reason the relation forbids an automatic retry:
        # a retried close on a failed number can close somebody else's epoch,
        # and that call returns zero rather than EBADF.
        reused = os.open(second_path, os.O_CREAT | os.O_RDWR, stat.S_IRUSR | stat.S_IWUSR)
        reused_identity = _identity(reused)
        observations.append(
            {
                "probe": "P3_number_is_reused_by_a_fresh_epoch",
                "number_reused": reused == number,
                "first_identity": list(first_identity),
                "second_identity": list(reused_identity),
                "identities_distinct": first_identity != reused_identity,
                "epochs_distinct": epochs_are_distinct(
                    Epoch("descriptor", number, 1), Epoch("descriptor", reused, 2)
                ),
            }
        )

        # P4: descriptor release is not open-file-description release. The dup
        # keeps the description alive after the original slot is gone.
        duplicate = os.dup(reused)
        os.close(reused)
        duplicate_identity = _identity(duplicate)
        observations.append(
            {
                "probe": "P4_description_outlives_the_descriptor",
                "closed_number_absent_after": _descriptor_is_absent(reused),
                "duplicate_still_usable": duplicate_identity == reused_identity,
                "relation_description_without_last_reference": close_poststate(
                    TARGET_LINUX,
                    Premises(last_reference=False),
                    Outcome(OUTCOME_ZERO),
                ).description,
            }
        )
        os.close(duplicate)

    return {
        "artifact": "real unsuppressed close(2)/open(2)/dup(2) calls",
        "host": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "target": SELECTED_TARGET_TRIPLE,
        "primary_source": "Linux man-pages 6.17, close(2), dated 2026-02-08",
        "observations": observations,
        "not_observed": [
            "EINTR: not reproducible without an interrupting signal on a "
            "filesystem whose close is interruptible",
            "EIO/ENOSPC/EDQUOT: not reproducible without a failing device, a "
            "full filesystem or an exhausted quota",
        ],
        "claims": [
            "these are real syscalls, not suppressed wrappers",
            "they witness the success and EBADF arms only",
            "no durability, no general OS failure rule and no verdict follows",
        ],
    }


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------


def _relation_table() -> list:
    rows = []
    cases = [
        (Outcome(OUTCOME_NOT_ATTEMPTED), Premises()),
        (Outcome(OUTCOME_RESULT_UNOBSERVED), Premises(result_observed=False)),
        (Outcome(OUTCOME_ZERO), Premises()),
        (Outcome(OUTCOME_ZERO), Premises(last_reference=False)),
        (Outcome(OUTCOME_MINUS_ONE, "EBADF"), Premises()),
        (Outcome(OUTCOME_MINUS_ONE, "EINTR"), Premises()),
        (Outcome(OUTCOME_MINUS_ONE, "EIO"), Premises()),
        (Outcome(OUTCOME_MINUS_ONE, "ENOSPC"), Premises()),
        (Outcome(OUTCOME_MINUS_ONE, "ENOTSUP"), Premises()),
        (Outcome(OUTCOME_MINUS_ONE, ERRNO_UNOBSERVED), Premises(errno_observed=False)),
    ]
    for target in TARGETS:
        for outcome, premises in cases:
            rows.append(
                {
                    "target": target,
                    "outcome": outcome.kind,
                    "errno": outcome.errno_name,
                    "last_reference": premises.last_reference,
                    **close_poststate(target, premises, outcome).as_dict(),
                }
            )
    return rows


def main(argv: Optional[list] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--relation", action="store_true", help="print the relation table")
    parser.add_argument("--probe", action="store_true", help="run the real Linux probe")
    parser.add_argument("--output", help="write the probe JSON to this path")
    args = parser.parse_args(argv)

    if not args.relation and not args.probe:
        parser.print_help()
        return 2

    payload = {}
    if args.relation:
        payload["relation"] = _relation_table()
    if args.probe:
        payload["probe"] = probe_linux()

    text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    if args.output:
        with open(args.output, "w", encoding="utf-8") as handle:
            handle.write(text)
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
