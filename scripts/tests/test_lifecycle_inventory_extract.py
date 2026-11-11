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

"""The chunk-function inventory transport of `W42-CHUNK-LIFECYCLE-CERT`.

This test classifies nothing. Every lifecycle verdict is decided in Rust by
`qcore_verification::chunk_lifecycle_coverage` and pinned by
`compiler/tests/chunk_lifecycle_coverage.rs`. What is pinned here is the
transport:

  1. the committed inventory is exactly the extract of the pinned sidecar, so a
     re-measured corpus cannot silently re-bind an already published result;
  2. the extract refuses a digest that does not match its recorded pin; and
  3. the committed lifecycle result is bound to the same artifact digest,
     revision and selection as the inventory it was decided from.
"""

import gzip
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
REPORT = ROOT / "docs/reports/w42-chunk-lifecycle-cert"
SCRIPT = REPORT / "scripts/lifecycle_inventory_extract.py"
ARTIFACT = ROOT / "docs/reports/w42-priority-integration/artifacts/chunk-contracts.json.gz"
# The B-series copy this card's predecessor read; the two are byte-identical
# uncompressed, and the extract is pinned to that digest.
B_SERIES = ROOT / "docs/reports/b-series-integration/artifacts/chunk-contracts.json.gz"


def run(*arguments, cwd=ROOT):
    return subprocess.run(
        [sys.executable, str(SCRIPT), *arguments],
        cwd=cwd,
        capture_output=True,
        text=True,
        check=False,
    )


class LifecycleInventoryExtract(unittest.TestCase):
    def test_committed_inventory_is_the_extract_of_the_pinned_sidecar(self):
        result = run(
            str(ARTIFACT.relative_to(ROOT)),
            "--check",
            str((REPORT / "inventory.json").relative_to(ROOT)),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("96 selected functions", result.stdout)

    def test_both_archived_copies_extract_identically(self):
        first = run(str(ARTIFACT.relative_to(ROOT)))
        second = run(str(B_SERIES.relative_to(ROOT)))
        self.assertEqual(first.returncode, 0, first.stderr)
        self.assertEqual(second.returncode, 0, second.stderr)
        # Only the recorded artifact path differs; the selection does not.
        first_json = json.loads(first.stdout)
        second_json = json.loads(second.stdout)
        self.assertNotEqual(first_json["artifact"], second_json["artifact"])
        for key in ("artifact_sha256", "source_revision", "functions", "selected_functions"):
            self.assertEqual(first_json[key], second_json[key])

    def test_a_changed_artifact_is_refused(self):
        raw = gzip.decompress(ARTIFACT.read_bytes())
        mutated = json.loads(raw)
        mutated["results"] = mutated["results"][:-1]
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "chunk-contracts.json.gz"
            path.write_bytes(gzip.compress(json.dumps(mutated).encode()))
            result = run(str(path))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match the pinned", result.stderr)

    def test_published_result_is_bound_to_the_same_inventory(self):
        inventory = json.loads((REPORT / "inventory.json").read_text())
        published = json.loads((REPORT / "lifecycle-coverage.json").read_text())
        for key in ("artifact", "artifact_sha256", "source_revision", "source_profile"):
            self.assertEqual(inventory[key], published[key])
        self.assertEqual(inventory["selected_functions"], published["selected_functions"])
        self.assertEqual(published["accounted_functions"], published["selected_functions"])
        self.assertEqual(published["safe_functions"], 0)
        self.assertEqual(
            published["unknown_functions"], published["selected_functions"]
        )
        functions = {row["function"] for row in inventory["functions"]}
        self.assertIn("input_chunk_get", functions)
        self.assertEqual(
            functions, {row["function"] for row in published["functions"]}
        )
        for row in published["functions"]:
            self.assertEqual(row["lifecycle"]["status"], "unknown")
            self.assertTrue(row["lifecycle"]["reason"].strip())
            self.assertEqual(len(row["lifecycle"]["outstanding_legs"]), 9)


if __name__ == "__main__":
    unittest.main()
