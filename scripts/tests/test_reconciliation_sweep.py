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

"""Tests for `scripts/reconciliation_sweep.py` (card `W32-RECONCILE-SWEEP`).

Two things are pinned. First the extractor's behaviour on synthetic reports,
including the shape that motivated the tool: a section whose owed half is a
closing paragraph after a list of files the lane changed itself. Second the
`--check` contract — a close record missing one verdict is refused, and a green
check is never allowed to read as "everything was applied".

One live-tree test pins the extractor against a frozen wave-31 report so that a
future change to the heading vocabulary cannot silently stop finding the very
list whose omission occasioned this card.
"""

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "reconciliation_sweep.py"
sys.path.insert(0, str(ROOT / "scripts"))

import reconciliation_sweep as sweep_module  # noqa: E402


def write_report(root: pathlib.Path, card: str, body: str) -> None:
    directory = root / "docs" / "reports" / card
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "README.md").write_text(body, encoding="utf-8")


#: The ledger an `applied` claim is measured against. The amendment is
#: deliberately WRAPPED mid-sentence, because that is how `AGENTS.md` carries
#: every real amendment, and a verification arm that cannot read a wrapped
#: amendment is one that gets switched off at its first false red.
LEDGER = """# Ledger

15p. **A worker exercises its lease**, and the owner holds
    no whole-payload authority while one is out.
"""


def write_agents_md(root: pathlib.Path, body: str = LEDGER) -> None:
    """Write the fixture `AGENTS.md` an `applied` claim is measured against.

    NOT named `write_ledger`: this file already defines one, 280 lines down, for
    the follow-on tracker the residual tests read. The first draft of this helper
    took that name, was silently shadowed by the later definition, and every new
    test went red with "the named location does not exist" — which is this wave's
    own `ADD-NAME-COLLISION` residual, found by a test that failed loudly rather
    than by care.
    """
    (root / "AGENTS.md").write_text(body, encoding="utf-8")


NUMBERED_REPORT = """# A card

## 3. Something else

Not addressed to anyone in particular.

## 12. Integrator reconciliation list

This card may not edit `AGENTS.md`.

1. **A constraint amendment** the card wrote out.
2. **A namespace row** that should record a widening.

## 13. Artifacts
"""

CARRY_REPORT = """# A lane

## 9. Integrator carry list

Files this lane owns and changed:

* `docs/reevaluation/ct_manifest.json` - mapped.
* `compiler/tests/ct_corpus.rs` - pinned tables grown.

**Not** changed, and left for the integrator: the headline numbers.
"""

FENCED_REPORT = """# A proof leg

## 6. Proposed proof-stage evidence (the integrator's flip)

The exact stage record:

```json
{
  "status": "executed",
  "observations": 1
}
```

The harness edits the flip requires, none of which this card makes.
"""

SILENT_REPORT = """# A card that owes nothing

## 1. What landed

Everything, in this card's own commit.
"""


class ExtractionTests(unittest.TestCase):
    def test_a_numbered_reconciliation_list_yields_one_item_per_entry(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = pathlib.Path(scratch)
            write_report(root, "a-card", NUMBERED_REPORT)
            items = sweep_module.sweep(root, ["a-card"])
        texts = [item.text for item in items]
        self.assertIn("This card may not edit `AGENTS.md`.", texts)
        self.assertTrue(any("A constraint amendment" in text for text in texts))
        self.assertTrue(any("A namespace row" in text for text in texts))
        self.assertTrue(all(item.card == "a-card" for item in items))
        self.assertEqual(
            [item.id for item in items],
            [f"a-card#integrator-reconciliation-list#{n}" for n in (1, 2, 3)],
        )

    def test_a_section_outside_the_vocabulary_contributes_nothing(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = pathlib.Path(scratch)
            write_report(root, "quiet", SILENT_REPORT)
            self.assertEqual(sweep_module.sweep(root, ["quiet"]), [])

    def test_the_owed_half_after_a_file_list_is_not_lost(self):
        """The `CT22-WIRE` shape: nine files changed, one paragraph owed."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = pathlib.Path(scratch)
            write_report(root, "a-lane", CARRY_REPORT)
            items = sweep_module.sweep(root, ["a-lane"])
        self.assertTrue(
            any("left for the integrator" in item.text for item in items),
            [item.text for item in items],
        )

    def test_a_fenced_record_is_not_read_as_a_list(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = pathlib.Path(scratch)
            write_report(root, "a-leg", FENCED_REPORT)
            items = sweep_module.sweep(root, ["a-leg"])
        self.assertFalse(
            any(item.text.strip().startswith('"status"') for item in items),
            [item.text for item in items],
        )
        self.assertTrue(any("The harness edits" in item.text for item in items))

    def test_section_numbers_do_not_enter_the_item_id(self):
        self.assertEqual(
            sweep_module.slug("12. Integrator reconciliation list"),
            "integrator-reconciliation-list",
        )
        self.assertEqual(
            sweep_module.slug("11.2 One expected Python failure, left for the integrator"),
            "one-expected-python-failure-left-for-the-integrator",
        )

    def test_a_missing_card_report_is_refused_rather_than_skipped(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            with self.assertRaises(SystemExit):
                sweep_module.sweep(pathlib.Path(scratch), ["no-such-card"])


class CheckTests(unittest.TestCase):
    def _tree(self, scratch, verdicts, cards=("a-card",)):
        root = pathlib.Path(scratch)
        write_report(root, "a-card", NUMBERED_REPORT)
        write_agents_md(root)
        record = root / "record.json"
        record.write_text(
            json.dumps({"wave": 99, "cards": list(cards), "verdicts": verdicts}),
            encoding="utf-8",
        )
        return root, record

    def _complete(self):
        return {
            "a-card#integrator-reconciliation-list#1": {
                "verdict": "applied",
                "verify": [
                    {
                        "file": "AGENTS.md",
                        "contains": "the owner holds no whole-payload authority",
                    }
                ],
            },
            "a-card#integrator-reconciliation-list#2": {
                "verdict": "applied",
                "note": "AGENTS.md constraint 15h",
                "unverifiable": (
                    "the matrix was regenerated; what landed is a derived count, "
                    "not a sentence to quote"
                ),
            },
            "a-card#integrator-reconciliation-list#3": {
                "verdict": "declined",
                "note": "spec §L21 already carries it",
            },
        }

    def test_a_complete_record_passes(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(scratch, self._complete())
            self.assertEqual(sweep_module.check(root, record), [])

    def test_one_missing_verdict_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            verdicts = self._complete()
            del verdicts["a-card#integrator-reconciliation-list#2"]
            root, record = self._tree(scratch, verdicts)
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1)
        self.assertIn("no verdict recorded", complaints[0])

    def test_a_verdict_outside_the_vocabulary_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            verdicts = self._complete()
            verdicts["a-card#integrator-reconciliation-list#1"] = {"verdict": "probably"}
            root, record = self._tree(scratch, verdicts)
            complaints = sweep_module.check(root, record)
        self.assertTrue(any("is not one of" in complaint for complaint in complaints))

    def test_a_non_applied_verdict_must_carry_a_note(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            verdicts = self._complete()
            verdicts["a-card#integrator-reconciliation-list#3"] = {"verdict": "deferred"}
            root, record = self._tree(scratch, verdicts)
            complaints = sweep_module.check(root, record)
        self.assertTrue(any("carries no note" in complaint for complaint in complaints))

    def test_a_verdict_for_an_unknown_item_is_reported(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            verdicts = self._complete()
            verdicts["a-card#invented#9"] = {"verdict": "applied"}
            root, record = self._tree(scratch, verdicts)
            complaints = sweep_module.check(root, record)
        self.assertTrue(any("the sweep does not find" in c for c in complaints))

    def test_a_swept_card_with_no_section_is_reported_rather_than_passed(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(scratch, self._complete(), cards=("a-card", "quiet"))
            write_report(root, "quiet", SILENT_REPORT)
            complaints = sweep_module.check(root, record)
        self.assertTrue(any(c.startswith("quiet:") for c in complaints))

    def _with_no_section(self, scratch, silent, cards=("a-card", "quiet")):
        root = pathlib.Path(scratch)
        write_report(root, "a-card", NUMBERED_REPORT)
        write_report(root, "quiet", SILENT_REPORT)
        write_agents_md(root)
        record = root / "record.json"
        record.write_text(
            json.dumps(
                {
                    "wave": 99,
                    "cards": list(cards),
                    "verdicts": self._complete(),
                    "no_section": silent,
                }
            ),
            encoding="utf-8",
        )
        return root, record

    def test_a_stated_reason_answers_the_no_section_complaint(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._with_no_section(scratch, {"quiet": "owed nothing; verified"})
            self.assertEqual(sweep_module.check(root, record), [])

    def test_an_empty_reason_does_not_answer_it(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._with_no_section(scratch, {"quiet": "   "})
            complaints = sweep_module.check(root, record)
        self.assertTrue(any(c.startswith("quiet:") for c in complaints))

    def test_no_section_cannot_wave_away_a_card_that_has_one(self):
        """Otherwise the escape hatch would reintroduce exactly this card's defect."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._with_no_section(
                scratch, {"quiet": "owed nothing", "a-card": "nothing to see"}
            )
            complaints = sweep_module.check(root, record)
        self.assertTrue(any("does have a section" in c for c in complaints))

    def test_noted_is_a_verdict_and_still_needs_a_note(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            verdicts = self._complete()
            verdicts["a-card#integrator-reconciliation-list#1"] = {"verdict": "noted"}
            root, record = self._tree(scratch, verdicts)
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1)
        self.assertIn("carries no note", complaints[0])

    def test_a_missing_record_is_a_complaint_not_a_pass(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = pathlib.Path(scratch)
            complaints = sweep_module.check(root, root / "absent.json")
        self.assertEqual(len(complaints), 1)
        self.assertIn("no reconciliation record", complaints[0])

    def test_green_output_disclaims_having_measured_the_tree(self):
        """A tool that reported completeness as correctness would be worse than none."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(scratch, self._complete())
            finished = subprocess.run(
                [sys.executable, str(SCRIPT), str(root), "--check", str(record)],
                capture_output=True,
                text=True,
                check=False,
            )
        self.assertEqual(finished.returncode, 0, finished.stderr)
        self.assertIn("says nothing about whether any item was applied", finished.stdout)


RESIDUAL_REPORT = """# A card with residuals

## 9. Named follow-ons

| Name | What is missing |
|---|---|
| `A-CARD-TRACKED` | already in the ledger |
| `A-CARD-UNTRACKED` | in no ledger at all |

Prose about `AGENTS` and `JSON`, neither of which is a residual name.
"""


def write_ledger(root: pathlib.Path, *names: str) -> None:
    """The one tracker: `named_followups` in the acceptance matrix."""
    matrix = root / "docs" / "reviews" / "r0_3"
    matrix.mkdir(parents=True, exist_ok=True)
    (matrix / "acceptance_matrix.json").write_text(
        json.dumps({"named_followups": [{"id": name} for name in names]}), encoding="utf-8"
    )


def names_of(found) -> list:
    return [entry.name for entry in found]


class ResidualTests(unittest.TestCase):
    def _tree(self, scratch, ledger=(), agents_text=""):
        root = pathlib.Path(scratch)
        write_report(root, "a-card", RESIDUAL_REPORT)
        write_ledger(root, *ledger)
        (root / "AGENTS.md").write_text(agents_text, encoding="utf-8")
        return root

    def test_a_name_the_ledger_does_not_carry_is_a_candidate(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(scratch, ledger=("A-CARD-TRACKED",))
            found = sweep_module.untracked_residuals(root, ["a-card"])
        self.assertEqual(names_of(found), ["A-CARD-UNTRACKED"])

    def test_single_words_are_not_residual_names(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(scratch)
            found = names_of(sweep_module.untracked_residuals(root, ["a-card"]))
        self.assertNotIn("AGENTS", found)
        self.assertNotIn("JSON", found)

    def test_a_card_whose_names_the_ledger_carries_is_omitted(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(scratch, ledger=("A-CARD-TRACKED", "A-CARD-UNTRACKED"))
            self.assertEqual(sweep_module.untracked_residuals(root, ["a-card"]), [])

    def test_a_name_only_a_constraint_mentions_is_untracked_work(self):
        """`RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK`, non-vacuously.

        This is the exact fixture the old `TRACKERS` got wrong: both names appear
        in `AGENTS.md` and neither is in the ledger, so the old tool returned
        `{}` - tracked - and said nothing. That silence is what left
        `RETURN-POSITION-ESCAPE-UNREACHABLE` and its sibling out of every
        open-work list. A constraint states a rule; the ledger states open work.
        """
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(
                scratch,
                ledger=(),
                agents_text=(
                    "## Standing constraints when touching specific subsystems\n\n"
                    "5b. `A-CARD-TRACKED` and `A-CARD-UNTRACKED` must survive refactors.\n"
                ),
            )
            found = sweep_module.untracked_residuals(root, ["a-card"])
        self.assertEqual(names_of(found), ["A-CARD-TRACKED", "A-CARD-UNTRACKED"])
        self.assertTrue(
            all(entry.mentions == ("AGENTS.md",) for entry in found),
            [entry.mentions for entry in found],
        )

    def test_a_residual_introduced_in_an_increment_record_is_seen(self):
        """`SWEEP-RESIDUALS-SCANS-REPORTS-ONLY`, non-vacuously.

        The name is written in no card report at all, so the old reports-only
        scan could not return it however the ledger was arranged.
        """
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(scratch, ledger=("A-CARD-TRACKED", "A-CARD-UNTRACKED"))
            records = root / "docs" / "development" / "increments"
            records.mkdir(parents=True)
            (records / "wave-99.md").write_text(
                "The integrator left `INCREMENT-ONLY-RESIDUAL` open.\n", encoding="utf-8"
            )
            found = sweep_module.untracked_residuals(root, ["a-card"])
        self.assertEqual(names_of(found), ["INCREMENT-ONLY-RESIDUAL"])
        self.assertEqual(
            found[0].sources, ("docs/development/increments/wave-99.md",)
        )

    def test_the_constraint_scan_reads_the_constraints_section_only(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(
                scratch,
                ledger=("A-CARD-TRACKED", "A-CARD-UNTRACKED"),
                agents_text=(
                    "## Status ledger\n\n"
                    "Card `LEDGER-ROW-NAME` moved the count.\n\n"
                    "## Standing constraints when touching specific subsystems\n\n"
                    "1. `A-CONSTRAINT-NAME` stands.\n\n"
                    "## Standing per-change gate\n\n"
                    "`A-GATE-NAME` is run before merge.\n"
                ),
            )
            found = names_of(sweep_module.untracked_residuals(root, ["a-card"]))
        self.assertIn("A-CONSTRAINT-NAME", found)
        self.assertNotIn("LEDGER-ROW-NAME", found)
        self.assertNotIn("A-GATE-NAME", found)

    def test_a_file_with_no_constraints_heading_is_read_whole(self):
        """`CLAUDE.md` is that file, and its platform records name residuals."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(scratch, ledger=("A-CARD-TRACKED", "A-CARD-UNTRACKED"))
            (root / "CLAUDE.md").write_text(
                "## Platform records\n\nOpen: `QUIRIN-ARM64-GATE`.\n", encoding="utf-8"
            )
            found = names_of(sweep_module.untracked_residuals(root, ["a-card"]))
        self.assertEqual(found, ["QUIRIN-ARM64-GATE"])

    def test_routed_rule_and_platform_names_are_seen_but_not_tracking(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(scratch, ledger=("A-CARD-TRACKED", "A-CARD-UNTRACKED"))
            policy = root / "docs/agents/gateway.md"
            policy.parent.mkdir(parents=True)
            policy.write_text("Rule: `ROUTED-ONLY-RESIDUAL`.\n", encoding="utf-8")
            platform = root / "docs/development/platform-evidence.md"
            platform.parent.mkdir(parents=True)
            platform.write_text("Open: `PLATFORM-ONLY-RESIDUAL`.\n", encoding="utf-8")
            found = sweep_module.untracked_residuals(root, ["a-card"])
            self.assertEqual(names_of(found), ["PLATFORM-ONLY-RESIDUAL", "ROUTED-ONLY-RESIDUAL"])
            self.assertEqual(found[1].mentions, ("docs/agents/gateway.md",))
            write_ledger(root, "A-CARD-TRACKED", "A-CARD-UNTRACKED", "ROUTED-ONLY-RESIDUAL")
            self.assertEqual(names_of(sweep_module.untracked_residuals(root, ["a-card"])),
                             ["PLATFORM-ONLY-RESIDUAL"])

    def test_routed_policy_discovery_does_not_treat_archives_as_live_rules(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(scratch, ledger=("A-CARD-TRACKED", "A-CARD-UNTRACKED"))
            archive = root / "docs/development/increments/structure-checkpoints/old.md"
            archive.parent.mkdir(parents=True)
            archive.write_text("Old rule: `ARCHIVE-ONLY-NAME`.\n", encoding="utf-8")
            self.assertNotIn(str(archive.relative_to(root)), sweep_module.rule_documents(root))
            self.assertEqual(sweep_module.untracked_residuals(root, ["a-card"]), [])

    def test_the_cli_says_it_classifies_nothing(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(scratch, "")
            finished = subprocess.run(
                [sys.executable, str(SCRIPT), str(root), "--cards", "a-card", "--residuals"],
                capture_output=True,
                text=True,
                check=False,
            )
        self.assertEqual(finished.returncode, 0, finished.stderr)
        self.assertIn("classifies none of them", finished.stdout)


def run_git(root: pathlib.Path, *args: str) -> str:
    finished = subprocess.run(
        ["git", *args], cwd=root, capture_output=True, text=True, check=True
    )
    return finished.stdout.strip()


def fixture_repo(root: pathlib.Path) -> dict:
    """A three-commit history with the `WAVE-BASE-UNSWEPT` shape in it.

    `close` - the previous wave's close commit.
    `base`  - a commit that writes a card report AND is what a later close would
              declare as its `--since` base. Its report is what the exclusive
              range could never see.
    `seam`  - a commit in the same position writing NO report at all, which is
              `37bc9004` exactly.
    """
    run_git(root, "init", "-q", "-b", "main")
    run_git(root, "config", "user.email", "t@example.invalid")
    run_git(root, "config", "user.name", "T")
    (root / "marker").write_text("0\n", encoding="utf-8")
    run_git(root, "add", "-A")
    run_git(root, "commit", "-q", "-m", "Close the previous wave")
    close = run_git(root, "rev-parse", "HEAD")

    write_report(root, "base-card", NUMBERED_REPORT)
    run_git(root, "add", "-A")
    run_git(root, "commit", "-q", "-m", "BASE-CARD: the report the base commit writes")
    base = run_git(root, "rev-parse", "HEAD")

    (root / "marker").write_text("1\n", encoding="utf-8")
    run_git(root, "add", "-A")
    run_git(root, "commit", "-q", "-m", "Fix some gates")
    seam = run_git(root, "rev-parse", "HEAD")
    return {"close": close, "base": base, "seam": seam}


class SeamTests(unittest.TestCase):
    """`WAVE-BASE-UNSWEPT`: the base commit's own work was invisible."""

    def test_the_base_commits_own_report_is_swept_only_when_the_base_is_included(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = pathlib.Path(scratch)
            shas = fixture_repo(root)
            exclusive = sweep_module.cards_changed_in_range(
                root, shas["base"], "HEAD", include_base=False
            )
            inclusive = sweep_module.cards_changed_in_range(
                root, shas["base"], "HEAD", include_base=True
            )
        self.assertEqual(exclusive, [], "the old behaviour, pinned so the repair is visible")
        self.assertEqual(inclusive, ["base-card"])

    def test_cards_changed_since_keeps_its_exclusive_meaning(self):
        """The primitive is unchanged; only the close's range moved."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = pathlib.Path(scratch)
            shas = fixture_repo(root)
            self.assertEqual(sweep_module.cards_changed_since(root, shas["base"]), [])

    def test_an_explicit_range_sweeps_between_two_closes(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = pathlib.Path(scratch)
            shas = fixture_repo(root)
            found = sweep_module.cards_changed_in_range(root, shas["close"], shas["seam"])
        self.assertEqual(found, ["base-card"])

    def test_parse_range_wants_two_ends(self):
        self.assertEqual(sweep_module.parse_range("a..b"), ("a", "b"))
        self.assertEqual(sweep_module.parse_range("a.."), ("a", "HEAD"))
        with self.assertRaises(SystemExit):
            sweep_module.parse_range("a b")

    def test_a_commit_that_writes_no_report_is_listed(self):
        """The `37bc9004` shape: real work, no card, invisible to every record."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = pathlib.Path(scratch)
            shas = fixture_repo(root)
            found = sweep_module.uncarded_commits(root, shas["base"], "HEAD")
        subjects = [subject for _, subject, _ in found]
        self.assertIn("Fix some gates", subjects)
        self.assertNotIn("BASE-CARD: the report the base commit writes", subjects)

    def test_uncarded_needs_a_range(self):
        finished = subprocess.run(
            [sys.executable, str(SCRIPT), str(ROOT), "--uncarded"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("needs --since or --range", finished.stderr + finished.stdout)

    def test_the_uncarded_listing_disclaims_classifying(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = pathlib.Path(scratch)
            shas = fixture_repo(root)
            finished = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    str(root),
                    "--range",
                    f"{shas['close']}..HEAD",
                    "--uncarded",
                ],
                capture_output=True,
                text=True,
                check=False,
            )
        self.assertEqual(finished.returncode, 0, finished.stderr)
        self.assertIn("classifies none of them", finished.stdout)


class AppliedClaimTests(unittest.TestCase):
    """`RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED` (`STABILITY-REEVAL` #2, wave 36).

    The wave-35 close recorded 44 verdicts `applied`, each with a `where` naming
    a file — and one of those files did not carry the asserted text. `where` is
    prose, and prose naming a location is not a measurement of it. This arm
    opens the named location and reads.

    Every test here is shown red **by mutation**: the fixture that passes is
    mutated in one place and the same check is asserted to complain. A test that
    only ever sees the green case is evidence about nothing.
    """

    def _tree(self, scratch, verdicts, ledger=LEDGER):
        root = pathlib.Path(scratch)
        write_report(root, "a-card", NUMBERED_REPORT)
        write_agents_md(root, ledger)
        record = root / "record.json"
        record.write_text(
            json.dumps({"wave": 99, "cards": ["a-card"], "verdicts": verdicts}),
            encoding="utf-8",
        )
        return root, record

    def _verdicts(self, first):
        rest = {
            f"a-card#integrator-reconciliation-list#{n}": {
                "verdict": "noted",
                "note": "not this test's subject",
            }
            for n in (2, 3)
        }
        return {"a-card#integrator-reconciliation-list#1": first, **rest}

    # --- the arm is green where the tree agrees ---------------------------

    def test_an_asserted_sentence_present_in_the_tree_passes(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "verify": [
                            {"file": "AGENTS.md", "contains": "A worker exercises its lease"}
                        ],
                    }
                ),
            )
            self.assertEqual(sweep_module.check(root, record), [])

    def test_a_quotation_the_tree_wraps_still_matches(self):
        """The whole arm turns on this: `AGENTS.md` wraps at 76 columns.

        The asserted text spans the fixture's line break. A literal substring
        search would call a landed amendment missing.
        """
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "verify": [
                            {
                                "file": "AGENTS.md",
                                "contains": "the owner holds no whole-payload authority",
                            }
                        ],
                    }
                ),
            )
            self.assertEqual(sweep_module.check(root, record), [])
            self.assertNotIn(
                "the owner holds no whole-payload authority",
                (root / "AGENTS.md").read_text(encoding="utf-8"),
                "the fixture must wrap, or this test proves nothing",
            )

    # --- and red, by mutation, where it does not --------------------------

    def test_an_asserted_sentence_the_tree_does_not_carry_is_refused(self):
        """The wave-35 defect itself: `applied`, a named file, and no such text."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "where": "AGENTS.md, constraint 15p.",
                        "verify": [
                            {"file": "AGENTS.md", "contains": "the owner retains read authority"}
                        ],
                    }
                ),
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("does NOT carry the asserted text", complaints[0])
        self.assertIn("the named location disagrees", complaints[0])

    def test_one_changed_word_is_a_changed_claim(self):
        """Invariant 1's hazard: an amendment silently SOFTENED while landing.

        The fixture is the green case with the ledger mutated in exactly one
        word — `no` becoming `some`. Normalising whitespace must not normalise
        this away.
        """
        softened = LEDGER.replace("no whole-payload authority", "some whole-payload authority")
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "verify": [
                            {
                                "file": "AGENTS.md",
                                "contains": "the owner holds no whole-payload authority",
                            }
                        ],
                    }
                ),
                ledger=softened,
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("does NOT carry the asserted text", complaints[0])

    def test_a_named_location_that_does_not_exist_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "verify": [{"file": "docs/NO_SUCH.md", "contains": "anything at all"}],
                    }
                ),
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("the named location does not exist", complaints[0])

    def test_an_absent_claim_is_refused_while_the_stale_text_stands(self):
        """The other half: a superseded sentence asserted gone but still there."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "verify": [
                            {"file": "AGENTS.md", "contains": "A worker exercises its lease"},
                            {"file": "AGENTS.md", "absent": "A worker exercises its lease"},
                        ],
                    }
                ),
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("STILL carries the text asserted absent", complaints[0])

    def test_an_absent_claim_passes_once_the_stale_text_is_gone(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "verify": [
                            {"file": "AGENTS.md", "absent": "proof 3 of 45"},
                        ],
                    }
                ),
            )
            self.assertEqual(sweep_module.check(root, record), [])

    # --- and it cannot be satisfied by saying nothing ---------------------

    def test_a_bare_applied_verdict_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(scratch, self._verdicts({"verdict": "applied"}))
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED", complaints[0])

    def test_prose_naming_a_location_is_not_a_measurement_of_it(self):
        """A `where` field alone is exactly what wave 35 had. It is not enough."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "where": "AGENTS.md, constraint 15p, landed verbatim.",
                        "note": "I looked and it is there.",
                    }
                ),
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("neither 'verify'", complaints[0])

    def test_a_declared_unverifiable_applied_claim_passes_with_a_ground(self):
        """The escape hatch exists, is deliberate, and is counted in the summary."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "unverifiable": "the matrix was regenerated; no sentence to quote",
                    }
                ),
            )
            self.assertEqual(sweep_module.check(root, record), [])

    def test_an_empty_ground_does_not_answer_it(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch, self._verdicts({"verdict": "applied", "unverifiable": "   "})
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED", complaints[0])

    def test_asserting_and_declaring_unverifiable_at_once_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "unverifiable": "no sentence to quote",
                        "verify": [
                            {"file": "AGENTS.md", "contains": "A worker exercises its lease"}
                        ],
                    }
                ),
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("never both", complaints[0])

    def test_the_escape_hatch_is_refused_on_a_verdict_that_asserts_no_landing(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "declined",
                        "note": "spec §L21 already carries it",
                        "unverifiable": "nothing landed",
                    }
                ),
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("asserts no landing", complaints[0])

    # --- malformed claims are complaints, never silent passes -------------

    def test_a_claim_carrying_both_kinds_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "verify": [{"file": "AGENTS.md", "contains": "a", "absent": "a"}],
                    }
                ),
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("carries exactly one of", complaints[0])

    def test_a_claim_carrying_neither_kind_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts({"verdict": "applied", "verify": [{"file": "AGENTS.md"}]}),
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("carries exactly one of", complaints[0])

    def test_a_claim_naming_no_file_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts({"verdict": "applied", "verify": [{"contains": "a sentence"}]}),
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("names no 'file'", complaints[0])

    def test_a_file_outside_the_repository_is_refused(self):
        """A close verifies the tree it is closing, not the machine it runs on."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "verify": [{"file": "../escape.md", "contains": "anything"}],
                    }
                ),
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("escapes the repository root", complaints[0])

    def test_every_claim_of_a_list_is_read(self):
        """Not just the first: a two-claim entry with a bad second one is red."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "verify": [
                            {"file": "AGENTS.md", "contains": "A worker exercises its lease"},
                            {"file": "AGENTS.md", "contains": "a sentence nobody wrote"},
                        ],
                    }
                ),
            )
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("verify[2]", complaints[0])

    def test_the_cli_reports_what_it_verified_and_what_it_did_not(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._tree(
                scratch,
                self._verdicts(
                    {
                        "verdict": "applied",
                        "verify": [
                            {"file": "AGENTS.md", "contains": "A worker exercises its lease"},
                            {"file": "AGENTS.md", "absent": "proof 3 of 45"},
                        ],
                    }
                ),
            )
            finished = subprocess.run(
                [sys.executable, str(SCRIPT), str(root), "--check", str(record)],
                capture_output=True,
                text=True,
                check=False,
            )
        self.assertEqual(finished.returncode, 0, finished.stderr + finished.stdout)
        self.assertIn("2 content claims (1 present, 1 absent)", finished.stdout)
        self.assertIn("1 of 1 'applied' verdicts carry checkable text", finished.stdout)
        self.assertIn("Whether it is the RIGHT string stays a person's judgment", finished.stdout)


class DeferredDecisionTests(unittest.TestCase):
    """`WAVE-33-DEFERRED-DECISIONS-UNTRACKED`: a deferral nothing ever re-reads."""

    OPEN = {
        "note": "one decision this close does not take",
        "items": [
            {
                "name": "A-DEFERRED-DECISION",
                "verdict": "deferred",
                "ground": "belongs to a mapping lane",
            }
        ],
    }

    def _closes(self, scratch, earlier, later_block):
        root = pathlib.Path(scratch)
        write_report(root, "a-card", NUMBERED_REPORT)
        earlier_dir = root / "docs" / "reports" / "wave40-close"
        earlier_dir.mkdir(parents=True, exist_ok=True)
        (earlier_dir / "reconciliation.json").write_text(
            json.dumps(
                {
                    "wave": 40,
                    "cards": ["a-card"],
                    "verdicts": {},
                    "deferred_decisions": earlier,
                }
            ),
            encoding="utf-8",
        )
        record = root / "docs" / "reports" / "wave41-close" / "reconciliation.json"
        record.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "wave": 41,
            "cards": ["a-card"],
            "verdicts": {
                f"a-card#integrator-reconciliation-list#{n}": {
                    "verdict": "applied",
                    "unverifiable": "this fixture is about deferrals, not about content claims",
                }
                for n in (1, 2, 3)
            },
        }
        if later_block is not None:
            payload["deferred_decisions"] = later_block
        record.write_text(json.dumps(payload), encoding="utf-8")
        return root, record

    def test_a_later_close_that_ignores_an_open_deferral_is_refused(self):
        """Before this card a deferral lived in one JSON file and expired in silence."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._closes(scratch, self.OPEN, None)
            complaints = sweep_module.check(root, record)
        self.assertEqual(len(complaints), 1, complaints)
        self.assertIn("A-DEFERRED-DECISION", complaints[0])
        self.assertIn("not re-listed here", complaints[0])

    def test_re_stating_the_deferral_answers_it(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._closes(
                scratch,
                self.OPEN,
                {
                    "items": [
                        {
                            "name": "A-DEFERRED-DECISION",
                            "verdict": "deferred",
                            "ground": "still a lane's, and no lane ran this wave",
                        }
                    ]
                },
            )
            self.assertEqual(sweep_module.check(root, record), [])

    def test_resolving_it_ends_the_carry(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._closes(
                scratch,
                self.OPEN,
                {
                    "items": [
                        {
                            "name": "A-DEFERRED-DECISION",
                            "verdict": "resolved",
                            "ground": "taken at wave 41",
                        }
                    ]
                },
            )
            self.assertEqual(sweep_module.check(root, record), [])
            self.assertEqual(sweep_module.outstanding_decisions(root, 42), {})

    def test_a_decision_verdict_outside_the_vocabulary_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._closes(
                scratch,
                self.OPEN,
                {"items": [{"name": "A-DEFERRED-DECISION", "verdict": "maybe", "ground": "x"}]},
            )
            complaints = sweep_module.check(root, record)
        self.assertTrue(any("is not one of" in c for c in complaints), complaints)

    def test_a_deferral_restated_without_a_ground_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._closes(
                scratch,
                self.OPEN,
                {"items": [{"name": "A-DEFERRED-DECISION", "verdict": "deferred"}]},
            )
            complaints = sweep_module.check(root, record)
        self.assertTrue(any("carries no ground" in c for c in complaints), complaints)

    def test_a_sibling_file_registers_a_closed_records_deferrals(self):
        """So a frozen close record never has to be edited (constraint 21)."""
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._closes(scratch, {}, None)
            sibling = root / "docs" / "reports" / "wave40-close" / "deferred_decisions.json"
            sibling.write_text(
                json.dumps(
                    {
                        "items": [
                            {"name": "A-RETRO-DECISION", "verdict": "deferred", "ground": "g"}
                        ]
                    }
                ),
                encoding="utf-8",
            )
            complaints = sweep_module.check(root, record)
        self.assertTrue(any("A-RETRO-DECISION" in c for c in complaints), complaints)

    def test_a_plain_map_is_read_as_well_as_the_items_list(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root, record = self._closes(
                scratch,
                {"A-DEFERRED-DECISION": {"verdict": "deferred", "ground": "g"}},
                {"A-DEFERRED-DECISION": {"verdict": "withdrawn", "ground": "overtaken"}},
            )
            self.assertEqual(sweep_module.check(root, record), [])


class LiveTreeTests(unittest.TestCase):
    def test_the_occasioning_list_is_still_found(self):
        """`WRITE-LEND-TASKS` §12 is the list whose omission occasioned this card."""
        items = sweep_module.sweep(ROOT, ["write-lend-tasks"])
        ids = {item.id for item in items}
        self.assertIn("write-lend-tasks#integrator-reconciliation-list#2", ids)
        self.assertTrue(
            any("constraint 15h" in item.text for item in items),
            sorted(ids),
        )

    def test_this_cards_own_retrospective_record_is_complete(self):
        """Waves 29-31 swept and given verdicts by card `W32-RECONCILE-SWEEP`.

        Complete, not correct: every item carries a human verdict measured
        against the tree at that card's tip. The tool checked the first half of
        that sentence and not the second.
        """
        record = ROOT / "docs/reports/w32-reconcile-sweep/reconciliation-waves-29-31.json"
        self.assertEqual(sweep_module.check(ROOT, record), [])

    def test_a_since_ref_with_no_diff_fails_loudly(self):
        """A sweep that quietly covered nothing is the failure mode being removed.

        The window is `HEAD..HEAD`, empty by construction. It used to be
        `--since HEAD`, which was empty only while HEAD carried no card report;
        since SWEEP-BLIND-SPOTS made `--since` inclusive of its base
        (WAVE-BASE-UNSWEPT), that premise fails on every main whose tip is a
        card merge - the integrator hit it at 56a97db0. The exclusive
        `cards_changed_since` keeps its meaning and is asserted unchanged.
        """
        self.assertEqual(sweep_module.cards_changed_since(ROOT, "HEAD"), [])
        finished = subprocess.run(
            [sys.executable, str(SCRIPT), str(ROOT), "--range", "HEAD..HEAD"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("no card reports changed", finished.stderr + finished.stdout)

    def test_an_unknown_since_ref_fails_loudly(self):
        finished = subprocess.run(
            [sys.executable, str(SCRIPT), str(ROOT), "--since", "no-such-ref-w32"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(finished.returncode, 0)
        self.assertIn("failed", finished.stderr + finished.stdout)

    def test_the_cli_runs_over_a_real_card(self):
        finished = subprocess.run(
            [sys.executable, str(SCRIPT), str(ROOT), "--cards", "write-lend-tasks", "--json"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(finished.returncode, 0, finished.stderr)
        payload = json.loads(finished.stdout)
        self.assertTrue(payload)
        self.assertTrue(all(entry["card"] == "write-lend-tasks" for entry in payload))


if __name__ == "__main__":
    unittest.main()
