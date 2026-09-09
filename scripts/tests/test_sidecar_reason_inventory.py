"""The reason inventory of the two adapter-assembled 185-unit sidecars.

Card REASON-CODES-B3. `source-effects.json` and `chunk-contracts.json` are
assembled by `compiler/examples/support/cycle19_compatibility.rs`, not by
`qcore-audit`, which is why the REASON-CODES card left them without a
`reason_code` field while `diagnostics.json` gained one; CORPUS-REPIN measured
that and recorded it as REASON-CODES-B3. The adapter now emits the category the
importer classified, so the next re-measurement of the pinned corpus will carry
it.

This test does NOT classify anything: duplicating the closed vocabulary in
Python would create exactly the second vocabulary the card exists to remove. It
pins two facts instead, and the classification stays in Rust:

  1. which reason strings the archived sidecars actually contain, and in what
     counts - so the predicted delta below is measured, not asserted; and
  2. that every one of those strings is a row of
     `crates/qcore-schemas/tests/reason_codes.rs`, whose Rust assertions are
     what fix each string's category.

The archived artifacts themselves are untouched by this card: adding the field
moves their bytes, and moving them is a deliberate corpus re-pin.
"""

import gzip
import json
import pathlib
import unittest
from collections import Counter

ROOT = pathlib.Path(__file__).resolve().parents[2]
# The current baseline, deliberately re-pinned by card E01-F1.
ARTIFACTS = ROOT / "docs/reports/remaining-e01-f1/artifacts/after"
REASON_CORPUS = ROOT / "crates/qcore-schemas/tests/reason_codes.rs"

# Measured on the archived bytes, read-only. Both sidecars carry the same 96
# rows: 93 `unknown` rows with a reason and 3 checked rows with none.
EXPECTED_REASONS = {
    "outside single borrowed chunk / int-or-void-result source profile": 74,
    "unmodeled statement DeclStmt": 13,
    "unmodeled expression CallExpr": 4,
    "expected one selected definition after preprocessing, found 0": 2,
}
EXPECTED_ROWS = 96
EXPECTED_UNKNOWN_ROWS = 93

SIDECARS = ["source-effects", "chunk-contracts"]


def load(name):
    return json.loads(gzip.open(ARTIFACTS / f"{name}.json.gz").read())


class SidecarReasonInventory(unittest.TestCase):
    def test_the_archived_sidecars_carry_the_measured_reason_inventory(self):
        for name in SIDECARS:
            rows = load(name)["results"]
            self.assertEqual(len(rows), EXPECTED_ROWS, name)
            unknown = [row for row in rows if row["status"] == "unknown"]
            self.assertEqual(len(unknown), EXPECTED_UNKNOWN_ROWS, name)
            self.assertEqual(Counter(row["reason"] for row in unknown), EXPECTED_REASONS, name)

    def test_the_archived_sidecars_do_not_yet_carry_the_field(self):
        """The blocking measurement, pinned rather than described.

        Emitting `reason_code` adds one key to each of the 93 `unknown` rows in
        each sidecar - 186 rows in all - and leaves the 3 checked rows and every
        other artifact byte alone. That is a pinned-artifact byte change, so the
        archived copies are left for the next deliberate corpus re-pin. This
        test fails the moment somebody re-pins them, which is when the
        prediction above has to be replaced by the measurement.
        """
        for name in SIDECARS:
            for row in load(name)["results"]:
                self.assertNotIn("reason_code", row, name)

    def test_every_archived_reason_is_a_row_of_the_rust_reason_corpus(self):
        """The Rust corpus is what fixes each string's category."""
        corpus = REASON_CORPUS.read_text()
        for reason in EXPECTED_REASONS:
            # The corpus wraps long literals, so compare on the distinctive
            # head of each string rather than the whole line.
            head = reason.split(",")[0][:48]
            self.assertIn(head, corpus, reason)

    def test_diagnostics_json_already_carries_the_field(self):
        """The control: the qcore-audit-assembled artifact is not affected."""
        units = load("diagnostics")["units"]
        self.assertEqual(len(units), 185)
        for unit in units:
            self.assertIn("reason_code", unit)
        codes = Counter(unit["reason_code"] for unit in units)
        self.assertEqual(
            codes,
            Counter(
                {"unsupported-syntax": 136, None: 48, "incomplete-obligation-coverage": 1}
            ),
        )
        # Every null is a unit with no reason at all, so REASON-CODES-B1's
        # opaque OS/tool text produces no unclassified row in this corpus.
        self.assertEqual(
            [unit for unit in units if unit["reason_code"] is None and unit["reason"]], []
        )


if __name__ == "__main__":
    unittest.main()
