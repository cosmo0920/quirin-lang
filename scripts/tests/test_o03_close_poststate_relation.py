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

"""Validation harness for the O03 `close(2)` failure post-state relation.

Every test either pins a post-state the design document derives, or shows that a
*forged* post-state is refused. A refusal test is the load-bearing kind: the
relation is only worth writing down if it rejects the convenient answer.

The probe tests execute real, unsuppressed syscalls on Linux. They are skipped
elsewhere. Nothing here promotes a verdict, exercises a production crate, or
claims an operating-system durability result.
"""

import platform
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import o03_close_poststate_relation as rel  # noqa: E402


LINUX = rel.TARGET_LINUX
POSIX = rel.TARGET_POSIX2024
UNPINNED = rel.TARGET_UNPINNED


def linux(outcome, **premises):
    return rel.close_poststate(LINUX, rel.Premises(**premises), outcome)


def minus_one(name):
    return rel.Outcome(rel.OUTCOME_MINUS_ONE, name)


ZERO = rel.Outcome(rel.OUTCOME_ZERO)
NOT_ATTEMPTED = rel.Outcome(rel.OUTCOME_NOT_ATTEMPTED)
UNOBSERVED = rel.Outcome(rel.OUTCOME_RESULT_UNOBSERVED)


class TaxonomyTest(unittest.TestCase):
    """Acceptance item 1: five distinguished post-states, all reachable."""

    def test_all_five_descriptor_states_are_reachable(self):
        reached = set()
        for target in rel.TARGETS:
            reached |= rel.reachable_descriptor_states(target)
        reached.add(rel.close_poststate(POSIX, rel.Premises(), minus_one("EINTR")).descriptor)
        self.assertEqual(reached, set(rel.DESCRIPTOR_STATES))

    def test_not_attempted_is_distinct_from_attempted(self):
        not_attempted = linux(NOT_ATTEMPTED)
        attempted = linux(UNOBSERVED, result_observed=False)
        self.assertEqual(not_attempted.descriptor, rel.NOT_ATTEMPTED)
        self.assertEqual(attempted.descriptor, rel.ATTEMPTED)
        # Not-attempted still owes a release and may still close; attempted
        # owes the conservative residual and may do nothing.
        self.assertIn(rel.DUTY_RELEASE, not_attempted.duties)
        self.assertIn(rel.RECOVERY_CLOSE, not_attempted.permitted_recovery)
        self.assertNotIn(rel.DUTY_RELEASE, attempted.duties)
        self.assertEqual(attempted.permitted_recovery, rel.RECOVERY_NONE)

    def test_attempted_and_indeterminate_share_duties_but_not_identity(self):
        attempted = linux(UNOBSERVED, result_observed=False)
        indeterminate = rel.close_poststate(UNPINNED, rel.Premises(), minus_one("EINTR"))
        self.assertEqual(attempted.descriptor, rel.ATTEMPTED)
        self.assertEqual(indeterminate.descriptor, rel.INDETERMINATE)
        self.assertTrue(rel.POISONED_DUTIES <= attempted.duties)
        self.assertTrue(rel.POISONED_DUTIES <= indeterminate.duties)


class LinuxRelationTest(unittest.TestCase):
    """The arms the man page determines for the selected target."""

    def test_success_releases_the_descriptor(self):
        self.assertEqual(linux(ZERO).descriptor, rel.RELEASED)

    def test_eintr_releases_the_descriptor_on_linux(self):
        state = linux(minus_one("EINTR"))
        self.assertEqual(state.descriptor, rel.RELEASED)
        self.assertEqual(state.writeback, rel.WRITEBACK_FAILED)

    def test_deferred_io_errors_release_the_descriptor_on_linux(self):
        for name in rel.ERRNO_DEFERRED_IO:
            with self.subTest(errno=name):
                state = linux(minus_one(name))
                self.assertEqual(state.descriptor, rel.RELEASED)
                self.assertEqual(state.writeback, rel.WRITEBACK_FAILED)
                self.assertIn(rel.DUTY_REPORT_DATA_LOSS, state.duties)

    def test_ebadf_is_the_one_arm_that_releases_nothing(self):
        state = linux(minus_one("EBADF"))
        self.assertEqual(state.descriptor, rel.INDETERMINATE)
        self.assertIn(rel.A2_EPOCH_LIVE, state.premise_violations)

    def test_an_unobserved_errno_cannot_conclude_a_release(self):
        state = linux(minus_one(rel.ERRNO_UNOBSERVED), errno_observed=False)
        self.assertEqual(state.descriptor, rel.ATTEMPTED)

    def test_success_never_discharges_the_data_duty(self):
        # close(2) CAVEATS: a successful close does not guarantee the data was
        # saved. No arm of the relation ever derives WriteBackDischarged.
        derived = set()
        for target in rel.TARGETS:
            for outcome in (
                NOT_ATTEMPTED,
                ZERO,
                minus_one("EBADF"),
                minus_one("EINTR"),
                minus_one("EIO"),
                minus_one("ENOTSUP"),
            ):
                derived.add(rel.close_poststate(target, rel.Premises(), outcome).writeback)
        self.assertNotIn(rel.WRITEBACK_DISCHARGED, derived)


class TargetDivergenceTest(unittest.TestCase):
    """Why the profile is target-qualified rather than portable."""

    def test_posix2024_retains_the_descriptor_on_eintr(self):
        state = rel.close_poststate(POSIX, rel.Premises(), minus_one("EINTR"))
        self.assertEqual(state.descriptor, rel.RETAINED)
        self.assertIn(rel.RECOVERY_CLOSE, state.permitted_recovery)

    def test_the_two_targets_prescribe_opposite_recovery_on_eintr(self):
        on_linux = linux(minus_one("EINTR"))
        on_posix = rel.close_poststate(POSIX, rel.Premises(), minus_one("EINTR"))
        self.assertNotIn(rel.RECOVERY_CLOSE, on_linux.permitted_recovery)
        self.assertIn(rel.RECOVERY_CLOSE, on_posix.permitted_recovery)

    def test_an_unpinned_target_is_indeterminate_and_permits_no_retry(self):
        # Acceptance item 3: no automatic retry is prescribed for an
        # indeterminate descriptor, precisely because one target requires the
        # retry the other forbids.
        state = rel.close_poststate(UNPINNED, rel.Premises(), minus_one("EINTR"))
        self.assertEqual(state.descriptor, rel.INDETERMINATE)
        self.assertEqual(state.permitted_recovery, rel.RECOVERY_NONE)
        self.assertIn(rel.DUTY_NO_RETRY, state.duties)
        self.assertIn(rel.DUTY_NO_USE, state.duties)


class ForgedPostStateTest(unittest.TestCase):
    """Refusals. Each of these is a claim someone could plausibly want."""

    def _refused(self, target, outcome, claim, premises=None):
        reason = rel.check_claim(target, premises or rel.Premises(), outcome, claim)
        self.assertIsNotNone(reason, f"claim {claim} was admitted and should not be")
        return reason

    def test_claiming_eintr_retains_on_linux_is_refused(self):
        self._refused(LINUX, minus_one("EINTR"), {"descriptor": rel.RETAINED})

    def test_claiming_eintr_releases_on_posix2024_is_refused(self):
        self._refused(POSIX, minus_one("EINTR"), {"descriptor": rel.RELEASED})

    def test_claiming_a_retry_is_permitted_after_a_failed_linux_close_is_refused(self):
        self._refused(
            LINUX, minus_one("EIO"), {"permitted_recovery": [rel.RECOVERY_CLOSE]}
        )

    def test_claiming_a_retry_is_permitted_from_an_indeterminate_state_is_refused(self):
        self._refused(
            UNPINNED, minus_one("EINTR"), {"permitted_recovery": [rel.RECOVERY_CLOSE]}
        )

    def test_claiming_ebadf_released_the_epoch_is_refused(self):
        self._refused(LINUX, minus_one("EBADF"), {"descriptor": rel.RELEASED})

    def test_claiming_an_unobserved_result_released_the_epoch_is_refused(self):
        self._refused(
            LINUX,
            UNOBSERVED,
            {"descriptor": rel.RELEASED},
            premises=rel.Premises(result_observed=False),
        )

    def test_claiming_success_discharged_the_data_duty_is_refused(self):
        self._refused(LINUX, ZERO, {"writeback": rel.WRITEBACK_DISCHARGED})

    def test_claiming_a_failed_close_discharged_the_data_duty_is_refused(self):
        self._refused(LINUX, minus_one("EIO"), {"writeback": rel.WRITEBACK_DISCHARGED})

    def test_claiming_the_description_died_without_the_last_reference_is_refused(self):
        self._refused(
            LINUX,
            ZERO,
            {"description": rel.DESCRIPTION_RELEASED},
            premises=rel.Premises(last_reference=False),
        )

    def test_claiming_success_identifies_the_epoch_without_sole_custody_is_refused(self):
        self._refused(
            LINUX,
            ZERO,
            {"descriptor": rel.RELEASED},
            premises=rel.Premises(sole_custodian=False),
        )

    def test_claiming_a_poisoned_epoch_leaves_the_denominator_is_refused(self):
        self._refused(UNPINNED, minus_one("EINTR"), {"duties": []})

    def test_an_unknown_post_state_field_is_refused(self):
        reason = self._refused(LINUX, ZERO, {"descriptor_is_fine": True})
        self.assertIn("unknown post-state field", reason)

    def test_the_true_post_state_is_admitted(self):
        # The refusals above are not vacuous: the derived answer is accepted.
        self.assertIsNone(
            rel.check_claim(
                LINUX,
                rel.Premises(),
                minus_one("EIO"),
                {
                    "descriptor": rel.RELEASED,
                    "writeback": rel.WRITEBACK_FAILED,
                    "permitted_recovery": [],
                },
            )
        )


class EpochIdentityTest(unittest.TestCase):
    """Acceptance item 2: generations are explicit and never merged."""

    def test_a_reused_number_is_a_distinct_epoch(self):
        first = rel.Epoch("descriptor", 7, 1)
        self.assertTrue(rel.epochs_are_distinct(first, first.successor()))
        self.assertFalse(rel.epochs_are_distinct(first, rel.Epoch("descriptor", 7, 1)))

    def test_a_released_epoch_permits_no_operation_on_its_own_generation(self):
        self.assertEqual(linux(ZERO).permitted_recovery, rel.RECOVERY_NONE)


class JoinTest(unittest.TestCase):
    def test_disagreeing_paths_widen_to_indeterminate_with_no_recovery(self):
        released = linux(ZERO)
        not_attempted = linux(NOT_ATTEMPTED)
        joined = rel.join(released, not_attempted)
        self.assertEqual(joined.descriptor, rel.INDETERMINATE)
        self.assertEqual(joined.permitted_recovery, rel.RECOVERY_NONE)
        self.assertIn(rel.DUTY_NO_RETRY, joined.duties)
        self.assertNotIn(rel.DUTY_RELEASE, joined.duties)

    def test_agreeing_paths_are_preserved_exactly(self):
        joined = rel.join(linux(ZERO), linux(minus_one("EIO")))
        self.assertEqual(joined.descriptor, rel.RELEASED)
        self.assertEqual(joined.writeback, rel.WRITEBACK_FAILED)

    def test_a_join_never_invents_a_permitted_recovery(self):
        joined = rel.join(linux(NOT_ATTEMPTED), linux(NOT_ATTEMPTED))
        self.assertEqual(joined.permitted_recovery, frozenset({rel.RECOVERY_CLOSE}))
        mixed = rel.join(linux(NOT_ATTEMPTED), linux(ZERO))
        self.assertEqual(mixed.permitted_recovery, rel.RECOVERY_NONE)


class SuppressedFixtureContrastTest(unittest.TestCase):
    """The distinction the card exists to draw.

    The repository's suppressed-close fixtures return -1/EIO from a wrapper
    that never reaches the kernel, so the descriptor necessarily survives. That
    post-state is the one the POSIX.1-2024 target prescribes, not the one real
    Linux produces. Reading those fixtures as evidence about real Linux close
    failures would invert the answer, and this test pins that.
    """

    def test_the_suppressed_wrapper_post_state_is_not_the_linux_post_state(self):
        suppressed_observation = rel.RETAINED  # what the fixtures record
        self.assertEqual(linux(minus_one("EIO")).descriptor, rel.RELEASED)
        self.assertNotEqual(linux(minus_one("EIO")).descriptor, suppressed_observation)

    def test_the_suppressed_observation_matches_the_other_targets_prescription(self):
        self.assertEqual(
            rel.close_poststate(POSIX, rel.Premises(), minus_one("EINTR")).descriptor,
            rel.RETAINED,
        )


@unittest.skipUnless(platform.system() == "Linux", "real close(2) probe")
class RealLinuxProbeTest(unittest.TestCase):
    """Real, unsuppressed syscalls. Success and EBADF only."""

    @classmethod
    def setUpClass(cls):
        cls.probe = rel.probe_linux()

    def _observation(self, name):
        for entry in self.probe["observations"]:
            if entry["probe"].startswith(name):
                return entry
        raise AssertionError(f"missing probe {name}")

    def test_a_successful_close_really_releases_the_slot(self):
        entry = self._observation("P1")
        self.assertTrue(entry["descriptor_absent_after"])
        self.assertEqual(entry["relation_descriptor"], rel.RELEASED)

    def test_a_second_close_really_reports_ebadf(self):
        entry = self._observation("P2")
        self.assertEqual(entry["errno"], "EBADF")
        self.assertEqual(entry["relation_descriptor"], rel.INDETERMINATE)

    def test_the_number_is_really_handed_to_a_fresh_unrelated_epoch(self):
        # This is the measured hazard behind the no-retry rule: the number came
        # straight back, bound to a different file, so a retried close would
        # have closed the new epoch and returned zero.
        entry = self._observation("P3")
        self.assertTrue(entry["number_reused"])
        self.assertTrue(entry["identities_distinct"])
        self.assertTrue(entry["epochs_distinct"])

    def test_the_description_really_outlives_the_descriptor(self):
        entry = self._observation("P4")
        self.assertTrue(entry["closed_number_absent_after"])
        self.assertTrue(entry["duplicate_still_usable"])
        self.assertEqual(
            entry["relation_description_without_last_reference"],
            rel.DESCRIPTION_RETAINED,
        )

    def test_the_probe_does_not_claim_the_unobservable_arms(self):
        joined = " ".join(self.probe["not_observed"])
        self.assertIn("EINTR", joined)
        self.assertIn("EIO", joined)
        for entry in self.probe["observations"]:
            self.assertNotIn("EINTR", str(entry))

    def test_the_probe_records_its_artifact_and_target(self):
        self.assertEqual(len(self.probe["observations"]), 4)
        self.assertEqual(self.probe["target"], rel.SELECTED_TARGET_TRIPLE)
        self.assertIn("real unsuppressed", self.probe["artifact"])


if __name__ == "__main__":
    unittest.main()
