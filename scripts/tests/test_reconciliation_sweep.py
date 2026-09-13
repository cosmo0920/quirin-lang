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


class ResidualTests(unittest.TestCase):
    def _tree(self, scratch, tracked_text):
        root = pathlib.Path(scratch)
        write_report(root, "a-card", RESIDUAL_REPORT)
        (root / "AGENTS.md").write_text(tracked_text, encoding="utf-8")
        return root

    def test_a_name_no_tracker_mentions_is_a_candidate(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(scratch, "constraint 1: `A-CARD-TRACKED` stands.\n")
            found = sweep_module.untracked_residuals(root, ["a-card"])
        self.assertEqual(found, {"a-card": ["A-CARD-UNTRACKED"]})

    def test_single_words_are_not_residual_names(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(scratch, "")
            found = sweep_module.untracked_residuals(root, ["a-card"])
        self.assertNotIn("AGENTS", found["a-card"])
        self.assertNotIn("JSON", found["a-card"])

    def test_a_card_whose_names_are_all_tracked_is_omitted(self):
        with tempfile.TemporaryDirectory(prefix="qcore-sweep-") as scratch:
            root = self._tree(scratch, "`A-CARD-TRACKED` and `A-CARD-UNTRACKED`\n")
            self.assertEqual(sweep_module.untracked_residuals(root, ["a-card"]), {})

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
