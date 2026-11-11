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

"""The reason inventory of the two adapter-assembled 185-unit sidecars.

Card REASON-CODES-B3. `source-effects.json` and `chunk-contracts.json` are
originally assembled by the compiler compatibility adapter (now
`crates/qcore-audit/src/pinned_route.rs`), which is why the REASON-CODES card left them without a
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
# The current baseline, deliberately re-pinned at the wave-17 consolidation
# (docs/reports/wave17-corpus/README.md); it carries REASON-CODES-B3's field.
ARTIFACTS = ROOT / "docs/reports/wave17-corpus/artifacts"
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

    def test_the_repinned_sidecars_carry_the_classified_field(self):
        """The measurement that replaced the pre-re-pin prediction.

        GEN-REASON predicted +1 key on each of the 93 `unknown` rows per
        sidecar and nothing on the 3 checked rows; the wave-9 re-pin measured
        exactly that (91 unsupported-syntax + 2 incomplete-obligation-coverage
        per sidecar). The pre-re-pin form of this test asserted the field's
        absence against the E01-F1 archive, which remains untouched as that
        card's record.
        """
        for name in SIDECARS:
            rows = load(name)["results"]
            unknown = [row for row in rows if row["status"] == "unknown"]
            codes = Counter(row["reason_code"] for row in unknown)
            self.assertEqual(
                codes,
                Counter({"unsupported-syntax": 91, "incomplete-obligation-coverage": 2}),
                name,
            )
            for row in rows:
                if row["status"] != "unknown":
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
