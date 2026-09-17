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
        record = root / "record.json"
        record.write_text(
            json.dumps({"wave": 99, "cards": list(cards), "verdicts": verdicts}),
            encoding="utf-8",
        )
        return root, record

    def _complete(self):
        return {
            "a-card#integrator-reconciliation-list#1": {"verdict": "applied"},
            "a-card#integrator-reconciliation-list#2": {
                "verdict": "applied",
                "note": "AGENTS.md constraint 15h",
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
                f"a-card#integrator-reconciliation-list#{n}": {"verdict": "applied"}
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
        """A sweep that quietly covered nothing is the failure mode being removed."""
        self.assertEqual(sweep_module.cards_changed_since(ROOT, "HEAD"), [])
        finished = subprocess.run(
            [sys.executable, str(SCRIPT), str(ROOT), "--since", "HEAD"],
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
